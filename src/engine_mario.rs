//! Mario rendered by the game itself: the Vagabond chest piece (bd_m_1280, built by
//! assets/flver.rs) carries Mario's mesh, each SM64 body part skinned 100% to one
//! skeleton bone with an identity bind. Every frame we write SM64's part matrices into those
//! bones' model pose (character space), after the game's animation and before rendering.

use std::sync::Mutex;

use glam::{Mat3, Quat, Vec3};

use crate::{explore, log};

/// The asset builder and renderer share one part-to-bone map.
const PART_BONES: [&str; PARTS] = crate::assets::flver::PART_BONES;
/// SM64's body parts (0..16) plus the eye variants (16..20).
pub const SM64_PARTS: usize = 16;
pub const PARTS: usize = 30;
/// SM64's right hand part, and our peace-sign copy of it
const RIGHT_HAND: usize = 9;
const PEACE: usize = 20;
/// Triangles SM64 draws for the right hand as a fist (the peace sign has more)
pub const FIST_TRIANGLES: usize = 46;
const HEAD: usize = 3;
/// SM64 eye texture cells for the variant parts 16..20
const EYE_CELLS: [u8; 4] = [5, 6, 7, 8];
/// Into the head, in the head part's mesh space (metres): minus the eyes' mean normal (0.185, 0.983, 0
/// in SM64's head space, X mirrored) times 6 mm.
const EYE_TUCK: Vec3 = Vec3::new(0.185 * 0.006, -0.983 * 0.006, 0.0);
/// Mario's model scale inside SM64's part matrices (the mesh already has it baked in).
const MODEL_SCALE: f32 = 0.25;

/// One part's transform relative to Mario, in (mirrored) ER world axes, metres.
#[derive(Clone, Copy)]
pub struct PartPose {
    pub rot: Quat,
    pub pos: Vec3,
    /// ~0 hides the part (unused eye variants)
    pub scale: f32,
}

/// Part poses relative to Mario from one SM64 tick's matrices; None if the layout is unexpected.
pub fn relative_parts(mats: &[f32], count: i32, mario: [f32; 3], eye_cell: u8, peace: bool) -> Option<[PartPose; PARTS]> {
    if (count as usize) < SM64_PARTS || mats.len() < SM64_PARTS * 16 {
        return None;
    }
    let mut out = [PartPose { rot: Quat::IDENTITY, pos: Vec3::ZERO, scale: 1.0 }; PARTS];
    for (i, p) in out.iter_mut().enumerate().take(SM64_PARTS).skip(1) {
        let m = &mats[i * 16..i * 16 + 16];
        // row-vector storage: rows of M are the columns of the column-vector matrix
        let a = Mat3::from_cols(Vec3::new(m[0], m[1], m[2]), Vec3::new(m[4], m[5], m[6]), Vec3::new(m[8], m[9], m[10]))
            * (1.0 / MODEL_SCALE);
        // SM64 grows the fist on a punch and the foot on a kick (up to 3x): that scale is in the
        // matrix on top of the model scale
        let grow = (a.x_axis.length() + a.y_axis.length() + a.z_axis.length()) / 3.0;
        p.scale = if grow.is_finite() && grow > 0.1 { grow } else { 1.0 };
        let q = Quat::from_mat3(&(a * (1.0 / p.scale))).normalize();
        // SM64 is mirrored on X: S R S
        p.rot = Quat::from_xyzw(q.x, -q.y, -q.z, q.w);
        let t = Vec3::new(m[12] - mario[0], m[13] - mario[1], m[14] - mario[2]) * crate::SCALE;
        p.pos = Vec3::new(-t.x, t.y, t.z);
    }
    // right hand: fist or peace sign (the hidden one shrinks; it only swaps twice per star dance)
    out[PEACE] = PartPose { scale: if peace { out[RIGHT_HAND].scale } else { 0.01 }, ..out[RIGHT_HAND] };
    if peace {
        out[RIGHT_HAND].scale = 0.01;
    }
    // eyes ride the head; only the variant SM64 is drawing is visible
    for (k, &cell) in EYE_CELLS.iter().enumerate() {
        // hidden variants stay full size, tucked 6 mm into the head behind the visible one (scaling
        // them away made the game's motion blur smear every blink; zero scale also darkened the model)
        let head = out[HEAD];
        let pos = if cell == eye_cell { head.pos } else { head.pos + head.rot * EYE_TUCK };
        out[SM64_PARTS + k] = PartPose { pos, ..head };
    }
    let fludd = crate::ap_fludd::visual();
    let torso = out[2];
    for (part, shown) in [
        (crate::assets::fludd::BODY, fludd.enabled),
        (crate::assets::fludd::HOVER, fludd.enabled && fludd.selected_nozzle == 1),
        (crate::assets::fludd::ROCKET, fludd.enabled && fludd.selected_nozzle == 2),
        (crate::assets::fludd::TURBO, fludd.enabled && fludd.selected_nozzle == 4),
        (crate::assets::fludd::JETS, fludd.enabled && fludd.active && fludd.selected_nozzle == 1),
        (crate::assets::fludd::ROCKET_JET, fludd.enabled && fludd.active && fludd.selected_nozzle == 2),
        (crate::assets::fludd::TURBO_JET, fludd.enabled && fludd.active && fludd.selected_nozzle == 4),
    ] {
        out[part] = PartPose { scale: if shown { torso.scale } else { 0.001 }, ..torso };
    }
    let cappy = crate::ap_cappy::visual();
    let head = out[HEAD];
    let cap = if cappy.flying {
        // Cap geometry uses the original head axes (-X up, +Y front, Z across).
        // Keep it horizontal as it spins, with the native projectile at its center.
        let axes = Mat3::from_cols(-Vec3::Y, Vec3::Z, -Vec3::X);
        let rot = Quat::from_rotation_y(-cappy.spin_yaw) * Quat::from_mat3(&axes);
        let delta = (Vec3::from_array(cappy.position) - Vec3::from_array(mario)) * crate::SCALE;
        let center = Vec3::new(-delta.x, delta.y, delta.z);
        PartPose { rot, pos: center - rot * Vec3::from_array(crate::assets::cappy::CENTER), scale: 1.0 }
    } else {
        head
    };
    out[crate::assets::cappy::CAP] = cap;
    out[crate::assets::cappy::EYES] = PartPose { scale: if cappy.enabled { cap.scale } else { 0.001 }, ..cap };
    Some(out)
}

