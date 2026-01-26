// Mood feature types - isolated for clean removal
// Delete this file to remove the feature

export const MOOD_FEATURE_ENABLED = true;

export interface AudioFeatures {
  valence: number;       // 0-1: sad to happy
  energy: number;        // 0-1: low to high
  danceability: number;  // 0-1: not to very danceable
  tempo: number;         // BPM
  key: number;           // 0-11: C to B
  loudness: number;      // dB (typically -60 to 0)
  instrumentalness: number; // 0-1: vocal to instrumental
  acousticness: number;  // 0-1: electronic to acoustic
  speechiness: number;   // 0-1: music to speech
  liveness: number;      // 0-1: studio to live
  analysis_version: number;
  analyzed_at?: string;
  analysis_error?: string | null;
}

export type MoodPreset = 'happy' | 'sad' | 'energetic' | 'chill' | 'focus' | 'workout';

export interface EssentiaStatus {
  available: boolean;
  python_version: string | null;
  essentia_version: string | null;
  error: string | null;
}

export interface AnalysisProgress {
  current: number;
  total: number;
  current_track: string;
  success_count: number;
  error_count: number;
}

// Key labels for display
export const KEY_LABELS = ['C', 'C♯/D♭', 'D', 'D♯/E♭', 'E', 'F', 'F♯/G♭', 'G', 'G♯/A♭', 'A', 'A♯/B♭', 'B'];

// Mood preset display info
export const MOOD_PRESETS: { id: MoodPreset; label: string; emoji: string; description: string }[] = [
  { id: 'happy', label: 'Happy', emoji: '😊', description: 'Uplifting, positive vibes' },
  { id: 'sad', label: 'Sad', emoji: '😢', description: 'Melancholic, emotional' },
  { id: 'energetic', label: 'Energetic', emoji: '⚡', description: 'High energy, pumped up' },
  { id: 'chill', label: 'Chill', emoji: '😌', description: 'Relaxed, laid-back' },
  { id: 'focus', label: 'Focus', emoji: '🎯', description: 'Concentration-friendly' },
  { id: 'workout', label: 'Workout', emoji: '💪', description: 'Exercise motivation' },
];
