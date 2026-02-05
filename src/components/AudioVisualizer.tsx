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

// Subtle Particle class - gentle floating orbs
class Particle {
    x: number = 0;
    y: number = 0;
    vx: number = (Math.random() - 0.5) * 0.3;  // Much slower
    vy: number = (Math.random() - 0.5) * 0.3;
    size: number = Math.random() * 2 + 0.5;   // Smaller
    alpha: number = Math.random() * 0.15 + 0.05;  // More transparent
    life: number = Math.random() * 200 + 100;  // Longer life

    update(width: number, height: number, energy: number) {
        this.x += this.vx * (1 + energy * 0.5);  // Less energy influence
        this.y += this.vy * (1 + energy * 0.5);
        this.life -= 0.3;

        if (this.x < 0 || this.x > width || this.y < 0 || this.y > height || this.life <= 0) {
            this.respawn(width, height);
        }
    }

    respawn(width: number, height: number) {
        // Spawn around the edges too, not just center
        const edge = Math.random();
        if (edge < 0.25) {
            this.x = Math.random() * width;
            this.y = 0;
        } else if (edge < 0.5) {
            this.x = Math.random() * width;
            this.y = height;
        } else if (edge < 0.75) {
            this.x = 0;
            this.y = Math.random() * height;
        } else {
            this.x = width;
            this.y = Math.random() * height;
        }
        this.vx = (Math.random() - 0.5) * 0.5;  // Slower
        this.vy = (Math.random() - 0.5) * 0.5;
        this.life = Math.random() * 200 + 100;
        this.alpha = Math.random() * 0.15 + 0.05;
    }

    draw(ctx: CanvasRenderingContext2D, color: string, _energy: number) {
        ctx.beginPath();
        ctx.arc(this.x, this.y, this.size, 0, Math.PI * 2);  // No size scaling with energy
        ctx.fillStyle = `rgba(${color}, ${this.alpha})`;
        ctx.fill();
    }
}

