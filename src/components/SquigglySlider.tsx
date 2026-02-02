import { useRef, useMemo, useState, useEffect } from 'react';
import { motion } from 'motion/react';

interface SquigglySliderProps {
    value: number;
    max: number;
    onChange: (newValue: number) => void;
    isPlaying?: boolean;
    className?: string;
    accentColor?: string;
}

export function SquigglySlider({ value, max, onChange, isPlaying = false, className = '', accentColor }: SquigglySliderProps) {
    const containerRef = useRef<HTMLDivElement>(null);
    const [width, setWidth] = useState(400);
    const [localValue, setLocalValue] = useState<number | null>(null);
    const [isDragging, setIsDragging] = useState(false);

    // Sync local value with prop when NOT dragging
    useEffect(() => {
        if (!isDragging) {
            setLocalValue(null);
        }
    }, [isDragging, value]);

    const displayValue = localValue !== null ? localValue : value;

    // Monitor container resizing to update wave density dynamically
    useEffect(() => {
        if (!containerRef.current) return;

        const observer = new ResizeObserver((entries) => {
            for (const entry of entries) {
                if (entry.contentRect.width > 0) {
                    setWidth(entry.contentRect.width);
                }
            }
        });

        observer.observe(containerRef.current);
        return () => observer.disconnect();
    }, []);

    const progress = Math.min(1, Math.max(0, displayValue / (max || 1)));
    const percent = progress * 100;

    // Generate paths based on ACTUAL container width
    const { straightPath, squigglyPath } = useMemo(() => {
        // Fallback for invalid width
        const safeWidth = typeof width === 'number' && !isNaN(width) && width > 0 ? width : 400;

        const wavelength = 25;
        const frequency = (2 * Math.PI) / wavelength;
        const amplitude = 4;
        const step = 2;

        let straight = `M 0 10`;
        let squiggly = `M 0 10`;

        for (let x = 0; x <= safeWidth; x += step) {
            straight += ` L ${x} 10`;
            const y = 10 + Math.sin(x * frequency) * amplitude;
            squiggly += ` L ${x} ${y}`;
        }

        straight += ` L ${safeWidth} 10`;
        squiggly += ` L ${safeWidth} ${10 + Math.sin(safeWidth * frequency) * amplitude}`;

        return {
            straightPath: straight || "M 0 10 L 400 10",
            squigglyPath: squiggly || "M 0 10 L 400 10"
        };
    }, [width]);

    const currentPath = (isPlaying ? squigglyPath : straightPath) || "M 0 10 L 400 10";

    const handleInput = (e: React.FormEvent<HTMLInputElement>) => {
        const val = parseFloat((e.target as HTMLInputElement).value);
        setLocalValue(val);
        setIsDragging(true);
    };

    const handleMouseUp = () => {
        if (localValue !== null) {
            onChange(localValue);
        }
        setIsDragging(false);
        setLocalValue(null);
    };

    return (
        <div className={`relative h-6 group ${className}`} ref={containerRef}>
            {/* SVG Background Layer (Inactive Track) */}
            <svg
                className="absolute inset-0 w-full h-full overflow-visible"
                preserveAspectRatio="none"
                viewBox={`0 0 ${width} 20`}
            >
                <motion.path
                    d={currentPath}
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="4"
                    className="text-white/10"
                    vectorEffect="non-scaling-stroke"
                    animate={{ d: currentPath }}
                    transition={{ duration: 0.8, type: "spring", bounce: 0.2 }}
                />
            </svg>

            {/* SVG Foreground Layer (Active Progress) - Clipped */}
            <div
                className="absolute inset-0 overflow-hidden"
                style={{ width: `${percent}%` }}
            >
                <svg
                    className="absolute top-0 left-0 h-full overflow-visible"
                    style={{ width: width }}
                    preserveAspectRatio="none"
                    viewBox={`0 0 ${width} 20`}
                >
                    <motion.path
                        d={currentPath}
                        fill="none"
                        stroke={accentColor || "currentColor"}
                        strokeWidth="4"
                        className={!accentColor ? "text-indigo-400 shadow-[0_0_10px_currentColor]" : "shadow-[0_0_10px_currentColor]"}
                        vectorEffect="non-scaling-stroke"
                        animate={{ d: currentPath }}
                        transition={{ duration: 0.8, type: "spring", bounce: 0.2 }}
                    />
                </svg>
            </div>

            {/* Thumb (Glowy Dot) */}
            <div
                className="absolute top-1/2 -translate-y-1/2 w-4 h-4 bg-white rounded-full shadow-lg opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none transform -translate-x-1/2"
                style={{ left: `${percent}%` }}
            >
                <div
                    className="absolute inset-0 rounded-full blur-[2px] opacity-50"
                    style={{ backgroundColor: accentColor || '#6366f1' }}
                />
            </div>

            {/* Invisible Range Input for Interaction */}
            <input
                type="range"
                min="0"
                max={max}
                step="0.1"
                value={displayValue}
                onInput={handleInput}
                onMouseUp={handleMouseUp}
                onTouchEnd={handleMouseUp}
                className="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-50 pointer-events-auto"
            />
        </div>
    );
}
