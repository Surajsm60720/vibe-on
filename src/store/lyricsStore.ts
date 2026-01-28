import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { LyricsLine } from '../types';

interface CachedLyricsResponse {
    syncedLyrics: string | null;
    plainLyrics: string | null;
    instrumental: boolean;
    isFetching: boolean;
    error: string | null;
    trackPath: string;
}

interface LyricsStore {
    // State
    lines: LyricsLine[] | null;      // Parsed synced lyrics
    plainLyrics: string | null;       // Fallback plain lyrics
    isLoading: boolean;
    loadingStatus: string;            // Verbose loading status
    error: string | null;
    showLyrics: boolean;              // Panel visibility
    currentTrackId: string | null;    // Track we fetched lyrics for (path)
    isInstrumental: boolean;
    lyricsMode: 'original' | 'romaji' | 'both';
    isTranslating: boolean;
    translationError: string | null;

    // Actions
    fetchLyrics: (artist: string, track: string, duration: number, trackPath: string) => Promise<void>;
    loadCachedLyrics: (trackPath: string) => Promise<void>;
    toggleLyrics: () => void;
    closeLyrics: () => void;
    clearLyrics: () => void;
    setLyricsMode: (mode: 'original' | 'romaji' | 'both') => void;
}

/**
 * Parse LRC format lyrics into structured array
 * LRC format: [mm:ss.xx]Lyric text
 * Example: [00:12.34]Hello world
 */
function parseLRC(lrcString: string): LyricsLine[] {
    const lines: LyricsLine[] = [];
    const regex = /\[(\d{2}):(\d{2})\.(\d{2,3})\](.*)/g;

    let match;
    while ((match = regex.exec(lrcString)) !== null) {
        const minutes = parseInt(match[1], 10);
        const seconds = parseInt(match[2], 10);
        const milliseconds = parseInt(match[3].padEnd(3, '0'), 10);
        const text = match[4].trim();

        // Convert to seconds (float)
        const time = minutes * 60 + seconds + milliseconds / 1000;

        if (text) { // Only add non-empty lines
            lines.push({ time, text });
        }
    }

    // Sort by timestamp (should already be sorted, but just in case)
    lines.sort((a, b) => a.time - b.time);

    return lines;
}

