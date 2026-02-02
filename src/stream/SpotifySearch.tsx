import React, { useState } from 'react';
import { useSpotifyStore } from './spotifyStore';
import {
    IconSearch,
    IconMusicNote,
    IconChartBar,
    IconBolt,
    IconMoodSmile,
    IconActivity,
    IconLock
} from '../components/Icons';
import { motion, AnimatePresence } from 'motion/react';

export const SpotifySearch: React.FC = () => {
    const [query, setQuery] = useState('');
    const [clientId, setClientId] = useState('');
    const [clientSecret, setClientSecret] = useState('');
    const [isAuthOpen, setIsAuthOpen] = useState(true);

    const {
        searchResults,
        isSearching,
        selectedTrack,
        selectedTrackFeatures,
        search,
        selectTrack,
        authenticate,
        error
    } = useSpotifyStore();

    const handleSearch = (e: React.FormEvent) => {
        e.preventDefault();
        search(query);
    };

    const handleAuth = async (e: React.FormEvent) => {
        e.preventDefault();
        const success = await authenticate(clientId, clientSecret);
        if (success) setIsAuthOpen(false);
    };

    return (
        <div className="flex h-full gap-6 p-6 overflow-hidden">
            {/* Search and Results Section */}
            <div className="flex flex-col flex-1 gap-6 overflow-hidden">
                {/* Search Bar */}
                <form onSubmit={handleSearch} className="relative group">
                    <input
                        type="text"
                        value={query}
                        onChange={(e) => setQuery(e.target.value)}
                        placeholder="Search Spotify Tracks..."
                        className="w-full h-14 pl-14 pr-6 bg-surface-container-high border-2 border-transparent focus:border-primary rounded-2xl text-title-medium transition-all outline-none"
                    />
                    <IconSearch className="absolute left-5 top-1/2 -translate-y-1/2 text-on-surface-variant group-focus-within:text-primary transition-colors" size={24} />
                </form>

                {/* Results List */}
                <div className="flex-1 overflow-y-auto pr-2 custom-scrollbar">
                    {isSearching ? (
                        <div className="flex flex-col items-center justify-center h-full gap-4 text-on-surface-variant">
                            <div className="w-10 h-10 border-4 border-primary/20 border-t-primary rounded-full animate-spin" />
                            <p>Searching Spotify...</p>
                        </div>
                    ) : error ? (
                        <div className="flex flex-col items-center justify-center h-full gap-4 text-error">
                            <IconLock size={48} />
                            <p>{error}</p>
                            <button
                                onClick={() => setIsAuthOpen(true)}
                                className="px-6 py-2 bg-primary text-on-primary rounded-full hover:bg-primary/90 transition-colors"
                            >
                                Configure Credentials
                            </button>
                        </div>
                    ) : searchResults.length > 0 ? (
                        <div className="grid grid-cols-1 gap-2">
                            {searchResults.map((track) => (
                                <motion.div
                                    key={track.id}
                                    layoutId={track.id}
                                    onClick={() => selectTrack(track)}
                                    initial={{ opacity: 0, y: 10 }}
                                    animate={{ opacity: 1, y: 0 }}
                                    className={`flex items-center gap-4 p-3 rounded-xl cursor-pointer transition-all ${selectedTrack?.id === track.id
                                            ? 'bg-primary/10 border-l-4 border-primary'
                                            : 'hover:bg-surface-container-highest border-l-4 border-transparent'
                                        }`}
                                >
                                    <img
                                        src={track.album.images[0]?.url}
                                        alt={track.name}
                                        className="w-14 h-14 rounded-lg shadow-md object-cover"
                                    />
                                    <div className="flex flex-col min-w-0 flex-1">
                                        <span className="text-title-medium font-semibold truncate">{track.name}</span>
                                        <span className="text-body-medium text-on-surface-variant truncate">
                                            {track.artists.map(a => a.name).join(', ')} • {track.album.name}
                                        </span>
                                    </div>
                                </motion.div>
                            ))}
                        </div>
                    ) : (
                        <div className="flex flex-col items-center justify-center h-full text-on-surface-variant opacity-50">
                            <IconMusicNote size={64} />
                            <p className="mt-4 text-body-large">Search for music on Spotify</p>
                        </div>
                    )}
                </div>
            </div>

            {/* Analytics Sidebar */}
            <AnimatePresence>
                {selectedTrack && (
                    <motion.div
                        initial={{ opacity: 0, x: 20 }}
                        animate={{ opacity: 1, x: 0 }}
                        exit={{ opacity: 0, x: 20 }}
                        className="w-80 bg-surface-container rounded-3xl p-6 flex flex-col gap-6 shadow-xl border border-outline-variant/30"
                    >
                        <div className="flex flex-col items-center text-center gap-4">
                            <img
                                src={selectedTrack.album.images[0]?.url}
                                className="w-48 h-48 rounded-2xl shadow-2xl"
                                alt="Album Cover"
                            />
                            <div className="flex flex-col gap-1">
                                <h3 className="text-headline-small font-bold">{selectedTrack.name}</h3>
                                <p className="text-on-surface-variant">{selectedTrack.artists[0]?.name}</p>
                            </div>
                        </div>

                        <div className="flex-1 overflow-y-auto custom-scrollbar">
                            <h4 className="text-label-large text-primary uppercase tracking-wider mb-4 flex items-center gap-2">
                                <IconChartBar size={18} />
                                Audio Features
                            </h4>

                            {selectedTrackFeatures ? (
                                <div className="grid grid-cols-1 gap-4">
                                    <FeatureItem
                                        icon={<IconBolt size={20} className="text-orange-400" />}
                                        label="Energy"
                                        value={selectedTrackFeatures.energy}
                                        color="bg-orange-400"
                                    />
                                    <FeatureItem
                                        icon={<IconActivity size={20} className="text-blue-400" />}
                                        label="Danceability"
                                        value={selectedTrackFeatures.danceability}
                                        color="bg-blue-400"
                                    />
                                    <FeatureItem
                                        icon={<IconMoodSmile size={20} className="text-green-400" />}
                                        label="Valence (Mood)"
                                        value={selectedTrackFeatures.valence}
                                        color="bg-green-400"
                                    />
                                    <div className="p-3 bg-surface-container-high rounded-xl flex justify-between items-center">
                                        <span className="text-body-medium font-medium">Tempo</span>
                                        <span className="text-title-medium font-bold text-primary">{Math.round(selectedTrackFeatures.tempo)} BPM</span>
                                    </div>
                                </div>
                            ) : (
                                <div className="animate-pulse space-y-4">
                                    <div className="h-12 bg-surface-container-highest rounded-xl" />
                                    <div className="h-12 bg-surface-container-highest rounded-xl" />
                                    <div className="h-12 bg-surface-container-highest rounded-xl" />
                                </div>
                            )}
                        </div>

                        <a
                            href={selectedTrack.external_urls.spotify}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="flex items-center justify-center gap-2 py-3 bg-[#1DB954] text-black font-bold rounded-full hover:scale-105 transition-transform"
                        >
                            <IconMusicNote size={20} />
                            Open in Spotify
                        </a>
                    </motion.div>
                )}
            </AnimatePresence>

            {/* Auth Modal Overlay */}
            <AnimatePresence>
                {isAuthOpen && (
                    <motion.div
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0 }}
                        className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
                    >
                        <motion.div
                            initial={{ scale: 0.9, y: 20 }}
                            animate={{ scale: 1, y: 0 }}
                            className="bg-surface-container-high p-8 rounded-[32px] w-[450px] shadow-2xl border border-outline-variant"
                        >
                            <h2 className="text-headline-medium font-bold mb-2">Spotify API Credentials</h2>
                            <p className="text-on-surface-variant mb-6 text-body-medium">
                                Enter your Spotify Developer Application credentials to enable native search and audio analysis.
                            </p>

                            <form onSubmit={handleAuth} className="flex flex-col gap-4">
                                <div className="flex flex-col gap-2">
                                    <label className="text-label-large font-semibold ml-2">Client ID</label>
                                    <input
                                        type="text"
                                        value={clientId}
                                        onChange={(e) => setClientId(e.target.value)}
                                        className="h-12 px-4 bg-surface rounded-xl border border-outline transition-colors focus:border-primary outline-none"
                                        placeholder="Enter your Client ID"
                                        required
                                    />
                                </div>
                                <div className="flex flex-col gap-2">
                                    <label className="text-label-large font-semibold ml-2">Client Secret</label>
                                    <input
                                        type="password"
                                        value={clientSecret}
                                        onChange={(e) => setClientSecret(e.target.value)}
                                        className="h-12 px-4 bg-surface rounded-xl border border-outline transition-colors focus:border-primary outline-none"
                                        placeholder="Enter your Client Secret"
                                        required
                                    />
                                </div>
                                <div className="flex gap-3 mt-4">
                                    <button
                                        type="button"
                                        onClick={() => setIsAuthOpen(false)}
                                        className="flex-1 py-3 text-primary font-bold hover:bg-primary/10 rounded-2xl transition-colors"
                                    >
                                        Cancel
                                    </button>
                                    <button
                                        type="submit"
                                        className="flex-[2] py-3 bg-primary text-on-primary font-bold rounded-2xl hover:bg-primary/90 transition-all shadow-lg"
                                    >
                                        Authenticate
                                    </button>
                                </div>
                            </form>
                        </motion.div>
                    </motion.div>
                )}
            </AnimatePresence>
        </div>
    );
};

const FeatureItem: React.FC<{ icon: React.ReactNode, label: string, value: number, color: string }> = ({
    icon, label, value, color
}) => {
    return (
        <div className="p-3 bg-surface-container-high rounded-2xl flex flex-col gap-2">
            <div className="flex items-center gap-2">
                {icon}
                <span className="text-label-medium font-bold opacity-70">{label}</span>
                <span className="ml-auto text-title-small font-bold">{Math.round(value * 100)}%</span>
            </div>
            <div className="h-1.5 w-full bg-surface rounded-full overflow-hidden">
                <motion.div
                    initial={{ width: 0 }}
                    animate={{ width: `${value * 100}%` }}
                    transition={{ duration: 1, ease: 'easeOut' }}
                    className={`h-full ${color}`}
                />
            </div>
        </div>
    );
};
