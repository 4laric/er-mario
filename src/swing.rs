//! Bowser's tail swing on bosses: break a boss's stance, and Mario's next hit grabs him like
//! Bowser (SM64's own grab, swing and throw: spin the stick to wind up, B to let go). The boss
//! is carried around Mario at arm's length while he swings, then flies off where Mario faces,
//! and whatever he hits first (a wall, a rock, the ground) hurts him badly.

use std::sync::Mutex;
use std::time::Instant;

use eldenring::cs::{FieldInsHandle, WorldChrMan};
use eldenring::position::HavokPosition;
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::log;

pub const ACT_PICKING_UP_BOWSER: u32 = 0x390;
pub const ACT_HOLDING_BOWSER: u32 = 0x391;
pub const ACT_RELEASING_BOWSER: u32 = 0x392;
/// SOUND_OBJ_BOWSER_TAIL_PICKUP, SOUND_GENERAL_BOWSER_BOMB_EXPLOSION
pub const SOUND_GRAB: i32 = 0x5005_0081;
pub const SOUND_IMPACT: i32 = 0x312F_0081;

/// How long a broken stance leaves a boss open to the grab (s)
const OPEN_FOR: f32 = 3.0;
/// The game's ragdoll blend (ChrCtrl +0x128 state, +0x12C amount): 0 animated, 4 blended, 2 full
/// (its death ragdoll, never used here). Amount 1 = all ragdoll.
const RAGDOLL_FULL: f32 = 0.99;
const DOWN_FOR: f32 = 2.0;
const GET_UP: f32 = 1.0;

fn ragdoll_on() -> bool {
    crate::paths::config("boss_ragdoll").is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "on" | "1" | "true" | "yes"))
}

/// Ragdoll amount 0..1 (0 = back to normal animation).
fn set_ragdoll(chr: &mut eldenring::cs::ChrIns, amount: f32) {
    let c = &mut chr.chr_ctrl;
    if amount <= 0.0 {
        c.chr_ragdoll_state = 0;
        c.ragdoll_revive_time = 1.0;
    } else {
        c.chr_ragdoll_state = 4;
        c.ragdoll_revive_time = amount.min(RAGDOLL_FULL);
    }
}

/// Whether this boss is lying there after a throw (no damage then).
pub fn is_down(h: &FieldInsHandle) -> bool {
    let st = STATE.lock().unwrap_or_else(|e| e.into_inner());
    matches!(st.phase, Phase::Down { boss, .. } if key(&boss) == key(h)) || matches!(st.phase, Phase::Flying { boss, .. } if key(&boss) == key(h))
}

/// Mario's stagger meter per boss hit (% of full); it drains after a few seconds without hits
pub const STANCE_GROUND_POUND: f32 = 34.0;
pub const STANCE_STOMP: f32 = 20.0;
pub const STANCE_DIVE: f32 = 12.0;
pub const STANCE_KICK: f32 = 10.0;
pub const STANCE_PUNCH: f32 = 8.0;
const STANCE_HOLD: f32 = 4.0;
const STANCE_DRAIN: f32 = 15.0;
/// SOUND_OBJ_BOWSER_DEFEATED: the cue that the boss is open for the grab
pub const SOUND_STAGGER: i32 = 0x5006_0081;
/// Impact damage: share of the boss's max HP, from a slow throw to a full-speed one
const IMPACT_MIN: f32 = 8.0;
const IMPACT_MAX: f32 = 25.0;
const GRAVITY: f32 = 16.0;

enum Phase {
    Idle,
    /// Mario has him: (boss, arm's length in m)
    Held { boss: FieldInsHandle, reach: f32 },
    Flying { boss: FieldInsHandle, pos: Vec3, vel: Vec3, since: Instant, radius: f32 },
    /// knocked flat after the impact (his ragdoll), until he gets back up
    Down { boss: FieldInsHandle, until: Instant },
}

struct State {
    phase: Phase,
    /// bosses whose stance just broke: (handle, until)
    open: Vec<(FieldInsHandle, Instant)>,
    /// per boss: poise last frame (a break shows as a drop to zero or a reset to full)
    toughness: Vec<(u64, f32)>,
    /// per boss: Mario's stagger meter (%) and his last hit
    meter: Vec<(FieldInsHandle, f32, Instant)>,
    /// Mario's face angle last tick (the swing's speed)
    last_face: Option<f32>,
    spin: f32,
}

