from fastapi import APIRouter, Request, Response, HTTPException
from fastapi.responses import FileResponse, JSONResponse
import hashlib
import os
from typing import Optional
import xml.etree.ElementTree as ET
from datetime import datetime

from .services import library_service, MUSIC_DIR, COVERS_DIR

# Subsonic API usually lives at /rest
router = APIRouter(prefix="/rest")

# --- Helper Functions ---

def create_base_data(status="ok", version="1.16.1", **kwargs):
    return {"status": status, "version": version, **kwargs}

def format_response(request: Request, data: dict, root_tag: str = "subsonic-response"):
    fmt = request.query_params.get('f', 'xml')
    
    if fmt == 'json':
        # JSON Structure: {"subsonic-response": { ...data... }}
        return JSONResponse({"subsonic-response": data})
    
    else:
        # XML Structure
        # Remove xmlns for compatibility if needed, or keep it.
        # Let's add it back but be careful. 
        # Actually, for XML, attributes of the root need to be set.
        root = ET.Element(root_tag, xmlns="http://subsonic.org/restapi")
        for k, v in data.items():
            if isinstance(v, (str, int, float, bool)):
                root.set(k, str(v))
            # Nested dicts/lists handled by specific endpoint logic usually, 
            # but here we need a generic converter if we want to be clean.
            # For this MVP, let's keep it simple: endpoints build the dict, 
            # we might need custom xml building if complex.
            
        # Wait, the refactor above makes it hard to share logic 1:1 if we rely on ET.SubElement in endpoints.
        # We should let endpoints return the DICT, and we handle conversion?
        # Converting arbitrary dict to Subsonic XML is tricky because of attributes vs text.
        # Subsonic is almost entirely attribute-based.
        pass
    return JSONResponse({"error": "not implemented generic"}) # Placeholder

# Better approach for MVP: Check format in each endpoint or helper
def render(request: Request, data: dict, root_tag: str = "subsonic-response"):
    fmt = request.query_params.get('f', 'xml')
    
    if fmt == 'json':
        return JSONResponse({root_tag: data})
    
    # XML fallback
    root = ET.Element(root_tag, xmlns="http://subsonic.org/restapi")
    
    def dict_to_xml(element, d):
        for k, v in d.items():
            if k == "entry": # Lists
                for item in v:
                    child = ET.SubElement(element, "entry" if "id" in item else "child") # Naming hard
                    # Actually Subsonic uses various tag names.
                    # This generic converter is too hard for 5 mins.
                    # Let's manual-path.
                    pass
            elif isinstance(v, list):
                 # We need to know the tag name for list items. 
                 # In JSON it's just a list. In XML it's repeated tags.
                 pass
            elif isinstance(v, dict):
                child = ET.SubElement(element, k)
                dict_to_xml(child, v)
            else:
                element.set(k, str(v))
    
    # Manual XML building in endpoints is safer for now.
    # We will just split logic in endpoints.
    return JSONResponse({"error": "XML path used old logic"})


# --- Refactored Endpoints (Hybrid) ---

def get_stable_id(s):
    return hashlib.md5(str(s).encode('utf-8')).hexdigest()

@router.get("/getAlbumList.view")
@router.post("/getAlbumList.view")
@router.get("/getAlbumList2.view")
@router.post("/getAlbumList2.view")
def get_album_list(request: Request):
    list_type = request.query_params.get('type', 'newest')
    size = int(request.query_params.get('size', 10))
    fmt = request.query_params.get('f', 'xml')
    
    tracks = library_service.get_tracks()
    print(f"DEBUG: getAlbumList2 called. Total tracks in library: {len(tracks)}")
    
    # We need to simulate Albums from Tracks
    # Group by Album
    albums = {}
    for t in tracks:
        # Use stable ID
        artist = t.get('artist', 'Unknown')
        album = t.get('album', 'Unknown')
        
        album_key = (artist, album)
        if album_key not in albums:
            # We construct a stable ID format: "album_<artist_hash>_<album_hash>"
            artist_id = get_stable_id(artist)
            album_id = get_stable_id(album)
            
            albums[album_key] = {
                "id": f"album_{artist_id}_{album_id}",
                "title": album,
                "artist": artist,
                "parent": f"artist_{artist_id}",
                "coverArt": t.get('cover_image'),
                "created": datetime.now().isoformat(), 
                "songCount": 0,
                "duration": 0,
                "isDir": "true"
            }
        albums[album_key]["songCount"] += 1
        albums[album_key]["duration"] += t.get('duration_secs', 0)

    album_list = list(albums.values())
    print(f"DEBUG: Found {len(album_list)} albums.")
    
    # Sort simple
    if list_type == 'random':
        import random
        random.shuffle(album_list)
    elif list_type == 'alphabeticalByArtist':
        album_list.sort(key=lambda x: x['artist'])
    elif list_type == 'alphabeticalByName':
        album_list.sort(key=lambda x: x['title'])
    # newest/recent/frequent - just return as is for MVP
        
    album_list = album_list[:size]
    
    if fmt == 'json':
        return JSONResponse({"subsonic-response": {
            "status": "ok", "version": "1.16.1", 
            "albumList2": {"album": album_list}
        }})
        
    root = ET.Element("subsonic-response", status="ok", version="1.16.1", xmlns="http://subsonic.org/restapi")
    album_list_el = ET.SubElement(root, "albumList2")
    for alb in album_list:
        ET.SubElement(album_list_el, "album", **{k: str(v) for k, v in alb.items() if v is not None})
    return to_xml_response(root)

