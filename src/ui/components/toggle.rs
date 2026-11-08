//! Reusable animated toggle switch component.

use nsh_core::prelude::*;

use crate::ui::theme::ClippiTheme;

/// Render a 32x18px pill toggle switch with 14px knob using Signal.
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
                .w(px(14.0))
                .h(px(14.0))
                .rounded(px(7.0))
                .bg(rgb(0xffffff))
                .style_with(move |s| {
                    if checked.get() {
                        s.ml(px(14.0))
                    } else {
                        s.ml(px(0.0))
                    }
                }),
        )
}

/// Render a 32x18px pill toggle switch with boolean value and callback, dynamically styled with store theme.
pub fn render_store_toggle(
    store: crate::state::app::AppStore,
    checked: bool,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    let is_on = signal(checked);

    view()
        .w(px(32.0))
        .h(px(18.0))
        .rounded(px(9.0))
        .bg(move || {
            if is_on.get() {
                store.theme().accent
            } else {
                store.theme().divider
            }
        })
        .cursor_pointer()
        .items_center()
        .p(px(2.0))
        .on_press(move || {
            let next = !is_on.get();
            is_on.set(next);
            on_change(next);
        })
        .child(
            view()
                .w(px(14.0))
                .h(px(14.0))
                .rounded(px(7.0))
                .bg(rgb(0xffffff))
                .style_with(move |s| {
                    if is_on.get() {
                        s.ml(px(14.0))
                    } else {
                        s.ml(px(0.0))
                    }
                }),
        )
}

/// Render a 32x18px pill toggle switch with boolean value and callback.
pub fn render_toggle(
    checked: bool,
    theme: ClippiTheme,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    let accent = theme.accent;
    let divider = theme.divider;
    let is_on = signal(checked);

    view()
        .w(px(32.0))
        .h(px(18.0))
        .rounded(px(9.0))
        .bg(move || if is_on.get() { accent } else { divider })
        .cursor_pointer()
        .items_center()
        .p(px(2.0))
        .on_press(move || {
            let next = !is_on.get();
            is_on.set(next);
            on_change(next);
        })
        .child(
            view()
                .w(px(14.0))
                .h(px(14.0))
                .rounded(px(7.0))
                .bg(rgb(0xffffff))
                .style_with(move |s| {
                    if is_on.get() {
                        s.ml(px(14.0))
                    } else {
                        s.ml(px(0.0))
                    }
                }),
        )
}
