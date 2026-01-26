import { useState } from 'react';
import { motion } from 'framer-motion';
import { useMoodStore } from '../../store/moodStore';
import { usePlayerStore } from '../../store/playerStore';
import { MOOD_PRESETS, MoodPreset } from '../../types/mood';
import { AnalyzeLibrarySection } from './AnalyzeLibrarySection';

export function MoodRadio() {
    const [selectedPreset, setSelectedPreset] = useState<MoodPreset | null>(null);
    const [isLoading, setIsLoading] = useState(false);
    const [queuePreview, setQueuePreview] = useState<string[]>([]);

    const { getMoodQueue, getSimilarTracks, essentiaStatus, checkEssentiaStatus, isCheckingEssentia } = useMoodStore();
    const { library, playQueue, status } = usePlayerStore();

    const handlePresetClick = async (preset: MoodPreset) => {
        setSelectedPreset(preset);
        setIsLoading(true);
        try {
            const queue = await getMoodQueue(preset, 20);
            setQueuePreview(queue);
        } catch (error) {
            console.error('Failed to get mood queue:', error);
            setQueuePreview([]);
        } finally {
            setIsLoading(false);
        }
    };

    const handleSimilarTracks = async () => {
        if (!status.track?.path) return;
        setSelectedPreset(null);
        setIsLoading(true);
        try {
            const similar = await getSimilarTracks(status.track.path, 20);
            setQueuePreview(similar);
        } catch (error) {
            console.error('Failed to get similar tracks:', error);
            setQueuePreview([]);
        } finally {
            setIsLoading(false);
        }
    };

    const handlePlayQueue = () => {
        if (queuePreview.length === 0) return;
        // Convert paths to TrackDisplay objects
        const tracksToPlay = queuePreview
            .map(path => library.find(t => t.path === path))
            .filter((t): t is NonNullable<typeof t> => t !== undefined);

        if (tracksToPlay.length > 0) {
            playQueue(tracksToPlay, 0);
        }
    };

    // Check Essentia on mount if not checked
    if (essentiaStatus === null && !isCheckingEssentia) {
        checkEssentiaStatus();
    }

    return (
        <div className="flex-1 h-full overflow-y-auto">
            <div className="max-w-3xl mx-auto p-6 pb-8">
                {/* Header */}
                <div className="mb-8">
                    <h1 className="text-display-small font-bold text-on-surface mb-2">Mood Radio</h1>
                    <p className="text-body-large text-on-surface-variant">
                        Generate playlists based on mood and audio features
                    </p>
                </div>

                {/* Essentia Status Warning */}
                {essentiaStatus && !essentiaStatus.available && (
                    <motion.div
                        initial={{ opacity: 0, y: -10 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="bg-error-container text-on-error-container rounded-2xl p-4 mb-6"
                    >
                        <h3 className="font-semibold mb-1">Audio Analysis Unavailable</h3>
                        <p className="text-body-medium">{essentiaStatus.error}</p>
                        <p className="text-body-small mt-2">
                            Mood Radio requires audio feature analysis. Please install Essentia to use this feature.
                        </p>
                    </motion.div>
                )}

                {/* Analyze Library Section */}
                {essentiaStatus?.available && (
                    <AnalyzeLibrarySection library={library} />
                )}

                {/* Mood Presets Grid */}
                <div className="mb-8">
                    <h2 className="text-title-large font-semibold text-on-surface mb-4">Choose a Mood</h2>
                    <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
                        {MOOD_PRESETS.map(({ id, label, emoji, description }) => (
                            <motion.button
                                key={id}
                                whileHover={{ scale: 1.02 }}
                                whileTap={{ scale: 0.98 }}
                                onClick={() => handlePresetClick(id)}
                                className={`
                  p-5 rounded-2xl text-left transition-colors
                  ${selectedPreset === id
                                        ? 'bg-primary text-on-primary'
                                        : 'bg-surface-container-high hover:bg-surface-container-highest text-on-surface'
                                    }
                `}
                            >
                                <span className="text-2xl mb-2 block">{emoji}</span>
                                <span className="text-title-medium font-semibold block">{label}</span>
                                <span className={`text-body-small ${selectedPreset === id ? 'text-on-primary/80' : 'text-on-surface-variant'}`}>
                                    {description}
                                </span>
                            </motion.button>
                        ))}
                    </div>
                </div>

                {/* Similar to Current Track */}
                {status.track && (
                    <div className="mb-8">
                        <h2 className="text-title-large font-semibold text-on-surface mb-4">Or Find Similar</h2>
                        <motion.button
                            whileHover={{ scale: isLoading ? 1 : 1.01 }}
                            whileTap={{ scale: isLoading ? 1 : 0.99 }}
                            onClick={handleSimilarTracks}
                            disabled={isLoading}
                            className={`w-full p-4 rounded-2xl text-left flex items-center gap-4 transition-colors ${
                                isLoading
                                    ? 'bg-secondary-container/60 text-on-secondary-container/60'
                                    : 'bg-secondary-container text-on-secondary-container hover:bg-secondary-container/90'
                            }`}
                        >
                            <div className={`w-12 h-12 rounded-xl ${isLoading ? 'bg-secondary/60 flex items-center justify-center' : 'bg-secondary flex items-center justify-center'} text-2xl`}>
                                {isLoading ? (
                                    <div className="animate-spin w-6 h-6 border-2 border-on-secondary-container border-t-transparent rounded-full" />
                                ) : (
                                    '🎵'
                                )}
                            </div>
                            <div className="flex-1">
                                <p className="text-body-medium font-medium">
                                    {isLoading ? 'Analyzing and finding similar...' : 'Similar to current track'}
                                </p>
                                <p className={`text-body-small truncate ${isLoading ? 'text-on-secondary-container/50' : 'text-on-secondary-container/70'}`}>
                                    {status.track.title} – {status.track.artist}
                                </p>
                            </div>
                            {!isLoading && (
                                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                                    <path d="M9 18l6-6-6-6" />
                                </svg>
                            )}
                        </motion.button>
                    </div>
                )}

                {/* Queue Preview */}
                {(isLoading || queuePreview.length > 0) && (
                    <div className="mb-8">
                        <div className="flex items-center justify-between mb-4">
                            <h2 className="text-title-large font-semibold text-on-surface">
                                {isLoading ? 'Building Queue...' : `Queue Preview (${queuePreview.length} tracks)`}
                            </h2>
                            {queuePreview.length > 0 && (
                                <motion.button
                                    whileHover={{ scale: 1.05 }}
                                    whileTap={{ scale: 0.95 }}
                                    onClick={handlePlayQueue}
                                    className="px-6 py-2 rounded-full bg-primary text-on-primary font-semibold"
                                >
                                    Play All
                                </motion.button>
                            )}
                        </div>

                        {isLoading ? (
                            <div className="flex items-center justify-center py-12">
                                <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full" />
                            </div>
                        ) : queuePreview.length === 0 ? (
                            <p className="text-body-medium text-on-surface-variant text-center py-8">
                                No tracks match this mood. Try analyzing your library first.
                            </p>
                        ) : (
                            <div className="bg-surface-container rounded-2xl divide-y divide-outline-variant/20 max-h-96 overflow-y-auto">
                                {queuePreview.slice(0, 10).map((path, i) => {
                                    const track = library.find(t => t.path === path);
                                    return track ? (
                                        <div key={path} className="p-3 flex items-center gap-3">
                                            <span className="text-body-small text-on-surface-variant w-6 text-center">{i + 1}</span>
                                            <div className="flex-1 min-w-0">
                                                <p className="text-body-medium text-on-surface truncate">{track.title}</p>
                                                <p className="text-body-small text-on-surface-variant truncate">{track.artist}</p>
                                            </div>
                                        </div>
                                    ) : null;
                                })}
                                {queuePreview.length > 10 && (
                                    <p className="p-3 text-body-small text-on-surface-variant text-center">
                                        +{queuePreview.length - 10} more tracks
                                    </p>
                                )}
                            </div>
                        )}
                    </div>
                )}
            </div>
        </div>
    );
}