static STATE: Mutex<State> =
    Mutex::new(State { phase: Phase::Idle, open: Vec::new(), toughness: Vec::new(), meter: Vec::new(), last_face: None, spin: 0.0 });
/// the stagger cue to play (SM64 thread)
static CUE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn take_cue() -> bool {
    CUE.swap(false, std::sync::atomic::Ordering::Relaxed)
}

/// Mario hit a boss: his stagger meter fills; full, the boss is open for the grab (and his poise
/// is broken, so the game staggers him too).
pub fn add_stance(h: &FieldInsHandle, amount: f32) {
    let mut st = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let k = key(h);
    if st.open.iter().any(|(o, _)| key(o) == k) {
        return;
    }
    let now = Instant::now();
    let i = match st.meter.iter().position(|(m, _, _)| key(m) == k) {
        Some(i) => i,
        None => {
            st.meter.push((*h, 0.0, now));
            st.meter.len() - 1
        }
    };
    let e = &mut st.meter[i];
    e.1 += amount;
    e.2 = now;
    log(format!("swing: stagger meter {:.0}%", e.1.min(100.0)));
    if e.1 >= 100.0 {
        st.meter.remove(i);
        st.open.push((*h, now + std::time::Duration::from_secs_f32(OPEN_FOR)));
        CUE.store(true, std::sync::atomic::Ordering::Relaxed);
        log("swing: boss staggered: grab him!");
        if let Some(chr) = unsafe { WorldChrMan::instance_mut() }.ok().and_then(|w| w.chr_ins_by_handle_mut(h)) {
            chr.modules.super_armor.sa_durability = 0.0;
        }
    }
}
/// the grab to start on the next SM64 tick
static START: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn key(h: &FieldInsHandle) -> u64 {
    unsafe { std::mem::transmute_copy::<FieldInsHandle, u64>(h) }
}

/// Every frame: watch the bosses' stance (those with a boss bar).
pub fn watch_stances(bosses: &[FieldInsHandle]) {
    let Ok(wcm) = (unsafe { WorldChrMan::instance() }) else { return };
    let mut st = STATE.lock().unwrap_or_else(|e| e.into_inner());
    st.open.retain(|(_, until)| Instant::now() < *until);
    // the stagger meters drain when Mario stops hitting
    for e in st.meter.iter_mut() {
        if e.2.elapsed().as_secs_f32() > STANCE_HOLD {
            e.1 = (e.1 - STANCE_DRAIN / 60.0).max(0.0);
        }
    }
    st.meter.retain(|e| e.1 > 0.0);
    for h in bosses {
        let Some(chr) = wcm.chr_ins_by_handle(h) else { continue };
        // (enemies' stance is their poise: the toughness module is the player's)
        let t = &chr.modules.super_armor;
        let (now, max) = (t.sa_durability, t.sa_durability_max.max(1.0));
        let broken_flag = chr.modules.super_armor.poise_broken_state;
        let k = key(h);
        let last = st.toughness.iter().find(|(kk, _)| *kk == k).map(|(_, v)| *v);
        let broke = broken_flag || (now <= 0.0 && last.is_some_and(|l| l > 0.0)) || last.is_some_and(|l| l < max * 0.3 && now >= max * 0.95);
        match st.toughness.iter_mut().find(|(kk, _)| *kk == k) {
            Some(e) => e.1 = now,
            None => st.toughness.push((k, now)),
        }
        if broke && !st.open.iter().any(|(o, _)| key(o) == k) {
            log(format!("swing: boss stance broken (toughness {now:.0}/{max:.0}): grab him!"));
            st.open.push((*h, Instant::now() + std::time::Duration::from_secs_f32(OPEN_FOR)));
        }
    }
}

/// Poise of a character (for the combat log).
pub fn toughness_of(h: &FieldInsHandle) -> Option<(f32, f32)> {
    let wcm = unsafe { WorldChrMan::instance() }.ok()?;
    let chr = wcm.chr_ins_by_handle(h)?;
    Some((chr.modules.super_armor.sa_durability, chr.modules.super_armor.sa_durability_max))
}

