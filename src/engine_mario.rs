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
pub const PARTS: usize = 32;
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
        (crate::assets::fludd::TURBO, fludd.enabled && matches!(fludd.selected_nozzle, 4 | 8)),
        (crate::assets::fludd::JETS, fludd.enabled && fludd.active && fludd.selected_nozzle == 1),
        (crate::assets::fludd::ROCKET_JET, fludd.enabled && fludd.active && fludd.selected_nozzle == 2),
        (crate::assets::fludd::TURBO_JET, fludd.enabled && fludd.active && matches!(fludd.selected_nozzle, 4 | 8)),
    ] {
        out[part] = PartPose { scale: if shown { torso.scale } else { 0.001 }, ..torso };
    }
    // Squirt reuses the authored horizontal outlet/stream facing forward, avoiding extra
    // skinned bones or a model rebuild. The torso's local -X is up and +Y is forward.
    if fludd.selected_nozzle == 8 {
        for part in [crate::assets::fludd::TURBO, crate::assets::fludd::TURBO_JET] {
            out[part].rot = torso.rot * Quat::from_rotation_x(std::f32::consts::PI);
        }
        if fludd.enabled && fludd.active {
            out[crate::assets::fludd::TURBO_JET] = squirt_stream_pose(torso);
        }
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
    let (capped, flying_cap, bare) = crate::assets::cappy::shown_parts(cappy.flying);
    out[HEAD] = PartPose { scale: if capped { head.scale } else { 0.001 }, ..head };
    out[crate::assets::cappy::CAP] = PartPose { scale: if flying_cap { cap.scale } else { 0.001 }, ..cap };
    out[crate::assets::cappy::BARE_HEAD] = PartPose { scale: if bare { head.scale } else { 0.001 }, ..head };
    out[crate::assets::addons::FLASK] = PartPose { scale: 0.001, ..out[RIGHT_HAND] };
    out[crate::assets::addons::BOARD] = PartPose { scale: 0.001, ..out[2] };
    Some(out)
}

