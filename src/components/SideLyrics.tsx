import { useEffect, useRef, useMemo, useState } from 'react';
import { motion } from 'motion/react';
import { useLyricsStore } from '../store/lyricsStore';
import { usePlayerStore } from '../store/playerStore';
import { useThemeStore } from '../store/themeStore';
import type { LyricsLine } from '../types';

interface SideLyricsProps {
    variant?: 'carousel' | 'scrollable';
}

export function SideLyrics({ variant = 'carousel' }: SideLyricsProps) {
    const { lines, plainLyrics, isLoading, error, isInstrumental, loadingStatus, lyricsMode } = useLyricsStore();
    const { status, seek } = usePlayerStore();
    const { colors } = useThemeStore();
    const { primary } = colors;

    const containerRef = useRef<HTMLDivElement>(null);
    const lineRefs = useRef<(HTMLDivElement | null)[]>([]);

    const [offsetY, setOffsetY] = useState(0);
    const position = status.position_secs;

    // Find current active line
    const activeLineIndex = useMemo(() => {
        if (!lines || lines.length === 0) return -1;
        for (let i = lines.length - 1; i >= 0; i--) {
            if (position >= lines[i].time) {
                return i;
            }
        }
        return -1;
    }, [lines, position]);

    // Carousel Logic
    useEffect(() => {
        if (variant === 'carousel' && activeLineIndex >= 0 && containerRef.current && lineRefs.current[activeLineIndex]) {
            const containerHeight = containerRef.current.clientHeight;
            const activeLine = lineRefs.current[activeLineIndex];

            if (activeLine) {
                const lineMid = activeLine.offsetTop + (activeLine.clientHeight / 2);
                const newOffset = (containerHeight / 2) - lineMid;
                setOffsetY(newOffset);
            }
        }
    }, [activeLineIndex, lines, variant, lyricsMode]);

    // Scrollable Logic (Auto-scroll to active)
    useEffect(() => {
        if (variant === 'scrollable' && activeLineIndex >= 0 && lineRefs.current[activeLineIndex]) {
            lineRefs.current[activeLineIndex]?.scrollIntoView({
                behavior: 'smooth',
                block: 'center',
                inline: 'nearest'
            });
        }
    }, [activeLineIndex, variant]);

    const renderLineContent = (line: LyricsLine, isActive: boolean) => {
        if (lyricsMode === 'romaji') {
            return line.romaji || line.text;
        }
        if (lyricsMode === 'both' && line.romaji) {
            return (
                <div className="flex flex-col items-center gap-0.5">
                    <span className={`text-sm opacity-70 ${isActive ? 'text-on-surface' : 'text-on-surface-variant'}`}>
                        {line.romaji}
                    </span>
                    <span>{line.text}</span>
                </div>
            );
        }
        return line.text;
    };

    if (isLoading) {
        return (
            <div className="flex flex-col items-center justify-center h-full gap-3 py-10 opacity-50 px-4 text-center">
                <div className="w-6 h-6 border-2 border-on-surface/30 border-t-on-surface rounded-full animate-spin shrink-0" />
                <p className="text-label-medium animate-pulse">{loadingStatus || "Loading lyrics..."}</p>
            </div>
        );
    }

    if (error) {
        return (
            <div className="flex flex-col items-center justify-center h-full gap-2 py-10 text-center opacity-50">
                <p className="text-body-medium">No lyrics found</p>
            </div>
        );
    }

    if (isInstrumental) {
        return (
            <div className="flex flex-col items-center justify-center h-full gap-2 py-10 text-center opacity-70">
                <p className="text-title-medium font-medium">Instrumental</p>
            </div>
        );
    }

    // Scrollable Variant Render
    if (variant === 'scrollable') {
        return (
            <div className="h-full overflow-y-auto px-4 py-8 scroll-smooth" ref={containerRef}>
                {lines && lines.length > 0 ? (
                    <div className="flex flex-col gap-4">
                        {lines.map((line, index) => {
                            const isActive = index === activeLineIndex;
                            return (
                                <div
                                    key={`${line.time}-${index}`}
                                    ref={el => { lineRefs.current[index] = el; }}
                                    onClick={() => seek(line.time)}
                                    className={`
                                        cursor-pointer p-3 rounded-lg transition-all duration-200 text-lg
                                        ${isActive
                                            ? 'bg-surface-container-highest text-on-surface font-semibold shadow-sm'
                                            : 'text-on-surface-variant/70 hover:bg-surface-container-high hover:text-on-surface'
                                        }
                                    `}
                                >
                                    {renderLineContent(line, isActive)}
                                </div>
                            );
                        })}
                    </div>
                ) : plainLyrics ? (
                    <div className="whitespace-pre-wrap text-body-medium text-on-surface/80 leading-relaxed max-w-2xl mx-auto">
                        {plainLyrics}
                    </div>
                ) : null}
            </div>
        );
    }

    // Default Carousel Render
    return (
        <div
            ref={containerRef}
            className="flex-1 overflow-hidden min-h-0 relative select-none fade-mask-y"
        >
            {/* Synced Lyrics Carousel */}
            {lines && lines.length > 0 && (
                <motion.div
                    className="flex flex-col gap-6 w-full items-center py-10"
                    animate={{ y: offsetY }}
                    transition={{ type: 'spring', stiffness: 100, damping: 20, mass: 1 }}
                >
                    {lines.map((line, index) => {
                        const isActive = index === activeLineIndex;
                        return (
                            <motion.div
                                key={`${line.time}-${index}`}
                                ref={el => { lineRefs.current[index] = el; }}
                                onClick={() => seek(line.time)}
                                initial={false}
                                animate={{
                                    opacity: isActive ? 1 : 0.25,
                                    scale: isActive ? 1.05 : 0.95,
                                }}
                                transition={{ duration: 0.3 }}
                                className={`
                                    cursor-pointer text-center px-6 max-w-full transition-colors duration-300
                                    ${isActive ? 'font-bold z-10' : 'hover:opacity-60'}
                                `}
                            >
                                <div
                                    className="text-xl md:text-2xl leading-relaxed"
                                    style={{
                                        color: isActive ? primary : undefined,
                                        textShadow: isActive ? `0 0 25px ${primary}60` : 'none'
                                    }}
                                >
                                    {renderLineContent(line, isActive)}
                                </div>
                            </motion.div>
                        );
                    })}
                </motion.div>
            )}

            {/* Plain Lyrics Scroll (Fallback) */}
            {!lines && plainLyrics && (
                <div className="h-full overflow-y-auto no-scrollbar p-6 whitespace-pre-wrap text-body-medium text-on-surface/80 leading-relaxed text-center">
                    {plainLyrics}
                </div>
            )}
        </div>
    );
}