pub fn blend(a: &[PartPose; PARTS], b: &[PartPose; PARTS], t: f32) -> [PartPose; PARTS] {
    let mut out = *b;
    for i in 0..PARTS {
        if a[i].pos.distance(b[i].pos) < 2.0 {
            out[i].pos = a[i].pos.lerp(b[i].pos, t);
            out[i].rot = a[i].rot.slerp(b[i].rot, t);
        }
    }
    out
}

/// The pose to show this frame, in character space (set by the Mario frame, applied by `apply`).
pub static POSE: Mutex<Option<[PartPose; PARTS]>> = Mutex::new(None);

/// Converts world-relative part poses to the character space of a character facing `char_rot`.
pub fn to_character(parts: &[PartPose; PARTS], char_rot: Quat) -> [PartPose; PARTS] {
    let inv = char_rot.inverse();
    parts.map(|p| PartPose { rot: (inv * p.rot).normalize(), pos: inv * p.pos, scale: p.scale })
}

struct BoneMap {
    skeleton: usize,
    bones: [usize; PARTS],
}

static BONES: Mutex<Option<BoneMap>> = Mutex::new(None);

fn c_str(p: usize) -> Option<String> {
    if p == 0 || !explore::readable(p & !7, 72) {
        return None;
    }
    let mut s = Vec::new();
    for k in 0..64 {
        let b = unsafe { *((p + k) as *const u8) };
        if b == 0 {
            break;
        }
        s.push(b);
    }
    String::from_utf8(s).ok()
}

fn map_bones(skeleton: usize) -> Option<BoneMap> {
    let names = explore::read_u64(skeleton + 0x30)? as usize;
    let count = (explore::read_u64(skeleton + 0x38)? as u32 as usize).min(1024);
    if names == 0 || !explore::readable(names, count * 16) {
        return None;
    }
    let all: Vec<String> = (0..count).map(|i| c_str(unsafe { *((names + i * 16) as *const usize) } & !1).unwrap_or_default()).collect();
    let mut bones = [usize::MAX; PARTS];
    for (i, name) in PART_BONES.iter().enumerate().skip(1) {
        let Some(b) = all.iter().position(|n| n == name) else {
            log(format!("engine mario: bone {name} not found in {count} names (first {:?}, raw {:#x})", all.iter().take(5).collect::<Vec<_>>(), unsafe { *(names as *const usize) }));
            return None;
        };
        bones[i] = b;
    }
    log(format!("engine mario: skeleton {skeleton:#x} ({count} bones), part bones {:?}", &bones[1..]));
    Some(BoneMap { skeleton, bones })
}

