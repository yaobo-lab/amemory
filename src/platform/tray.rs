//! System tray notification icon and context menu.

use nsh_core::element::desktop::{MenuItem, MenuRole, order_front, quit, set_application_menu};

use crate::core::i18n_keys::I18nKey;

/// Configure desktop application menus.
pub fn configure_desktop_menus() {
    let file_menu = MenuItem::submenu(
        I18nKey::AppName.text(),
        vec![
            MenuItem::new(I18nKey::TrayShow.text()).on_select(order_front),
            MenuItem::separator(),
            MenuItem::role(MenuRole::Quit).label(I18nKey::TrayQuit.text()).on_select(quit),
        ],
    );

    set_application_menu(vec![file_menu]);
}
