import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { TorrentBrowser } from './TorrentBrowser';
import { downloadDir } from '@tauri-apps/api/path';
import { motion, AnimatePresence } from 'framer-motion';

interface TorrentStatus {
    id: number;
    name: string;
    progress: number;
    download_speed: number;
    upload_speed: number;
    state: string;
    total_size: number;
    downloaded_size: number;
    peers_connected: number;
    error: string | null;
}

export function TorrentManager() {
    const [activeTab, setActiveTab] = useState<'downloads' | 'browse'>('downloads');
    const [torrents, setTorrents] = useState<TorrentStatus[]>([]);
    const [initError, setInitError] = useState<string | null>(null);

    const fetchTorrents = async () => {
        try {
            const list = await invoke<TorrentStatus[]>('get_torrents');
            setTorrents(list);
        } catch (e) {
            console.error("Failed to fetch torrents", e);
        }
    };

    const handleDownloadAdded = () => {
        fetchTorrents();
        // Switch to downloads tab to see progress
        setActiveTab('downloads');
    };

    useEffect(() => {
        const init = async () => {
            try {
                const baseDir = await downloadDir();
                const path = `${baseDir}vibe-on-music`;
                await invoke('init_torrent_backend', { downloadDir: path });
                fetchTorrents();
                // Poll for updates
                const interval = setInterval(fetchTorrents, 2000);
                return () => clearInterval(interval);
            } catch (e) {
                setInitError(String(e));
                console.error("Backend init failed", e);
            }
        };
        init();
    }, []);

    const formatSize = (bytes: number) => {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    };

    const formatSpeed = (bytesPerSec: number) => {
        return formatSize(bytesPerSec) + '/s';
    };

    const handlePause = async (id: number) => {
        try {
            await invoke('pause_torrent', { id });
            fetchTorrents();
        } catch (e) {
            console.error("Failed to pause", e);
        }
    };

    const handleResume = async (id: number) => {
        try {
            await invoke('resume_torrent', { id });
            fetchTorrents();
        } catch (e) {
            console.error("Failed to resume", e);
        }
    };



    const [deleteConfirmId, setDeleteConfirmId] = useState<number | null>(null);

    const requestDelete = (id: number) => {
        setDeleteConfirmId(id);
    };

    const confirmDelete = async (deleteFiles: boolean) => {
        if (deleteConfirmId === null) return;
        try {
            await invoke('delete_torrent', { id: deleteConfirmId, deleteFiles });
            fetchTorrents();
        } catch (e) {
            console.error("Failed to delete", e);
        } finally {
            setDeleteConfirmId(null);
        }
    };

    return (
        <div className="h-full flex flex-col overflow-hidden relative">
            {/* Delete Confirmation Modal */}
            <AnimatePresence>
                {deleteConfirmId !== null && (
                    <div className="absolute inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-6">
                        <motion.div
                            initial={{ opacity: 0, scale: 0.9 }}
                            animate={{ opacity: 1, scale: 1 }}
                            exit={{ opacity: 0, scale: 0.9 }}
                            className="bg-surface-container-high p-6 rounded-2xl shadow-2xl max-w-sm w-full border border-outline-variant"
                        >
                            <h3 className="text-xl font-bold mb-2 text-on-surface">Remove Torrent?</h3>
                            <p className="text-on-surface-variant mb-6 text-sm">
                                Do you want to remove this torrent from the list, or also delete the downloaded files?
                            </p>
                            <div className="flex flex-col gap-3">
                                <button
                                    onClick={() => confirmDelete(false)}
                                    className="w-full py-3 rounded-xl bg-secondary-container text-on-secondary-container font-medium hover:opacity-90 transition-opacity"
                                >
                                    Remove from list only
                                </button>
                                <button
                                    onClick={() => confirmDelete(true)}
                                    className="w-full py-3 rounded-xl bg-error text-on-error font-medium hover:opacity-90 transition-opacity"
                                >
                                    Delete list + files
                                </button>
                                <button
                                    onClick={() => setDeleteConfirmId(null)}
                                    className="w-full py-2 text-on-surface-variant font-medium hover:text-on-surface transition-colors mt-2"
                                >
                                    Cancel
                                </button>
                            </div>
                        </motion.div>
                    </div>
                )}
            </AnimatePresence>

            {/* Header / Tabs */}
            <div className="px-6 pt-6 pb-2 shrink-0 flex items-center justify-between">
                <div className="flex gap-2">
                    <button
                        onClick={() => setActiveTab('downloads')}
                        className={`px-4 py-2 rounded-xl text-sm font-bold transition-all ${activeTab === 'downloads'
                            ? 'bg-primary text-on-primary shadow-lg'
                            : 'bg-surface-container text-on-surface-variant hover:bg-surface-container-high hover:text-on-surface'
                            }`}
                    >
                        Active Downloads
                        {torrents.length > 0 && (
                            <span className={`ml-2 px-1.5 py-0.5 rounded text-[10px] ${activeTab === 'downloads' ? 'bg-white/20' : 'bg-black/10'
                                }`}>
                                {torrents.length}
                            </span>
                        )}
                    </button>
                    <button
                        onClick={() => setActiveTab('browse')}
                        className={`px-4 py-2 rounded-xl text-sm font-bold transition-all ${activeTab === 'browse'
                            ? 'bg-primary text-on-primary shadow-lg'
                            : 'bg-surface-container text-on-surface-variant hover:bg-surface-container-high hover:text-on-surface'
                            }`}
                    >
                        Browse & Search
                    </button>
                </div>
            </div>

            {/* Error State */}
            {initError && (
                <div className="mx-6 mb-2 p-4 bg-error/10 text-error rounded-2xl border border-error/20 flex gap-4 items-center">
                    <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    <div>
                        <p className="font-bold">Backend Initialization Failed</p>
                        <p className="text-sm opacity-80">{initError}</p>
                    </div>
                </div>
            )}

            {/* Content Area */}
            <div className="flex-1 overflow-hidden relative">
                <AnimatePresence mode="wait">
                    {activeTab === 'downloads' ? (
                        <motion.div
                            key="downloads"
                            initial={{ opacity: 0, x: -20 }}
                            animate={{ opacity: 1, x: 0 }}
                            exit={{ opacity: 0, x: -20 }}
                            className="h-full p-6 pt-2 overflow-y-auto space-y-4 scrollbar-thin scrollbar-thumb-outline/20"
                        >
                            {torrents.length === 0 ? (
                                <div className="h-full flex flex-col items-center justify-center text-on-surface-variant opacity-60">
                                    <div className="bg-surface-container-highest p-8 rounded-full mb-6">
                                        <svg className="w-16 h-16" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                                        </svg>
                                    </div>
                                    <p className="text-xl font-medium">Nothing downloading yet</p>
                                    <div className="mt-4">
                                        <button
                                            onClick={() => setActiveTab('browse')}
                                            className="text-primary font-bold hover:underline"
                                        >
                                            Go to Browse
                                        </button>
                                        <span className="text-sm ml-1">to add new torrents</span>
                                    </div>
                                </div>
                            ) : (
                                torrents.map((t) => (
                                    <div
                                        key={t.id}
                                        className="bg-surface-container-low rounded-3xl p-6 border border-outline-variant/20 hover:border-primary/20 transition-colors shadow-sm group"
                                    >
                                        {/* Existing Card Content Structure reused exactly */}
                                        <div className="flex items-center gap-6">
                                            {/* Icon / Status */}
                                            <div className={`
                                                w-12 h-12 rounded-2xl flex items-center justify-center shrink-0
                                                ${t.state === 'Finished' ? 'bg-green-500/20 text-green-500' :
                                                    t.state === 'Downloading' ? 'bg-primary/20 text-primary' :
                                                        'bg-surface-container-highest text-on-surface-variant'}
                                            `}>
                                                {t.state === 'Finished' ? (
                                                    <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M5 13l4 4L19 7" />
                                                    </svg>
                                                ) : t.state === 'Downloading' ? (
                                                    <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                                                    </svg>
                                                ) : (
                                                    <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M10 9v6m4-6v6" />
                                                    </svg>
                                                )}
                                            </div>

                                            {/* Info */}
                                            <div className="flex-1 min-w-0">
                                                <div className="flex justify-between items-start mb-2">
                                                    <h3 className="text-lg font-bold text-on-surface truncate pr-4" title={t.name}>
                                                        {t.name}
                                                    </h3>
                                                    <div className="flex items-center gap-3">
                                                        <span className={`
                                                            text-xs font-bold uppercase tracking-wider px-2 py-1 rounded-md
                                                            ${t.state === 'Finished' ? 'bg-green-500/10 text-green-500' :
                                                                t.state === 'Downloading' ? 'bg-primary/10 text-primary' :
                                                                    t.error ? 'bg-error/10 text-error' :
                                                                        'bg-surface-container-highest text-on-surface-variant'}
                                                        `}>
                                                            {t.state}
                                                        </span>
                                                    </div>
                                                </div>

                                                {/* Progress Bar */}
                                                <div className="h-3 bg-surface-container-highest rounded-full overflow-hidden mb-2">
                                                    <div
                                                        className={`h-full transition-all duration-500 ${t.state === 'Finished' ? 'bg-green-500' :
                                                            t.error ? 'bg-error' : 'bg-primary'
                                                            }`}
                                                        style={{ width: `${t.progress * 100}%` }}
                                                    />
                                                </div>

                                                {/* Stats */}
                                                <div className="flex items-center gap-6 text-sm text-on-surface-variant font-medium">
                                                    <span>
                                                        {Math.round(t.progress * 100)}%
                                                        <span className="mx-2 opacity-50">•</span>
                                                        {formatSize(t.downloaded_size)} of {formatSize(t.total_size)}
                                                    </span>

                                                    {t.state === 'Downloading' && (
                                                        <>
                                                            <span className="text-primary flex items-center gap-1">
                                                                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 14l-7 7m0 0l-7-7m7 7V3" />
                                                                </svg>
                                                                {formatSpeed(t.download_speed)}
                                                            </span>
                                                            <span className="flex items-center gap-1">
                                                                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283-.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0z" />
                                                                </svg>
                                                                {t.peers_connected} peers
                                                            </span>
                                                        </>
                                                    )}
                                                </div>
                                            </div>

                                            {/* Actions */}
                                            <div className="flex items-center gap-2 opacity-100 sm:opacity-0 group-hover:opacity-100 transition-opacity">
                                                {t.state === 'Downloading' ? (
                                                    <button
                                                        onClick={() => handlePause(t.id)}
                                                        className="w-12 h-12 flex items-center justify-center rounded-2xl bg-surface-container-highest text-on-surface hover:bg-primary hover:text-on-primary transition-all"
                                                        title="Pause"
                                                    >
                                                        <svg className="w-6 h-6" fill="currentColor" viewBox="0 0 24 24">
                                                            <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
                                                        </svg>
                                                    </button>
                                                ) : t.state === 'Paused' ? (
                                                    <button
                                                        onClick={() => handleResume(t.id)}
                                                        className="w-12 h-12 flex items-center justify-center rounded-2xl bg-surface-container-highest text-on-surface hover:bg-primary hover:text-on-primary transition-all"
                                                        title="Resume"
                                                    >
                                                        <svg className="w-6 h-6" fill="currentColor" viewBox="0 0 24 24">
                                                            <path d="M8 5v14l11-7z" />
                                                        </svg>
                                                    </button>
                                                ) : null}

                                                <button
                                                    onClick={() => requestDelete(t.id)}
                                                    className="w-12 h-12 flex items-center justify-center rounded-2xl bg-surface-container-highest text-on-surface hover:bg-error hover:text-on-error transition-all"
                                                    title="Delete"
                                                >
                                                    <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                                                    </svg>
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                ))
                            )}
                        </motion.div>
                    ) : (
                        <motion.div
                            key="browse"
                            initial={{ opacity: 0, x: 20 }}
                            animate={{ opacity: 1, x: 0 }}
                            exit={{ opacity: 0, x: 20 }}
                            className="h-full p-6 pt-2 overflow-hidden"
                        >
                            <TorrentBrowser onAdded={handleDownloadAdded} />
                        </motion.div>
                    )}
                </AnimatePresence>
            </div>
        </div>
    );
}
