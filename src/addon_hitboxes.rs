//! Swept addon hitboxes in SM64 coordinates, independent of game pointers.
//! Geometry determines candidates; Combat owns damage and the shared cooldown.
use std::collections::{HashMap, HashSet};

const CAP_RADIUS: f32 = 30.0;
const SONIC_RADIUS: f32 = 45.0;
const WATER_RADIUS: f32 = 12.0;
const WATER_INTERVAL: u32 = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Cap,
    Squirt,
    Sonic,
}

#[derive(Clone, Copy)]
pub struct Target {
    pub key: u64,
    pub feet: [f32; 3],
    pub radius: f32,
    pub height: f32,
    pub index: usize,
}

#[derive(Clone, Copy)]
pub struct Frame {
    pub tick: u32,
    pub mario_previous: [f32; 3],
    pub mario_current: [f32; 3],
    pub cap: Option<[f32; 3]>,
    /// Only present on an actual dash movement step; identifies its native burst.
    pub sonic_burst: Option<u32>,
    pub squirt: Option<([f32; 3], [f32; 3])>,
}

#[derive(Default)]
pub struct Hitboxes {
    cap_previous: Option<[f32; 3]>,
    cap_hits: HashSet<u64>,
    sonic_hits: HashSet<u64>,
    sonic_previous: Option<u32>,
    water_hits: HashMap<u64, u32>,
}

impl Hitboxes {
    pub fn new() -> Self {
        Self::default()
    }
    /// Call when Mario is unavailable, controls are suspended, or the world origin changes.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// `visible` tests map obstruction from the attack source to the nearest
    /// target-cylinder contact (not its center, which may be behind its own body).
    /// A missing cap ends its throw; changing Sonic generation starts a new burst.
    pub fn step(
        &mut self,
        frame: Frame,
        targets: &[Target],
        mut visible: impl FnMut([f32; 3], [f32; 3]) -> bool,
    ) -> Vec<(usize, Kind)> {
        if frame.cap.is_none() {
            self.cap_hits.clear();
        }
        if frame.sonic_burst.is_none() || frame.sonic_burst != self.sonic_previous {
            self.sonic_hits.clear();
        }
        self.sonic_previous = frame.sonic_burst;
        self.water_hits
            .retain(|_, tick| frame.tick.wrapping_sub(*tick) < WATER_INTERVAL);
        let mut out = Vec::new();
        for target in targets {
            if !valid_target(target) {
                continue;
            }
            if let Some(cap) = frame.cap {
                let previous = self.cap_previous.unwrap_or(cap);
                if !self.cap_hits.contains(&target.key)
                    && swept_sphere(previous, cap, CAP_RADIUS, target, 0.0, 0.0)
                    && visible(previous, contact(previous, target))
                {
                    self.cap_hits.insert(target.key);
                    out.push((target.index, Kind::Cap));
                }
            }
            // Mario's 100-unit tall capsule has sphere centers 45..55 above his feet.
            let dash_source = [
                frame.mario_previous[0],
                frame.mario_previous[1] + 50.0,
                frame.mario_previous[2],
            ];
            if frame.sonic_burst.is_some()
                && !self.sonic_hits.contains(&target.key)
                && swept_sphere(
                    frame.mario_previous,
                    frame.mario_current,
                    SONIC_RADIUS,
                    target,
                    -55.0,
                    -45.0,
                )
                && visible(dash_source, contact(dash_source, target))
            {
                self.sonic_hits.insert(target.key);
                out.push((target.index, Kind::Sonic));
            }
            if let Some((start, end)) = frame.squirt {
                if !self.water_hits.contains_key(&target.key)
                    && swept_sphere(start, end, WATER_RADIUS, target, 0.0, 0.0)
                    && visible(start, contact(start, target))
                {
                    self.water_hits.insert(target.key, frame.tick);
                    out.push((target.index, Kind::Squirt));
                }
            }
        }
        self.cap_previous = frame.cap.filter(|p| p.iter().all(|v| v.is_finite()));
        out
    }
}

