//! Stepped slider component for numeric values and scales.

use nsh_core::prelude::*;

use crate::ui::theme::ClippiTheme;

/// Render a stepped numeric slider with step buttons and value display.
pub fn stepped_slider(
    min: f32,
    max: f32,
    step: f32,
    current: f32,
    theme: ClippiTheme,
    on_change: impl Fn(f32) + Copy + 'static,
) -> impl IntoElement {
    let accent = theme.accent;
    let divider = theme.divider;
    let text_1 = theme.text_1;
    let val_sig = signal(current);

    let decrease = move || {
        let cur = val_sig.get();
        let next = (cur - step).max(min);
        val_sig.set(next);
        on_change(next);
    };

    let increase = move || {
        let cur = val_sig.get();
        let next = (cur + step).min(max);
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
                .bg(divider)
                .items_center()
                .justify_center()
                .cursor_pointer()
                .on_press(decrease)
                .child(text("-").text_color(text_1).text_xs()),
        )
        .child(
            view()
                .w(px(80.0))
                .h(px(6.0))
                .rounded(px(3.0))
                .bg(divider)
                .child(
                    view()
                        .w(move || {
                            let pct = ((val_sig.get() - min) / (max - min)).clamp(0.0, 1.0);
                            px(80.0 * pct)
                        })
                        .h_full()
                        .rounded(px(3.0))
                        .bg(accent),
                ),
        )
        .child(
            view()
                .w(px(22.0))
                .h(px(22.0))
                .rounded(px(5.0))
                .bg(divider)
                .items_center()
                .justify_center()
                .cursor_pointer()
                .on_press(increase)
                .child(text("+").text_color(text_1).text_xs()),
        )
        .child(
            text(move || format!("{:.1}x", val_sig.get()))
                .text_color(text_1)
                .text_xs(),
        )
}
