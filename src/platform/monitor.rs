//! Multi-monitor workspace detection and window positioning.

use nsh_core::desktop::WindowFrame;
use nsh_core::element::desktop::{displays, set_frame, window_state};
use nsh_core::geometry::Points;

use crate::core::settings::WindowPositionMode;

/// Position the main window on screen according to the configured mode.
pub fn position_window(mode: WindowPositionMode, width: f32, height: f32) {
    let all_displays = displays();
    let primary = all_displays.first();

    let (screen_w, screen_h) = if let Some(d) = primary {
        (d.bounds.width.0, d.bounds.height.0)
    } else {
        (1920.0, 1080.0)
    };

    match mode {
        WindowPositionMode::Center => {
            let x = ((screen_w - width) / 2.0).max(0.0);
            let y = ((screen_h - height) / 2.0).max(0.0);
            set_frame(WindowFrame {
                x: Some(Points(x)),
                y: Some(Points(y)),
                width: Some(Points(width)),
                height: Some(Points(height)),
                animated: Some(false),
            });
        }
        WindowPositionMode::FollowMouse | WindowPositionMode::Remember => {
            // If window state is available, keep position or set default
            if let Some(state) = window_state() {
                set_frame(WindowFrame {
                    x: Some(state.bounds.x),
                    y: Some(state.bounds.y),
                    width: Some(Points(width)),
                    height: Some(Points(height)),
                    animated: Some(false),
                });
            } else {
                let x = ((screen_w - width) / 2.0).max(0.0);
                let y = ((screen_h - height) / 2.0).max(0.0);
                set_frame(WindowFrame {
                    x: Some(Points(x)),
                    y: Some(Points(y)),
                    width: Some(Points(width)),
                    height: Some(Points(height)),
                    animated: Some(false),
                });
            }
        }
    }
}
