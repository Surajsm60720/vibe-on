from fastapi import APIRouter, Request, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel
from typing import List, Optional
import sys
import os
import json
import subprocess
from .services import MUSIC_DIR, library_service

router = APIRouter(prefix="/api/mood")

class AnalyzeParams(BaseModel):
    trackPaths: List[str]

class QueueParams(BaseModel):
    preset: str
    limit: int

class SimilarParams(BaseModel):
    sourcePath: str
    limit: int

# Update: Check if analyze_track.py exists where we expect it
ANALYZER_SCRIPT = "/app/backend/services/analyze_track.py"

# Preset Definitions (Approximate)
PRESET_RANGES = {
    "happy": {"min_valence": 0.6, "min_energy": 0.4},
    "sad": {"max_valence": 0.4, "max_energy": 0.6},
    "energetic": {"min_energy": 0.7, "min_danceability": 0.5},
    "chill": {"max_energy": 0.5, "max_tempo": 120},
    "focus": {"max_energy": 0.6, "min_instrumentalness": 0.5},
    "workout": {"min_energy": 0.8, "min_tempo": 110},
}

@router.get("/status")
def check_status():
    # We check if we can run the script
    if not os.path.exists(ANALYZER_SCRIPT):
         return {"available": False, "error": "Analysis script missing"}

    try:
        result = subprocess.run(
            ["python3", ANALYZER_SCRIPT, "--check"], 
            capture_output=True, 
            text=True
        )
        if result.returncode == 0:
            info = json.loads(result.stdout)
            return {
                "available": True,
                "engine": info.get("engine"),
                "python_version": sys.version,
                "essentia_version": info.get("essentia_version"),
                "error": None
            }
    except Exception as e:
        return {"available": False, "error": f"Check failed: {e}"}
        
    return {
        "available": True, # It exists, maybe check failed but we can try running?
        "engine": "unknown",
        "python_version": sys.version,
        "essentia_version": None, 
        "error": "Status check returned non-zero"
    }

# Features Store (Simple JSON)
DATA_DIR = "/app/data"
FEATURES_FILE = os.path.join(DATA_DIR, "features.json")

def load_features():
    if os.path.exists(FEATURES_FILE):
        try:
            with open(FEATURES_FILE, 'r') as f:
                return json.load(f)
        except: return {}
    return {}

def save_features(features_map):
    with open(FEATURES_FILE, 'w') as f:
        json.dump(features_map, f)

@router.post("/analyze")
async def analyze_library(params: AnalyzeParams):
    # Background Task (Simplest: Run in main thread for MVP or use background tasks)
    # We will do a blocking loop for a few tracks to demonstrate, usually this should be async
    
    # WARNING: Blocking in FastAPI blocks the whole server unless we use threadpool or async
    # We'll just check if we have features, if not, compute.
    
    # For the "Enable" button to feel responsive, we'll verify dependencies and just return success.
    # The actual heavy lifting of analyzing ALL tracks is too much for a sync request.
    # But the UI calls this to "Start".
    
    # Let's minimally analyze the tracks passed in (which might be the whole lib).
    # We'll rely on the client to batch or we just do it for 10-20 to show it works.
    
    existing = load_features()
    count = 0
    
    # Limit to first 20 for MVP responsiveness
    for path in params.trackPaths[:20]: 
        if path in existing: continue
        
        # Analyze
        feats = get_features(path)
        if feats:
            existing[path] = feats
            count += 1
            
    if count > 0:
        save_features(existing)
        
    return {"status": "ok", "processed": count}

@router.get("/stats")
def get_stats():
    feats = load_features()
    return [len(feats), 0] 

def get_features(path: str):
    # Check cache first
    existing = load_features()
    if path in existing: return existing[path]

    # Run the analyzer script for a single track
    if not os.path.exists(path):
        if not path.startswith("/"):
             path = os.path.join(MUSIC_DIR, path)
    
    if not os.path.exists(path):
        print(f"Analysis failed: File not found: {path}")
        return None # Silent fail for features

    try:
        # Run subprocess
        result = subprocess.run(
            ["python3", ANALYZER_SCRIPT, path], 
            capture_output=True, 
            text=True,
            timeout=30
        )
        
        if result.returncode != 0:
            print(f"Analysis failed for {path}:")
            print(f"  Return code: {result.returncode}")
            print(f"  Stdout: {result.stdout}")
            print(f"  Stderr: {result.stderr}")
            return None
            
        data = json.loads(result.stdout)
        if "error" in data:
            print(f"Analysis error for {path}: {data['error']}")
            return None
        return data
        
    except subprocess.TimeoutExpired:
        print(f"Analysis timeout for {path}")
        return None
    except json.JSONDecodeError as e:
        print(f"JSON decode error for {path}: {e}")
        print(f"  Output: {result.stdout}")
        return None
    except Exception as e:
        print(f"Error running analysis for {path}: {e}")
        return None

@router.post("/queue")
def get_queue(params: QueueParams):
    feats = load_features()
    if not feats:
         # Fallback to random if no features (prevents empty list)
        tracks = library_service.get_tracks()
        import random
        random.shuffle(tracks)
        return [t['path'] for t in tracks[:params.limit]]
        
    # Validation
    preset_key = params.preset.lower()
    ranges = PRESET_RANGES.get(preset_key, {})
    
    matches = []
    
    for path, f in feats.items():
        valid = True
        
        # Check all conditions in the preset
        if "min_valence" in ranges and f.get("valence", 0) < ranges["min_valence"]: valid = False
        if "max_valence" in ranges and f.get("valence", 0) > ranges["max_valence"]: valid = False
        
        if "min_energy" in ranges and f.get("energy", 0) < ranges["min_energy"]: valid = False
        if "max_energy" in ranges and f.get("energy", 0) > ranges["max_energy"]: valid = False
        
        if "min_danceability" in ranges and f.get("danceability", 0) < ranges["min_danceability"]: valid = False
        
        if "min_instrumentalness" in ranges and f.get("instrumentalness", 0) < ranges["min_instrumentalness"]: valid = False
        
        if "min_tempo" in ranges and f.get("tempo", 0) < ranges["min_tempo"]: valid = False
        if "max_tempo" in ranges and f.get("tempo", 0) > ranges["max_tempo"]: valid = False
        
        if valid: matches.append(path)
    
    # If no matches found (too strict?), fallback to partial or random
    if not matches:
        print("No matches found for preset, relaxing constraints...")
        # For MVP, just return random mixed with some loosely valid ones could be better
        # But let's just return nothing so user knows they need to analyze more?
        # Or return random to not break flow.
        pass
        
    import random
    random.shuffle(matches)
    return matches[:params.limit]

@router.post("/similar")
def get_similar(params: SimilarParams):
    feats = load_features()
    source = feats.get(params.sourcePath)
    
    if not source:
        # Try to analyze strictly the source if missing
        source = get_features(params.sourcePath)
        
    if not source or not feats:
        tracks = library_service.get_tracks()
        import random
        random.shuffle(tracks)
        return [t['path'] for t in tracks[:params.limit]]
        
    # Simple Euclidean distance
    candidates = []
    for path, f in feats.items():
        if path == params.sourcePath: continue
        
        dist = (
            (f["valence"] - source["valence"])**2 + 
            (f["energy"] - source["energy"])**2 +
            (f["danceability"] - source["danceability"])**2
        )
        candidates.append((dist, path))
        
    candidates.sort(key=lambda x: x[0])
    return [c[1] for c in candidates[:params.limit]]
