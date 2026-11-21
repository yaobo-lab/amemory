//! Semi-transparent overlay backdrop for modal dialogs with entrance animations.

use nsh_core::prelude::*;

use crate::ui::theme::ClippiTheme;

/// Semi-transparent dark overlay mask with fade-in motion.
pub fn modal_overlay(on_click_outside: impl Fn() + 'static) -> impl IntoElement {
    let mut timing = KeyframeTiming::default();
    timing.duration_ms = 180.0;
    timing.easing = Easing::EaseOut;
    let backdrop_motion =
        motion().entering(MotionSpec::new(timing).values(MotionChannel::Opacity, [0.0, 1.0]));

    view()
        .absolute()
        .w_full()
        .h_full()
        .bg(rgba(0x00000066))
        .bind_motion(&backdrop_motion)
        .on_press(on_click_outside)
}

/// Modal overlay container wrapping dialog content in center with smooth pop-in animation.
pub fn modal_overlay_view(
    content: impl IntoElement + 'static,
    _theme: ClippiTheme,
    on_close: impl Fn() + 'static,
) -> impl IntoElement {
    let mut timing = KeyframeTiming::default();
    timing.duration_ms = 220.0;
    timing.easing = Easing::Bezier(0.16, 1.0, 0.3, 1.0);
    let dialog_motion = motion().entering(
        MotionSpec::new(timing)
            .values(MotionChannel::Opacity, [0.0, 1.0])
            .values(MotionChannel::Scale, [0.92, 1.0])
            .values(MotionChannel::TranslateY, [10.0, 0.0]),
    );

    view()
        .absolute()
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .child(modal_overlay(on_close))
        .child(
            view()
                .rounded(px(10.0))
                .shadow_xl()
                .bind_motion(&dialog_motion)
                .child(content),
        )
}
