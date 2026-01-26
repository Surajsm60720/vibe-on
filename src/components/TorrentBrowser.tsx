import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { downloadDir } from '@tauri-apps/api/path';
import { TorrentSearch } from './TorrentSearch';

interface TorrentFile {
    index: number;
    name: string;
    path: string;
    size: number;
    is_audio: boolean;
}

interface InspectResult {
    name: string;
    files: TorrentFile[];
}

interface Props {
    onAdded: () => void;
}

export function TorrentBrowser({ onAdded }: Props) {
    const [step, setStep] = useState<'input' | 'inspecting' | 'selection'>('input');
    const [inputType, setInputType] = useState<'search' | 'magnet' | 'file'>('search');
    const [magnetLink, setMagnetLink] = useState('');
    const [fileBytes, setFileBytes] = useState<Uint8Array | null>(null);
    const [fileName, setFileName] = useState('');
    const [torrentName, setTorrentName] = useState('');

    const [files, setFiles] = useState<TorrentFile[]>([]);
    const [selectedIndices, setSelectedIndices] = useState<number[]>([]);
    const [downloadPath, setDownloadPath] = useState('');
    const [error, setError] = useState<string | null>(null);
    const [isStarting, setIsStarting] = useState(false);

    // Initialize (similar to useEffect on open)
    useEffect(() => {
        const initPath = async () => {
            const configuredPath = await import('../store/settingsStore').then(m => m.useSettingsStore.getState().downloadPath);

            if (configuredPath) {
                setDownloadPath(configuredPath);
            } else {
                try {
                    const baseDownloadDir = await downloadDir();
                    setDownloadPath(`${baseDownloadDir}vibe-on-music/`);
                } catch {
                    setDownloadPath('');
                }
            }
        };
        initPath();
    }, []);

    const handleFileSelect = async () => {
        try {
            const selected = await open({
                multiple: false,
                filters: [{ name: 'Torrent Files', extensions: ['torrent'] }]
            });
            if (selected && typeof selected === 'string') {
                const { readFile } = await import('@tauri-apps/plugin-fs');
                const bytes = await readFile(selected);
                setFileBytes(bytes);
                setFileName(selected.split('/').pop() || 'Selected File');
            }
        } catch (e) {
            setError('Failed to read file: ' + String(e));
        }
    };

    const handleInspect = async (magnetOverride?: string) => {
        setStep('inspecting');
        setError(null);
        try {
            let result: InspectResult;
            if (inputType === 'magnet' || (inputType === 'search' && magnetOverride)) {
                const mag = magnetOverride || magnetLink;
                if (!mag) throw new Error("Magnet link required");
                // Update state if it was an override
                if (magnetOverride) {
                    setMagnetLink(magnetOverride);
                    setInputType('magnet');
                }
                result = await invoke<InspectResult>('inspect_magnet', { magnet: mag });
            } else {
                if (!fileBytes) throw new Error("File required");
                result = await invoke<InspectResult>('inspect_torrent_file', { data: Array.from(fileBytes) });
            }
            setTorrentName(result.name);
            setFiles(result.files);
            // Select only audio files by default
            const audioIndices = result.files.filter(f => f.is_audio).map(f => f.index);
            setSelectedIndices(audioIndices.length > 0 ? audioIndices : result.files.map(f => f.index));
            setStep('selection');
        } catch (e) {
            setError(String(e));
            setStep('input');
        }
    };

    const handleBrowsePath = async () => {
        try {
            const selected = await open({
                directory: true,
                defaultPath: downloadPath || undefined,
            });
            if (selected && typeof selected === 'string') {
                // Ensure path ends with slash
                setDownloadPath(selected.endsWith('/') ? selected : selected + '/');
            }
        } catch (e) {
            console.error('Failed to select path:', e);
        }
    };

    const handleStartDownload = async () => {
        if (selectedIndices.length === 0) {
            setError('Please select at least one file to download');
            return;
        }

        setIsStarting(true);
        setError(null);

        try {
            // Sanitize torrent name for folder usage
            const safeName = torrentName.replace(/[<>:"/\\|?*]/g, '_').trim();
            const fullPath = downloadPath.replace(/[\\/]$/, '') + '/' + safeName;

            await invoke('add_torrent_with_options', {
                magnet: inputType === 'magnet' || inputType === 'search' ? magnetLink : null,
                fileBytes: inputType === 'file' && fileBytes ? Array.from(fileBytes) : null,
                path: fullPath,
                selectedFiles: selectedIndices,
            });
            onAdded();

            // Reset state after adding
            setStep('input');
            setMagnetLink('');
            setFileBytes(null);
            setFileName('');
            setTorrentName('');
            setFiles([]);
            setSelectedIndices([]);
        } catch (e) {
            setError(String(e));
            setIsStarting(false);
        }
    };

    const formatSize = (bytes: number) => {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    };

    const audioFileCount = files.filter(f => f.is_audio).length;
    const selectedAudioCount = files.filter(f => f.is_audio && selectedIndices.includes(f.index)).length;

    return (
        <div className="flex flex-col h-full overflow-hidden">
            {/* Steps & Navigation (embedded in content) */}
            {step === 'selection' && (
                <div className="shrink-0 mb-4 px-1">
                    <button
                        onClick={() => setStep('input')}
                        className="text-sm text-primary hover:underline flex items-center gap-1"
                    >
                        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                        </svg>
                        Back to Search
                    </button>
                </div>
            )}

            {step === 'input' && (
                <div className="flex-1 flex flex-col overflow-hidden">
                    <div className="flex gap-3 p-1 w-fit mb-6">
                        <button
                            onClick={() => setInputType('search')}
                            className={`px-5 py-2.5 rounded-xl text-sm font-bold transition-all shadow-sm ${inputType === 'search'
                                ? 'bg-primary text-on-primary ring-2 ring-primary/20'
                                : 'bg-surface-container-high text-on-surface-variant hover:bg-surface-container-highest hover:text-on-surface'
                                }`}
                        >
                            Search
                        </button>
                        <button
                            onClick={() => setInputType('magnet')}
                            className={`px-5 py-2.5 rounded-xl text-sm font-bold transition-all shadow-sm ${inputType === 'magnet'
                                ? 'bg-primary text-on-primary ring-2 ring-primary/20'
                                : 'bg-surface-container-high text-on-surface-variant hover:bg-surface-container-highest hover:text-on-surface'
                                }`}
                        >
                            Magnet Link
                        </button>
                        <button
                            onClick={() => setInputType('file')}
                            className={`px-5 py-2.5 rounded-xl text-sm font-bold transition-all shadow-sm ${inputType === 'file'
                                ? 'bg-primary text-on-primary ring-2 ring-primary/20'
                                : 'bg-surface-container-high text-on-surface-variant hover:bg-surface-container-highest hover:text-on-surface'
                                }`}
                        >
                            Torrent File
                        </button>
                    </div>

                    <div className="flex-1 overflow-hidden relative">
                        {inputType === 'search' && (
                            <TorrentSearch onSelectMagnet={(magnet) => handleInspect(magnet)} />
                        )}

                        {inputType === 'magnet' && (
                            <div className="space-y-4 max-w-2xl">
                                <label className="text-sm font-medium text-on-surface-variant">Magnet Link</label>
                                <textarea
                                    value={magnetLink}
                                    onChange={(e) => setMagnetLink(e.target.value)}
                                    placeholder="magnet:?xt=urn:btih:..."
                                    className="w-full h-32 px-4 py-3 rounded-xl bg-surface-container-high text-on-surface border-none focus:ring-2 focus:ring-primary outline-none resize-none"
                                />
                                <div className="bg-surface-container-highest p-4 rounded-xl text-sm text-on-surface-variant">
                                    Paste a magnet link above to start inspecting the torrent metadata.
                                </div>
                                <div className="flex justify-end">
                                    <button
                                        onClick={() => handleInspect()}
                                        disabled={!magnetLink}
                                        className="px-6 py-2 bg-primary text-on-primary rounded-xl font-medium hover:opacity-90 transition-opacity disabled:opacity-50"
                                    >
                                        Inspect
                                    </button>
                                </div>
                            </div>
                        )}

                        {inputType === 'file' && (
                            <div className="space-y-4 max-w-2xl">
                                <label className="text-sm font-medium text-on-surface-variant">Torrent File</label>
                                <div
                                    onClick={handleFileSelect}
                                    className="w-full h-48 border-2 border-dashed border-outline-variant rounded-xl flex flex-col items-center justify-center cursor-pointer hover:bg-surface-container-high transition-colors"
                                >
                                    {fileName ? (
                                        <div className="text-primary font-medium flex flex-col items-center">
                                            <svg className="w-8 h-8 mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                            </svg>
                                            {fileName}
                                        </div>
                                    ) : (
                                        <>
                                            <svg className="w-8 h-8 text-on-surface-variant mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                                            </svg>
                                            <span className="text-on-surface-variant">Click to select .torrent file</span>
                                        </>
                                    )}
                                </div>
                                <div className="flex justify-end">
                                    <button
                                        onClick={() => handleInspect()}
                                        disabled={!fileBytes}
                                        className="px-6 py-2 bg-primary text-on-primary rounded-xl font-medium hover:opacity-90 transition-opacity disabled:opacity-50"
                                    >
                                        Inspect
                                    </button>
                                </div>
                            </div>
                        )}
                    </div>
                </div>
            )}

            {step === 'inspecting' && (
                <div className="flex-1 flex flex-col items-center justify-center">
                    <div className="w-12 h-12 border-4 border-primary border-t-transparent rounded-full animate-spin mb-4" />
                    <p className="text-on-surface animate-pulse">Fetching Metadata...</p>
                    <p className="text-sm text-on-surface-variant mt-2">This may take up to a minute for magnet links</p>
                </div>
            )}

            {step === 'selection' && (
                <div className="flex-1 flex flex-col overflow-hidden">
                    <div className="space-y-6 h-full flex flex-col">
                        {/* Torrent Name */}
                        <div className="space-y-1 shrink-0">
                            <label className="text-sm font-medium text-on-surface-variant">Torrent</label>
                            <p className="text-on-surface font-medium truncate" title={torrentName}>{torrentName}</p>
                        </div>

                        {/* Path Selection */}
                        <div className="space-y-2 shrink-0">
                            <label className="text-sm font-medium text-on-surface-variant">Download Location</label>
                            <div className="flex gap-2">
                                <input
                                    readOnly
                                    value={downloadPath}
                                    className="flex-1 px-4 py-2 rounded-xl bg-surface-container-high text-on-surface text-sm border-none outline-none opacity-80"
                                />
                                <button
                                    onClick={handleBrowsePath}
                                    className="px-4 py-2 bg-surface-container-highest text-on-surface rounded-xl hover:bg-surface-container-high transition-colors text-sm font-medium"
                                >
                                    Browse
                                </button>
                            </div>
                        </div>

                        {/* M3 Expressive File List */}
                        <div className="space-y-3 flex-1 flex flex-col min-h-0">
                            <div className="flex justify-between items-end shrink-0 px-2">
                                <div>
                                    <label className="text-title-small font-bold text-on-surface">
                                        Files to Download
                                    </label>
                                    <p className="text-body-small text-on-surface-variant">
                                        {selectedIndices.length} of {files.length} selected
                                        {audioFileCount > 0 && (
                                            <span className="ml-2 text-primary font-medium">• {selectedAudioCount} audio files</span>
                                        )}
                                    </p>
                                </div>
                                <div className="flex gap-3">
                                    {audioFileCount > 0 && audioFileCount < files.length && (
                                        <button
                                            onClick={() => setSelectedIndices(files.filter(f => f.is_audio).map(f => f.index))}
                                            className="px-3 py-1.5 rounded-lg text-label-large font-medium text-primary hover:bg-primary/10 transition-colors"
                                        >
                                            Audio Only
                                        </button>
                                    )}
                                    <button
                                        onClick={() => {
                                            if (selectedIndices.length === files.length) setSelectedIndices([]);
                                            else setSelectedIndices(files.map(f => f.index));
                                        }}
                                        className="px-3 py-1.5 rounded-lg text-label-large font-medium text-primary hover:bg-primary/10 transition-colors"
                                    >
                                        {selectedIndices.length === files.length ? 'Deselect All' : 'Select All'}
                                    </button>
                                </div>
                            </div>

                            <div className="flex-1 overflow-y-auto rounded-[2rem] bg-surface-container-low p-3 scrollbar-thin scrollbar-thumb-outline/20">
                                {files.map((file) => {
                                    const isSelected = selectedIndices.includes(file.index);
                                    return (
                                        <div
                                            key={file.index}
                                            onClick={() => {
                                                if (isSelected) setSelectedIndices(selectedIndices.filter(i => i !== file.index));
                                                else setSelectedIndices([...selectedIndices, file.index]);
                                            }}
                                            className={`
                                                group flex items-center gap-4 p-4 mb-2 last:mb-0 rounded-[1.25rem] border transition-all duration-200 cursor-pointer
                                                ${isSelected
                                                    ? 'bg-secondary-container border-transparent'
                                                    : 'bg-surface-container hover:bg-surface-container-high border-transparent'
                                                }
                                            `}
                                        >
                                            {/* Custom Checkbox */}
                                            <div className={`
                                                w-6 h-6 rounded-lg border-2 flex items-center justify-center transition-colors
                                                ${isSelected
                                                    ? 'bg-primary border-primary'
                                                    : 'border-outline group-hover:border-primary'
                                                }
                                            `}>
                                                {isSelected && (
                                                    <svg className="w-4 h-4 text-on-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                                                    </svg>
                                                )}
                                            </div>

                                            {/* File Icon */}
                                            <div className={`
                                                w-10 h-10 rounded-xl flex items-center justify-center shrink-0
                                                ${isSelected
                                                    ? 'bg-primary/20 text-primary-dark'
                                                    : 'bg-surface-container-highest text-on-surface-variant'
                                                }
                                            `}>
                                                {file.is_audio ? (
                                                    <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
                                                    </svg>
                                                ) : (
                                                    <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
                                                    </svg>
                                                )}
                                            </div>

                                            {/* Metadata */}
                                            <div className="flex-1 min-w-0">
                                                <div className={`text-body-large font-medium truncate ${isSelected ? 'text-on-secondary-container' : 'text-on-surface'}`}>
                                                    {file.name}
                                                </div>
                                                <div className="flex items-center gap-2 text-label-medium text-on-surface-variant/80">
                                                    <span>{formatSize(file.size)}</span>
                                                    {file.is_audio && (
                                                        <span className="px-1.5 py-0.5 rounded-md bg-primary/10 text-primary text-[10px] font-bold uppercase tracking-wider">
                                                            Audio
                                                        </span>
                                                    )}
                                                </div>
                                            </div>
                                        </div>
                                    );
                                })}
                            </div>
                        </div>

                        <div className="flex justify-end pt-4">
                            <button
                                onClick={handleStartDownload}
                                disabled={selectedIndices.length === 0 || isStarting}
                                className="px-6 py-2 bg-primary text-on-primary rounded-xl font-medium hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
                            >
                                {isStarting ? (
                                    <>
                                        <div className="w-4 h-4 border-2 border-on-primary border-t-transparent rounded-full animate-spin" />
                                        Starting...
                                    </>
                                ) : (
                                    'Start Download'
                                )}
                            </button>
                        </div>
                    </div>
                </div>
            )}

            {error && (
                <div className="mt-4 p-3 bg-error/10 text-error text-sm rounded-lg border border-error/10 shrink-0">
                    {error}
                </div>
            )}
        </div>
    );
}
