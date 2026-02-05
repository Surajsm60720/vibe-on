import React, { useRef, useState } from 'react';
import { usePlayerStore } from '../store/playerStore';
import { motion, AnimatePresence } from 'framer-motion';

import { open, save } from '@tauri-apps/plugin-dialog';
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
import { downloadDir } from '@tauri-apps/api/path';

const BANDS = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
const BAND_LABELS = ['31', '62', '125', '250', '500', '1k', '2k', '4k', '8k', '16k'];

// Default preset IDs that cannot be deleted
const DEFAULT_PRESET_IDS = new Set([
    'flat', 'acoustic', 'classical', 'dance', 'deep', 'electronic', 'hip-hop',
    'jazz', 'latin', 'loudness', 'lounge', 'piano', 'pop', 'r&b', 'rock',
    'small-speakers', 'spoken-word', 'increase-bass', 'reduce-bass',
    'increase-treble', 'reduce-treble', 'increase-vocals'
]);


const Equalizer: React.FC<{ onClose: () => void }> = ({ onClose }) => {
    const { eqGains, setEqGain, presets, applyPreset, removePreset, activePresetId } = usePlayerStore();

    // State for save preset dialog
    const [showSaveDialog, setShowSaveDialog] = useState(false);
    const [presetName, setPresetName] = useState('');
    const [saveMessage, setSaveMessage] = useState<{ type: 'success' | 'error', text: string } | null>(null);

    const handleReset = () => {
        BANDS.forEach((_, index) => {
            setEqGain(index, 0);
        });
    };

    const handleSavePreset = () => {
        setPresetName('');
        setShowSaveDialog(true);
    };

    const confirmSavePreset = () => {
        if (!presetName.trim()) {
            setSaveMessage({ type: 'error', text: 'Please enter a preset name' });
            setTimeout(() => setSaveMessage(null), 3000);
            return;
        }

        // Create and apply the new preset immediately
        const newId = `custom-${Date.now()}`;
        const newPreset = { id: newId, name: presetName.trim(), gains: [...eqGains] };

        usePlayerStore.setState(state => ({
            presets: [...state.presets, newPreset],
            activePresetId: newId
        }));

        setShowSaveDialog(false);
        setPresetName('');
        setSaveMessage({ type: 'success', text: `Preset "${presetName.trim()}" saved!` });
        setTimeout(() => setSaveMessage(null), 3000);
    };

    const handleDeletePreset = (presetId: string, presetNameToDelete: string) => {
        if (DEFAULT_PRESET_IDS.has(presetId)) {
            setSaveMessage({ type: 'error', text: 'Cannot delete built-in presets' });
            setTimeout(() => setSaveMessage(null), 3000);
            return;
        }

        if (confirm(`Delete preset "${presetNameToDelete}"?`)) {
            removePreset(presetId);
            // If we deleted the active preset, switch to Flat
            if (activePresetId === presetId) {
                const flatPreset = presets.find(p => p.id === 'flat');
                if (flatPreset) applyPreset(flatPreset);
            }
            setSaveMessage({ type: 'success', text: `Preset "${presetNameToDelete}" deleted` });
            setTimeout(() => setSaveMessage(null), 3000);
        }
    };

    const handleExport = async () => {
        try {
            // Get current preset name if one is selected
            const currentPreset = presets.find(p => p.id === activePresetId);
            const defaultName = currentPreset ? `${currentPreset.name.replace(/\s+/g, '-').toLowerCase()}-preset.json` : 'my-eq-preset.json';

            // Get downloads directory as default location
            const downloadsPath = await downloadDir();

            const result = await save({
                filters: [{
                    name: 'JSON Preset',
                    extensions: ['json']
                }],
                defaultPath: `${downloadsPath}/${defaultName}`
            });

            if (result) {
                const content = JSON.stringify({
                    name: currentPreset?.name || 'Custom Preset',
                    gains: eqGains,
                    exportedAt: new Date().toISOString()
                }, null, 2);
                await writeTextFile(result, content);

                // Show success message with path
                setSaveMessage({ type: 'success', text: `Exported to: ${result}` });
                setTimeout(() => setSaveMessage(null), 5000);
            }
        } catch (e) {
            console.error("Export failed:", e);
            setSaveMessage({ type: 'error', text: `Export failed: ${e}` });
            setTimeout(() => setSaveMessage(null), 4000);
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

                const content = await readTextFile(path);
                let data;
                try {
                    data = JSON.parse(content);
                } catch (err) {
                    setSaveMessage({ type: 'error', text: 'Invalid JSON file' });
                    setTimeout(() => setSaveMessage(null), 3000);
                    return;
                }

                if (data.gains && Array.isArray(data.gains) && data.gains.length === 10) {
                    const importedName = data.name || "Imported Preset";
                    // Add the preset and immediately apply it
                    const newId = `custom-${Date.now()}`;
                    const newPreset = { id: newId, name: importedName, gains: [...data.gains] };

                    // Manually set the preset and apply it via store
                    usePlayerStore.setState(state => ({
                        presets: [...state.presets, newPreset],
                        activePresetId: newId,
                        eqGains: [...data.gains]
                    }));

                    // Apply gains to backend
                    data.gains.forEach((gain: number, index: number) => {
                        setEqGain(index, gain);
                    });

                    setSaveMessage({ type: 'success', text: `Imported & applied "${importedName}"!` });
                    setTimeout(() => setSaveMessage(null), 3000);
                } else {
                    setSaveMessage({ type: 'error', text: 'Invalid preset format (must have 10 gain values)' });
                    setTimeout(() => setSaveMessage(null), 3000);
                }
            }
        } catch (e) {
            console.error("Import failed:", e);
            setSaveMessage({ type: 'error', text: `Import failed: ${e}` });
            setTimeout(() => setSaveMessage(null), 4000);
        }
    };

    return (
        <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 pointer-events-auto"
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

                    {/* Toast Message */}
                    <AnimatePresence>
                        {saveMessage && (
                            <motion.div
                                initial={{ opacity: 0, y: -10 }}
                                animate={{ opacity: 1, y: 0 }}
                                exit={{ opacity: 0, y: -10 }}
                                className={`px-4 py-2 rounded-xl text-body-medium ${saveMessage.type === 'success'
                                        ? 'bg-primary/20 text-primary'
                                        : 'bg-error/20 text-error'
                                    }`}
                            >
                                {saveMessage.text}
                            </motion.div>
                        )}
                    </AnimatePresence>

                    {/* Presets Control Bar */}
                    <div className="flex gap-2 items-center bg-surface-container p-2 rounded-xl">
                        <div className="flex-1 flex items-center gap-2">
                            <select
                                className="flex-1 bg-transparent text-body-medium text-on-surface border-none outline-none cursor-pointer"
                                onChange={(e) => {
                                    const preset = presets.find(p => p.id === e.target.value);
                                    if (preset) applyPreset(preset);
                                }}
                                value={activePresetId || "manual"}
                            >
                                <option value="manual" disabled>Manual</option>
                                <optgroup label="Built-in Presets">
                                    {presets.filter(p => DEFAULT_PRESET_IDS.has(p.id)).map(p => (
                                        <option key={p.id} value={p.id}>{p.name}</option>
                                    ))}
                                </optgroup>
                                {presets.filter(p => !DEFAULT_PRESET_IDS.has(p.id)).length > 0 && (
                                    <optgroup label="Custom Presets">
                                        {presets.filter(p => !DEFAULT_PRESET_IDS.has(p.id)).map(p => (
                                            <option key={p.id} value={p.id}>★ {p.name}</option>
                                        ))}
                                    </optgroup>
                                )}
                            </select>

                            {/* Delete button for custom presets */}
                            {activePresetId && !DEFAULT_PRESET_IDS.has(activePresetId) && (
                                <button
                                    onClick={() => {
                                        const preset = presets.find(p => p.id === activePresetId);
                                        if (preset) handleDeletePreset(activePresetId, preset.name);
                                    }}
                                    className="p-1.5 hover:bg-error/20 rounded-full transition-colors text-on-surface-variant hover:text-error"
                                    title="Delete this preset"
                                >
                                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                                        <path d="M3 6h18M8 6V4h8v2M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
                                    </svg>
                                </button>
                            )}
                        </div>

                        <div className="w-px h-6 bg-outline-variant/30" />
                        <button onClick={handleSavePreset} className="text-label-medium text-primary hover:text-primary/80 px-2 py-1 rounded-lg hover:bg-primary/10 transition-colors">Save</button>
                        <button onClick={handleImport} className="text-label-medium text-on-surface-variant hover:text-on-surface px-2 py-1 rounded-lg hover:bg-surface-container-highest transition-colors">Import</button>
                        <button onClick={handleExport} className="text-label-medium text-on-surface-variant hover:text-on-surface px-2 py-1 rounded-lg hover:bg-surface-container-highest transition-colors">Export</button>
                    </div>

                    {/* Save Preset Dialog */}
                    <AnimatePresence>
                        {showSaveDialog && (
                            <motion.div
                                initial={{ opacity: 0, height: 0 }}
                                animate={{ opacity: 1, height: 'auto' }}
                                exit={{ opacity: 0, height: 0 }}
                                className="overflow-hidden"
                            >
                                <div className="flex gap-2 items-center bg-primary/10 p-3 rounded-xl border border-primary/30">
                                    <input
                                        type="text"
                                        placeholder="Enter preset name..."
                                        value={presetName}
                                        onChange={(e) => setPresetName(e.target.value)}
                                        onKeyDown={(e) => {
                                            if (e.key === 'Enter') confirmSavePreset();
                                            if (e.key === 'Escape') setShowSaveDialog(false);
                                        }}
                                        autoFocus
                                        className="flex-1 bg-surface-container px-3 py-2 rounded-lg text-body-medium text-on-surface outline-none border border-outline-variant/30 focus:border-primary transition-colors"
                                    />
                                    <button
                                        onClick={confirmSavePreset}
                                        className="px-4 py-2 bg-primary text-on-primary rounded-lg font-medium hover:bg-primary/90 transition-colors"
                                    >
                                        Save
                                    </button>
                                    <button
                                        onClick={() => setShowSaveDialog(false)}
                                        className="px-4 py-2 bg-surface-container text-on-surface rounded-lg font-medium hover:bg-surface-container-highest transition-colors"
                                    >
                                        Cancel
                                    </button>
                                </div>
                            </motion.div>
                        )}
                    </AnimatePresence>
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
                <div className="flex flex-col gap-4 pt-6 border-t border-outline-variant/10">
                    <h3 className="text-title-small font-semibold text-on-surface-variant mb-2">Advanced Audio</h3>
                    <p className="text-body-small text-on-surface-variant/60 -mt-2 mb-2">Drag up/down or left/right • Scroll to fine-tune • Double-click to reset</p>
                    <div className="flex justify-around items-start">
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
                            format={(v) => v === 0 ? "Center" : (v < 0 ? `L ${Math.abs(v * 100).toFixed(0)}%` : `R ${(v * 100).toFixed(0)}%`)}
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
                            format={(v) => `${(v * 100).toFixed(0)}%`}
                            onChange={(val) => usePlayerStore.getState().setReverb(usePlayerStore.getState().reverbMix, val)}
                            defaultValue={0.5}
                        />
                    </div>
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
    const knobRef = useRef<HTMLDivElement>(null);
    const [isDragging, setIsDragging] = useState(false);
    const startPos = useRef<{ x: number; y: number }>({ x: 0, y: 0 });
    const startVal = useRef<number>(0);

    const range = max - min;
    const normalized = (value - min) / range; // 0..1
    const angle = (normalized * 270) - 135; // -135 to +135 degrees

    const updateValue = (newVal: number) => {
        // Step quantization
        if (step > 0) {
            newVal = Math.round(newVal / step) * step;
        }
        // Clamp
        newVal = Math.min(Math.max(newVal, min), max);
        onChange(newVal);
    };

    const handlePointerDown = (e: React.PointerEvent) => {
        e.preventDefault();
        e.currentTarget.setPointerCapture(e.pointerId);
        setIsDragging(true);
        startPos.current = { x: e.clientX, y: e.clientY };
        startVal.current = value;
    };

    const handlePointerMove = (e: React.PointerEvent) => {
        if (!isDragging) return;
        e.preventDefault();

        // Support both vertical AND horizontal drag (use whichever is larger)
        const deltaX = e.clientX - startPos.current.x;
        const deltaY = startPos.current.y - e.clientY; // Invert Y so up = increase

        // Use the larger delta for more intuitive control
        const delta = Math.abs(deltaX) > Math.abs(deltaY) ? deltaX : deltaY;

        // Sensitivity: 150 pixels = full range
        const sensitivity = 150;
        const deltaVal = (delta / sensitivity) * range;

        updateValue(startVal.current + deltaVal);
    };

    const handlePointerUp = (e: React.PointerEvent) => {
        setIsDragging(false);
        e.currentTarget.releasePointerCapture(e.pointerId);
    };

    const handleWheel = (e: React.WheelEvent) => {
        e.preventDefault();
        // Scroll up = increase, scroll down = decrease
        const direction = e.deltaY < 0 ? 1 : -1;
        const newVal = value + (step * direction * 2); // 2x step for faster scrolling
        updateValue(newVal);
    };

    const handleDoubleClick = () => {
        onChange(defaultValue);
    };

    // Arc path calculation for the progress indicator
    const radius = 32;
    const strokeWidth = 6;
    const circumference = 2 * Math.PI * radius;
    const arcLength = circumference * 0.75; // 270 degrees
    const progressOffset = arcLength * (1 - normalized);

    return (
        <div className="flex flex-col items-center gap-3 select-none group">
            {/* Value display - always visible when interacting */}
            <div className={`h-6 text-label-medium font-mono transition-all duration-150 ${isDragging ? 'text-primary scale-110' : 'text-on-surface-variant'
                }`}>
                {format(value)}
            </div>

            <div
                ref={knobRef}
                className={`relative w-20 h-20 cursor-grab active:cursor-grabbing touch-none transition-transform ${isDragging ? 'scale-105' : 'hover:scale-102'
                    }`}
                onPointerDown={handlePointerDown}
                onPointerMove={handlePointerMove}
                onPointerUp={handlePointerUp}
                onWheel={handleWheel}
                onDoubleClick={handleDoubleClick}
            >
                {/* Background circle with track */}
                <svg width="80" height="80" viewBox="0 0 80 80" className="absolute inset-0">
                    {/* Track background (270 degree arc) */}
                    <circle
                        cx="40" cy="40" r={radius}
                        fill="none"
                        stroke="currentColor"
                        strokeWidth={strokeWidth}
                        className="text-surface-container-highest"
                        strokeLinecap="round"
                        strokeDasharray={`${arcLength} ${circumference}`}
                        transform="rotate(135 40 40)"
                    />
                    {/* Active progress arc */}
                    <circle
                        cx="40" cy="40" r={radius}
                        fill="none"
                        stroke="currentColor"
                        strokeWidth={strokeWidth}
                        className={`transition-colors ${isDragging ? 'text-primary' : 'text-primary/70 group-hover:text-primary'}`}
                        strokeLinecap="round"
                        strokeDasharray={`${arcLength} ${circumference}`}
                        strokeDashoffset={progressOffset}
                        transform="rotate(135 40 40)"
                    />
                </svg>

                {/* Knob body */}
                <div className={`absolute inset-2 rounded-full transition-all ${isDragging
                        ? 'bg-surface-container-high shadow-lg'
                        : 'bg-surface-container shadow-md group-hover:bg-surface-container-high'
                    }`}>
                    {/* Center dot */}
                    <div className="absolute inset-0 flex items-center justify-center">
                        <div className={`w-2 h-2 rounded-full transition-colors ${isDragging ? 'bg-primary' : 'bg-on-surface-variant/30'
                            }`} />
                    </div>

                    {/* Indicator line */}
                    <div
                        className="absolute inset-0 flex items-center justify-center"
                        style={{ transform: `rotate(${angle}deg)` }}
                    >
                        <div className={`absolute top-2 w-1 h-4 rounded-full transition-colors ${isDragging ? 'bg-primary' : 'bg-on-surface-variant group-hover:bg-primary/70'
                            }`} />
                    </div>
                </div>
            </div>

            <span className="text-label-small font-medium text-on-surface-variant">
                {label}
            </span>
        </div>
    );
};

export default Equalizer;
