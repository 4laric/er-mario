//! Run without game/dependencies: rustc --edition=2024 --test tests/ap_bridge.rs -o ap_bridge_tests.exe
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static SM64_READY: AtomicBool = AtomicBool::new(false);
static ENABLED: AtomicBool = AtomicBool::new(false);
mod paths {
    pub fn config(_: &str) -> Option<String> {
        None
    }
}
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
    static FLUDD: std::sync::Mutex<[u32; 5]> = std::sync::Mutex::new([0, 0, 0, 0, 60]);
    pub unsafe fn sm64_er_fludd_configure(enabled: u32, mask: u32, level: u32) {
        let mut s = FLUDD.lock().unwrap();
        s[0] = enabled;
        if s[1] & mask == 0 {
            s[1] = mask & mask.wrapping_neg();
        }
        s[4] = 60 + 20 * level;
        s[3] = s[3].min(s[4]);
        if enabled == 0 {
            s[1] = 0;
            s[2] = 0;
            s[3] = 0;
        }
    }
    pub unsafe fn sm64_er_fludd_get_state(out: *mut u32) {
        unsafe { std::ptr::copy_nonoverlapping(FLUDD.lock().unwrap().as_ptr(), out, 5) };
    }
    pub static C_MAX_WEDGES: AtomicU64 = AtomicU64::new(8);
    pub unsafe fn sm64_er_ap_set_max_wedges(wedges: u32) {
        C_MAX_WEDGES.store(u64::from(wedges), Ordering::Relaxed);
    }
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
#[path = "../src/ap_fludd.rs"]
mod ap_fludd;
#[path = "../src/ap_stats.rs"]
mod ap_stats;

