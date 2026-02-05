import { useEffect } from 'react';
import { usePlayerStore } from '../store/playerStore';
import { useCoverArt } from '../hooks/useCoverArt';
import { useImageColors } from '../hooks/useImageColors';
import { useThemeStore } from '../store/themeStore';

export function ThemeManager() {
    const track = usePlayerStore(state => state.status.track);
    const library = usePlayerStore(state => state.library);
    const setColors = useThemeStore(state => state.setColors);

    // Get cover URL
    const currentIndex = library.findIndex(t => t.path === track?.path);
    const currentLibraryTrack = currentIndex >= 0 ? library[currentIndex] : null;
    // Desktop uses local files
    const coverUrl = useCoverArt(currentLibraryTrack?.cover_image);

    // Extract colors using new M3 Logic
    const colors = useImageColors(coverUrl);

    // Update global theme store & CSS Variables
    useEffect(() => {
        setColors(colors);

        // Update CSS Variables for Tailwind M3 Tokens
        const root = document.documentElement;

        // --- Core Colors ---

        // Primary
        root.style.setProperty('--md-sys-color-primary', colors.primary);
        root.style.setProperty('--md-sys-color-on-primary', colors.onPrimary);
        root.style.setProperty('--md-sys-color-primary-container', colors.primaryContainer);
        root.style.setProperty('--md-sys-color-on-primary-container', colors.onPrimaryContainer);

        // Secondary
        root.style.setProperty('--md-sys-color-secondary', colors.secondary);
        root.style.setProperty('--md-sys-color-on-secondary', colors.onSecondary);
        root.style.setProperty('--md-sys-color-secondary-container', colors.secondaryContainer);
        root.style.setProperty('--md-sys-color-on-secondary-container', colors.onSecondaryContainer);

        // Tertiary
        root.style.setProperty('--md-sys-color-tertiary', colors.tertiary);
        root.style.setProperty('--md-sys-color-on-tertiary', colors.onTertiary);
        root.style.setProperty('--md-sys-color-tertiary-container', colors.tertiaryContainer);
        root.style.setProperty('--md-sys-color-on-tertiary-container', colors.onTertiaryContainer);

        // Surface & Text
        root.style.setProperty('--md-sys-color-surface', colors.surface);
        root.style.setProperty('--md-sys-color-on-surface', colors.onSurface);
        root.style.setProperty('--md-sys-color-surface-variant', colors.surfaceVariant);
        root.style.setProperty('--md-sys-color-on-surface-variant', colors.onSurfaceVariant);

        // Surface Containers (New M3 System)
        root.style.setProperty('--md-sys-color-surface-container-lowest', colors.surfaceContainerLowest); // T4
        root.style.setProperty('--md-sys-color-surface-container-low', colors.surfaceContainerLow);       // T10
        root.style.setProperty('--md-sys-color-surface-container', colors.surfaceContainer);             // T12
        root.style.setProperty('--md-sys-color-surface-container-high', colors.surfaceContainerHigh);     // T17
        root.style.setProperty('--md-sys-color-surface-container-highest', colors.surfaceContainerHighest);// T22

        // Outline
        root.style.setProperty('--md-sys-color-outline', colors.outline);
        root.style.setProperty('--md-sys-color-outline-variant', colors.outlineVariant);

    }, [colors, setColors]);

    return null; // This component renders nothing
}

