//! Original skating mode. Native state belongs exclusively to the SM64 worker.
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Default)]
pub struct Visual {
    pub enabled: bool,
    pub mounted: bool,
    pub airborne: bool,
    pub speed: f32,
    pub lean: f32,
    pub trick: u32,
    pub bail_ticks: u32,
}

static PUBLISHED: Mutex<Visual> = Mutex::new(Visual {
    enabled: false,
    mounted: false,
    airborne: false,
    speed: 0.0,
    lean: 0.0,
    trick: 0,
    bail_ticks: 0,
});

pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        crate::paths::config("skateboard")
            .is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "on" | "1" | "true" | "yes"))
    })
}

pub fn visual() -> Visual {
    *PUBLISHED.lock().unwrap_or_else(|e| e.into_inner())
}

/// Worker only. Configuration is idempotent and does not remount after a bail.
pub fn apply() {
    unsafe { crate::sm64::sm64_er_skate_configure(enabled() as u32) };
}

pub fn publish() {
    let mut state = [0u32; 5];
    let mut motion = [0.0f32; 2];
    unsafe { crate::sm64::sm64_er_skate_get_state(state.as_mut_ptr(), motion.as_mut_ptr()) };
    let next = Visual {
        enabled: state[0] != 0,
        mounted: state[1] != 0,
        airborne: state[2] != 0,
        trick: state[3],
        bail_ticks: state[4],
        speed: if motion[0].is_finite() {
            motion[0]
        } else {
            0.0
        },
        lean: if motion[1].is_finite() {
            motion[1].clamp(-1.0, 1.0)
        } else {
            0.0
        },
    };
    let mut previous = PUBLISHED.lock().unwrap_or_else(|e| e.into_inner());
    if previous.mounted != next.mounted || previous.trick != next.trick {
        crate::log(format!(
            "skate: mounted={} airborne={} speed={:.1} trick={} bail={}",
            next.mounted, next.airborne, next.speed, next.trick, next.bail_ticks
        ));
    }
    *previous = next;
}

/// Worker only; reset inputs and mounting on travel, menus and forced movement.
pub fn suspend() {
    unsafe { crate::sm64::sm64_er_skate_reset() };
    publish();
}
