//! Pure policy for the native Regression reveal interaction.
pub const LAW_OF_REGRESSION: u32 = 0x4000_1a4a;
pub const REVEALED: u32 = 11009556;
pub const WAITING: u32 = 11009468;
pub const STATUE: u32 = 11000716;
pub const REGRESSION_EFFECT: i32 = 1673014;

#[derive(Clone, Copy)]
pub struct Context {
    pub interact: bool,
    pub managed_live: bool,
    pub alive: bool,
    pub menu: bool,
    pub revealed: bool,
    pub waiting: bool,
    pub owns_law: bool,
    pub distance_squared: Option<f32>,
}
pub fn permitted(c: Context) -> bool {
    c.interact
        && c.managed_live
        && c.alive
        && !c.menu
        && !c.revealed
        && c.waiting
        && c.owns_law
        && c.distance_squared
            .is_some_and(|d| d.is_finite() && (0.0..=16.0).contains(&d))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> Context {
        Context {
            interact: true,
            managed_live: true,
            alive: true,
            menu: false,
            revealed: false,
            waiting: true,
            owns_law: true,
            distance_squared: Some(16.0),
        }
    }
    #[test]
    fn each_guard_is_required_and_range_is_three_dimensional() {
        assert!(permitted(valid()));
        for denied in 0..7 {
            let mut c = valid();
            match denied {
                0 => c.interact = false,
                1 => c.managed_live = false,
                2 => c.alive = false,
                3 => c.menu = true,
                4 => c.revealed = true,
                5 => c.waiting = false,
                _ => c.owns_law = false,
            }
            assert!(!permitted(c));
        }
        for distance in [
            None,
            Some(16.01),
            Some(f32::NAN),
            Some(f32::INFINITY),
            Some(-1.0),
        ] {
            assert!(!permitted(Context {
                distance_squared: distance,
                ..valid()
            }));
        }
        // A target directly above Mario is out of range even with zero X/Z separation.
        assert!(!permitted(Context {
            distance_squared: Some(4.01_f32.powi(2)),
            ..valid()
        }));
    }
}
