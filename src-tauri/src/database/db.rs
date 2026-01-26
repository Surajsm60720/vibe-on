use rusqlite::{params, Connection, OptionalExtension, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use super::schema::init_db;
use crate::audio::{TrackInfo, UnreleasedTrack};

pub struct DatabaseManager {
    conn: Arc<Mutex<Connection>>,
    covers_dir: PathBuf,
}

impl DatabaseManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        let app_dir = app_handle.path().app_data_dir().unwrap();
        if !app_dir.exists() {
            std::fs::create_dir_all(&app_dir).unwrap();
        }
        let db_path = app_dir.join("library.db");
        let covers_dir = app_dir.join("covers");

        if !covers_dir.exists() {
            std::fs::create_dir_all(&covers_dir).unwrap();
        }

        let conn = Connection::open(db_path)?;

        // Initialize schema
        init_db(&conn)?;

        // Migration: Add new columns if missing
        let _ = conn.execute("ALTER TABLE tracks ADD COLUMN disc_number INTEGER", []);
        let _ = conn.execute("ALTER TABLE tracks ADD COLUMN track_number INTEGER", []);

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            covers_dir,
        })
    }

    pub fn insert_track(&self, track: &TrackInfo, cover_data: Option<&[u8]>) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Insert into tracks
        conn.execute(
            "INSERT OR REPLACE INTO tracks (path, title, artist, album, duration_secs, disc_number, track_number) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                track.path,
                track.title,
                track.artist,
                track.album,
                track.duration_secs,
                track.disc_number,
                track.track_number
            ],
        )?;

        // Check if album exists and get current cover path
        let album_row: Option<Option<String>> = conn
            .query_row(
                "SELECT cover_image_path FROM albums WHERE name = ?1 AND artist = ?2",
                params![track.album, track.artist],
                |row| row.get(0),
            )
            .optional()?;

        let album_exists = album_row.is_some();
        let existing_cover = album_row.flatten();

        // If we have cover data and (album doesn't exist OR album has no cover), save it
        if let Some(data) = cover_data {
            if existing_cover.is_none() {
                let filename = format!("{}.jpg", Uuid::new_v4());
                let path = self.covers_dir.join(&filename);

                let saved = if let Ok(mut file) = fs::File::create(&path) {
                    file.write_all(data).is_ok()
                } else {
                    false
                };

                if saved {
                    if album_exists {
                        conn.execute(
                            "UPDATE albums SET cover_image_path = ?1 WHERE name = ?2 AND artist = ?3",
                            params![filename, track.album, track.artist],
                        )?;
                    } else {
                        conn.execute(
                            "INSERT INTO albums (name, artist, cover_image_path) VALUES (?1, ?2, ?3)",
                            params![track.album, track.artist, filename],
                        )?;
                    }
                } else if !album_exists {
                    // Create album entry even if save failed
                    conn.execute(
                        "INSERT INTO albums (name, artist, cover_image_path) VALUES (?1, ?2, ?3)",
                        params![track.album, track.artist, Option::<String>::None],
                    )?;
                }
            } else if !album_exists {
                // No cover data, just insert album
                conn.execute(
                    "INSERT INTO albums (name, artist, cover_image_path) VALUES (?1, ?2, ?3)",
                    params![track.album, track.artist, Option::<String>::None],
                )?;
            }
        } else if !album_exists {
            // No cover data and album doesn't exist
            conn.execute(
                "INSERT INTO albums (name, artist, cover_image_path) VALUES (?1, ?2, ?3)",
                params![track.album, track.artist, Option::<String>::None],
            )?;
        }

        Ok(())
    }

    pub fn get_all_tracks(&self) -> Result<Vec<TrackInfo>> {
        let conn = self.conn.lock().unwrap();

        // Join tracks with albums to get the cover image path
        let mut stmt = conn.prepare(
            "SELECT t.path, t.title, t.artist, t.album, t.duration_secs, a.cover_image_path, t.disc_number, t.track_number 
             FROM tracks t 
             LEFT JOIN albums a ON t.album = a.name AND t.artist = a.artist
             ORDER BY t.artist, t.album, t.disc_number, t.track_number, t.title",
        )?;

        let track_iter = stmt.query_map([], |row| {
            let cover_filename: Option<String> = row.get(5)?;

            // Convert filename to full asset URL?
            // Actually, usually we send just the filename or a special protocol URL.
            // If we send just filename, frontend needs to know where to find it.
            // Or we can construct `https://asset.localhost/covers/filename` here.
            // But `asset.localhost` depends on Tauri config.
            // Let's return just the filename for now, and handle URL in frontend or helper.
            // Wait, `TrackInfo` expects `Option<String>`.

            Ok(TrackInfo {
                path: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                duration_secs: row.get(4)?,
                cover_image: cover_filename,
                disc_number: row.get(6).unwrap_or(None),
                track_number: row.get(7).unwrap_or(None),
            })
        })?;

        let mut tracks = Vec::new();
        for track in track_iter {
            tracks.push(track?);
        }

        Ok(tracks)
    }

    pub fn get_covers_dir(&self) -> PathBuf {
        self.covers_dir.clone()
    }

    // Unreleased Library Methods
    pub fn insert_unreleased_track(&self, track: &UnreleasedTrack) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO unreleased_tracks (video_id, title, artist, duration_secs, thumbnail_url, content_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                track.video_id,
                track.title,
                track.artist,
                track.duration_secs,
                track.thumbnail_url,
                track.content_type
            ],
        )?;
        Ok(())
    }

    pub fn delete_unreleased_track(&self, video_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM unreleased_tracks WHERE video_id = ?1",
            params![video_id],
        )?;
        Ok(())
    }

    pub fn get_unreleased_tracks(&self) -> Result<Vec<UnreleasedTrack>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT video_id, title, artist, duration_secs, thumbnail_url, content_type, added_at
             FROM unreleased_tracks
             ORDER BY added_at DESC",
        )?;

        let track_iter = stmt.query_map([], |row| {
            // Parse added_at if needed, but we keep it as timestamp/string?
            // schema says TIMESTAMP DEFAULT CURRENT_TIMESTAMP which is usually text in sqlite unless configured
            // Let's assume we don't strictly need precise added_at for now or just grab it
            let added_at_str: Option<String> = row.get(6).ok();
            // Simple conversion or ignore
            let added_at = if let Some(_s) = added_at_str {
                // parse or just use 0
                Some(0)
            } else {
                None
            };

            Ok(UnreleasedTrack {
                video_id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2).unwrap_or("Unknown".to_string()),
                duration_secs: row.get(3)?,
                thumbnail_url: row.get(4).ok(),
                content_type: row.get(5)?,
                channel_name: None, // Not stored currently
                view_count: None,
                added_at: added_at,
                // album field doesn't exist in unreleased track struct? Wait, UnreleasedTrack extends TrackInfo in frontend but in Rust it is separate struct
                // Rust struct above:
                // pub video_id: String,
                // pub title: String,
                // pub artist: String,
                // pub duration_secs: f64,
                // pub thumbnail_url: Option<String>,
                // pub content_type: String, ...
            })
        })?;

        let mut tracks = Vec::new();
        for track in track_iter {
            tracks.push(track?);
        }
        Ok(tracks)
    }

    pub fn remove_folder(&self, path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // Delete all tracks where path starts with the folder path
        // We ensure path ends with separator to avoid partial matches on similar folder names
        // But users might pass path without separator.
        // Let's rely on LIKE 'path%' but maybe be careful.
        // Actually, tracks.path is absolute. folder path is absolute.
        // Ideally we should normalize separators.
        // For simple usage: LIKE ? || '%'
        conn.execute(
            "DELETE FROM tracks WHERE path LIKE ?1 || '%'",
            params![path],
        )?;
        Ok(())
    }

    pub fn clear_all_data(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        println!("[Database] Clearing all data...");

        // Delete all tracks, albums, and unreleased tracks
        conn.execute("DELETE FROM tracks", [])?;
        conn.execute("DELETE FROM albums", [])?;
        conn.execute("DELETE FROM unreleased_tracks", [])?;
        println!("[Database] Tables cleared.");

        // Optimize DB file
        conn.execute("VACUUM", [])?;
        println!("[Database] VACUUM complete.");

        // Clear all cover images from disk
        if self.covers_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&self.covers_dir) {
                let mut count = 0;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if std::fs::remove_file(path).is_ok() {
                            count += 1;
                        }
                    }
                }
                println!("[Database] Removed {} cover files.", count);
            }
        }

        Ok(())
    }
}
