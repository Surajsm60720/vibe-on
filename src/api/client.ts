import { tauriClient } from './tauri';
import { webClient } from './web';
import { VibeApiClient } from './types';

// Detect mode.
// 2. Check for VITE_APP_MODE env var - Build-time check
const isWebMode = import.meta.env.VITE_APP_MODE === 'web';

// Priority: If explicitly Web Mode, use Web Client. Otherwise loop back to Tauri (or Tauri mock if strict).
// Realistically, if we are in Tauri, we use Tauri. 
// If we are in the Browser (Host Mode), we use Web.
export const client: VibeApiClient = (isWebMode) ? webClient : tauriClient;
