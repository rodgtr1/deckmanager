//! Simple in-memory image cache with LRU eviction.
//!
//! Caches loaded images to avoid re-fetching URLs on every sync.
//! Local files are also cached but can be invalidated if modified.

use anyhow::{Context, Result};
use image::DynamicImage;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

/// Cache entry with metadata
struct CacheEntry {
    image: DynamicImage,
    /// When this entry was cached
    cached_at: Instant,
    /// When this entry was last accessed (for LRU eviction)
    last_accessed: Instant,
    /// For local files: modification time when cached
    file_mtime: Option<SystemTime>,
}

/// Global image cache
static IMAGE_CACHE: Mutex<Option<ImageCache>> = Mutex::new(None);

/// Maximum age for URL cache entries (5 minutes)
const URL_CACHE_TTL: Duration = Duration::from_secs(300);

/// Maximum number of entries in the cache (prevents unbounded growth)
const MAX_CACHE_ENTRIES: usize = 100;

/// Image cache implementation
struct ImageCache {
    entries: HashMap<String, CacheEntry>,
}

impl ImageCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get a cached image if valid, or None if not cached/expired
    fn get(&mut self, source: &str) -> Option<DynamicImage> {
        let entry = self.entries.get_mut(source)?;

        // Check if entry is still valid
        if source.starts_with("http://") || source.starts_with("https://") {
            // URL: check TTL
            if entry.cached_at.elapsed() > URL_CACHE_TTL {
                return None;
            }
        } else {
            // Local file: check if modified
            if let Some(cached_mtime) = entry.file_mtime {
                if let Ok(metadata) = std::fs::metadata(source) {
                    if let Ok(current_mtime) = metadata.modified() {
                        if current_mtime != cached_mtime {
                            return None; // File was modified
                        }
                    }
                }
            }
        }

        // Update last accessed time for LRU tracking
        entry.last_accessed = Instant::now();

        Some(entry.image.clone())
    }

    /// Store an image in the cache, evicting oldest entries if at capacity
    fn put(&mut self, source: &str, image: DynamicImage) {
        // Evict oldest entries if at capacity (LRU eviction)
        while self.entries.len() >= MAX_CACHE_ENTRIES {
            if let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.last_accessed)
                .map(|(k, _)| k.clone())
            {
                self.entries.remove(&oldest_key);
            } else {
                break;
            }
        }

        let file_mtime = if !source.starts_with("http://") && !source.starts_with("https://") {
            // Get file modification time for local files
            std::fs::metadata(source)
                .ok()
                .and_then(|m| m.modified().ok())
        } else {
            None
        };

        let now = Instant::now();
        self.entries.insert(
            source.to_string(),
            CacheEntry {
                image,
                cached_at: now,
                last_accessed: now,
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

/// Load an image with caching.
/// Returns cached image if available and valid, otherwise loads and caches.
pub fn load_cached(source: &str) -> Result<DynamicImage> {
    // Check cache first
    if let Some(img) = with_cache(|c| c.get(source)) {
        return Ok(img);
    }

    // Load the image
    let img = load_image_uncached(source)?;

    // Cache it
    with_cache(|c| c.put(source, img.clone()));

    Ok(img)
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
            last_accessed: now,
            file_mtime: None,
        };

        assert!(cache_entry.cached_at.elapsed() < Duration::from_secs(1));
        assert!(cache_entry.last_accessed.elapsed() < Duration::from_secs(1));
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

        // Fill cache to just under max
        for i in 0..(MAX_CACHE_ENTRIES - 1) {
            cache.put(&format!("key{}", i), DynamicImage::new_rgba8(1, 1));
        }

        assert_eq!(cache.entries.len(), MAX_CACHE_ENTRIES - 1);

        // Access first entry to make it recently used
        let _ = cache.get("key0");

        // Add one more to hit max
        cache.put("new_key", DynamicImage::new_rgba8(1, 1));

        // Should still be at max (no eviction needed yet since we had room)
        assert!(cache.entries.len() <= MAX_CACHE_ENTRIES);

        // Add another - should evict oldest (key1, not key0 since we accessed it)
        cache.put("another_key", DynamicImage::new_rgba8(1, 1));

        assert!(cache.entries.len() <= MAX_CACHE_ENTRIES);
        // key0 should still be there since we accessed it
        assert!(cache.get("key0").is_some());
    }
}
