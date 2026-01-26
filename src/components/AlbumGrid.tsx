import { useState, useMemo, forwardRef } from 'react';
import { VirtuosoGrid, Virtuoso } from 'react-virtuoso';
import { usePlayerStore } from '../store/playerStore';
import { useNavigationStore } from '../store/navigationStore';
import { useCoverArt } from '../hooks/useCoverArt';
import type { TrackDisplay } from '../types';
import { IconMusicNote, IconPlay, IconAlbum } from './Icons';
import { motion } from 'framer-motion';


interface Album {
    name: string;
    artist: string;
    cover: string | null;
    tracks: TrackDisplay[];
}

// M3 Very Sunny Shape for Play Button
const VerySunnyPlayButton = ({ onClick }: { onClick: (e: React.MouseEvent) => void }) => {
    const [isHovered, setIsHovered] = useState(false);

    return (
        <div
            onClick={onClick}
            onMouseEnter={() => setIsHovered(true)}
            onMouseLeave={() => setIsHovered(false)}
            className="absolute -bottom-2 -right-2 w-11 h-11 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-200 z-30 cursor-pointer"
        >
            <motion.svg
                viewBox="0 0 320 320"
                className="absolute w-full h-full drop-shadow-lg"
                style={{ color: 'var(--md-sys-color-primary)' }}
                animate={{ rotate: isHovered ? 120 : 0, scale: isHovered ? 1.1 : 1 }}
                transition={{ type: "spring", stiffness: 260, damping: 20 }}
            >
                <path d="M136.72 13.1925C147.26 -4.3975 172.74 -4.3975 183.28 13.1925L195.12 32.9625C201.27 43.2125 213.4 48.2425 224.99 45.3325L247.35 39.7325C267.24 34.7525 285.25 52.7626 280.27 72.6526L274.67 95.0126C271.76 106.603 276.79 118.733 287.04 124.883L306.81 136.723C324.4 147.263 324.4 172.743 306.81 183.283L287.04 195.123C276.79 201.273 271.76 213.403 274.67 224.993L280.27 247.353C285.25 267.243 267.24 285.253 247.35 280.273L224.99 274.673C213.4 271.763 201.27 276.793 195.12 287.043L183.28 306.813C172.74 324.403 147.26 324.403 136.72 306.813L124.88 287.043C118.73 276.793 106.6 271.763 95.0102 274.673L72.6462 280.273C52.7632 285.253 34.7472 267.243 39.7292 247.353L45.3332 224.993C48.2382 213.403 43.2143 201.273 32.9603 195.123L13.1873 183.283C-4.39575 172.743 -4.39575 147.263 13.1873 136.723L32.9603 124.883C43.2143 118.733 48.2382 106.603 45.3332 95.0126L39.7292 72.6526C34.7472 52.7626 52.7633 34.7525 72.6453 39.7325L95.0102 45.3325C106.6 48.2425 118.73 43.2125 124.88 32.9625L136.72 13.1925Z" fill="currentColor" />
            </motion.svg>
            <IconPlay size={18} fill="var(--md-sys-color-on-primary)" className="relative z-10 pointer-events-none" />
        </div>
    );
};

// M3 Rounded Square
const M3RoundedSquareImage = ({ src, fallback }: { src: string | null, fallback: React.ReactNode }) => {
    const uniqueId = useMemo(() => `rounded-square-${Math.random().toString(36).substr(2, 9)}`, []);

    return (
        <svg viewBox="0 0 320 320" className="w-full h-full">
            <defs>
                <clipPath id={uniqueId}>
                    <path d="M320 172C320 216.72 320 239.08 312.98 256.81C302.81 282.49 282.49 302.81 256.81 312.98C239.08 320 216.72 320 172 320H148C103.28 320 80.9199 320 63.1899 312.98C37.5099 302.81 17.19 282.49 7.02002 256.81C1.95503e-05 239.08 0 216.72 0 172V148C0 103.28 1.95503e-05 80.92 7.02002 63.19C17.19 37.515 37.5099 17.187 63.1899 7.02197C80.9199 -2.71797e-05 103.28 0 148 0H172C216.72 0 239.08 -2.71797e-05 256.81 7.02197C282.49 17.187 302.81 37.515 312.98 63.19C320 80.92 320 103.28 320 148V172Z" />
                </clipPath>
            </defs>
            {src ? (
                <image href={src} x="0" y="0" width="320" height="320" preserveAspectRatio="xMidYMid slice" clipPath={`url(#${uniqueId})`} />
            ) : (
                <g clipPath={`url(#${uniqueId})`}>
                    <rect x="0" y="0" width="320" height="320" fill="var(--md-sys-color-surface-container-highest)" />
                    <g transform="translate(128, 128)">
                        {fallback}
                    </g>
                </g>
            )}
        </svg>
    );
};

