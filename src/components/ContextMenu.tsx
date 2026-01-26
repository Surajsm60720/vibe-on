import { useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { TrackDisplay } from '../types';
import { usePlayerStore } from '../store/playerStore';
import { IconHeart, IconNext, IconPlus } from './Icons';

interface ContextMenuProps {
    x: number;
    y: number;
    track: TrackDisplay;
    onClose: () => void;
}

export function ContextMenu({ x, y, track, onClose }: ContextMenuProps) {
    const menuRef = useRef<HTMLDivElement>(null);
    const { playNext, addToQueue, toggleFavorite, isFavorite } = usePlayerStore();

    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
                onClose();
            }
        };

        const handleScroll = () => onClose();

        document.addEventListener('mousedown', handleClickOutside);
        window.addEventListener('scroll', handleScroll, true);
        return () => {
            document.removeEventListener('mousedown', handleClickOutside);
            window.removeEventListener('scroll', handleScroll, true);
        };
    }, [onClose]);

    // Adjust position to keep within viewport
    // Simplification: just ensuring it doesn't go off bottom/right would need window measuring.
    // CSS constrained for now.

    const handleAction = (action: () => void) => {
        action();
        onClose();
    };

    return (
        <AnimatePresence>
            <motion.div
                ref={menuRef}
                initial={{ opacity: 0, scale: 0.9, y: -10 }}
                animate={{ opacity: 1, scale: 1, y: 0 }}
                exit={{ opacity: 0, scale: 0.9, y: -10 }}
                transition={{ duration: 0.1 }}
                style={{ top: y, left: x }}
                className="fixed z-[9999] min-w-[200px] bg-surface-container-high border border-outline-variant/20 rounded-xl shadow-elevation-3 py-2 flex flex-col overflow-hidden"
            >
                <div className="px-4 py-2 border-b border-outline-variant/10 mb-1">
                    <div className="text-label-medium font-bold text-on-surface truncate max-w-[180px]">{track.title}</div>
                    <div className="text-label-small text-on-surface-variant truncate max-w-[180px]">{track.artist}</div>
                </div>

                <button
                    onClick={() => handleAction(() => playNext(track))}
                    className="flex items-center gap-3 px-4 py-2 text-label-large text-on-surface hover:bg-surface-container-highest transition-colors text-left"
                >
                    <IconNext size={18} className="text-on-surface-variant" />
                    Play Next
                </button>

                <button
                    onClick={() => handleAction(() => addToQueue(track))}
                    className="flex items-center gap-3 px-4 py-2 text-label-large text-on-surface hover:bg-surface-container-highest transition-colors text-left"
                >
                    <IconPlus size={18} className="text-on-surface-variant" />
                    Add to Queue
                </button>

                <div className="my-1 border-t border-outline-variant/10" />

                <button
                    onClick={() => handleAction(() => toggleFavorite(track.path))}
                    className="flex items-center gap-3 px-4 py-2 text-label-large text-on-surface hover:bg-surface-container-highest transition-colors text-left"
                >
                    <IconHeart size={18} filled={isFavorite(track.path)} className={isFavorite(track.path) ? "text-error" : "text-on-surface-variant"} />
                    {isFavorite(track.path) ? "Remove from Favorites" : "Favorite"}
                </button>
            </motion.div>
        </AnimatePresence>
    );
}