@router.get("/getAlbum.view")
@router.post("/getAlbum.view")
def get_album(request: Request):
    id_param = request.query_params.get('id', '')
    fmt = request.query_params.get('f', 'xml')
    
    # Expected ID format: album_<artist_hash>_<album_hash>
    # We need to find tracks belonging to this album
    tracks = library_service.get_tracks()
    
    album_tracks = []
    found_album_name = "Unknown"
    found_artist_name = "Unknown"
    cover_art = None
    
    parts = id_param.split('_')
    if len(parts) >= 3:
        target_artist_hash = parts[1]
        target_album_hash = parts[2]
        
        for track in tracks:
            artist = track.get('artist', 'Unknown')
            album = track.get('album', 'Unknown Album')
            
            if get_stable_id(artist) == target_artist_hash and get_stable_id(album) == target_album_hash:
                found_album_name = album
                found_artist_name = artist
                if not cover_art: cover_art = track.get('cover_image')
                
                s = {
                    "id": track['path'], 
                    "title": track['title'], 
                    "artist": artist, 
                    "album": album,
                    "duration": str(int(track.get('duration_secs', 0))),
                    "path": track['path'],
                    "coverArt": track.get('cover_image'),
                    "contentType": "audio/flac" if track['path'].endswith('.flac') else "audio/mpeg",
                    "isDir": "false",
                    "type": "music",
                    "track": str(track.get('track_number', 0)) if track.get('track_number') else "0"
                }
                album_tracks.append(s)

    # Sort by track number or title
    album_tracks.sort(key=lambda x: (int(x.get('track', 0)), x['title']))
    
    album_data = {
        "id": id_param,
        "name": found_album_name,
        "artist": found_artist_name,
        "songCount": len(album_tracks),
        "duration": sum(int(t['duration']) for t in album_tracks),
        "coverArt": cover_art,
        "song": album_tracks
    }

    if fmt == 'json':
        return JSONResponse({"subsonic-response": {
            "status": "ok", "version": "1.16.1", 
            "album": album_data
        }})

    root = ET.Element("subsonic-response", status="ok", version="1.16.1", xmlns="http://subsonic.org/restapi")
    album_el = ET.SubElement(root, "album")
    for k, v in album_data.items():
        if k == "song":
            for song in v:
                ET.SubElement(album_el, "song", **{sk: str(sv) for sk, sv in song.items() if sv is not None})
        elif v is not None:
             album_el.set(k, str(v))
             
    return to_xml_response(root)

@router.get("/getRandomSongs.view")
@router.post("/getRandomSongs.view")
def get_random_songs(request: Request):
    size = int(request.query_params.get('size', 10))
    fmt = request.query_params.get('f', 'xml')
    
    tracks = library_service.get_tracks()[:]
    print(f"DEBUG: getRandomSongs called. Total tracks: {len(tracks)}")
    
    import random
    random.shuffle(tracks)
    selected = tracks[:size]
    
    songs = []
    for t in selected:
        s = {
            "id": t['path'], 
            "title": t['title'], 
            "artist": t['artist'], 
            "album": t['album'],
            "duration": str(int(t.get('duration_secs', 0))),
            "path": t['path'],
            "coverArt": t.get('cover_image'),
            "contentType": "audio/flac" if t['path'].endswith('.flac') else "audio/mpeg",
            "isDir": "false",
            "type": "music"
        }
        songs.append(s)

    if fmt == 'json':
        return JSONResponse({"subsonic-response": {
            "status": "ok", "version": "1.16.1", 
            "randomSongs": {"song": songs}
        }})

    root = ET.Element("subsonic-response", status="ok", version="1.16.1", xmlns="http://subsonic.org/restapi")
    random_el = ET.SubElement(root, "randomSongs")
    for s in songs:
        ET.SubElement(random_el, "song", **{k: str(v) for k, v in s.items() if v is not None})
    return to_xml_response(root)

