import { VibeApiClient } from './types';
import { PlayerStatus, TrackDisplay } from '../types';

const API_BASE = '/api';

// --- Local Audio Controller ---
class WebAudioController {
    audio: HTMLAudioElement;
    context!: AudioContext;
    source: MediaElementAudioSourceNode | null = null;
    gainNode!: GainNode;

    // State
    status: PlayerStatus = {
        state: 'Stopped',
        track: null,
        position_secs: 0,
        volume: 1.0
    };
    currentPath: string | null = null;

    constructor() {
        this.audio = new Audio();
        this.audio.crossOrigin = "anonymous";

        // Don't create context immediately to avoid 'suspended' state without user interaction context
        // We will create it lazily in play()
    }

    private getContext() {
        if (!this.context) {
            this.context = new (window.AudioContext || (window as any).webkitAudioContext)();
            this.gainNode = this.context.createGain();
            this.gainNode.connect(this.context.destination);

            // Connect audio to context
            if (!this.source) {
                this.source = this.context.createMediaElementSource(this.audio);
                this.source.connect(this.gainNode);
            }
        }
        return this.context;
    }

    // Event Listeners setup (lazy)
    private ensureListeners() {
        // (Re-attaching is fine or check flag)
        this.audio.ontimeupdate = () => {
            this.status.position_secs = this.audio.currentTime;
        };
        this.audio.onended = () => {
            this.status.state = 'Stopped';
        };
        this.audio.onplay = () => {
            this.status.state = 'Playing';
        };
        this.audio.onpause = () => {
            this.status.state = 'Paused';
        };
    }

    async play(path: string) {
        this.ensureListeners();
        const ctx = this.getContext();

        // Ensure context is running
        if (ctx.state === 'suspended') {
            await ctx.resume();
        }

        const streamUrl = `${API_BASE}/stream?path=${encodeURIComponent(path)}`;

        // Only reset src if changing track to avoid reloading same track
        if (this.currentPath !== path || !this.audio.src) {
            this.audio.src = streamUrl;
            this.currentPath = path;
            this.audio.load(); // Force load
        }

        // Update track metadata (Optimistic update)
        this.status.track = {
            path,
            title: path.split('/').pop() || 'Unknown',
            artist: 'Unknown',
            album: 'Unknown',
            duration_secs: 0,
            cover_image: null
        };

        try {
            await this.audio.play();
            this.status.state = 'Playing';
        } catch (e: any) {
            console.error("Audio Playback Failed:", e);
            if (e.name === 'NotAllowedError') {
                // This often happens if the user hasn't interacted with the page yet.
                // We could show a UI toast here, but for now we just log it.
                console.warn("Autoplay blocked. User gesture required.");
            }
            throw e;
        }
    }

    pause() {
        this.audio.pause();
    }

    resume() {
        this.getContext().resume(); // Ensure context allowed
        this.audio.play();
    }

    stop() {
        this.audio.pause();
        this.audio.currentTime = 0;
        this.status.state = 'Stopped';
    }

    setVolume(val: number) {
        // Gain node might not exist if context not init
        if (this.gainNode) {
            this.gainNode.gain.value = val;
        }
        this.status.volume = val;
    }

    seek(val: number) {
        this.audio.currentTime = val;
        this.status.position_secs = val;
    }

    getState(): PlayerStatus {
        return { ...this.status };
    }
}

const audioController = new WebAudioController();
// --- End Local Audio Controller ---


async function apiCall<T>(endpoint: string, method: 'GET' | 'POST' = 'GET', body?: any): Promise<T> {
    const headers: HeadersInit = { 'Content-Type': 'application/json' };
    const config: RequestInit = { method, headers };
    if (body) config.body = JSON.stringify(body);

    const response = await fetch(`${API_BASE}${endpoint}`, config);
    if (!response.ok) throw new Error(`API Error: ${response.statusText}`);
    return response.json();
}

export const webClient: VibeApiClient = {
    playFile: async (path) => {
        // Find track info first?
        // The store calls getPlayerState anyway.
        await audioController.play(path);
    },
    pause: async () => audioController.pause(),
    resume: async () => audioController.resume(),
    stop: async () => audioController.stop(),
    setVolume: async (value) => audioController.setVolume(value),
    seek: async (value) => audioController.seek(value),

    getPlayerState: async () => audioController.getState(),

    // Library Calls -> Backend
    initLibrary: (path) => apiCall<TrackDisplay[]>('/library/scan', 'POST', { path }),
    getLibraryTracks: () => apiCall<TrackDisplay[]>('/library/tracks'),
    getCoversDir: async () => '/api/covers', // Covers prefix
    removeFolder: (path) => apiCall('/library/folder/remove', 'POST', { path }),
    clearAllData: () => apiCall('/system/reset', 'POST'),

    // Mock DSP (Web Audio API implementation is possible but skipped for MVP)
    setEq: async (_band, _gain) => { console.log('Web EQ not implemented yet'); },
    setSpeed: async (value) => {
        audioController.audio.playbackRate = value;
    },
    setReverb: async (_mix, _decay) => { console.log('Web Reverb not implemented yet'); },

    ytControl: async (_action, _value) => { console.log('YouTube not supported in Host Mode yet'); },

    getTrackAudioFeatures: (path) => apiCall('/analysis/features', 'POST', { path }),
};
