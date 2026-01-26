import { motion } from 'motion/react';
import { usePlayerStore } from '../store/playerStore';
import { useCoverArt } from '../hooks/useCoverArt';
import { useThemeStore } from '../store/themeStore';
import { IconClose, IconPlay, IconPause, IconNext, IconPrevious, IconMusicNote, IconShuffle, IconRepeat, IconHeart, IconVolume } from './Icons';
import { SquigglySlider } from './SquigglySlider';
import { SideLyrics } from './SideLyrics';
import { QueueItem } from './RightPanel';

export function ImmersiveView() {
    const { status, toggleImmersiveMode, toggleShuffle, isShuffled, pause, resume, nextTrack, prevTrack, seek, library, queue, playFile, repeatMode, cycleRepeatMode } = usePlayerStore();
    const { track, state, position_secs } = status;

    // Consume global colors instead of re-extracting
    const colors = useThemeStore(state => state.colors);

    // Get correct cover from library (status.track might not have full path)
    const currentIndex = library.findIndex(t => t.path === track?.path);
    const currentLibraryTrack = currentIndex >= 0 ? library[currentIndex] : null;
    const coverUrl = useCoverArt(currentLibraryTrack?.cover_image || track?.cover_image);

    const isPlaying = state === 'Playing';

    return (
        <motion.div
            initial={{ opacity: 0, y: '100%' }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: '100%' }}
            transition={{ type: 'spring', damping: 25, stiffness: 200 }}
            className="fixed inset-0 z-[100] flex flex-col items-center justify-center overflow-hidden"
            style={{ backgroundColor: colors.surfaceContainerLowest }}
        >
            {/* Close Button */}
            <button
                onClick={toggleImmersiveMode}
                className="absolute top-8 right-8 z-50 p-4 rounded-full transition-colors hover:bg-black/10"
                style={{ color: colors.onSurface }}
            >
                <IconClose size={32} />
            </button>

            {/* Content Container - Grid Layout */}
            <div className="relative z-10 w-full h-full flex flex-col xl:flex-row gap-6 p-6 md:p-12 overflow-hidden">

                {/* Left: Queue (Visible on XL screens) */}
                <div
                    className="hidden xl:flex flex-col w-80 shrink-0 gap-4 overflow-hidden rounded-2xl border p-4"
                    style={{
                        backgroundColor: colors.surfaceContainer,
                        borderColor: colors.outlineVariant
                    }}
                >
                    <h3 className="text-title-medium font-bold px-2" style={{ color: colors.onSurface }}>Up Next</h3>
                    <div className="flex-1 overflow-y-auto no-scrollbar space-y-2">
                        {queue.map((t, i) => (
                            <QueueItem
                                key={`${t.path}-${i}`}
                                track={t}
                                isActive={t.path === track?.path}
                                onClick={() => playFile(t.path)}
                            />
                        ))}
                    </div>
                </div>

                {/* Center: Main Content */}
                <div className="flex-1 flex flex-col items-center justify-center min-w-0">
                    <div className="flex flex-col items-center justify-center gap-8 md:gap-12 w-full max-w-2xl">

                        {/* Album Art */}
                        <div className="relative group shrink-0">
                            <motion.div
                                layoutId="immersive-art"
                                className="w-[30vh] h-[30vh] md:w-[40vh] md:h-[40vh] rounded-[2rem] md:rounded-[3rem] overflow-hidden"
                                style={{ backgroundColor: colors.surfaceContainerHigh }}
                            >
                                {coverUrl ? (
                                    <img
                                        src={coverUrl}
                                        alt={track?.album}
                                        className="w-full h-full object-cover"
                                    />
                                ) : (
                                    <div
                                        className="w-full h-full flex items-center justify-center"
                                        style={{ backgroundColor: colors.surfaceContainerHighest, color: colors.onSurfaceVariant }}
                                    >
                                        <IconMusicNote size={120} />
                                    </div>
                                )}
                            </motion.div>
                        </div>

                        {/* Metadata & Controls */}
                        <div className="flex flex-col items-center text-center w-full">

                            {/* Text Info */}
                            <div className="mb-8 space-y-2">
                                <motion.h1
                                    className="text-3xl md:text-5xl font-black tracking-tight leading-tight line-clamp-2"
                                    style={{ color: colors.onSurface }}
                                >
                                    {track?.title || "Not Playing"}
                                </motion.h1>
                                <motion.h2
                                    className="text-xl md:text-2xl font-medium opacity-80 line-clamp-1"
                                    style={{ color: colors.onSurfaceVariant }}
                                >
                                    {track?.artist || "Unknown Artist"}
                                </motion.h2>
                            </div>

                            {/* Progress Bar (Squiggly) */}
                            <div className="w-full mb-8 max-w-lg">
                                <SquigglySlider
                                    value={position_secs}
                                    max={track?.duration_secs || 100}
                                    onChange={(val) => seek(val)}
                                    isPlaying={isPlaying}
                                    className="h-12 w-full cursor-pointer"
                                    accentColor={colors.primary}
                                />
                                <div className="flex justify-between mt-2 text-sm font-medium opacity-60" style={{ color: colors.onSurfaceVariant }}>
                                    <span>{formatTime(position_secs)}</span>
                                    <span>{formatTime(track?.duration_secs || 0)}</span>
                                </div>
                            </div>

                            {/* Playback Controls */}
                            <div className="flex flex-col items-center gap-6 w-full">
                                {/* Main Transport Controls */}
                                <div className="flex items-center gap-8 md:gap-12">
                                    <button
                                        onClick={toggleShuffle}
                                        className="p-3 rounded-full transition-transform active:scale-95 hover:bg-black/5 relative group"
                                        style={{ color: isShuffled ? colors.primary : colors.onSurfaceVariant }}
                                        title={isShuffled ? 'Shuffle On' : 'Shuffle Off'}
                                    >
                                        <IconShuffle size={28} />
                                    </button>

                                    <button
                                        onClick={prevTrack}
                                        className="p-3 rounded-full transition-transform active:scale-95 hover:bg-black/5"
                                        style={{ color: colors.onSurface }}
                                    >
                                        <IconPrevious size={42} />
                                    </button>

                                    <button
                                        onClick={isPlaying ? pause : resume}
                                        className="p-6 rounded-[2.5rem] shadow-xl hover:scale-105 active:scale-95 transition-all"
                                        style={{
                                            backgroundColor: colors.primary,
                                            color: colors.onPrimary,
                                            boxShadow: `0 8px 24px -6px ${colors.primary}60`
                                        }}
                                    >
                                        {isPlaying ? <IconPause size={48} /> : <IconPlay size={48} />}
                                    </button>

                                    <button
                                        onClick={nextTrack}
                                        className="p-3 rounded-full transition-transform active:scale-95 hover:bg-black/5"
                                        style={{ color: colors.onSurface }}
                                    >
                                        <IconNext size={42} />
                                    </button>

                                    <button
                                        onClick={cycleRepeatMode}
                                        className="p-3 rounded-full transition-transform active:scale-95 hover:bg-black/5 relative group"
                                        style={{ color: repeatMode !== 'off' ? colors.primary : colors.onSurfaceVariant }}
                                        title={`Repeat: ${repeatMode}`}
                                    >
                                        <IconRepeat size={28} mode={repeatMode} />
                                    </button>
                                </div>

                                {/* Secondary Controls Row */}
                                <div className="flex items-center gap-6 md:gap-10 mt-2">
                                    {/* Favorite Toggle */}
                                    <button
                                        onClick={() => usePlayerStore.getState().toggleFavorite(track?.path || '')}
                                        className="p-3 rounded-full hover:bg-black/5 transition-colors"
                                        style={{ color: usePlayerStore.getState().isFavorite(track?.path || '') ? colors.tertiary : colors.onSurfaceVariant }}
                                        title="Toggle Favorite"
                                    >
                                        <IconHeart size={26} filled={usePlayerStore.getState().isFavorite(track?.path || '')} />
                                    </button>

                                    {/* Volume Slider (Compact) */}
                                    <div className="flex items-center gap-3 group">
                                        <IconVolume size={24} style={{ color: colors.onSurfaceVariant }} />
                                        <input
                                            type="range"
                                            min="0" max="1" step="0.01"
                                            value={status.volume}
                                            onChange={(e) => usePlayerStore.getState().setVolume(parseFloat(e.target.value))}
                                            className="w-24 accent-current h-1 rounded-full opacity-60 group-hover:opacity-100 transition-opacity"
                                            style={{ accentColor: colors.primary }}
                                        />
                                    </div>

                                    {/* Show Lyrics/Queue Toggle (Mobile/Small screens only - Logic TBD, for now just an icon) */}
                                    {/* We can reuse the SideLyrics logic or just show this as a visual placeholder for 'more' options if needed, but per request we added the main ones. */}
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                {/* Right: Lyrics (Visible on XL screens) */}
                <div
                    className="hidden xl:flex flex-col w-96 shrink-0 overflow-hidden rounded-2xl border relative"
                    style={{
                        backgroundColor: colors.surfaceContainer,
                        borderColor: colors.outlineVariant
                    }}
                >
                    <SideLyrics variant="carousel" />
                </div>

            </div>
        </motion.div>
    );
}

function formatTime(seconds: number) {
    if (!seconds || isNaN(seconds)) return "0:00";
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
}
