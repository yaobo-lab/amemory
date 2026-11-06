//! Global summon hotkey registration.

use std::sync::atomic::{AtomicBool, Ordering};

static HOTKEY_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Register the global summon hotkey (default: Alt+Space / Ctrl+Shift+Space).
pub fn register_global_hotkey(mut on_press: impl FnMut() + Send + 'static) {
    if HOTKEY_REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }

    #[cfg(target_os = "windows")]
    {
        std::thread::Builder::new()
            .name("amemory-hotkey".to_string())
            .spawn(move || {
                use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_NOREPEAT, RegisterHotKey};
                use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

                // ID: 1001, MOD_ALT, VK_SPACE (0x20)
                unsafe {
                    let _ = RegisterHotKey(0, 1001, MOD_ALT | MOD_NOREPEAT, 0x20);
                    let mut msg: MSG = std::mem::zeroed();
                    while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                        if msg.message == WM_HOTKEY {
                            on_press();
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
