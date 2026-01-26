import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AudioFeatures, AnalysisProgress, EssentiaStatus, MoodPreset } from '../types/mood';

interface MoodStore {
    // Essentia status
    essentiaStatus: EssentiaStatus | null;
    isCheckingEssentia: boolean;

    // Analysis state
    isAnalyzing: boolean;
    analysisProgress: AnalysisProgress | null;

    // Current track features
    currentTrackFeatures: AudioFeatures | null;
    isFetchingFeatures: boolean;

    // Actions
    checkEssentiaStatus: () => Promise<void>;
    fetchTrackFeatures: (trackPath: string) => Promise<AudioFeatures | null>;
    startLibraryAnalysis: (trackPaths: string[]) => Promise<void>;
    cancelAnalysis: () => Promise<void>;
    getMoodQueue: (preset: MoodPreset, limit?: number) => Promise<string[]>;
    getSimilarTracks: (sourcePath: string, limit?: number) => Promise<string[]>;
    clearFeatures: () => void;
    clearAnalysisData: () => Promise<void>;
}

export const useMoodStore = create<MoodStore>((set) => ({
    essentiaStatus: null,
    isCheckingEssentia: false,
    isAnalyzing: false,
    analysisProgress: null,
    currentTrackFeatures: null,
    isFetchingFeatures: false,

    checkEssentiaStatus: async () => {
        set({ isCheckingEssentia: true });
        try {
            const status = await invoke<EssentiaStatus>('check_essentia_available');
            set({ essentiaStatus: status });
        } catch (error) {
            console.error('Failed to check Essentia status:', error);
            set({
                essentiaStatus: {
                    available: false,
                    python_version: null,
                    essentia_version: null,
                    error: String(error),
                },
            });
        } finally {
            set({ isCheckingEssentia: false });
        }
    },

    fetchTrackFeatures: async (trackPath: string) => {
        set({ isFetchingFeatures: true });
        try {
            const features = await invoke<AudioFeatures | null>('get_track_audio_features', { path: trackPath });
            set({ currentTrackFeatures: features });
            return features;
        } catch (error) {
            console.error('Failed to fetch track features:', error);
            set({ currentTrackFeatures: null });
            return null;
        } finally {
            set({ isFetchingFeatures: false });
        }
    },

    startLibraryAnalysis: async (trackPaths: string[]) => {
        set({ isAnalyzing: true, analysisProgress: null });

        // Listen for progress events
        const unlisten = await listen<AnalysisProgress>('mood:analysis_progress', (event) => {
            set({ analysisProgress: event.payload });
        });

        try {
            await invoke('analyze_library', { trackPaths });
        } catch (error) {
            console.error('Library analysis failed:', error);
        } finally {
            unlisten();
            set({ isAnalyzing: false });
        }
    },

    cancelAnalysis: async () => {
        try {
            await invoke('cancel_analysis');
        } catch (error) {
            console.error('Failed to cancel analysis:', error);
        }
    },

    getMoodQueue: async (preset: MoodPreset, limit = 50) => {
        try {
            return await invoke<string[]>('get_mood_radio_queue', { preset, limit });
        } catch (error) {
            console.error('Failed to get mood queue:', error);
            return [];
        }
    },

    getSimilarTracks: async (sourcePath: string, limit = 20) => {
        try {
            return await invoke<string[]>('get_similar_tracks', { sourcePath, limit });
        } catch (error) {
            console.error('Failed to get similar tracks:', error);
            return [];
        }
    },

    clearFeatures: () => {
        set({ currentTrackFeatures: null });
    },

    clearAnalysisData: async () => {
        try {
            await invoke('clear_analysis_data');
            console.log('Analysis data cleared successfully');
        } catch (error) {
            console.error('Failed to clear analysis data:', error);
        }
    },
}));