// Virtuoso Custom Components
const GridList = forwardRef<HTMLDivElement, React.ComponentPropsWithoutRef<'div'>>(({ style, children, ...props }, ref) => (
    <div
        ref={ref}
        {...props}
        className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-6 p-8 content-start"
        style={{ ...style, width: '100%' }}
    >
        {children}
    </div>
));

const GridItem = ({ children, ...props }: React.ComponentPropsWithoutRef<'div'>) => (
    <div {...props} style={{ padding: 0, margin: 0 }}>{children}</div>
);

export function AlbumGrid() {
    const library = usePlayerStore(state => state.library);
    const playQueue = usePlayerStore(state => state.playQueue);
    const { selectedAlbumKey, navigateToAlbum, clearSelectedAlbum } = useNavigationStore();

    const albums = useMemo(() => {
        const albumMap = new Map<string, Album>();

        // Regex to normalize album names by stripping "Disc X", "CD X" suffixes
        const normalizeAlbumName = (name: string) => {
            return name.replace(/\s*[\(\[]?(?:Disc|CD)\s*\d+[\)\]]?\s*$/i, "").trim();
        };

        library.forEach(track => {
            const normalizedName = normalizeAlbumName(track.album);
            const key = `${normalizedName}-${track.artist}`;
            if (!albumMap.has(key)) {
                albumMap.set(key, {
                    name: normalizedName,
                    artist: track.artist,
                    cover: track.cover_image || null,
                    tracks: []
                });
            }
            albumMap.get(key)?.tracks.push(track);
        });

        // Use array sort (stable or not) - sorting by name
        return Array.from(albumMap.values()).sort((a, b) => a.name.localeCompare(b.name));
    }, [library]);

    const handlePlayAlbum = (album: Album) => {
        if (album.tracks.length > 0) {
            playQueue(album.tracks, 0);
        }
    };

    if (selectedAlbumKey) {
        const album = albums.find(a => `${a.name}-${a.artist}` === selectedAlbumKey);

        if (!album) {
            clearSelectedAlbum();
            return null;
        }

        return (
            <div className="h-full">
                <AlbumDetailView
                    album={album}
                    onBack={() => clearSelectedAlbum()}
                />
            </div>
        );
    }

    return (
        <VirtuosoGrid
            style={{ height: '100%' }}
            data={albums}
            overscan={200}
            components={{
                List: GridList,
                Item: GridItem
            }}
            itemContent={(_, album) => (
                <AlbumCard
                    key={`${album.name}-${album.artist}`}
                    album={album}
                    onClick={() => navigateToAlbum(album.name, album.artist)}
                    onPlay={() => handlePlayAlbum(album)}
                />
            )}
        />
    );
}

function AlbumCard({ album, onClick, onPlay }: { album: Album, onClick: () => void, onPlay: () => void }) {
    const coverUrl = useCoverArt(album.cover);

    return (
        <div
            onClick={onClick}
            className="group flex flex-col gap-4 p-4 rounded-[2rem] hover:bg-surface-container-high transition-colors cursor-pointer"
        >
            <div className="aspect-square w-full relative">
                <div className="w-full h-full">
                    <M3RoundedSquareImage
                        src={coverUrl}
                        fallback={<IconMusicNote size={64} />}
                    />
                </div>
                <VerySunnyPlayButton onClick={(e) => { e.stopPropagation(); onPlay(); }} />
            </div>

            <div className="px-1 flex flex-col gap-0.5">
                <div className="text-title-large font-semibold text-on-surface truncate" title={album.name}>{album.name}</div>
                <div className="text-body-large text-on-surface-variant truncate" title={album.artist}>{album.artist}</div>
            </div>
        </div>
    );
}

// Helper Type for Display Items
type DisplayItem =
    | { type: 'header', disc: number }
    | { type: 'track', track: TrackDisplay, index: number, originalIndex: number };

