import { create } from 'zustand';

export type AppView = 'tracks' | 'albums' | 'artists' | 'settings' | 'ytmusic' | 'favorites' | 'statistics' | 'torrents';

interface NavigationState {
    view: AppView;
    selectedAlbumKey: string | null;

    setView: (view: AppView) => void;
    navigateToAlbum: (albumName: string, artistName: string) => void;
    clearSelectedAlbum: () => void;
}

export const useNavigationStore = create<NavigationState>((set) => ({
    view: 'tracks',
    selectedAlbumKey: null,

    setView: (view) => set({ view, selectedAlbumKey: null }), // Clearing album when switching main views usually makes sense
    navigateToAlbum: (albumName, artistName) => set({
        view: 'albums',
        selectedAlbumKey: `${albumName}-${artistName}`
    }),
    clearSelectedAlbum: () => set({ selectedAlbumKey: null })
}));
