import { motion, AnimatePresence } from 'motion/react';
import React, { useRef, useState, useEffect, useMemo } from 'react';
import { usePlayerStore } from '../store/playerStore';
import { useLyricsStore } from '../store/lyricsStore';
import { useCoverArt } from '../hooks/useCoverArt';
import { useThemeStore } from '../store/themeStore';
import { IconClose, IconPlay, IconPause, IconNext, IconPrevious, IconMusicNote, IconShuffle, IconHeart } from './Icons';
import { CurvedList } from './CurvedList';


import { formatTime } from '../utils/formatTime';

export function ImmersiveView() {
    // Granular Selectors
    const activeTrack = usePlayerStore(s => s.status.track);
    const activeState = usePlayerStore(s => s.status.state);
    const isShuffled = usePlayerStore(s => s.isShuffled);
    const library = usePlayerStore(s => s.library);
    const queue = usePlayerStore(s => s.queue);
    const error = usePlayerStore(s => s.error);
    const isLoading = usePlayerStore(s => s.isLoading);

    // Actions
    const toggleImmersiveMode = usePlayerStore(s => s.toggleImmersiveMode);
    const toggleShuffle = usePlayerStore(s => s.toggleShuffle);
    const pause = usePlayerStore(s => s.pause);
    const { lines } = useLyricsStore();
    const resume = usePlayerStore(s => s.resume);
    const nextTrack = usePlayerStore(s => s.nextTrack);
    const prevTrack = usePlayerStore(s => s.prevTrack);
    const playFile = usePlayerStore(s => s.playFile);
    const setError = usePlayerStore(s => s.setError);

    // Derived
    const isPlaying = activeState === 'Playing';

    // Note: position_secs is NOT destructured here to prevent re-renders. 
    // It is used inside IsolatedProgressRing and IsolatedLyrics logic.

    // Auto-clear error
    useEffect(() => {
        if (error) {
            const timer = setTimeout(() => setError(null), 4000);
            return () => clearTimeout(timer);
        }
    }, [error, setError]);

    // Consume global colors
    const colors = useThemeStore(state => state.colors);

    // Get correct cover
    // Get correct cover
    const currentIndex = library.findIndex(t => t.path === activeTrack?.path);
    const currentLibraryTrack = currentIndex >= 0 ? library[currentIndex] : null;
    const coverUrl = useCoverArt(currentLibraryTrack?.cover_image || activeTrack?.cover_image);

    // const isPlaying = state === 'Playing'; // Already derived above

    // Queue Logic
    const currentQueueIndex = useMemo(() => {
        return queue.findIndex(t => t.path === activeTrack?.path);
    }, [queue, activeTrack]);

    // Calculate Progress for Circle
    // Note: This logic is duplicated here just for reference? No, we don't need it in the main view if we don't render it.
    // Actually, we don't need 'progressPercent' here anymore since IsolatedProgressRing handles it.
    // We can remove it.

    // --- Keyboard Shortcuts ---
    // We still need keyboard shortcuts global listener here.


    // --- Keyboard Shortcuts ---
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

            switch (e.key) {
                case 'ArrowLeft':
                    // seek now needs to be fetched from store since we don't have position_secs here?
                    // Actually we can grab it from store state
                    {
                        const { position_secs, track } = usePlayerStore.getState().status;
                        if (track) usePlayerStore.getState().seek(Math.max(0, position_secs - 10));
                    }
                    break;
                case 'ArrowRight':
                    {
                        const { position_secs, track } = usePlayerStore.getState().status;
                        if (track) usePlayerStore.getState().seek(Math.min(track.duration_secs, position_secs + 10));
                    }
                    break;
                case 'ArrowUp':
                    e.preventDefault();
                    // Volume Up (Need access to setVolume from store? It's not in props)
                    // We can use getState or add it to destructuring
                    usePlayerStore.getState().setVolume(Math.min(1, usePlayerStore.getState().status.volume + 0.05));
                    break;
                case 'ArrowDown':
                    e.preventDefault();
                    usePlayerStore.getState().setVolume(Math.max(0, usePlayerStore.getState().status.volume - 0.05));
                    break;
                case 'l':
                case 'L':
                    if (activeTrack) usePlayerStore.getState().toggleFavorite(activeTrack.path);
                    break;
                case 'Escape':
                    toggleImmersiveMode();
                    break;
            }
        };

        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [toggleImmersiveMode]); // Removed deps that cause re-binds

    return (
        <motion.div
            initial={{ opacity: 0, y: '100%' }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: '100%' }}
            transition={{ type: 'spring', damping: 25, stiffness: 200 }}
            className="fixed inset-0 z-[100] flex flex-col items-center justify-center overflow-hidden select-none"
            style={{ backgroundColor: colors.surfaceContainerLowest, fontFamily: 'Outfit, sans-serif' }}
        >
            {/* Loading Overlay */}
            <AnimatePresence>
                {isLoading && (
                    <motion.div
                        initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
                        className="absolute inset-0 z-[60] bg-black/50 backdrop-blur-sm flex items-center justify-center"
                    >
                        <div className="w-12 h-12 border-4 border-primary border-t-transparent rounded-full animate-spin"></div>
                    </motion.div>
                )}
            </AnimatePresence>

            {/* Error Snackbar - Top Center */}
            <AnimatePresence>
                {error && (
                    <motion.div
                        initial={{ opacity: 0, y: -50, scale: 0.9 }}
                        animate={{ opacity: 1, y: 20, scale: 1 }}
                        exit={{ opacity: 0, y: -20, scale: 0.95 }}
                        className="absolute top-0 z-[70] mt-8 bg-error text-onError px-6 py-3 rounded-full shadow-xl font-medium flex items-center gap-3 backdrop-blur-md bg-opacity-90"
                    >
                        <span>⚠️ {error}</span>
                        <button onClick={() => setError(null)} className="opacity-80 hover:opacity-100 p-1">
                            <IconClose size={18} />
                        </button>
                    </motion.div>
                )}
            </AnimatePresence>
            {/* Close Button */}
            <button
                onClick={toggleImmersiveMode}
                className="absolute top-8 right-8 z-50 p-4 rounded-full transition-colors hover:bg-black/10"
                style={{ color: colors.onSurface }}
            >
                <IconClose size={32} />
            </button>

            {/* MAIN STAGE */}
            <div className="relative w-full h-full flex items-center justify-center overflow-hidden">

                {/* LEFT: CURVED QUEUE */}
                <div className="absolute inset-0 pointer-events-none z-10">
                    <CurvedList
                        items={queue}
                        activeIndex={currentQueueIndex}
                        side="left"
                        radius={600}
                        itemHeight={90}
                        visibleRange={4}
                        renderItem={useMemo(() => (item: any, _, isActive) => (
                            <div
                                className={`
                                    flex flex-row items-center justify-end px-6 py-2 transition-all duration-300 gap-4
                                    ${isActive ? 'opacity-100 scale-105' : 'opacity-40 scale-95'}
                                `}
                                onClick={() => usePlayerStore.getState().playFile(item.path)}
                            >
                                <div className="text-right">
                                    <div className={`text-lg md:text-xl font-bold truncate max-w-[180px] md:max-w-[300px] leading-tight ${isActive ? '' : 'text-right'}`}
                                        style={{ color: colors.onSurface }}>
                                        {item.title}
                                    </div>
                                    <div className="text-xs md:text-sm font-medium opacity-70 mt-0.5 flex items-center justify-end gap-2"
                                        style={{ color: colors.onSurfaceVariant }}>
                                        <span>{formatTime(item.duration_secs)}</span>
                                        <span>•</span>
                                        <span>{item.artist}</span>
                                    </div>
                                </div>

                                {/* Thumbnail */}
                                <div className={`relative w-10 h-10 md:w-12 md:h-12 rounded-lg overflow-hidden shadow-md flex-shrink-0 ${isActive ? 'ring-2 ring-white/20' : ''}`}>
                                    {item.cover_image ? (
                                        <img src={`https://music.youtube.com/${item.cover_image}`} className="w-full h-full object-cover" />
                                        // Note: Assuming cover_image is a partial path or we need a proper hook for it. 
                                        // For queue, we might not have the full resolved URL. 
                                        // Let's use a simple placeholder if it fails or assume it's a URL.
                                        // Actually, queue items usually have 'cover_image' which might be a filename.
                                        // Best to use a placeholder icon if unsure, or try to render.
                                    ) : (
                                        <div className="w-full h-full bg-white/10 flex items-center justify-center">
                                            <IconMusicNote size={20} />
                                        </div>
                                    )}
                                </div>
                            </div>
                        ), [colors, playFile])}
                    />
                </div>

                {/* RIGHT: CURVED LYRICS */}
                <div className="absolute inset-0 pointer-events-none z-10">
                    <IsolatedLyricsRenderer lines={lines || []} colors={colors} />
                </div>


                {/* CENTRAL VINYL */}
                <div className="relative z-20 flex flex-col items-center justify-center h-full w-full">

                    {/* Vinyl Container - Responsive vmin size */}
                    <div className="relative w-[45vmin] h-[45vmin] max-w-[500px] max-h-[500px] flex items-center justify-center mt-[-40px]">

                        {/* ROTATING Album Art - Maximized */}
                        <motion.div
                            className="w-full h-full rounded-full overflow-hidden shadow-2xl relative z-10 bg-black"
                            animate={{ rotate: isPlaying ? 360 : 0 }}
                            transition={{ duration: 8, repeat: Infinity, ease: 'linear' }}
                        >
                            {coverUrl ? (
                                <img src={coverUrl} alt="Album Art" className="w-full h-full object-cover" />
                            ) : (
                                <div className="w-full h-full flex items-center justify-center bg-zinc-800 text-zinc-500">
                                    <IconMusicNote size={150} />
                                </div>
                            )}
                        </motion.div>

                        {/* PROGRESS RING */}
                        <div className="absolute inset-[-15%] z-20 flex items-center justify-center">
                            <div className="w-full h-full">
                                <IsolatedProgressRing colors={colors} />
                            </div>
                        </div>

                    </div>

                    {/* Time Display - Isolated */}
                    <div className="absolute bottom-24 z-30">
                        <IsolatedTimeDisplay colors={colors} />
                    </div>

                    {/* CONTROLS (Bottom, Compact) */}
                    <div className="absolute bottom-8 flex items-center gap-6 z-30 bg-black/20 backdrop-blur-md p-3 rounded-full border border-white/5 scale-90 md:scale-100 shadow-2xl">
                        <button
                            onClick={toggleShuffle}
                            className="p-2 opacity-70 hover:opacity-100 transition-opacity hover:scale-110 active:scale-95"
                            style={{ color: isShuffled ? colors.primary : colors.onSurfaceVariant }}
                            title="Shuffle"
                        >
                            <IconShuffle size={18} />
                        </button>
                        <button
                            onClick={prevTrack}
                            className="p-2 rounded-full hover:bg-white/10 transition-all hover:scale-110 active:scale-95"
                            style={{ color: colors.onSurface }}
                            title="Previous"
                        >
                            <IconPrevious size={24} />
                        </button>
                        <button
                            onClick={isPlaying ? pause : resume}
                            className="p-3 rounded-full hover:scale-110 active:scale-95 transition-all shadow-lg hover:shadow-primary/50"
                            style={{
                                backgroundColor: colors.primary,
                                color: colors.onPrimary,
                                boxShadow: isPlaying ? `0 0 15px ${colors.primary}40` : 'none'
                            }}
                            title={isPlaying ? "Pause" : "Play"}
                        >
                            {isPlaying ? <IconPause size={28} /> : <IconPlay size={28} />}
                        </button>
                        <button
                            onClick={nextTrack}
                            className="p-2 rounded-full hover:bg-white/10 transition-all hover:scale-110 active:scale-95"
                            style={{ color: colors.onSurface }}
                            title="Next"
                        >
                            <IconNext size={24} />
                        </button>
                        <button
                            onClick={() => usePlayerStore.getState().toggleFavorite(activeTrack?.path || '')}
                            className="p-2 opacity-70 hover:opacity-100 transition-opacity hover:scale-110 active:scale-95"
                            style={{ color: usePlayerStore.getState().isFavorite(activeTrack?.path || '') ? colors.tertiary : colors.onSurfaceVariant }}
                            title="Favorite"
                        >
                            <IconHeart size={18} filled={usePlayerStore.getState().isFavorite(activeTrack?.path || '')} />
                        </button>
                    </div>

                </div>

            </div>
        </motion.div>
    );
}

