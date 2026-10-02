//! Additive ABI v1 stats extension. Only the SM64 worker applies requested snapshots.
use std::sync::atomic::{AtomicU64, Ordering};
pub const SUPPORTS_STATS: u32 = 16;
const DEFAULT: u64 = (8u64 << 32) | 10000;
static REQUESTED: AtomicU64 = AtomicU64::new(DEFAULT);
static APPLIED: AtomicU64 = AtomicU64::new(DEFAULT);
#[repr(C)]
pub struct StatState {
    pub abi_version: u32,
    pub flags: u32,
    pub max_wedges: u32,
    pub power_basis_points: u32,
}
pub fn max_wedges() -> u32 {
    (APPLIED.load(Ordering::Acquire) >> 32) as u32
}
pub fn power_basis_points() -> u32 {
    APPLIED.load(Ordering::Acquire) as u32
}
pub fn full_health() -> u16 {
    ((max_wedges() << 8) + 0x80) as u16
}
/// Shared by ordinary attacks and every enemy/boss throw-impact path.
pub fn percent_damage(max_hp: i32, percent: f32) -> i32 {
    let multiplier = power_basis_points() as f32 / 10000.0;
    ((max_hp as f32 * percent / 100.0 * multiplier).ceil() as i32).max(1)
}
pub fn hud_pie(wedges: u8, capacity: u32) -> u8 {
    ((u32::from(wedges) * 8).div_ceil(capacity.clamp(4, 8))).min(8) as u8
}
pub fn apply() {
    let requested = REQUESTED.load(Ordering::Acquire);
    unsafe { crate::sm64::sm64_er_ap_set_max_wedges((requested >> 32) as u32) };
    APPLIED.store(requested, Ordering::Release);
}
#[unsafe(no_mangle)]
pub extern "C" fn er_mario_ap_set_stats(max_wedges: u32, power_basis_points: u32) -> u32 {
    if !(4..=8).contains(&max_wedges) || !matches!(power_basis_points, 7500 | 10000 | 12500 | 15000)
    {
        return 0;
    }
    REQUESTED.store(
        (u64::from(max_wedges) << 32) | u64::from(power_basis_points),
        Ordering::Release,
    );
    1
}
/// `out` must point to writable StatState storage. Null is rejected.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn er_mario_ap_get_stats_state(out: *mut StatState) -> u32 {
    if out.is_null() {
        return 0;
    }
    let applied = APPLIED.load(Ordering::Acquire);
    let mut base = crate::ap_capabilities::State {
        abi_version: 0,
        flags: 0,
        managed: 0,
        unlocked: 0,
    };
    unsafe { crate::ap_capabilities::er_mario_ap_get_state(&mut base) };
    unsafe {
        out.write(StatState {
            abi_version: 1,
            flags: (base.flags & 3)
                | (u32::from(applied == REQUESTED.load(Ordering::Acquire)) << 2),
            max_wedges: (applied >> 32) as u32,
            power_basis_points: applied as u32,
        })
    };
    1
}
