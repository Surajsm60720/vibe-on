use std::path::Path;
use std::process::{Command, Stdio};

use super::rust_analyzer;
use super::types::{AudioFeatures, EssentiaStatus, ANALYSIS_VERSION};

/// Path to the Python sidecar script relative to app resources
const SIDECAR_SCRIPT: &str = "sidecar/analyze_track.py";

/// Analyzer that tries Rust DSP first, falls back to Python Essentia
pub struct AudioAnalyzer {
    sidecar_path: std::path::PathBuf,
    prefer_python: bool, // Force Python if explicitly requested
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

        Self {
            sidecar_path,
            prefer_python: false,
        }
    }

    /// Create analyzer with Python preference
    pub fn new_prefer_python(resources_dir: &Path) -> Self {
        let mut analyzer = Self::new(resources_dir);
        analyzer.prefer_python = true;
        analyzer
    }

    /// Get the Python command to use
    fn get_python_command() -> Command {
        // Try to find python3.11 in PATH first
        if Command::new("python3.11").arg("--version").output().is_ok() {
            return Command::new("python3.11");
        }

        // Fallback to searching common Homebrew paths on macOS
        let paths = [
            "/opt/homebrew/bin/python3.11",
            "/usr/local/bin/python3.11",
            "/usr/bin/python3.11",
        ];

        for path in paths {
            if Path::new(path).exists() {
                return Command::new(path);
            }
        }

        // Final fallback
        Command::new("python3.11")
    }

    /// Check if Python and Essentia are available
    pub fn check_availability(&self) -> EssentiaStatus {
        // Check Python version
        let python_result = Self::get_python_command().args(["--version"]).output();

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
        let essentia_result = Self::get_python_command()
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
    /// 1. Try Rust analyzer first (always available, works offline)
    /// 2. If prefer_python is set, try Python/Essentia
    /// 3. Return whichever succeeds
    pub fn analyze_track(&self, audio_path: &str) -> Result<AudioFeatures, String> {
        // Verify file exists
        if !Path::new(audio_path).exists() {
            return Err(format!("Audio file not found: {}", audio_path));
        }

        // Strategy 1: Try Python/Essentia first (High Accuracy)
        let python_status = self.check_availability();
        if python_status.available {
            println!(
                "[Analyzer] Attempting Python/Essentia analysis for: {}",
                audio_path
            );
            match self.analyze_track_python(audio_path) {
                Ok(mut features) => {
                    println!("[Analyzer] Python analysis succeeded");
                    features.analysis_backend = Some("essentia".to_string());
                    return Ok(features);
                }
                Err(python_err) => {
                    println!(
                        "[Analyzer] Python analysis failed: {}. Falling back to Rust.",
                        python_err
                    );
                    // Fallthrough to Rust
                }
            }
        } else {
            println!(
                "[Analyzer] Python/Essentia not available ({:?}). Using Rust fallback.",
                python_status.error
            );
        }

        // Strategy 2: Rust Analyzer (Fallback)
        println!(
            "[Analyzer] Attempting Rust analysis (Fallback) for: {}",
            audio_path
        );
        match rust_analyzer::analyze_audio_file_rust(audio_path) {
            Ok(mut features) => {
                println!("[Analyzer] Rust analysis succeeded");
                features.analysis_backend = Some("rust".to_string());
                Ok(features)
            }
            Err(rust_err) => {
                println!("[Analyzer] Rust analysis failed: {}", rust_err);
                Err(format!("All analyzers failed. Rust error: {}", rust_err))
            }
        }
    }

    /// Analyze using Python sidecar (Essentia)
    fn analyze_track_python(&self, audio_path: &str) -> Result<AudioFeatures, String> {
        // Spawn Python sidecar
        let output = Self::get_python_command()
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
