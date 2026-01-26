import { SideLyrics } from './SideLyrics';
import { useLyricsStore } from '../store/lyricsStore';
import { AnimatePresence, motion } from 'motion/react';
import { useEffect, useState } from 'react';
import { usePlayerStore } from '../store/playerStore';
import { useNavigationStore } from '../store/navigationStore';
import { useCoverArt } from '../hooks/useCoverArt';
import { IconMusicNote, IconPlay, IconQueue, IconAlbum, IconLyrics, IconFullscreen } from './Icons';
import { MarqueeText } from './MarqueeText';
import { SquigglySlider } from './SquigglySlider';

export function RightPanel() {
    const { status, queue, playFile, toggleImmersiveMode } = usePlayerStore();
    const { lines, plainLyrics, isInstrumental, isLoading, fetchLyrics, error } = useLyricsStore();
    const { track } = status;
    const { navigateToAlbum } = useNavigationStore();

    // Clover Shape from TitleBar
    const CloverIcon = ({ children, active, color }: { children: React.ReactNode, active: boolean, color: string }) => (
        <div className="relative w-8 h-8 flex items-center justify-center">
            {active && (
                <svg viewBox="0 0 280 280" className="absolute inset-0 w-full h-full opacity-20" style={{ color }}>
                    <path d="M178.73 6.2068C238.87 -19.9132 299.91 41.1269 273.79 101.267L269.47 111.207C261.5 129.577 261.5 150.417 269.47 168.787L273.79 178.727C299.91 238.867 238.87 299.907 178.73 273.787L168.79 269.467C150.42 261.497 129.58 261.497 111.21 269.467L101.27 273.787C41.1281 299.907 -19.9139 238.867 6.20706 178.727L10.5261 168.787C18.5011 150.417 18.5011 129.577 10.5261 111.207L6.20706 101.267C-19.9139 41.1269 41.1281 -19.9132 101.27 6.2068L111.21 10.5269C129.58 18.4969 150.42 18.4969 168.79 10.5269L178.73 6.2068Z" fill="currentColor" />
                </svg>
            )}
            <div className="relative z-10" style={{ color: active ? 'var(--md-sys-color-primary)' : 'var(--md-sys-color-on-surface-variant)' }}>
                {children}
            </div>
        </div>
    );

    // Fetch lyrics automatically when track changes
    useEffect(() => {
        if (track?.path) {
            console.log('[RightPanel] Track changed:', track.title);
            console.log('[RightPanel] Triggering fetchLyrics for:', track.path);
            // We use fetchLyrics (not loadCachedLyrics) to ensure we actually trigger a fetch if not cached
            fetchLyrics(
                track.artist,
                track.title,
                track.duration_secs,
                track.path
            );
        }
    }, [track?.path, track?.title, track?.artist, track?.duration_secs, fetchLyrics]);

    // Debug logging for state changes
    useEffect(() => {
        console.log('[RightPanel] Lyrics updated:', {
            hasLines: !!lines,
            hasPlain: !!plainLyrics,
            instrumental: isInstrumental,
            loading: isLoading,
            error: error
        });
    }, [lines, plainLyrics, isInstrumental, isLoading, error]);

    // Get cover from library
    const { library } = usePlayerStore();
    const currentIndex = library.findIndex(t => t.path === track?.path);
    const currentLibraryTrack = currentIndex >= 0 ? library[currentIndex] : null;
    const coverUrl = useCoverArt(currentLibraryTrack?.cover_image);

    // Determine what to show in the bottom section
    const hasContent = (lines && lines.length > 0) || (plainLyrics && plainLyrics.trim().length > 0);

    // Ideal state: Show if loading OR (valid content AND not instrumental AND no error)
    const shouldShowLyricsIdeally = isLoading || (hasContent && !isInstrumental && !error);

    const [showLyrics, setShowLyrics] = useState<boolean>(!!shouldShowLyricsIdeally);

    useEffect(() => {
        if (shouldShowLyricsIdeally) {
            setShowLyrics(true);
        } else {
            // Keep showing if we have a specific error message to let user read it
            if (error && error.includes("recents view")) {
                const timer = setTimeout(() => setShowLyrics(false), 3000);
                return () => clearTimeout(timer);
            } else {
                setShowLyrics(false);
            }
        }
    }, [shouldShowLyricsIdeally, error]);

    const handleArtClick = () => {
        if (track?.album && track?.artist) {
            navigateToAlbum(track.album, track.artist);
        }
    };

    return (
        <aside className="h-full flex flex-col p-6 gap-6 overflow-hidden">
            {/* Now Playing Header */}
            <div className="flex items-center justify-between shrink-0">
                <h2 className="text-title-medium font-bold text-on-surface">Now Playing</h2>
                <button
                    onClick={toggleImmersiveMode}
                    className="p-2 -mr-2 rounded-full hover:bg-surface-container-highest transition-colors text-on-surface-variant hover:text-on-surface"
                    title="Immersive Mode"
                >
                    <IconFullscreen size={20} />
                </button>
            </div>

            {/* Main Art & Info - UPDATED: Larger Art, No Album Text, Clickable */}
            <div className="flex flex-col items-center gap-6 shrink-0 transition-all duration-300">
                {/* Large Art (Increased size to w-72 h-72 approx / allow scaling) */}
                <div
                    onClick={handleArtClick}
                    className="w-72 h-72 rounded-[2rem] bg-surface-container-high shadow-elevation-2 relative group overflow-hidden shrink-0 cursor-pointer hover:scale-[1.02] active:scale-95 transition-transform duration-200"
                    title={track?.album ? `Go to album: ${track.album}` : "Album Art"}
                >
                    {coverUrl ? (
                        <img
                            src={coverUrl}
                            alt={track?.album || "Album Art"}
                            className="w-full h-full object-cover"
                        />
                    ) : (
                        <div className="w-full h-full flex items-center justify-center text-on-surface-variant/50">
                            <IconMusicNote size={64} />
                        </div>
                    )}

                    {/* Hover Hint */}
                    {track?.album && (
                        <div className="absolute inset-0 bg-black/20 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
                            <IconAlbum size={32} className="text-white drop-shadow-lg" />
                        </div>
                    )}
                </div>

                {/* Info */}
                <div className="flex flex-col items-center text-center gap-1 w-full px-2">
                    <div className="w-full text-headline-medium font-bold text-on-surface truncate">
                        <MarqueeText text={track?.title || "Not Playing"} />
                    </div>
                    <div className="text-title-large text-on-surface-variant truncate w-full font-medium">
                        {track?.artist || "Pick a song"}
                    </div>
                    {/* Album text removed as requested */}
                </div>
            </div>

            {/* Interactive Separator / Seeker */}
            <div className="px-8 py-2 w-full">
                <SquigglySlider
                    value={status.position_secs}
                    max={status.track?.duration_secs || 100}
                    onChange={(val) => usePlayerStore.getState().seek(val)}
                    isPlaying={status.state === 'Playing'}
                    className="h-6 w-full cursor-pointer text-primary hover:text-primary-container transition-colors"
                    accentColor="currentColor"
                />
            </div>

            {/* Content Switcher: Lyrics or Queue */}
            <div className="flex-1 min-h-0 relative">
                <AnimatePresence mode="wait">
                    {showLyrics ? (
                        <motion.div
                            key="lyrics"
                            initial={{ opacity: 0, scale: 0.95 }}
                            animate={{ opacity: 1, scale: 1 }}
                            exit={{ opacity: 0, scale: 0.95 }}
                            transition={{ duration: 0.3, ease: 'backOut' }}
                            className="absolute inset-0 flex flex-col"
                        >
                            <div className="flex items-center justify-between mb-2 px-1">
                                <div className="flex items-center gap-2">
                                    <span className="w-2 h-2 rounded-full bg-primary animate-pulse" />
                                    <h3 className="text-title-small font-semibold text-on-surface">Lyrics</h3>
                                </div>
                                <div className="flex items-center gap-1 bg-surface-container rounded-full p-1">
                                    <button
                                        onClick={() => setShowLyrics(false)}
                                        className="transition-all duration-200"
                                        title="Show Queue"
                                    >
                                        <CloverIcon active={!showLyrics} color="var(--md-sys-color-primary)">
                                            <IconQueue size={18} />
                                        </CloverIcon>
                                    </button>
                                    <button
                                        onClick={() => setShowLyrics(true)}
                                        className="transition-all duration-200"
                                        title="Show Lyrics"
                                    >
                                        <CloverIcon active={showLyrics} color="var(--md-sys-color-primary)">
                                            <IconLyrics size={18} />
                                        </CloverIcon>
                                    </button>
                                </div>
                            </div>

                            {/* Window-like container */}
                            <div className="flex-1 rounded-2xl bg-surface-container-low overflow-hidden relative flex flex-col group">
                                <SideLyrics />
                            </div>
                        </motion.div>
                    ) : (
                        <motion.div
                            key="queue"
                            initial={{ opacity: 0, y: 20 }}
                            animate={{ opacity: 1, y: 0 }}
                            exit={{ opacity: 0, y: -20 }}
                            className="absolute inset-0 flex flex-col"
                        >
                            <div className="flex items-center justify-between mb-4 px-1">
                                <h3 className="text-title-small font-semibold text-on-surface-variant/80">Queue</h3>
                                <div className="flex items-center gap-1 bg-surface-container rounded-full p-1">
                                    <button
                                        onClick={() => setShowLyrics(false)}
                                        className="transition-all duration-200"
                                        title="Show Queue"
                                    >
                                        <CloverIcon active={!showLyrics} color="var(--md-sys-color-primary)">
                                            <IconQueue size={18} />
                                        </CloverIcon>
                                    </button>
                                    <button
                                        onClick={() => setShowLyrics(true)}
                                        className="transition-all duration-200"
                                        title="Show Lyrics"
                                    >
                                        <CloverIcon active={showLyrics} color="var(--md-sys-color-primary)">
                                            <IconLyrics size={18} />
                                        </CloverIcon>
                                    </button>
                                </div>
                            </div>
                            <div className="flex-1 overflow-y-auto scrollbar-thin scrollbar-thumb-surface-container-high gap-2 flex flex-col pb-4">
                                {queue.length === 0 && (
                                    <div className="p-4 rounded-xl bg-surface-container-high/50 text-center">
                                        <p className="text-body-small text-on-surface-variant">Queue is empty</p>
                                    </div>
                                )}

                                {queue.map((t, i) => (
                                    <QueueItem
                                        key={`${t.path}-${i}`} // Use index in key to handle duplicate tracks in queue if we support that later
                                        track={t}
                                        isActive={!!(track && t.path === track.path)}
                                        onClick={() => playFile(t.path)}
                                    />
                                ))}
                            </div>
                        </motion.div>
                    )}
                </AnimatePresence>
            </div>
        </aside>
    );
}