// --- Isolated Sub-Components ---

const IsolatedProgressRing = React.memo(({ colors }: { colors: any }) => {
    // Only subscribe to position/duration/track-presence
    const { position_secs, track } = usePlayerStore(state => state.status);
    const { seek } = usePlayerStore();

    // Standard Circle Props
    const radius = 42;
    const circumference = 2 * Math.PI * radius;
    const duration = track?.duration_secs || 100;
    const progressPercent = Math.min(100, Math.max(0, (position_secs / duration) * 100));

    // --- Interaction Logic (Seek) ---
    const [isDragging, setIsDragging] = useState(false);
    const svgRef = useRef<SVGSVGElement>(null);

    const handleSeek = (e: React.PointerEvent<SVGSVGElement> | PointerEvent) => {
        if (!svgRef.current || !track) return;

        const rect = svgRef.current.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        const x = e.clientX - centerX;
        const y = e.clientY - centerY;
        let angle = Math.atan2(y, x);

        let degrees = angle * (180 / Math.PI);
        degrees += 90; // Shift so Up is 0
        if (degrees < 0) degrees += 360;

        const percentage = degrees / 360;
        const seekTime = percentage * duration;

        seek(seekTime);
    };

    const onPointerDown = (e: React.PointerEvent<SVGSVGElement>) => {
        e.preventDefault();
        setIsDragging(true);
        handleSeek(e);
    };

    useEffect(() => {
        const onPointerMove = (e: PointerEvent) => {
            if (isDragging) {
                handleSeek(e);
            }
        };

        const onPointerUp = () => {
            setIsDragging(false);
        };

        if (isDragging) {
            window.addEventListener('pointermove', onPointerMove);
            window.addEventListener('pointerup', onPointerUp);
        }
        return () => {
            window.removeEventListener('pointermove', onPointerMove);
            window.removeEventListener('pointerup', onPointerUp);
        };
    }, [isDragging, duration, seek, track]);

    return (
        <svg
            ref={svgRef}
            viewBox="0 0 100 100"
            className={`w-full h-full overflow-visible ${(isDragging || !track) ? 'cursor-grab active:cursor-grabbing' : ''}`}
            style={{ transform: 'rotate(-90deg)' }}
            onPointerDown={track ? onPointerDown : undefined}
        >
            {/* Track Background */}
            <circle
                cx="50" cy="50" r={radius}
                fill="none"
                stroke={colors.primary}
                strokeWidth="2"
                strokeOpacity="0.2"
                className="transition-all duration-300 hover:stroke-width-4"
            />
            {/* Progress Ring */}
            <motion.circle
                cx="50" cy="50" r={radius}
                fill="none"
                stroke={colors.primary}
                strokeWidth={isDragging ? 6 : 4}
                strokeLinecap="round"
                strokeDasharray={circumference}
                strokeDashoffset={circumference - (progressPercent / 100) * circumference}
                initial={false}
                animate={{ strokeDashoffset: circumference - (progressPercent / 100) * circumference }}
                transition={isDragging ? { duration: 0 } : { type: 'spring', stiffness: 100, damping: 20 }}
            />

            {/* Interaction Target */}
            <circle
                cx="50" cy="50" r={radius}
                fill="none"
                stroke="transparent"
                strokeWidth="12"
                className="cursor-pointer"
            />

            {/* Thumb Indicator */}
            {(isDragging) && (
                <motion.circle
                    cx="50" cy="50" r="3"
                    fill={colors.onPrimary}
                    style={{
                        translateX: 42,
                        rotate: (progressPercent * 3.6),
                        originX: "50px", originY: "50px"
                    }}
                />
            )}
        </svg>
    );
}); // End memo

