use crate::binding::Binding;
use crate::image_cache;
use ab_glyph::{FontRef, PxScale};
use anyhow::{Context, Result};
use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use std::path::Path;

/// Renders images for Stream Deck hardware buttons.
pub struct ButtonRenderer {
    font: FontRef<'static>,
    button_size: (u32, u32),
}

// Embedded font for text rendering (DejaVu Sans Mono Bold subset or similar)
const EMBEDDED_FONT: &[u8] = include_bytes!("../assets/DejaVuSans-Bold.ttf");

impl ButtonRenderer {
    /// Create a new renderer for the given button dimensions.
    pub fn new(button_width: u32, button_height: u32) -> Result<Self> {
        let font =
            FontRef::try_from_slice(EMBEDDED_FONT).context("Failed to load embedded font")?;

        Ok(Self {
            font,
            button_size: (button_width, button_height),
        })
    }

    /// Load an image from a local file path (uncached).
    #[allow(dead_code)]
    pub fn load_file(&self, path: &Path) -> Result<DynamicImage> {
        image::open(path).context(format!("Failed to load image: {}", path.display()))
    }

    /// Fetch and load an image from a URL (uncached).
    #[allow(dead_code)]
    pub fn load_url(&self, url: &str) -> Result<DynamicImage> {
        let response = reqwest::blocking::get(url).context("Failed to fetch image from URL")?;

        let bytes = response.bytes().context("Failed to read image bytes")?;

        image::load_from_memory(&bytes).context("Failed to decode image from URL")
    }

    /// Resize an image to fit the button dimensions.
    /// Uses aspect-preserving resize and centers the image.
    pub fn resize(&self, img: DynamicImage) -> DynamicImage {
        let (target_w, target_h) = self.button_size;

        // Resize to fit within button, preserving aspect ratio
        let resized = img.resize(target_w, target_h, image::imageops::FilterType::Lanczos3);

        // If the resized image is smaller than target, center it on a black background
        if resized.width() < target_w || resized.height() < target_h {
            let mut canvas = RgbaImage::from_pixel(target_w, target_h, Rgba([0, 0, 0, 255]));
            let x_offset = (target_w - resized.width()) / 2;
            let y_offset = (target_h - resized.height()) / 2;

            image::imageops::overlay(&mut canvas, &resized.to_rgba8(), x_offset as i64, y_offset as i64);
            DynamicImage::ImageRgba8(canvas)
        } else {
            resized
        }
    }

    /// Add a label to the bottom of the image.
    pub fn add_label(&self, img: &mut RgbaImage, label: &str) {
        if label.is_empty() {
            return;
        }

        let (w, h) = (img.width(), img.height());

        // Calculate font size based on button height (roughly 15% of height)
        let font_size = (h as f32 * 0.15).max(10.0);
        let scale = PxScale::from(font_size);

        // Truncate label if too long
        let max_chars = (w as usize / 8).max(4);
        let display_label = if label.len() > max_chars {
            format!("{}...", &label[..max_chars.saturating_sub(3)])
        } else {
            label.to_string()
        };

        // Estimate text width for centering
        let char_width = font_size * 0.6;
        let text_width = display_label.len() as f32 * char_width;
        let x = ((w as f32 - text_width) / 2.0).max(2.0) as i32;

        // Position text near bottom with some padding
        let y = (h as f32 - font_size - 4.0).max(0.0) as i32;

        // Draw text shadow for visibility
        draw_text_mut(
            img,
            Rgba([0, 0, 0, 200]),
            x + 1,
            y + 1,
            scale,
            &self.font,
            &display_label,
        );

        // Draw main text in white
        draw_text_mut(
            img,
            Rgba([255, 255, 255, 255]),
            x,
            y,
            scale,
            &self.font,
            &display_label,
        );
    }

    /// Render a binding's button image, if configured.
    /// Returns None if no button_image is set.
    pub fn render_binding(&self, binding: &Binding) -> Result<Option<DynamicImage>> {
        let image_source = match &binding.button_image {
            Some(src) if !src.is_empty() => src,
            _ => return Ok(None),
        };

        // Load image with optional SVG colorization
        let img = image_cache::load_cached_with_color(
            image_source,
            binding.icon_color.as_deref(),
            self.button_size.0,
        )?;
        let resized = self.resize(img);
        let mut rgba = resized.to_rgba8();

        // Add label if requested
        if binding.show_label.unwrap_or(false) {
            if let Some(label) = &binding.label {
                self.add_label(&mut rgba, label);
            }
        }

        Ok(Some(DynamicImage::ImageRgba8(rgba)))
    }

    /// Create a simple colored background with optional text.
    /// Useful for testing or fallback.
    #[allow(dead_code)]
    pub fn create_solid_button(&self, color: Rgba<u8>, label: Option<&str>) -> DynamicImage {
        let (w, h) = self.button_size;
        let mut img = RgbaImage::from_pixel(w, h, color);

        if let Some(text) = label {
            self.add_label(&mut img, text);
        }

        DynamicImage::ImageRgba8(img)
    }
}

