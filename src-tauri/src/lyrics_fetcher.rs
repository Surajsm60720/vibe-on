use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};

/// LRCLIB API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResponse {
    pub id: Option<i64>,
    pub track_name: Option<String>,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    pub instrumental: Option<bool>,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
}

/// Try to find a local .lrc file next to the audio file
/// This is INSTANT and should be tried first
pub fn find_local_lrc(audio_path: &str) -> Option<LyricsResponse> {
    let path = Path::new(audio_path);

    // Try same name with .lrc extension
    let lrc_path = path.with_extension("lrc");
    if lrc_path.exists() {
        println!("[Lyrics] Found local LRC file: {:?}", lrc_path);
        if let Ok(content) = std::fs::read_to_string(&lrc_path) {
            // Check for .romaji.lrc or .trans.lrc
            let romaji_path = path.with_extension("romaji.lrc");
            let final_content = if romaji_path.exists() {
                println!("[Lyrics] Found local Romaji file: {:?}", romaji_path);
                if let Ok(romaji_content) = std::fs::read_to_string(&romaji_path) {
                    merge_lrc_content(&content, &romaji_content)
                } else {
                    content
                }
            } else {
                content
            };

            return Some(LyricsResponse {
                id: None,
                track_name: path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string()),
                artist_name: None,
                album_name: None,
                duration: None,
                instrumental: Some(false),
                plain_lyrics: None,
                synced_lyrics: Some(final_content),
            });
        }
    }

    // Try common LRC naming patterns in same directory
    if let Some(parent) = path.parent() {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            // Try variations: "song.lrc", "Song.lrc", "SONG.lrc"
            for name in &[format!("{}.lrc", stem), format!("{}.LRC", stem)] {
                let lrc_path = parent.join(name);
                if lrc_path.exists() {
                    println!("[Lyrics] Found local LRC file: {:?}", lrc_path);
                    if let Ok(content) = std::fs::read_to_string(&lrc_path) {
                        // Check for romaji variation
                        let romaji_name = format!("{}.romaji.lrc", stem);
                        let romaji_path = parent.join(romaji_name);

                        let final_content = if romaji_path.exists() {
                            println!("[Lyrics] Found local Romaji file: {:?}", romaji_path);
                            if let Ok(romaji_content) = std::fs::read_to_string(&romaji_path) {
                                merge_lrc_content(&content, &romaji_content)
                            } else {
                                content
                            }
                        } else {
                            content
                        };

                        return Some(LyricsResponse {
                            id: None,
                            track_name: Some(stem.to_string()),
                            artist_name: None,
                            album_name: None,
                            duration: None,
                            instrumental: Some(false),
                            plain_lyrics: None,
                            synced_lyrics: Some(final_content),
                        });
                    }
                }
            }
        }
    }

    None
}

/// Helper to merge main LRC with translation/romaji LRC
fn merge_lrc_content(main: &str, romaji: &str) -> String {
    use std::collections::BTreeMap;

    // Parse both into maps: Timestamp -> Text
    let parse = |s: &str| -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for line in s.lines() {
            if let Some(start) = line.find('[') {
                if let Some(end) = line.find(']') {
                    if end > start {
                        let timestamp = &line[start..=end]; // Keep brackets for simple matching
                        let text = line[end + 1..].trim();
                        if !text.is_empty() {
                            map.insert(timestamp.to_string(), text.to_string());
                        }
                    }
                }
            }
        }
        map
    };

    let main_map = parse(main);
    let romaji_map = parse(romaji);

    // Reconstruct with merger
    // We iterate over main_map to preserve original timing/order
    let mut result = String::new();

    // Basic heuristics: if main map is empty, just return romaji? No, return main.
    if main_map.is_empty() {
        return main.to_string();
    }

    // Re-read line by line to preserve headers/tags?
    // Simplified approach: Rebuild from map.
    // Better approach: Iterate lines of main, check if timestamp exists in romaji_map.

    for line in main.lines() {
        if let Some(start) = line.find('[') {
            if let Some(end) = line.find(']') {
                let timestamp = &line[start..=end];
                let text = line[end + 1..].trim();

                if let Some(romaji_text) = romaji_map.get(timestamp) {
                    // MERGE!
                    result.push_str(&format!("{} {} / {}\n", timestamp, text, romaji_text));
                } else {
                    result.push_str(&format!("{}\n", line));
                }
                continue;
            }
        }
        result.push_str(&format!("{}\n", line));
    }

    result
}
/// Extract first artist from comma/feat-separated list
fn extract_primary_artist(artist: &str) -> String {
    let separators = [
        ",", " feat ", " feat. ", " ft ", " ft. ", " & ", " x ", " and ", " with ",
    ];
    let mut result = artist.to_string();

    for sep in separators {
        if let Some(pos) = result.to_lowercase().find(sep) {
            result = result[..pos].to_string();
        }
    }
    result.trim().to_string()
}