const IsolatedLyricsRenderer = React.memo(({ lines, colors }: { lines: any[] | null, colors: any }) => {
    // Only subscribe to position_secs for Active Index Calculation
    const position_secs = usePlayerStore(state => state.status.position_secs);

    const activeLineIndex = useMemo(() => {
        if (!lines || lines.length === 0) return -1;
        for (let i = lines.length - 1; i >= 0; i--) {
            if (position_secs >= lines[i].time) return i;
        }
        return -1;
    }, [lines, position_secs]);

    if (!lines || lines.length === 0) {
        return (
            <div className="absolute right-[10%] top-1/2 -translate-y-1/2 text-center opacity-30 max-w-sm">
                <IconMusicNote size={64} className="mx-auto mb-4" />
                <p className="text-xl font-medium">No synced lyrics</p>
            </div>
        );
    }

    return (
        <CurvedList
            items={lines}
            activeIndex={activeLineIndex}
            side="right"
            radius={600}
            itemHeight={60}
            visibleRange={5}
            renderItem={useMemo(() => (item: any, _, isActive) => (
                <div
                    className={`
                        flex flex-col items-start justify-center px-8 py-4 transition-all duration-500 cursor-pointer origin-left
                    `}
                    style={{
                        transform: isActive ? 'scale(1.5)' : 'scale(1.0)',
                        opacity: isActive ? 1 : 0.3,
                        filter: 'none' // Explicitly no blur
                    }}
                    onClick={() => usePlayerStore.getState().seek(item.time)}
                >
                    <span
                        className={`text-xl md:text-3xl font-bold max-w-[400px] leading-tight transition-colors duration-300`}
                        style={{
                            color: isActive ? colors.onSurface : colors.onSurfaceVariant,
                        }}
                    >
                        {item.text}
                    </span>
                </div>
            ), [colors])}
        />
    );
});

const IsolatedTimeDisplay = React.memo(({ colors }: { colors: any }) => {
    const { position_secs, track } = usePlayerStore(state => state.status);
    const duration = track?.duration_secs || 0;

    return (
        <div className="flex items-center gap-2 text-sm font-medium tracking-wider opacity-80" style={{ color: colors.onSurfaceVariant }}>
            <span>{formatTime(position_secs)}</span>
            <span className="opacity-50">/</span>
            <span>{formatTime(duration)}</span>
        </div>
    );
});