/// Original skating stance, with both feet on the separate deck and arms balancing.
/// Native mechanics own movement/actions; this changes only the rendered pose.
pub fn skate_pose(parts: &mut [PartPose; PARTS], speed: f32, airborne: bool, lean: f32,
    push_phase: f32, trick: u32, trick_progress: f32) {
    let torso = parts[2];
    let mut up = (torso.rot * -Vec3::X).normalize_or_zero();
    if up.length_squared() < 0.5 { up = Vec3::Y; }
    let forward = (torso.rot * Vec3::Y).normalize_or_zero();
    let across = forward.cross(up).normalize_or_zero();
    let axes = Mat3::from_cols(-up, forward, across);
    let rot = Quat::from_mat3(&axes).normalize();
    let feet = (parts[12].pos + parts[15].pos) * 0.5;
    let board = PartPose { rot, pos: feet - up * 0.035, scale: 1.0 };
    parts[crate::assets::addons::BOARD] = board;
    let crouch = if airborne { 0.08 } else { 0.035 + speed.abs().min(100.0) * 0.00045 };
    // Native animation supplies a neutral skating body; lower the upper body into a stance.
    for part in [1,2,3,4,5,6,7,8,9,16,17,18,19,20,21,22,23,24,25,26,27] {
        parts[part].pos -= up * crouch;
    }
    // Keep cap/bare-head poses attached; an airborne Cappy projectile is independent.
    if parts[crate::assets::cappy::BARE_HEAD].scale < 0.01 {
        parts[crate::assets::cappy::CAP].pos -= up * crouch;
    }
    parts[crate::assets::cappy::BARE_HEAD].pos -= up * crouch;
    for (thigh, shin, shoe, sign) in [(10,11,12,-1.0),(13,14,15,1.0)] {
        let hip = parts[thigh].pos;
        let knee = feet + forward * (sign * 0.14 + 0.055) + across * (sign * 0.04) + up * 0.16;
        let foot = feet + forward * (sign * 0.14) + across * (sign * 0.04);
        let orient = |from: Vec3, to: Vec3| if from.length_squared()>1e-8 && to.length_squared()>1e-8 {
            Quat::from_rotation_arc(from.normalize(),to.normalize())
        } else { Quat::IDENTITY };
        parts[thigh].rot = orient(parts[shin].pos-hip,knee-hip) * parts[thigh].rot;
        parts[shin].rot = orient(parts[shoe].pos-parts[shin].pos,foot-knee) * parts[shin].rot;
        parts[shin].pos = knee;
        parts[shoe].pos = foot;
        parts[shoe].rot = Quat::from_axis_angle(up, sign * 0.85) * parts[shoe].rot;
    }
    for (upper, lower, hand, sign) in [(4,5,6,-1.0),(7,8,9,1.0)] {
        let shoulder = parts[upper].pos;
        let elbow = shoulder + across * (sign * 0.14) + forward * 0.02 - up * 0.03;
        let wrist = elbow + across * (sign * 0.12) + forward * 0.08;
        let orient = |from: Vec3, to: Vec3| if from.length_squared()>1e-8 && to.length_squared()>1e-8 {
            Quat::from_rotation_arc(from.normalize(),to.normalize())
        } else { Quat::IDENTITY };
        parts[upper].rot = orient(parts[lower].pos-shoulder,elbow-shoulder) * parts[upper].rot;
        parts[lower].rot = orient(parts[hand].pos-parts[lower].pos,wrist-elbow) * parts[lower].rot;
        parts[lower].pos = elbow;
        parts[hand].pos = wrist;
    }
    parts[PEACE].scale = 0.001;
    if !airborne && push_phase.is_finite() && push_phase >= 0.0 {
        let phase = push_phase.clamp(0.0,1.0);
        let stroke = (phase * std::f32::consts::PI).sin().max(0.0);
        let target = parts[12].pos - forward * (0.22 * stroke)
            - across * (0.10 * stroke) - up * (0.07 * stroke);
        let planted = parts[15].pos;
        let balance = -up * (0.10 * stroke) - forward * (0.08 * stroke);
        let cap_flying = parts[crate::assets::cappy::BARE_HEAD].scale >= 0.01;
        for (i, part) in parts.iter_mut().enumerate().skip(1) {
            if i == crate::assets::addons::BOARD || i == crate::assets::addons::FLASK
                || (i == crate::assets::cappy::CAP && cap_flying) { continue; }
            part.pos += balance;
        }
        skate_leg_target(parts,10,11,12,target,forward);
        skate_leg_target(parts,13,14,15,planted,forward);
        // Counterbalance the pushing leg with a small shoulder and hand motion.
        for i in [4,5,6] { parts[i].pos += forward * (0.035 * stroke); }
        for i in [7,8,9] { parts[i].pos -= forward * (0.035 * stroke); }
    }
    if airborne && matches!(trick,1..=3) && trick_progress.is_finite() {
        let phase = trick_progress.clamp(0.0,1.0);
        let lift = (phase * std::f32::consts::PI).sin().max(0.0) * 0.20;
        let cap_flying = parts[crate::assets::cappy::BARE_HEAD].scale >= 0.01;
        for (i, part) in parts.iter_mut().enumerate().skip(1) {
            if i == crate::assets::addons::BOARD || i == crate::assets::addons::FLASK
                || (i == crate::assets::cappy::CAP && cap_flying) { continue; }
            part.pos += up * lift;
        }
        let spin = match trick {
            2 => Quat::from_axis_angle(forward,phase * std::f32::consts::TAU),
            3 => Quat::from_axis_angle(up,phase * std::f32::consts::PI),
            _ => Quat::IDENTITY,
        };
        parts[crate::assets::addons::BOARD].rot = spin * board.rot;
    }
    // Carve around the deck's forward axis; keep the native head, eyes and attachments together.
    // A flying cap remains on its own projectile path, and the hidden flask stays independent.
    let lean = if lean.is_finite() { lean.clamp(-1.0, 1.0) } else { 0.0 };
    let carve = Quat::from_axis_angle(forward, lean * 0.22);
    let cap_flying = parts[crate::assets::cappy::BARE_HEAD].scale >= 0.01;
    for (i, part) in parts.iter_mut().enumerate().skip(1) {
        if i == crate::assets::addons::FLASK || (i == crate::assets::cappy::CAP && cap_flying) { continue; }
        part.pos = board.pos + carve * (part.pos - board.pos);
        part.rot = carve * part.rot;
    }
}

