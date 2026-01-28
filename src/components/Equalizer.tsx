import React, { useRef, useState } from 'react';
import { usePlayerStore } from '../store/playerStore';
import { motion } from 'framer-motion';

import { open, save } from '@tauri-apps/plugin-dialog';
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';

const BANDS = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
const BAND_LABELS = ['31', '62', '125', '250', '500', '1k', '2k', '4k', '8k', '16k'];


const Equalizer: React.FC<{ onClose: () => void }> = ({ onClose }) => {
    const { eqGains, setEqGain, presets, applyPreset, addPreset, activePresetId } = usePlayerStore();

    const handleReset = () => {
        BANDS.forEach((_, index) => {
            setEqGain(index, 0);
        });
    };

    const handleSavePreset = async () => {
        // Use a non-blocking dialog or stick to prompt for now, but ensure it works.
        // Prompt is fine for MVP but maybe we can make it nicer later.
        const name = prompt("Enter preset name:");
        if (name) {
            addPreset(name, eqGains);
            // Force re-render if needed, but store subscription should handle it.
        }
    };

    const handleExport = async () => {
        try {
            const result = await save({
                filters: [{
                    name: 'JSON Preset',
                    extensions: ['json']
                }],
                defaultPath: 'my-preset.json'
            });

            if (result) {
                const content = JSON.stringify({
                    name: 'Custom Preset',
                    gains: eqGains
                }, null, 2);
                await writeTextFile(result, content);
                // Use a toast or non-blocking notification if possible, revert to alert for now
                alert("Preset exported successfully!");
            }
        } catch (e) {
            console.error("Export failed:", e);
            alert("Failed to export preset");
        }
    };

    const handleImport = async () => {
        try {
            const result = await open({
                multiple: false,
                filters: [{
                    name: 'JSON Preset',
                    extensions: ['json']
                }]
            });

            if (result) {
                const path = result as string;
                // Note: v2 dialog returns string | null for single file

                const content = await readTextFile(path);
                let data;
                try {
                    data = JSON.parse(content);
                } catch (err) {
                    alert("Invalid JSON file.");
                    return;
                }

                if (data.gains && Array.isArray(data.gains) && data.gains.length === 10) {
                    const presetName = data.name || "Imported Preset";
                    addPreset(presetName, data.gains);
                    alert(`Imported "${presetName}" successfully!`);
                } else {
                    alert("Invalid preset format (must have 10 gain values).");
                }
            }
        } catch (e) {
            console.error("Import failed:", e);
            alert("Failed to import preset: " + e);
        }
    };

    return (
        <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm pointer-events-auto"
            onClick={(e) => {
                // Close if clicking the backdrop
                if (e.target === e.currentTarget) onClose();
            }}
        >
            <motion.div
                initial={{ scale: 0.9, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                exit={{ scale: 0.9, opacity: 0 }}
                transition={{ type: "spring", duration: 0.4 }}
                className="bg-surface-container-high border border-outline-variant/20 p-8 rounded-[2rem] shadow-elevation-3 w-[800px] pointer-events-auto max-h-[90vh] overflow-y-auto"
                onClick={(e) => e.stopPropagation()}
            >
                <div className="flex flex-col gap-4 mb-6">
                    <div className="flex justify-between items-center">
                        <h2 className="text-title-large font-bold text-on-surface">
                            Equalizer
                        </h2>
                        <div className="flex gap-2">
                            <button
                                onClick={handleReset}
                                className="p-2 hover:bg-surface-container-highest rounded-full transition-colors text-on-surface-variant hover:text-primary group"
                                title="Reset EQ"
                            >
                                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="transition-transform group-active:rotate-180">
                                    <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                                    <path d="M3 3v5h5" />
                                </svg>
                            </button>
                            <button
                                onClick={onClose}
                                className="px-4 py-2 rounded-full hover:bg-surface-container-highest text-on-surface font-medium transition-colors"
                            >
                                Close
                            </button>
                        </div>
                    </div>

                    {/* Presets Control Bar */}
                    <div className="flex gap-2 items-center bg-surface-container p-2 rounded-xl">
                        <select
                            className="flex-1 bg-transparent text-body-medium text-on-surface border-none outline-none cursor-pointer"
                            onChange={(e) => {
                                const preset = presets.find(p => p.id === e.target.value);
                                if (preset) applyPreset(preset);
                            }}
                            value={activePresetId || "manual"}
                        >
                            <option value="manual" disabled>Manual</option>
                            <option value="" disabled hidden>Select Preset...</option>
                            {presets.map(p => (
                                <option key={p.id} value={p.id}>{p.name}</option>
                            ))}
                        </select>
                        <div className="w-px h-6 bg-outline-variant/30" />
                        <button onClick={handleSavePreset} className="text-label-medium text-primary hover:text-primary-hover px-2">Save</button>
                        <button onClick={handleImport} className="text-label-medium text-on-surface-variant hover:text-on-surface px-2">Import</button>
                        <button onClick={handleExport} className="text-label-medium text-on-surface-variant hover:text-on-surface px-2">Export</button>
                    </div>
                </div>

                {/* EQ Bands - NOW 11 Columns (Preamp + 10 Bands) */}
                <div className="grid grid-cols-11 gap-2 h-64 items-end pb-4 pt-2">
                    {/* Preamp Slider */}
                    <VerticalSlider
                        label="Preamp"
                        value={usePlayerStore(s => s.preampDb)}
                        onChange={usePlayerStore(s => s.setPreamp)}
                        isPreamp={true}
                    />

                    {/* Frequency Bands */}
                    {BANDS.map((freq, index) => {
                        const gain = eqGains[index] || 0;

                        return (
                            <VerticalSlider
                                key={freq}
                                label={BAND_LABELS[index]}
                                value={gain}
                                onChange={(val) => setEqGain(index, val)}
                            />
                        );
                    })}
                </div>

                <div className="text-center text-body-small text-on-surface-variant mt-2 mb-8">
                    Adjust Preamp and frequency bands from -12dB to +12dB
                </div>

                {/* Advanced DSP Controls - Rotary Knobs */}
                <div className="flex justify-around items-center pt-6 border-t border-outline-variant/10">
                    <RotaryKnob
                        label="Stereo Width"
                        value={usePlayerStore(s => s.stereoWidth)}
                        min={0} max={2.0}
                        step={0.1}
                        format={(v) => `${(v * 100).toFixed(0)}%`}
                        onChange={usePlayerStore(s => s.setStereoWidth)}
                        defaultValue={1.0}
                    />
                    <RotaryKnob
                        label="Balance"
                        value={usePlayerStore(s => s.balance)}
                        min={-1} max={1}
                        step={0.05}
                        format={(v) => v === 0 ? "Center" : (v < 0 ? `L ${Math.abs(v).toFixed(2)}` : `R ${Math.abs(v).toFixed(2)}`)}
                        onChange={usePlayerStore(s => s.setBalance)}
                        defaultValue={0.0}
                    />
                    <RotaryKnob
                        label="Speed"
                        value={usePlayerStore(s => s.speed)}
                        min={0.5} max={2.0}
                        step={0.05}
                        format={(v) => `${v.toFixed(2)}x`}
                        onChange={usePlayerStore(s => s.setSpeed)}
                        defaultValue={1.0}
                    />
                    <RotaryKnob
                        label="Reverb Mix"
                        value={usePlayerStore(s => s.reverbMix)}
                        min={0.0} max={1.0}
                        step={0.05}
                        format={(v) => `${(v * 100).toFixed(0)}%`}
                        onChange={(val) => usePlayerStore.getState().setReverb(val, usePlayerStore.getState().reverbDecay)}
                        defaultValue={0.0}
                    />
                    <RotaryKnob
                        label="Reverb Decay"
                        value={usePlayerStore(s => s.reverbDecay)}
                        min={0.0} max={1.0}
                        step={0.05}
                        format={(v) => v.toFixed(2)}
                        onChange={(val) => usePlayerStore.getState().setReverb(usePlayerStore.getState().reverbMix, val)}
                        defaultValue={0.5}
                    />
                </div>
            </motion.div>
        </motion.div>
    );
};

// Updated VerticalSlider to handle Preamp distinct styling if needed
const VerticalSlider = ({ label, value, onChange, isPreamp = false }: { label: string, value: number, onChange: (val: number) => void, isPreamp?: boolean }) => {
    const sliderRef = useRef<HTMLDivElement>(null);
    const [isDragging, setIsDragging] = useState(false);
    const [isEditing, setIsEditing] = useState(false);
    const [editValue, setEditValue] = useState(value.toString());

    // Normalized 0 to 1
    // -12 -> 0, +12 -> 1, 0 -> 0.5
    const normalized = (value + 12) / 24;
    const percent = Math.min(Math.max(normalized * 100, 0), 100);

    const updateValue = (clientY: number) => {
        if (!sliderRef.current) return;
        const rect = sliderRef.current.getBoundingClientRect();
        const height = rect.height;
        const bottom = rect.bottom;

        // Calculate Y relative to bottom (0 at bottom, height at top)
        const y = bottom - clientY;

        // Clamp 0 to height
        const clampedY = Math.min(Math.max(y, 0), height);

        const ratio = clampedY / height;

        // Map 0..1 to -12..12
        const newValue = (ratio * 24) - 12;
        onChange(newValue);
    };

    const handlePointerDown = (e: React.PointerEvent) => {
        if (isEditing) return; // Don't drag if editing
        e.preventDefault();
        e.currentTarget.setPointerCapture(e.pointerId);
        setIsDragging(true);
        updateValue(e.clientY);
    };

    const handlePointerMove = (e: React.PointerEvent) => {
        if (isDragging && !isEditing) {
            e.preventDefault();
            updateValue(e.clientY);
        }
    };

    const handlePointerUp = (e: React.PointerEvent) => {
        if (isDragging) {
            e.preventDefault();
            setIsDragging(false);
            e.currentTarget.releasePointerCapture(e.pointerId);
        }
    };

    const handleEditStart = (e: React.MouseEvent) => {
        e.stopPropagation();
        setEditValue(value.toFixed(1));
        setIsEditing(true);
    };

    const handleEditSubmit = () => {
        setIsEditing(false);
        const num = parseFloat(editValue);
        if (!isNaN(num)) {
            // Clamp between -12 and 12
            const clamped = Math.min(Math.max(num, -12), 12);
            onChange(clamped);
        }
    };

    return (
        <div className="flex flex-col items-center h-full gap-3 group select-none">
            {/* Value Tooltip / Input */}
            {isEditing ? (
                <input
                    autoFocus
                    type="number"
                    step="0.1"
                    min="-12"
                    max="12"
                    value={editValue}
                    onChange={(e) => setEditValue(e.target.value)}
                    onBlur={handleEditSubmit}
                    onKeyDown={(e) => e.key === 'Enter' && handleEditSubmit()}
                    className="w-12 text-center text-label-small font-mono bg-surface-container-highest rounded border border-primary outline-none text-on-surface"
                />
            ) : (
                <div
                    onClick={handleEditStart}
                    className={`
                        text-label-small font-mono mb-1 transition-opacity cursor-text
                        ${isDragging ? 'opacity-100 text-primary' : 'opacity-0 group-hover:opacity-100 text-on-surface-variant'}
                    `}
                    title="Click to edit value"
                >
                    {value > 0 ? '+' : ''}{value.toFixed(1)}
                </div>
            )}

            <div
                ref={sliderRef}
                className={`relative w-8 h-full rounded-full cursor-ns-resize touch-none ${isPreamp ? 'bg-surface-container-highest' : 'bg-surface-container-highest'}`}
                onPointerDown={handlePointerDown}
                onPointerMove={handlePointerMove}
                onPointerUp={handlePointerUp}
            >
                {/* Center Line (0dB) */}
                <div className="absolute top-1/2 left-0 right-0 h-px bg-on-surface-variant/20" />

                {/* Fill Bar */}
                {/* We want to show fill from 0dB (center) to the value */}
                {/* 0dB is at 50% height. */}
                {/* properties: bottom is always 50%? No. */}
                {/* If val > 0, bottom is 50%, height is (val/12)*50% */}
                {/* If val < 0, top is 50%, height is (-val/12)*50% */}

                <div
                    className={`absolute w-full rounded-full transition-colors duration-100 ${isDragging ? 'bg-primary' : (isPreamp ? 'bg-tertiary' : 'bg-primary/80 group-hover:bg-primary')}`}
                    style={{
                        bottom: value >= 0 ? '50%' : `${50 - (Math.abs(value) / 24) * 100}%`,
                        height: `${(Math.abs(value) / 24) * 100}%`,
                        // Use a min-height so 0 is visible as a line? Or just let center line handle it.
                    }}
                />

                {/* Handle / Thumb */}
                <div
                    className={`absolute left-1/2 -translate-x-1/2 w-4 h-4 rounded-full shadow-md pointer-events-none transition-transform duration-75 ${isPreamp ? 'bg-on-tertiary' : 'bg-on-primary'}`}
                    style={{
                        bottom: `calc(${percent}% - 8px)`,
                        transform: isDragging ? 'translateX(-50%) scale(1.2)' : 'translateX(-50%) scale(1)'
                    }}
                />
            </div>

            <span className={`text-label-small font-medium ${isPreamp ? 'text-tertiary' : 'text-on-surface-variant'}`}>
                {label}
            </span>
        </div>
    );
};

const RotaryKnob = ({
    label, value, min, max, step, format, onChange, defaultValue
}: {
    label: string,
    value: number,
    min: number,
    max: number,
    step: number,
    format: (v: number) => string,
    onChange: (val: number) => void,
    defaultValue: number
}) => {
    const [isDragging, setIsDragging] = useState(false);
    const startY = useRef<number>(0);
    const startVal = useRef<number>(0);

    // Calculate rotation angle
    // Map min..max to -135deg .. +135deg (270 degree range)
    const range = max - min;
    const normalized = (value - min) / range; // 0..1
    const angle = (normalized * 270) - 135;

    const handlePointerDown = (e: React.PointerEvent) => {
        e.preventDefault();
        e.currentTarget.setPointerCapture(e.pointerId);
        setIsDragging(true);
        startY.current = e.clientY;
        startVal.current = value;
    };

    const handlePointerMove = (e: React.PointerEvent) => {
        if (!isDragging) return;
        e.preventDefault();

        // Drag Sensitivity
        const deltaY = startY.current - e.clientY;
        // 100 pixels = full range?
        const pixelRange = 200;
        const deltaVal = (deltaY / pixelRange) * range;

        let newVal = startVal.current + deltaVal;
        // Clamp
        newVal = Math.min(Math.max(newVal, min), max);

        // Step quantization
        if (step > 0) {
            newVal = Math.round(newVal / step) * step;
        }

        onChange(newVal);
    };

    const handleDoubleClick = () => {
        onChange(defaultValue);
    }

    return (
        <div className="flex flex-col items-center gap-2 select-none group">
            {/* Value text above */}
            <div className="h-5 text-label-medium font-mono text-primary opacity-0 group-hover:opacity-100 transition-opacity">
                {format(value)}
            </div>

            <div
                className="relative w-16 h-16 cursor-ns-resize"
                onPointerDown={handlePointerDown}
                onPointerMove={handlePointerMove}
                onPointerUp={(e) => {
                    setIsDragging(false);
                    e.currentTarget.releasePointerCapture(e.pointerId);
                }}
                onDoubleClick={handleDoubleClick}
                title="Double click to reset"
            >
                {/* Background Track Circle */}
                <svg width="64" height="64" viewBox="0 0 64 64" className="transform -rotate-90">
                    <circle
                        cx="32" cy="32" r="28"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="4"
                        className="text-surface-container-highest"
                        strokeDasharray={`${2 * Math.PI * 28 * 0.75} ${2 * Math.PI * 28 * 0.25}`}
                    // Dasharray for 270 degrees 
                    />
                    {/* Active Arc */}
                    <circle
                        cx="32" cy="32" r="28"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="4"
                        className="text-primary"
                        strokeDasharray={`${2 * Math.PI * 28}`}
                        strokeDashoffset={`${2 * Math.PI * 28 * (1 - (normalized * 0.75))}`}
                        // This math is tricky for SVG arcs. 
                        // Let's simplify: simple knob with visual marker is easier than SVG arc math for quick tasks.
                        // But SVG arc looks premium.
                        // Circumference C = 175.9
                        // Visible arc 75% = 132
                        // Offset starts at 0 (full) to 132 (empty).
                        // if value is 0 (min), offset should be 132 (hidden).
                        // if value is 1 (max), offset should be 0 (full).
                        strokeLinecap="round"
                        style={{
                            strokeDasharray: `${2 * Math.PI * 28 * 0.75} ${2 * Math.PI * 28}`,
                            strokeDashoffset: (2 * Math.PI * 28 * 0.75) * (1 - normalized)
                        }}
                    />
                </svg>

                {/* Knob body (invisible trigger or center) */}

                {/* Marker (Dot) */}
                <div
                    className="absolute top-0 left-0 w-full h-full pointer-events-none flex items-center justify-center"
                    style={{ transform: `rotate(${angle}deg)` }}
                >
                    <div className="absolute top-[8px] w-1.5 h-1.5 bg-on-primary rounded-full shadow-sm" />
                </div>
            </div>

            <span className="text-label-small font-medium text-on-surface-variant">
                {label}
            </span>
        </div>
    );
};

export default Equalizer;
