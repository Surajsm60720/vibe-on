import { motion } from 'framer-motion';
import { useState } from 'react';
import { useMoodStore } from '../../store/moodStore';
import { AnalysisProgress } from './AnalysisProgress';
import type { TrackDisplay } from '../../types';

interface AnalyzeLibrarySectionProps {
    library: TrackDisplay[];
}

export function AnalyzeLibrarySection({ library }: AnalyzeLibrarySectionProps) {
    const { startLibraryAnalysis, isAnalyzing, clearAnalysisData } = useMoodStore();
    const [showClearConfirm, setShowClearConfirm] = useState(false);

    const handleAnalyzeLibrary = async () => {
        if (library.length === 0) return;
        const trackPaths = library.map(t => t.path);
        await startLibraryAnalysis(trackPaths);
    };

    const handleClearData = async () => {
        await clearAnalysisData();
        setShowClearConfirm(false);
    };

    return (
        <div className="mb-8">
            {/* Analysis Progress Widget */}
            {isAnalyzing && (
                <>
                    <AnalysisProgress />
                    <div className="text-center text-body-medium text-on-surface-variant mt-4">
                        Analyzing library... This may take a few minutes.
                    </div>
                </>
            )}

            {/* Analyze Button */}
            {!isAnalyzing && (
                <motion.div
                    initial={{ opacity: 0, y: -10 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="bg-primary-container text-on-primary-container rounded-2xl p-4 mb-6"
                >
                    <div className="flex items-center justify-between gap-4">
                        <div className="flex-1">
                            <h3 className="font-semibold mb-1">Analyze Your Library</h3>
                            <p className="text-body-small text-on-primary-container/80">
                                Extract audio features from {library.length} tracks to enable mood-based discovery
                            </p>
                        </div>
                        <motion.button
                            whileHover={{ scale: 1.05 }}
                            whileTap={{ scale: 0.95 }}
                            onClick={handleAnalyzeLibrary}
                            className="px-6 py-2 rounded-full bg-primary text-on-primary font-semibold whitespace-nowrap text-body-medium"
                        >
                            Analyze
                        </motion.button>
                    </div>
                </motion.div>
            )}

            {/* Clear Data Button */}
            {!isAnalyzing && (
                <motion.button
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    whileHover={{ scale: 1.02 }}
                    whileTap={{ scale: 0.98 }}
                    onClick={() => setShowClearConfirm(true)}
                    className="text-label-small text-error hover:text-error/80 transition-colors underline"
                >
                    Clear Analysis Data
                </motion.button>
            )}

            {/* Clear Confirmation Modal */}
            {showClearConfirm && (
                <motion.div
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    className="fixed inset-0 z-40 flex items-center justify-center bg-black/40 backdrop-blur-sm"
                    onClick={() => setShowClearConfirm(false)}
                >
                    <motion.div
                        initial={{ scale: 0.9 }}
                        animate={{ scale: 1 }}
                        className="bg-surface-container rounded-2xl p-6 max-w-sm mx-4"
                        onClick={(e) => e.stopPropagation()}
                    >
                        <h3 className="text-headline-small font-bold text-on-surface mb-2">Clear Analysis Data?</h3>
                        <p className="text-body-medium text-on-surface-variant mb-6">
                            This will delete all audio feature analysis. You can re-analyze anytime.
                        </p>
                        <div className="flex gap-3 justify-end">
                            <button
                                onClick={() => setShowClearConfirm(false)}
                                className="px-4 py-2 rounded-full text-body-medium font-semibold text-on-surface hover:bg-surface-container-high"
                            >
                                Cancel
                            </button>
                            <button
                                onClick={handleClearData}
                                className="px-4 py-2 rounded-full text-body-medium font-semibold text-on-error bg-error-container hover:bg-error/20"
                            >
                                Clear
                            </button>
                        </div>
                    </motion.div>
                </motion.div>
            )}
        </div>
    );
}