/// A hit landed on `h`: if its stance is broken, Mario grabs it (returns true: no normal damage).
pub fn try_grab(h: &FieldInsHandle, radius_m: f32) -> bool {
    let mut st = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if !matches!(st.phase, Phase::Idle) {
        return false;
    }
    let k = key(h);
    let Some(i) = st.open.iter().position(|(o, _)| key(o) == k) else { return false };
    st.open.remove(i);
    st.phase = Phase::Held { boss: *h, reach: radius_m + 0.9 };
    st.last_face = None;
    st.spin = 0.0;
    START.store(true, std::sync::atomic::Ordering::Relaxed);
    log("swing: grabbed the boss by the tail");
    true
}

/// SM64 tick: whether to start the grab now (Mario goes into SM64's Bowser pickup).
pub fn take_start() -> bool {
    START.swap(false, std::sync::atomic::Ordering::Relaxed)
}

/// Every frame in Mario mode: carry, throw and fly the boss. `mario` feet (game coordinates),
/// `face` SM64 face angle, `action` Mario's SM64 action. Returns an impact: (boss, damage share
/// of max HP in %).
pub fn update(dt: f32, mario: Vec3, face: f32, action: u32, hit: impl Fn(Vec3, Vec3) -> Option<Vec3>) -> Option<(FieldInsHandle, f32)> {
    let Ok(wcm) = (unsafe { WorldChrMan::instance_mut() }) else { return None };
    let mut st = STATE.lock().unwrap_or_else(|e| e.into_inner());
    // Mario's forward in game coordinates (SM64 is mirrored on X)
    let fwd = Vec3::new(-face.sin(), 0.0, face.cos());
    // the swing's angular speed (rad/s) from Mario's turning
    if let Some(l) = st.last_face {
        let mut d = face - l;
        while d > std::f32::consts::PI {
            d -= std::f32::consts::TAU;
        }
        while d < -std::f32::consts::PI {
            d += std::f32::consts::TAU;
        }
        st.spin = st.spin * 0.7 + (d / dt.max(1e-3)) * 0.3;
    }
    st.last_face = Some(face);
    match st.phase {
        Phase::Idle => None,
        Phase::Down { boss, until } => {
            let Some(chr) = wcm.chr_ins_by_handle_mut(&boss) else {
                st.phase = Phase::Idle;
                return None;
            };
            // lying limp, then blending back into his own animation
            let left = until.saturating_duration_since(Instant::now()).as_secs_f32();
            if left > GET_UP {
                set_ragdoll(chr, RAGDOLL_FULL);
            } else if left > 0.0 {
                set_ragdoll(chr, RAGDOLL_FULL * left / GET_UP);
            } else {
                set_ragdoll(chr, 0.0);
                log("swing: boss back on his feet");
                st.phase = Phase::Idle;
            }
            None
        }
        Phase::Held { boss, reach } => {
            let Some(chr) = wcm.chr_ins_by_handle_mut(&boss) else {
                st.phase = Phase::Idle;
                return None;
            };
            let holding = action == ACT_PICKING_UP_BOWSER || action == ACT_HOLDING_BOWSER;
            let released = action == ACT_RELEASING_BOWSER;
            let ph = &mut chr.modules.physics;
            if holding || START.load(std::sync::atomic::Ordering::Relaxed) {
                // at arm's length where Mario faces, a little off the ground, facing away (Bowser's
                // tail is in Mario's hands)
                let p = mario + fwd * reach + Vec3::Y * 0.4;
                ph.position = HavokPosition(p.x, p.y, p.z, 0.0);
                ph.chr_proxy_pos_update_requested = true;
                ph.gravity_disabled = true;
                let q = glam::Quat::from_rotation_y(std::f32::consts::PI - face);
                ph.orientation = eldenring::rotation::Quaternion(q.x, q.y, q.z, q.w);
                None
            } else if released {
                // thrown where Mario faces, as fast as he was swinging
                let speed = (st.spin.abs() * reach * 1.5).clamp(10.0, 40.0);
                let vel = fwd * speed + Vec3::Y * 9.0;
                let pos = Vec3::new(ph.position.0, ph.position.1, ph.position.2);
                log(format!("swing: thrown at {speed:.1} m/s"));
                let radius = (reach - 0.9).max(0.4);
                st.phase = Phase::Flying { boss, pos, vel, since: Instant::now(), radius };
                None
            } else {
                // let go some other way (hurt, fell): just drop him
                ph.gravity_disabled = false;
                st.phase = Phase::Idle;
                None
            }
        }
        Phase::Flying { boss, pos, vel, since, radius } => {
            let Some(chr) = wcm.chr_ins_by_handle_mut(&boss) else {
                st.phase = Phase::Idle;
                return None;
            };
            let ph = &mut chr.modules.physics;
            let mut vel = vel;
            vel.y -= GRAVITY * dt;
            let next = pos + vel * dt;
            // his body's leading edge against the map: walls, rocks, the ground
            let lead = (next - pos).normalize_or_zero() * radius;
            let impact = hit(pos + Vec3::Y * 0.5, next + lead + Vec3::Y * 0.5).or_else(|| hit(pos + Vec3::Y * 0.5, next - Vec3::Y * 0.1));
            let speed = vel.length();
            match impact {
                Some(h) if since.elapsed().as_secs_f32() > 0.08 => {
                    let back = (h - pos).normalize_or_zero() * radius;
                    let rest = h - back;
                    ph.position = HavokPosition(rest.x, rest.y.max(h.y), rest.z, 0.0);
                    ph.chr_proxy_pos_update_requested = true;
                    ph.gravity_disabled = false;
                    // lying there for a moment if he went limp, then back up (the game doesn't
                    // bring a ragdoll back by itself)
                    // (experiment, boss_ragdoll = on) he collapses where he hit: the game's
                    // blendable ragdoll (state 4), not its death ragdoll (state 2). Not in the air:
                    // the ragdoll's bodies don't get his flight's speed and would stretch him
                    if ragdoll_on() && chr.chr_ctrl.ragdoll_ins != 0 {
                        set_ragdoll(chr, RAGDOLL_FULL);
                        log("swing: ragdoll on impact");
                    }
                    st.phase = if chr.chr_ctrl.chr_ragdoll_state != 0 {
                        Phase::Down { boss, until: Instant::now() + std::time::Duration::from_secs_f32(DOWN_FOR + GET_UP) }
                    } else {
                        Phase::Idle
                    };
                    let pct = IMPACT_MIN + (IMPACT_MAX - IMPACT_MIN) * ((speed - 10.0) / 30.0).clamp(0.0, 1.0);
                    log(format!("swing: boss hit something at {speed:.1} m/s: {pct:.0}% of his HP"));
                    Some((boss, pct))
                }
                _ if since.elapsed().as_secs_f32() > 4.0 => {
                    ph.gravity_disabled = false;
                    set_ragdoll(chr, 0.0);
                    st.phase = Phase::Idle;
                    None
                }
                _ => {
                    let ph = &mut chr.modules.physics;
                    ph.position = HavokPosition(next.x, next.y, next.z, 0.0);
                    ph.chr_proxy_pos_update_requested = true;
                    ph.gravity_disabled = true;
                    st.phase = Phase::Flying { boss, pos: next, vel, since, radius };
                    None
                }
            }
        }
    }
}

/// Mario died or left Mario mode: let go of the boss (his gravity back on).
pub fn reset() {
    let mut st = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let boss = match st.phase {
        Phase::Held { boss, .. } | Phase::Flying { boss, .. } | Phase::Down { boss, .. } => Some(boss),
        Phase::Idle => None,
    };
    if let (Some(boss), Ok(wcm)) = (boss, unsafe { WorldChrMan::instance_mut() }) {
        if let Some(chr) = wcm.chr_ins_by_handle_mut(&boss) {
            chr.modules.physics.gravity_disabled = false;
            if chr.modules.data.hp > 0 {
                set_ragdoll(chr, 0.0);
            }
        }
    }
    st.phase = Phase::Idle;
    st.open.clear();
}
