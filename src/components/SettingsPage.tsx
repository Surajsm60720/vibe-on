import { useSettingsStore } from '../store/settingsStore';
import { useThemeStore } from '../store/themeStore';
import { usePlayerStore } from '../store/playerStore';
import { open } from '@tauri-apps/plugin-dialog';
import { ask } from '@tauri-apps/plugin-dialog';
import packageJson from '../../package.json';

export function SettingsPage() {
    const {
        albumArtStyle, setAlbumArtStyle,
        expandedArtMode, setExpandedArtMode,
        autoplay, setAutoplay,
        downloadPath, setDownloadPath
    } = useSettingsStore();
    const { colors } = useThemeStore();
    const { primary } = colors;

    // Player store for folders
    const { folders, scanFolder, removeFolder, clearAllData } = usePlayerStore();
    const appVersion = packageJson.version;

    const handleOpenFolder = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: 'Select Music Folder',
            });

            if (selected && typeof selected === 'string') {
                await scanFolder(selected);
            }
        } catch (e) {
            console.error('Failed to open folder:', e);
        }
    };

    const handleClearAllData = async () => {
        console.log('[SettingsPage] Clear Data requested');
        const confirmed = await ask(
            'This will permanently delete all music library data, covers, recently played history, and folder settings. This action cannot be undone.',
            {
                title: 'Clear All Data?',
                kind: 'warning',
            }
        );

        if (confirmed) {
            console.log('[SettingsPage] User confirmed clear data');
            try {
                await clearAllData();
                console.log('[SettingsPage] Data cleared successfully');
                // Optionally show success message
                await ask('All data has been cleared successfully.', {
                    title: 'Success',
                    kind: 'info',
                });
            } catch (e) {
                console.error('Failed to clear data:', e);
                await ask('Failed to clear data. Please try again.', {
                    title: 'Error',
                    kind: 'error',
                });
            }
        } else {
            console.log('[SettingsPage] Clear data cancelled');
        }
    };

    return (
        <div className="h-full w-full overflow-y-auto p-8 pt-20">
            <h1 className="text-3xl font-bold text-white mb-8">Settings</h1>

            <div className="max-w-2xl flex flex-col gap-8">

                {/* Section: Minimal Player Appearance */}
                <section>
                    <h2 className="text-xl font-semibold text-white mb-4">Minimal Player</h2>
                    <div className="bg-white/5 rounded-xl p-6">
                        <div className="flex items-center justify-between mb-4">
                            <div>
                                <h3 className="text-base font-medium text-white">Album Art Style</h3>
                                <p className="text-sm text-white/50">Choose how the album art looks in the minimized player.</p>
                            </div>
                        </div>

                        <div className="flex gap-4">
                            <OptionButton
                                label="Vinyl Style"
                                active={albumArtStyle === 'vinyl'}
                                onClick={() => setAlbumArtStyle('vinyl')}
                                accentColor={primary}
                            >
                                <div className="w-8 h-8 rounded-full bg-white/20 relative overflow-hidden animate-[spin_4s_linear_infinite]">
                                    <div className="absolute inset-0 border-2 border-white/20 rounded-full"></div>
                                    <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-2 h-2 bg-black rounded-full"></div>
                                </div>
                            </OptionButton>

                            <OptionButton
                                label="Full Cover"
                                active={albumArtStyle === 'full'}
                                onClick={() => setAlbumArtStyle('full')}
                                accentColor={primary}
                            >
                                <div className="w-8 h-8 rounded bg-white/20"></div>
                            </OptionButton>
                        </div>
                    </div>
                </section>

                {/* Section: Expanded Player Appearance */}
                <section>
                    <h2 className="text-xl font-semibold text-white mb-4">Expanded Player</h2>
                    <div className="bg-white/5 rounded-xl p-6">
                        <div className="flex items-center justify-between mb-4">
                            <div>
                                <h3 className="text-base font-medium text-white">Layout Mode</h3>
                                <p className="text-sm text-white/50">Customize the layout when the player is expanded.</p>
                            </div>
                        </div>

                        <div className="flex gap-4">
                            <OptionButton
                                label="Background Art"
                                active={expandedArtMode === 'background'}
                                onClick={() => setExpandedArtMode('background')}
                                accentColor={primary}
                            >
                                {/* Mini diagram of pill with bg art */}
                                <div className="w-16 h-8 rounded-full border border-white/20 relative overflow-hidden flex items-center">
                                    <div className="absolute left-0 top-0 bottom-0 w-1/2 bg-white/20 rounded-l-full"></div>
                                    <div className="w-2 h-2 bg-white/40 ml-auto mr-2 rounded-full"></div>
                                </div>
                            </OptionButton>

                            <OptionButton
                                label="Pill Art (Left)"
                                active={expandedArtMode === 'pill'}
                                onClick={() => setExpandedArtMode('pill')}
                                accentColor={primary}
                            >
                                {/* Mini diagram of pill with art on left */}
                                <div className="w-16 h-8 rounded-full border border-white/20 flex items-center p-1 gap-1">
                                    <div className="w-6 h-6 rounded-full bg-white/20"></div>
                                    <div className="flex-1 h-1 bg-white/10 rounded-full"></div>
                                </div>
                            </OptionButton>
                        </div>
                    </div>
                </section>

                {/* Section: Local Files */}
                <section>
                    <h2 className="text-xl font-semibold text-white mb-4">Local Files</h2>
                    <div className="bg-white/5 rounded-xl p-6">
                        <div className="flex items-center justify-between mb-6">
                            <div>
                                <h3 className="text-base font-medium text-white">Music Folders</h3>
                                <p className="text-sm text-white/50">Manage the folders where Vibe searches for music.</p>
                            </div>
                            <div className="flex gap-2">
                                <button
                                    onClick={() => usePlayerStore.getState().refreshLibrary()}
                                    className="flex items-center gap-2 px-4 py-2 rounded-full bg-white/10 hover:bg-white/20 text-white transition-colors text-sm font-medium"
                                >
                                    <svg viewBox="0 0 24 24" fill="none" className={`w-4 h-4 ${usePlayerStore((s) => s.isLoading) ? 'animate-spin' : ''}`} stroke="currentColor" strokeWidth={2}>
                                        <path d="M23 4v6h-6" />
                                        <path d="M1 20v-6h6" />
                                        <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
                                    </svg>
                                    Refresh
                                </button>
                                <button
                                    onClick={handleOpenFolder}
                                    className="flex items-center gap-2 px-4 py-2 rounded-full bg-primary/20 hover:bg-primary/30 text-primary transition-colors text-sm font-medium"
                                    style={{ color: primary, backgroundColor: `${primary}33` }}
                                >
                                    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4" stroke="currentColor" strokeWidth={2}>
                                        <path d="M12 5v14m-7-7h14" strokeLinecap="round" strokeLinejoin="round" />
                                    </svg>
                                    Add Folder
                                </button>
                            </div>
                        </div>

                        <div className="flex flex-col gap-2">
                            {folders && folders.length > 0 ? (
                                folders.map((folder) => (
                                    <div key={folder} className="flex items-center justify-between p-3 rounded-lg bg-white/5 group">
                                        <div className="flex items-center gap-3 overflow-hidden">
                                            <svg viewBox="0 0 24 24" fill="currentColor" className="w-5 h-5 flex-shrink-0 text-white/50">
                                                <path d="M19.5 21a3 3 0 0 0 3-3v-4.5a3 3 0 0 0-3-3h-15a3 3 0 0 0-3 3V18a3 3 0 0 0 3 3h15ZM1.5 10.146V6a3 3 0 0 1 3-3h5.379a2.25 2.25 0 0 1 1.59.659l2.122 2.121c.14.141.331.22.53.22H19.5a3 3 0 0 1 3 3v1.146A4.483 4.483 0 0 0 19.5 9h-15a4.483 4.483 0 0 0-3 1.146Z" />
                                            </svg>
                                            <span className="text-sm text-white/80 truncate font-mono" title={folder}>
                                                {folder}
                                            </span>
                                        </div>
                                        <button
                                            onClick={() => removeFolder(folder)}
                                            className="p-2 rounded-full hover:bg-red-500/20 text-white/30 hover:text-red-400 transition-colors opacity-0 group-hover:opacity-100"
                                            title="Remove Folder"
                                        >
                                            <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4" stroke="currentColor" strokeWidth={2}>
                                                <path d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" strokeLinecap="round" strokeLinejoin="round" />
                                            </svg>
                                        </button>
                                    </div>
                                ))
                            ) : (
                                <div className="text-center py-8 text-white/30 text-sm">
                                    No folders added yet. Click "Add Folder" to start.
                                </div>
                            )}
                        </div>
                    </div>
                </section>

                {/* Section: Playback */}
                <section>
                    <h2 className="text-xl font-semibold text-white mb-4">Playback</h2>
                    <div className="bg-white/5 rounded-xl p-6">
                        <div className="flex items-center justify-between">
                            <div>
                                <h3 className="text-base font-medium text-white">Autoplay</h3>
                                <p className="text-sm text-white/50">Automatically play next track when current track ends.</p>
                            </div>
                            <ToggleSwitch
                                enabled={autoplay}
                                onChange={setAutoplay}
                                accentColor={primary}
                            />
                        </div>
                    </div>
                </section>

                {/* Section: Downloads */}
                <section>
                    <h2 className="text-xl font-semibold text-white mb-4">Downloads</h2>
                    <div className="bg-white/5 rounded-xl p-6">
                        <div className="flex items-center justify-between mb-4">
                            <div>
                                <h3 className="text-base font-medium text-white">Default Download Path</h3>
                                <p className="text-sm text-white/50">Where new torrents will be downloaded by default.</p>
                            </div>
                            <button
                                onClick={async () => {
                                    try {
                                        const selected = await open({
                                            directory: true,
                                            multiple: false,
                                            title: 'Select Download Folder',
                                        });
                                        if (selected && typeof selected === 'string') {
                                            // Ensure path ends with slash for consistency
                                            const path = selected.endsWith('/') || selected.endsWith('\\') ? selected : selected + '/';
                                            setDownloadPath(path);
                                        }
                                    } catch (e) {
                                        console.error('Failed to select folder:', e);
                                    }
                                }}
                                className="px-4 py-2 rounded-lg bg-surface-container-highest hover:bg-surface-container-high text-on-surface transition-colors text-sm font-medium"
                            >
                                Change
                            </button>
                        </div>
                        <div className="flex items-center gap-3 p-3 rounded-lg bg-black/20 font-mono text-sm text-white/80 break-all">
                            <svg className="w-5 h-5 text-white/40 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                            </svg>
                            {downloadPath || 'Default (Music Folder)'}
                        </div>
                        {downloadPath && (
                            <button
                                onClick={() => setDownloadPath(null)}
                                className="mt-2 text-xs text-primary hover:underline ml-1"
                            >
                                Reset to Default
                            </button>
                        )}
                    </div>
                </section>

                {/* Section: About */}
                <section>
                    <h2 className="text-xl font-semibold text-white mb-4">About</h2>
                    <div className="bg-white/5 rounded-xl p-6">
                        <div className="flex items-center justify-between">
                            <div>
                                <h3 className="text-base font-medium text-white">Version</h3>
                                <p className="text-sm text-white/50">Current installed version.</p>
                            </div>
                            <div className="text-white/80 font-mono bg-white/10 px-3 py-1 rounded">
                                v{appVersion}
                            </div>
                        </div>
                    </div>
                </section>

                {/* Section: Data Management */}
                <section>
                    <h2 className="text-xl font-semibold text-white mb-4">Data Management</h2>
                    <div className="bg-white/5 rounded-xl p-6">
                        <div className="flex items-center justify-between">
                            <div className="flex-1">
                                <h3 className="text-base font-medium text-white">Clear All Data</h3>
                                <p className="text-sm text-white/50">Remove all music library, covers, history, and settings.</p>
                            </div>
                            <button
                                onClick={handleClearAllData}
                                className="px-4 py-2 rounded-lg bg-red-500/20 hover:bg-red-500/30 text-red-400 hover:text-red-300 transition-colors text-sm font-medium"
                            >
                                Clear Data
                            </button>
                        </div>
                    </div>
                </section>
            </div>
        </div>
    );
}

