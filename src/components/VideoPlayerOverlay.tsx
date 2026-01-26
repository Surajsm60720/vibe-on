import { useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { usePlayerStore } from '../store/playerStore';
import { invoke } from '@tauri-apps/api/core';
import type { UnreleasedTrack } from '../types';

interface VideoPlayerOverlayProps {
    isVisible: boolean;
    onClose: () => void;
}

export function VideoPlayerOverlay({ isVisible, onClose }: VideoPlayerOverlayProps) {
    const containerRef = useRef<HTMLDivElement>(null);
    const { status } = usePlayerStore();

    const handleFullscreen = () => {
        if (containerRef.current) {
            if (document.fullscreenElement) {
                document.exitFullscreen();
            } else {
                containerRef.current.requestFullscreen();
            }
        }
    };

    const handleVersionSwitch = async (variant: string) => {
        if (!status.track) return;

        const query = `${status.track.artist} - ${status.track.title} ${variant}`;
        try {
            // Tiny loading feedback could be added here
            const results = await invoke<UnreleasedTrack[]>('search_youtube', {
                filter: { query, content_type: null, max_results: 1 }
            });

            if (results.length > 0) {
                const match = results[0];
                const videoUrl = `https://www.youtube.com/watch?v=${match.video_id}`;
                await invoke('yt_navigate', { url: videoUrl });
                // Update store with new video URL if needed, or just let it play
            }
        } catch (e) {
            console.error("Failed to switch version:", e);
        }
    };

    // Show overlay when we have a track and are in video mode
    if (!status.track) {
        return null;
    }

    return (
        <AnimatePresence>
            {isVisible && (
                <motion.div
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.3 }}
                    className="fixed inset-0 z-50 flex items-center justify-center bg-black/90"
                    onClick={onClose}
                >
                    <motion.div
                        ref={containerRef}
                        initial={{ scale: 0.9, opacity: 0 }}
                        animate={{ scale: 1, opacity: 1 }}
                        exit={{ scale: 0.9, opacity: 0 }}
                        transition={{ duration: 0.3, ease: 'easeOut' }}
                        className="relative w-[90vw] h-[80vh] bg-black rounded-2xl overflow-hidden shadow-2xl"
                        onClick={(e) => e.stopPropagation()}
                    >
                        {/* Video Control Overlay */}
                        <div className="absolute top-4 right-4 z-10 flex gap-2">
                            <button
                                onClick={handleFullscreen}
                                className="p-3 rounded-full bg-surface-container-high hover:bg-surface-container-highest text-white transition-all shadow-lg"
                                title="Toggle Fullscreen"
                            >
                                <svg
                                    width="20"
                                    height="20"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    strokeWidth="2"
                                >
                                    <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
                                </svg>
                            </button>
                            <button
                                onClick={onClose}
                                className="p-3 rounded-full bg-surface-container-high hover:bg-surface-container-highest text-white transition-all shadow-lg"
                                title="Close Video"
                            >
                                <svg
                                    width="20"
                                    height="20"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    strokeWidth="2"
                                >
                                    <line x1="18" y1="6" x2="6" y2="18" />
                                    <line x1="6" y1="6" x2="18" y2="18" />
                                </svg>
                            </button>
                        </div>

                        {/* Track Info Overlay */}
                        <div className="absolute bottom-4 left-4 z-10 max-w-md">
                            <motion.div
                                initial={{ y: 20, opacity: 0 }}
                                animate={{ y: 0, opacity: 1 }}
                                transition={{ delay: 0.2 }}
                                className="bg-surface-container-high rounded-xl p-4 text-white shadow-lg"
                            >
                                <h3 className="text-lg font-semibold truncate">{status.track.title}</h3>
                                <p className="text-sm text-white/70 truncate">{status.track.artist}</p>

                                {/* Version Selector */}
                                <div className="mt-3 flex gap-2 flex-wrap">
                                    <button
                                        onClick={() => handleVersionSwitch('')}
                                        className="text-xs px-3 py-1 rounded-full bg-white/20 hover:bg-white/30 transition-colors"
                                    >
                                        Official
                                    </button>
                                    <button
                                        onClick={() => handleVersionSwitch('slowed reverb')}
                                        className="text-xs px-3 py-1 rounded-full bg-white/20 hover:bg-white/30 transition-colors"
                                    >
                                        Slowed+Reverb
                                    </button>
                                    <button
                                        onClick={() => handleVersionSwitch('1 hour loop')}
                                        className="text-xs px-3 py-1 rounded-full bg-white/20 hover:bg-white/30 transition-colors"
                                    >
                                        1 Hour Loop
                                    </button>
                                    <button
                                        onClick={() => handleVersionSwitch('live')}
                                        className="text-xs px-3 py-1 rounded-full bg-white/20 hover:bg-white/30 transition-colors"
                                    >
                                        Live
                                    </button>
                                </div>
                            </motion.div>
                        </div>

                        {/* Video will be positioned here by the YouTube webview */}
                        <div className="w-full h-full flex items-center justify-center text-white/30">
                            <p className="text-sm">Video player loading...</p>
                        </div>
                    </motion.div>
                </motion.div>
            )}
        </AnimatePresence>
    );
}
