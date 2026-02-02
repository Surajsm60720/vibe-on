import { create } from 'zustand';
import { SpotifyTrack, AudioFeatures, spotifyApi } from './spotifyApi';

interface SpotifyState {
    searchResults: SpotifyTrack[];
    isSearching: boolean;
    selectedTrack: SpotifyTrack | null;
    selectedTrackFeatures: AudioFeatures | null;
    error: string | null;

    search: (query: string) => Promise<void>;
    selectTrack: (track: SpotifyTrack) => Promise<void>;
    authenticate: (clientId: string, clientSecret: string) => Promise<boolean>;
}

export const useSpotifyStore = create<SpotifyState>((set) => ({
    searchResults: [],
    isSearching: false,
    selectedTrack: null,
    selectedTrackFeatures: null,
    error: null,

    authenticate: async (clientId: string, clientSecret: string) => {
        try {
            const success = await spotifyApi.authenticate(clientId, clientSecret);
            if (!success) set({ error: 'Authentication failed' });
            return success;
        } catch (e) {
            set({ error: (e as Error).message });
            return false;
        }
    },

    search: async (query: string) => {
        if (!query) return;
        set({ isSearching: true, error: null });
        try {
            const results = await spotifyApi.searchTracks(query);
            set({ searchResults: results, isSearching: false });
        } catch (e) {
            set({ error: (e as Error).message, isSearching: false });
        }
    },

    selectTrack: async (track: SpotifyTrack) => {
        set({ selectedTrack: track, selectedTrackFeatures: null });
        try {
            const features = await spotifyApi.getAudioFeatures(track.id);
            set({ selectedTrackFeatures: features });
        } catch (e) {
            console.error('Failed to fetch audio features:', e);
        }
    },
}));
