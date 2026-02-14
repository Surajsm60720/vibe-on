import { useRef, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AnimatePresence, motion } from 'framer-motion';
import { useVisualizerStore } from '../store/visualizerStore';
import { usePlayerStore } from '../store/playerStore';
import { useThemeStore } from '../store/themeStore';

interface VisualizerData {
    frequency_bins: number[];
    waveform: number[];
}

// Helper to parse hex color
const hexToRgb = (hex: string) => {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return result ? {
        r: parseInt(result[1], 16),
        g: parseInt(result[2], 16),
        b: parseInt(result[3], 16)
    } : { r: 100, g: 100, b: 255 }; // Default fallback
};

// Helper to interpolate colors
const interpolateColor = (c1: { r: number, g: number, b: number }, c2: { r: number, g: number, b: number }, factor: number) => {
    return {
        r: Math.round(c1.r + (c2.r - c1.r) * factor),
        g: Math.round(c1.g + (c2.g - c1.g) * factor),
        b: Math.round(c1.b + (c2.b - c1.b) * factor)
    };
};

export function FullscreenVisualizer() {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const animationRef = useRef<number | null>(null);
    const coverImageRef = useRef<HTMLImageElement | null>(null);

    // Performance optimization: Use ref for data to avoid React re-renders
    const audioDataRef = useRef<{ bins: number[], waveform: number[] }>({
        bins: new Array(64).fill(0),
        waveform: new Array(128).fill(0)
    });

    // Visualizer state
    const displayMode = useVisualizerStore(s => s.displayMode);
    const setDisplayMode = useVisualizerStore(s => s.setDisplayMode);
    const sensitivity = useVisualizerStore(s => s.sensitivity);

    // Theme colors
    const colors = useThemeStore(s => s.colors);

    // Player state
    const track = usePlayerStore(s => s.status.track);
    const isPlaying = usePlayerStore(s => s.status.state === 'Playing');
    const pause = usePlayerStore(s => s.pause);
    const resume = usePlayerStore(s => s.resume);
    const nextTrack = usePlayerStore(s => s.nextTrack);
    const prevTrack = usePlayerStore(s => s.prevTrack);
    const activeSource = usePlayerStore(s => s.activeSource);

    const isVisible = displayMode === 'fullscreen';

    // Load cover image logic (Keep it for potential center display or fallback)
    useEffect(() => {
        if (!track || !isVisible) {
            coverImageRef.current = null;
            return;
        }

        let isMounted = true;
        const img = new Image();
        img.crossOrigin = "Anonymous";

        let src = '';
        if (activeSource === 'local') {
            src = `/cover/${encodeURIComponent(track.path)}`;
        } else if (track.cover_image) {
            src = track.cover_image;
        }

        if (src) {
            img.src = src;
            img.onload = () => {
                if (isMounted) coverImageRef.current = img;
            };
        } else {
            coverImageRef.current = null;
        }

        return () => { isMounted = false; };
    }, [track, activeSource, isVisible]);

    // Handle resize
    useEffect(() => {
        if (!isVisible) return;

        const handleResize = () => {
            const canvas = canvasRef.current;
            if (canvas) {
                canvas.width = window.innerWidth;
                canvas.height = window.innerHeight;
            }
        };

        handleResize();
        window.addEventListener('resize', handleResize);
        return () => window.removeEventListener('resize', handleResize);
    }, [isVisible]);

    // Keyboard shortcuts
    useEffect(() => {
        if (!isVisible) return;

        const handleKeyDown = (e: KeyboardEvent) => {
            switch (e.key) {
                case 'Escape':
                case 'v':
                case 'V':
                    setDisplayMode('off');
                    break;
                case ' ':
                    e.preventDefault();
                    isPlaying ? pause() : resume();
                    break;
                case 'ArrowRight':
                    nextTrack();
                    break;
                case 'ArrowLeft':
                    prevTrack();
                    break;
            }
        };

        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [isVisible, isPlaying, pause, resume, nextTrack, prevTrack, setDisplayMode]);

    // Draw functions
    const drawRadialBars = useCallback((
        ctx: CanvasRenderingContext2D,
        width: number, height: number,
        centerX: number, centerY: number,
        bins: number[],
        bassEnergy: number
    ) => {
        const numBars = bins.length;
        // Make radius responsive
        const baseRadius = Math.min(width, height) * 0.15;
        const maxBarHeight = Math.min(width, height) * 0.35; // Taller bars for simpler view

        // Parse theme colors
        const cPrimary = hexToRgb(colors.primary || '#bfc2ff');
        const cSecondary = hexToRgb(colors.secondary || '#c6bfff');
        const cTertiary = hexToRgb(colors.tertiary || '#ffb0cd');

        // Draw Center Art if available
        if (coverImageRef.current) {
            const coreSize = baseRadius * 0.8 + bassEnergy * 10;

            ctx.save();
            ctx.beginPath();
            ctx.arc(centerX, centerY, coreSize, 0, Math.PI * 2);
            ctx.clip();
            ctx.drawImage(coverImageRef.current, centerX - coreSize, centerY - coreSize, coreSize * 2, coreSize * 2);
            ctx.restore();

            // Ring
            ctx.beginPath();
            ctx.arc(centerX, centerY, coreSize, 0, Math.PI * 2);
            ctx.lineWidth = 2;
            ctx.strokeStyle = colors.outline || 'rgba(255,255,255,0.5)';
            ctx.stroke();
        } else {
            // Default Glow if no art
            const gradient = ctx.createRadialGradient(
                centerX, centerY, 0,
                centerX, centerY, baseRadius + bassEnergy * 40
            );
            gradient.addColorStop(0, `rgba(${cPrimary.r}, ${cPrimary.g}, ${cPrimary.b}, 0.5)`);
            gradient.addColorStop(0.6, `rgba(${cPrimary.r}, ${cPrimary.g}, ${cPrimary.b}, 0.1)`);
            gradient.addColorStop(1, 'transparent');

            ctx.beginPath();
            ctx.arc(centerX, centerY, baseRadius + bassEnergy * 40, 0, Math.PI * 2);
            ctx.fillStyle = gradient;
            ctx.fill();
        }

        // Bars
        for (let i = 0; i < numBars; i++) {
            const angle = (i / numBars) * Math.PI * 2 - Math.PI / 2;
            const rawValue = Math.min(bins[i] * sensitivity, 1);
            // Cubic ease for snappier bars
            const value = Math.pow(rawValue, 0.8);
            const barHeight = value * maxBarHeight;

            // Bass kick effect on radius
            const kick = Math.pow(bassEnergy, 2) * 20;
            const r1 = baseRadius + 10 + kick;
            const r2 = r1 + barHeight;

            const x1 = centerX + Math.cos(angle) * r1;
            const y1 = centerY + Math.sin(angle) * r1;
            const x2 = centerX + Math.cos(angle) * r2;
            const y2 = centerY + Math.sin(angle) * r2;

            // Gradient Logic
            const progress = i / numBars;
            let r, g, b;

            if (progress < 0.33) {
                const p = progress / 0.33;
                const c = interpolateColor(cPrimary, cSecondary, p);
                r = c.r; g = c.g; b = c.b;
            } else if (progress < 0.66) {
                const p = (progress - 0.33) / 0.33;
                const c = interpolateColor(cSecondary, cTertiary, p);
                r = c.r; g = c.g; b = c.b;
            } else {
                const p = (progress - 0.66) / 0.34;
                const c = interpolateColor(cTertiary, cPrimary, p);
                r = c.r; g = c.g; b = c.b;
            }

            ctx.beginPath();
            ctx.moveTo(x1, y1);
            ctx.lineTo(x2, y2);
            ctx.strokeStyle = `rgba(${r}, ${g}, ${b}, ${0.8})`; // Higher opacity
            ctx.lineWidth = 4; // Thicker bars
            ctx.lineCap = 'round';
            ctx.stroke();
        }

    }, [sensitivity, colors]);


    // Main draw function
    const draw = useCallback(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const ctx = canvas.getContext('2d', { alpha: false });
        if (!ctx) return;

        const { width, height } = canvas;
        const centerX = width / 2;
        const centerY = height / 2;

        // Clean Background
        ctx.fillStyle = colors.surface;
        ctx.fillRect(0, 0, width, height);

        // Vignette
        const bgGradient = ctx.createRadialGradient(centerX, centerY, 0, centerX, centerY, Math.max(width, height));
        bgGradient.addColorStop(0, 'rgba(0,0,0,0)');
        bgGradient.addColorStop(1, 'rgba(0,0,0,0.4)');
        ctx.fillStyle = bgGradient;
        ctx.fillRect(0, 0, width, height);

        const { bins } = audioDataRef.current;

        // Calculate Energy for kicks
        const bassEnergy = bins.slice(0, 8).reduce((a: number, b: number) => a + b, 0) / 8;

        // Force Bars Mode
        drawRadialBars(ctx, width, height, centerX, centerY, bins, bassEnergy);

    }, [sensitivity, colors, drawRadialBars]);

    // Animation loop
    useEffect(() => {
        if (!isVisible) return;

        let mounted = true;

        const fetchData = async () => {
            if (!mounted) return;
            try {
                const data = await invoke<VisualizerData>('get_visualizer_data');
                // Update the REF instead of state
                if (mounted && data) {
                    audioDataRef.current = {
                        bins: data.frequency_bins,
                        waveform: data.waveform
                    };
                }
            } catch (e) {
                // Silently fail
            }
        };

        const animate = () => {
            if (!mounted) return;
            fetchData();
            draw();
            animationRef.current = requestAnimationFrame(animate);
        };

        animate();

        return () => {
            mounted = false;
            if (animationRef.current) {
                cancelAnimationFrame(animationRef.current);
            }
        };
    }, [isVisible, draw]);

    if (!isVisible) return null;

    return (
        <AnimatePresence>
            <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.5 }}
                className="fixed inset-0 z-[200] flex flex-col"
                style={{ backgroundColor: colors.surface }}
            >
                {/* Canvas */}
                <canvas
                    ref={canvasRef}
                    className="absolute inset-0 w-full h-full"
                />

                {/* Visualizer Controls */}
                <div className="absolute top-8 left-8 flex flex-col gap-2 pointer-events-auto">
                    <h1 className="text-white/80 text-xl font-light tracking-wider" style={{ color: colors.onSurface }}>
                        Visualizer
                    </h1>
                    <p className="text-white/40 text-xs tracking-wider uppercase" style={{ color: colors.onSurfaceVariant }}>
                        ESC to close
                    </p>
                </div>

                {/* Top bar with close button */}
                <div className="absolute top-0 left-0 right-0 p-6 flex justify-end items-start z-10">
                    <button
                        onClick={() => setDisplayMode('off')}
                        className="p-3 rounded-full hover:bg-white/10 transition-colors"
                        style={{ color: colors.onSurface, backgroundColor: colors.surfaceContainer }}
                    >
                        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path d="M18 6L6 18M6 6l12 12" />
                        </svg>
                    </button>
                </div>

                {/* Bottom bar with track info and controls */}
                <div className="absolute bottom-0 left-0 right-0 p-8 flex flex-col items-center gap-6 z-10">
                    {/* Track info */}
                    <div className="text-center">
                        <h2
                            className="text-2xl font-semibold mb-1 shadow-black drop-shadow-md"
                            style={{ color: colors.onSurface }}
                        >
                            {track?.title || 'No Track Playing'}
                        </h2>
                        <p
                            className="text-sm opacity-80"
                            style={{ color: colors.onSurfaceVariant }}
                        >
                            {track?.artist || 'Unknown Artist'}
                        </p>
                    </div>

                    {/* Playback controls */}
                    <div className="flex items-center gap-6">
                        <button
                            onClick={prevTrack}
                            className="p-3 rounded-full hover:bg-white/10 transition-colors"
                            style={{ color: colors.onSurface }}
                        >
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                                <path d="M6 6h2v12H6zm3.5 6l8.5 6V6z" />
                            </svg>
                        </button>
                        <button
                            onClick={() => isPlaying ? pause() : resume()}
                            className="p-4 rounded-full transition-colors"
                            style={{ backgroundColor: colors.primary, color: colors.onPrimary }}
                        >
                            {isPlaying ? (
                                <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor">
                                    <path d="M6 4h4v16H6zm8 0h4v16h-4z" />
                                </svg>
                            ) : (
                                <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor">
                                    <path d="M8 5v14l11-7z" />
                                </svg>
                            )}
                        </button>
                        <button
                            onClick={nextTrack}
                            className="p-3 rounded-full hover:bg-white/10 transition-colors"
                            style={{ color: colors.onSurface }}
                        >
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                                <path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z" />
                            </svg>
                        </button>
                    </div>
                </div>
            </motion.div>
        </AnimatePresence>
    );
}

// Legacy export for compatibility
export const AudioVisualizer = FullscreenVisualizer;
