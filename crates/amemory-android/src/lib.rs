//! Android entry point for Amemory app, bridging NSH Native UI and JNI lifecycle.

#[cfg(target_os = "android")]
use nsh_platform_android::{Root, main};
#[cfg(target_os = "android")]
use xcat::ui::window_manager::AmemoryGuiComponent;

#[cfg(target_os = "android")]
main!(
    Root::of::<AmemoryGuiComponent>()
        .app_id("com.amemory.app")
        .font(xcat::ui::assets::ICONFONT_TTF_BYTES.to_vec())
);

#[cfg(not(target_os = "android"))]
pub fn dummy_android_export() {}
