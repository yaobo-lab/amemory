//! System tray notification icon and context menu.

/// Configure desktop application menus.
pub fn configure_desktop_menus() {
    // 仅在 macOS 下配置系统顶部全局菜单栏 (Apple Bar)；
    // Windows / Linux 下不挂载窗口级菜单栏，以保持纯净的一体化无边框桌面卡片风格，避免多出一行冗余菜单栏目。
    #[cfg(target_os = "macos")]
    {
        use crate::core::i18n_keys::I18nKey;
        use nsh_core::element::desktop::{MenuItem, order_front, set_application_menu};

        let file_menu = MenuItem::submenu(
            I18nKey::AppName.text(),
            vec![
                MenuItem::new(I18nKey::TrayShow.text()).on_select(order_front),
                MenuItem::separator(),
                MenuItem::new(I18nKey::TrayQuit.text()).on_select(|| std::process::exit(0)),
            ],
        );

        set_application_menu(vec![file_menu]);
    }

    #[cfg(target_os = "windows")]
    {
        nsh_core::element::desktop::set_application_menu(None);
        nsh_core::element::desktop::set_menu_bar_visibility(
            nsh_core::desktop::MenuBarMode::AutoHide,
        );

        let hwnd = crate::platform::window_frame::get_app_hwnd();
        if hwnd != 0 {
            use windows_sys::Win32::UI::WindowsAndMessaging::{DrawMenuBar, SetMenu};
            unsafe {
                SetMenu(hwnd, 0);
                let _ = DrawMenuBar(hwnd);
            }
        }
    }
}
