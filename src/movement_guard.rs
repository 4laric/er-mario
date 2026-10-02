//! Check live player-blocking geometry before accepting SM64's horizontal movement.
//! Imported meshes are incomplete; directly updating the character proxy otherwise
//! bypasses blockers (including fog gates) omitted by the layer/shape decoder.

pub fn constrain(
    from: [f32; 3],
    to: [f32; 3],
    game_driven: bool,
    mut cast: impl FnMut([f32; 3], [f32; 3]) -> Option<[f32; 3]>,
) -> [f32; 3] {
    if game_driven {
        return to;
    }
    let delta: [f32; 3] = std::array::from_fn(|i| to[i] - from[i]);
    let horizontal_squared = delta[0] * delta[0] + delta[2] * delta[2];
    if horizontal_squared < 0.0001 {
        return to;
    }
    let mut fraction = 1.0f32;
    // Chest/head samples avoid turning ordinary stair risers into walls. Follow
    // the actual segment, including its height change, rather than a stale scan.
    for height in [90.0, 150.0] {
        let start = [from[0], from[1] + height, from[2]];
        if let Some(hit) = cast(start, delta) {
            let along = ((hit[0] - start[0]) * delta[0] + (hit[2] - start[2]) * delta[2])
                / horizontal_squared;
            if along.is_finite() && (0.0..=1.0).contains(&along) {
                fraction = fraction.min((along - 2.0 / horizontal_squared.sqrt()).max(0.0));
            }
        }
    }
    // Preserve vertical simulation/gravity; this guard only forbids crossing a
    // player-blocking wall and does not snap Mario upward or reset his action.
    [
        from[0] + delta[0] * fraction,
        to[1],
        from[2] + delta[2] * fraction,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_fog_blocker_stops_a_walk_even_without_an_imported_mesh() {
        let result = constrain([0.0; 3], [20.0, 0.0, 0.0], false, |start, _| {
            Some([10.0, start[1], 0.0])
        });
        assert_eq!(result, [8.0, 0.0, 0.0]);
    }

    #[test]
    fn actual_entry_animation_leaves_position_to_the_game() {
        assert_eq!(
            constrain([0.0; 3], [20.0; 3], true, |_, _| panic!(
                "entry must bypass guard"
            )),
            [20.0; 3]
        );
    }

    #[test]
    fn removed_blocker_allows_passage_immediately() {
        assert_eq!(
            constrain([0.0; 3], [20.0, 5.0, 0.0], false, |_, _| None),
            [20.0, 5.0, 0.0]
        );
    }

    #[test]
    fn jump_or_dash_cannot_cross_and_gravity_is_preserved() {
        let result = constrain([0.0; 3], [100.0, -20.0, 0.0], false, |start, _| {
            (start[1] == 150.0).then_some([50.0, 140.0, 0.0])
        });
        assert_eq!(result, [48.0, -20.0, 0.0]);
    }

    #[test]
    fn vertical_jump_does_not_become_a_wall_collision() {
        assert_eq!(
            constrain([0.0; 3], [0.0, 30.0, 0.0], false, |_, _| panic!(
                "vertical movement"
            )),
            [0.0, 30.0, 0.0]
        );
    }

    #[test]
    fn reverse_and_diagonal_crossings_stop_at_the_nearest_hit() {
        let result = constrain([100.0, 0.0, 100.0], [0.0, 0.0, 0.0], false, |start, _| {
            let coordinate = if start[1] == 90.0 { 60.0 } else { 80.0 };
            Some([coordinate, start[1], coordinate])
        });
        assert!((result[0] - (80.0 + 2.0f32.sqrt())).abs() < 0.0001);
        assert_eq!(result[0], result[2]);
    }
}
