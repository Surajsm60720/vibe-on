import { motion, AnimatePresence } from 'framer-motion';
import { AudioFeatures, KEY_LABELS } from '../../types/mood';

interface TrackStatsProps {
    features: AudioFeatures;
    trackTitle: string;
    trackArtist: string;
    onClose: () => void;
}

export function TrackStats({ features, trackTitle, trackArtist, onClose }: TrackStatsProps) {
    const stats = [
        { label: 'Valence', value: features.valence, description: 'Sad ← → Happy' },
        { label: 'Energy', value: features.energy, description: 'Low ← → High' },
        { label: 'Danceability', value: features.danceability, description: 'How suitable for dancing' },
        { label: 'Instrumentalness', value: features.instrumentalness, description: 'Vocal ← → Instrumental' },
        { label: 'Acousticness', value: features.acousticness, description: 'Electronic ← → Acoustic' },
        { label: 'Speechiness', value: features.speechiness, description: 'Music ← → Speech' },
        { label: 'Liveness', value: features.liveness, description: 'Studio ← → Live' },
    ];

    const keyLabel = features.key >= 0 && features.key < 12 ? KEY_LABELS[features.key] : 'Unknown';

    return (
        <AnimatePresence>
            <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
                onClick={onClose}
            >
                <motion.div
                    initial={{ scale: 0.9, opacity: 0 }}
                    animate={{ scale: 1, opacity: 1 }}
                    exit={{ scale: 0.9, opacity: 0 }}
                    transition={{ type: 'spring', damping: 25, stiffness: 300 }}
                    className="bg-surface-container rounded-3xl p-6 max-w-md w-full mx-4 shadow-2xl"
                    onClick={(e) => e.stopPropagation()}
                >
                    {/* Header */}
                    <div className="flex items-center justify-between mb-6">
                        <div>
                            <h2 className="text-headline-small font-bold text-on-surface">Song Stats</h2>
                            <p className="text-body-medium text-on-surface-variant truncate max-w-[250px]">
                                {trackTitle} – {trackArtist}
                            </p>
                        </div>
                        <button
                            onClick={onClose}
                            className="p-2 rounded-full hover:bg-surface-container-high transition-colors"
                        >
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                                <path d="M18 6L6 18M6 6l12 12" />
                            </svg>
                        </button>
                    </div>

                    {/* Key/Tempo Info */}
                    <div className="flex gap-4 mb-6">
                        <div className="flex-1 bg-surface-container-high rounded-2xl p-4 text-center">
                            <p className="text-label-small text-on-surface-variant uppercase tracking-wider mb-1">Key</p>
                            <p className="text-title-large font-bold text-primary">{keyLabel}</p>
                        </div>
                        <div className="flex-1 bg-surface-container-high rounded-2xl p-4 text-center">
                            <p className="text-label-small text-on-surface-variant uppercase tracking-wider mb-1">Tempo</p>
                            <p className="text-title-large font-bold text-primary">{Math.round(features.tempo)} BPM</p>
                        </div>
                        <div className="flex-1 bg-surface-container-high rounded-2xl p-4 text-center">
                            <p className="text-label-small text-on-surface-variant uppercase tracking-wider mb-1">Loudness</p>
                            <p className="text-title-large font-bold text-primary">{features.loudness.toFixed(1)} dB</p>
                        </div>
                    </div>

                    {/* Feature Bars */}
                    <div className="space-y-4">
                        {stats.map(({ label, value, description }) => (
                            <div key={label} className="group">
                                <div className="flex justify-between items-center mb-1">
                                    <span className="text-body-medium text-on-surface font-medium">{label}</span>
                                    <span className="text-label-medium text-on-surface-variant">
                                        {Math.round(value * 100)}%
                                    </span>
                                </div>
                                <div className="relative h-3 bg-surface-container-highest rounded-full overflow-hidden">
                                    <motion.div
                                        initial={{ width: 0 }}
                                        animate={{ width: `${value * 100}%` }}
                                        transition={{ duration: 0.5, ease: 'easeOut' }}
                                        className="absolute inset-y-0 left-0 bg-gradient-to-r from-primary to-secondary rounded-full"
                                    />
                                </div>
                                <p className="text-label-small text-on-surface-variant mt-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
                                    {description}
                                </p>
                            </div>
                        ))}
                    </div>

                    {/* Analysis Info */}
                    {features.analyzed_at && (
                        <p className="text-label-small text-on-surface-variant text-center mt-6">
                            Analyzed: {new Date(features.analyzed_at).toLocaleDateString()}
                        </p>
                    )}
                </motion.div>
            </motion.div>
        </AnimatePresence>
    );
}
