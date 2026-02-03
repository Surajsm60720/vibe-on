import { useState, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { QRCodeSVG } from 'qrcode.react';
import { useMobileStore } from '../store/mobileStore';
import { useThemeStore } from '../store/themeStore';
import { IconClose, IconRefresh, IconWifi, IconCheck, IconMobileDevice } from './Icons';

interface MobilePairingPopupProps {
    anchorRef: React.RefObject<HTMLButtonElement>;
}

// Decorative floating shape component
function FloatingShape({
    color,
    size,
    delay,
    x,
    y,
    shape = 'circle'
}: {
    color: string;
    size: number;
    delay: number;
    x: number;
    y: number;
    shape?: 'circle' | 'blob' | 'star';
}) {
    const getPath = () => {
        if (shape === 'blob') {
            return (
                <path
                    d="M50 10 C70 10, 90 30, 90 50 C90 70, 70 90, 50 90 C30 90, 10 70, 10 50 C10 30, 30 10, 50 10"
                    fill={color}
                />
            );
        }
        if (shape === 'star') {
            return (
                <path
                    d="M50 5 L58 35 L90 35 L65 55 L75 90 L50 70 L25 90 L35 55 L10 35 L42 35 Z"
                    fill={color}
                />
            );
        }
        return <circle cx="50" cy="50" r="40" fill={color} />;
    };

    return (
        <motion.div
            className="absolute pointer-events-none"
            style={{ left: `${x}%`, top: `${y}%` }}
            initial={{ scale: 0, opacity: 0 }}
            animate={{
                scale: [0.8, 1.1, 0.9, 1],
                opacity: [0.3, 0.5, 0.4, 0.35],
                x: [0, 10, -5, 0],
                y: [0, -8, 4, 0],
            }}
            transition={{
                duration: 8,
                delay,
                repeat: Infinity,
                repeatType: 'reverse',
                ease: 'easeInOut',
            }}
        >
            <svg width={size} height={size} viewBox="0 0 100 100" className="opacity-40">
                {getPath()}
            </svg>
        </motion.div>
    );
}

export function MobilePairingPopup({ anchorRef }: MobilePairingPopupProps) {
    const {
        status,
        serverRunning,
        serverPort,
        localIP,
        discoveredDevices,
        connectedDevice,
        lastConnectedDevice,
        error,
        popupOpen,
        setPopupOpen,
        startServer,
        stopServer,
        disconnect,
        connectToDevice,
        startDiscoveryPolling,
        stopDiscoveryPolling,
    } = useMobileStore();

    const { colors } = useThemeStore();
    const popupRef = useRef<HTMLDivElement>(null);
    const [manualIP, setManualIP] = useState('');
    const [position, setPosition] = useState({ top: 0, right: 0 });

    // Calculate popup position based on anchor
    useEffect(() => {
        if (anchorRef.current && popupOpen) {
            const rect = anchorRef.current.getBoundingClientRect();
            setPosition({
                top: rect.bottom + 8,
                right: window.innerWidth - rect.right,
            });
        }
    }, [anchorRef, popupOpen]);

    // Start/stop discovery polling when popup opens/closes
    useEffect(() => {
        if (popupOpen && serverRunning) {
            startDiscoveryPolling();
        } else {
            stopDiscoveryPolling();
        }

        return () => stopDiscoveryPolling();
    }, [popupOpen, serverRunning, startDiscoveryPolling, stopDiscoveryPolling]);

    // Close on outside click
    useEffect(() => {
        const handleClickOutside = (e: MouseEvent) => {
            if (
                popupRef.current &&
                !popupRef.current.contains(e.target as Node) &&
                anchorRef.current &&
                !anchorRef.current.contains(e.target as Node)
            ) {
                setPopupOpen(false);
            }
        };

        if (popupOpen) {
            document.addEventListener('mousedown', handleClickOutside);
            return () => document.removeEventListener('mousedown', handleClickOutside);
        }
    }, [popupOpen, setPopupOpen, anchorRef]);

    // Auto-start server when popup opens
    useEffect(() => {
        if (popupOpen && !serverRunning) {
            startServer();
        }
    }, [popupOpen, serverRunning, startServer]);

    // Generate connection URL for QR code
    const connectionUrl = localIP
        ? `vibeon://${localIP}:${serverPort}`
        : null;

    const getStatusText = () => {
        switch (status) {
            case 'disconnected': return 'Server Offline';
            case 'searching': return 'Waiting for connection...';
            case 'connecting': return 'Connecting...';
            case 'connected': return `Connected to ${connectedDevice?.name || 'device'}`;
        }
    };

    const getStatusColor = () => {
        switch (status) {
            case 'disconnected': return colors.outline;
            case 'searching': return colors.tertiary;
            case 'connecting': return colors.secondary;
            case 'connected': return colors.primary;
        }
    };

    return (
        <AnimatePresence>
            {popupOpen && (
                <motion.div
                    ref={popupRef}
                    className="fixed z-[9999] w-80 overflow-hidden"
                    style={{
                        top: position.top,
                        right: position.right,
                        backgroundColor: colors.surfaceContainerHigh,
                        borderRadius: '24px',
                        boxShadow: `0 8px 32px rgba(0,0,0,0.4), 0 0 0 1px ${colors.outlineVariant}30`,
                    }}
                    initial={{ opacity: 0, scale: 0.9, y: -10 }}
                    animate={{ opacity: 1, scale: 1, y: 0 }}
                    exit={{ opacity: 0, scale: 0.9, y: -10 }}
                    transition={{ type: 'spring', stiffness: 400, damping: 30 }}
                    onClick={(e) => e.stopPropagation()}
                >
                    {/* Decorative Shapes Background */}
                    <div className="absolute inset-0 overflow-hidden pointer-events-none">
                        <FloatingShape color={colors.primary} size={60} delay={0} x={-10} y={-5} shape="circle" />
                        <FloatingShape color={colors.secondary} size={45} delay={0.5} x={85} y={10} shape="blob" />
                        <FloatingShape color={colors.tertiary} size={35} delay={1} x={75} y={70} shape="star" />
                        <FloatingShape color={colors.primary} size={50} delay={1.5} x={5} y={75} shape="blob" />
                        <FloatingShape color={colors.secondary} size={30} delay={2} x={50} y={85} shape="circle" />
                    </div>

                    {/* Content */}
                    <div className="relative z-10 p-5">
                        {/* Header */}
                        <div className="flex items-center justify-between mb-4">
                            <div className="flex items-center gap-2">
                                <IconMobileDevice size={20} style={{ color: colors.primary }} />
                                <span
                                    className="font-semibold text-base"
                                    style={{ color: colors.onSurface }}
                                >
                                    Mobile Companion
                                </span>
                            </div>
                            <button
                                onClick={() => setPopupOpen(false)}
                                className="p-1 rounded-full transition-colors hover:bg-white/10"
                            >
                                <IconClose size={18} style={{ color: colors.onSurfaceVariant }} />
                            </button>
                        </div>

                        {/* Status Indicator */}
                        <div
                            className="flex items-center gap-2 mb-4 px-3 py-2 rounded-full"
                            style={{ backgroundColor: `${getStatusColor()}20` }}
                        >
                            <div
                                className={`w-2 h-2 rounded-full ${status === 'searching' || status === 'connecting' ? 'animate-pulse' : ''}`}
                                style={{ backgroundColor: getStatusColor() }}
                            />
                            <span
                                className="text-sm font-medium"
                                style={{ color: getStatusColor() }}
                            >
                                {getStatusText()}
                            </span>
                        </div>

                        {/* Error Message */}
                        {error && (
                            <div
                                className="mb-4 px-3 py-2 rounded-xl text-sm"
                                style={{ backgroundColor: '#ef444420', color: '#ef4444' }}
                            >
                                {error}
                            </div>
                        )}

                        {/* QR Code Section */}
                        {serverRunning && connectionUrl && (
                            <div className="mb-4">
                                <div
                                    className="p-4 rounded-2xl flex flex-col items-center gap-3"
                                    style={{ backgroundColor: colors.surfaceContainer }}
                                >
                                    <div
                                        className="p-3 rounded-xl"
                                        style={{ backgroundColor: '#ffffff' }}
                                    >
                                        <QRCodeSVG
                                            value={connectionUrl}
                                            size={140}
                                            level="M"
                                            fgColor={colors.surface}
                                            bgColor="#ffffff"
                                        />
                                    </div>
                                    <p
                                        className="text-xs text-center"
                                        style={{ color: colors.onSurfaceVariant }}
                                    >
                                        Scan with VIBE-ON! mobile app
                                    </p>
                                </div>
                            </div>
                        )}

                        {/* Connection Info */}
                        {serverRunning && localIP && (
                            <div
                                className="mb-4 p-3 rounded-xl"
                                style={{ backgroundColor: colors.surfaceContainer }}
                            >
                                <div className="flex items-center gap-2 mb-2">
                                    <IconWifi size={16} style={{ color: colors.secondary }} />
                                    <span
                                        className="text-sm font-medium"
                                        style={{ color: colors.onSurface }}
                                    >
                                        Server Address
                                    </span>
                                </div>
                                <code
                                    className="block text-sm font-mono px-2 py-1 rounded"
                                    style={{
                                        backgroundColor: colors.surfaceContainerHighest,
                                        color: colors.primary,
                                    }}
                                >
                                    {localIP}:{serverPort}
                                </code>
                            </div>
                        )}

                        {/* Discovered Mobile Devices */}
                        {serverRunning && discoveredDevices.length > 0 && (
                            <div
                                className="mb-4 p-3 rounded-xl"
                                style={{ backgroundColor: colors.surfaceContainer }}
                            >
                                <div className="flex items-center gap-2 mb-3">
                                    <IconMobileDevice size={16} style={{ color: colors.tertiary }} />
                                    <span
                                        className="text-sm font-medium"
                                        style={{ color: colors.onSurface }}
                                    >
                                        Discovered Devices ({discoveredDevices.length})
                                    </span>
                                </div>
                                <div className="space-y-2">
                                    {discoveredDevices.map((device) => (
                                        <motion.div
                                            key={device.id}
                                            initial={{ opacity: 0, x: -10 }}
                                            animate={{ opacity: 1, x: 0 }}
                                            className="flex items-center justify-between p-2 rounded-lg cursor-pointer transition-colors hover:opacity-80"
                                            style={{ backgroundColor: colors.surfaceContainerHighest }}
                                            onClick={() => connectToDevice(device)}
                                        >
                                            <div className="flex items-center gap-2">
                                                <div
                                                    className="w-8 h-8 rounded-full flex items-center justify-center"
                                                    style={{ backgroundColor: `${colors.tertiary}30` }}
                                                >
                                                    <IconMobileDevice size={16} style={{ color: colors.tertiary }} />
                                                </div>
                                                <div>
                                                    <p
                                                        className="text-sm font-medium"
                                                        style={{ color: colors.onSurface }}
                                                    >
                                                        {device.name}
                                                    </p>
                                                    <p
                                                        className="text-xs"
                                                        style={{ color: colors.onSurfaceVariant }}
                                                    >
                                                        {device.ip} • {device.platform}
                                                    </p>
                                                </div>
                                            </div>
                                            <div
                                                className="px-2 py-1 rounded-full text-xs font-medium"
                                                style={{
                                                    backgroundColor: `${colors.primary}20`,
                                                    color: colors.primary,
                                                }}
                                            >
                                                Connect
                                            </div>
                                        </motion.div>
                                    ))}
                                </div>
                            </div>
                        )}

                        {/* Scanning indicator when no devices found */}
                        {serverRunning && discoveredDevices.length === 0 && status === 'searching' && (
                            <div
                                className="mb-4 p-3 rounded-xl"
                                style={{ backgroundColor: colors.surfaceContainer }}
                            >
                                <div className="flex items-center gap-3">
                                    <div className="w-5 h-5 border-2 border-t-transparent rounded-full animate-spin"
                                        style={{ borderColor: `${colors.tertiary}40`, borderTopColor: 'transparent' }}
                                    />
                                    <span
                                        className="text-sm"
                                        style={{ color: colors.onSurfaceVariant }}
                                    >
                                        Scanning for mobile devices...
                                    </span>
                                </div>
                            </div>
                        )}

                        {/* Manual IP Entry (for mobile connecting back) */}
                        <div
                            className="mb-4 p-3 rounded-xl"
                            style={{ backgroundColor: colors.surfaceContainer }}
                        >
                            <label
                                className="block text-sm font-medium mb-2"
                                style={{ color: colors.onSurfaceVariant }}
                            >
                                Manual Connection
                            </label>
                            <div className="flex gap-2">
                                <input
                                    type="text"
                                    value={manualIP}
                                    onChange={(e) => setManualIP(e.target.value)}
                                    placeholder="192.168.1.x"
                                    className="flex-1 px-3 py-2 rounded-xl text-sm outline-none transition-colors"
                                    style={{
                                        backgroundColor: colors.surfaceContainerHighest,
                                        color: colors.onSurface,
                                        border: `1px solid ${colors.outlineVariant}40`,
                                    }}
                                />
                                <button
                                    className="px-3 py-2 rounded-xl transition-colors"
                                    style={{
                                        backgroundColor: colors.primaryContainer,
                                        color: colors.onPrimaryContainer,
                                    }}
                                >
                                    <IconCheck size={18} />
                                </button>
                            </div>
                        </div>

                        {/* Last Connected Device */}
                        {lastConnectedDevice && status !== 'connected' && (
                            <div
                                className="mb-4 p-3 rounded-xl cursor-pointer transition-colors hover:opacity-80"
                                style={{ backgroundColor: colors.surfaceContainer }}
                                onClick={() => {
                                    // Reconnect to last device
                                }}
                            >
                                <div className="flex items-center justify-between">
                                    <div>
                                        <p
                                            className="text-sm font-medium"
                                            style={{ color: colors.onSurface }}
                                        >
                                            {lastConnectedDevice.name}
                                        </p>
                                        <p
                                            className="text-xs"
                                            style={{ color: colors.onSurfaceVariant }}
                                        >
                                            Last connected
                                        </p>
                                    </div>
                                    <IconRefresh size={18} style={{ color: colors.primary }} />
                                </div>
                            </div>
                        )}

                        {/* Action Buttons */}
                        <div className="flex gap-2">
                            {!serverRunning ? (
                                <button
                                    onClick={startServer}
                                    className="flex-1 py-2.5 rounded-full font-medium text-sm transition-all hover:opacity-90"
                                    style={{
                                        backgroundColor: colors.primary,
                                        color: colors.onPrimary,
                                    }}
                                >
                                    Start Server
                                </button>
                            ) : status === 'connected' ? (
                                <button
                                    onClick={disconnect}
                                    className="flex-1 py-2.5 rounded-full font-medium text-sm transition-all hover:opacity-90"
                                    style={{
                                        backgroundColor: colors.secondaryContainer,
                                        color: colors.onSecondaryContainer,
                                    }}
                                >
                                    Disconnect
                                </button>
                            ) : (
                                <button
                                    onClick={stopServer}
                                    className="flex-1 py-2.5 rounded-full font-medium text-sm transition-all hover:opacity-90"
                                    style={{
                                        backgroundColor: colors.surfaceContainerHighest,
                                        color: colors.onSurface,
                                    }}
                                >
                                    Stop Server
                                </button>
                            )}
                        </div>
                    </div>
                </motion.div>
            )}
        </AnimatePresence>
    );
}
