import { motion } from 'framer-motion';
import { useMoodStore } from '../../store/moodStore';

interface AnalysisProgressProps {
    onCancel?: () => void;
}

export function AnalysisProgress({ onCancel }: AnalysisProgressProps) {
    const { isAnalyzing, analysisProgress, cancelAnalysis } = useMoodStore();

    if (!isAnalyzing) return null;
    if (!analysisProgress) {
        return (
            <div className="bg-surface-container-high rounded-2xl p-4 mb-6">
                <div className="flex items-center gap-3">
                    <div className="animate-spin w-5 h-5 border-2 border-primary border-t-transparent rounded-full" />
                    <p className="text-body-medium text-on-surface font-medium">Starting analysis...</p>
                </div>
            </div>
        );
    }

    const percentage = (analysisProgress.current / analysisProgress.total) * 100;

    const handleCancel = () => {
        cancelAnalysis();
        onCancel?.();
    };

    return (
        <div className="bg-surface-container-high rounded-2xl p-4">
            <div className="flex items-center justify-between mb-3">
                <div>
                    <h3 className="text-title-small font-semibold text-on-surface">Analyzing Library</h3>
                    <p className="text-body-small text-on-surface-variant">
                        {analysisProgress.current} / {analysisProgress.total} tracks
                    </p>
                </div>
                <button
                    onClick={handleCancel}
                    className="px-4 py-1.5 rounded-full text-body-small font-medium bg-error-container text-on-error-container hover:bg-error/20 transition-colors"
                >
                    Cancel
                </button>
            </div>

            {/* Progress Bar */}
            <div className="relative h-2 bg-surface-container-highest rounded-full overflow-hidden mb-2">
                <motion.div
                    initial={{ width: 0 }}
                    animate={{ width: `${percentage}%` }}
                    transition={{ duration: 0.3 }}
                    className="absolute inset-y-0 left-0 bg-primary rounded-full"
                />
            </div>

            {/* Current Track */}
            <p className="text-body-small text-on-surface-variant truncate">
                {analysisProgress.current_track.split('/').pop()}
            </p>

            {/* Stats */}
            <div className="flex gap-4 mt-2 text-label-small">
                <span className="text-primary">✓ {analysisProgress.success_count} success</span>
                {analysisProgress.error_count > 0 && (
                    <span className="text-error">✗ {analysisProgress.error_count} failed</span>
                )}
            </div>
        </div>
    );
}
