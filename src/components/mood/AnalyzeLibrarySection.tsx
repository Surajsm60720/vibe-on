import { useEffect } from 'react';
import { motion } from 'framer-motion';
import { useMoodStore } from '../../store/moodStore';
import { TrackInfo } from '../../types';

interface AnalyzeLibrarySectionProps {
    library: TrackInfo[];
}

export function AnalyzeLibrarySection({ library }: AnalyzeLibrarySectionProps) {
    const {
        analysisProgress,
        analysisStats,
        isAnalyzing,
        analyzeLibrary,
        cancelAnalysis,
        getAnalysisStats
    } = useMoodStore();

    // Refresh stats on mount
    useEffect(() => {
        getAnalysisStats();
    }, [getAnalysisStats]);

    // Calculate percentage
    const successCount = analysisStats?.success || 0;
    const errorCount = analysisStats?.error || 0;
    const totalAnalyzed = successCount + errorCount;
    const totalTracks = library.length;

    const percentage = totalTracks > 0
        ? Math.round((totalAnalyzed / totalTracks) * 100)
        : 0;

    const handleAnalyze = () => {
        const paths = library.map(t => t.path);
        analyzeLibrary(paths);
    };

    return (
        <div className="bg-surface-container rounded-2xl p-6 mb-8">
            <div className="flex items-center justify-between mb-4">
                <div>
                    <h2 className="text-title-medium font-semibold text-on-surface">Library Analysis</h2>
                    <p className="text-body-medium text-on-surface-variant">
                        Analyze tracks to enable mood features
                    </p>
                </div>
                {isAnalyzing ? (
                    <motion.button
                        whileHover={{ scale: 1.05 }}
                        whileTap={{ scale: 0.95 }}
                        onClick={cancelAnalysis}
                        className="px-4 py-2 rounded-full bg-error-container text-on-error-container font-medium text-label-large"
                    >
                        Stop Analysis
                    </motion.button>
                ) : (
                    <motion.button
                        whileHover={{ scale: 1.05 }}
                        whileTap={{ scale: 0.95 }}
                        onClick={handleAnalyze}
                        disabled={totalTracks === 0}
                        className="px-6 py-2 rounded-full bg-primary text-on-primary font-semibold disabled:opacity-50"
                    >
                        {totalAnalyzed > 0 ? 'Update Analysis' : 'Analyze Library'}
                    </motion.button>
                )}
            </div>

            {/* Stats */}
            <div className="flex gap-6 mb-4 text-body-small text-on-surface-variant">
                <div className="flex items-center gap-2">
                    <div className="w-2 h-2 rounded-full bg-primary" />
                    <span>Analyzed: {successCount}</span>
                </div>
                {errorCount > 0 && (
                    <div className="flex items-center gap-2">
                        <div className="w-2 h-2 rounded-full bg-error" />
                        <span>Errors: {errorCount}</span>
                    </div>
                )}
                <div className="flex items-center gap-2">
                    <div className="w-2 h-2 rounded-full bg-surface-variant" />
                    <span>Total: {totalTracks}</span>
                </div>
            </div>

            {/* Progress Bar */}
            <div className="relative h-2 bg-surface-variant/30 rounded-full overflow-hidden">
                <motion.div
                    className="absolute top-0 left-0 h-full bg-primary"
                    initial={{ width: 0 }}
                    animate={{ width: `${percentage}%` }}
                    transition={{ duration: 0.5 }}
                />
            </div>

            {isAnalyzing && analysisProgress && (
                <div className="mt-2 flex justify-between text-body-small text-on-surface-variant">
                    <span className="truncate pr-4">Processing: {analysisProgress.current_track.split(/[/\\]/).pop()}</span>
                    <span>{percentage}%</span>
                </div>
            )}
        </div>
    );
}
