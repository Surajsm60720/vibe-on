import { useEffect, useRef, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import { usePlayerStore } from '../store/playerStore';
import { useCoverArt } from '../hooks/useCoverArt';
import { useSettingsStore } from '../store/settingsStore';
import { SquigglySlider } from './SquigglySlider';
import { MarqueeText } from './MarqueeText';
import { TrackStats } from './mood';
import {
    IconPrevious,
    IconPlay,
    IconPause,
    IconNext,
    IconVolume,
    IconMusicNote,
    IconShuffle,
} from './Icons';

// Repeat icon component
function IconRepeat({ size = 24, mode = 'off' }: { size?: number; mode?: 'off' | 'all' | 'one' }) {
    return (
        <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <polyline points="17 1 21 5 17 9" />
            <path d="M3 11V9a4 4 0 0 1 4-4h14" />
            <polyline points="7 23 3 19 7 15" />
            <path d="M21 13v2a4 4 0 0 1-4 4H3" />
            {mode === 'one' && <text x="12" y="14" fontSize="8" textAnchor="middle" fill="currentColor" stroke="none">1</text>}
        </svg>
    );
}

// Heart icon for favorites
function IconHeart({ size = 24, filled = false, className }: { size?: number; filled?: boolean; className?: string }) {
    return (
        <svg width={size} height={size} viewBox="0 0 24 24" fill={filled ? "currentColor" : "none"} stroke="currentColor" strokeWidth="2" className={className}>
            <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z" />
        </svg>
    );
}

// Format seconds to MM:SS
function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
}

// Expressive Button with Click Animation and Scalloped Shape
const ExpressiveControlButton = ({ onClick, icon, disabled, direction = 'left' }: { onClick: () => void, icon: React.ReactNode, disabled: boolean, direction?: 'left' | 'right' }) => {
    const [rotation, setRotation] = useState(0);

    const handleClick = () => {
        if (!disabled) {
            setRotation(prev => prev + 120);
            onClick();
        }
    };

    return (
        <motion.button
            initial={{ opacity: 0, scale: 0.8, x: direction === 'left' ? 20 : -20 }}
            animate={{ opacity: 1, scale: 1, x: 0 }}
            exit={{ opacity: 0, scale: 0.8, x: direction === 'left' ? 20 : -20 }}
            whileHover={{ scale: 1.2 }}
            onClick={handleClick}
            disabled={disabled}
            className="group relative w-10 h-10 flex items-center justify-center pointer-events-auto transition-transform disabled:opacity-50"
        >
            {/* Rotating Background Shape */}
            <motion.svg
                viewBox="0 0 340 340"
                className="absolute inset-0 w-full h-full text-tertiary-container drop-shadow-md group-hover:drop-shadow-lg transition-all"
                animate={{ rotate: rotation }}
                transition={{ type: "spring", stiffness: 260, damping: 20 }}
            >
                <path d="M261.856 41.2425C272.431 41.9625 277.718 42.3226 281.991 44.1826C288.175 46.8926 293.111 51.8325 295.816 58.0125C297.685 62.2825 298.044 67.5725 298.762 78.1425L300.402 102.273C300.693 106.553 300.838 108.693 301.303 110.733C301.975 113.683 303.142 116.503 304.754 119.063C305.869 120.843 307.279 122.453 310.097 125.683L326.001 143.903C332.97 151.893 336.455 155.882 338.155 160.222C340.615 166.512 340.615 173.493 338.155 179.783C336.455 184.123 332.97 188.112 326.001 196.102L310.097 214.322C307.279 217.552 305.869 219.162 304.754 220.942C303.142 223.502 301.975 226.323 301.303 229.273C300.838 231.313 300.693 233.452 300.402 237.732L298.762 261.863C298.044 272.433 297.685 277.723 295.816 281.993C293.111 288.173 288.175 293.112 281.991 295.822C277.718 297.682 272.431 298.043 261.856 298.763L237.725 300.403C233.448 300.693 231.31 300.843 229.267 301.303C226.316 301.973 223.499 303.143 220.937 304.753C219.164 305.873 217.549 307.283 214.319 310.103L196.097 326.003C188.111 332.973 184.119 336.453 179.775 338.153C173.491 340.623 166.509 340.623 160.225 338.153C155.881 336.453 151.889 332.973 143.903 326.003L125.681 310.103C122.451 307.283 120.836 305.873 119.063 304.753C116.501 303.143 113.684 301.973 110.733 301.303C108.69 300.843 106.552 300.693 102.275 300.403L78.1438 298.763C67.5694 298.043 62.2822 297.682 58.0088 295.822C51.8252 293.112 46.8887 288.173 44.1844 281.993C42.3154 277.723 41.9561 272.433 41.2375 261.863L39.5977 237.732C39.3071 233.452 39.1618 231.313 38.6969 229.273C38.0251 226.323 36.8584 223.502 35.2463 220.942C34.1306 219.162 32.7213 217.552 29.9027 214.322L13.999 196.102C7.02996 188.112 3.54542 184.123 1.84516 179.783C-0.615054 173.493 -0.615053 166.512 1.84516 160.222C3.54542 155.882 7.02996 151.893 13.999 143.903L29.9027 125.683C32.7213 122.453 34.1306 120.843 35.2463 119.063C36.8584 116.503 38.0251 113.683 38.6969 110.733C39.1618 108.693 39.3071 106.553 39.5977 102.273L41.2375 78.1425C41.9561 67.5725 42.3154 62.2825 44.1844 58.0125C46.8887 51.8325 51.8252 46.8926 58.0088 44.1826C62.2823 42.3226 67.5694 41.9625 78.1438 41.2425L102.275 39.6025C106.552 39.3125 108.69 39.1625 110.733 38.7025C113.684 38.0325 116.501 36.8625 119.063 35.2525C120.836 34.1325 122.451 32.7225 125.681 29.9025L143.903 14.0025C151.889 7.03252 155.881 3.5525 160.225 1.8525C166.509 -0.6175 173.491 -0.6175 179.775 1.8525C184.119 3.5525 188.111 7.03252 196.097 14.0025L214.319 29.9025C217.549 32.7225 219.164 34.1325 220.937 35.2525C223.499 36.8625 226.316 38.0325 229.267 38.7025C231.31 39.1625 233.448 39.3125 237.725 39.6025L261.856 41.2425Z" fill="currentColor" />
            </motion.svg>

            {/* Icon */}
            <div className="relative z-10 text-on-tertiary-container">
                {icon}
            </div>
        </motion.button>
    );
};

