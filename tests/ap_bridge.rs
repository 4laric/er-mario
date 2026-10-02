//! Run without game/dependencies: rustc --edition=2024 --test tests/ap_bridge.rs -o ap_bridge_tests.exe
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
static SM64_READY: AtomicBool = AtomicBool::new(false);
static ENABLED: AtomicBool = AtomicBool::new(false);
mod assets {
    pub fn ready() -> bool {
        true
    }
}
mod worker {
    pub fn hung() -> bool {
        false
    }
}
mod sm64 {
    use super::*;
    pub static C_APPLIED: AtomicU64 = AtomicU64::new(0);
    pub unsafe fn sm64_er_ap_set_capabilities(managed: u32, unlocked: u32) {
        C_APPLIED.store(
            (u64::from(managed) << 32) | u64::from(unlocked),
            Ordering::Relaxed,
        );
    }
}
#[path = "../src/ap_capabilities.rs"]
mod ap_capabilities;

#[test]
fn abi_snapshots_are_validated_and_acknowledged_only_after_worker_application() {
    use ap_capabilities::*;
    let mut state = State {
        abi_version: 0,
        flags: 0,
        managed: 0,
        unlocked: 0,
    };
    assert_eq!(er_mario_ap_abi_version(), 1);
    assert_eq!(unsafe { er_mario_ap_get_state(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { er_mario_ap_get_state(&mut state) }, 1);
    assert_eq!(state.flags, 4 | SUPPORTS_REGRESSION_INTERACT);
    assert!(allows(ALL));
    assert_eq!(er_mario_ap_set_capabilities(ALL + 1, 0), 0);
    assert_eq!(er_mario_ap_set_capabilities(1, 2), 0);
    assert_eq!(er_mario_ap_set_capabilities(0, ALL), 0);
    unsafe { er_mario_ap_get_state(&mut state) };
    assert_eq!(
        (state.managed, state.unlocked, state.flags),
        (0, 0, 4 | SUPPORTS_REGRESSION_INTERACT)
    );
    for unlocked in 0..=ALL {
        assert_eq!(er_mario_ap_set_capabilities(ALL, unlocked), 1);
        unsafe { er_mario_ap_get_state(&mut state) };
        assert_eq!(state.flags & 4, 0);
        apply();
        unsafe { er_mario_ap_get_state(&mut state) };
        assert_eq!(
            (state.managed, state.unlocked, state.flags),
            (ALL, unlocked, 4 | SUPPORTS_REGRESSION_INTERACT)
        );
        for bit in [1, 2, 4, 8, 16, ENEMY_GRAB, BOSS_SWING, 128, 256, 512] {
            assert_eq!(allows(bit), unlocked & bit != 0);
        }
    }
    assert_eq!(er_mario_ap_set_capabilities(2, 0), 1);
    apply();
    assert!(allows(ALL & !2));
    assert!(!allows(2));
    assert_eq!(er_mario_ap_set_capabilities(0, 0), 1);
    apply();
    assert!(allows(ALL));
    SM64_READY.store(true, Ordering::Relaxed);
    ENABLED.store(true, Ordering::Relaxed);
    unsafe { er_mario_ap_get_state(&mut state) };
    assert_eq!(state.flags, 6 | SUPPORTS_REGRESSION_INTERACT); // Enabled/assets/libsm64 cannot prove a live Mario.
    set_live_instance(true);
    unsafe { er_mario_ap_get_state(&mut state) };
    assert_eq!(state.flags, 7 | SUPPORTS_REGRESSION_INTERACT);
    set_live_instance(false);
    unsafe { er_mario_ap_get_state(&mut state) };
    assert_eq!(state.flags, 6 | SUPPORTS_REGRESSION_INTERACT);
    assert_eq!(sm64::C_APPLIED.load(Ordering::Relaxed), 0);
    assert_eq!(std::mem::size_of::<State>(), 16);
}