#[derive(Clone, Copy, PartialEq)]
struct PoseLayout {
    imp: usize,
    skeleton: usize,
    model: usize,
    local: usize,
    parents: usize,
    count: usize,
}

static LAYOUT: Mutex<Option<(usize, PoseLayout)>> = Mutex::new(None);

/// The player's pose buffers, validated once per change of any pointer.
fn pose_layout(chr: usize, raw: impl Fn(usize) -> usize) -> Option<PoseLayout> {
    let mut cached = LAYOUT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((c, l)) = *cached {
        // cheap check: same character, same importer, same buffers
        if c == chr && raw(chr + 0x398) == l.imp && raw(l.imp + 0x60) == l.model && raw(l.imp + 0x50) == l.local {
            return Some(l);
        }
    }
    let imp = explore::read_u64(chr + 0x398)? as usize;
    let skeleton = explore::read_u64(imp + 0x48)? as usize;
    let model = explore::read_u64(imp + 0x60)? as usize;
    let count = explore::read_u64(imp + 0x68)? as u32 as usize;
    let local = explore::read_u64(imp + 0x50)? as usize;
    let local_count = explore::read_u64(imp + 0x58)? as u32 as usize;
    let parents = explore::read_u64(skeleton + 0x20)? as usize;
    let parent_count = explore::read_u64(skeleton + 0x28)? as u32 as usize;
    if count == 0 || count > 1024 || local_count != count || parent_count != count {
        return None;
    }
    if !explore::readable(model, count * 0x30) || !explore::readable(local, count * 0x30) || !explore::readable(parents & !7, count * 2 + 8) {
        return None;
    }
    let l = PoseLayout { imp, skeleton, model, local, parents, count };
    *cached = Some((chr, l));
    Some(l)
}

/// Where the game's character animation job has just written a pose (found with a hardware
/// watchpoint on a bone; run for every character, on the game's worker threads): right after the call that
/// writes the model pose (the job goes on to hand the bones to rendering, so later is too late),
/// and the job's return as a fallback.
const ANIM_DONE_RVAS: [usize; 2] = [0x41da14, 0x402194];

/// The renderer draws the previous frame while the next one updates: without this, Mario's bones
/// sit in the Tarnished's animated pose from the animation job until our next task runs, and
/// anything reading them in between (shadows) gets the Tarnished's shape. Putting our pose back the
/// moment the animation job is done closes that window.
pub unsafe fn install_anim_hook() {
    use ilhook::x64::{CallbackOption, HookFlags, hook_closure_jmp_back};
    let Ok(base) = (unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(None) }) else { return };
    for rva in ANIM_DONE_RVAS {
        let hook = |_: *mut ilhook::x64::Registers| {
            let _ = std::panic::catch_unwind(reassert);
        };
        match unsafe { hook_closure_jmp_back(base.0 as usize + rva, hook, CallbackOption::None, HookFlags::empty()) } {
            Ok(h) => {
                std::mem::forget(h);
                log(format!("engine mario: hooked the animation job at +{rva:#x}"));
            }
            Err(e) => log(format!("engine mario: animation hook at +{rva:#x} failed: {e:?}")),
        }
    }
}

/// Our pose was overwritten (the head bone isn't where we put it): write it again.
fn reassert() {
    if !crate::ENABLED.load(std::sync::atomic::Ordering::Relaxed) || POSE.lock().unwrap_or_else(|e| e.into_inner()).is_none() {
        return;
    }
    // (waits if one of our pose tasks is writing right now: it runs alongside the animation jobs,
    // and the animation may land after its write)
    let Some((chr, l)) = *LAYOUT.lock().unwrap_or_else(|e| e.into_inner()) else { return };
    let Some(b) = BONES.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|m| m.bones[HEAD]) else { return };
    let Some(want) = *LAST_HEAD.lock().unwrap_or_else(|e| e.into_inner()) else { return };
    // the cached character may be gone after a load: only the live player, with the same pose
    // buffers (no memory checks here: this runs on the game's animation workers many times a frame)
    use fromsoftware_shared::FromStatic;
    let live = unsafe { eldenring::cs::WorldChrMan::instance() }
        .ok()
        .and_then(|w| w.main_player.as_ref())
        .map(|p| &p.chr_ins as *const _ as usize);
    let raw = |a: usize| unsafe { *(a as *const usize) };
    if live != Some(chr) || b >= l.count || raw(chr + 0x398) != l.imp || raw(l.imp + 0x60) != l.model {
        return;
    }
    let now = unsafe { *((l.model + b * 0x30) as *const [f32; 3]) };
    if Vec3::from(now).distance(Vec3::from(want)) > 1e-4 {
        apply(chr);
    }
}