/// Keep the pushing leg's segment lengths while moving its shoe off the deck.
fn skate_leg_target(parts: &mut [PartPose; PARTS], thigh: usize, shin: usize, shoe: usize,
    target: Vec3, bend_axis: Vec3) {
    let hip = parts[thigh];
    let knee = parts[shin];
    let foot = parts[shoe];
    let a = hip.pos.distance(knee.pos);
    let b = knee.pos.distance(foot.pos);
    if a <= 0.001 || b <= 0.001 { return; }
    let direction = (target - hip.pos).normalize_or_zero();
    if direction.length_squared() < 0.5 { return; }
    let distance = hip.pos.distance(target).clamp((a-b).abs()+0.0001,a+b-0.0001);
    let along = (a*a-b*b+distance*distance) / (2.0*distance);
    let height = (a*a-along*along).max(0.0).sqrt();
    let raw_bend = knee.pos - hip.pos;
    let mut bend = (raw_bend-direction*raw_bend.dot(direction)).normalize_or_zero();
    if bend.length_squared() < 0.5 {
        bend = (bend_axis-direction*bend_axis.dot(direction)).normalize_or_zero();
    }
    let joint = hip.pos+direction*along+bend*height;
    let end = hip.pos+direction*distance;
    let orient = |from: Vec3,to: Vec3| if from.length_squared()>1e-8 && to.length_squared()>1e-8 {
        Quat::from_rotation_arc(from.normalize(),to.normalize())
    } else { Quat::IDENTITY };
    parts[thigh].rot = orient(knee.pos-hip.pos,joint-hip.pos) * hip.rot;
    parts[shin].rot = orient(foot.pos-knee.pos,end-joint) * knee.rot;
    parts[shin].pos = joint;
    parts[shoe].pos = end;
}

/// Presentation only: raise the native right arm, sip, then lower it during a flask drink.
/// Call after relative_parts for an active drink; otherwise its default pose hides the bottle.
pub fn flask_pose(parts: &mut [PartPose; PARTS], progress: f32) {
    if !progress.is_finite() { return; }
    let p = progress.clamp(0.0, 1.0);
    let rise = (p / 0.30).min(1.0);
    let lower = ((1.0 - p) / 0.25).min(1.0);
    let t = rise.min(lower);
    let t = t * t * (3.0 - 2.0 * t);
    // Use the visible native head's transform even when Cappy has hidden its capped mesh.
    let head = parts[HEAD];
    let shoulder = parts[7];
    let elbow = parts[8];
    let hand = parts[RIGHT_HAND];
    let bottle_rot = hand.rot.slerp(head.rot * Quat::from_rotation_z(-0.55), t);
    let mouth = head.pos + head.rot * Vec3::new(-0.05, 0.22, 0.0);
    let target = mouth - bottle_rot * Vec3::from_array(crate::assets::addons::FLASK_MOUTH);
    let a = shoulder.pos.distance(elbow.pos);
    let b = elbow.pos.distance(hand.pos);
    let mut wrist = hand.pos.lerp(target, t);
    if a > 0.001 && b > 0.001 {
        let direction = (wrist - shoulder.pos).normalize_or_zero();
        let distance = shoulder.pos.distance(wrist).clamp((a - b).abs() + 0.0001, a + b - 0.0001);
        wrist = shoulder.pos + direction * distance;
        let along = (a*a - b*b + distance*distance) / (2.0 * distance);
        let height = (a*a - along*along).max(0.0).sqrt();
        let mut bend = elbow.pos - shoulder.pos;
        bend = (bend - direction * bend.dot(direction)).normalize_or_zero();
        if bend.length_squared() < 0.1 {
            let forward = head.rot * Vec3::Y;
            bend = (forward - direction * forward.dot(direction)).normalize_or_zero();
        }
        let joint = shoulder.pos + direction * along + bend * height;
        let orient = |old: Vec3, new: Vec3| {
            if old.length_squared() > 1e-8 && new.length_squared() > 1e-8 {
                Quat::from_rotation_arc(old.normalize(), new.normalize())
            } else { Quat::IDENTITY }
        };
        parts[7].rot = orient(elbow.pos - shoulder.pos, joint - shoulder.pos) * shoulder.rot;
        parts[8].rot = orient(hand.pos - elbow.pos, wrist - joint) * elbow.rot;
        parts[8].pos = joint;
    }
    parts[RIGHT_HAND] = PartPose { rot: bottle_rot, pos: wrist, scale: 1.0 };
    parts[PEACE].scale = 0.001;
    parts[crate::assets::addons::FLASK] = PartPose { rot: bottle_rot, pos: wrist, scale: 1.0 };
}

