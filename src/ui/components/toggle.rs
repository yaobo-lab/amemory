//! Reusable animated toggle switch component.

use nsh_core::prelude::*;

use crate::ui::theme::ClippiTheme;

/// Render a 32x18px pill toggle switch with 14px knob.
pub fn toggle_switch(
    checked: Signal<bool>,
    theme: ClippiTheme,
    on_toggle: impl Fn() + 'static,
) -> impl IntoElement {
    let accent = theme.accent;
    let divider = theme.divider;

    view()
        .w(px(32.0))
        .h(px(18.0))
        .rounded(px(9.0))
        .bg(move || if checked.get() { accent } else { divider })
        .cursor_pointer()
        .items_center()
        .p(px(2.0))
        .on_press(on_toggle)
        .child(
            view()
                .size(px(14.0))
                .rounded(px(7.0))
                .bg(rgb(0xff, 0xff, 0xff))
                .style_with(move |s| {
                    if checked.get() {
                        s.ml(px(14.0))
                    } else {
                        s.ml(px(0.0))
                    }
                }),
        )
}
