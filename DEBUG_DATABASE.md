# Viewing Database Values During Development

There are multiple ways to inspect the audio features database during development:

## Option 1: Debug Tauri Commands (Recommended for Development)

The debug commands are only available in debug builds and provide structured JSON output.

### Get All Audio Features

```javascript
// In your frontend console or component
const allFeatures = await invoke('debug_get_all_features');
console.table(allFeatures);
```

Returns an array of objects with:
```json
{
  "track_path": "/path/to/song.mp3",
  "tempo": 120.5,
  "key": 3,
  "energy": 0.75,
  "valence": 0.82,
  "danceability": 0.68,
  "instrumentalness": 0.15,
  "acousticness": 0.35,
  "speechiness": 0.05,
  "liveness": 0.12,
  "loudness": -5.2,
  "analysis_backend": "rust",
  "last_analyzed": "2024-01-15T10:30:00Z"
}
```

### Get Analysis Statistics

```javascript
// In your frontend console or component
const stats = await invoke('debug_get_statistics');
console.log(stats);
```

Returns:
```json
{
  "total_tracks": 1234,
  "rust_analyzed": 950,
  "python_analyzed": 284,
  "latest_analysis": "2024-01-15T10:30:00Z"
}
```

## Option 2: sqlite3 Command Line

### Find the Database Location

The database is located at:
```
~/Library/Application Support/moe.memesta.vibe-on/library.db
```

Or run this to verify:
```bash
ls -la ~/Library/Application\ Support/moe.memesta.vibe-on/library.db
```

### Open with sqlite3

```bash
sqlite3 ~/Library/Application\ Support/moe.memesta.vibe-on/library.db
```

### Useful SQL Queries

View all tracks with features:
```sql
SELECT 
  track_path,
  tempo,
  key,
  energy,
  valence,
  analysis_backend,
  last_analyzed
FROM audio_features
ORDER BY last_analyzed DESC
LIMIT 10;
```

Count by backend:
```sql
SELECT 
  analysis_backend,
  COUNT(*) as count
FROM audio_features
GROUP BY analysis_backend;
```

Find high-energy tracks:
```sql
SELECT 
  track_path,
  energy,
  valence,
  danceability
FROM audio_features
WHERE energy > 0.7
ORDER BY energy DESC
LIMIT 20;
```

Get average features:
```sql
SELECT 
  AVG(tempo) as avg_tempo,
  AVG(energy) as avg_energy,
  AVG(valence) as avg_valence,
  AVG(danceability) as avg_danceability
FROM audio_features;
```

## Option 3: DB Browser for SQLite (GUI)

1. Install: `brew install --cask db-browser-for-sqlite`
2. Open the app
3. File → Open Database
4. Navigate to: `~/Library/Application Support/moe.memesta.vibe-on/library.db`
5. Browse the `audio_features` table

## Option 4: VSCode SQLite Extension

1. Install the "SQLite" extension by alexcvzz
2. Open Command Palette (Cmd+Shift+P)
3. Type "SQLite: Open Database"
4. Select the database file
5. Browse tables in the SQLite Explorer panel

## Database Schema

The `audio_features` table structure:

| Column | Type | Description |
|--------|------|-------------|
| track_path | TEXT | Primary key, full path to audio file |
| tempo | REAL | BPM (60-200 typical range) |
| key | INTEGER | Musical key (0=C, 1=C#, ..., 11=B) |
| energy | REAL | Energy level (0.0-1.0) |
| valence | REAL | Musical positivity (0.0-1.0) |
| danceability | REAL | How suitable for dancing (0.0-1.0) |
| instrumentalness | REAL | Vocal presence (0.0-1.0, higher = more instrumental) |
| acousticness | REAL | Acoustic vs electronic (0.0-1.0) |
| speechiness | REAL | Presence of spoken words (0.0-1.0) |
| liveness | REAL | Audience presence (0.0-1.0) |
| loudness | REAL | Overall loudness in dB (typically -60 to 0) |
| analysis_version | INTEGER | Schema version (currently 1) |
| analysis_backend | TEXT | "rust" or "python" |
| last_analyzed | TEXT | ISO 8601 timestamp |

## Development Workflow

1. **During Testing**: Use the Tauri debug commands from browser console
   ```javascript
   // Check what's in the database
   invoke('debug_get_statistics').then(console.log);
   
   // View all features
   invoke('debug_get_all_features').then(console.table);
   ```

2. **For SQL Analysis**: Use sqlite3 CLI or DB Browser for complex queries

3. **Before Committing**: Verify database state after changes

## Notes

- Debug commands are **only available in debug builds** (`cargo build` or `npm run tauri dev`)
- Production builds (`cargo build --release`) will not include these commands
- The database file is in the app's data directory, separate from your code
- You can safely delete the database to start fresh (it will be recreated)
