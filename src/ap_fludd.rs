//! Atomic FLUDD ABI; native configuration, fuel and physics belong to the SM64 worker.
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
pub const SUPPORTS_FLUDD: u32 = 32;
pub const HOVER: u32 = 1;
pub const ROCKET: u32 = 2;
pub const TURBO: u32 = 4;
const EXTERNAL: u64 = 1 << 63;
static REQUESTED: AtomicU64 = AtomicU64::new(0);
static APPLIED: AtomicU64 = AtomicU64::new(0);
static VISUAL: AtomicU64 = AtomicU64::new(60 << 16);
#[repr(C)]
pub struct FluddState {
    pub abi_version: u32,
    pub flags: u32,
    pub unlocked_nozzles: u32,
    pub tank_level: u32,
    pub selected_nozzle: u32,
    pub water_units: u32,
    pub capacity_units: u32,
}
/// Nozzle values are 0=none, 1=hover, 2=rocket, 4=turbo, matching the unlock mask.
#[derive(Clone, Copy)]
pub struct Visual {
    pub enabled: bool,
    pub selected_nozzle: u32,
    pub active: bool,
    pub water_units: u32,
    pub capacity_units: u32,
}
pub fn visual() -> Visual {
    decode(VISUAL.load(Ordering::Acquire))
}
fn decode(v: u64) -> Visual {
    Visual {
        enabled: v & 1 != 0,
        active: v & 2 != 0,
        selected_nozzle: ((v >> 2) & 7) as u32,
        water_units: ((v >> 8) & 255) as u32,
        capacity_units: ((v >> 16) & 255) as u32,
    }
}
fn requested() -> u64 {
    let r = REQUESTED.load(Ordering::Acquire);
    if r & EXTERNAL != 0 {
        return r;
    }
    static DEFAULT: OnceLock<u64> = OnceLock::new();
    *DEFAULT.get_or_init(|| {
        if crate::paths::config("fludd")
            .is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "on" | "1" | "true" | "yes"))
        {
            1 | (7 << 8)
        } else {
            0
        }
    })
}
/// Worker only: acknowledges configuration after native application, never refills fuel.
pub fn apply() {
    let r = requested();
    unsafe {
        crate::sm64::sm64_er_fludd_configure(
            (r & 1) as u32,
            ((r >> 8) & 7) as u32,
            ((r >> 16) & 3) as u32,
        )
    };
    APPLIED.store(r, Ordering::Release);
    publish();
}
/// Worker only, after a tick/refill/suspension; render and ABI readers use the atomic copy.
pub fn publish() {
    let mut s = [0u32; 5];
    unsafe { crate::sm64::sm64_er_fludd_get_state(s.as_mut_ptr()) };
    let r = APPLIED.load(Ordering::Acquire);
    VISUAL.store(
        ((r >> 8) & 7) << 24
            | ((r >> 16) & 3) << 27
            | ((r >> 63) << 29)
            | u64::from(s[0])
            | (u64::from(s[2] != 0) << 1)
            | (u64::from(s[1]) << 2)
            | (u64::from(s[3]) << 8)
            | (u64::from(s[4]) << 16),
        Ordering::Release,
    );
}
#[unsafe(no_mangle)]
pub extern "C" fn er_mario_ap_set_fludd(enabled: u32, nozzles: u32, tank_level: u32) -> u32 {
    if enabled > 1
        || nozzles & !7 != 0
        || tank_level > 3
        || (enabled == 0 && (nozzles != 0 || tank_level != 0))
    {
        return 0;
    }
    REQUESTED.store(
        EXTERNAL | u64::from(enabled) | (u64::from(nozzles) << 8) | (u64::from(tank_level) << 16),
        Ordering::Release,
    );
    1
}
/// # Safety
/// `out` must point to writable FluddState storage. Null pointers are rejected.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn er_mario_ap_get_fludd_state(out: *mut FluddState) -> u32 {
    if out.is_null() {
        return 0;
    }
    let r = APPLIED.load(Ordering::Acquire);
    let mut base = crate::ap_capabilities::State {
        abi_version: 0,
        flags: 0,
        managed: 0,
        unlocked: 0,
    };
    unsafe { crate::ap_capabilities::er_mario_ap_get_state(&mut base) };
    let packed = VISUAL.load(Ordering::Acquire);
    let v = decode(packed);
    let same = (packed & 1) == (r & 1)
        && ((packed >> 24) & 7) == ((r >> 8) & 7)
        && ((packed >> 27) & 3) == ((r >> 16) & 3)
        && ((packed >> 29) & 1) == (r >> 63);
    unsafe {
        out.write(FluddState {
            abi_version: 1,
            flags: (base.flags & 1)
                | (((r & 1) as u32) << 1)
                | (u32::from(same && r == requested()) << 2),
            unlocked_nozzles: ((r >> 8) & 7) as u32,
            tank_level: ((r >> 16) & 3) as u32,
            selected_nozzle: v.selected_nozzle,
            water_units: v.water_units,
            capacity_units: v.capacity_units,
        })
    };
    1
}