export function PlayerBar() {
    const {
        status, pause, resume, setVolume, refreshStatus, nextTrack, prevTrack,
        getCurrentTrackIndex, library, playFile, seek, repeatMode, cycleRepeatMode,
        error, setError, isShuffled, toggleShuffle, favorites, toggleFavorite
    } = usePlayerStore();
    const { albumArtStyle, expandedArtMode } = useSettingsStore();
    const { state, track, position_secs, volume } = status;
    const lastStateRef = useRef(state);

    // Sync DSP Settings on Hydration/Change
    const { reverbMix, reverbDecay, speed, preampDb, balance, stereoWidth } = usePlayerStore();
    useEffect(() => {
        // This ensures that when the store rehydrates (and values change from defaults),
        // or when any component changes these values, the backend is updated.
        // This covers the startup sync case.
        invoke('set_reverb', { mix: reverbMix, decay: reverbDecay }).catch(console.error);
        invoke('set_speed', { value: speed }).catch(console.error);
        invoke('set_eq', { band: 10, gain: preampDb }).catch(console.error);
        invoke('set_eq', { band: 11, gain: balance }).catch(console.error);
        invoke('set_eq', { band: 12, gain: stereoWidth }).catch(console.error);
    }, [reverbMix, reverbDecay, speed, preampDb, balance, stereoWidth]);

    // Poll for status updates while playing
    useEffect(() => {
        if (state === 'Playing') {
            const interval = setInterval(refreshStatus, 200);
            return () => clearInterval(interval);
        }
    }, [state, refreshStatus]);

    // Auto-clear error
    useEffect(() => {
        if (error) {
            const timer = setTimeout(() => setError(null), 4000);
            return () => clearTimeout(timer);
        }
    }, [error, setError]);

    // Auto-play next track when current track ends
    useEffect(() => {
        if (lastStateRef.current === 'Playing' && state === 'Stopped' && track) {
            // Threshold increased to 2.0s to account for polling latency
            const isFinished = position_secs >= track.duration_secs - 2.0;

            console.log('[Autoplay] Transitioned to Stopped. Finished?', isFinished, { pos: position_secs, dur: track.duration_secs });

            if (isFinished) {
                if (repeatMode === 'one') {
                    // Replay current track
                    playFile(track.path);
                } else {
                    nextTrack();
                }
            }
        }
        lastStateRef.current = state;
    }, [state, track, position_secs, nextTrack, repeatMode, playFile]);

    const handlePlayPause = () => {
        if (state === 'Playing') {
            pause();
        } else if (state === 'Paused') {
            resume();
        } else if (state === 'Stopped' && track) {
            playFile(track.path);
        }
    };

    const handleVolumeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setVolume(parseFloat(e.target.value));
    };

    const handleSeek = (newValue: number) => {
        seek(newValue);
    };

    const currentIndex = getCurrentTrackIndex();
    const { queue } = usePlayerStore();
    const hasPrev = currentIndex > 0;
    const hasNext = currentIndex >= 0 && currentIndex < queue.length - 1;

    // Find the current track in the library to get authoritative metadata (like cover_image path)
    const libIndex = library.findIndex(t => t.path === track?.path);
    const currentLibraryTrack = libIndex >= 0 ? library[libIndex] : null;

    // Determine Cover URL
    const isYtUrl = currentLibraryTrack?.cover_url !== undefined || track?.cover_url !== undefined;
    // Prefer library cover image if available, falling back to track's cover image
    const coverImageToLoad = currentLibraryTrack?.cover_image || track?.cover_image;

    // Only call useCoverArt if we have a local cover image and NOT a YT url
    const localCoverUrl = useCoverArt(coverImageToLoad);

    const activeCoverUrl = (isYtUrl ? (currentLibraryTrack?.cover_url || track?.cover_url) : localCoverUrl);

    const [isHovered, setIsHovered] = useState(false);
    const [showStats, setShowStats] = useState(false);
    const hoverTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    const handleMouseEnter = () => {
        if (hoverTimeoutRef.current) {
            clearTimeout(hoverTimeoutRef.current);
            hoverTimeoutRef.current = null;
        }
        setIsHovered(true);
    };

    const handleMouseLeave = () => {
        hoverTimeoutRef.current = setTimeout(() => {
            setIsHovered(false);
        }, 300); // 300ms buffer to prevent glitching
    };

    return (
        <div className="absolute bottom-6 left-0 right-0 z-50 flex flex-col items-center justify-end pointer-events-none gap-4">
            {/* Error Toast */}
            <AnimatePresence>
                {error && (
                    <motion.div
                        initial={{ opacity: 0, y: 20, scale: 0.9 }}
                        animate={{ opacity: 1, y: 0, scale: 1 }}
                        exit={{ opacity: 0, y: 10, scale: 0.95 }}
                        className="bg-red-500 text-white px-6 py-3 rounded-full shadow-elevation-3 font-medium text-body-medium flex items-center gap-3 pointer-events-auto"
                    >
                        <span>⚠️ {error}</span>
                        <button onClick={() => setError(null)} className="opacity-80 hover:opacity-100">✕</button>
                    </motion.div>
                )}
            </AnimatePresence>

            {/* Container for Pill and Side Buttons */}
            <div className="flex items-center justify-center gap-6 w-full px-8">
                {/* Previous Button */}
                <AnimatePresence mode="popLayout">
                    {!isHovered && (
                        <ExpressiveControlButton
                            onClick={prevTrack}
                            disabled={!hasPrev}
                            icon={<IconPrevious size={24} />}
                            direction="left"
                        />
                    )}
                </AnimatePresence>

                {/* Player Container */}
                <motion.div
                    layout
                    onMouseEnter={handleMouseEnter}
                    onMouseLeave={handleMouseLeave}
                    initial={false}
                    animate={{
                        width: isHovered ? '100%' : '25rem',
                        height: isHovered ? '6rem' : '4.5rem',
                        borderRadius: '9999px',
                        maxWidth: isHovered ? '100%' : '23rem',
                        backgroundColor: isHovered
                            ? 'rgba(0,0,0,0.4)'
                            : 'var(--md-sys-color-surface-container-high)',
                        borderColor: isHovered ? 'rgba(255,255,255,0.1)' : 'rgba(255,255,255,0.05)',
                        boxShadow: isHovered
                            ? '0 20px 40px -10px rgba(0,0,0,0.5), 0 0 20px -5px var(--md-sys-color-primary)'
                            : '0 10px 30px -10px rgba(0,0,0,0.4)'
                    }}
                    transition={{ type: "spring", stiffness: 100, damping: 15, mass: 1 }}
                    className={`
                    pointer-events-auto
                    relative 
                    z-20
                    text-on-surface
                    overflow-hidden
                    isolation-isolate
                    flex items-center
                    backdrop-blur-xl
                    border
                    ${isHovered ? 'px-4 py-4 gap-8' : 'px-3 py-3 gap-5'}
                `}
                >
                    {/* Background Art Overlay (Expanded Mode - Smoother Bleed) */}
                    <AnimatePresence>
                        {isHovered && expandedArtMode === 'background' && activeCoverUrl && (
                            <motion.div
                                initial={{ opacity: 0 }}
                                animate={{ opacity: 1 }}
                                exit={{ opacity: 0 }}
                                className="absolute inset-0 z-0 pointer-events-none overflow-hidden"
                            >
                                {/* Layer 1: Blurred base for smooth bleed */}
                                <div className="absolute inset-0">
                                    <img
                                        src={activeCoverUrl}
                                        alt=""
                                        className="w-full h-full object-cover opacity-30 blur-2xl scale-110"
                                    />
                                </div>

                                {/* Layer 2: Main art with gradient mask */}
                                <div className="absolute left-0 top-0 bottom-0 w-full overflow-hidden">
                                    <img
                                        src={activeCoverUrl}
                                        alt=""
                                        className="h-full w-1/2 object-cover opacity-40 transition-opacity duration-500"
                                        style={{
                                            maskImage: 'linear-gradient(to right, black 20%, transparent 90%)',
                                            WebkitMaskImage: 'linear-gradient(to right, black 20%, transparent 90%)'
                                        }}
                                    />
                                    {/* Overlay to ensure controls are legible */}
                                    <div className="absolute inset-0 bg-gradient-to-r from-transparent via-surface-container-high/20 to-surface-container-high" />
                                </div>
                            </motion.div>
                        )}
                    </AnimatePresence>

                    <AnimatePresence>
                        {!isHovered && track && (
                            <motion.div
                                key="progress-wave"
                                initial={{ opacity: 0 }}
                                animate={{ opacity: 1 }}
                                exit={{ opacity: 0 }}
                                transition={{ duration: 0.3 }}
                                className="absolute left-0 top-0 bottom-0 z-[1] pointer-events-none flex items-stretch text-secondary-container rounded-l-full"
                                style={{
                                    width: `${(position_secs / (track.duration_secs || 1)) * 100}%`
                                }}
                            >
                                {/* Solid fill part */}
                                <div className="flex-1 bg-current" />

                                {/* Vertical Wave Edge */}
                                <div className="w-[12px] h-full shrink-0 overflow-hidden relative -mr-[1px]">
                                    <motion.div
                                        className="w-full absolute top-0 left-0"
                                        style={{ height: 'calc(100% + 40px)', top: '-40px' }}
                                        animate={state === 'Playing' ? { y: ['0px', '40px'] } : { y: '0px' }}
                                        transition={{
                                            duration: 1,
                                            ease: "linear",
                                            repeat: Infinity
                                        }}
                                    >
                                        <svg
                                            className="h-full w-full"
                                            preserveAspectRatio="none"
                                        >
                                            <defs>
                                                <pattern id="pill-progress-wave" x="0" y="0" width="12" height="40" patternUnits="userSpaceOnUse">
                                                    <motion.path
                                                        initial={false}
                                                        animate={{
                                                            d: state === 'Playing'
                                                                ? "M 0 0 L 6 0 Q 1 10 6 20 T 6 40 L 0 40 Z"
                                                                : "M 0 0 L 6 0 Q 6 10 6 20 T 6 40 L 0 40 Z"
                                                        }}
                                                        transition={{ duration: 0.3 }}
                                                        fill="currentColor"
                                                    />
                                                </pattern>
                                            </defs>
                                            <rect x="0" y="0" width="100%" height="100%" fill="url(#pill-progress-wave)" />
                                        </svg>
                                    </motion.div>
                                </div>
                            </motion.div>
                        )}
                    </AnimatePresence>

                    <AnimatePresence mode="wait">
                        {isHovered ? (
                            /* EXPANDED LAYOUT */
                            <motion.div
                                key="expanded"
                                initial={{ opacity: 0, scale: 0.95 }}
                                animate={{ opacity: 1, scale: 1 }}
                                exit={{ opacity: 0, scale: 0.95 }}
                                transition={{ duration: 0.2 }}
                                className="flex w-full items-center gap-6 relative z-10 h-full"
                            >
                                {/* Left: Info */}
                                <div className="flex items-center gap-4 flex-1 min-w-0">
                                    {/* Cover Art - Only show in pill mode or if background art is disabled */}
                                    {expandedArtMode === 'pill' && (
                                        <div
                                            className={`
                                            relative shrink-0 overflow-hidden bg-surface-container-low shadow-sm transition-all duration-300 group cursor-pointer
                                            ${albumArtStyle === 'vinyl' ? 'w-16 h-16 rounded-full' : 'w-16 h-16 rounded-xl'}
                                        `}
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                if (track) toggleFavorite(track.path);
                                            }}
                                        >
                                            {activeCoverUrl ? (
                                                <img
                                                    src={activeCoverUrl}
                                                    alt=""
                                                    className={`
                                                    w-full h-full object-cover
                                                    ${albumArtStyle === 'vinyl' && state === 'Playing' ? 'animate-spin-slow' : ''}
                                                `}
                                                />
                                            ) : (
                                                <div className="w-full h-full flex items-center justify-center text-on-surface-variant">
                                                    <IconMusicNote size={24} />
                                                </div>
                                            )}

                                            {/* Vinyl Center Hole */}
                                            {albumArtStyle === 'vinyl' && (
                                                <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
                                                    <div className="w-4 h-4 bg-surface-container-high rounded-full border border-surface-container-low/50" />
                                                </div>
                                            )}

                                            {/* Favorite Overlay */}
                                            <div className="absolute inset-0 bg-black/40 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity z-20">
                                                <AnimatePresence mode="wait">
                                                    {track && favorites.has(track.path) ? (
                                                        <motion.div
                                                            key="fav-filled"
                                                            initial={{ scale: 0.5, opacity: 0 }}
                                                            animate={{ scale: 1, opacity: 1 }}
                                                            exit={{
                                                                scale: 1.5,
                                                                opacity: 0,
                                                                rotate: [-10, 10, -10, 10, 0], // Shake/Break effect
                                                                filter: "blur(4px)"
                                                            }}
                                                            transition={{ duration: 0.3 }}
                                                        >
                                                            <IconHeart size={28} filled={true} className="text-error" />
                                                        </motion.div>
                                                    ) : (
                                                        <motion.div
                                                            key="fav-outline"
                                                            initial={{ scale: 0.8, opacity: 0 }}
                                                            animate={{ scale: 1, opacity: 1 }}
                                                            exit={{ scale: 0.5, opacity: 0 }}
                                                            transition={{ duration: 0.2 }}
                                                        >
                                                            <IconHeart size={28} filled={false} className="text-white" />
                                                        </motion.div>
                                                    )}
                                                </AnimatePresence>
                                            </div>
                                        </div>
                                    )}

                                    <div className="flex flex-col min-w-0 justify-center">
                                        <div className="text-title-medium font-bold truncate  text-on-surface">
                                            <MarqueeText text={track?.title || "Not Playing"} />
                                        </div>
                                        <div className="text-body-medium text-on-surface-variant truncate">
                                            {track?.artist || "Pick a song"}
                                        </div>
                                    </div>
                                </div>

                                {/* Center: Controls */}
                                <div className="flex flex-col items-center justify-center flex-[2] gap-1 relative z-20">
                                    <div className="flex items-center justify-center gap-6">
                                        <button onClick={prevTrack} disabled={!hasPrev} className="text-on-surface-variant hover:text-on-surface disabled:opacity-30 p-2 rounded-full hover:bg-surface-container-highest">
                                            <IconPrevious size={28} />
                                        </button>
                                        <button
                                            onClick={handlePlayPause}
                                            className="w-14 h-14 bg-primary text-on-primary rounded-2xl flex items-center justify-center hover:bg-primary/90 shadow-elevation-1 transition-transform active:scale-95"
                                        >
                                            {state === 'Playing' ? (
                                                <IconPause size={32} />
                                            ) : (
                                                <IconPlay size={32} />
                                            )}
                                        </button>
                                        <button onClick={nextTrack} disabled={!hasNext} className="text-on-surface-variant hover:text-on-surface disabled:opacity-30 p-2 rounded-full hover:bg-surface-container-highest">
                                            <IconNext size={28} />
                                        </button>
                                    </div>

                                    {/* Seeker */}
                                    <div className="w-full flex items-center gap-3 text-label-small font-medium text-on-surface-variant/80">
                                        <span className="w-10 text-right">{formatTime(position_secs)}</span>
                                        <div className="flex-1 h-6 relative flex items-center">
                                            <SquigglySlider
                                                value={position_secs}
                                                max={track?.duration_secs || 100}
                                                onChange={handleSeek}
                                                isPlaying={state === 'Playing'}
                                                accentColor="var(--md-sys-color-primary)"
                                                className="w-full"
                                            />
                                        </div>
                                        <span className="w-10">{track ? formatTime(track.duration_secs) : '0:00'}</span>
                                    </div>
                                </div>

                                {/* Right: Actions */}
                                <div className="flex items-center gap-2 flex-1 justify-end">
                                    {/* Stats Button */}
                                    {track && (
                                        <button
                                            onClick={() => setShowStats(true)}
                                            className="p-2 rounded-full transition-colors text-on-surface-variant hover:text-on-surface hover:bg-surface-container-highest"
                                            title="View Song Analysis"
                                        >
                                            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                                                <polyline points="12 3 20 7.5 20 16.5 12 21 4 16.5 4 7.5 12 3" />
                                                <polyline points="12 12 20 7.5" />
                                                <polyline points="12 21 12 12" />
                                                <polyline points="4 7.5 12 12" />
                                            </svg>
                                        </button>
                                    )}


                                    {/* Shuffle Button */}
                                    <button
                                        onClick={toggleShuffle}
                                        className={`p-2 rounded-full transition-colors ${isShuffled
                                            ? 'text-primary bg-primary-container'
                                            : 'text-on-surface-variant hover:text-on-surface hover:bg-surface-container-highest'
                                            }`}
                                        title={isShuffled ? 'Shuffle On' : 'Shuffle Off'}
                                    >
                                        <IconShuffle size={22} />
                                    </button>

                                    {/* Repeat Button */}
                                    <button
                                        onClick={cycleRepeatMode}
                                        className={`p-2 rounded-full transition-colors ${repeatMode !== 'off'
                                            ? 'text-primary bg-primary-container'
                                            : 'text-on-surface-variant hover:text-on-surface hover:bg-surface-container-highest'
                                            }`}
                                        title={`Repeat: ${repeatMode === 'off' ? 'Off' : repeatMode === 'all' ? 'All' : 'One'}`}
                                    >
                                        <IconRepeat size={22} mode={repeatMode} />
                                    </button>

                                    <div className="flex items-center gap-2 group relative">
                                        <IconVolume size={24} className="text-on-surface-variant" />
                                        <input
                                            type="range" min="0" max="1" step="0.01" value={volume}
                                            onChange={handleVolumeChange}
                                            className="w-24 accent-primary"
                                        />
                                    </div>
                                </div>
                            </motion.div>
                        ) : (
                            /* MINIMAL LAYOUT */
                            <motion.div
                                key="minimal"
                                initial={{ opacity: 0, scale: 0.95 }}
                                animate={{ opacity: 1, scale: 1 }}
                                exit={{ opacity: 0, scale: 0.95 }}
                                transition={{ duration: 0.2 }}
                                className="flex w-full items-center gap-4 relative z-10"
                            >
                                {/* Cover Art (Tiny) */}
                                <div
                                    className={`
                                    relative shrink-0 overflow-hidden bg-surface-container-low border border-outline-variant/20 transition-all duration-300
                                    ${albumArtStyle === 'vinyl' ? 'w-10 h-10 rounded-full' : 'w-10 h-10 rounded-sm'}
                                `}
                                >
                                    {activeCoverUrl ? (
                                        <img
                                            src={activeCoverUrl}
                                            alt=""
                                            className={`
                                            w-full h-full object-cover
                                            ${albumArtStyle === 'vinyl' && state === 'Playing' ? 'animate-spin-slow' : ''}
                                        `}
                                        />
                                    ) : (
                                        <div className="w-full h-full flex items-center justify-center text-on-surface-variant">
                                            <IconMusicNote size={16} />
                                        </div>
                                    )}

                                    {/* Vinyl Center Hole (Minimal) */}
                                    {albumArtStyle === 'vinyl' && (
                                        <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
                                            <div className="w-2 h-2 bg-surface-container-high rounded-full border border-surface-container-low/30" />
                                        </div>
                                    )}
                                </div>

                                {/* Text Info */}
                                <div className="flex-1 min-w-0 flex flex-col justify-center overflow-hidden z-10">
                                    <div className="text-label-large font-bold truncate text-on-surface w-full">
                                        <MarqueeText text={track?.title || "Not Playing"} />
                                    </div>
                                    <div className="text-label-small text-on-surface-variant truncate w-full">
                                        {track?.artist || "Artist"}
                                    </div>
                                </div>

                                {/* Time */}
                                <div className="text-label-small font-medium text-on-surface-variant tabular-nums z-10">
                                    {formatTime(position_secs)} / {track ? formatTime(track.duration_secs) : '0:00'}
                                </div>

                                {/* Progress Indicator (Background or Border?) */}
                                {/* Optional: Add a subtle progress bar at the bottom? */}
                            </motion.div>
                        )}
                    </AnimatePresence>
                </motion.div>

                {/* Next Button */}
                <AnimatePresence mode="popLayout">
                    {!isHovered && (
                        <ExpressiveControlButton
                            onClick={nextTrack}
                            disabled={!hasNext}
                            icon={<IconNext size={24} />}
                            direction="right"
                        />
                    )}
                </AnimatePresence>
            </div >

            {/* Track Stats Modal */}
            <AnimatePresence>
                {showStats && track && (
                    <TrackStatsModal
                        track={track}
                        onClose={() => setShowStats(false)}
                    />
                )}
            </AnimatePresence>
        </div >
    );
}

