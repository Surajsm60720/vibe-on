import Kuroshiro from 'kuroshiro';
import KuromojiAnalyzer from 'kuroshiro-analyzer-kuromoji';
import { LyricsLine } from '../types';

// Singleton instance
let kuroshiroParams: {
    instance: Kuroshiro | null;
    initPromise: Promise<void> | null;
} = {
    instance: null,
    initPromise: null
};

/**
 * Initialize Kuroshiro with the Kuromoji analyzer.
 * Uses the dictionary files located in /public/dict
 */
async function getKuroshiro(): Promise<Kuroshiro> {
    if (kuroshiroParams.instance) return kuroshiroParams.instance;

    if (!kuroshiroParams.initPromise) {
        kuroshiroParams.initPromise = (async () => {
            console.log('[Lyrics] Initializing Kuroshiro...');
            try {
                // Verify dictionary access first
                console.log('[Lyrics] Fetching dictionary base.dat.gz to verify...');
                const response = await fetch('/dict/base.dat.gz');
                console.log('[Lyrics] Dict response headers:', Object.fromEntries(response.headers.entries()));

                if (!response.ok) {
                    throw new Error(`Dictionary files not found at /dict/base.dat.gz (${response.status})`);
                }

                // Inspect the start of the file for magic numbers
                const buffer = await response.clone().arrayBuffer();
                const view = new Uint8Array(buffer.slice(0, 5));
                console.log('[Lyrics] Dict magic bytes:', Array.from(view));
                // GZIP magic number should be 0x1F (31), 0x8B (139)

                // If the browser already decompressed it, the library's gunzip will fail
                // But kuroshiro-analyzer-kuromoji expects gzipped input.

                console.log('[Lyrics] Dictionary files verified.');

                const kuroshiro = new Kuroshiro();
                // Initialize with dictPath relative to public root
                await kuroshiro.init(new KuromojiAnalyzer({
                    dictPath: '/dict'
                }));
                console.log('[Lyrics] Kuroshiro initialized successfully.');
                kuroshiroParams.instance = kuroshiro;
            } catch (error) {
                console.error('[Lyrics] Failed to initialize Kuroshiro:', error);
                kuroshiroParams.initPromise = null; // Allow retry
                throw error;
            }
        })();
    }

    await kuroshiroParams.initPromise;
    if (!kuroshiroParams.instance) throw new Error('Kuroshiro failed to initialize');
    return kuroshiroParams.instance;
}

/**
 * Check if text contains Japanese characters (Hiragana, Katakana, or Kanji)
 */
export function hasJapanese(text: string): boolean {
    return Kuroshiro.Util.hasJapanese(text);
}

/**
 * Convert an array of lyrics lines to Romaji
 */
export async function convertLyrics(lines: LyricsLine[]): Promise<LyricsLine[]> {
    try {
        // First check if we even need conversion
        const needsConversion = lines.some(line => hasJapanese(line.text));
        if (!needsConversion) return lines;

        const kuroshiro = await getKuroshiro();

        const convertedLines = await Promise.all(lines.map(async (line) => {
            if (!hasJapanese(line.text)) {
                return line;
            }

            try {
                const romaji = await kuroshiro.convert(line.text, {
                    to: 'romaji',
                    mode: 'spaced', // or 'normal', 'okurigana', 'furigana'
                    romajiSystem: 'hepburn'
                });
                return { ...line, romaji };
            } catch (err) {
                console.warn(`[Lyrics] Failed to convert line: "${line.text}"`, err);
                return line;
            }
        }));

        return convertedLines;
    } catch (error) {
        console.error('[Lyrics] Conversion failed:', error);
        throw error; // Re-throw so store handles it
    }
}
