/**
 * Spotify Web API integration for native search and analytics.
 * This is isolated within the stream/ directory for easy maintenance.
 */

export interface SpotifyTrack {
    id: string;
    name: string;
    artists: { name: string }[];
    album: {
        name: string;
        images: { url: string; width: number; height: number }[];
    };
    duration_ms: number;
    preview_url: string | null;
    external_urls: { spotify: string };
}

export interface AudioFeatures {
    danceability: number;
    energy: number;
    key: number;
    loudness: number;
    mode: number;
    speechiness: number;
    acousticness: number;
    instrumentalness: number;
    liveness: number;
    valence: number;
    tempo: number;
}

class SpotifyAPI {
    private accessToken: string | null = null;

    async authenticate(clientId: string, clientSecret: string) {
        const response = await fetch('https://accounts.spotify.com/api/token', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/x-www-form-urlencoded',
                Authorization: 'Basic ' + btoa(clientId + ':' + clientSecret),
            },
            body: 'grant_type=client_credentials',
        });

        const data = await response.json();
        if (data.access_token) {
            this.accessToken = data.access_token;
            return true;
        }
        return false;
    }

    async searchTracks(query: string): Promise<SpotifyTrack[]> {
        if (!this.accessToken) return [];

        const response = await fetch(
            `https://api.spotify.com/1/search?q=${encodeURIComponent(query)}&type=track&limit=20`,
            {
                headers: {
                    Authorization: `Bearer ${this.accessToken}`,
                },
            }
        );

        const data = await response.json();
        return data.tracks?.items || [];
    }

    async getAudioFeatures(trackId: string): Promise<AudioFeatures | null> {
        if (!this.accessToken) return null;

        const response = await fetch(`https://api.spotify.com/1/audio-features/${trackId}`, {
            headers: {
                Authorization: `Bearer ${this.accessToken}`,
            },
        });

        return await response.json();
    }
}

export const spotifyApi = new SpotifyAPI();
