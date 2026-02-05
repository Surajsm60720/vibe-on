# Android Integration Guide: Using the Streaming Feature

## Quick Start

### 1. Initialize Streaming Client in UI

In your Compose screen, initialize both WebSocket and HTTP clients:

```kotlin
@Composable
fun LibraryBrowseScreen(
    viewModel: BrowseViewModel = hiltViewModel()
) {
    val musicStreamClient = remember { 
        MusicStreamClient(host = "192.168.1.100", port = 5000)
    }
    val webSocketClient = remember { WebSocketClient() }
    
    LaunchedEffect(Unit) {
        // Connect to desktop
        webSocketClient.connect(
            host = "192.168.1.100", 
            port = 5443, 
            clientName = "MyPhone"
        )
        
        // Load library
        musicStreamClient.browseLibrary(offset = 0, limit = 50)
    }
    
    // Your UI here
}
```

### 2. Display Library Tracks

```kotlin
@Composable
fun TrackListItem(track: TrackInfo, onPlayClick: (String) -> Unit) {
    Card(modifier = Modifier
        .fillMaxWidth()
        .padding(8.dp)
        .clickable { onPlayClick(track.path) }
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Cover Art
            if (track.coverUrl != null) {
                AsyncImage(
                    model = track.coverUrl,
                    contentDescription = "Album cover",
                    modifier = Modifier
                        .size(56.dp)
                        .clip(RoundedCornerShape(4.dp)),
                    contentScale = ContentScale.Crop
                )
            }
            
            Spacer(modifier = Modifier.width(16.dp))
            
            // Track Info
            Column(modifier = Modifier.weight(1f)) {
                Text(track.title, style = MaterialTheme.typography.titleSmall)
                Text(track.artist, style = MaterialTheme.typography.bodySmall)
                Text(
                    "${track.duration.toInt() / 60}:${(track.duration.toInt() % 60).toString().padStart(2, '0')}",
                    style = MaterialTheme.typography.labelSmall
                )
            }
        }
    }
}
```

### 3. Play a Track

```kotlin
fun playTrack(
    streamClient: MusicStreamClient,
    wsClient: WebSocketClient,
    track: TrackInfo,
    mediaController: MediaController
) {
    // Get stream URL
    val streamUrl = streamClient.getStreamUrl(track.path)
    
    // Create media item
    val mediaItem = MediaItem.Builder()
        .setUri(streamUrl)
        .setMediaMetadata(
            MediaMetadata.Builder()
                .setTitle(track.title)
                .setArtist(track.artist)
                .setAlbumTitle(track.album)
                .setArtworkUri(Uri.parse(streamClient.getCoverUrl(track.path)))
                .build()
        )
        .build()
    
    // Send command to desktop via WebSocket
    wsClient.sendPlayTrack(track.path)
    
    // Also play on mobile
    mediaController.apply {
        setMediaItem(mediaItem)
        prepare()
        play()
    }
}
```

### 4. Handle Search

```kotlin
@Composable
fun SearchScreen(
    streamClient: MusicStreamClient,
    onTrackSelected: (TrackInfo) -> Unit
) {
    var searchQuery by remember { mutableStateOf("") }
    var searchResults by remember { mutableStateOf<List<TrackInfo>>(emptyList()) }
    var isLoading by remember { mutableStateOf(false) }
    
    Column {
        TextField(
            value = searchQuery,
            onValueChange = { 
                searchQuery = it
                // Debounce search
                viewModelScope.launch {
                    delay(500)
                    if (searchQuery.isNotEmpty()) {
                        isLoading = true
                        searchResults = streamClient.searchLibrary(searchQuery) ?: emptyList()
                        isLoading = false
                    }
                }
            },
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            label = { Text("Search songs...") }
        )
        
        if (isLoading) {
            CircularProgressIndicator()
        } else {
            LazyColumn {
                items(searchResults) { track ->
                    TrackListItem(
                        track = track,
                        onPlayClick = { onTrackSelected(track) }
                    )
                }
            }
        }
    }
}
```

### 5. Queue Display

```kotlin
@Composable
fun QueueScreen(webSocketClient: WebSocketClient) {
    val queue by webSocketClient.queue.collectAsState()
    val currentIndex by webSocketClient.currentIndex.collectAsState()
    
    LazyColumn {
        itemsIndexed(queue) { index, item ->
            val isCurrentTrack = index == currentIndex
            TrackListItem(
                track = TrackInfo(
                    path = item.path,
                    title = item.title,
                    artist = item.artist,
                    album = item.album,
                    duration = item.duration
                ),
                onPlayClick = { },
                modifier = Modifier.background(
                    if (isCurrentTrack) Color.Blue.copy(alpha = 0.1f) 
                    else Color.Transparent
                )
            )
        }
    }
}
```

### 6. Playback Controls

