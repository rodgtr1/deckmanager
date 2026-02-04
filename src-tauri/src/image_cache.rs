//! Simple in-memory image cache with LRU eviction.
//!
//! Caches loaded images to avoid re-fetching URLs on every sync.
//! Local files are also cached but can be invalidated if modified.

use anyhow::{Context, Result};
use image::{DynamicImage, RgbaImage};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

/// Cache entry with metadata
struct CacheEntry {
    image: DynamicImage,
    /// When this entry was cached
    cached_at: Instant,
    /// For local files: modification time when cached
    file_mtime: Option<SystemTime>,
}

/// Global image cache
static IMAGE_CACHE: Mutex<Option<ImageCache>> = Mutex::new(None);

/// Maximum age for URL cache entries (5 minutes)
const URL_CACHE_TTL: Duration = Duration::from_secs(300);

/// Maximum number of entries in the cache (prevents unbounded growth)
const MAX_CACHE_ENTRIES: usize = 100;

/// Image cache implementation using O(1) LRU eviction
struct ImageCache {
    entries: LruCache<String, CacheEntry>,
}

impl ImageCache {
    fn new() -> Self {
        Self {
            entries: LruCache::new(NonZeroUsize::new(MAX_CACHE_ENTRIES).unwrap()),
        }
    }

    /// Get a cached image if valid, or None if not cached/expired
    fn get(&mut self, source: &str) -> Option<DynamicImage> {
        // Use peek to check validity first without promoting
        let entry = self.entries.peek(source)?;

        // Check if entry is still valid
        let is_url = source.starts_with("http://") || source.starts_with("https://");
        if is_url {
            // URL: check TTL
            if entry.cached_at.elapsed() > URL_CACHE_TTL {
                self.entries.pop(source);
                return None;
            }
        } else {
            // Local file: check if modified
            if let Some(cached_mtime) = entry.file_mtime {
                if let Ok(metadata) = std::fs::metadata(source) {
                    if let Ok(current_mtime) = metadata.modified() {
                        if current_mtime != cached_mtime {
                            self.entries.pop(source);
                            return None; // File was modified
                        }
                    }
                }
            }
        }

        // Entry is valid, get it (which promotes it in LRU order)
        self.entries.get(source).map(|e| e.image.clone())
    }

    /// Store an image in the cache (LRU eviction happens automatically)
    fn put(&mut self, source: &str, image: DynamicImage) {
        let file_mtime = if !source.starts_with("http://") && !source.starts_with("https://") {
            // Get file modification time for local files
            std::fs::metadata(source)
                .ok()
                .and_then(|m| m.modified().ok())
        } else {
            None
        };

        self.entries.put(
            source.to_string(),
            CacheEntry {
                image,
                cached_at: Instant::now(),
                file_mtime,
            },
        );
    }

    /// Clear all cached entries
    #[allow(dead_code)]
    fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Get or initialize the global cache
fn with_cache<F, R>(f: F) -> R
where
    F: FnOnce(&mut ImageCache) -> R,
{
    let mut guard = IMAGE_CACHE.lock().unwrap();
    let cache = guard.get_or_insert_with(ImageCache::new);
    f(cache)
}

/// Load an image without caching (internal use)
fn load_image_uncached(source: &str) -> Result<DynamicImage> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let response = reqwest::blocking::get(source).context("Failed to fetch image from URL")?;
        let bytes = response.bytes().context("Failed to read image bytes")?;
        image::load_from_memory(&bytes).context("Failed to decode image from URL")
    } else {
        image::open(Path::new(source)).context(format!("Failed to load image: {}", source))
    }
}

/// Check if a source is an SVG file
fn is_svg(source: &str) -> bool {
    source.to_lowercase().ends_with(".svg")
        || (source.contains("/icons/") && source.contains("lucide"))
}

/// Colorize SVG text by replacing stroke colors
fn colorize_svg_text(svg_text: &str, color: &str) -> String {
    let mut result = svg_text.to_string();

    // Replace stroke and fill colors
    result = result.replace("stroke=\"currentColor\"", &format!("stroke=\"{}\"", color));
    result = result.replace("fill=\"currentColor\"", &format!("fill=\"{}\"", color));
    result = result.replace("stroke=\"#000000\"", &format!("stroke=\"{}\"", color));
    result = result.replace("stroke=\"#000\"", &format!("stroke=\"{}\"", color));
    result = result.replace("stroke=\"black\"", &format!("stroke=\"{}\"", color));

    // Add stroke to svg element if not present
    if !result.contains("stroke=\"") {
        result = result.replacen("<svg", &format!("<svg stroke=\"{}\"", color), 1);
    }

    result
}