#[test]
fn abi_snapshots_are_validated_and_acknowledged_only_after_worker_application() {
    let _guard = TEST_LOCK.lock().unwrap();
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
    assert_eq!(
        state.flags,
        4 | (SUPPORTS_REGRESSION_INTERACT | ap_stats::SUPPORTS_STATS | ap_fludd::SUPPORTS_FLUDD)
    );
    assert!(allows(ALL));
    assert_eq!(er_mario_ap_set_capabilities(ALL + 1, 0), 0);
    assert_eq!(er_mario_ap_set_capabilities(1, 2), 0);
    assert_eq!(er_mario_ap_set_capabilities(0, ALL), 0);
    unsafe { er_mario_ap_get_state(&mut state) };
    assert_eq!(
        (state.managed, state.unlocked, state.flags),
        (
            0,
            0,
            4 | (SUPPORTS_REGRESSION_INTERACT
                | ap_stats::SUPPORTS_STATS
                | ap_fludd::SUPPORTS_FLUDD)
        )
    );
    for unlocked in 0..=ALL {
        assert_eq!(er_mario_ap_set_capabilities(ALL, unlocked), 1);
        unsafe { er_mario_ap_get_state(&mut state) };
        assert_eq!(state.flags & 4, 0);
        apply();
        unsafe { er_mario_ap_get_state(&mut state) };
        assert_eq!(
            (state.managed, state.unlocked, state.flags),
            (
                ALL,
                unlocked,
                4 | (SUPPORTS_REGRESSION_INTERACT
                    | ap_stats::SUPPORTS_STATS
                    | ap_fludd::SUPPORTS_FLUDD)
            )
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
    assert_eq!(
        state.flags,
        6 | (SUPPORTS_REGRESSION_INTERACT | ap_stats::SUPPORTS_STATS | ap_fludd::SUPPORTS_FLUDD)
    ); // Enabled/assets/libsm64 cannot prove a live Mario.
    set_live_instance(true);
    unsafe { er_mario_ap_get_state(&mut state) };
    assert_eq!(
        state.flags,
        7 | (SUPPORTS_REGRESSION_INTERACT | ap_stats::SUPPORTS_STATS | ap_fludd::SUPPORTS_FLUDD)
    );
    set_live_instance(false);
    unsafe { er_mario_ap_get_state(&mut state) };
    assert_eq!(
        state.flags,
        6 | (SUPPORTS_REGRESSION_INTERACT | ap_stats::SUPPORTS_STATS | ap_fludd::SUPPORTS_FLUDD)
    );
    assert_eq!(sm64::C_APPLIED.load(Ordering::Relaxed), 0);
    assert_eq!(std::mem::size_of::<State>(), 16);
}

#[test]
fn stats_extension_queues_valid_snapshots_and_uses_applied_values_for_damage_and_hud() {
    let _guard = TEST_LOCK.lock().unwrap();
    use ap_stats::*;
    let mut state = StatState {
        abi_version: 0,
        flags: 0,
        max_wedges: 0,
        power_basis_points: 0,
    };
    assert_eq!(std::mem::size_of::<StatState>(), 16);
    assert_eq!(
        unsafe { er_mario_ap_get_stats_state(std::ptr::null_mut()) },
        0
    );
    assert_eq!(unsafe { er_mario_ap_get_stats_state(&mut state) }, 1);
    assert_eq!((state.max_wedges, state.power_basis_points), (8, 10000));
    for (wedges, power) in [
        (0, 10000),
        (3, 10000),
        (9, 10000),
        (8, 7499),
        (8, 15001),
        (u32::MAX, u32::MAX),
    ] {
        assert_eq!(er_mario_ap_set_stats(wedges, power), 0);
    }
    for wedges in 4..=8 {
        for power in [7500, 10000, 12500, 15000] {
            let before = (max_wedges(), power_basis_points());
            assert_eq!(er_mario_ap_set_stats(wedges, power), 1);
            assert_eq!((max_wedges(), power_basis_points()), before);
            unsafe { er_mario_ap_get_stats_state(&mut state) };
            if before != (wedges, power) {
                assert_eq!(state.flags & 4, 0);
            }
            apply();
            unsafe { er_mario_ap_get_stats_state(&mut state) };
            assert_eq!(
                (state.max_wedges, state.power_basis_points, state.flags & 4),
                (wedges, power, 4)
            );
            assert_eq!(
                sm64::C_MAX_WEDGES.load(Ordering::Relaxed),
                u64::from(wedges)
            );
            assert_eq!(full_health(), ((wedges << 8) + 0x80) as u16);
            // Ordinary attack shares, enemy collision throws and boss impacts all call this policy.
            for percent in [25.0, 34.0, 50.0, 67.0, 100.0] {
                let expected = (1000.0f32 * percent / 100.0 * power as f32 / 10000.0).ceil() as i32;
                assert_eq!(percent_damage(1000, percent), expected);
            }
            assert_eq!(percent_damage(1, 0.01), 1);
            assert_eq!(hud_pie(wedges as u8, wedges), 8);
            assert!(hud_pie((wedges - 1) as u8, wedges) < 8);
        }
    }
    assert_eq!(er_mario_ap_set_stats(8, 10000), 1);
    apply();
    assert_eq!((max_wedges(), power_basis_points()), (8, 10000));
}

#[test]
fn fludd_abi_is_additive_validated_and_worker_acknowledged() {
    let _guard = TEST_LOCK.lock().unwrap();
    use ap_fludd::*;
    assert_eq!(std::mem::size_of::<FluddState>(), 28);
    assert_eq!(HOVER | ROCKET | TURBO, 7);
    assert_eq!(
        unsafe { er_mario_ap_get_fludd_state(std::ptr::null_mut()) },
        0
    );
    let mut s = FluddState {
        abi_version: 0,
        flags: 0,
        unlocked_nozzles: 0,
        tank_level: 0,
        selected_nozzle: 0,
        water_units: 0,
        capacity_units: 0,
    };
    for args in [
        (2, 0, 0),
        (1, 8, 0),
        (1, 7, 4),
        (0, 1, 0),
        (0, 0, 1),
        (u32::MAX, u32::MAX, u32::MAX),
    ] {
        assert_eq!(er_mario_ap_set_fludd(args.0, args.1, args.2), 0);
    }
    for mask in 0..=7 {
        for tier in 0..=3 {
            assert_eq!(er_mario_ap_set_fludd(1, mask, tier), 1);
            unsafe { er_mario_ap_get_fludd_state(&mut s) };
            assert_eq!(s.flags & 4, 0);
            apply();
            unsafe { er_mario_ap_get_fludd_state(&mut s) };
            assert_eq!(
                (
                    s.flags & 6,
                    s.unlocked_nozzles,
                    s.tank_level,
                    s.capacity_units
                ),
                (6, mask, tier, 60 + 20 * tier)
            );
            assert_eq!(s.selected_nozzle & !mask, 0);
            assert_eq!(er_mario_ap_set_fludd(1, mask, tier), 1);
            apply();
            assert_eq!(visual().capacity_units, 60 + 20 * tier);
            assert!(visual().enabled && !visual().active);
        }
    }
    assert_eq!(er_mario_ap_set_fludd(0, 0, 0), 1);
    apply();
    unsafe { er_mario_ap_get_fludd_state(&mut s) };
    assert_eq!(
        (
            s.flags & 6,
            s.selected_nozzle,
            s.water_units,
            s.capacity_units
        ),
        (4, 0, 0, 60)
    );
}