fn squirt_stream_pose(torso: PartPose) -> PartPose {
    let rot = torso.rot * Quat::from_rotation_x(std::f32::consts::PI);
    let scale = torso.scale * crate::assets::fludd::SQUIRT_STREAM_SCALE;
    let mouth = Vec3::from_array(crate::assets::fludd::STREAM_MOUTH);
    PartPose { rot, scale, pos: torso.pos + rot * mouth * (torso.scale - scale) }
}

#[cfg(test)]
mod skate_tests {
    use super::*;
    fn fixture() -> [PartPose; PARTS] {
        let rot = Quat::from_mat3(&Mat3::from_cols(-Vec3::Y, Vec3::Z, -Vec3::X));
        let mut parts = [PartPose {rot,pos:Vec3::new(0.0,0.5,0.0),scale:1.0}; PARTS];
        parts[12].pos = Vec3::new(-0.1,0.12,0.0);
        parts[15].pos = Vec3::new(0.1,0.12,0.0);
        parts[crate::assets::addons::FLASK].scale = 0.001;
        parts[crate::assets::cappy::BARE_HEAD].scale = 0.001;
        parts
    }
    #[test]
    fn push_recovers_to_deck_and_preserves_both_leg_segment_lengths() {
        let mut rest = fixture();
        skate_pose(&mut rest,50.0,false,0.0,-1.0,0,0.0);
        let upper = rest[10].pos.distance(rest[11].pos);
        let lower = rest[11].pos.distance(rest[12].pos);
        for phase in [0.0,0.25,0.5,0.75,1.0,-1.0,f32::INFINITY,f32::NAN] {
            let mut parts = fixture();
            skate_pose(&mut parts,50.0,false,0.0,phase,0,0.0);
            assert!((parts[10].pos.distance(parts[11].pos)-upper).abs()<1e-4);
            assert!((parts[11].pos.distance(parts[12].pos)-lower).abs()<1e-4);
            assert!(parts[15].pos.distance(rest[15].pos)<1e-4);
            assert!(parts.iter().all(|p| p.pos.is_finite() && p.rot.is_finite()));
            if phase == 0.5 {
                assert!(parts[12].pos.z < rest[12].pos.z - 0.05);
                assert!(parts[12].pos.y < rest[12].pos.y);
            }
            if phase == 0.0 || phase == 1.0 || phase < 0.0 || !phase.is_finite() {
                assert!(parts[12].pos.distance(rest[12].pos)<1e-4);
            }
        }
    }
    #[test]
    fn tricks_spin_only_the_board_and_clear_it_before_returning_to_contact() {
        let mut rest = fixture();
        skate_pose(&mut rest,50.0,true,0.0,-1.0,0,0.0);
        let initial = rest[crate::assets::addons::BOARD];
        for trick in [1,2,3] {
            for phase in [0.0,0.25,0.5,0.75,1.0,20.0,-20.0,f32::NAN] {
                let mut parts = fixture();
                skate_pose(&mut parts,50.0,true,0.0,-1.0,trick,phase);
                let board = parts[crate::assets::addons::BOARD];
                assert!(board.pos.distance(initial.pos)<1e-5);
                assert!(parts.iter().all(|p| p.pos.is_finite() && p.rot.is_finite()));
                if phase == 0.5 {
                    assert!(parts[12].pos.y > board.pos.y+0.20);
                    assert!(parts[15].pos.y > board.pos.y+0.20);
                }
                if phase == 1.0 {
                    assert!(parts[12].pos.distance(rest[12].pos)<1e-4);
                    if trick == 3 {
                        assert!((board.rot*Vec3::Y).dot(initial.rot*Vec3::Y)< -0.999);
                    } else {
                        assert!(board.rot.dot(initial.rot).abs()>0.999);
                    }
                }
                assert_eq!(parts[crate::assets::addons::FLASK].scale,0.001);
            }
        }
    }
    #[test]
    fn stance_plants_both_feet_on_the_deck_and_keeps_flask_independent() {
        let rot = Quat::from_mat3(&Mat3::from_cols(-Vec3::Y, Vec3::Z, -Vec3::X));
        for (airborne, lean) in [(false,0.0),(false,-1.0),(true,1.0),(true,f32::NAN),(false,20.0)] {
            let mut parts = [PartPose {rot, pos:Vec3::new(0.0,0.5,0.0),scale:1.0}; PARTS];
            parts[12].pos = Vec3::new(-0.1,0.12,0.0);
            parts[15].pos = Vec3::new(0.1,0.12,0.0);
            parts[crate::assets::addons::FLASK].scale = 0.001;
            parts[crate::assets::cappy::BARE_HEAD].scale = 0.001;
            let flask = parts[crate::assets::addons::FLASK];
            skate_pose(&mut parts,80.0,airborne,lean,-1.0,0,0.0);
            let board = parts[crate::assets::addons::BOARD];
            assert_eq!(board.scale,1.0);
            for shoe in [12,15] {
                let local = board.rot.inverse() * (parts[shoe].pos - board.pos);
                assert!((local.x + 0.035).abs()<1e-5);
                assert!(local.y.abs()<0.32 && local.z.abs()<0.13);
            }
            assert_eq!(parts[crate::assets::addons::FLASK].pos,flask.pos);
            assert_eq!(parts[crate::assets::addons::FLASK].scale,flask.scale);
            assert!(parts.iter().all(|p| p.pos.is_finite() && p.rot.is_finite()));
        }
    }
}

