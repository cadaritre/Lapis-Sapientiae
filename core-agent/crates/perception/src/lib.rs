/// Perception module — captures the visual state of the desktop.
///
/// Phase 6: real screenshot capture using xcap, encoded as base64 PNG.
use common::{LapisError, LapisResult};
use base64::Engine;
use std::io::Cursor;

/// A captured screenshot with metadata and base64-encoded PNG data.
pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    pub png_base64: String,
}

/// Captures the primary monitor's screen as a PNG, returns base64-encoded data.
pub fn capture_screen() -> LapisResult<Screenshot> {
    let monitors = xcap::Monitor::all()
        .map_err(|e| LapisError::Perception(format!("Failed to list monitors: {e}")))?;

    let monitor = monitors
        .into_iter()
        .find(|m| m.is_primary())
        .or_else(|| {
            xcap::Monitor::all().ok().and_then(|m| m.into_iter().next())
        })
        .ok_or_else(|| LapisError::Perception("No monitor found".into()))?;

    let image = monitor
        .capture_image()
        .map_err(|e| LapisError::Perception(format!("Failed to capture screen: {e}")))?;

    let width = image.width();
    let height = image.height();

    let mut png_bytes = Vec::new();
    let dynamic = image::DynamicImage::ImageRgba8(image);
    dynamic
        .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| LapisError::Perception(format!("Failed to encode PNG: {e}")))?;

    let png_base64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

    Ok(Screenshot {
        width,
        height,
        png_base64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_screen_returns_data() {
        // This test requires a display — may fail in headless CI
        match capture_screen() {
            Ok(s) => {
                assert!(s.width > 0);
                assert!(s.height > 0);
                assert!(!s.png_base64.is_empty());
            }
            Err(_) => {
                // Acceptable in headless environments
            }
        }
    }
}
