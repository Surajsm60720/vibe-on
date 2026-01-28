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

    const handleSavePreset = () => {
        const name = prompt("Enter preset name:");
        if (name) {
            addPreset(name, eqGains);
        }
    };

    const handleExport = async () => {
        try {
            const result = await save({
                filters: [{
                    name: 'JSON Preset',
                    extensions: ['json']
                }]
            });

            if (result) {
                const content = JSON.stringify({
                    name: 'Custom Preset',
                    gains: eqGains
                }, null, 2);
                await writeTextFile(result, content);
                alert("Preset export successful!");
            }
        } catch (e) {
            console.error(e);
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
                // plugin-dialog returns path string (or array, or null)
                // In v2 it returns string | null for multiple: false ??
                // Wait, result is FileResponse or string? 
                // Actually @tauri-apps/plugin-dialog v2 returns string path directly or null.
                const path = result as string;

                const content = await readTextFile(path);
                const data = JSON.parse(content);

                if (data.gains && Array.isArray(data.gains) && data.gains.length === 10) {
                    addPreset(data.name || "Imported Preset", data.gains);
                    alert(`Imported "${data.name}" successfully!`);
                } else {
                    alert("Invalid preset file format.");
                }
            }
        } catch (e) {
            console.error(e);
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
                className="bg-surface-container-high border border-outline-variant/20 p-8 rounded-[2rem] shadow-elevation-3 w-[700px] pointer-events-auto"
                onClick={(e) => e.stopPropagation()}
            >
                <div className="flex flex-col gap-4 mb-4">
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

                <div className="grid grid-cols-10 gap-3 h-56 items-end pb-4">
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

                <div className="text-center text-body-small text-on-surface-variant mt-4">
                    Adjust frequency bands from -12dB to +12dB
                </div>
            </motion.div>
        </motion.div>
    );
};

const VerticalSlider = ({ label, value, onChange }: { label: string, value: number, onChange: (val: number) => void }) => {
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

    const handleEditChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setEditValue(e.target.value);
    };

    const handleEditSubmit = () => {
        setIsEditing(false);
        const num = parseFloat(editValue);
        if (!isNaN(num)) {
            // Clamp between -12 and 12
            const clamped = Math.min(Math.max(num, -12), 12);
            onChange(clamped);
        }
        // If NaN, just revert (do nothing, effective value stays same)
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter') handleEditSubmit();
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
                    onChange={handleEditChange}
                    onBlur={handleEditSubmit}
                    onKeyDown={handleKeyDown}
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
                className="relative w-8 h-full bg-surface-container-highest rounded-full cursor-ns-resize touch-none"
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
                    className={`absolute w-full rounded-full transition-colors duration-100 ${isDragging ? 'bg-primary' : 'bg-primary/80 group-hover:bg-primary'}`}
                    style={{
                        bottom: value >= 0 ? '50%' : `${50 - (Math.abs(value) / 24) * 100}%`,
                        height: `${(Math.abs(value) / 24) * 100}%`,
                        // Use a min-height so 0 is visible as a line? Or just let center line handle it.
                    }}
                />

                {/* Handle / Thumb */}
                <div
                    className="absolute left-1/2 -translate-x-1/2 w-4 h-4 bg-on-primary rounded-full shadow-md pointer-events-none transition-transform duration-75"
                    style={{
                        bottom: `calc(${percent}% - 8px)`,
                        transform: isDragging ? 'translateX(-50%) scale(1.2)' : 'translateX(-50%) scale(1)'
                    }}
                />
            </div>

            <span className="text-label-small text-on-surface-variant font-medium">
                {label}
            </span>
        </div>
    );
};

export default Equalizer;