#[cfg(test)]
mod flask_tests {
    use super::*;
    #[test]
    fn sip_pose_preserves_arm_lengths_and_returns_to_the_native_hand() {
        let base = PartPose { rot: Quat::IDENTITY, pos: Vec3::ZERO, scale: 1.0 };
        let mut original = [base; PARTS];
        original[HEAD].pos = Vec3::new(-0.3, 0.0, 0.0);
        original[7].pos = Vec3::new(0.0, 0.0, 0.20);
        original[8].pos = Vec3::new(0.20, 0.0, 0.20);
        original[RIGHT_HAND].pos = Vec3::new(0.35, 0.10, 0.20);
        let upper = original[7].pos.distance(original[8].pos);
        let lower = original[8].pos.distance(original[RIGHT_HAND].pos);
        for progress in [0.0, 0.15, 0.4, 0.7, 0.9, 1.0] {
            let mut parts = original;
            flask_pose(&mut parts, progress);
            assert!((parts[7].pos.distance(parts[8].pos) - upper).abs() < 1e-4);
            assert!((parts[8].pos.distance(parts[RIGHT_HAND].pos) - lower).abs() < 1e-4);
            let flask = parts[crate::assets::addons::FLASK];
            assert!(flask.pos.is_finite() && flask.rot.is_finite());
            assert!(flask.pos.distance(parts[RIGHT_HAND].pos) < 1e-5);
            if progress == 0.0 || progress == 1.0 {
                assert!(parts[RIGHT_HAND].pos.distance(original[RIGHT_HAND].pos) < 1e-4);
            }
            assert_eq!(parts[HEAD].pos, original[HEAD].pos);
        }
    }
}

/// The damage beam uses the exact endpoints of the displayed nozzle stream.
pub fn squirt_segment(parts: &[PartPose; PARTS], mario: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    jet_segment(parts[crate::assets::fludd::TURBO_JET], mario,
        crate::assets::fludd::STREAM_MOUTH, crate::assets::fludd::STREAM_END)
}