/// Load and render an SVG with optional colorization
fn load_svg(source: &str, color: Option<&str>, target_size: u32) -> Result<DynamicImage> {
    // Fetch the SVG content
    let svg_text = if source.starts_with("http://") || source.starts_with("https://") {
        let response = reqwest::blocking::get(source).context("Failed to fetch SVG from URL")?;
        response.text().context("Failed to read SVG text")?
    } else {
        std::fs::read_to_string(source).context(format!("Failed to read SVG file: {}", source))?
    };

    // Colorize if color is specified
    let svg_text = match color {
        Some(c) => colorize_svg_text(&svg_text, c),
        None => svg_text,
    };

    // Parse and render SVG
    let opts = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&svg_text, &opts)
        .context("Failed to parse SVG")?;

    // Create a pixmap for rendering
    let size = resvg::tiny_skia::IntSize::from_wh(target_size, target_size)
        .context("Invalid target size")?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .context("Failed to create pixmap")?;

    // Calculate scale to fit SVG in target size
    let svg_size = tree.size();
    let scale = (target_size as f32 / svg_size.width()).min(target_size as f32 / svg_size.height());

    // Center the SVG
    let x_offset = (target_size as f32 - svg_size.width() * scale) / 2.0;
    let y_offset = (target_size as f32 - svg_size.height() * scale) / 2.0;

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale)
        .post_translate(x_offset, y_offset);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert to image::DynamicImage
    let width = pixmap.width();
    let height = pixmap.height();
    let data = pixmap.take();

    // tiny_skia uses premultiplied alpha, convert to straight alpha
    let rgba_data: Vec<u8> = data
        .chunks(4)
        .flat_map(|pixel| {
            let a = pixel[3] as f32 / 255.0;
            if a > 0.0 {
                [
                    (pixel[0] as f32 / a).min(255.0) as u8,
                    (pixel[1] as f32 / a).min(255.0) as u8,
                    (pixel[2] as f32 / a).min(255.0) as u8,
                    pixel[3],
                ]
            } else {
                [0, 0, 0, 0]
            }
        })
        .collect();

    let img = RgbaImage::from_raw(width, height, rgba_data)
        .context("Failed to create image from SVG render")?;

    Ok(DynamicImage::ImageRgba8(img))
}

/// Load an image with optional SVG colorization (with caching).
/// For SVG files, the color parameter is used to colorize the icon.
/// All images are resized to fit within target_size while preserving aspect ratio.
pub fn load_cached_with_color(source: &str, color: Option<&str>, target_size: u32) -> Result<DynamicImage> {
    // Create cache key including color and size
    let cache_key = match color {
        Some(c) => format!("{}@{}@{}", source, c, target_size),
        None => format!("{}@{}", source, target_size),
    };

    // Check cache first
    if let Some(img) = with_cache(|c| c.get(&cache_key)) {
        return Ok(img);
    }

    // Load the image
    let img = if is_svg(source) {
        load_svg(source, color, target_size)?
    } else {
        let loaded = load_image_uncached(source)?;
        // Resize non-SVG images to fit within target_size (preserving aspect ratio)
        if loaded.width() > target_size || loaded.height() > target_size {
            loaded.resize(target_size, target_size, image::imageops::FilterType::Lanczos3)
        } else {
            loaded
        }
    };

    // Cache it
    with_cache(|c| c.put(&cache_key, img.clone()));

    Ok(img)
}

/// Clear the image cache (e.g., when bindings change significantly)
#[allow(dead_code)]
pub fn clear_cache() {
    with_cache(|c| c.clear());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_detection() {
        let http_url = "http://example.com/image.png";
        let https_url = "https://example.com/image.png";
        let local_path = "/home/user/image.png";

        assert!(http_url.starts_with("http://") || http_url.starts_with("https://"));
        assert!(https_url.starts_with("http://") || https_url.starts_with("https://"));
        assert!(!local_path.starts_with("http://") && !local_path.starts_with("https://"));
    }

    #[test]
    fn test_cache_ttl_constant() {
        // URL cache should be reasonable (1-10 minutes)
        assert!(URL_CACHE_TTL >= Duration::from_secs(60));
        assert!(URL_CACHE_TTL <= Duration::from_secs(600));
    }

    #[test]
    fn test_cache_entry_creation() {
        let now = Instant::now();
        let cache_entry = CacheEntry {
            image: DynamicImage::new_rgba8(1, 1),
            cached_at: now,
            file_mtime: None,
        };

        assert!(cache_entry.cached_at.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn test_image_cache_new() {
        let cache = ImageCache::new();
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn test_cache_put_and_get() {
        let mut cache = ImageCache::new();
        let img = DynamicImage::new_rgba8(10, 10);

        cache.put("test_key", img.clone());

        let retrieved = cache.get("test_key");
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = ImageCache::new();
        let retrieved = cache.get("nonexistent_key");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = ImageCache::new();
        cache.put("key1", DynamicImage::new_rgba8(1, 1));
        cache.put("key2", DynamicImage::new_rgba8(1, 1));

        assert_eq!(cache.entries.len(), 2);

        cache.clear();

        assert!(cache.entries.is_empty());
    }

    #[test]
    fn test_max_cache_entries_constant() {
        // Max cache entries should be reasonable (50-500)
        assert!(MAX_CACHE_ENTRIES >= 50);
        assert!(MAX_CACHE_ENTRIES <= 500);
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = ImageCache::new();

        // Fill cache to max
        for i in 0..MAX_CACHE_ENTRIES {
            cache.put(&format!("key{}", i), DynamicImage::new_rgba8(1, 1));
        }

        assert_eq!(cache.entries.len(), MAX_CACHE_ENTRIES);

        // Access first entry to make it recently used
        let _ = cache.get("key0");

        // Add one more - should evict the least recently used (key1, not key0)
        cache.put("new_key", DynamicImage::new_rgba8(1, 1));

        // Should still be at max
        assert_eq!(cache.entries.len(), MAX_CACHE_ENTRIES);

        // key0 should still be there since we accessed it
        assert!(cache.get("key0").is_some());
        // key1 should have been evicted (it was the LRU after we accessed key0)
        assert!(cache.entries.peek(&"key1".to_string()).is_none());
        // new_key should be there
        assert!(cache.get("new_key").is_some());
    }
}
