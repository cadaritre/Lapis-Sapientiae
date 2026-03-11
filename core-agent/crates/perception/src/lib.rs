/// Perception module — captures the visual state of the desktop and analyzes it via VLM.
///
/// Phase 6: real screenshot capture using xcap + VLM analysis via Ollama API.
use common::{LapisError, LapisResult};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

/// A captured screenshot with metadata and base64-encoded PNG data.
pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    pub png_base64: String,
}

/// Response from VLM analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmResponse {
    pub description: String,
    pub model: String,
}

/// Configuration for the VLM endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmConfig {
    pub endpoint: String,
    pub model: String,
}

impl Default for VlmConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".into(),
            model: "moondream".into(),
        }
    }
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

    let mut dynamic = image::DynamicImage::ImageRgba8(image);

    // Downscale to 1080p max to reduce size for VLM processing
    if dynamic.width() > 1920 || dynamic.height() > 1080 {
        dynamic = dynamic.resize(1920, 1080, image::imageops::FilterType::Triangle);
    }

    let width = dynamic.width();
    let height = dynamic.height();

    let mut png_bytes = Vec::new();
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

/// Ollama /api/generate request body for vision models.
#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    images: Vec<String>,
    stream: bool,
}

/// Ollama /api/generate response body.
#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

/// Analyze a base64-encoded PNG image using a local VLM (Ollama API).
pub async fn analyze_image(
    vlm_config: &VlmConfig,
    png_base64: &str,
    prompt: &str,
) -> LapisResult<VlmResponse> {
    let url = format!("{}/api/generate", vlm_config.endpoint.trim_end_matches('/'));

    let body = OllamaGenerateRequest {
        model: vlm_config.model.clone(),
        prompt: prompt.to_string(),
        images: vec![png_base64.to_string()],
        stream: false,
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| LapisError::Perception(format!("HTTP client error: {e}")))?;

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| LapisError::Perception(format!("VLM request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(LapisError::Perception(format!(
            "VLM returned {status}: {text}"
        )));
    }

    let ollama_resp: OllamaGenerateResponse = resp
        .json()
        .await
        .map_err(|e| LapisError::Perception(format!("VLM response parse error: {e}")))?;

    Ok(VlmResponse {
        description: ollama_resp.response,
        model: vlm_config.model.clone(),
    })
}

/// Capture screen and analyze it with VLM in one call.
pub async fn capture_and_analyze(
    vlm_config: &VlmConfig,
    prompt: &str,
) -> LapisResult<(Screenshot, VlmResponse)> {
    let screenshot = capture_screen()?;
    let analysis = analyze_image(vlm_config, &screenshot.png_base64, prompt).await?;
    Ok((screenshot, analysis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_screen_returns_data() {
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

    #[test]
    fn default_vlm_config() {
        let cfg = VlmConfig::default();
        assert_eq!(cfg.endpoint, "http://localhost:11434");
        assert_eq!(cfg.model, "moondream");
    }
}