export const useLyricsStore = create<LyricsStore>()((set, get) => ({
    // Initial state
    lines: null,
    plainLyrics: null,
    isLoading: false,
    loadingStatus: '',
    error: null,
    showLyrics: false,
    currentTrackId: null,
    isInstrumental: false,

    isTranslating: false, // Initial state
    translationError: null,
    lyricsMode: (localStorage.getItem('lyricsMode') as 'original' | 'romaji' | 'both') || 'original',

    setLyricsMode: (mode: 'original' | 'romaji' | 'both') => {
        localStorage.setItem('lyricsMode', mode);
        set({ lyricsMode: mode });
    },

    // Load lyrics from backend cache (prefetched when song started)
    loadCachedLyrics: async (trackPath: string) => {
        // Don't refetch for same track if we already have lyrics
        if (get().currentTrackId === trackPath && (get().lines || get().plainLyrics || get().isInstrumental)) {
            console.log('[Lyrics] Already have lyrics for this track, skipping');
            return;
        }

        console.log('[Lyrics] Loading cached lyrics for:', trackPath);
        set({ isLoading: true, loadingStatus: 'Checking cache...', error: null, lines: null, plainLyrics: null, isInstrumental: false, isTranslating: false });

        // Poll the cache - lyrics are being prefetched in the background
        const maxAttempts = 20; // 10 seconds max wait
        let attempts = 0;

        const checkCache = async (): Promise<boolean> => {
            try {
                console.log('[Lyrics] Checking cache, attempt:', attempts + 1);
                // Also update status for user feedback
                if (attempts > 0) set({ loadingStatus: `Waiting for background fetch...` });

                const response = await invoke<CachedLyricsResponse>('get_cached_lyrics', {
                    trackPath
                });
                console.log('[Lyrics] Cache response:', response);

                // If still fetching, wait and try again
                if (response.isFetching && attempts < maxAttempts) {
                    console.log('[Lyrics] Still fetching, will retry...');
                    return false;
                }

                // Process the cached result
                if (response.syncedLyrics) {
                    console.log('[Lyrics] Found synced lyrics');
                    const parsed = parseLRC(response.syncedLyrics);
                    set({
                        lines: parsed,
                        plainLyrics: response.plainLyrics,
                        currentTrackId: trackPath,
                        isLoading: false,
                        isInstrumental: response.instrumental
                    });

                    // Attempt conversion
                    set({ isTranslating: true, translationError: null });
                    import('../utils/lyricsUtils').then(({ convertLyrics }) => {
                        convertLyrics(parsed).then(converted => {
                            if (get().currentTrackId === trackPath) {
                                set({ lines: converted, isTranslating: false });
                            }
                        }).catch(err => {
                            console.error('[LyricsStore] Conversion failed', err);
                            // Set error state so user can see it
                            if (get().currentTrackId === trackPath) {
                                set({
                                    isTranslating: false,
                                    translationError: err.message || String(err)
                                });
                            }
                        });
                    }).catch(err => {
                        console.error('[LyricsStore] Failed to import converter', err);
                        set({ isTranslating: false, translationError: 'Failed to load converter' });
                    });

                } else if (response.plainLyrics) {
                    console.log('[Lyrics] Found plain lyrics');
                    set({
                        lines: null,
                        plainLyrics: response.plainLyrics,
                        currentTrackId: trackPath,
                        isLoading: false,
                        isInstrumental: response.instrumental,
                        isTranslating: false
                    });
                } else if (response.instrumental) {
                    console.log('[Lyrics] Track is instrumental');
                    set({
                        lines: null,
                        plainLyrics: null,
                        currentTrackId: trackPath,
                        isLoading: false,
                        isInstrumental: true,
                        error: null,
                        isTranslating: false
                    });
                } else if (response.error) {
                    console.log('[Lyrics] Error from cache:', response.error);
                    set({
                        error: response.error === 'No lyrics cached for this track' ? 'No lyrics found' : response.error,
                        currentTrackId: trackPath,
                        isLoading: false,
                        isTranslating: false
                    });
                } else {
                    console.log('[Lyrics] No lyrics found');
                    set({
                        error: 'No lyrics found',
                        currentTrackId: trackPath,
                        isLoading: false,
                        isTranslating: false
                    });
                }
                return true;
            } catch (e) {
                console.error('[Lyrics] Error checking cache:', e);
                set({
                    error: String(e),
                    isLoading: false,
                    currentTrackId: trackPath,
                    isTranslating: false
                });
                return true;
            }
        };

        // Initial check
        if (await checkCache()) return;

        // Poll until ready or timeout using a promise-based approach
        const pollWithDelay = (): Promise<void> => {
            return new Promise((resolve) => {
                const poll = async () => {
                    attempts++;
                    if (await checkCache()) {
                        resolve();
                        return;
                    }
                    if (attempts < maxAttempts) {
                        setTimeout(poll, 500);
                    } else {
                        console.log('[Lyrics] Max attempts reached, giving up');
                        set({
                            error: 'Lyrics fetch timed out',
                            currentTrackId: trackPath,
                            isLoading: false,
                            isTranslating: false
                        });
                        resolve();
                    }
                };
                setTimeout(poll, 500);
            });
        };

        await pollWithDelay();
    },

    fetchLyrics: async (artist: string, track: string, duration: number, trackPath: string) => {
        // Don't refetch for same track
        if (get().currentTrackId === trackPath && (get().lines || get().plainLyrics)) {
            return;
        }

        set({ isLoading: true, loadingStatus: 'Starting search...', error: null, lines: null, plainLyrics: null, isInstrumental: false, isTranslating: false });

        // Listen for progress events
        let unlisten: (() => void) | undefined;
        try {
            unlisten = await import('@tauri-apps/api/event').then(m =>
                m.listen<string>('lyrics-loading-status', (event) => {
                    set({ loadingStatus: event.payload });
                })
            );
        } catch (e) {
            console.warn('Failed to setup lyrics progress listener', e);
        }

        try {
            // Duration needs to be in whole seconds for the API
            const durationInt = Math.round(duration);

            const response = await invoke<{
                syncedLyrics: string | null;
                plainLyrics: string | null;
                instrumental: boolean | null;
            }>('get_lyrics', {
                audioPath: trackPath,
                artist,
                track,
                duration: durationInt
            });

            const isInstrumental = response.instrumental ?? false;

            if (response.syncedLyrics) {
                // Parse LRC format
                const parsed = parseLRC(response.syncedLyrics);
                set({
                    lines: parsed,
                    plainLyrics: response.plainLyrics,
                    currentTrackId: trackPath,
                    isLoading: false,
                    isInstrumental
                });

                // Attempt conversion
                set({ isTranslating: true, translationError: null });
                import('../utils/lyricsUtils').then(({ convertLyrics }) => {
                    convertLyrics(parsed).then(converted => {
                        if (get().currentTrackId === trackPath) {
                            set({ lines: converted, isTranslating: false });
                        }
                    }).catch(err => {
                        console.error('[LyricsStore] Conversion failed', err);
                        if (get().currentTrackId === trackPath) {
                            set({
                                isTranslating: false,
                                translationError: err.message || String(err)
                            });
                        }
                    });
                }).catch(err => {
                    console.error('[LyricsStore] Failed to import converter', err);
                    set({ isTranslating: false, translationError: 'Failed to load converter' });
                });

            } else if (response.plainLyrics) {
                // Fallback to plain lyrics
                set({
                    lines: null,
                    plainLyrics: response.plainLyrics,
                    currentTrackId: trackPath,
                    isLoading: false,
                    isInstrumental,
                    isTranslating: false
                });
            } else if (isInstrumental) {
                set({
                    lines: null,
                    plainLyrics: null,
                    currentTrackId: trackPath,
                    isLoading: false,
                    isInstrumental: true,
                    error: null,
                    isTranslating: false
                });
            } else {
                set({
                    error: 'No lyrics found',
                    currentTrackId: trackPath,
                    isLoading: false,
                    isTranslating: false
                });
            }
        } catch (e) {
            set({
                error: String(e),
                isLoading: false,
                currentTrackId: trackPath,
                isTranslating: false
            });
        } finally {
            if (unlisten) unlisten();
        }
    },

    toggleLyrics: () => {
        set(state => ({ showLyrics: !state.showLyrics }));
    },

    closeLyrics: () => {
        set({ showLyrics: false });
    },

    clearLyrics: () => {
        set({
            lines: null,
            plainLyrics: null,
            error: null,
            currentTrackId: null,
            isInstrumental: false,
            isTranslating: false,
            translationError: null
        });
    },
}));