@router.get("/getStarred.view")
@router.post("/getStarred.view")
@router.get("/getStarred2.view")
@router.post("/getStarred2.view")
def get_starred(request: Request):
    fmt = request.query_params.get('f', 'xml')
    # Empty for MVP
    if fmt == 'json':
         return JSONResponse({"subsonic-response": {"status": "ok", "version": "1.16.1", "starred2": {"song": []}}})
    root = ET.Element("subsonic-response", status="ok", version="1.16.1", xmlns="http://subsonic.org/restapi")
    ET.SubElement(root, "starred2")
    return to_xml_response(root)

@router.get("/ping.view")
@router.post("/ping.view")
def ping(request: Request):
    fmt = request.query_params.get('f', 'xml')
    if fmt == 'json':
        return JSONResponse({"subsonic-response": {"status": "ok", "version": "1.16.1"}})
    
    root = ET.Element("subsonic-response", status="ok", version="1.16.1", xmlns="http://subsonic.org/restapi")
    return to_xml_response(root)

@router.get("/getLicense.view")
@router.post("/getLicense.view")
def get_license(request: Request):
    fmt = request.query_params.get('f', 'xml')
    data = {"valid": "true", "email": "user@example.com", "key": "ABC", "date": "2023-01-01T00:00:00"}
    
    if fmt == 'json':
        return JSONResponse({"subsonic-response": {"status": "ok", "version": "1.16.1", "license": data}})

    root = ET.Element("subsonic-response", status="ok", version="1.16.1", xmlns="http://subsonic.org/restapi")
    license_el = ET.SubElement(root, "license")
    for k, v in data.items(): license_el.set(k, v)
    return to_xml_response(root)

@router.get("/getMusicFolders.view")
@router.post("/getMusicFolders.view")
def get_music_folders(request: Request):
    fmt = request.query_params.get('f', 'xml')
    folder_data = {"id": "1", "name": "Music Library"}
    
    if fmt == 'json':
        return JSONResponse({"subsonic-response": {
            "status": "ok", "version": "1.16.1", 
            "musicFolders": {"musicFolder": [folder_data]}
        }})

    root = ET.Element("subsonic-response", status="ok", version="1.16.1", xmlns="http://subsonic.org/restapi")
    folders = ET.SubElement(root, "musicFolders")
    ET.SubElement(folders, "musicFolder", **folder_data)
    return to_xml_response(root)

@router.get("/getIndexes.view")
@router.post("/getIndexes.view")
def get_indexes(request: Request):
    fmt = request.query_params.get('f', 'xml')
    tracks = library_service.get_tracks()
    tracks.sort(key=lambda x: x.get('artist', 'Unknown'))
    
    # Build Structure
    last_mod = str(int(datetime.now().timestamp() * 1000))
    indexes_list = []
    
    artists_seen = set()
    current_index = None
    
    # For JSON, typical structure: indexes: { index: [ { name: "A", artist: [...] } ] }
    
    # Helper for XML
    root = ET.Element("subsonic-response", status="ok", version="1.16.1", xmlns="http://subsonic.org/restapi")
    indexes_el = ET.SubElement(root, "indexes", lastModified=last_mod)
    
    # Logic shared?
    # Iterate and build both?
    
    json_indexes = []
    
    for track in tracks:
        artist = track.get('artist', 'Unknown Artist')
        if artist in artists_seen: continue
        artists_seen.add(artist)
        
        first_char = artist[0].upper()
        if first_char < 'A' or first_char > 'Z': first_char = '#'
        
        # Check if we need new index group
        if not current_index or current_index['name'] != first_char:
            current_index = {'name': first_char, 'artist': []}
            json_indexes.append(current_index)
            # XML
            current_index_el = ET.SubElement(indexes_el, "index", name=first_char)
        
        artist_id = f"artist_{hash(artist)}"
        artist_obj = {"id": artist_id, "name": artist}
        
        # Add to JSON
        current_index['artist'].append(artist_obj)
        # Add to XML
        ET.SubElement(current_index_el, "artist", **artist_obj)

    if fmt == 'json':
        return JSONResponse({"subsonic-response": {
            "status": "ok", "version": "1.16.1", 
            "indexes": {"lastModified": last_mod, "index": json_indexes}
        }})
        
    return to_xml_response(root)

