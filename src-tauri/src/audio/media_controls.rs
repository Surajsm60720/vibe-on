//! Windows System Media Transport Controls integration using souvlaki
//!
//! Service pattern: Spawns a dedicated thread to manage media controls.
//! Uses 'windows' crate for message pumping on the background thread.

#[cfg(target_os = "windows")]
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use std::sync::mpsc::{channel, Sender};
#[cfg(target_os = "windows")]
use std::sync::mpsc::{Receiver, TryRecvError};
#[cfg(target_os = "windows")]
use std::thread;
use tauri::AppHandle;
#[cfg(target_os = "windows")]
use tauri::Emitter;

#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE, WM_QUIT,
};

#[derive(Debug)]
#[allow(dead_code)] // Variants unused on non-Windows
pub enum MediaCmd {
    SetMetadata {
        title: String,
        artist: String,
        album: String,
    },
    SetPlaying,
    SetPaused,
    SetStopped,
    Shutdown,
}

#[allow(dead_code)] // Struct unused on non-Windows
pub struct MediaControlService;

#[allow(dead_code)] // Methods unused on non-Windows
impl MediaControlService {
    #[cfg(target_os = "windows")]
    pub fn start(app: AppHandle, hwnd: isize) -> Sender<MediaCmd> {
        let (tx, rx) = channel::<MediaCmd>();

        thread::spawn(move || {
            if let Err(e) = Self::run_loop(app, hwnd, rx) {
                eprintln!("[MediaControls] Thread error: {}", e);
            }
        });

        tx
    }

    #[cfg(not(target_os = "windows"))]
    pub fn start(_app: AppHandle, _hwnd: isize) -> Sender<MediaCmd> {
        let (tx, _) = channel::<MediaCmd>();
        tx
    }

    #[cfg(target_os = "windows")]
    fn run_loop(app: AppHandle, _hwnd: isize, rx: Receiver<MediaCmd>) -> Result<(), String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }

        // Create a dummy window for the background thread to receive messages
        // This is necessary because souvlaki requires an HWND on Windows,
        // and using the main window from a background thread causes thread-safety issues.
        let dummy_hwnd = unsafe {
            use windows::Win32::System::LibraryLoader::GetModuleHandleW;
            use windows::Win32::UI::WindowsAndMessaging::{
                CreateWindowExW, RegisterClassW, CS_OWNDC, CW_USEDEFAULT, WNDCLASSW,
                WS_OVERLAPPEDWINDOW,
            };

            let instance = GetModuleHandleW(None).unwrap_or_default();
            let class_name = windows::core::w!("VibeOnMediaDummy");

            use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

            unsafe extern "system" fn media_wnd_proc(
                hwnd: HWND,
                msg: u32,
                wparam: WPARAM,
                lparam: LPARAM,
            ) -> LRESULT {
                windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(media_wnd_proc),
                hInstance: std::mem::transmute(instance),
                lpszClassName: class_name,
                style: CS_OWNDC,
                ..Default::default()
            };

            RegisterClassW(&wnd_class);

            CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                class_name,
                windows::core::w!("VibeOnMediaDummy"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,                                // Parent HWND
                None,                                // Menu HMENU
                Some(std::mem::transmute(instance)), // hInstance
                None,
            )
        };

        if dummy_hwnd.0 == 0 {
            return Err("Failed to create dummy window for media controls".to_string());
        }

        let config = PlatformConfig {
            dbus_name: "vibeon",  // Must be alphanumeric for MPRIS (no underscores/hyphens)
            display_name: "VIBE-ON!",
            hwnd: Some(dummy_hwnd.0 as *mut std::ffi::c_void),
        };

        let mut controls = MediaControls::new(config)
            .map_err(|e| format!("Failed to create media controls: {:?}", e))?;

        // Attach event handler
        let app_clone = app.clone();
        controls
            .attach(move |event: MediaControlEvent| {
                let event_name = match event {
                    MediaControlEvent::Play | MediaControlEvent::Toggle => "media:play",
                    MediaControlEvent::Pause => "media:pause",
                    MediaControlEvent::Next => "media:next",
                    MediaControlEvent::Previous => "media:prev",
                    MediaControlEvent::Stop => "media:stop",
                    _ => return,
                };

                // Emit event to frontend
                let _ = app_clone.emit(event_name, ());
            })
            .map_err(|e| format!("Failed to attach event handler: {:?}", e))?;

        // Loop handling commands AND pumping Windows messages
        loop {
            // 1. Process all pending commands from channel non-blocking
            loop {
                match rx.try_recv() {
                    Ok(MediaCmd::SetMetadata {
                        title,
                        artist,
                        album,
                    }) => {
                        let _ = controls.set_metadata(MediaMetadata {
                            title: Some(&title),
                            artist: Some(&artist),
                            album: Some(&album),
                            ..Default::default()
                        });
                    }
                    Ok(MediaCmd::SetPlaying) => {
                        let _ = controls.set_playback(MediaPlayback::Playing { progress: None });
                        crate::taskbar_controls::update_play_status(true);
                    }
                    Ok(MediaCmd::SetPaused) => {
                        let _ = controls.set_playback(MediaPlayback::Paused { progress: None });
                        crate::taskbar_controls::update_play_status(false);
                    }
                    Ok(MediaCmd::SetStopped) => {
                        let _ = controls.set_playback(MediaPlayback::Stopped);
                        crate::taskbar_controls::update_play_status(false);
                    }
                    Ok(MediaCmd::Shutdown) => return Ok(()),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return Ok(()),
                }
            }

            // 2. Pump Windows Messages (Required for souvlaki's internal window to receive events)
            unsafe {
                let mut msg = MSG::default();
                // PeekMessage with HWND(0) / None checks all windows on this thread
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        return Ok(());
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            // 3. Sleep briefly to prevent high CPU usage
            // (A more robust solution would use MsgWaitForMultipleObjects, but this is sufficient for metadata updates)
            thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}
