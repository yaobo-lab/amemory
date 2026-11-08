//! Global summon hotkey registration and window summon activation.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

static HOTKEY_INITIALIZED: AtomicBool = AtomicBool::new(false);
static HOTKEY_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static CURRENT_HOTKEY: OnceLock<Mutex<String>> = OnceLock::new();
static CALLBACK: OnceLock<Mutex<Box<dyn FnMut() + Send + 'static>>> = OnceLock::new();

const WM_HOTKEY_RELOAD: u32 = 0x0400 + 1; // WM_USER + 1

fn current_hotkey_lock() -> &'static Mutex<String> {
    CURRENT_HOTKEY.get_or_init(|| Mutex::new("Alt+Space".to_string()))
}

/// Register the global summon hotkey (default: Alt+Space / Ctrl+Shift+Space / Alt+M).
pub fn register_global_hotkey(on_press: impl FnMut() + Send + 'static) {
    let _ = CALLBACK.set(Mutex::new(Box::new(on_press)));

    if HOTKEY_INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }

    #[cfg(target_os = "windows")]
    {
        std::thread::Builder::new()
            .name("amemory-hotkey".to_string())
            .spawn(move || {
                use windows_sys::Win32::System::Threading::GetCurrentThreadId;
                use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                    RegisterHotKey, UnregisterHotKey,
                };
                use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

                let tid = unsafe { GetCurrentThreadId() };
                HOTKEY_THREAD_ID.store(tid, Ordering::SeqCst);

                let initial_def = current_hotkey_lock()
                    .lock()
                    .map_or_else(|_| "Alt+Space".to_string(), |g| g.clone());
                let (mut mods, mut vk) = parse_hotkey(&initial_def);

                unsafe {
                    let _ = RegisterHotKey(0, 1001, mods, vk);
                    let mut msg: MSG = std::mem::zeroed();
                    while GetMessageW(&raw mut msg, 0, 0, 0) > 0 {
                        if msg.message == WM_HOTKEY {
                            summon_app_window();
                            if let Some(cb_mtx) = CALLBACK.get()
                                && let Ok(mut cb) = cb_mtx.lock()
                            {
                                cb();
                            }
                        } else if msg.message == WM_HOTKEY_RELOAD {
                            let _ = UnregisterHotKey(0, 1001);
                            let new_def = current_hotkey_lock()
                                .lock()
                                .map_or_else(|_| "Alt+Space".to_string(), |g| g.clone());
                            let (new_mods, new_vk) = parse_hotkey(&new_def);
                            mods = new_mods;
                            vk = new_vk;
                            let _ = RegisterHotKey(0, 1001, mods, vk);
                        }
                    }
                }
            })
            .expect("failed to spawn global hotkey thread");
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = on_press;
    }
}

/// Dynamically update the global hotkey configuration at runtime.
pub fn update_global_hotkey(new_hotkey: &str) {
    if let Ok(mut lock) = current_hotkey_lock().lock() {
        *lock = new_hotkey.to_string();
    }

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW;

        let tid = HOTKEY_THREAD_ID.load(Ordering::SeqCst);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_HOTKEY_RELOAD, 0, 0);
            }
        }
    }
}

/// Summon and activate the application window to foreground.
pub fn summon_app_window() {
    let settings = crate::core::settings::GuiSettings::load();
    crate::platform::monitor::position_window(
        settings.window_position_mode,
        crate::platform::window_frame::DEFAULT_WINDOW_W,
        crate::platform::window_frame::DEFAULT_WINDOW_H,
    );

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            FindWindowW, IsIconic, SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow,
        };
        let title: Vec<u16> = "amemory\0".encode_utf16().collect();
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
            if hwnd != 0 {
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                } else {
                    ShowWindow(hwnd, SW_SHOW);
                }
                SetForegroundWindow(hwnd);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn parse_hotkey(hotkey_str: &str) -> (u32, u32) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
    };

    let lower = hotkey_str.to_lowercase();
    let mut mods = MOD_NOREPEAT;
    if lower.contains("alt") {
        mods |= MOD_ALT;
    }
    if lower.contains("ctrl") || lower.contains("control") {
        mods |= MOD_CONTROL;
    }
    if lower.contains("shift") {
        mods |= MOD_SHIFT;
    }
    if lower.contains("win") {
        mods |= MOD_WIN;
    }

    let vk = if lower.contains("space") {
        0x20
    } else if lower.ends_with('m') {
        0x4D
    } else if lower.ends_with('v') {
        0x56
    } else {
        0x20
    };
    (mods, vk)
}