/// Remove common suffixes like "(Official Audio)", "[Remastered]", etc.
fn clean_track_name(track: &str) -> String {
    let mut result = track.to_string();

    while let Some(start) = result.find('(') {
        if let Some(end) = result.find(')') {
            if end > start {
                result = format!("{}{}", &result[..start], &result[end + 1..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }

    while let Some(start) = result.find('[') {
        if let Some(end) = result.find(']') {
            if end > start {
                result = format!("{}{}", &result[..start], &result[end + 1..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }

    result.trim().to_string()
}

/// Create HTTP client
fn create_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("vibe-on/1.0 (https://github.com/vibe-on)")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

fn has_lyrics(resp: &LyricsResponse) -> bool {
    resp.synced_lyrics.is_some() || resp.plain_lyrics.is_some()
}

fn try_exact_match(
    client: &reqwest::blocking::Client,
    artist: &str,
    track: &str,
    duration_secs: u32,
) -> Option<LyricsResponse> {
    let url = format!(
        "https://lrclib.net/api/get?artist_name={}&track_name={}&duration={}",
        urlencoding::encode(artist),
        urlencoding::encode(track),
        duration_secs
    );
    println!("[Lyrics] → exact: {} - {}", artist, track);

    client
        .get(&url)
        .send()
        .ok()
        .and_then(|r| {
            if r.status().is_success() {
                r.json::<LyricsResponse>().ok()
            } else {
                None
            }
        })
        .filter(has_lyrics)
}

fn try_artist_track_search(
    client: &reqwest::blocking::Client,
    artist: &str,
    track: &str,
) -> Option<LyricsResponse> {
    let url = format!(
        "https://lrclib.net/api/search?artist_name={}&track_name={}",
        urlencoding::encode(artist),
        urlencoding::encode(track)
    );
    println!("[Lyrics] → search: {} - {}", artist, track);

    client
        .get(&url)
        .send()
        .ok()
        .and_then(|r| {
            if r.status().is_success() {
                r.json::<Vec<LyricsResponse>>().ok()
            } else {
                None
            }
        })
        .and_then(|results| {
            results
                .iter()
                .find(|r| r.synced_lyrics.is_some())
                .cloned()
                .or_else(|| results.iter().find(|r| r.plain_lyrics.is_some()).cloned())
        })
}

fn try_generic_search(client: &reqwest::blocking::Client, query: &str) -> Option<LyricsResponse> {
    let url = format!(
        "https://lrclib.net/api/search?q={}",
        urlencoding::encode(query)
    );
    println!("[Lyrics] → query: {}", query);

    client
        .get(&url)
        .send()
        .ok()
        .and_then(|r| {
            if r.status().is_success() {
                r.json::<Vec<LyricsResponse>>().ok()
            } else {
                None
            }
        })
        .and_then(|results| {
            results
                .iter()
                .find(|r| r.synced_lyrics.is_some())
                .cloned()
                .or_else(|| results.iter().find(|r| r.plain_lyrics.is_some()).cloned())
        })
}

/// Main function - LOCAL LRC FIRST, then API search
pub fn fetch_lyrics<F: Fn(&str)>(
    artist: &str,
    track: &str,
    duration_secs: u32,
    on_progress: F,
) -> Result<LyricsResponse, String> {
    println!("[Lyrics] Searching: {} - {}", artist, track);

    let start = Instant::now();
    let timeout = Duration::from_secs(10);
    let client = create_client()?;

    let clean_track = clean_track_name(track);
    let primary_artist = extract_primary_artist(artist);

    macro_rules! check_timeout {
        () => {
            if start.elapsed() > timeout {
                println!("[Lyrics] ✗ Timeout");
                return Err("Timeout".to_string());
            }
        };
    }

    // Strategy 1: Exact match
    on_progress("Searching exact match...");
    if let Some(lyrics) = try_exact_match(&client, artist, track, duration_secs) {
        println!("[Lyrics] ✓ Found exact match!");
        return Ok(lyrics);
    }
    check_timeout!();

    // Strategy 2: Clean track
    if clean_track != track {
        on_progress("Searching with cleaned track name...");
        if let Some(lyrics) = try_exact_match(&client, artist, &clean_track, duration_secs) {
            println!("[Lyrics] ✓ Found with clean track!");
            return Ok(lyrics);
        }
        check_timeout!();
    }

    // Strategy 3: Primary artist
    if primary_artist != artist {
        on_progress("Searching for primary artist...");
        if let Some(lyrics) = try_exact_match(&client, &primary_artist, track, duration_secs) {
            println!("[Lyrics] ✓ Found with primary artist!");
            return Ok(lyrics);
        }
        check_timeout!();
    }

    // Strategy 4: Search
    on_progress("Searching via LrcLib API...");
    if let Some(lyrics) = try_artist_track_search(&client, artist, track) {
        println!("[Lyrics] ✓ Found via search!");
        return Ok(lyrics);
    }
    check_timeout!();

    // Strategy 5: Clean search
    if clean_track != track || primary_artist != artist {
        on_progress("Retrying with cleaned metadata...");
        if let Some(lyrics) = try_artist_track_search(&client, &primary_artist, &clean_track) {
            println!("[Lyrics] ✓ Found via clean search!");
            return Ok(lyrics);
        }
        check_timeout!();
    }

    // Strategy 6: Generic query
    let query = format!("{} {}", artist, track);
    on_progress(&format!("Searching query: {}", query));
    if let Some(lyrics) = try_generic_search(&client, &query) {
        println!("[Lyrics] ✓ Found via generic!");
        return Ok(lyrics);
    }
    check_timeout!();

    // Strategy 7: Track only
    on_progress("Final attempt: searching by track name only...");
    if let Some(lyrics) = try_generic_search(&client, track) {
        println!("[Lyrics] ✓ Found via track only!");
        return Ok(lyrics);
    }

    println!("[Lyrics] ✗ Not found");
    Err("No sources founded for lyrics changing to recents view".to_string())
}

pub fn fetch_lyrics_fallback<F: Fn(&str)>(
    artist: &str,
    track: &str,
    on_progress: F,
) -> Result<LyricsResponse, String> {
    let client = create_client()?;

    on_progress("Fallback search: Artist + Track...");
    if let Some(lyrics) = try_artist_track_search(&client, artist, track) {
        return Ok(lyrics);
    }

    on_progress("Fallback search: Generic query...");
    let query = format!("{} {}", artist, track);
    if let Some(lyrics) = try_generic_search(&client, &query) {
        return Ok(lyrics);
    }

    Err("No sources founded for lyrics changing to recents view".to_string())
}