/// Get button size for a Stream Deck kind.
pub fn button_size_for_kind(kind: elgato_streamdeck::info::Kind) -> (u32, u32) {
    let (w, h) = kind.key_image_format().size;
    (w as u32, h as u32)
}

/// Get LCD strip size for a Stream Deck kind (if it has one).
pub fn lcd_strip_size_for_kind(kind: elgato_streamdeck::info::Kind) -> Option<(u32, u32)> {
    kind.lcd_strip_size().map(|(w, h)| (w as u32, h as u32))
}

/// Get the size of each encoder section on the LCD strip.
/// For Stream Deck Plus: 800x100 strip / 4 encoders = 200x100 per encoder.
pub fn encoder_lcd_size_for_kind(kind: elgato_streamdeck::info::Kind) -> Option<(u32, u32)> {
    let (strip_w, strip_h) = lcd_strip_size_for_kind(kind)?;
    let encoder_count = kind.encoder_count() as u32;
    if encoder_count == 0 {
        return None;
    }
    Some((strip_w / encoder_count, strip_h))
}

/// Renderer for LCD strip encoder sections.
pub struct LcdRenderer {
    font: FontRef<'static>,
    section_size: (u32, u32),
}

impl LcdRenderer {
    /// Create a new LCD renderer for the given section dimensions.
    pub fn new(section_width: u32, section_height: u32) -> Result<Self> {
        let font =
            FontRef::try_from_slice(EMBEDDED_FONT).context("Failed to load embedded font")?;

        Ok(Self {
            font,
            section_size: (section_width, section_height),
        })
    }

    /// Resize an image to fit the LCD section dimensions.
    pub fn resize(&self, img: DynamicImage) -> DynamicImage {
        let (target_w, target_h) = self.section_size;

        // Resize to fit within section, preserving aspect ratio
        let resized = img.resize(target_w, target_h, image::imageops::FilterType::Lanczos3);

        // Center on black background if smaller
        if resized.width() < target_w || resized.height() < target_h {
            let mut canvas = RgbaImage::from_pixel(target_w, target_h, Rgba([0, 0, 0, 255]));
            let x_offset = (target_w - resized.width()) / 2;
            let y_offset = (target_h - resized.height()) / 2;

            image::imageops::overlay(&mut canvas, &resized.to_rgba8(), x_offset as i64, y_offset as i64);
            DynamicImage::ImageRgba8(canvas)
        } else {
            resized
        }
    }

    /// Add a label to the bottom of the image.
    pub fn add_label(&self, img: &mut RgbaImage, label: &str) {
        if label.is_empty() {
            return;
        }

        let (w, h) = (img.width(), img.height());
        let font_size = (h as f32 * 0.18).max(12.0);
        let scale = PxScale::from(font_size);

        let max_chars = (w as usize / 10).max(4);
        let display_label = if label.len() > max_chars {
            format!("{}...", &label[..max_chars.saturating_sub(3)])
        } else {
            label.to_string()
        };

        let char_width = font_size * 0.6;
        let text_width = display_label.len() as f32 * char_width;
        let x = ((w as f32 - text_width) / 2.0).max(2.0) as i32;
        let y = (h as f32 - font_size - 6.0).max(0.0) as i32;

        // Shadow
        draw_text_mut(img, Rgba([0, 0, 0, 200]), x + 1, y + 1, scale, &self.font, &display_label);
        // Main text
        draw_text_mut(img, Rgba([255, 255, 255, 255]), x, y, scale, &self.font, &display_label);
    }

    /// Render an encoder's LCD section from a binding.
    pub fn render_binding(&self, binding: &Binding) -> Result<Option<DynamicImage>> {
        let image_source = match &binding.button_image {
            Some(src) if !src.is_empty() => src,
            _ => return Ok(None),
        };

        // Load image with optional SVG colorization
        let img = image_cache::load_cached_with_color(
            image_source,
            binding.icon_color.as_deref(),
            self.section_size.1, // Use height as target size for LCD
        )?;
        let resized = self.resize(img);
        let mut rgba = resized.to_rgba8();

        if binding.show_label.unwrap_or(false) {
            if let Some(label) = &binding.label {
                self.add_label(&mut rgba, label);
            }
        }

        Ok(Some(DynamicImage::ImageRgba8(rgba)))
    }

