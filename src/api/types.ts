import { PlayerStatus, TrackDisplay } from '../types';

export interface VibeApiClient {
    // Playback
    playFile: (path: string) => Promise<void>;
    pause: () => Promise<void>;
    resume: () => Promise<void>;
    stop: () => Promise<void>;
    setVolume: (value: number) => Promise<void>;
    seek: (value: number) => Promise<void>;

    // State
    getPlayerState: () => Promise<PlayerStatus>;

    // Library
    // initLibrary Scans a folder and returns tracks
    initLibrary: (path: string) => Promise<TrackDisplay[]>;
    getLibraryTracks: () => Promise<TrackDisplay[]>;
    getCoversDir: () => Promise<string>;
    removeFolder: (path: string) => Promise<void>;
    clearAllData: () => Promise<void>;

    // DSP & Audio Effects
    setEq: (band: number, gain: number) => Promise<void>;
    setSpeed: (value: number) => Promise<void>;
    setReverb: (mix: number, decay: number) => Promise<void>;

    // YouTube (Maybe optional for Web Mode initially, but good to have)
    ytControl: (action: string, value?: number) => Promise<void>;

    // Analysis
    getTrackAudioFeatures: (path: string) => Promise<any>;
}