export function FullscreenVisualizer() {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const animationRef = useRef<number | null>(null);
    const particlesRef = useRef<Particle[]>([]);

    // Visualizer state
    const displayMode = useVisualizerStore(s => s.displayMode);
    const setDisplayMode = useVisualizerStore(s => s.setDisplayMode);
    const mode = useVisualizerStore(s => s.mode);
    const sensitivity = useVisualizerStore(s => s.sensitivity);
    const updateData = useVisualizerStore(s => s.updateData);
    const frequencyBins = useVisualizerStore(s => s.frequencyBins);
    const cycleMode = useVisualizerStore(s => s.cycleMode);

    // Theme colors
    const colors = useThemeStore(s => s.colors);

    // Player state
    const track = usePlayerStore(s => s.status.track);
    const isPlaying = usePlayerStore(s => s.status.state === 'Playing');
    const pause = usePlayerStore(s => s.pause);
    const resume = usePlayerStore(s => s.resume);
    const nextTrack = usePlayerStore(s => s.nextTrack);
    const prevTrack = usePlayerStore(s => s.prevTrack);

    const isVisible = displayMode === 'fullscreen';

    // Initialize particles - fewer for subtle effect
    useEffect(() => {
        if (isVisible) {
            particlesRef.current = Array.from({ length: 20 }, () => new Particle());
        }
    }, [isVisible]);

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
                case 'm':
                case 'M':
                    cycleMode();
                    break;
            }
        };

        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [isVisible, isPlaying, pause, resume, nextTrack, prevTrack, setDisplayMode, cycleMode]);

    // Draw functions
    const drawRadialBars = useCallback((
        ctx: CanvasRenderingContext2D,
        width: number, height: number,
        centerX: number, centerY: number,
        bins: number[],
        bassEnergy: number
    ) => {
        const numBars = bins.length;
        const baseRadius = Math.min(width, height) * 0.15;  // Smaller base
        const maxBarHeight = Math.min(width, height) * 0.25;  // Shorter max

        for (let i = 0; i < numBars; i++) {
            const angle = (i / numBars) * Math.PI * 2 - Math.PI / 2;
            // Clamp value and use a softer curve (square root for better range)
            const rawValue = Math.min(bins[i] * sensitivity, 1);
            const value = Math.sqrt(rawValue) * 0.7;  // Softer response
            const barHeight = value * maxBarHeight;

            // Less bass influence on base radius
            const bassOffset = Math.min(bassEnergy, 0.8) * 15;
            const x1 = centerX + Math.cos(angle) * (baseRadius + bassOffset);
            const y1 = centerY + Math.sin(angle) * (baseRadius + bassOffset);
            const x2 = centerX + Math.cos(angle) * (baseRadius + barHeight + bassOffset);
            const y2 = centerY + Math.sin(angle) * (baseRadius + barHeight + bassOffset);

            // Color gradient based on frequency
            const hue = 260 + (i / numBars) * 60;
            const saturation = 70 + Math.min(value * 20, 30);
            const lightness = 45 + Math.min(value * 25, 40);

            ctx.beginPath();
            ctx.moveTo(x1, y1);
            ctx.lineTo(x2, y2);
            ctx.strokeStyle = `hsla(${hue}, ${saturation}%, ${lightness}%, ${0.6 + value * 0.3})`;
            ctx.lineWidth = 3;
            ctx.lineCap = 'round';
            ctx.stroke();
        }

        // Draw center glow
        const gradient = ctx.createRadialGradient(
            centerX, centerY, 0,
            centerX, centerY, baseRadius + bassEnergy * 40
        );
        gradient.addColorStop(0, 'rgba(191, 194, 255, 0.3)');
        gradient.addColorStop(0.5, 'rgba(191, 194, 255, 0.1)');
        gradient.addColorStop(1, 'transparent');

        ctx.beginPath();
        ctx.arc(centerX, centerY, baseRadius + bassEnergy * 40, 0, Math.PI * 2);
        ctx.fillStyle = gradient;
        ctx.fill();
    }, [sensitivity, colors.primary]);

    const drawOrbVisualizer = useCallback((
        ctx: CanvasRenderingContext2D,
        width: number, height: number,
        centerX: number, centerY: number,
        bins: number[],
        bassEnergy: number,
        midEnergy: number,
        highEnergy: number
    ) => {
        const time = Date.now() * 0.001;
        const baseRadius = Math.min(width, height) * 0.2;
        const radiusModulation = bassEnergy * 80 * sensitivity;

        for (let layer = 0; layer < 3; layer++) {
            const layerOffset = layer * 0.3;
            const points = 128;
            const radius = baseRadius + radiusModulation * (1 - layer * 0.2);

            ctx.beginPath();

            for (let i = 0; i <= points; i++) {
                const angle = (i / points) * Math.PI * 2;
                const binIndex = Math.floor((i / points) * bins.length);
                const value = (bins[binIndex] ?? 0) * sensitivity;

                const noise = Math.sin(angle * 3 + time * 2 + layerOffset) * 0.3 +
                    Math.sin(angle * 5 - time * 1.5) * 0.2 +
                    Math.sin(angle * 7 + time * 3) * 0.1;

                const r = radius + value * 150 + noise * 30 * (1 + bassEnergy);
                const x = centerX + Math.cos(angle) * r;
                const y = centerY + Math.sin(angle) * r;

                if (i === 0) {
                    ctx.moveTo(x, y);
                } else {
                    ctx.lineTo(x, y);
                }
            }

            ctx.closePath();

            const hue = 260 + layer * 20 - midEnergy * 30;
            const saturation = 70 + highEnergy * 30;
            const lightness = 50 + layer * 10;
            const alpha = 0.4 - layer * 0.1;

            ctx.fillStyle = `hsla(${hue}, ${saturation}%, ${lightness}%, ${alpha})`;
            ctx.fill();

            ctx.strokeStyle = `hsla(${hue + 10}, ${saturation}%, ${lightness + 20}%, ${alpha + 0.2})`;
            ctx.lineWidth = 2;
            ctx.stroke();
        }

        // Inner core glow
        const coreGradient = ctx.createRadialGradient(
            centerX, centerY, 0,
            centerX, centerY, baseRadius * 0.8
        );
        coreGradient.addColorStop(0, `rgba(255, 255, 255, ${0.3 + bassEnergy * 0.3})`);
        coreGradient.addColorStop(0.5, `rgba(200, 150, 255, ${0.2 + midEnergy * 0.2})`);
        coreGradient.addColorStop(1, 'transparent');

        ctx.beginPath();
        ctx.arc(centerX, centerY, baseRadius * 0.8, 0, Math.PI * 2);
        ctx.fillStyle = coreGradient;
        ctx.fill();
    }, [sensitivity]);

    const updateAndDrawParticles = useCallback((
        ctx: CanvasRenderingContext2D,
        width: number, height: number,
        energy: number
    ) => {
        const particles = particlesRef.current;

        for (const particle of particles) {
            particle.update(width, height, energy);
            particle.draw(ctx, '191, 194, 255', energy);  // Use fixed RGB values
        }
    }, []);

    // Main draw function
    const draw = useCallback(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        const { width, height } = canvas;
        const centerX = width / 2;
        const centerY = height / 2;

        // Clear canvas completely each frame (no ghost trails)
        ctx.fillStyle = '#0a0a14';
        ctx.fillRect(0, 0, width, height);

        // Get audio energy
        const bins = frequencyBins.length > 0 ? frequencyBins : new Array(64).fill(0);
        const bassEnergy = bins.slice(0, 8).reduce((a: number, b: number) => a + b, 0) / 8;
        const midEnergy = bins.slice(8, 32).reduce((a: number, b: number) => a + b, 0) / 24;
        const highEnergy = bins.slice(32).reduce((a: number, b: number) => a + b, 0) / 32;
        const totalEnergy = (bassEnergy + midEnergy + highEnergy) / 3;

        if (mode === 'bars') {
            drawRadialBars(ctx, width, height, centerX, centerY, bins, bassEnergy);
        } else {
            drawOrbVisualizer(ctx, width, height, centerX, centerY, bins, bassEnergy, midEnergy, highEnergy);
        }

        // Draw particles
        updateAndDrawParticles(ctx, width, height, totalEnergy * sensitivity);

    }, [frequencyBins, mode, sensitivity, drawRadialBars, drawOrbVisualizer, updateAndDrawParticles]);

    // Animation loop - runs continuously when visible
    useEffect(() => {
        if (!isVisible) return;

        let mounted = true;

        const fetchData = async () => {
            if (!mounted) return;
            try {
                const data = await invoke<VisualizerData>('get_visualizer_data');
                if (mounted) {
                    updateData(data.frequency_bins, data.waveform);
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
    }, [isVisible, updateData, draw]);

    if (!isVisible) return null;

    return (
        <AnimatePresence>
            <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.5 }}
                className="fixed inset-0 z-[200] bg-[#0a0a14] flex flex-col"
            >
                {/* Canvas */}
                <canvas
                    ref={canvasRef}
                    className="absolute inset-0 w-full h-full"
                />

                {/* Top bar with close button */}
                <div className="absolute top-0 left-0 right-0 p-6 flex justify-between items-start z-10">
                    <div className="flex flex-col gap-1">
                        <span
                            className="text-sm font-medium opacity-60"
                            style={{ color: colors.onSurface }}
                        >
                            {mode === 'bars' ? 'Radial Bars' : 'Organic Flow'}
                        </span>
                        <span
                            className="text-xs opacity-40"
                            style={{ color: colors.onSurfaceVariant }}
                        >
                            Press M to switch • ESC to close
                        </span>
                    </div>
                    <button
                        onClick={() => setDisplayMode('off')}
                        className="p-3 rounded-full bg-white/5 hover:bg-white/10 transition-colors"
                        style={{ color: colors.onSurface }}
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
                            className="text-2xl font-semibold mb-1"
                            style={{ color: colors.onSurface }}
                        >
                            {track?.title || 'No Track Playing'}
                        </h2>
                        <p
                            className="text-sm opacity-60"
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
                            style={{
                                backgroundColor: colors.primary,
                                color: colors.onPrimary
                            }}
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
