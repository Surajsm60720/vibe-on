import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type MobileConnectionStatus = 'disconnected' | 'searching' | 'connecting' | 'connected';

export interface DiscoveredDevice {
    id: string;
    name: string;
    ip: string;
    port: number;
    platform: string;
}

// P2P peer from Rust backend
interface P2PPeer {
    peer_id: string;
    addresses: string[];
    device_name: string;
    platform: string;
    version: string;
    is_local: boolean;
}

export interface ConnectedDevice {
    id: string;
    name: string;
    ip: string;
    port: number;
    connectedAt: number;
}

interface MobileStore {
    // State
    status: MobileConnectionStatus;
    serverRunning: boolean;
    serverPort: number;
    localIP: string | null;
    discoveredDevices: DiscoveredDevice[];
    connectedDevice: ConnectedDevice | null;
    lastConnectedDevice: ConnectedDevice | null;
    error: string | null;

    // UI State
    popupOpen: boolean;

    // Actions
    setPopupOpen: (open: boolean) => void;
    togglePopup: () => void;

    // Server Actions
    startServer: () => Promise<void>;
    stopServer: () => Promise<void>;
    checkServerStatus: () => Promise<void>;

    // Connection Actions
    setStatus: (status: MobileConnectionStatus) => void;
    addDiscoveredDevice: (device: DiscoveredDevice) => void;
    removeDiscoveredDevice: (id: string) => void;
    clearDiscoveredDevices: () => void;
    connectToDevice: (device: DiscoveredDevice) => Promise<void>;
    setConnectedDevice: (device: ConnectedDevice) => void;
    disconnect: () => void;
    setError: (error: string | null) => void;

    // Network & Discovery
    fetchLocalIP: () => Promise<void>;
    scanForDevices: () => Promise<void>;
    startDiscoveryPolling: () => void;
    stopDiscoveryPolling: () => void;
    setupListeners: () => Promise<void>;
}