export function QueueItem({ track, isActive, onClick }: {
    track: { title: string; artist: string; cover_image?: string | null };
    isActive: boolean;
    onClick?: () => void;
}) {
    const coverUrl = useCoverArt(track.cover_image);

    return (
        <button
            onClick={onClick}
            className={`
                group flex items-center gap-3 p-2 rounded-xl text-left transition-all duration-200
                ${isActive
                    ? 'bg-secondary-container text-on-secondary-container'
                    : 'hover:bg-surface-container-high text-on-surface'
                }
            `}
        >
            {/* Tiny Art */}
            <div className={`
                w-10 h-10 rounded-lg overflow-hidden shrink-0 bg-surface-container-highest relative
                ${isActive ? 'shadow-sm' : ''}
            `}>
                {coverUrl ? (
                    <img src={coverUrl} alt="" className="w-full h-full object-cover" />
                ) : (
                    <div className="w-full h-full flex items-center justify-center">
                        <IconMusicNote size={16} className="opacity-50" />
                    </div>
                )}

                {/* Hover Play Overlay */}
                <div className="absolute inset-0 bg-black/40 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                    <IconPlay size={16} className="text-white fill-white" />
                </div>
            </div>

            <div className="flex flex-col min-w-0 flex-1">
                <span className={`text-label-large font-medium truncate ${isActive ? '' : 'text-on-surface'}`}>
                    {track.title}
                </span>
                <span className={`text-label-small truncate ${isActive ? 'text-on-secondary-container/80' : 'text-on-surface-variant'}`}>
                    {track.artist}
                </span>
            </div>

            {/* Playing Indicator */}
            {isActive && (
                <div className="w-2 h-2 rounded-full bg-primary shrink-0 mr-2" />
            )}
        </button>
    );
}
