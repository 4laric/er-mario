//! A button starts a request; only the game's consumed Crimson charge earns healing.
pub fn crimson_tier(item: u32) -> Option<u32> {
    let id = item.checked_sub(0x4000_0000 + 1001)?;
    (id <= 24 && id % 2 == 0).then_some(id / 2)
}

/// Two wedges for the native base flask's 250 HP. Native upgrades and flask
/// correction effects scale this, in quarter-wedges; libsm64 clamps capacity.
pub fn healing(points: i32, correction: f32) -> u8 {
    if points >= 0 || !correction.is_finite() || correction <= 0.0 {
        return 0;
    }
    ((-(points as f64) * correction as f64 * 4.0 / 125.0)
        .round()
        .clamp(0.0, 255.0)) as u8
}

#[derive(Clone, Copy)]
pub struct Snapshot {
    pub player: usize,
    pub allocation: u8,
    pub total: u32,
    pub tiers: u16,
}

#[derive(Default)]
pub struct Tracker {
    previous: Option<Snapshot>,
    remaining: f32,
    queued: Option<u32>,
    before_anim: i32,
    drink_anim: Option<i32>,
    elapsed: f32,
}

impl Tracker {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Returns the consumed item once. Allocation/tier/player transitions prime
    /// a new baseline, so rests, reallocation and load/replay cannot become drinks.
    pub fn observe(
        &mut self,
        sample: Option<Snapshot>,
        pressed: Option<i32>,
        queued: Option<u32>,
        anim: i32,
        dt: f32,
    ) -> Option<u32> {
        let Some(now) = sample else {
            self.reset();
            return None;
        };
        let old = self.previous.replace(now);
        let Some(old) = old else {
            return None;
        };
        if old.player != now.player
            || old.allocation != now.allocation
            || (old.tiers != now.tiers && now.total != 0)
            || now.total > old.total
        {
            self.remaining = 0.0;
            self.drink_anim = None;
            self.queued = None;
            return None;
        }
        if let Some(before) = pressed.filter(|_| old.total > 0) {
            self.remaining = 2.5;
            self.before_anim = before;
            // Native queued repeat drinks can replay the same animation ID.
            // Keep an existing pose; animation IDs are presentation, not proof
            // that a charge was or wasn't consumed.
            if self.drink_anim != Some(anim) {
                self.drink_anim = None;
            }
            self.elapsed = 0.0;
            self.queued = None;
        }
        if self.remaining > 0.0 {
            if let Some(item) = queued.filter(|&i| crimson_tier(i).is_some()) {
                self.queued = Some(item);
            }
            if self.queued.is_some() && anim != self.before_anim && self.drink_anim.is_none() {
                self.drink_anim = Some(anim);
                self.elapsed = 0.0;
            }
        }
        let consumed = (old.total.checked_sub(now.total) == Some(1) && self.remaining > 0.0)
            .then_some(self.queued)
            .flatten();
        if self.drink_anim.is_some_and(|a| a != anim) {
            self.drink_anim = None;
            // ER can transition between startup, drinking and recovery before
            // the inventory decrement reaches our frame observer. An animation
            // change ends the pose, not proof of the pending native consumption.
        }
        self.remaining = (self.remaining - dt.clamp(0.0, 0.25)).max(0.0);
        self.elapsed += dt.clamp(0.0, 0.25);
        if self.remaining == 0.0 {
            self.drink_anim = None;
            self.queued = None;
        }
        consumed
    }

    pub fn pose(&self) -> Option<f32> {
        self.drink_anim
            .map(|_| (self.elapsed / 1.8).clamp(0.0, 1.0))
    }

