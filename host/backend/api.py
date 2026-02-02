from fastapi import APIRouter, HTTPException, BackgroundTasks
from fastapi.responses import FileResponse, JSONResponse, StreamingResponse
from pydantic import BaseModel
from typing import Optional, List
import os

from .services import library_service, MUSIC_DIR, COVERS_DIR

router = APIRouter()

class ScanRequest(BaseModel):
    path: str = MUSIC_DIR

class PathRequest(BaseModel):
    path: str

class VolumeRequest(BaseModel):
    value: float

class SeekRequest(BaseModel):
    value: float

class EqRequest(BaseModel):
    band: int
    gain: float

class SpeedRequest(BaseModel):
    value: float

class ReverbRequest(BaseModel):
    mix: float
    decay: float

# --- Library ---

@router.post("/library/scan")
def scan_library(req: ScanRequest):
    # Security check: ensure path is within MUSIC_DIR or allowed volumes
    # For MVP, we trust the docker mount
    tracks = library_service.scan_folder(req.path)
    return tracks

@router.get("/library/tracks")
def get_tracks():
    return library_service.get_tracks()

@router.post("/library/folder/remove")
def remove_folder(req: PathRequest):
    # Not implemented fully in service, just re-scan without it?
    # For now, no-op
    return {"status": "ok"}

@router.post("/system/reset")
def reset_system():
    # Clear library
    library_service.tracks = []
    library_service.save_library()
    return {"status": "ok"}

# --- Streaming & Covers ---

@router.get("/stream")
def stream_music(path: str):
    if not os.path.exists(path):
        raise HTTPException(status_code=404, detail="File not found")
    
    # Simple FileResponse supports Range headers automatically
    return FileResponse(path, media_type="audio/mpeg", filename=os.path.basename(path))

@router.get("/covers/{filename}")
def get_cover(filename: str):
    path = os.path.join(COVERS_DIR, filename)
    if os.path.exists(path):
        return FileResponse(path)
    # Return placeholder?
    raise HTTPException(status_code=404, detail="Cover not found")

# --- Analysis ---

@router.post("/analysis/features")
def analyze_features(req: PathRequest):
    return library_service.analyze_track(req.path)

# --- Playback State (Stub) ---
# Since playback is local to the web client, these endpoints are no-ops
# just to satisfy any legacy API calls or prospective synchronization.

@router.get("/playback/state")
def get_state():
    return {"state": "Stopped", "volume": 1.0, "position_secs": 0, "track": None}

@router.post("/playback/play")
def play(req: PathRequest):
    return {"status": "ok"}

@router.post("/playback/pause")
def pause():
    return {"status": "ok"}

@router.post("/playback/resume")
def resume():
    return {"status": "ok"}

@router.post("/playback/stop")
def stop():
    return {"status": "ok"}

@router.post("/playback/volume")
def volume(req: VolumeRequest):
    return {"status": "ok"}

@router.post("/playback/seek")
def seek(req: SeekRequest):
    return {"status": "ok"}

@router.post("/audio/eq")
def set_eq(req: EqRequest):
    return {"status": "ok"}

@router.post("/audio/speed")
def set_speed(req: SpeedRequest):
    return {"status": "ok"}

@router.post("/audio/reverb")
def set_reverb(req: ReverbRequest):
    return {"status": "ok"}

@router.post("/youtube/control")
def yt_control():
    return {"status": "not_supported"}
