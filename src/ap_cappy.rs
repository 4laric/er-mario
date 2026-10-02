//! Worker-owned original cappy traversal; external callers only queue immutable snapshots.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
pub const SUPPORTS_CAPPY: u32 = 64;
const MASK: u32 = 3;
const EXTERNAL: u64 = 1 << 63;
static REQUESTED: AtomicU64 = AtomicU64::new(0);
static APPLIED: AtomicU64 = AtomicU64::new(0);
#[repr(C)]
pub struct AddonState {
    pub abi_version: u32,
    pub flags: u32,
    pub unlocked: u32,
    pub runtime_state: u32,
}
#[derive(Clone, Copy, Default)]
pub struct Visual {
    pub enabled: bool,
    pub flying: bool,
    /// Projectile center in SM64 world coordinates, not Mario's head-rig origin.
    pub position: [f32; 3],
    pub spin_yaw: f32,
    /// 0=held, 1=outbound/hover, 2=returning.
    pub phase: u32,
}
static PUBLISHED: Mutex<(u64, Visual)> = Mutex::new((
    0,
    Visual {
        enabled: false,
        flying: false,
        position: [0.; 3],
        spin_yaw: 0.,
        phase: 0,
    },
));
pub fn visual() -> Visual {
    PUBLISHED.lock().unwrap_or_else(|e| e.into_inner()).1
}
fn requested() -> u64 {
    let r = REQUESTED.load(Ordering::Acquire);
    if r & EXTERNAL != 0 {
        return r;
    }
    static DEFAULT: OnceLock<u64> = OnceLock::new();
    *DEFAULT.get_or_init(|| {
        if crate::paths::config("cappy")
            .is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "on" | "1" | "true" | "yes"))
        {
            1 | ((MASK as u64) << 8)
        } else {
            0
        }
    })
}
/// Worker only. Applying the same identity must not reset a projectile, charge or airtime resource.
pub fn apply() {
    let r = requested();
    unsafe {
        crate::sm64::sm64_er_cappy_configure((r & 1) as u32, ((r >> 8) & MASK as u64) as u32)
    };
    APPLIED.store(r, Ordering::Release);
    publish();
}
/// Worker only. ABI/render readers never access native global state.
pub fn publish() {
    let mut s = [0u32; 3];
    let mut f = [0f32; 4];
    unsafe { crate::sm64::sm64_er_cappy_get_state(s.as_mut_ptr(), f.as_mut_ptr()) };
    let v = Visual {
        enabled: s[0] != 0,
        flying: s[2] != 0,
        position: [f[0], f[1], f[2]],
        spin_yaw: f[3],
        phase: s[2],
    };
    let r = APPLIED.load(Ordering::Acquire);
    let identity = (r & EXTERNAL) | u64::from(s[0]) | (u64::from(s[1]) << 8);
    *PUBLISHED.lock().unwrap_or_else(|e| e.into_inner()) = (identity, v);
}
#[unsafe(no_mangle)]
pub extern "C" fn er_mario_ap_set_cappy(enabled: u32, unlocks: u32) -> u32 {
    if enabled > 1 || unlocks & !MASK != 0 || (enabled == 0 && unlocks != 0) {
        return 0;
    }
    REQUESTED.store(
        EXTERNAL | u64::from(enabled) | (u64::from(unlocks) << 8),
        Ordering::Release,
    );
    1
}
/// # Safety
/// `out` must point to writable AddonState storage; null is rejected.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn er_mario_ap_get_cappy_state(out: *mut AddonState) -> u32 {
    if out.is_null() {
        return 0;
    }
    let r = APPLIED.load(Ordering::Acquire);
    let (identity, v) = *PUBLISHED.lock().unwrap_or_else(|e| e.into_inner());
    let mut base = crate::ap_capabilities::State {
        abi_version: 0,
        flags: 0,
        managed: 0,
        unlocked: 0,
    };
    unsafe { crate::ap_capabilities::er_mario_ap_get_state(&mut base) };
    unsafe {
        out.write(AddonState {
            abi_version: 1,
            flags: (base.flags & 1)
                | ((v.enabled as u32) << 1)
                | ((u32::from(identity == r && r == requested())) << 2),
            unlocked: ((identity >> 8) & MASK as u64) as u32,
            runtime_state: v.phase,
        })
    };
    1
}