    pub fn pending(&self) -> bool {
        self.remaining > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const RED: u32 = 0x4000_03e9;
    #[test]
    fn animation_transition_before_inventory_update_keeps_consumption_request() {
        let mut t = Tracker::default();
        t.observe(sample(3), None, None, 0, 0.0);
        t.observe(sample(3), Some(0), Some(RED), 50, 0.1);
        t.observe(sample(3), None, None, 51, 0.1);
        assert_eq!(t.observe(sample(2), None, None, 51, 0.1), Some(RED));
        assert_eq!(t.observe(sample(2), None, None, 0, 0.1), None);
    }
    fn sample(total: u32) -> Option<Snapshot> {
        Some(Snapshot {
            player: 1,
            allocation: 4,
            total,
            tiers: 1,
        })
    }
    #[test]
    fn completed_drink_once_but_empty_cancelled_and_blue_do_not_heal() {
        let mut t = Tracker::default();
        assert_eq!(t.observe(sample(3), None, None, 0, 0.0), None);
        assert_eq!(t.observe(sample(3), Some(0), Some(RED), 0, 0.1), None);
        assert_eq!(t.observe(sample(2), None, Some(RED), 50, 0.1), Some(RED));
        assert_eq!(t.observe(sample(2), None, Some(RED), 50, 0.1), None);
        t.reset();
        t.observe(sample(1), None, None, 0, 0.0);
        assert_eq!(
            t.observe(sample(0), Some(0), Some(0x4000_041b), 50, 0.1),
            None
        );
        t.reset();
        t.observe(sample(1), None, None, 0, 0.0);
        t.observe(sample(1), Some(0), Some(RED), 0, 0.1);
        for _ in 0..30 {
            assert_eq!(t.observe(sample(1), None, Some(RED), 0, 0.1), None);
        }
        assert_eq!(t.observe(sample(0), None, Some(RED), 50, 0.1), None);
        assert_eq!(t.pose(), None);
    }
    #[test]
    fn reallocation_upgrade_load_and_refill_cannot_heal() {
        for changed in [
            Snapshot {
                allocation: 2,
                ..sample(2).unwrap()
            },
            Snapshot {
                tiers: 2,
                ..sample(2).unwrap()
            },
            Snapshot {
                player: 2,
                ..sample(2).unwrap()
            },
        ] {
            let mut t = Tracker::default();
            t.observe(sample(3), None, None, 0, 0.0);
            t.observe(sample(3), Some(0), Some(RED), 0, 0.1);
            assert_eq!(t.observe(Some(changed), None, Some(RED), 50, 0.1), None);
        }
        let mut t = Tracker::default();
        t.observe(sample(3), None, None, 0, 0.0);
        t.observe(sample(3), Some(0), Some(RED), 0, 0.1);
        assert_eq!(t.observe(None, None, None, -1, 0.1), None);
        assert_eq!(t.observe(sample(2), None, Some(RED), 50, 0.1), None);
        assert_eq!(t.observe(sample(4), None, Some(RED), 50, 0.1), None);
        assert_eq!(t.observe(sample(3), None, Some(RED), 50, 0.1), None);
    }
    #[test]
    fn native_potency_and_no_flask_correction() {
        assert_eq!(healing(-250, 1.0), 8);
        assert_eq!(healing(-810, 1.0), 26);
        assert_eq!(healing(-250, 0.0), 0);
        assert_eq!(healing(-250, 1.2), 10);
        for correction in [f32::NAN, f32::INFINITY, -1.0] {
            assert_eq!(healing(-250, correction), 0);
        }
        assert_eq!(healing(250, 1.0), 0);
        assert_eq!(crimson_tier(RED), Some(0));
        assert_eq!(crimson_tier(RED + 24), Some(12));
        for item in [RED - 1, RED + 1, RED + 26, 1001, 0x4000_041b] {
            assert_eq!(crimson_tier(item), None);
        }
    }

    #[test]
    fn native_animation_can_start_before_observer_and_last_charge_can_disappear() {
        let mut t = Tracker::default();
        t.observe(sample(1), None, None, 0, 0.0);
        // Input was captured before behavior, but frame observes the new animation.
        t.observe(sample(1), Some(0), Some(RED), 50, 0.1);
        assert!(t.pose().is_some());
        let empty = Snapshot {
            tiers: 0,
            ..sample(0).unwrap()
        };
        assert_eq!(t.observe(Some(empty), None, None, 50, 0.1), Some(RED));
        assert_eq!(t.observe(Some(empty), None, Some(RED), 50, 0.1), None);
        t.observe(Some(empty), None, Some(RED), 0, 0.1);
        assert_eq!(t.pose(), None);
    }

    #[test]
    fn quantity_changes_without_use_requests_do_not_heal_and_same_anim_repeat_does() {
        let mut t = Tracker::default();
        t.observe(sample(3), None, None, 0, 0.0);
        assert_eq!(t.observe(sample(2), None, Some(RED), 0, 0.1), None);
        assert_eq!(t.pose(), None);
        t.reset();
        t.observe(sample(3), None, None, 0, 0.0);
        t.observe(sample(3), Some(0), Some(RED), 50, 0.1);
        assert_eq!(t.observe(sample(2), None, Some(RED), 50, 0.1), Some(RED));
        t.observe(sample(2), Some(50), Some(RED), 50, 0.1);
        assert_eq!(t.observe(sample(1), None, Some(RED), 50, 0.1), Some(RED));
    }
}