/// Where apply last put the head bone (model space).
static LAST_HEAD: Mutex<Option<[f32; 3]>> = Mutex::new(None);

/// Writes the Mario pose into the player's render skeleton (after the game's animation, in every
/// task group up to drawing).
pub fn apply(chr: usize) {
    let Some(pose) = *POSE.lock().unwrap_or_else(|e| e.into_inner()) else { return };
    // ChrIns+0x398 CSFD4LocationHkaPoseImporter: +0x48 hkaSkeleton, +0x50 local / +0x60 model pose
    // (hkQsTransform, 0x30 each, counts at +0x58/+0x68); hkaSkeleton +0x20 parent indices.
    // Validating memory costs a slow Wine server call, and this runs ~9 times a frame: validate once,
    // then only re-validate when one of the pointers changes.
    let raw = |a: usize| unsafe { *(a as *const usize) };
    let Some(layout) = pose_layout(chr, raw) else { return };
    let PoseLayout { imp, skeleton, model, local, parents, count } = layout;
    let mut guard = BONES.lock().unwrap_or_else(|e| e.into_inner());
    if guard.as_ref().is_none_or(|b| b.skeleton != skeleton) {
        *guard = map_bones(skeleton);
    }
    let Some(map) = guard.as_ref() else {
        static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            log(format!("engine mario: bone mapping failed (importer {imp:#x}, skeleton {skeleton:#x})"));
        }
        return;
    };
    type Qs = (Vec3, Quat);
    let read = |base: usize, b: usize| -> Qs {
        let v = unsafe { *((base + b * 0x30) as *const [f32; 12]) };
        (Vec3::new(v[0], v[1], v[2]), Quat::from_xyzw(v[4], v[5], v[6], v[7]).normalize())
    };
    let write = |base: usize, b: usize, t: Qs, s: f32| unsafe {
        *((base + b * 0x30) as *mut [f32; 12]) = [t.0.x, t.0.y, t.0.z, 0.0, t.1.x, t.1.y, t.1.z, t.1.w, s, s, s, 0.0];
    };
    let n = count.min(512);
    let parent_of = |b: usize| unsafe { *((parents + b * 2) as *const i16) };
    let mut pinned: [Option<(Qs, f32)>; 512] = [None; 512];
    // bones that must keep scale 1: Mario's bones and their ancestors (the rest is scaled to 0,
    // which hides everything else on the Tarnished: weapons, face, fingers...)
    let mut keep = [false; 512];
    for i in 1..PARTS {
        let mut b = map.bones[i];
        if b >= n {
            continue;
        }
        pinned[b] = Some(((pose[i].pos, pose[i].rot), pose[i].scale));
        while b < n && !keep[b] {
            keep[b] = true;
            let p = parent_of(b);
            if p < 0 {
                break;
            }
            b = p as usize;
        }
    }
    let mut world: Vec<Qs> = Vec::with_capacity(n);
    for b in 0..n {
        let p = parent_of(b);
        let parent: Qs = if p >= 0 && (p as usize) < b { world[p as usize] } else { (Vec3::ZERO, Quat::IDENTITY) };
        let scale = match pinned[b] {
            Some((_, s)) => s,
            None if keep[b] => 1.0,
            None => 0.0,
        };
        let m = match pinned[b] {
            Some((want, s)) => {
                let inv = parent.1.inverse();
                write(local, b, (inv * (want.0 - parent.0), (inv * want.1).normalize()), s);
                want
            }
            None if keep[b] => {
                let l = read(local, b);
                (parent.0 + parent.1 * l.0, (parent.1 * l.1).normalize())
            }
            None => {
                // hidden: scaled to 0 where the animation puts it (not moved: the game aims the camera
                // at bones like the head, e.g. when resting at a grace)
                let l = read(local, b);
                write(local, b, l, 0.0);
                (parent.0 + parent.1 * l.0, (parent.1 * l.1).normalize())
            }
        };
        write(model, b, m, scale);
        world.push(m);
    }
    if let Some(head) = world.get(map.bones[HEAD]) {
        *LAST_HEAD.lock().unwrap_or_else(|e| e.into_inner()) = Some(head.0.into());
    }
}
