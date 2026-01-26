import { useState } from 'react';
import { usePlayerStore } from '../store/playerStore';
import { motion } from 'framer-motion';
import { WavySeparator } from './WavySeparator';

import {
    IconHome,
    IconAlbum,
    IconMicrophone,
    IconSettings,
    IconYoutube,
    IconDownload,
    IconHeart,
    IconStats,
} from './Icons';

import { MOOD_FEATURE_ENABLED } from '../types/mood';

// Mood Radio icon
function IconMood({ size = 24 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="10" />
            <path d="M8 14s1.5 2 4 2 4-2 4-2" />
            <line x1="9" y1="9" x2="9.01" y2="9" />
            <line x1="15" y1="9" x2="15.01" y2="9" />
        </svg>
    );
}

interface SidebarProps {
    view: 'tracks' | 'albums' | 'artists' | 'settings' | 'ytmusic' | 'favorites' | 'statistics' | 'torrents' | 'moodradio';
    onViewChange: (view: 'tracks' | 'albums' | 'artists' | 'settings' | 'ytmusic' | 'favorites' | 'statistics' | 'torrents' | 'moodradio') => void;
}

const sidebarSpring = {
    type: "spring",
    stiffness: 1400,
    damping: 90,
    mass: 1
} as const;

export function Sidebar({ view, onViewChange }: SidebarProps) {
    const { library } = usePlayerStore();
    const [isCollapsed, setIsCollapsed] = useState(false);

    return (
        <motion.aside
            initial={false}
            animate={{ width: isCollapsed ? 88 : 320 }}
            transition={sidebarSpring}
            className={`
                h-full flex flex-col
                bg-surface-container 
                z-20 border-r border-transparent rounded-[2rem] overflow-hidden shadow-sm
            `}
        >
            {/* Toggle Button - Minimal header */}
            <div data-tauri-drag-region className={`h-14 flex items-center px-4 shrink-0 ${isCollapsed ? 'justify-center' : 'justify-end'}`}>
                <button
                    onClick={() => setIsCollapsed(!isCollapsed)}
                    className="p-2.5 rounded-full hover:bg-surface-container-high text-on-surface-variant transition-colors"
                    title={isCollapsed ? "Expand" : "Collapse"}
                >
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z" />
                    </svg>
                </button>
            </div>

            {/* Navigation - Centered in collapsed mode */}
            <nav className={`flex-1 px-4 py-2 flex flex-col gap-1 overflow-y-auto scrollbar-thin scrollbar-thumb-outline/20 ${isCollapsed ? 'items-center' : ''}`}>

                {/* Library Section */}
                <div className={`flex flex-col gap-1 ${isCollapsed ? 'items-center w-full' : ''}`}>
                    <NavItem
                        icon={<IconHome size={28} />}
                        label="Songs"
                        active={view === 'tracks'}
                        onClick={() => onViewChange('tracks')}
                        count={library.length}
                        collapsed={isCollapsed}
                    />
                    <NavItem
                        icon={<IconAlbum size={28} />}
                        label="Albums"
                        active={view === 'albums'}
                        onClick={() => onViewChange('albums')}
                        collapsed={isCollapsed}
                    />
                    <NavItem
                        icon={<IconMicrophone size={28} />}
                        label="Artists"
                        active={view === 'artists'}
                        onClick={() => onViewChange('artists')}
                        collapsed={isCollapsed}
                    />
                    <NavItem
                        icon={<IconHeart size={24} filled={view === 'favorites'} />}
                        label="Favorites"
                        active={view === 'favorites'}
                        onClick={() => onViewChange('favorites')}
                        count={usePlayerStore.getState().favorites.size}
                        collapsed={isCollapsed}
                    />
                    {MOOD_FEATURE_ENABLED && (
                        <NavItem
                            icon={<IconMood size={24} />}
                            label="Mood Radio"
                            active={view === 'moodradio'}
                            onClick={() => onViewChange('moodradio')}
                            collapsed={isCollapsed}
                        />
                    )}
                </div>

                {/* Wavy Separator */}
                {!isCollapsed && (
                    <div className="px-2">
                        <WavySeparator label="" color="var(--md-sys-color-outline-variant)" />
                    </div>
                )}
                {isCollapsed && <div className="my-3 w-8 h-px bg-outline-variant/30" />}

                {/* Online Music Section */}
                <div className={`flex flex-col gap-1 ${isCollapsed ? 'items-center w-full' : ''}`}>
                    <NavItem
                        icon={<IconYoutube size={28} />}
                        label="YouTube Music"
                        active={view === 'ytmusic'}
                        onClick={() => onViewChange('ytmusic')}
                        collapsed={isCollapsed}
                    />
                </div>

                {/* Wavy Separator */}
                {!isCollapsed && (
                    <div className="px-2">
                        <WavySeparator label="" color="var(--md-sys-color-outline-variant)" />
                    </div>
                )}
                {isCollapsed && <div className="my-3 w-8 h-px bg-outline-variant/30" />}

                {/* Settings Section */}
                <div className={`flex flex-col gap-1 ${isCollapsed ? 'items-center w-full' : ''}`}>
                    <NavItem
                        icon={<IconDownload size={24} />}
                        label="Downloads"
                        active={view === 'torrents'}
                        onClick={() => onViewChange('torrents')}
                        collapsed={isCollapsed}
                    />
                    <NavItem
                        icon={<IconStats size={24} />}
                        label="Statistics"
                        active={view === 'statistics'}
                        onClick={() => onViewChange('statistics')}
                        collapsed={isCollapsed}
                    />



                    <NavItem
                        icon={<IconSettings size={28} />}
                        label="Settings"
                        active={view === 'settings'}
                        onClick={() => onViewChange('settings')}
                        collapsed={isCollapsed}
                    />
                </div>

            </nav>

            {/* Footer / Current Folder */}
            <div className={`p-4 shrink-0 flex flex-col gap-3 ${isCollapsed ? 'items-center' : ''}`}>
                {/* Current Folder Info */}

            </div>
        </motion.aside>
    );
}

