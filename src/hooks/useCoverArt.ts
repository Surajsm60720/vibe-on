import { useState, useEffect } from 'react';
import { readFile } from '@tauri-apps/plugin-fs';
import { usePlayerStore } from '../store/playerStore';
const isWebMode = import.meta.env.VITE_APP_MODE === 'web';
// LRU Cache configuration
const MAX_CACHE_SIZE = 200;

// Global cache for cover art blob URLs - persists across component lifecycles
const coverArtCache = new Map<string, string>();
// Track access order for LRU eviction
const accessOrder: string[] = [];
// Track pending loads to prevent duplicate requests
const pendingLoads = new Map<string, Promise<string | null>>();

// LRU cache helper - moves key to end (most recently used)
function touchCache(key: string) {
    const index = accessOrder.indexOf(key);
    if (index > -1) {
        accessOrder.splice(index, 1);
    }
    accessOrder.push(key);
}

// Evict oldest entries if cache is too large
function evictIfNeeded() {
    while (coverArtCache.size > MAX_CACHE_SIZE && accessOrder.length > 0) {
        const oldest = accessOrder.shift();
        if (oldest) {
            const url = coverArtCache.get(oldest);
            if (url) {
                URL.revokeObjectURL(url);
            }
            coverArtCache.delete(oldest);
        }
    }
}

export function useCoverArt(coverPath: string | null | undefined) {
    const { coversDir } = usePlayerStore();
    const [imageUrl, setImageUrl] = useState<string | null>(() => {
        // Initialize from cache if available
        if (coverPath && coversDir) {
            const cached = coverArtCache.get(coverPath);
            if (cached) {
                touchCache(coverPath);
                return cached;
            }
        }
        return null;
    });

    useEffect(() => {
        if (!coverPath || !coversDir) {
            setImageUrl(null);
            return;
        }

        // Check cache first
        const cached = coverArtCache.get(coverPath);
        if (cached) {
            touchCache(coverPath);
            setImageUrl(cached);
            return;
        }

        let active = true;

        const loadCover = async () => {
            // Check if already loading
            let loadPromise = pendingLoads.get(coverPath);

            if (!loadPromise) {
                // Start new load
                loadPromise = (async () => {
                    if (isWebMode) {
                        try {
                            // In Web Mode, covers are served statically.
                            // Assuming coversDir is a URL prefix like '/covers'
                            // and coverPath is the filename.
                            const url = `${coversDir}/${coverPath}`.replace(/\/+/g, '/');
                            return url;
                        } catch (e) {
                            console.error('Failed to construct cover URL:', e);
                            return null;
                        }
                    }

                    try {
                        const path = `${coversDir}/${coverPath}`.replace(/\\/g, '/');
                        const data = await readFile(path);
                        const blob = new Blob([data]);
                        const objectUrl = URL.createObjectURL(blob);

                        // Evict old entries if needed
                        evictIfNeeded();

                        // Cache it with LRU tracking
                        coverArtCache.set(coverPath, objectUrl);
                        touchCache(coverPath);

                        return objectUrl;
                    } catch (error) {
                        console.error('Failed to load cover:', coverPath, error);
                        return null;
                    } finally {
                        pendingLoads.delete(coverPath);
                    }
                })();

                pendingLoads.set(coverPath, loadPromise);
            }

            const url = await loadPromise;
            if (active && url) {
                setImageUrl(url);
            }
        };

        loadCover();

        return () => {
            active = false;
            // Don't revoke URL - keep it cached (LRU will handle eviction)
        };
    }, [coverPath, coversDir]);

    return imageUrl;
}

// Optional: Call this to clear cache if needed (e.g., when library changes)
export function clearCoverArtCache() {
    coverArtCache.forEach(url => URL.revokeObjectURL(url));
    coverArtCache.clear();
}
