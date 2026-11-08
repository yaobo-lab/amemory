//! Stepped slider component for numeric values and scales.

use nsh_core::prelude::*;

use crate::state::app::AppStore;

/// Render a stepped numeric slider with step buttons and value display.
pub fn stepped_slider(
    min: f32,
    max: f32,
    step: f32,
    current: f32,
    store: AppStore,
    on_change: impl Fn(f32) + Copy + 'static,
) -> impl IntoElement {
    let val_sig = signal(current);

    let decrease = move || {
        let cur = val_sig.get();
        let next = ((cur - step) * 10.0).round() / 10.0;
        let next = next.max(min);
        val_sig.set(next);
        on_change(next);
    };

    let increase = move || {
        let cur = val_sig.get();
        let next = ((cur + step) * 10.0).round() / 10.0;
        let next = next.min(max);
        val_sig.set(next);
        on_change(next);
    };

    view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            view()
                .w(px(22.0))
                .h(px(22.0))
                .rounded(px(5.0))
                .bg(move || store.theme().divider)
                .items_center()
                .justify_center()
                .cursor_pointer()
                .on_press(decrease)
                .child(text("-").text_color(move || store.theme().text_1).text_xs()),
        )
        .child(
            view()
                .w(px(80.0))
                .h(px(6.0))
                .rounded(px(3.0))
                .bg(move || store.theme().divider)
                .child(
                    view()
                        .w(move || {
                            let pct = ((val_sig.get() - min) / (max - min)).clamp(0.0, 1.0);
                            px(80.0 * pct)
                        })
                        .h_full()
                        .rounded(px(3.0))
                        .bg(move || store.theme().accent),
                ),
        )
        .child(
            view()
                .w(px(22.0))
                .h(px(22.0))
                .rounded(px(5.0))
                .bg(move || store.theme().divider)
                .items_center()
                .justify_center()
                .cursor_pointer()
                .on_press(increase)
                .child(text("+").text_color(move || store.theme().text_1).text_xs()),
        )
        .child(
            text(move || format!("{:.1}x", val_sig.get()))
                .text_color(move || store.theme().text_1)
                .text_xs(),
        )
}