function NavItem({
    icon,
    label,
    active = false,
    onClick,
    count,
    collapsed,
}: {
    icon: React.ReactNode;
    label: string;
    active?: boolean;
    onClick?: () => void;
    count?: number;
    collapsed: boolean;
}) {
    return (
        <button
            onClick={onClick}
            className={`
                group flex items-center relative
                transition-colors duration-200
                ${collapsed ? 'justify-center w-14 h-14 rounded-full' : 'w-full h-14 px-5 rounded-full gap-5'}
                ${active
                    ? 'bg-secondary-container text-on-secondary-container font-semibold'
                    : 'text-on-surface-variant hover:bg-surface-container-high hover:text-on-surface'
                }
            `}
            title={collapsed ? label : undefined}
        >
            {/* Active Indicator (Overlay) */}
            {active && (
                <motion.div
                    layoutId="activeNavParams"
                    className="absolute inset-0 rounded-full bg-on-secondary-container/5 pointer-events-none"
                    transition={{ type: "spring", bounce: 0.2, duration: 0.6 }}
                />
            )}

            <span className="z-10 relative">{icon}</span>

            {!collapsed && (
                <motion.span
                    initial={{ opacity: 0, x: -10 }}
                    animate={{ opacity: 1, x: 0 }}
                    className="text-title-medium whitespace-nowrap overflow-hidden text-ellipsis z-10 flex-1 text-left"
                >
                    {label}
                </motion.span>
            )}

            {!collapsed && count !== undefined && (
                <motion.span
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    className={`text-label-medium z-10 ${active ? 'text-on-secondary-container/70' : 'text-on-surface-variant/60'}`}
                >
                    {count}
                </motion.span>
            )}
        </button>
    );
}
