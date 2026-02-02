import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export default defineConfig({
    plugins: [react()],
    // Set root to the parent directory (project root) so it finds index.html and src/
    root: path.resolve(__dirname, '..'),
    build: {
        // Output to host/dist
        outDir: path.resolve(__dirname, 'dist'),
        emptyOutDir: true,
    },
    resolve: {
        alias: {
            // Redirect Tauri calls to mock
            '@tauri-apps/api/core': path.resolve(__dirname, 'mock-tauri.ts'),
            '@tauri-apps/plugin-fs': path.resolve(__dirname, 'mock-tauri.ts'),
            '@tauri-apps/plugin-dialog': path.resolve(__dirname, 'mock-tauri.ts'),
            '@tauri-apps/api/event': path.resolve(__dirname, 'mock-tauri.ts'),
            '@tauri-apps/plugin-opener': path.resolve(__dirname, 'mock-tauri.ts'),
            '@tauri-apps/api/window': path.resolve(__dirname, 'mock-tauri.ts'),
            '@tauri-apps/api/webviewWindow': path.resolve(__dirname, 'mock-tauri.ts'),
            '@tauri-apps/api/path': path.resolve(__dirname, 'mock-tauri.ts'),
            // Removed root @tauri-apps/api alias to prevent ENOTDIR errors on subpath imports
        }
    },
    define: {
        // Feature flag to enable Web Mode
        'import.meta.env.VITE_APP_MODE': '"web"',
    },
    css: {
        postcss: path.resolve(__dirname, '../postcss.config.js'),
    },
});