export const useMobileStore = create<MobileStore>()(
    persist(
        (set, get) => ({
            // Initial State
            status: 'disconnected',
            serverRunning: false,
            serverPort: 5443,
            localIP: null,
            discoveredDevices: [],
            connectedDevice: null,
            lastConnectedDevice: null,
            error: null,
            popupOpen: false,

            // UI Actions
            setPopupOpen: (open) => set({ popupOpen: open }),
            togglePopup: () => set((state) => ({ popupOpen: !state.popupOpen })),

            // Server Actions
            startServer: async () => {
                try {
                    set({ error: null, status: 'searching' });
                    // Ensure listeners are active
                    get().setupListeners();
                    await invoke('start_mobile_server');
                    set({ serverRunning: true });
                    // Wait a bit for server to fully start, then fetch info
                    await new Promise(resolve => setTimeout(resolve, 500));
                    await get().fetchLocalIP();
                } catch (e) {
                    set({ error: String(e), status: 'disconnected' });
                }
            },

            stopServer: async () => {
                try {
                    await invoke('stop_mobile_server');
                    set({
                        serverRunning: false,
                        status: 'disconnected',
                        connectedDevice: null,
                    });
                } catch (e) {
                    set({ error: String(e) });
                }
            },

            checkServerStatus: async () => {
                try {
                    const running = await invoke<boolean>('get_server_status');
                    set({ serverRunning: running });
                    if (running && get().status === 'disconnected') {
                        set({ status: 'searching' });
                        await get().fetchLocalIP();
                    }
                } catch (e) {
                    console.error('Failed to check server status:', e);
                }
            },

            // Connection Actions
            setStatus: (status) => set({ status }),

            addDiscoveredDevice: (device) => set((state) => ({
                discoveredDevices: state.discoveredDevices.some(d => d.id === device.id)
                    ? state.discoveredDevices
                    : [...state.discoveredDevices, device],
            })),

            removeDiscoveredDevice: (id) => set((state) => ({
                discoveredDevices: state.discoveredDevices.filter(d => d.id !== id),
            })),

            clearDiscoveredDevices: () => set({ discoveredDevices: [] }),

            connectToDevice: async (device) => {
                set({ status: 'connecting', error: null });
                try {
                    // In future: establish actual connection
                    // For now, just mark as connected
                    const connectedDevice: ConnectedDevice = {
                        ...device,
                        connectedAt: Date.now(),
                    };
                    set({
                        status: 'connected',
                        connectedDevice,
                        lastConnectedDevice: connectedDevice,
                    });
                } catch (e) {
                    set({ status: 'searching', error: String(e) });
                }
            },

            setConnectedDevice: (device) => {
                set({
                    status: 'connected',
                    connectedDevice: device,
                    lastConnectedDevice: device,
                });
            },

            disconnect: () => {
                set({
                    status: get().serverRunning ? 'searching' : 'disconnected',
                    connectedDevice: null,
                });
            },

            setError: (error) => set({ error }),

            // Network & Discovery
            fetchLocalIP: async () => {
                try {
                    // Fetch server info which includes local IP
                    const port = get().serverPort;
                    const response = await fetch(`http://localhost:${port}/api/info`).catch(() => null);
                    if (response?.ok) {
                        const data = await response.json();
                        console.log('[Mobile] Server info:', data);
                        if (data.localIp) {
                            set({ localIP: data.localIp, serverPort: data.port || port });
                            return;
                        }
                    }
                    // Fallback: just use localhost indicator
                    set({ localIP: 'localhost' });
                } catch (e) {
                    console.error('Failed to fetch local IP:', e);
                    set({ localIP: null });
                }
            },

            scanForDevices: async () => {
                try {
                    const peers = await invoke<P2PPeer[]>('get_p2p_peers');
                    console.log('[Mobile] Discovered peers:', peers);

                    // Convert P2P peers to DiscoveredDevice format
                    const devices: DiscoveredDevice[] = peers
                        .filter(peer => peer.platform === 'android' || peer.platform === 'ios' || peer.platform === 'mobile')
                        .map(peer => {
                            // Extract IP from multiaddr (e.g., "/ip4/192.168.1.x/udp/...")
                            let ip = 'unknown';
                            for (const addr of peer.addresses) {
                                const match = addr.match(/\/ip4\/([0-9.]+)\//);
                                if (match) {
                                    ip = match[1];
                                    break;
                                }
                            }

                            return {
                                id: peer.peer_id,
                                name: peer.device_name || 'Mobile Device',
                                ip,
                                port: 5443,
                                platform: peer.platform,
                            };
                        });

                    set({ discoveredDevices: devices });
                } catch (e) {
                    console.error('Failed to scan for devices:', e);
                }
            },

            startDiscoveryPolling: () => {
                // Poll for devices every 3 seconds
                const poll = async () => {
                    if (get().serverRunning && get().popupOpen) {
                        await get().scanForDevices();
                    }
                };

                // Initial scan
                poll();

                // Set up interval (stored in window for cleanup)
                const intervalId = setInterval(poll, 3000);
                (window as unknown as { __mobileDiscoveryInterval?: number }).__mobileDiscoveryInterval = intervalId;
            },

            stopDiscoveryPolling: () => {
                const intervalId = (window as unknown as { __mobileDiscoveryInterval?: number }).__mobileDiscoveryInterval;
                if (intervalId) {
                    clearInterval(intervalId);
                    (window as unknown as { __mobileDiscoveryInterval?: number }).__mobileDiscoveryInterval = undefined;
                }
            },

            setupListeners: async () => {
                // Listen for connection events from Rust backend
                await listen('mobile_client_connected', (event: any) => {
                    console.log('[Mobile] Connected event:', event);
                    const clientInput = event.payload;

                    const device: ConnectedDevice = {
                        id: clientInput.client_id,
                        name: clientInput.client_name,
                        ip: 'unknown',
                        port: get().serverPort,
                        connectedAt: Date.now()
                    };

                    set({
                        status: 'connected',
                        connectedDevice: device,
                        lastConnectedDevice: device,
                    });
                });

                await listen('mobile_client_disconnected', (event: any) => {
                    console.log('[Mobile] Disconnected event:', event);
                    const current = get().connectedDevice;
                    if (current && current.id === event.payload.client_id) {
                        set({ status: get().serverRunning ? 'searching' : 'disconnected', connectedDevice: null });
                    }
                });
            },
        }),
        {
            name: 'vibe-mobile',
            partialize: (state) => ({
                lastConnectedDevice: state.lastConnectedDevice,
                serverPort: state.serverPort,
            }),
        }
    )
);
