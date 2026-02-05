import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type VisualizerMode = 'bars' | 'waveform';
export type VisualizerDisplay = 'off' | 'mini' | 'compact' | 'fullscreen';

interface VisualizerState {
    // Settings
    mode: VisualizerMode;
    displayMode: VisualizerDisplay;
    sensitivity: number; // 0.5 - 2.0, multiplier for visual intensity
    smoothing: number; // 0.0 - 0.9, higher = smoother transitions

    // Runtime state (not persisted)
    frequencyBins: number[];
    waveform: number[];

    // Actions
    setMode: (mode: VisualizerMode) => void;
    setDisplayMode: (displayMode: VisualizerDisplay) => void;
    setSensitivity: (sensitivity: number) => void;
    setSmoothing: (smoothing: number) => void;
    toggleVisualizer: () => void;
    cycleMode: () => void;
    updateData: (frequencyBins: number[], waveform: number[]) => void;
}

export const useVisualizerStore = create<VisualizerState>()(
    persist(
        (set, get) => ({
            // Default settings
            mode: 'bars',
            displayMode: 'off',
            sensitivity: 1.0,
            smoothing: 0.7,

            // Runtime state
            frequencyBins: [],
            waveform: [],

            // Actions
            setMode: (mode) => set({ mode }),

            setDisplayMode: (displayMode) => set({ displayMode }),

            setSensitivity: (sensitivity) => set({
                sensitivity: Math.max(0.5, Math.min(2.0, sensitivity))
            }),

            setSmoothing: (smoothing) => set({
                smoothing: Math.max(0.0, Math.min(0.9, smoothing))
            }),

            toggleVisualizer: () => {
                const current = get().displayMode;
                // Cycle: off -> mini -> compact -> off
                const next: VisualizerDisplay =
                    current === 'off' ? 'mini' :
                        current === 'mini' ? 'compact' : 'off';
                set({ displayMode: next });
            },

            cycleMode: () => {
                const current = get().mode;
                set({ mode: current === 'bars' ? 'waveform' : 'bars' });
            },

            updateData: (frequencyBins, waveform) => {
                const { smoothing } = get();
                const prevBins = get().frequencyBins;

                // Apply smoothing (lerp between previous and new values)
                const smoothedBins = frequencyBins.map((bin, i) => {
                    const prev = prevBins[i] ?? 0;
                    return prev * smoothing + bin * (1 - smoothing);
                });

                set({
                    frequencyBins: smoothedBins,
                    waveform
                });
            },
        }),
        {
            name: 'vibe-on-visualizer',
            // Only persist settings, not runtime data
            partialize: (state) => ({
                mode: state.mode,
                displayMode: state.displayMode,
                sensitivity: state.sensitivity,
                smoothing: state.smoothing,
            }),
        }
    )
);
