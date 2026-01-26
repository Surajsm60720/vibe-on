use std::path::Path;
use std::process::{Command, Stdio};

use super::types::{AudioFeatures, EssentiaStatus, ANALYSIS_VERSION};

/// Path to the Python sidecar script relative to app resources
const SIDECAR_SCRIPT: &str = "sidecar/analyze_track.py";

/// Analyzer that spawns Python sidecar for Essentia analysis
pub struct AudioAnalyzer {
    sidecar_path: std::path::PathBuf,
}

impl AudioAnalyzer {
    /// Create analyzer with path to sidecar script
    /// Searches multiple locations to work in both dev and production
    pub fn new(resources_dir: &Path) -> Self {
        // Try multiple possible sidecar locations
        let possible_paths = [
            // Production: relative to executable
            resources_dir.join(SIDECAR_SCRIPT),
            // Dev mode: in src-tauri/sidecar
            resources_dir.join("../sidecar/analyze_track.py"),
            // Dev mode: look up from target/debug
            resources_dir.join("../../sidecar/analyze_track.py"),
            // Absolute fallback for development
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sidecar/analyze_track.py"),
        ];

        let sidecar_path = possible_paths
            .into_iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| resources_dir.join(SIDECAR_SCRIPT));

        Self { sidecar_path }
    }

    /// Check if Python and Essentia are available
    pub fn check_availability(&self) -> EssentiaStatus {
        // Check Python version
        let python_result = Command::new("python3.11").args(["--version"]).output();

        let python_version = match python_result {
            Ok(output) if output.status.success() => {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            _ => None,
        };

        if python_version.is_none() {
            return EssentiaStatus {
                available: false,
                python_version: None,
                essentia_version: None,
                error: Some("Python 3.11 not found. Run: brew install python@3.11".to_string()),
            };
        }

        // Check Essentia import
        let essentia_result = Command::new("python3.11")
            .args(["-c", "import essentia; print(essentia.__version__)"])
            .output();

        let essentia_version = match essentia_result {
            Ok(output) if output.status.success() => {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return EssentiaStatus {
                    available: false,
                    python_version,
                    essentia_version: None,
                    error: Some(format!(
                        "Essentia not installed. Run: pip3.11 install --user essentia\nError: {}",
                        stderr.lines().next().unwrap_or("Unknown error")
                    )),
                };
            }
            Err(e) => {
                return EssentiaStatus {
                    available: false,
                    python_version,
                    essentia_version: None,
                    error: Some(format!("Failed to check Essentia: {}", e)),
                };
            }
        };

        // Check sidecar script exists
        if !self.sidecar_path.exists() {
            return EssentiaStatus {
                available: false,
                python_version,
                essentia_version,
                error: Some(format!(
                    "Sidecar script not found at: {}",
                    self.sidecar_path.display()
                )),
            };
        }

        EssentiaStatus {
            available: true,
            python_version,
            essentia_version,
            error: None,
        }
    }

    /// Analyze a single audio file
    /// Returns AudioFeatures on success, or error string
    pub fn analyze_track(&self, audio_path: &str) -> Result<AudioFeatures, String> {
        // Verify file exists
        if !Path::new(audio_path).exists() {
            return Err(format!("Audio file not found: {}", audio_path));
        }

        // Spawn Python sidecar
        let output = Command::new("python3.11")
            .arg(&self.sidecar_path)
            .arg(audio_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Failed to spawn analyzer: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let code = output.status.code().unwrap_or(-1);
            return Err(format!(
                "Analysis failed (code {}): {}",
                code,
                stderr.trim()
            ));
        }

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut features: AudioFeatures = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse analyzer output: {}\nOutput: {}", e, stdout))?;

        // Ensure version is set
        features.analysis_version = ANALYSIS_VERSION;
        features.analysis_error = None;

        Ok(features)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_availability() {
        let analyzer = AudioAnalyzer::new(Path::new("."));
        let status = analyzer.check_availability();
        // This will fail in CI without Python, but useful for local testing
        println!("Essentia status: {:?}", status);
    }
}