function OptionButton({
    label,
    active,
    onClick,
    accentColor,
    children
}: {
    label: string,
    active: boolean,
    onClick: () => void,
    accentColor: string,
    children?: React.ReactNode
}) {
    return (
        <button
            onClick={onClick}
            className={`flex-1 flex flex-col items-center gap-3 p-4 rounded-lg border transition-all duration-200 ${active
                ? 'bg-white/10 border-transparent'
                : 'bg-transparent border-white/10 hover:bg-white/5'
                }`}
            style={active ? { borderColor: accentColor, boxShadow: `0 0 0 1px ${accentColor}` } : {}}
        >
            <div className={`opacity-80 ${active ? 'text-white' : 'text-white/50'}`}>
                {children}
            </div>
            <span className={`text-sm font-medium ${active ? 'text-white' : 'text-white/60'}`}>
                {label}
            </span>
        </button>
    );
}

function ToggleSwitch({
    enabled,
    onChange,
    accentColor
}: {
    enabled: boolean,
    onChange: (value: boolean) => void,
    accentColor: string
}) {
    return (
        <button
            onClick={() => onChange(!enabled)}
            className={`relative w-12 h-7 rounded-full transition-colors duration-200 ${enabled ? '' : 'bg-white/10'}`}
            style={enabled ? { backgroundColor: accentColor } : {}}
        >
            <div
                className={`absolute top-1 w-5 h-5 rounded-full bg-white shadow-md transition-transform duration-200 ${enabled ? 'translate-x-6' : 'translate-x-1'}`}
            />
        </button>
    );
}
