import { useMemo, useState } from 'react';
import { Virtuoso } from 'react-virtuoso';
import { usePlayerStore } from '../store/playerStore';
import { useCoverArt } from '../hooks/useCoverArt';
import { IconMusicNote, IconPlay, IconHeart, IconPlus, IconPause } from './Icons';
import { WavySeparator } from './WavySeparator';
import { ContextMenu } from './ContextMenu';
import type { TrackDisplay } from '../types';


// Format seconds to MM:SS
function formatDuration(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
}

function Equalizer() {
    return (
        <div className="flex items-end gap-[2px] h-4 w-4">
            <div className="w-[3px] bg-primary rounded-t-sm animate-equalize" style={{ animationDelay: '0s' }}></div>
            <div className="w-[3px] bg-primary rounded-t-sm animate-equalize" style={{ animationDelay: '0.2s' }}></div>
            <div className="w-[3px] bg-primary rounded-t-sm animate-equalize" style={{ animationDelay: '0.4s' }}></div>
        </div>
    );
}

function TrackRow({ track, index, isActive, isPlaying, onClick, onContextMenu }: {
    track: TrackDisplay,
    index: number,
    isActive: boolean,
    isPlaying: boolean,
    onClick: () => void,
    onContextMenu: (e: React.MouseEvent) => void
}) {
    const coverUrl = useCoverArt(track.cover_image);
    const isFavorite = usePlayerStore(state => state.isFavorite(track.path));
    const toggleFavorite = usePlayerStore(state => state.toggleFavorite);
    const addToQueue = usePlayerStore(state => state.addToQueue);
    const pause = usePlayerStore(state => state.pause);
    const resume = usePlayerStore(state => state.resume);

    const handlePlayPauseClick = (e: React.MouseEvent) => {
        e.stopPropagation();
        if (isActive && isPlaying) {
            pause();
        } else if (isActive && !isPlaying) {
            resume();
        } else {
            onClick(); // Play this track
        }
    };

    return (
        <div
            onClick={onClick}
            onContextMenu={onContextMenu}
            className={`
                group grid grid-cols-[3rem_minmax(0,1.5fr)_minmax(0,1fr)_minmax(0,1fr)_4rem_6rem] gap-4 items-center 
                px-4 py-3 mx-2 rounded-xl cursor-pointer transition-all duration-200
                ${isActive
                    ? 'bg-secondary-container/10 text-primary'
                    : 'text-on-surface-variant hover:text-on-surface hover:bg-surface-container-highest/50'
                }
            `}
        >
            <span className="flex justify-center font-medium relative w-8">
                {isActive && isPlaying ? (
                    <div className="group-hover:hidden"><Equalizer /></div>
                ) : (
                    <span className="group-hover:hidden text-label-medium">{isActive ? <IconPlay size={16} fill="currentColor" /> : index + 1}</span>
                )}

                <span 
                    className="hidden group-hover:flex absolute inset-0 items-center justify-center text-primary animate-in fade-in zoom-in duration-200 cursor-pointer"
                    onClick={handlePlayPauseClick}
                >
                    {isActive && isPlaying ? <IconPause size={20} fill="currentColor" /> : <IconPlay size={20} fill="currentColor" />}
                </span>
            </span>

            <span className="flex items-center gap-4 min-w-0">
                <div className="w-12 h-12 shrink-0 rounded-lg overflow-hidden bg-surface-container shadow-sm group-hover:scale-110 transition-transform duration-300">
                    {coverUrl ? (
                        <img src={coverUrl} alt="" loading="lazy" className="w-full h-full object-cover" />
                    ) : (
                        <div className="w-full h-full flex items-center justify-center bg-surface-container-high/50">
                            <IconMusicNote size={16} />
                        </div>
                    )}
                </div>
                <div className="flex flex-col min-w-0">
                    <span className={`truncate font-medium text-body-large ${isActive ? 'text-primary' : 'text-on-surface'}`}>
                        {track.title}
                    </span>
                    <span className="truncate text-body-small opacity-70 lg:hidden">{track.artist}</span>
                </div>
            </span>

            <span className="truncate text-body-medium opacity-80 hidden lg:block">{track.artist}</span>
            <span className="truncate text-body-medium opacity-80 hidden xl:block">{track.album}</span>

            <span className="text-right text-label-medium tabular-nums opacity-60 group-hover:hidden">
                {formatDuration(track.duration_secs)}
            </span>

            {/* Hover Actions replacing Duration/Extra Space */}
            <div className="hidden group-hover:flex items-center justify-end gap-1 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
                <button
                    onClick={(e) => { e.stopPropagation(); toggleFavorite(track.path); }}
                    className={`p-2 rounded-full hover:bg-on-surface/10 ${isFavorite ? 'text-red-400' : 'text-on-surface-variant'}`}
                    title={isFavorite ? "Remove from Favorites" : "Add to Favorites"}
                >
                    <IconHeart size={18} filled={isFavorite} />
                </button>
                <button
                    onClick={(e) => { e.stopPropagation(); addToQueue(track); }}
                    className="p-2 rounded-full hover:bg-on-surface/10 text-on-surface-variant"
                    title="Add to Queue"
                >
                    <IconPlus size={18} />
                </button>
            </div>
        </div>
    );
}