function AlbumDetailView({ album, onBack }: { album: Album, onBack: () => void }) {
    const { playQueue } = usePlayerStore();
    const coverUrl = useCoverArt(album.cover);

    // Prepare sorted tracks and inject headers
    const displayItems = useMemo<DisplayItem[]>(() => {
        // Sort tracks by disc, then track
        const sorted = [...album.tracks].sort((a, b) => {
            const discA = a.disc_number || 1;
            const discB = b.disc_number || 1;
            if (discA !== discB) return discA - discB;

            const trackA = a.track_number || 0;
            const trackB = b.track_number || 0;
            return trackA - trackB;
        });

        // Identify multi-disc
        const discs = new Set(sorted.map(t => t.disc_number || 1));
        const isMultiDisc = discs.size > 1;

        if (!isMultiDisc) {
            return sorted.map((track, i) => ({
                type: 'track',
                track,
                index: i + 1,
                originalIndex: album.tracks.indexOf(track) // We need index in original album.tracks for playQueue? 
                // Wait, playQueue takes array of tracks. 
                // Ideally we should play the sorted list.
            }));
        }

        const items: DisplayItem[] = [];
        let currentDisc = -1;

        sorted.forEach((track, i) => {
            const disc = track.disc_number || 1;
            if (disc !== currentDisc) {
                items.push({ type: 'header', disc });
                currentDisc = disc;
            }
            items.push({
                type: 'track',
                track,
                index: track.track_number || (i + 1), // Use track number if available, else somewhat fallback
                originalIndex: -1 // see below
            });
        });

        return items;
    }, [album.tracks]);

    // When playing sorted tracks, we should pass the sorted list to the queue
    const sortedTracks = useMemo(() => {
        return [...album.tracks].sort((a, b) => {
            const discA = a.disc_number || 1;
            const discB = b.disc_number || 1;
            if (discA !== discB) return discA - discB;
            return (a.track_number || 0) - (b.track_number || 0);
        });
    }, [album.tracks]);


    return (
        <div className="flex flex-col h-full bg-surface">
            {/* Header */}
            <div className="p-8 pb-10 flex gap-10 items-end bg-surface-container-low shrink-0 rounded-b-[3rem] shadow-elevation-1 z-10">
                <div className="w-64 h-64 shrink-0 shadow-elevation-3">
                    <M3RoundedSquareImage
                        src={coverUrl}
                        fallback={<IconMusicNote size={80} />}
                    />
                </div>

                <div className="flex flex-col gap-4 mb-2 min-w-0 flex-1">
                    <div>
                        <div className="text-label-large font-medium text-on-surface-variant uppercase tracking-wider mb-2">Album</div>
                        <h1 className="text-display-medium font-bold text-on-surface tracking-tight truncate leading-tight">{album.name}</h1>
                    </div>

                    <div className="flex items-center gap-3 text-headline-small">
                        <span className="font-semibold text-primary">{album.artist}</span>
                        <span className="text-on-surface-variant">• {album.tracks.length} tracks</span>
                    </div>

                    <div className="flex gap-4 mt-4">
                        <button
                            onClick={() => playQueue(sortedTracks, 0)}
                            className="h-12 px-8 bg-primary text-on-primary rounded-full font-medium hover:bg-primary/90 flex items-center gap-2 shadow-elevation-2 transition-transform active:scale-95 text-title-medium"
                        >
                            <IconPlay size={24} fill="currentColor" /> Play
                        </button>
                        <button
                            onClick={onBack}
                            className="h-12 px-8 border border-outline rounded-full text-on-surface font-medium hover:bg-surface-container-high transition-colors text-title-medium"
                        >
                            Back
                        </button>
                    </div>
                </div>
            </div>

            {/* Tracklist */}
            <div className="flex-1 p-6">
                <Virtuoso
                    style={{ height: '100%' }}
                    data={displayItems}
                    overscan={100}
                    itemContent={(i, item) => {
                        if (item.type === 'header') {
                            return (
                                <div className="flex items-center gap-4 py-4 mt-2">
                                    <IconAlbum size={20} className="text-primary" />
                                    <span className="text-title-medium font-bold text-primary">Disc {item.disc}</span>
                                    <div className="h-px flex-1 bg-surface-container-highest"></div>
                                </div>
                            );
                        }

                        // Track Item
                        return (
                            <div
                                key={item.track.id}
                                onClick={() => {
                                    // Find correct index in sortedTracks
                                    const index = sortedTracks.indexOf(item.track);
                                    playQueue(sortedTracks, index);
                                }}
                                className="group flex items-center gap-6 p-4 rounded-xl hover:bg-surface-container-highest cursor-pointer text-on-surface-variant hover:text-on-surface transition-colors"
                            >
                                <span className="w-8 text-center text-title-medium font-medium opacity-60 group-hover:opacity-100">{item.track.track_number || (i + 1)}</span>
                                <span className="flex-1 font-medium text-body-large truncate">{item.track.title}</span>
                                <span className="text-label-large font-medium opacity-60 tabular-nums">
                                    {Math.floor(item.track.duration_secs / 60)}:
                                    {Math.floor(item.track.duration_secs % 60).toString().padStart(2, '0')}
                                </span>
                            </div>
                        );
                    }}
                    components={{
                        Footer: () => <div className="h-32"></div>
                    }}
                />
            </div>
        </div>
    );
}