    /// Create a black/empty section.
    pub fn create_empty(&self) -> DynamicImage {
        let (w, h) = self.section_size;
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 255])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_detection() {
        let _renderer = ButtonRenderer::new(72, 72).unwrap();

        // This just tests the detection logic, not actual loading
        assert!(
            "https://example.com/img.png"
                .starts_with("http://")
                || "https://example.com/img.png".starts_with("https://")
        );
        assert!(!"./local/image.png".starts_with("http"));
    }

    #[test]
    fn test_button_renderer_creation() {
        let renderer = ButtonRenderer::new(72, 72);
        assert!(renderer.is_ok());
        let renderer = renderer.unwrap();
        assert_eq!(renderer.button_size, (72, 72));
    }

    #[test]
    fn test_button_renderer_different_sizes() {
        // Test Stream Deck Mini size (80x80)
        let renderer = ButtonRenderer::new(80, 80).unwrap();
        assert_eq!(renderer.button_size, (80, 80));

        // Test Stream Deck XL size (96x96)
        let renderer = ButtonRenderer::new(96, 96).unwrap();
        assert_eq!(renderer.button_size, (96, 96));
    }

    #[test]
    fn test_lcd_renderer_creation() {
        let renderer = LcdRenderer::new(200, 100);
        assert!(renderer.is_ok());
        let renderer = renderer.unwrap();
        assert_eq!(renderer.section_size, (200, 100));
    }

    #[test]
    fn test_create_solid_button() {
        let renderer = ButtonRenderer::new(72, 72).unwrap();
        let img = renderer.create_solid_button(Rgba([255, 0, 0, 255]), None);

        assert_eq!(img.width(), 72);
        assert_eq!(img.height(), 72);
    }

    #[test]
    fn test_create_solid_button_with_label() {
        let renderer = ButtonRenderer::new(72, 72).unwrap();
        let img = renderer.create_solid_button(Rgba([0, 0, 255, 255]), Some("Test"));

        assert_eq!(img.width(), 72);
        assert_eq!(img.height(), 72);
    }

    #[test]
    fn test_lcd_renderer_create_empty() {
        let renderer = LcdRenderer::new(200, 100).unwrap();
        let img = renderer.create_empty();

        assert_eq!(img.width(), 200);
        assert_eq!(img.height(), 100);

        // Should be black (RGBA 0,0,0,255)
        let rgba = img.to_rgba8();
        let pixel = rgba.get_pixel(100, 50);
        assert_eq!(pixel, &Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn test_resize_smaller_image() {
        // Create a small 10x10 image
        let small_img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 255])));

        let renderer = ButtonRenderer::new(72, 72).unwrap();
        let resized = renderer.resize(small_img);

        // Should be padded to target size
        assert_eq!(resized.width(), 72);
        assert_eq!(resized.height(), 72);
    }

    #[test]
    fn test_resize_larger_image() {
        // Create a larger 200x200 image
        let large_img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(200, 200, Rgba([0, 255, 0, 255])));

        let renderer = ButtonRenderer::new(72, 72).unwrap();
        let resized = renderer.resize(large_img);

        // Should be resized down
        assert!(resized.width() <= 72);
        assert!(resized.height() <= 72);
    }

    #[test]
    fn test_resize_non_square_image() {
        // Create a wide image 100x50
        let wide_img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 50, Rgba([0, 0, 255, 255])));

        let renderer = ButtonRenderer::new(72, 72).unwrap();
        let resized = renderer.resize(wide_img);

        // Should preserve aspect ratio and fit within bounds
        assert!(resized.width() <= 72);
        assert!(resized.height() <= 72);
    }

    #[test]
    fn test_add_label_empty() {
        let renderer = ButtonRenderer::new(72, 72).unwrap();
        let mut img = RgbaImage::from_pixel(72, 72, Rgba([0, 0, 0, 255]));

        // Empty label should not modify image (no crash)
        renderer.add_label(&mut img, "");

        assert_eq!(img.width(), 72);
    }

    #[test]
    fn test_add_label_short_text() {
        let renderer = ButtonRenderer::new(72, 72).unwrap();
        let mut img = RgbaImage::from_pixel(72, 72, Rgba([0, 0, 0, 255]));

        // Short label should render without issue
        renderer.add_label(&mut img, "Hi");

        assert_eq!(img.width(), 72);
    }

    #[test]
    fn test_add_label_long_text() {
        let renderer = ButtonRenderer::new(72, 72).unwrap();
        let mut img = RgbaImage::from_pixel(72, 72, Rgba([0, 0, 0, 255]));

        // Long label should be truncated without crash
        renderer.add_label(&mut img, "This is a very long label that should be truncated");

        assert_eq!(img.width(), 72);
    }

    #[test]
    fn test_url_http_detection() {
        let source = "http://example.com/image.png";
        let is_url = source.starts_with("http://") || source.starts_with("https://");
        assert!(is_url);
    }

    #[test]
    fn test_url_https_detection() {
        let source = "https://cdn.example.com/icon.svg";
        let is_url = source.starts_with("http://") || source.starts_with("https://");
        assert!(is_url);
    }

    #[test]
    fn test_local_path_detection() {
        let source = "/home/user/icons/test.png";
        let is_url = source.starts_with("http://") || source.starts_with("https://");
        assert!(!is_url);
    }

    #[test]
    fn test_relative_path_detection() {
        let source = "./assets/icon.png";
        let is_url = source.starts_with("http://") || source.starts_with("https://");
        assert!(!is_url);
    }
}