fn valid_target(t: &Target) -> bool {
    t.feet.iter().all(|x| x.is_finite())
        && t.radius.is_finite()
        && t.radius > 0.0
        && t.height.is_finite()
        && t.height > 0.0
}

fn contact(source: [f32; 3], target: &Target) -> [f32; 3] {
    let dx = source[0] - target.feet[0];
    let dz = source[2] - target.feet[2];
    let distance = (dx * dx + dz * dz).sqrt();
    let scale = if distance > target.radius {
        target.radius / distance
    } else {
        1.0
    };
    [
        target.feet[0] + dx * scale,
        source[1].clamp(target.feet[1], target.feet[1] + target.height),
        target.feet[2] + dz * scale,
    ]
}

/// Minimum distance from a swept sphere center to a finite vertical cylinder.
/// Squared distance to a convex body along a segment is convex; minimizing it
/// catches a fast cap/dash that crossed a target between simulation ticks and
/// retains the cylinder's vertical bounds rather than flattening everything.
fn swept_sphere(
    start: [f32; 3],
    end: [f32; 3],
    radius: f32,
    target: &Target,
    bottom_offset: f32,
    top_offset: f32,
) -> bool {
    if start.iter().chain(end.iter()).any(|v| !v.is_finite()) {
        return false;
    }
    let distance = |t: f32| {
        let p = std::array::from_fn::<_, 3, _>(|i| start[i] + (end[i] - start[i]) * t);
        let horizontal = ((p[0] - target.feet[0]).powi(2) + (p[2] - target.feet[2]).powi(2)).sqrt();
        let radial = (horizontal - target.radius).max(0.0);
        let bottom = target.feet[1] + bottom_offset;
        let top = target.feet[1] + target.height + top_offset;
        let vertical = (bottom - p[1]).max(p[1] - top).max(0.0);
        radial * radial + vertical * vertical
    };
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..40 {
        let a = lo + (hi - lo) / 3.0;
        let b = hi - (hi - lo) / 3.0;
        if distance(a) <= distance(b) {
            hi = b;
        } else {
            lo = a;
        }
    }
    distance((lo + hi) * 0.5)
        .min(distance(0.0))
        .min(distance(1.0))
        <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(tick: u32) -> Frame {
        Frame {
            tick,
            mario_previous: [-400.0, 0.0, 0.0],
            mario_current: [400.0, 0.0, 0.0],
            cap: None,
            sonic_burst: None,
            squirt: None,
        }
    }
    fn target() -> Target {
        Target {
            key: 10,
            feet: [0.0; 3],
            radius: 30.0,
            height: 180.0,
            index: 4,
        }
    }
    #[test]
    fn cap_sweeps_between_frames_and_hits_once_per_throw() {
        let mut h = Hitboxes::new();
        let t = [target()];
        let mut f = frame(0);
        f.cap = Some([-400.0, 90.0, 0.0]);
        assert!(h.step(f, &t, |_, _| true).is_empty());
        f.cap = Some([400.0, 90.0, 0.0]);
        assert_eq!(h.step(f, &t, |_, _| true), [(4, Kind::Cap)]);
        f.cap = Some([0.0, 90.0, 0.0]);
        assert!(h.step(f, &t, |_, _| true).is_empty());
        f.cap = None;
        h.step(f, &t, |_, _| true);
        f.cap = Some([0.0, 90.0, 0.0]);
        assert_eq!(h.step(f, &t, |_, _| true), [(4, Kind::Cap)]);
    }
    #[test]
    fn vertical_misses_and_map_obstruction_are_rejected() {
        let mut h = Hitboxes::new();
        let t = [target()];
        let mut f = frame(0);
        f.cap = Some([0.0, 220.0, 0.0]);
        assert!(h.step(f, &t, |_, _| true).is_empty());
        h.reset();
        f.cap = Some([0.0, 90.0, 0.0]);
        assert!(h.step(f, &t, |_, _| false).is_empty());
        // A rejected obstruction must not burn this target's throw entitlement.
        assert_eq!(h.step(f, &t, |_, _| true), [(4, Kind::Cap)]);
    }
    #[test]
    fn dash_capsule_sweeps_without_multiple_hits_or_index_identity() {
        let mut h = Hitboxes::new();
        let mut t = [target()];
        let mut f = frame(0);
        f.sonic_burst = Some(8);
        assert_eq!(h.step(f, &t, |_, _| true), [(4, Kind::Sonic)]);
        t[0].index = 99;
        assert!(h.step(f, &t, |_, _| true).is_empty());
        f.sonic_burst = None;
        h.step(f, &t, |_, _| true);
        f.sonic_burst = Some(8);
        assert_eq!(h.step(f, &t, |_, _| true), [(99, Kind::Sonic)]);
        h.reset();
        f.mario_previous[1] = 220.0;
        f.mario_current[1] = 220.0;
        assert!(h.step(f, &t, |_, _| true).is_empty());
    }
    #[test]
    fn water_repeat_is_bounded_even_after_release_and_tick_wrap() {
        let mut h = Hitboxes::new();
        let t = [target()];
        let mut f = frame(u32::MAX - 5);
        f.squirt = Some(([-400.0, 90.0, 0.0], [400.0, 90.0, 0.0]));
        assert_eq!(h.step(f, &t, |_, _| true), [(4, Kind::Squirt)]);
        for elapsed in 1..12 {
            f.tick = (u32::MAX - 5).wrapping_add(elapsed);
            assert!(h.step(f, &t, |_, _| true).is_empty());
        }
        f.squirt = None;
        h.step(f, &t, |_, _| true);
        f.tick = 6;
        f.squirt = Some(([-400.0, 90.0, 0.0], [400.0, 90.0, 0.0]));
        assert_eq!(h.step(f, &t, |_, _| true), [(4, Kind::Squirt)]);
    }
    #[test]
    fn rounded_cylinder_corners_do_not_become_square_hitboxes() {
        let t = target();
        assert!(!swept_sphere(
            [50.0, 200.0, 0.0],
            [50.0, 200.0, 0.0],
            25.0,
            &t,
            0.0,
            0.0
        ));
        assert!(swept_sphere(
            [45.0, 195.0, 0.0],
            [45.0, 195.0, 0.0],
            25.0,
            &t,
            0.0,
            0.0
        ));
    }
    #[test]
    fn invalid_coordinates_and_dimensions_do_not_hit() {
        let mut h = Hitboxes::new();
        let mut f = frame(0);
        f.cap = Some([f32::NAN, 90.0, 0.0]);
        assert!(h.step(f, &[target()], |_, _| true).is_empty());
        let mut t = target();
        t.radius = -1.0;
        f.cap = Some([0.0, 90.0, 0.0]);
        assert!(h.step(f, &[t], |_, _| true).is_empty());
    }
    #[test]
    fn occlusion_rays_end_at_the_near_body_surface() {
        let mut h = Hitboxes::new();
        let mut f = frame(0);
        f.squirt = Some(([-400.0, 90.0, 0.0], [400.0, 90.0, 0.0]));
        assert_eq!(
            h.step(f, &[target()], |start, end| {
                assert_eq!(start, [-400.0, 90.0, 0.0]);
                assert!((end[0] + 30.0).abs() < 0.001);
                assert_eq!([end[1], end[2]], [90.0, 0.0]);
                true
            }),
            [(4, Kind::Squirt)]
        );
    }
    #[test]
    fn air_dash_after_rolling_is_a_new_burst_but_charge_is_not_an_attack() {
        let mut h = Hitboxes::new();
        let mut f = frame(0);
        f.sonic_burst = Some(8);
        assert_eq!(h.step(f, &[target()], |_, _| true), [(4, Kind::Sonic)]);
        f.sonic_burst = Some(9);
        assert_eq!(h.step(f, &[target()], |_, _| true), [(4, Kind::Sonic)]);
        assert!(h.step(f, &[target()], |_, _| true).is_empty());
        f.sonic_burst = None;
        assert!(h.step(f, &[target()], |_, _| true).is_empty());
    }
}
