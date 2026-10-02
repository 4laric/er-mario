//! Native SM64 capped/uncapped heads and the original detachable cap.
//! Head-local metres: -X up, +Y front, Z across. No authored Cappy eyes.
use super::model::Tri;

pub const CAP: usize = 28;
pub const BARE_HEAD: usize = 29;
pub const CENTER: [f32; 3] = [-0.475, 0.0, 0.0];
// Retained atlas reservations keep the existing texture builder compatible;
// no geometry uses these colors now.
pub const SWATCH_Y: usize = 1536;
pub const SWATCH_X: [usize; 3] = [1728, 1856, 1984];
pub const COLORS: [[f32; 4]; 3] = [
    [1.0, 1.0, 1.0, 1.0],
    [0.015, 0.02, 0.025, 1.0],
    [0.22, 0.02, 0.015, 1.0],
];

/// The adapter tags native cap triangles as 28. Keep a duplicate on the
/// native capped head for rest, and retain the original on 28 for flight.
/// The complete native uncapped head uses 29. FLVER shares the existing eye
/// variants between both heads, instead of adding another overlapping face.
pub fn native_heads(capped: &[Tri], bare: &[Tri]) -> Option<Vec<Tri>> {
    if !capped.iter().any(|t| t.part == CAP as i32)
        || !capped.iter().any(|t| t.part == 3)
        || !bare.iter().any(|t| t.part == 3)
    {
        return None;
    }
    let mut out = capped.to_vec();
    out.extend(
        capped
            .iter()
            .filter(|t| t.part == CAP as i32)
            .map(|t| Tri { part: 3, ..*t }),
    );
    out.extend(bare.iter().filter(|t| t.part == 3).map(|t| Tri {
        part: BARE_HEAD as i32,
        ..*t
    }));
    Some(out)
}

/// (native capped head, projectile cap, native uncapped head).
pub fn shown_parts(flying: bool) -> (bool, bool, bool) {
    (!flying, flying, flying)
}
