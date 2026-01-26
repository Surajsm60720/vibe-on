import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { motion, AnimatePresence } from 'framer-motion';

interface SearchResult {
    title: string;
    size: string;
    seeds: number;
    leechers: number;
    magnet: string;
    date: string;
    category: string;
    url: string;
}

interface Props {
    onSelectMagnet: (magnet: string) => void;
}

export function TorrentSearch({ onSelectMagnet }: Props) {
    const [query, setQuery] = useState('');
    const [results, setResults] = useState<SearchResult[]>([]);
    const [isSearching, setIsSearching] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const [sortBy, setSortBy] = useState<'seeders' | 'size' | 'id' | 'downloads'>('seeders');
    const [sortOrder, setSortOrder] = useState<'desc' | 'asc'>('desc');

    const handleSearch = async (e?: React.FormEvent) => {
        if (e) e.preventDefault();
        if (!query.trim()) return;

        setIsSearching(true);
        setError(null);
        setResults([]);

        try {
            const data = await invoke<SearchResult[]>('search_torrents', {
                query,
                sortBy,
                sortOrder
            });
            setResults(data);
        } catch (err) {
            console.error(err);
            setError(String(err));
        } finally {
            setIsSearching(false);
        }
    };

    return (
        <div className="flex flex-col h-full overflow-hidden">
            {/* Search Bar */}
            <div className="flex flex-col gap-3 mb-4 shrink-0">
                <form onSubmit={handleSearch} className="flex gap-2 p-1">
                    <input
                        type="text"
                        value={query}
                        onChange={(e) => setQuery(e.target.value)}
                        placeholder="Search Nyaa..."
                        className="flex-1 px-4 py-3 rounded-xl bg-surface-container-high text-on-surface border-none focus:ring-2 focus:ring-primary outline-none"
                        autoFocus
                    />
                    <button
                        type="submit"
                        disabled={isSearching || !query.trim()}
                        className="px-6 py-3 bg-primary text-on-primary font-bold rounded-xl hover:opacity-90 transition-opacity disabled:opacity-50"
                    >
                        {isSearching ? '...' : 'Search'}
                    </button>
                </form>

                {/* Sort Controls */}
                <div className="flex items-center gap-3 overflow-x-auto pb-4 pt-1 px-1 scrollbar-none">
                    <span className="text-sm font-medium text-on-surface-variant shrink-0 mr-1">Sort by:</span>
                    {(['seeders', 'date', 'size', 'downloads'] as const).map((key) => {
                        // Map UI label to value
                        const val = key === 'date' ? 'id' : key;
                        const label = key.charAt(0).toUpperCase() + key.slice(1);
                        const isActive = sortBy === val;

                        return (
                            <button
                                key={key}
                                onClick={() => {
                                    if (isActive) {
                                        setSortOrder(prev => prev === 'desc' ? 'asc' : 'desc');
                                    } else {
                                        setSortBy(val);
                                        setSortOrder('desc');
                                    }
                                    // Trigger search immediately if query exists
                                    if (query.trim()) setTimeout(() => handleSearch(), 0);
                                }}
                                type="button"
                                className={`
                                    px-4 py-2.5 rounded-xl text-sm font-bold transition-all flex items-center gap-2 shadow-sm
                                    ${isActive
                                        ? 'bg-secondary text-on-secondary shadow-md ring-2 ring-transparent'
                                        : 'bg-surface-container-high text-on-surface-variant hover:bg-surface-container-highest hover:text-on-surface'
                                    }
                                `}
                            >
                                {label}
                                {isActive && (
                                    <svg
                                        className={`w-4 h-4 transition-transform ${sortOrder === 'asc' ? 'rotate-180' : ''}`}
                                        fill="none" viewBox="0 0 24 24" stroke="currentColor"
                                    >
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M19 9l-7 7-7-7" />
                                    </svg>
                                )}
                            </button>
                        );
                    })}
                </div>
            </div>

            {/* Error Message */}
            {error && (
                <div className="mb-4 p-3 bg-error/10 text-error rounded-xl text-sm border border-error/10">
                    {error}
                </div>
            )}

            {/* Results List */}
            <div className="flex-1 overflow-y-auto pr-2 space-y-2 scrollbar-thin scrollbar-thumb-outline/20">
                {results.length === 0 && !isSearching && !error && (
                    <div className="flex flex-col items-center justify-center h-48 opacity-50 text-on-surface-variant">
                        <svg className="w-12 h-12 mb-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                        </svg>
                        <p>Search for anime or music</p>
                    </div>
                )}

                <AnimatePresence>
                    {results.map((result, index) => (
                        <motion.div
                            key={index}
                            initial={{ opacity: 0, y: 10 }}
                            animate={{ opacity: 1, y: 0 }}
                            transition={{ delay: index * 0.05 }}
                            className="p-4 bg-surface-container-high rounded-xl hover:bg-surface-container-highest transition-colors group flex flex-col gap-2"
                        >
                            <div className="flex justify-between items-start gap-4">
                                <div className="flex-1 min-w-0">
                                    <div className="flex items-center gap-2 mb-1">
                                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold uppercase tracking-wide">
                                            {result.category}
                                        </span>
                                    </div>
                                    <h3 className="font-medium text-on-surface text-sm leading-snug break-words" title={result.title}>
                                        {result.title}
                                    </h3>
                                </div>
                                <button
                                    onClick={() => onSelectMagnet(result.magnet)}
                                    className="shrink-0 px-4 py-2 bg-primary/10 text-primary text-sm font-bold rounded-xl hover:bg-primary hover:text-on-primary transition-colors"
                                >
                                    Download
                                </button>
                            </div>

                            <div className="flex items-center gap-4 text-xs text-on-surface-variant">
                                <span className="flex items-center gap-1">
                                    <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
                                    </svg>
                                    {result.size}
                                </span>
                                <span className="flex items-center gap-1 text-green-500">
                                    <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 10l7-7m0 0l7 7m-7-7v18" />
                                    </svg>
                                    {result.seeds}
                                </span>
                                <span className="flex items-center gap-1 text-red-400">
                                    <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 14l-7 7m0 0l-7-7m7 7V3" />
                                    </svg>
                                    {result.leechers}
                                </span>
                                <span className="opacity-60 ml-auto">{result.date}</span>
                            </div>
                        </motion.div>
                    ))}
                </AnimatePresence>
            </div>
        </div>
    );
}
