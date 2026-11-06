//! Stepped slider component for numeric values and scales.

use nsh_core::prelude::*;

use crate::ui::theme::ClippiTheme;

/// Render a stepped numeric slider with step buttons and value display.
pub fn stepped_slider(
    value: Signal<f32>,
    min: f32,
    max: f32,
    step: f32,
    display_suffix: &'static str,
    theme: ClippiTheme,
    on_change: impl Fn(f32) + Copy + 'static,
) -> impl IntoElement {
    let accent = theme.accent;
    let divider = theme.divider;
    let text_1 = theme.text_1;

    let decrease = move || {
        let cur = value.get_untracked();
        let next = (cur - step).max(min);
        on_change(next);
    };

    let increase = move || {
        let cur = value.get_untracked();
        let next = (cur + step).min(max);
        on_change(next);
    };

    view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            view()
                .size(px(22.0))
                .rounded(px(5.0))
                .bg(divider)
                .items_center()
                .justify_center()
                .cursor_pointer()
                .active(|s| s.opacity(0.7))
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
                        .h_full()
                        .rounded(px(3.0))
                        .bg(accent)
                        .style_with(move |s| {
                            let cur = value.get();
                            let pct = ((cur - min) / (max - min)).clamp(0.0, 1.0);
                            s.w(percentage(pct))
                        }),
                ),
        )
        .child(
            view()
                .size(px(22.0))
                .rounded(px(5.0))
                .bg(divider)
                .items_center()
                .justify_center()
                .cursor_pointer()
                .active(|s| s.opacity(0.7))
                .on_press(increase)
                .child(text("+").text_color(text_1).text_xs()),
        )
        .child(
            text(move || format!("{:.2}{display_suffix}", value.get()))
                .text_color(text_1)
                .text_xs(),
        )
}