```kotlin
@Composable
fun PlaybackControlsScreen(
    webSocketClient: WebSocketClient,
    mediaController: MediaController?
) {
    val isPlaying by webSocketClient.isPlaying.collectAsState()
    val currentTrack by webSocketClient.currentTrack.collectAsState()
    
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        // Track Info
        Text(currentTrack.title, style = MaterialTheme.typography.headlineSmall)
        Text(currentTrack.artist, style = MaterialTheme.typography.bodyLarge)
        
        Spacer(modifier = Modifier.height(24.dp))
        
        // Controls
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 32.dp),
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalAlignment = Alignment.CenterVertically
        ) {
            IconButton(onClick = { webSocketClient.sendPrevious() }) {
                Icon(Icons.Default.SkipPrevious, contentDescription = "Previous")
            }
            
            IconButton(
                onClick = {
                    if (isPlaying) {
                        webSocketClient.sendPause()
                        mediaController?.pause()
                    } else {
                        webSocketClient.sendPlay()
                        mediaController?.play()
                    }
                },
                modifier = Modifier.size(64.dp)
            ) {
                Icon(
                    if (isPlaying) Icons.Default.Pause else Icons.Default.PlayArrow,
                    contentDescription = if (isPlaying) "Pause" else "Play",
                    modifier = Modifier.size(40.dp)
                )
            }
            
            IconButton(onClick = { webSocketClient.sendNext() }) {
                Icon(Icons.Default.SkipNext, contentDescription = "Next")
            }
        }
    }
}
```

## Integration with Existing UI

### ConnectionViewModel Update

Add streaming support to your existing connection view model:

```kotlin
class ConnectionViewModel(
    private val discoveryRepository: DiscoveryRepository
) : ViewModel() {
    private val _streamClient = MutableLiveData<MusicStreamClient?>()
    val streamClient: LiveData<MusicStreamClient?> = _streamClient
    
    private val _library = MutableLiveData<List<TrackInfo>>(emptyList())
    val library: LiveData<List<TrackInfo>> = _library
    
    fun connectToDevice(device: DiscoveredDevice) {
        viewModelScope.launch {
            // Connect WebSocket as before
            connectionViewModel.connectToDevice(device)
            
            // Initialize streaming client
            val streamClient = MusicStreamClient(device.host, device.port)
            _streamClient.value = streamClient
            
            // Load library
            val library = streamClient.browseLibrary()
            _library.value = library?.tracks ?: emptyList()
        }
    }
}
```

## Playing to ExoPlayer

### Using P2PDataSource (Existing)

If you're using the existing P2PDataSource:

```kotlin
class P2PDataSource(
    private val streamRepository: StreamRepository,
    private val scope: CoroutineScope
) : DataSource {
    // Existing implementation
}
```

### Using HTTP DataSource (Simpler)

For simpler HTTP streaming, ExoPlayer has built-in support:

```kotlin
val httpDataSourceFactory = HttpDataSource.Factory()
    .setUserAgent("vibe-on")
    .setConnectTimeoutMs(30000)
    .setReadTimeoutMs(30000)

val mediaSourceFactory = DefaultMediaSourceFactory(context)
    .setDataSourceFactory(httpDataSourceFactory)

val player = ExoPlayer.Builder(context)
    .setMediaSourceFactory(mediaSourceFactory)
    .build()

// Now you can directly play HTTP URLs:
val mediaItem = MediaItem.Builder()
    .setUri("http://192.168.1.100:5000/stream/encoded_path")
    .build()

player.setMediaItem(mediaItem)
player.prepare()
player.play()
```

## Handling Connection States

```kotlin
@Composable
fun StreamingUI(
    streamClient: MusicStreamClient?,
    webSocketClient: WebSocketClient?
) {
    when {
        streamClient == null || webSocketClient == null -> {
            Text("Connecting to server...")
        }
        !webSocketClient.isConnected.collectAsState().value -> {
            Text("WebSocket disconnected")
            Button(onClick = { 
                webSocketClient.disconnect()
                webSocketClient.connect("192.168.1.100", 5443)
            }) {
                Text("Reconnect")
            }
        }
        else -> {
            // Show library, playback controls, etc.
            LibraryBrowseScreen(streamClient, webSocketClient)
        }
    }
}
```

## Error Handling

```kotlin
val errorMessage by streamClient?.error?.collectAsState() ?: remember { mutableStateOf(null) }

if (errorMessage != null) {
    Snackbar(
        modifier = Modifier.padding(16.dp),
        action = {
            TextButton(onClick = { /* Clear error */ }) {
                Text("Dismiss")
            }
        }
    ) {
        Text(errorMessage)
    }
}
```

## Testing

### Local Testing
```kotlin
// In a test or debug activity
val streamClient = MusicStreamClient("10.0.0.2", 5000)
val library = streamClient.browseLibrary()
assert(library.tracks.isNotEmpty())
assert(library.total > 0)
```

### Real Device Testing
1. Connect PC and phone to same WiFi
2. Get PC's IP: `ipconfig` (Windows) or `ifconfig` (Linux/Mac)
3. Use IP in client initialization
4. Test discovery first to ensure mDNS works

## Dependencies

Make sure your `build.gradle` includes:

```gradle
dependencies {
    // Media3/ExoPlayer
    implementation 'androidx.media3:media3-exoplayer:1.1.1'
    implementation 'androidx.media3:media3-session:1.1.1'
    implementation 'androidx.media3:media3-datasource-okhttp:1.1.1'
    
    // HTTP Client
    implementation 'com.squareup.okhttp3:okhttp:4.11.0'
    
    // JSON
    implementation 'org.json:json:20230227'
    
    // Coroutines
    implementation 'org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3'
}
```

That's it! You now have a fully functional local music streaming system like Spotify but for your LAN.
