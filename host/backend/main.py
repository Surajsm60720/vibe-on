from fastapi import FastAPI, Request
from fastapi.staticfiles import StaticFiles
from fastapi.responses import FileResponse, JSONResponse
from fastapi.middleware.cors import CORSMiddleware
import os

app = FastAPI(title="Vibe-On Host")

# CORS for development
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# API Routers (Placeholders for now)
# API Routers (Placeholders for now)
from .api import router as api_router
app.include_router(api_router, prefix="/api")

from .subsonic import router as subsonic_router
app.include_router(subsonic_router) # Subsonic uses /rest prefix internally defined

from .mood import router as mood_router
app.include_router(mood_router)


# Startup Event
@app.on_event("startup")
async def startup_event():
    print("Starting background library scan...")
    from .services import library_service, MUSIC_DIR
    # Run in background to not block startup
    import threading
    threading.Thread(target=library_service.scan_folder, args=(MUSIC_DIR,)).start()


# Serve React App
# In production, we expect the 'dist' folder to be copied to /app/dist
DIST_DIR = os.getenv("DIST_DIR", "/app/dist")

@app.get("/api/health")
def health_check():
    return {"status": "ok", "mode": "host"}

# Fallback for SPA routing
@app.exception_handler(404)
async def spa_exception_handler(request: Request, exc):
    if request.url.path.startswith("/api") or request.url.path.startswith("/rest"):
        return JSONResponse({"error": "Not Found"}, status_code=404)
    
    index_path = os.path.join(DIST_DIR, "index.html")
    if os.path.exists(index_path):
        return FileResponse(index_path)
    return JSONResponse({"error": "Frontend not found"}, status_code=404)

if os.path.exists(DIST_DIR):
    app.mount("/", StaticFiles(directory=DIST_DIR, html=True), name="static")

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