/// Modal wrapper for TrackStats that fetches features
function TrackStatsModal({ track, onClose }: { track: any; onClose: () => void }) {
    const [features, setFeatures] = useState<any>(null);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        const loadFeatures = async () => {
            try {
                const result = await invoke('get_track_audio_features', {
                    path: track.path,
                });
                setFeatures(result);
                setError(null);
            } catch (err) {
                setError(String(err));
                setFeatures(null);
            } finally {
                setIsLoading(false);
            }
        };
        loadFeatures();
    }, [track.path]);

    // Handle escape key
    useEffect(() => {
        const handleEscape = (e: KeyboardEvent) => {
            if (e.key === 'Escape') {
                onClose();
            }
        };
        window.addEventListener('keydown', handleEscape);
        return () => window.removeEventListener('keydown', handleEscape);
    }, [onClose]);

    if (isLoading) {
        return (
            <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm pointer-events-auto" onClick={onClose}>
                <div className="flex flex-col items-center gap-4 pointer-events-none">
                    <div className="animate-spin w-10 h-10 border-4 border-primary border-t-transparent rounded-full" />
                    <p className="text-on-surface text-body-medium">Analyzing audio features...</p>
                </div>
            </div>
        );
    }

    if (error || !features) {
        return (
            <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm pointer-events-auto" onClick={onClose}>
                <div className="flex flex-col items-center gap-4 bg-surface-container rounded-2xl p-6 max-w-md mx-4 pointer-events-auto">
                    <p className="text-error text-body-large font-medium">Analysis Failed</p>
                    <p className="text-on-surface-variant text-body-medium text-center">{error || 'Could not load audio features'}</p>
                    <button
                        onClick={onClose}
                        className="px-6 py-2 rounded-full bg-primary text-on-primary font-semibold hover:bg-primary/90"
                    >
                        Close
                    </button>
                </div>
            </div>
        );
    }

    return (
        <div className="fixed inset-0 z-50 pointer-events-auto" onClick={onClose}>
            <TrackStats
                features={features}
                trackTitle={track.title}
                trackArtist={track.artist}
                onClose={onClose}
            />
        </div>
    );
}
