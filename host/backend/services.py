import os
import hashlib
import json
import logging
import subprocess
from glob import glob
from typing import List, Dict, Optional
from fastapi import UploadFile
import mutagen

logger = logging.getLogger("vibe-on")

DATA_DIR = os.getenv("DATA_DIR", "./data")
MUSIC_DIR = os.getenv("MUSIC_DIR", "./music")
COVERS_DIR = os.getenv("COVERS_DIR", "./covers")
LIBRARY_FILE = os.path.join(DATA_DIR, "library.json")

# Ensure dirs
os.makedirs(DATA_DIR, exist_ok=True)
os.makedirs(COVERS_DIR, exist_ok=True)

class LibraryService:
    def __init__(self):
        self.tracks = []
        self.load_library()

    def load_library(self):
        if os.path.exists(LIBRARY_FILE):
            try:
                with open(LIBRARY_FILE, 'r') as f:
                    self.tracks = json.load(f)
            except Exception:
                self.tracks = []

    def save_library(self):
        with open(LIBRARY_FILE, 'w') as f:
            json.dump(self.tracks, f)

    def scan_folder(self, path: str = MUSIC_DIR):
        # In Docker, we usually map /music to the user's music.
        # So we scan recursively.
        logger.info(f"Scanning {path}")
        
        supported_exts = {'.mp3', '.flac', '.m4a', '.wav', '.ogg'}
        new_tracks = []

        for root, _, files in os.walk(path):
            for file in files:
                ext = os.path.splitext(file)[1].lower()
                if ext in supported_exts:
                    full_path = os.path.join(root, file)
                    track = self.process_file(full_path)
                    if track:
                        new_tracks.append(track)
        
        # Merge
        # Simple merge: replace if path exists
        existing_map = {t['path']: t for t in self.tracks}
        for t in new_tracks:
            existing_map[t['path']] = t
        
        self.tracks = list(existing_map.values())
        self.save_library()
        return new_tracks

    def process_file(self, path: str):
        try:
            audio = mutagen.File(path, easy=True)
            if not audio:
                return None
            
            # Extract basic metadata
            title = audio.get('title', [os.path.basename(path)])[0]
            artist = audio.get('artist', ['Unknown Artist'])[0]
            album = audio.get('album', ['Unknown Album'])[0]
            duration = audio.info.length if audio.info else 0
            
            # Extract Cover Art (Needs non-easy mutagen)
            # We can hash the path or artist/album to name the cover file
            cover_filename = None
            try:
                f = mutagen.File(path)
                art_data = None
                
                # ID3
                if hasattr(f, 'tags') and hasattr(f.tags, 'getall'):
                    for tag in f.tags.getall('APIC'):
                        art_data = tag.data
                        break
                # FLAC
                elif hasattr(f, 'pictures'):
                     if f.pictures:
                         art_data = f.pictures[0].data
                
                if art_data:
                    h = hashlib.md5(f"{artist}_{album}".encode('utf-8')).hexdigest()
                    cover_filename = f"{h}.jpg"
                    cover_path = os.path.join(COVERS_DIR, cover_filename)
                    if not os.path.exists(cover_path):
                        with open(cover_path, 'wb') as img_f:
                            img_f.write(art_data)
            except Exception as e:
                logger.error(f"Error extracting cover for {path}: {e}")

            return {
                "path": path, # This is the server-side path
                "title": title,
                "artist": artist,
                "album": album,
                "duration_secs": duration,
                "cover_image": cover_filename # Filename only
            }
        except Exception as e:
            logger.error(f"Error processing {path}: {e}")
            return None

    def get_tracks(self):
        return self.tracks

    def analyze_track(self, path: str):
        # Run the sidecar script
        script_path = "/app/backend/services/analyze_track.py"
        if not os.path.exists(script_path):
            return {"error": "Analysis script not found"}
        
        try:
            result = subprocess.run(
                ["python", script_path, path],
                capture_output=True,
                text=True,
                check=False
            )
            if result.returncode != 0:
                logger.error(f"Analysis failed: {result.stderr}")
                return {"error": "Analysis failed"}
            
            return json.loads(result.stdout)
        except Exception as e:
            return {"error": str(e)}

library_service = LibraryService()
