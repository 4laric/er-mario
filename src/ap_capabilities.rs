//! Archipelago ABI: external callers publish snapshots; only the SM64 worker applies them.
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub const ALL: u32 = 0x3ff;
pub const ENEMY_GRAB: u32 = 32;
pub const BOSS_SWING: u32 = 64;
/// Immutable feature support, independent of current Mario liveness.
pub const SUPPORTS_REGRESSION_INTERACT: u32 = 8;
static REQUESTED: AtomicU64 = AtomicU64::new(0);
static APPLIED: AtomicU64 = AtomicU64::new(0);
// Published by the game thread after a successful tick and usable Mario pose.
// The ABI getter must never acquire MARIO's mutex from an external callback.
static LIVE_INSTANCE: AtomicBool = AtomicBool::new(false);

pub fn set_live_instance(live: bool) {
    LIVE_INSTANCE.store(live, Ordering::Release);
}

#[repr(C)]
pub struct State {
    pub abi_version: u32,
    pub flags: u32,
    pub managed: u32,
    pub unlocked: u32,
}

fn snapshot(managed: u32, unlocked: u32) -> Option<u64> {
    if managed & !ALL != 0 || unlocked & !managed != 0 {
        None
    } else {
        Some((u64::from(managed) << 32) | u64::from(unlocked))
    }
}

pub fn allows(capability: u32) -> bool {
    let state = APPLIED.load(Ordering::Acquire);
    let managed = (state >> 32) as u32;
    let unlocked = state as u32;
    capability & managed & !unlocked == 0
}

/// Must be called only on the worker, before executing jobs (including ticks).
pub fn apply() {
    let state = REQUESTED.load(Ordering::Acquire);
    unsafe { crate::sm64::sm64_er_ap_set_capabilities((state >> 32) as u32, state as u32) };
    APPLIED.store(state, Ordering::Release);
    crate::ap_stats::apply();
    crate::ap_fludd::apply();
    crate::ap_cappy::apply();
    crate::ap_sonic::apply();
}

#[unsafe(no_mangle)]
pub extern "C" fn er_mario_ap_abi_version() -> u32 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn er_mario_ap_set_capabilities(managed: u32, unlocked: u32) -> u32 {
    let Some(state) = snapshot(managed, unlocked) else {
        return 0;
    };
    REQUESTED.store(state, Ordering::Release);
    1
}

/// `out` must point to writable storage for State. Null pointers are rejected.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn er_mario_ap_get_state(out: *mut State) -> u32 {
    if out.is_null() {
        return 0;
    }
    let state = APPLIED.load(Ordering::Acquire);
    let ready = LIVE_INSTANCE.load(Ordering::Acquire)
        && crate::SM64_READY.load(Ordering::Relaxed)
        && crate::assets::ready()
        && !crate::worker::hung();
    let enabled = crate::ENABLED.load(Ordering::Relaxed);
    let applied = state == REQUESTED.load(Ordering::Acquire);
    unsafe {
        out.write(State {
            abi_version: 1,
            flags: u32::from(ready)
                | (u32::from(enabled) << 1)
                | (u32::from(applied) << 2)
                | SUPPORTS_REGRESSION_INTERACT
                | crate::ap_stats::SUPPORTS_STATS
                | crate::ap_fludd::SUPPORTS_FLUDD
                | crate::ap_cappy::SUPPORTS_CAPPY
                | crate::ap_sonic::SUPPORTS_SONIC,
            // Stats feature support does not depend on current liveness.
            managed: (state >> 32) as u32,
            unlocked: state as u32,
        });
    }
    1
}