/// Both downward Hover jets follow their rendered torso pose.
pub fn hover_segments(parts: &[PartPose; PARTS], mario: [f32; 3]) -> [([f32; 3], [f32; 3]); 2] {
    std::array::from_fn(|i| jet_segment(parts[crate::assets::fludd::JETS], mario,
        crate::assets::fludd::HOVER_MOUTHS[i], crate::assets::fludd::HOVER_ENDS[i]))
}

fn jet_segment(jet: PartPose, mario: [f32; 3], start: [f32; 3], end: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let to_sm = |local: [f32; 3]| {
        let world = (jet.pos + jet.rot * Vec3::from_array(local) * jet.scale) / crate::SCALE;
        [mario[0] - world.x, mario[1] + world.y, mario[2] + world.z]
    };
    (to_sm(start), to_sm(end))
}

/// Keep the water mesh on the nozzle while shortening it at a map obstruction.
pub fn clip_squirt_stream(parts: &mut [PartPose; PARTS], fraction: f32) {
    let jet = &mut parts[crate::assets::fludd::TURBO_JET];
    let old = jet.scale;
    let fraction = if fraction.is_finite() { fraction.clamp(0.0, 1.0) } else { 0.0 };
    jet.scale = (old * fraction).max(0.001);
    jet.pos += jet.rot * Vec3::from_array(crate::assets::fludd::STREAM_MOUTH) * (old - jet.scale);
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

#[cfg(test)]
mod squirt_tests {
    use super::*;
    #[test]
    fn hover_hitboxes_follow_both_rendered_jets_at_every_yaw() {
        for yaw in [0.0, 0.7, -1.4] {
            // Native torso local -X points up.
            let axes = Mat3::from_cols(-Vec3::Y, Vec3::Z, -Vec3::X);
            let torso = PartPose { rot: Quat::from_rotation_y(yaw) * Quat::from_mat3(&axes), pos: Vec3::new(0.1, 0.8, -0.2), scale: 1.0 };
            let parts = [torso; PARTS];
            let jets = hover_segments(&parts, [100.0, 200.0, -400.0]);
            for (start, end) in jets {
                assert!((start[1] - end[1] - 100.0).abs() < 0.001);
                assert!((start[0] - end[0]).abs() < 0.001);
                assert!((start[2] - end[2]).abs() < 0.001);
            }
            assert!((Vec3::from_array(jets[0].0).distance(Vec3::from_array(jets[1].0)) - 32.0).abs() < 0.001);
        }
    }

    #[test]
    fn enlarged_spray_keeps_its_mouth_at_the_nozzle_and_hitbox_at_the_visible_tip() {
        for yaw in [0.0, 0.7, -1.4] {
            let torso = PartPose { rot: Quat::from_rotation_y(yaw), pos: Vec3::new(0.1, 0.8, -0.2), scale: 1.0 };
            let jet = squirt_stream_pose(torso);
            let mouth = Vec3::from_array(crate::assets::fludd::STREAM_MOUTH);
            let unchanged_mouth = torso.pos + jet.rot * mouth;
            assert!((jet.pos + jet.rot * mouth * jet.scale - unchanged_mouth).length() < 1e-5);
            let mut parts = [torso; PARTS];
            parts[crate::assets::fludd::TURBO_JET] = jet;
            let mario = [100.0, 200.0, -400.0];
            let (start, end) = squirt_segment(&parts, mario);
            assert!((Vec3::from_array(end).distance(Vec3::from_array(start)) - 124.0).abs() < 0.001);
            let visible_tip = jet.pos + jet.rot * Vec3::from_array(crate::assets::fludd::STREAM_END) * jet.scale;
            let world_tip = Vec3::new(-(end[0] - mario[0]), end[1] - mario[1], end[2] - mario[2]) * crate::SCALE;
            assert!(world_tip.distance(visible_tip) < 1e-5);
            clip_squirt_stream(&mut parts, 0.25);
            let (clipped_start, clipped_end) = squirt_segment(&parts, mario);
            assert!(Vec3::from_array(clipped_start).distance(Vec3::from_array(start)) < 0.001);
            assert!((Vec3::from_array(clipped_end).distance(Vec3::from_array(clipped_start)) - 31.0).abs() < 0.001);
        }
    }
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
