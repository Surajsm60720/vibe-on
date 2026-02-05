# VIBE-ON! Mobile Design System

This document provides comprehensive design guidelines for implementing the VIBE-ON! mobile app to match the desktop experience. Based on **Material 3 Expressive** with custom adaptations.

---

## Table of Contents

1. [Design Philosophy](#design-philosophy)
2. [Typography](#typography)
3. [Color System](#color-system)
4. [Shapes & Corner Radius](#shapes--corner-radius)
5. [Motion & Animation](#motion--animation)
6. [Iconography](#iconography)
7. [Components](#components)
8. [Screens & Navigation](#screens--navigation)
9. [Special UI Elements](#special-ui-elements)

---

## Design Philosophy

VIBE-ON! follows **Material 3 Expressive** guidelines with emphasis on:

- **Bold Typography** — Large, impactful text with Outfit font
- **Organic Motion** — Spring-based physics animations, not linear easing
- **Dynamic Colors** — Theme derived from album art using Material Color Utilities
- **Soft Geometry** — Extra-large corner radii (32px+) for friendly, organic feel
- **Dark Mode First** — Always dark theme, colors adapt to content

### Key Principles

1. **Content-First** — Album art and track info are heroes
2. **Playful Controls** — Buttons use organic "scalloped" shapes
3. **Fluid Transitions** — Spring physics for all state changes
4. **Immersive Experience** — Full-screen modes with ambient backgrounds

---

## Typography

### Font Family

**Primary:** `Outfit` (Google Fonts)
- Geometric sans-serif
- Complements rounded UI shapes
- Weights: 300 (Light), 400 (Regular), 500 (Medium), 600 (SemiBold), 700 (Bold)

### Mobile Type Scale

| Token | Size | Line Height | Weight | Usage |
|-------|------|-------------|--------|-------|
| `displayLarge` | 48sp | 56sp | 300 | Now Playing track title |
| `displayMedium` | 36sp | 44sp | 400 | Album detail headers |
| `displaySmall` | 32sp | 40sp | 400 | Section headers |
| `headlineLarge` | 28sp | 36sp | 400 | Page titles |
| `headlineMedium` | 24sp | 32sp | 400 | Subsection titles |
| `titleLarge` | 20sp | 28sp | 400 | List item titles |
| `titleMedium` | 16sp | 24sp | 500 | Track names, card titles |
| `titleSmall` | 14sp | 20sp | 500 | Secondary titles |
| `labelLarge` | 14sp | 20sp | 500 | Buttons, tabs |
| `labelMedium` | 12sp | 16sp | 500 | Chips, badges |
| `labelSmall` | 11sp | 16sp | 500 | Timestamps, metadata |
| `bodyLarge` | 16sp | 24sp | 400 | Primary body text |
| `bodyMedium` | 14sp | 20sp | 400 | Artist names, descriptions |
| `bodySmall` | 12sp | 16sp | 400 | Captions, hints |

### Jetpack Compose Implementation

```kotlin
val OutfitFontFamily = FontFamily(
    Font(R.font.outfit_light, FontWeight.Light),
    Font(R.font.outfit_regular, FontWeight.Normal),
    Font(R.font.outfit_medium, FontWeight.Medium),
    Font(R.font.outfit_semibold, FontWeight.SemiBold),
    Font(R.font.outfit_bold, FontWeight.Bold)
)

val VibeOnTypography = Typography(
    displayLarge = TextStyle(
        fontFamily = OutfitFontFamily,
        fontWeight = FontWeight.Light,
        fontSize = 48.sp,
        lineHeight = 56.sp,
        letterSpacing = (-0.02).em
    ),
    titleMedium = TextStyle(
        fontFamily = OutfitFontFamily,
        fontWeight = FontWeight.Medium,
        fontSize = 16.sp,
        lineHeight = 24.sp,
        letterSpacing = 0.01.em
    ),
    // ... etc
)
```

---

## Color System

### Dynamic Color Engine

Colors are derived from album artwork using `@material/material-color-utilities`:

- **Method:** `sourceColorFromImage → SchemeTonalSpot (Expressive)`
- **Mode:** Dark theme only
- **Fallback:** Purple-based palette when no album art

### Tonal Palette (Dark Theme)

| Role | Tone | Hex (Fallback) | Usage |
|------|------|----------------|-------|
| `primary` | T80 | `#7C4DFF` | Play buttons, active states, accent |
| `onPrimary` | T20 | `#FFFFFF` | Text/icons on primary |
| `primaryContainer` | T30 | `#5F2EEA` | High-emphasis containers |
| `secondary` | T80 | `#00E5FF` | Secondary accents, sliders |
| `onSecondary` | T20 | `#000000` | Text on secondary |
| `tertiary` | T80 | `#FF4081` | Favorites, hearts |
| `surface` | T6 | `#0F0F13` | Main background |
| `surfaceContainerLow` | T10 | `#18181E` | Elevated surfaces |
| `surfaceContainer` | T12 | `#1F1F26` | Cards, sheets |
| `surfaceContainerHigh` | T17 | `#2B2B35` | Dialogs, navigation |
| `surfaceContainerHighest` | T22 | `#353541` | Highest elevation |
| `onSurface` | T90 | `#F4F4F6` | Primary text |
| `onSurfaceVariant` | T80 | `#C6C5D0` | Secondary text |
| `outline` | T60 | `#938F99` | Borders |
| `outlineVariant` | T30 | `#484649` | Subtle dividers |
| `error` | T80 | `#FF5252` | Error states |

### Jetpack Compose Implementation

```kotlin
// Dynamic color from album art
fun generateThemeFromBitmap(bitmap: Bitmap): ColorScheme {
    val sourceColor = MaterialColors.sourceColorFromImage(bitmap)
    val scheme = SchemeTonalSpot(Hct.fromInt(sourceColor), true, 0.0) // Dark mode
    
    return darkColorScheme(
        primary = Color(scheme.primary),
        onPrimary = Color(scheme.onPrimary),
        primaryContainer = Color(scheme.primaryContainer),
        surface = Color(scheme.surface),
        // ... map all roles
    )
}

// Fallback colors
val VibeOnFallbackColors = darkColorScheme(
    primary = Color(0xFF7C4DFF),
    onPrimary = Color.White,
    surface = Color(0xFF0F0F13),
    onSurface = Color(0xFFF4F4F6),
    surfaceContainer = Color(0xFF1F1F26),
    // ...
)
```

---

## Shapes & Corner Radius

VIBE-ON! uses **Extra-Large rounding** for organic, friendly appearance.

### Radius Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `extraSmall` | 4dp | Small chips, badges |
| `small` | 8dp | Input fields |
| `medium` | 12dp | Small cards |
| `large` | 16dp | Standard cards |
| `extraLarge` | 24dp | Dialogs, sheets |
| `xxl` | 32dp | Main containers, album cards |
| `full` | 50% | Circular buttons, avatars |

### Jetpack Compose Shapes

```kotlin
val VibeOnShapes = Shapes(
    extraSmall = RoundedCornerShape(4.dp),
    small = RoundedCornerShape(8.dp),
    medium = RoundedCornerShape(12.dp),
    large = RoundedCornerShape(16.dp),
    extraLarge = RoundedCornerShape(24.dp)
)

// Custom shapes
val AlbumCardShape = RoundedCornerShape(32.dp)
val BottomSheetShape = RoundedCornerShape(topStart = 32.dp, topEnd = 32.dp)
val PlayButtonShape = CircleShape
```

### M3 Rounded Square (Album Art)

Album covers use a special "squircle" shape with mathematically smooth corners:

```kotlin
@Composable
fun M3RoundedSquare(
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit
) {
    Box(
        modifier = modifier
            .clip(RoundedCornerShape(20)) // 20% of size
            .background(MaterialTheme.colorScheme.surfaceContainerHighest)
    ) {
        content()
    }
}
```

---

## Motion & Animation

### Motion Principles

- **No linear easing** — Always use spring physics or emphasized curves
- **Organic feel** — Elements should feel weighted and natural
- **Responsive** — Fast feedback on interaction, smooth settle

### Animation Tokens

| Token | Spec | Usage |
|-------|------|-------|
| `standardSpring` | stiffness: 300, damping: 30 | General transitions |
| `smoothSpring` | stiffness: 300, damping: 40 | Playback controls |
| `snappySpring` | stiffness: 1400, damping: 90 | Sidebar/navigation |
| `bouncySpring` | stiffness: 400, damping: 10 | Play button pulse |
| `emphasized` | cubic-bezier(0.2, 0.0, 0.0, 1.0) | M3 emphasized easing |

### Jetpack Compose Animation

```kotlin
// Spring specs
val StandardSpring = spring<Float>(
    stiffness = Spring.StiffnessLow,
    dampingRatio = Spring.DampingRatioMediumBouncy
)

val SmoothSpring = spring<Float>(
    stiffness = 300f,
    dampingRatio = 0.7f
)

val SnappySpring = spring<Float>(
    stiffness = 1400f,
    dampingRatio = 0.9f
)

// Button press animation
@Composable
fun AnimatedButton(onClick: () -> Unit, content: @Composable () -> Unit) {
    var pressed by remember { mutableStateOf(false) }
    val scale by animateFloatAsState(
        targetValue = if (pressed) 0.9f else 1f,
        animationSpec = spring(stiffness = 400f, dampingRatio = 0.5f)
    )
    
    Box(
        modifier = Modifier
            .scale(scale)
            .pointerInput(Unit) {
                detectTapGestures(
                    onPress = {
                        pressed = true
                        tryAwaitRelease()
                        pressed = false
                        onClick()
                    }
                )
            }
    ) {
        content()
    }
}
```

---

## Iconography

All icons are custom SVG paths. Use outlined style with 2px stroke or filled depending on state.

### Icon Sizes

| Size | Value | Usage |
|------|-------|-------|
| Small | 20dp | Inline icons, metadata |
| Medium | 24dp | Standard icons |
| Large | 28dp | Navigation icons |
| XLarge | 32dp | Player controls |
| Hero | 48dp+ | Now playing main controls |

### Icon Set (SVG Paths)

```kotlin
object VibeOnIcons {
    // Navigation
    val Home = "M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"
    val Album = "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 14.5c-2.49 0-4.5-2.01-4.5-4.5S9.51 7.5 12 7.5s4.5 2.01 4.5 4.5-2.01 4.5-4.5 4.5zm0-5.5c-.55 0-1 .45-1 1s.45 1 1 1 1-.45 1-1-.45-1-1-1z"
    val Artist = "M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3zM17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-2.08c3.39-.49 6-3.39 6-6.92v-1h-2z"
    val Settings = "M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.488.488 0 0 0-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 0 0-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"
    
    // Playback
    val Play = "M8 5v14l11-7z"
    val Pause = "M6 19h4V5H6v14zm8-14v14h4V5h-4z"
    val Next = "M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"
    val Previous = "M6 6h2v12H6zm3.5 6l8.5 6V6z"
    val Shuffle = "M10.59 9.17L5.41 4 4 5.41l5.17 5.17 1.42-1.41zM14.5 4l2.04 2.04L4 18.59 5.41 20 17.96 7.46 20 9.5V4h-5.5zm.33 9.41l-1.41 1.41 3.13 3.13L14.5 20H20v-5.5l-2.04 2.04-3.13-3.13z"
    val Repeat = "M7 7h10v3l4-4-4-4v3H5v6h2V7zm10 10H7v-3l-4 4 4 4v-3h12v-6h-2v4z"
    
    // UI
    val Search = "M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"
    val Heart = "M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"
    val HeartOutline = "M16.5 3c-1.74 0-3.41.81-4.5 2.09C10.91 3.81 9.24 3 7.5 3 4.42 3 2 5.42 2 8.5c0 3.78 3.4 6.86 8.55 11.54L12 21.35l1.45-1.32C18.6 15.36 22 12.28 22 8.5 22 5.42 19.58 3 16.5 3zm-4.4 15.55l-.1.1-.1-.1C7.14 14.24 4 11.39 4 8.5 4 6.5 5.5 5 7.5 5c1.54 0 3.04.99 3.57 2.36h1.87C13.46 5.99 14.96 5 16.5 5c2 0 3.5 1.5 3.5 3.5 0 2.89-3.14 5.74-7.9 10.05z"
    val Queue = "M15 6H3v2h12V6zm0 4H3v2h12v-2zM3 16h8v-2H3v2zM17 6v8.18c-.31-.11-.65-.18-1-.18-1.66 0-3 1.34-3 3s1.34 3 3 3 3-1.34 3-3V8h3V6h-5z"
    val Volume = "M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77z"
    val Lyrics = "M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-5 14H7v-2h7v2zm3-4H7v-2h10v2zm0-4H7V7h10v2z"
    val Fullscreen = "M7 14H5v5h5v-2H7v-3zm-2-4h2V7h3V5H5v5zm12 7h-3v2h5v-5h-2v3zM14 5v2h3v3h2V5h-5z"
    val Close = "M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"
    val MusicNote = "M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"
    val Stats = "M18 20V10M12 20V4M6 20v-6" // Stroke-based
    val Download = "M5 20h14v-2H5v2zM19 9h-4V3H9v6H5l7 7 7-7z"
    val Check = "M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"
    val Refresh = "M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"
    val Wifi = "M1 9l2 2c4.97-4.97 13.03-4.97 18 0l2-2C16.93 2.93 7.08 2.93 1 9zm8 8l3 3 3-3c-1.65-1.66-4.34-1.66-6 0zm-4-4l2 2c2.76-2.76 7.24-2.76 10 0l2-2C15.14 9.14 8.87 9.14 5 13z"
    val Mobile = "M17 1.01L7 1c-1.1 0-2 .9-2 2v18c0 1.1.9 2 2 2h10c1.1 0 2-.9 2-2V3c0-1.1-.9-1.99-2-1.99zM17 19H7V5h10v14z"
}

@Composable
fun VibeOnIcon(
    path: String,
    modifier: Modifier = Modifier,
    size: Dp = 24.dp,
    tint: Color = LocalContentColor.current
) {
    Icon(
        painter = rememberVectorPainter(
            defaultWidth = size,
            defaultHeight = size,
            viewportWidth = 24f,
            viewportHeight = 24f,
            autoMirror = false
        ) { _, _ ->
            Path(
                pathData = PathParser().parsePathString(path).toNodes(),
                fill = SolidColor(tint)
            )
        },
        contentDescription = null,
        modifier = modifier.size(size),
        tint = tint
    )
}
```

---

## Components

### 1. Squiggly Slider (Progress Bar)

The signature animated progress bar that "squiggles" when playing:

```kotlin
@Composable
fun SquigglySlider(
    value: Float,
    onValueChange: (Float) -> Unit,
    isPlaying: Boolean,
    modifier: Modifier = Modifier,
    accentColor: Color = MaterialTheme.colorScheme.primary
) {
    val animatedAmplitude by animateFloatAsState(
        targetValue = if (isPlaying) 4f else 0f,
        animationSpec = spring(stiffness = 300f, dampingRatio = 0.6f)
    )
    
    Canvas(modifier = modifier.height(24.dp)) {
        val width = size.width
        val height = size.height
        val centerY = height / 2
        
        // Draw wave path
        val path = Path().apply {
            moveTo(0f, centerY)
            for (x in 0..width.toInt() step 2) {
                val y = centerY + sin(x * 0.15f) * animatedAmplitude
                lineTo(x.toFloat(), y)
            }
        }
        
        // Inactive track
        drawPath(path, color = Color.White.copy(alpha = 0.1f), style = Stroke(4.dp.toPx()))
        
        // Active track (clipped to progress)
        clipRect(right = width * value) {
            drawPath(path, color = accentColor, style = Stroke(4.dp.toPx()))
        }
    }
}
```

### 2. Scalloped Button (Organic Shape)

The expressive "sunny" or "scalloped" button shape used for skip controls:

```kotlin
@Composable
fun ScallopedButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit
) {
    var rotation by remember { mutableFloatStateOf(0f) }
    val animatedRotation by animateFloatAsState(
        targetValue = rotation,
        animationSpec = spring(stiffness = 200f, dampingRatio = 0.5f)
    )
    
    Box(
        modifier = modifier
            .size(48.dp)
            .clickable {
                rotation += 120f
                onClick()
            },
        contentAlignment = Alignment.Center
    ) {
        // Rotating scalloped background
        Canvas(modifier = Modifier.fillMaxSize().rotate(animatedRotation)) {
            // Draw 12-point scalloped shape
            drawScallopedShape(color = surfaceContainer)
        }
        content()
    }
}
```

### 3. Album Card

```kotlin
@Composable
fun AlbumCard(
    album: Album,
    onClick: () -> Unit,
    onPlayClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier
            .aspectRatio(1f)
            .clip(RoundedCornerShape(32.dp))
            .clickable(onClick = onClick)
    ) {
        // Album art with M3 squircle shape
        AsyncImage(
            model = album.coverUrl,
            contentDescription = album.name,
            modifier = Modifier.fillMaxSize(),
            contentScale = ContentScale.Crop
        )
        
        // Gradient overlay
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(
                    Brush.verticalGradient(
                        colors = listOf(Color.Transparent, Color.Black.copy(alpha = 0.7f)),
                        startY = 0.5f
                    )
                )
        )
        
        // Title & Artist
        Column(
            modifier = Modifier
                .align(Alignment.BottomStart)
                .padding(16.dp)
        ) {
            Text(album.name, style = MaterialTheme.typography.titleMedium)
            Text(album.artist, style = MaterialTheme.typography.bodyMedium, color = onSurfaceVariant)
        }
        
        // Floating play button
        SunnyPlayButton(
            onClick = onPlayClick,
            modifier = Modifier.align(Alignment.BottomEnd).offset(x = 8.dp, y = 8.dp)
        )
    }
}
```

### 4. Track List Item

```kotlin
@Composable
fun TrackListItem(
    track: Track,
    isPlaying: Boolean,
    onClick: () -> Unit,
    onFavoriteClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(12.dp))
            .background(if (isPlaying) primaryContainer.copy(alpha = 0.2f) else Color.Transparent)
            .clickable(onClick = onClick)
            .padding(12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        // Cover art (small)
        AsyncImage(
            model = track.coverUrl,
            contentDescription = null,
            modifier = Modifier
                .size(48.dp)
                .clip(RoundedCornerShape(8.dp))
        )
        
        Spacer(Modifier.width(12.dp))
        
        // Track info
        Column(modifier = Modifier.weight(1f)) {
            Text(
                track.title,
                style = MaterialTheme.typography.titleMedium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
            Text(
                track.artist,
                style = MaterialTheme.typography.bodyMedium,
                color = onSurfaceVariant,
                maxLines = 1
            )
        }
        
        // Duration
        Text(
            track.formattedDuration,
            style = MaterialTheme.typography.labelMedium,
            color = onSurfaceVariant
        )
        
        // Favorite button
        IconButton(onClick = onFavoriteClick) {
            Icon(
                if (track.isFavorite) Icons.Filled.Favorite else Icons.Outlined.FavoriteBorder,
                contentDescription = "Favorite",
                tint = if (track.isFavorite) tertiary else onSurfaceVariant
            )
        }
    }
}
```

---

## Screens & Navigation

### Desktop → Mobile Screen Mapping

| Desktop View | Mobile Screen | Description |
|--------------|---------------|-------------|
| `TrackList` | `LibraryScreen` | All songs with search/filter |
| `AlbumGrid` | `AlbumsScreen` / `AlbumDetailScreen` | Album grid + detail view |
| `ArtistList` | `ArtistsScreen` / `ArtistDetailScreen` | Artist list + detail view |
| `FavoritesView` | `FavoritesScreen` | Liked tracks |
| `StatisticsPage` | `StatsScreen` | Listening statistics |
| `SettingsPage` | `SettingsScreen` | App preferences |
| `ImmersiveView` | `NowPlayingScreen` | Full-screen player |
| `PlayerBar` | `MiniPlayer` + `NowPlayingScreen` | Bottom bar + expandable |
| `LyricsPanel` | `LyricsScreen` or embedded in NowPlaying | Synced lyrics |
| `YouTubeMusic` | *(optional)* | YouTube Music integration |
| `TorrentManager` | *(optional)* | Torrent downloads |

### Mobile-Specific Screens

| Screen | Purpose |
|--------|---------|
| `DiscoveryScreen` | mDNS device discovery, connect to desktop |
| `QrScannerScreen` | Scan QR code to pair with desktop |
| `ConnectionScreen` | Show connection status, reconnect options |

### Navigation Architecture

```kotlin
sealed class Screen(val route: String) {
    // Bottom Nav
    object Library : Screen("library")
    object Albums : Screen("albums")
    object Artists : Screen("artists")
    object Favorites : Screen("favorites")
    object Settings : Screen("settings")
    
    // Detail screens
    data class AlbumDetail(val albumId: String) : Screen("album/{albumId}")
    data class ArtistDetail(val artistId: String) : Screen("artist/{artistId}")
    
    // Player
    object NowPlaying : Screen("now-playing")
    
    // Connection
    object Discovery : Screen("discovery")
    object QrScanner : Screen("qr-scanner")
}
```

### Bottom Navigation

```kotlin
@Composable
fun VibeOnBottomNav(
    currentRoute: String,
    onNavigate: (Screen) -> Unit
) {
    NavigationBar(
        containerColor = surfaceContainer,
        tonalElevation = 0.dp
    ) {
        NavigationBarItem(
            selected = currentRoute == Screen.Library.route,
            onClick = { onNavigate(Screen.Library) },
            icon = { VibeOnIcon(VibeOnIcons.Home, size = 28.dp) },
            label = { Text("Songs") }
        )
        NavigationBarItem(
            selected = currentRoute == Screen.Albums.route,
            onClick = { onNavigate(Screen.Albums) },
            icon = { VibeOnIcon(VibeOnIcons.Album, size = 28.dp) },
            label = { Text("Albums") }
        )
        NavigationBarItem(
            selected = currentRoute == Screen.Artists.route,
            onClick = { onNavigate(Screen.Artists) },
            icon = { VibeOnIcon(VibeOnIcons.Artist, size = 28.dp) },
            label = { Text("Artists") }
        )
        NavigationBarItem(
            selected = currentRoute == Screen.Favorites.route,
            onClick = { onNavigate(Screen.Favorites) },
            icon = { VibeOnIcon(VibeOnIcons.Heart, size = 28.dp) },
            label = { Text("Favorites") }
        )
    }
}
```

---

## Special UI Elements

### 1. Mini Player (Bottom Bar)

Persistent bottom bar showing current track, expandable to full Now Playing:

```kotlin
@Composable
fun MiniPlayer(
    track: Track?,
    isPlaying: Boolean,
    progress: Float,
    onPlayPause: () -> Unit,
    onExpand: () -> Unit,
    modifier: Modifier = Modifier
) {
    Surface(
        modifier = modifier
            .fillMaxWidth()
            .height(64.dp)
            .clickable(onClick = onExpand),
        color = surfaceContainer,
        shape = RoundedCornerShape(topStart = 16.dp, topEnd = 16.dp)
    ) {
        Column {
            // Progress indicator (thin line at top)
            LinearProgressIndicator(
                progress = progress,
                modifier = Modifier.fillMaxWidth().height(2.dp),
                color = primary,
                trackColor = Color.Transparent
            )
            
            Row(
                modifier = Modifier.fillMaxSize().padding(horizontal = 12.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                // Album art
                AsyncImage(
                    model = track?.coverUrl,
                    modifier = Modifier.size(48.dp).clip(RoundedCornerShape(8.dp))
                )
                
                Spacer(Modifier.width(12.dp))
                
                // Track info (marquee if too long)
                Column(modifier = Modifier.weight(1f)) {
                    MarqueeText(track?.title ?: "Not Playing", style = titleMedium)
                    Text(track?.artist ?: "", style = bodyMedium, color = onSurfaceVariant)
                }
                
                // Play/Pause button
                IconButton(onClick = onPlayPause) {
                    Icon(
                        if (isPlaying) Icons.Filled.Pause else Icons.Filled.PlayArrow,
                        contentDescription = if (isPlaying) "Pause" else "Play",
                        modifier = Modifier.size(32.dp)
                    )
                }
            }
        }
    }
}
```

### 2. Ambient Background

Blurred, color-extracted background from album art:

```kotlin
@Composable
fun AmbientBackground(
    imageUrl: String?,
    modifier: Modifier = Modifier
) {
    Box(modifier = modifier.fillMaxSize()) {
        AsyncImage(
            model = imageUrl,
            contentDescription = null,
            modifier = Modifier
                .fillMaxSize()
                .blur(100.dp)
                .alpha(0.3f),
            contentScale = ContentScale.Crop
        )
        
        // Gradient overlay for readability
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(
                    Brush.verticalGradient(
                        colors = listOf(
                            surface.copy(alpha = 0.8f),
                            surface.copy(alpha = 0.95f)
                        )
                    )
                )
        )
    }
}
```

### 3. Wavy Separator

Organic separator line used between sections:

```kotlin
@Composable
fun WavySeparator(
    modifier: Modifier = Modifier,
    color: Color = outlineVariant
) {
    Canvas(modifier = modifier.fillMaxWidth().height(8.dp)) {
        val path = Path().apply {
            moveTo(0f, size.height / 2)
            for (x in 0..size.width.toInt() step 4) {
                val y = size.height / 2 + sin(x * 0.1f) * 2
                lineTo(x.toFloat(), y)
            }
        }
        drawPath(path, color = color, style = Stroke(1.dp.toPx()))
    }
}
```

### 4. Circular Progress Ring (Now Playing)

```kotlin
@Composable
fun ProgressRing(
    progress: Float,
    modifier: Modifier = Modifier,
    strokeWidth: Dp = 4.dp,
    color: Color = primary
) {
    Canvas(modifier = modifier) {
        val sweepAngle = progress * 360f
        
        // Background ring
        drawArc(
            color = color.copy(alpha = 0.2f),
            startAngle = -90f,
            sweepAngle = 360f,
            useCenter = false,
            style = Stroke(strokeWidth.toPx(), cap = StrokeCap.Round)
        )
        
        // Progress ring
        drawArc(
            color = color,
            startAngle = -90f,
            sweepAngle = sweepAngle,
            useCenter = false,
            style = Stroke(strokeWidth.toPx(), cap = StrokeCap.Round)
        )
    }
}
```

---

## Implementation Checklist

### Required Screens
- [ ] `DiscoveryScreen` - mDNS discovery + QR scan
- [ ] `LibraryScreen` - Track list with search
- [ ] `AlbumsScreen` - Album grid
- [ ] `AlbumDetailScreen` - Album tracks
- [ ] `ArtistsScreen` - Artist list
- [ ] `ArtistDetailScreen` - Artist albums/tracks
- [ ] `FavoritesScreen` - Liked tracks
- [ ] `NowPlayingScreen` - Full-screen player with lyrics
- [ ] `SettingsScreen` - Preferences

### Required Components
- [ ] `MiniPlayer` - Bottom bar player
- [ ] `SquigglySlider` - Animated progress bar
- [ ] `AlbumCard` - Grid item with play button
- [ ] `TrackListItem` - Track row
- [ ] `ArtistCard` - Artist grid item
- [ ] `AmbientBackground` - Blurred album art background
- [ ] `WavySeparator` - Section divider
- [ ] `ScallopedButton` - Organic skip buttons

### Required Icons
- [ ] All navigation icons
- [ ] All playback control icons
- [ ] Heart (filled + outline)
- [ ] Queue, lyrics, volume icons

---

## Resources

- **Outfit Font:** https://fonts.google.com/specimen/Outfit
- **Material Color Utilities:** https://github.com/nicklockwood/SwiftFormat/blob/main/ColorUtils.swift (or Android equivalent)
- **M3 Expressive Guidelines:** https://m3.material.io/styles/motion/overview

---

*This document should be kept in sync between desktop (`vibe-on`) and mobile (`vibe-mobile`) repositories.*