@router.get("/getMusicDirectory.view")
@router.post("/getMusicDirectory.view")
def get_music_directory(request: Request):
    id_param = request.query_params.get('id', '')
    fmt = request.query_params.get('f', 'xml')
    
    # Data container
    children = []
    
    tracks = library_service.get_tracks()
    
    if id_param.startswith("artist_"):
        target_artist_hash = id_param.replace("artist_", "")
        albums_seen = set()
        for track in tracks:
            artist = track.get('artist', 'Unknown')
            if str(hash(artist)) == target_artist_hash:
                album = track.get('album', 'Unknown Album')
                if album not in albums_seen:
                    albums_seen.add(album)
                    album_id = f"album_{hash(artist)}_{hash(album)}"
                    children.append({
                        "id": album_id, "title": album, "artist": artist, "isDir": "true", "parent": id_param
                    })
                    
    elif id_param.startswith("album_"):
        parts = id_param.split('_')
        if len(parts) >= 3:
            target_artist_hash = parts[1]
            target_album_hash = parts[2]
            for track in tracks:
                artist = track.get('artist', 'Unknown')
                album = track.get('album', 'Unknown Album')
                if str(hash(artist)) == target_artist_hash and str(hash(album)) == target_album_hash:
                    child = {
                        "id": track['path'], "title": track['title'], "artist": artist, "album": album, 
                        "isDir": "false", "path": track['path'], "parent": id_param,
                        "contentType": "audio/flac" if track['path'].lower().endswith('.flac') else "audio/mpeg"
                    }
                    if track.get('duration_secs'): child['duration'] = str(int(track['duration_secs']))
                    if track.get('cover_image'): child['coverArt'] = track['cover_image']
                    children.append(child)

    if fmt == 'json':
        return JSONResponse({"subsonic-response": {
            "status": "ok", "version": "1.16.1", 
            "directory": {"id": id_param, "name": id_param, "child": children}
        }})

    root = ET.Element("subsonic-response", status="ok", version="1.16.1", xmlns="http://subsonic.org/restapi")
    directory = ET.SubElement(root, "directory", id=id_param, name=id_param)
    for child in children:
        entry = ET.SubElement(directory, "child")
        for k, v in child.items(): entry.set(k, str(v))
        
    return to_xml_response(root)

@router.get("/stream.view")
@router.post("/stream.view")
def stream(request: Request):
    id_param = request.query_params.get('id')
    if not id_param or not os.path.exists(id_param):
        # Subsonic often wants a 404 object, but raw 404 is usually fine for stream
        raise HTTPException(status_code=404, detail="File not found")
    
    filename = os.path.basename(id_param)
    media_type = "audio/mpeg"
    if filename.lower().endswith(".flac"):
        media_type = "audio/flac"
        
    return FileResponse(id_param, media_type=media_type, filename=filename)

@router.get("/getCoverArt.view")
@router.post("/getCoverArt.view")
def get_cover_art(request: Request):
    id_param = request.query_params.get('id')
    if not id_param: raise HTTPException(status_code=404, detail="Not found")
        
    path = os.path.join(COVERS_DIR, id_param)
    if os.path.exists(path): return FileResponse(path)
    return Response(status_code=404)

@router.get("/getUser.view")
@router.post("/getUser.view")
def get_user(request: Request):
    username = request.query_params.get('u', 'admin')
    data = {
        "username": username, "email": "admin@example.com", "scrobblingEnabled": "true", 
        "adminRole": "true", "settingsRole": "true", "downloadRole": "true", "uploadRole": "true", 
        "playlistRole": "true", "coverArtRole": "true", "commentRole": "true", "podcastRole": "true", 
        "streamRole": "true", "jukeboxRole": "true", "shareRole": "true", "videoConversionRole": "true"
    }
    
    fmt = request.query_params.get('f', 'xml')
    if fmt == 'json':
         return JSONResponse({"subsonic-response": {"status": "ok", "version": "1.16.1", "user": data}})
         
    root = ET.Element("subsonic-response", status="ok", version="1.16.1", xmlns="http://subsonic.org/restapi")
    ET.SubElement(root, "user", **data)
    return to_xml_response(root)