// Helper component for Sortable Headers
function SortHeader({ label, sortKey, align = 'left' }: { label: string, sortKey: keyof TrackDisplay, align?: 'left' | 'right' }) {
    const sort = usePlayerStore(state => state.sort);
    const setSort = usePlayerStore(state => state.setSort);
    const isActive = sort?.key === sortKey;

    return (
        <div
            onClick={() => setSort(sortKey)}
            className={`
                flex items-center gap-1 cursor-pointer hover:text-on-surface transition-colors select-none py-3 text-label-large font-medium
                ${align === 'right' ? 'justify-end' : 'justify-start'}
                ${isActive ? 'text-primary' : 'text-on-surface-variant'}
            `}
        >
            {label}
            {isActive && (
                <span className="text-[0.7rem]">
                    {sort?.direction === 'asc' ? '▲' : '▼'}
                </span>
            )}
        </div>
    );
}

export function TrackList() {
    const library = usePlayerStore(state => state.library);
    const playQueue = usePlayerStore(state => state.playQueue);
    const isLoading = usePlayerStore(state => state.isLoading);
    const currentPath = usePlayerStore(state => state.status.track?.path);
    const isPlaying = usePlayerStore(state => state.status.state === 'Playing');
    const searchQuery = usePlayerStore(state => state.searchQuery);

    // Context Menu State
    const [contextMenu, setContextMenu] = useState<{ x: number; y: number; track: TrackDisplay } | null>(null);

    const handleContextMenu = (e: React.MouseEvent, track: TrackDisplay) => {
        e.preventDefault();
        setContextMenu({ x: e.clientX, y: e.clientY, track });
    };

    // Filter library based on search query
    const filteredLibrary = useMemo(() => {
        if (!searchQuery.trim()) return library;
        const query = searchQuery.toLowerCase();
        return library.filter(track =>
            track.title.toLowerCase().includes(query) ||
            track.artist.toLowerCase().includes(query) ||
            track.album.toLowerCase().includes(query)
        );
    }, [library, searchQuery]);

    if (isLoading) {
        return (
            <div className="flex flex-col items-center justify-center h-full text-on-surface-variant">
                <div className="animate-pulse">Scanning music folder...</div>
            </div>
        );
    }

    if (library.length === 0) {
        return (
            <div className="flex flex-col items-center justify-center h-full gap-4 text-on-surface-variant/60">
                <IconMusicNote size={64} />
                <div className="text-center">
                    <h3 className="text-headline-small text-on-surface font-semibold">No tracks loaded</h3>
                    <p className="text-body-medium">Open a folder to load your music library</p>
                </div>
            </div>
        );
    }

    return (
        <div className="flex flex-col h-full bg-surface">
            {/* Header - Non-sticky */}
            <div className="grid grid-cols-[3rem_minmax(0,1.5fr)_minmax(0,1fr)_minmax(0,1fr)_4rem] gap-4 px-6 bg-surface">
                <span className="py-3 text-center text-label-large font-medium text-on-surface-variant">#</span>
                <SortHeader label="Title" sortKey="title" />
                <SortHeader label="Artist" sortKey="artist" />
                <SortHeader label="Album" sortKey="album" />
                <SortHeader label="Duration" sortKey="duration_secs" align="right" />
            </div>
            {/* Wavy separator below header */}
            <div className="px-6">
                <WavySeparator label="" color="var(--md-sys-color-outline-variant)" />
            </div>
            <div className="flex-1">
                <Virtuoso
                    style={{ height: '100%' }}
                    data={filteredLibrary}
                    overscan={200}
                    itemContent={(index, track) => {
                        // Check for album change
                        const prevTrack = index > 0 ? filteredLibrary[index - 1] : null;
                        const showSeparator = prevTrack && prevTrack.album !== track.album;

                        return (
                            <div key={track.id}>
                                {showSeparator && (
                                    <div className="px-6">
                                        <WavySeparator label={track.album} color="var(--md-sys-color-primary)" />
                                    </div>
                                )}
                                <TrackRow
                                    track={track}
                                    index={index}
                                    isActive={currentPath === track.path}
                                    isPlaying={isPlaying}
                                    onClick={() => playQueue(filteredLibrary, index)}
                                    onContextMenu={(e) => handleContextMenu(e, track)}
                                />
                            </div>
                        );
                    }}
                    components={{
                        Footer: () => <div className="h-24"></div>
                    }}
                />
            </div>
            {contextMenu && (
                <ContextMenu
                    x={contextMenu.x}
                    y={contextMenu.y}
                    track={contextMenu.track}
                    onClose={() => setContextMenu(null)}
                />
            )}
        </div>
    );
}

