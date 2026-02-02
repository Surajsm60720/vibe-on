// Mock Tauri APIs for Web Build

// We need to map Tauri commands to Backend API calls
export const invoke = async (cmd: string, args: any = {}) => {
    console.log(`[WebMock] invoke: ${cmd}`, args);

    // --- Mood Radio Commands ---
    if (cmd === 'check_essentia_available') {
        return fetch('/api/mood/status').then(r => r.json());
    }
    if (cmd === 'analyze_library') {
        return fetch('/api/mood/analyze', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(args)
        }).then(r => r.json());
    }
    if (cmd === 'get_track_audio_features') {
        // args: { path: string }
        // We probably need to encode the path
        const params = new URLSearchParams({ path: args.path });
        return fetch(`/api/mood/features?${params.toString()}`).then(r => r.json());
    }
    if (cmd === 'get_mood_radio_queue') {
        return fetch('/api/mood/queue', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(args)
        }).then(r => r.json());
    }
    if (cmd === 'get_similar_tracks') {
        return fetch('/api/mood/similar', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(args)
        }).then(r => r.json());
    }
    if (cmd === 'get_analysis_stats') {
        return fetch('/api/mood/stats').then(r => r.json());
    }

    // --- Torrent Commands ---
    if (['get_torrents', 'search_torrents'].includes(cmd)) {
        console.warn(`[WebMock] Torrent command ${cmd} not supported in web mode.`);
        return [];
    }
    if (['init_torrent_backend', 'pause_torrent', 'resume_torrent', 'delete_torrent'].includes(cmd)) {
        console.warn(`[WebMock] Torrent action ${cmd} ignored.`);
        return null;
    }

    // --- Shell / Opener ---
    if (cmd === 'show_item_in_folder') {
        console.warn("Reveal in folder not supported in web");
        return;
    }

    console.warn(`[WebMock] Unhandled invoke: ${cmd}`);
    return null;
};

// File System
export const readFile = async () => { console.warn("Tauri readFile called in Web Mode"); return new Uint8Array(); };
export const readTextFile = async () => { console.warn("Tauri readTextFile called in Web Mode"); return ""; };
export const writeTextFile = async () => { console.warn("Tauri writeTextFile called in Web Mode"); };

// Assets
export const convertFileSrc = (path: string) => {
    // In web mode, we can't load local files directly unless served by backend.
    // Backend serves covers at /api/covers/...
    // But direct file paths (e.g. /music/song.mp3) needs /api/stream?id=... logic?
    // Actually the player logic might use convertFileSrc for audio src.
    // Our backend `main.py` or `api.py` should hopefully handle streaming.
    // For local files in docker:
    // If path starts with /music, map to /api/stream?path=...

    if (path.startsWith('/music') || path.startsWith('/app/data')) {
        // Encode path for URL
        return `/rest/stream.view?id=${encodeURIComponent(path)}&u=admin&p=admin&v=1.16.1&c=web`;
    }
    if (path.startsWith('http')) return path;

    return path;
};

// Dialog
export const open = async () => { console.warn("Tauri dialog open called in Web Mode"); return null; };
export const save = async () => { console.warn("Tauri dialog save called in Web Mode"); return null; };
export const ask = async () => { console.warn("Tauri dialog ask called in Web Mode"); return false; };
export const confirm = async () => { console.warn("Tauri dialog confirm called in Web Mode"); return false; };
export const message = async () => { console.warn("Tauri dialog message called in Web Mode"); };

// Events
export const listen = async (event: string, handler: any) => {
    // Mock mood events
    if (event === 'mood:analysis_progress') {
        // Polling loop in backend? Or server sent events?
        // For MVP, just don't emit anything.
    }
    console.warn(`Tauri event listen ${event} called in Web Mode`);
    return () => { };
};
export const emit = async () => { console.warn("Tauri event emit called in Web Mode"); };

// Opener
export const revealItemInDir = async () => { console.warn("Tauri opener reveal called in Web Mode"); };

// Window
export const getCurrentWindow = () => ({
    close: async () => console.warn("Window close mock"),
    minimize: async () => console.warn("Window minimize mock"),
    toggleMaximize: async () => console.warn("Window maximize mock"),
});

// WebviewWindow
export class WebviewWindow {
    constructor(label: string, options: any) {
        console.warn("New WebviewWindow mock", label);
    }
    static async getByLabel(label: string) {
        return null;
    }
    once(event: string, handler: any) { }
    async setFocus() { }
}

// Path
export const downloadDir = async () => { return "/music/downloads"; };
export const join = async (...args: string[]) => args.join("/");

