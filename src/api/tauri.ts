import { invoke } from '@tauri-apps/api/core';
import { VibeApiClient } from './types';
import { PlayerStatus, TrackDisplay } from '../types';

export const tauriClient: VibeApiClient = {
    playFile: (path) => invoke('play_file', { path }),
    pause: () => invoke('pause'),
    resume: () => invoke('resume'),
    stop: () => invoke('stop'),
    setVolume: (value) => invoke('set_volume', { value }),
    seek: (value) => invoke('seek', { value }),

    getPlayerState: () => invoke<PlayerStatus>('get_player_state'),

    initLibrary: (path) => invoke<TrackDisplay[]>('init_library', { path }),
    getLibraryTracks: () => invoke<TrackDisplay[]>('get_library_tracks'),
    getCoversDir: () => invoke<string>('get_covers_dir'),
    removeFolder: (path) => invoke('remove_folder', { path }),
    clearAllData: () => invoke('clear_all_data'),

    setEq: (band, gain) => invoke('set_eq', { band, gain }),
    setSpeed: (value) => invoke('set_speed', { value }),
    setReverb: (mix, decay) => invoke('set_reverb', { mix, decay }),

    ytControl: (action, value) => invoke('yt_control', { action, value }),

    getTrackAudioFeatures: (path) => invoke('get_track_audio_features', { path }),
};
