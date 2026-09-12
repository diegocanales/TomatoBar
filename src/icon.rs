use std::sync::LazyLock;

use image::GenericImageView;

use crate::timer::{Phase, Timer};

static IDLE_ICON: LazyLock<ksni::Icon> =
    LazyLock::new(|| load_embedded_icon(include_bytes!("../assets/icons/idle.png")));
static WORK_ICON: LazyLock<ksni::Icon> =
    LazyLock::new(|| load_embedded_icon(include_bytes!("../assets/icons/work.png")));
static REST_ICON: LazyLock<ksni::Icon> =
    LazyLock::new(|| load_embedded_icon(include_bytes!("../assets/icons/rest.png")));
static LONG_REST_ICON: LazyLock<ksni::Icon> =
    LazyLock::new(|| load_embedded_icon(include_bytes!("../assets/icons/long_rest.png")));
static PAUSE_ICON: LazyLock<ksni::Icon> =
    LazyLock::new(|| load_embedded_icon(include_bytes!("../assets/icons/pause.png")));

fn load_embedded_icon(bytes: &'static [u8]) -> ksni::Icon {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .expect("icon must be a valid PNG");
    let (width, height) = img.dimensions();
    let mut data = img.into_rgba8().into_vec();
    for pixel in data.chunks_exact_mut(4) {
        // Force symbolic white; keep alpha (macOS template icons are black+alpha).
        let a = pixel[3];
        pixel[0] = 255;
        pixel[1] = 255;
        pixel[2] = 255;
        pixel[3] = a;
        pixel.rotate_right(1); // RGBA → ARGB
    }
    ksni::Icon {
        width: width as i32,
        height: height as i32,
        data,
    }
}

pub fn phase_base_icon(phase: Phase, paused: bool) -> &'static ksni::Icon {
    if paused && phase.is_running() {
        return &PAUSE_ICON;
    }
    match phase {
        Phase::Idle => &IDLE_ICON,
        Phase::Work => &WORK_ICON,
        Phase::ShortRest => &REST_ICON,
        Phase::LongRest => &LONG_REST_ICON,
    }
}

/// Tomato pixmap only. Countdown goes in the Ayatana tray label so GNOME can
/// grow the panel item instead of squashing a wide pixmap into a square slot.
pub fn tray_icon(timer: &Timer) -> ksni::Icon {
    phase_base_icon(timer.phase, timer.paused).clone()
}

/// Text next to the icon when `show_timer_in_icon` is on (empty = hide label).
pub fn tray_label(timer: &Timer) -> String {
    if timer.phase.is_running() && timer.config.show_timer_in_icon {
        timer.display_time()
    } else {
        String::new()
    }
}

/// Fixed guide so the panel reserves width for `mm:ss` and does not jiggle.
pub fn tray_label_guide(timer: &Timer) -> String {
    if timer.phase.is_running() && timer.config.show_timer_in_icon {
        "88:88".into()
    } else {
        String::new()
    }
}
