# 🎧 Vibe-On Design System — Material 3 Expressive

This document outlines the specific design tokens, rules, and implementation details for the **Vibe-On** music player.

The system adheres to **Material 3 Expressive** guidelines, emphasizing:

- Bolder typography  
- Organic motion  
- Mathematically harmonic dynamic colors  
- Soft, friendly geometry  

---

## 1. Typography (Type Scale)

**Primary Typeface:** `Outfit`  
A geometric sans-serif that complements the rounded, organic shapes of the UI.

---

### Expressive Scale

| Token | Size | Line Height | Weight | Usage |
|------|------|-------------|--------|-------|
| `text-display-hero` | 96px | 112px | 300 (Light) | Main welcome headers (expanded sidebar) |
| `text-display-large` | 57px | 64px | 400 (Regular) | Large album titles |
| `text-display-medium` | 52px | 64px | 400 (Regular) | Standard album titles (detail view) |
| `text-display-small` | 44px | 52px | 400 (Regular) | Section headers |

---

### Standard Scale

| Token | Size | Weight | Usage |
|------|------|--------|-------|
| `text-headline-large` | 32px | 400 | Page titles, sidebar headers |
| `text-title-medium` | 16px | 500 | Track names, list items |
| `text-label-large` | 14px | 500 (Medium) | Buttons, navigation links |
| `text-body-medium` | 14px | 400 | Secondary text, artist names |

---

## 2. Dynamic Color System

Vibe-On derives its theme entirely from the **currently playing album art**.

**Engine:** `@material/material-color-utilities`  
**Method:** `sourceColorFromImage → SchemeTonalSpot (Expressive)`  
**Theme Mode:** Dark mode (always on)

---

### Tonal Mapping (Dark Theme)

| Role | Tone Value | Description |
|------|------------|-------------|
| Primary | T80 | Main accent color (play buttons, active states). Pastel / high luminance |
| On Primary | T20 | Text color on top of primary |
| Primary Container | T30 | Fill for high-emphasis containers |
| Secondary | T80 | Secondary elements (sliders, toggles) |
| Surface | T6 | Main background — very dark, tinted neutral |
| Surface Container | T12 | Default container fill (cards, sidebars) |
| Outline | T60 | Borders and dividers |

---

## 3. Shapes & Corner Radius

Vibe-On uses an **Extra Large rounding strategy** for a friendly, organic appearance.

---

### Radius Tokens

| Token | Value | Usage |
|-------|-------|-------|
| Standard Radius | 32px (`rounded-2xl`) | Sidebar, main content panel, album detail header |
| Card Radius | 32px (`rounded-[2rem]`) | Album cards in grid |
| Full Radius | 9999px (`rounded-full`) | Buttons, pills, navigation items |

---

## 4. Organic Motion (Physics)

Linear or eased CSS transitions are avoided in favor of **mathematically harmonic** motion.

---

### Motion Tokens

| Token | Value | Usage |
|-------|-------|-------|
| Standard Duration | 250ms | Fast, snappy interactions |
| Slow Duration | 500ms | Slower, more deliberate transitions |
| Easing | `cubic-bezier(0.4, 0, 0.2, 1)` | Harmonic easing for smooth motion |
