//! Embedded WAV feedback sounds and Win32 `PlaySoundW` playback.

use std::sync::Mutex;
use std::time::{Duration, Instant};

const SOUND_PENCLICK: &[u8] = include_bytes!("../../assets/copy_penclick.wav");
const SOUND_KACHA: &[u8] = include_bytes!("../../assets/copy_kacha.wav");
const SOUND_BLIP: &[u8] = include_bytes!("../../assets/copy_blip.wav");
const SOUND_BUBBLE: &[u8] = include_bytes!("../../assets/copy_bubble.wav");
const SOUND_CLACK: &[u8] = include_bytes!("../../assets/copy_clack.wav");
const SOUND_MECHKB: &[u8] = include_bytes!("../../assets/copy_mechkb.wav");

static LAST_PLAY: Mutex<Option<Instant>> = Mutex::new(None);
const PLAY_DEBOUNCE: Duration = Duration::from_millis(60);

/// All available sound options.
pub const AVAILABLE_SOUNDS: [&str; 6] = [
    "copy_penclick.wav",
    "copy_kacha.wav",
    "copy_blip.wav",
    "copy_bubble.wav",
    "copy_clack.wav",
    "copy_mechkb.wav",
];

/// Look up embedded sound bytes by filename.
#[must_use]
pub fn get_sound_data(filename: &str) -> Option<&'static [u8]> {
    match filename {
        "copy_penclick.wav" => Some(SOUND_PENCLICK),
        "copy_kacha.wav" => Some(SOUND_KACHA),
        "copy_blip.wav" => Some(SOUND_BLIP),
        "copy_bubble.wav" => Some(SOUND_BUBBLE),
        "copy_clack.wav" => Some(SOUND_CLACK),
        "copy_mechkb.wav" => Some(SOUND_MECHKB),
        _ => Some(SOUND_KACHA),
    }
}

/// Play audio feedback by numeric index (1..=5).
pub fn play_copy_sound(index: usize) {
    let name = match index {
        1 => "copy_penclick.wav",
        2 => "copy_kacha.wav",
        3 => "copy_blip.wav",
        4 => "copy_bubble.wav",
        5 => "copy_clack.wav",
        _ => "copy_mechkb.wav",
    };
    play_copy_sound_name(name);
}

/// Play audio feedback by filename.
pub fn play_copy_sound_name(filename: &str) {
    if let Ok(mut last) = LAST_PLAY.lock() {
        let now = Instant::now();
        if let Some(prev) = *last {
            if now.duration_since(prev) < PLAY_DEBOUNCE {
                return;
            }
        }
        *last = Some(now);
    }

    let Some(data) = get_sound_data(filename) else {
        return;
    };

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY};
        unsafe {
            let _ = PlaySoundW(data.as_ptr().cast(), 0, SND_MEMORY | SND_ASYNC);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = data;
    }
}
