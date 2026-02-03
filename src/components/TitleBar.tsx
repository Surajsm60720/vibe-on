import { getCurrentWindow } from '@tauri-apps/api/window';
import { useState, useEffect, useRef } from 'react';
import { SearchBar } from './SearchBar';
import { MobilePairingPopup } from './MobilePairingPopup';
import { useMobileStore } from '../store/mobileStore';
import { IconMobileDevice } from './Icons';

export function TitleBar() {
    const appWindow = getCurrentWindow();
    const [isMaximized, setIsMaximized] = useState(false);
    const [closeHovered, setCloseHovered] = useState(false);
    const [isMacOS, setIsMacOS] = useState(false);
    
    // Mobile pairing state
    const { status, togglePopup, checkServerStatus } = useMobileStore();
    const mobileButtonRef = useRef<HTMLButtonElement>(null);
    
    // Check server status on mount
    useEffect(() => {
        checkServerStatus();
    }, [checkServerStatus]);

    // Detect OS on mount
    useEffect(() => {
        const platform = navigator.platform.toLowerCase();
        const userAgent = navigator.userAgent.toLowerCase();
        setIsMacOS(platform.includes('mac') || userAgent.includes('mac'));
    }, []);
    
    // Get mobile icon style based on status
    const getMobileIconStyle = () => {
        switch (status) {
            case 'connected':
                return {
                    color: 'var(--md-sys-color-primary)',
                    filter: 'drop-shadow(0 0 4px var(--md-sys-color-primary))',
                };
            case 'connecting':
            case 'searching':
                return {
                    color: 'var(--md-sys-color-tertiary)',
                };
            default:
                return {
                    color: 'var(--md-sys-color-outline)',
                };
        }
    };
    
    const getMobileIconClass = () => {
        if (status === 'searching' || status === 'connecting') {
            return 'animate-pulse';
        }
        return '';
    };
    
    // Mobile pairing button component
    const mobileButton = (
        <button
            ref={mobileButtonRef}
            onClick={togglePopup}
            className={`w-5 h-5 flex items-center justify-center opacity-80 hover:opacity-100 transition-all duration-150 ${getMobileIconClass()}`}
            title="Mobile Companion"
            style={getMobileIconStyle()}
        >
            <IconMobileDevice size={16} />
        </button>
    );

    const nameSection = (
        <div data-tauri-drag-region className="flex items-center gap-3">
            <div data-tauri-drag-region className="text-label-large font-bold tracking-wider opacity-90 pointer-events-none flex items-center gap-2">
                <span className="text-primary text-xl">♪</span> VIBE-ON!
            </div>
        </div>
    );

    return (
        <div
            data-tauri-drag-region
            className="h-10 flex items-center justify-between px-4 bg-surface text-on-surface select-none z-50 fixed top-0 right-0 left-0 border-b border-white/5 transition-colors duration-300"
        >
            {/* macOS: Controls on Left, then App Name | Windows: App Name on Left */}
            {isMacOS ? (
                <>
                    <div className="flex items-center gap-2 pl-2">
                        {/* macOS Order: Close, Minimize, Maximize */}
                        <button
                            onClick={() => appWindow.close()}
                            onMouseEnter={() => setCloseHovered(true)}
                            onMouseLeave={() => setCloseHovered(false)}
                            className="w-4 h-4 group flex items-center justify-center opacity-80 hover:opacity-100 transition-all duration-150"
                            title="Close"
                            style={{ filter: closeHovered ? 'drop-shadow(0 0 6px #ef4444)' : 'none' }}
                        >
                            <svg viewBox="0 0 280 280" className="w-full h-full transition-colors" style={{ color: closeHovered ? '#ef4444' : 'var(--md-sys-color-secondary)' }}>
                                <path d="M178.73 6.2068C238.87 -19.9132 299.91 41.1269 273.79 101.267L269.47 111.207C261.5 129.577 261.5 150.417 269.47 168.787L273.79 178.727C299.91 238.867 238.87 299.907 178.73 273.787L168.79 269.467C150.42 261.497 129.58 261.497 111.21 269.467L101.27 273.787C41.1281 299.907 -19.9139 238.867 6.20706 178.727L10.5261 168.787C18.5011 150.417 18.5011 129.577 10.5261 111.207L6.20706 101.267C-19.9139 41.1269 41.1281 -19.9132 101.27 6.2068L111.21 10.5269C129.58 18.4969 150.42 18.4969 168.79 10.5269L178.73 6.2068Z" fill="currentColor" />
                            </svg>
                        </button>
                        <button
                            onClick={() => appWindow.minimize()}
                            className="w-4 h-4 group flex items-center justify-center opacity-80 hover:opacity-100 transition-all duration-150"
                            title="Minimize"
                        >
                            <svg viewBox="0 0 316 278" className="w-full h-full transition-colors" style={{ color: 'var(--md-sys-color-tertiary)' }}>
                                <path d="M271.57 155.799C257.552 177.379 243.535 198.989 229.517 220.569C220.423 234.579 211.167 248.778 198.872 259.908C186.576 271.058 170.648 278.938 154.316 277.908C139.976 276.988 126.684 269.278 116.191 259.208C105.698 249.138 97.5464 236.738 89.5284 224.458C67.8424 191.278 46.1303 158.098 24.4443 124.898C14.1393 109.138 3.56535 92.6884 0.713353 73.9084C-2.73065 51.2184 6.55235 28.1085 23.0183 13.0185C40.2373 -2.76147 68.1384 -1.48141 89.0984 2.83859C112.075 7.58859 134.541 16.5185 157.975 16.4885C178.047 16.4885 197.446 9.88858 216.979 5.08859C236.485 0.318604 257.445 -2.62152 276.279 4.47849C299.659 13.2685 316.448 38.2684 315.991 63.9284C315.56 87.3384 302.457 108.248 289.839 127.728C283.758 137.078 277.678 146.449 271.597 155.799H271.57Z" fill="currentColor" />
                            </svg>
                        </button>
                        <button
                            onClick={() => {
                                appWindow.toggleMaximize();
                                setIsMaximized(!isMaximized);
                            }}
                            className="w-4 h-4 group flex items-center justify-center opacity-80 hover:opacity-100 transition-all duration-150"
                            title="Maximize"
                        >
                            <svg viewBox="0 0 320 320" className="w-full h-full transition-colors" style={{ color: 'var(--md-sys-color-primary)' }}>
                                <path d="M136.72 13.1925C147.26 -4.3975 172.74 -4.3975 183.28 13.1925L195.12 32.9625C201.27 43.2125 213.4 48.2425 224.99 45.3325L247.35 39.7325C267.24 34.7525 285.25 52.7626 280.27 72.6526L274.67 95.0126C271.76 106.603 276.79 118.733 287.04 124.883L306.81 136.723C324.4 147.263 324.4 172.743 306.81 183.283L287.04 195.123C276.79 201.273 271.76 213.403 274.67 224.993L280.27 247.353C285.25 267.243 267.24 285.253 247.35 280.273L224.99 274.673C213.4 271.763 201.27 276.793 195.12 287.043L183.28 306.813C172.74 324.403 147.26 324.403 136.72 306.813L124.88 287.043C118.73 276.793 106.6 271.763 95.0102 274.673L72.6462 280.273C52.7632 285.253 34.7472 267.243 39.7292 247.353L45.3332 224.993C48.2382 213.403 43.2143 201.273 32.9603 195.123L13.1873 183.283C-4.39575 172.743 -4.39575 147.263 13.1873 136.723L32.9603 124.883C43.2143 118.733 48.2382 106.603 45.3332 95.0126L39.7292 72.6526C34.7472 52.7626 52.7633 34.7525 72.6453 39.7325L95.0102 45.3332C106.6 48.2425 118.73 43.2125 124.88 32.9625L136.72 13.1925Z" fill="currentColor" />
                            </svg>
                        </button>
                    </div>
                    <div data-tauri-drag-region className="flex-1 flex justify-center">
                        <SearchBar />
                    </div>
                    <div className="flex items-center gap-3">
                        {mobileButton}
                        {nameSection}
                    </div>
                </>
            ) : (
                <>
                    {nameSection}
                    <div data-tauri-drag-region className="flex-1 flex justify-end pr-8">
                        <SearchBar />
                    </div>
                    <div className="flex items-center gap-3 pr-2">
                        {/* Mobile Pairing Button */}
                        {mobileButton}
                        
                        {/* Windows Order: Minimize, Maximize, Close */}
                        <button
                            onClick={() => appWindow.minimize()}
                            className="w-4 h-4 group flex items-center justify-center opacity-80 hover:opacity-100 transition-all duration-150"
                            title="Minimize"
                        >
                            <svg viewBox="0 0 316 278" className="w-full h-full transition-colors text-tertiary">
                                <path d="M271.57 155.799C257.552 177.379 243.535 198.989 229.517 220.569C220.423 234.579 211.167 248.778 198.872 259.908C186.576 271.058 170.648 278.938 154.316 277.908C139.976 276.988 126.684 269.278 116.191 259.208C105.698 249.138 97.5464 236.738 89.5284 224.458C67.8424 191.278 46.1303 158.098 24.4443 124.898C14.1393 109.138 3.56535 92.6884 0.713353 73.9084C-2.73065 51.2184 6.55235 28.1085 23.0183 13.0185C40.2373 -2.76147 68.1384 -1.48141 89.0984 2.83859C112.075 7.58859 134.541 16.5185 157.975 16.4885C178.047 16.4885 197.446 9.88858 216.979 5.08859C236.485 0.318604 257.445 -2.62152 276.279 4.47849C299.659 13.2685 316.448 38.2684 315.991 63.9284C315.56 87.3384 302.457 108.248 289.839 127.728C283.758 137.078 277.678 146.449 271.597 155.799H271.57Z" fill="currentColor" />
                            </svg>
                        </button>
                        <button
                            onClick={() => {
                                appWindow.toggleMaximize();
                                setIsMaximized(!isMaximized);
                            }}
                            className="w-4 h-4 group flex items-center justify-center opacity-80 hover:opacity-100 transition-all duration-150"
                            title="Maximize"
                        >
                            <svg viewBox="0 0 320 320" className="w-full h-full transition-colors text-primary">
                                <path d="M136.72 13.1925C147.26 -4.3975 172.74 -4.3975 183.28 13.1925L195.12 32.9625C201.27 43.2125 213.4 48.2425 224.99 45.3325L247.35 39.7325C267.24 34.7525 285.25 52.7626 280.27 72.6526L274.67 95.0126C271.76 106.603 276.79 118.733 287.04 124.883L306.81 136.723C324.4 147.263 324.4 172.743 306.81 183.283L287.04 195.123C276.79 201.273 271.76 213.403 274.67 224.993L280.27 247.353C285.25 267.243 267.24 285.253 247.35 280.273L224.99 274.673C213.4 271.763 201.27 276.793 195.12 287.043L183.28 306.813C172.74 324.403 147.26 324.403 136.72 306.813L124.88 287.043C118.73 276.793 106.6 271.763 95.0102 274.673L72.6462 280.273C52.7632 285.253 34.7472 267.243 39.7292 247.353L45.3332 224.993C48.2382 213.403 43.2143 201.273 32.9603 195.123L13.1873 183.283C-4.39575 172.743 -4.39575 147.263 13.1873 136.723L32.9603 124.883C43.2143 118.733 48.2382 106.603 45.3332 95.0126L39.7292 72.6526C34.7472 52.7626 52.7633 34.7525 72.6453 39.7325L95.0102 45.3325C106.6 48.2425 118.73 43.2125 124.88 32.9625L136.72 13.1925Z" fill="currentColor" />
                            </svg>
                        </button>
                        <button
                            onClick={() => appWindow.close()}
                            onMouseEnter={() => setCloseHovered(true)}
                            onMouseLeave={() => setCloseHovered(false)}
                            className="w-4 h-4 group flex items-center justify-center opacity-80 hover:opacity-100 transition-all duration-150"
                            title="Close"
                            style={{ filter: closeHovered ? 'drop-shadow(0 0 6px #ef4444)' : 'none' }}
                        >
                            <svg viewBox="0 0 280 280" className={`w-full h-full transition-colors ${closeHovered ? 'text-[#ef4444]' : 'text-secondary'}`}>
                                <path d="M178.73 6.2068C238.87 -19.9132 299.91 41.1269 273.79 101.267L269.47 111.207C261.5 129.577 261.5 150.417 269.47 168.787L273.79 178.727C299.91 238.867 238.87 299.907 178.73 273.787L168.79 269.467C150.42 261.497 129.58 261.497 111.21 269.467L101.27 273.787C41.1281 299.907 -19.9139 238.867 6.20706 178.727L10.5261 168.787C18.5011 150.417 18.5011 129.577 10.5261 111.207L6.20706 101.267C-19.9139 41.1269 41.1281 -19.9132 101.27 6.2068L111.21 10.5269C129.58 18.4969 150.42 18.4969 168.79 10.5269L178.73 6.2068Z" fill="currentColor" />
                            </svg>
                        </button>
                    </div>
                </>
            )}
            
            {/* Mobile Pairing Popup */}
            <MobilePairingPopup anchorRef={mobileButtonRef} />
        </div>
    );
}
