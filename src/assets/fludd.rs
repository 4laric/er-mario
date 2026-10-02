//! Original low-poly water backpack, generated locally with Mario's armour.
//! Coordinates are metres in the mirrored torso's space: -X up, -Y back, Z across.
//! No third-party model, texture or sound is embedded here.

use super::flver::Vertex;

pub const BODY: usize = 21;
pub const HOVER: usize = 22;
pub const ROCKET: usize = 23;
pub const TURBO: usize = 24;
pub const JETS: usize = 25;
pub const ROCKET_JET: usize = 26;
pub const TURBO_JET: usize = 27;
// Reuse the horizontal stream, reversed and enlarged about its mouth, for Squirt.
pub const SQUIRT_STREAM_SCALE: f32 = 4.0;
pub const STREAM_MOUTH: [f32; 3] = [-0.055, -0.37, 0.0];
pub const STREAM_END: [f32; 3] = [-0.055, -0.68, 0.0];
pub const COLORS: [[f32; 4]; 4] = [
    [0.95, 0.68, 0.08, 1.0],
    [0.30, 0.34, 0.40, 1.0],
    [0.12, 0.65, 0.90, 1.0],
    [0.60, 0.92, 1.00, 1.0],
];
// The six Mario colour bands end before y=1500; solid swatches start at y=1600.
pub const SWATCH_Y: usize = 1536;
pub const SWATCH_X: [usize; 4] = [192, 576, 960, 1344];

/// Hidden attachment bones must never scale a different rendered part through its ancestry.
/// Check the player's actual skeleton before producing an armour package.
pub fn safe_attachment_hierarchy(parents: &[i16], parts: &[usize]) -> bool {
    if parts.len() <= BODY || parts.iter().any(|&b| b >= parents.len()) {
        return false;
    }
    let attachments = &parts[BODY..];
    for &bone in &parts[1..] {
        let mut parent = parents[bone];
        let mut steps = 0;
        while parent >= 0 {
            let p = parent as usize;
            if p >= parents.len() || attachments.contains(&p) || steps >= parents.len() {
                return false;
            }
            parent = parents[p];
            steps += 1;
        }
    }
    true
}

fn face(
    verts: &mut Vec<Vertex>,
    tris: &mut Vec<[u16; 3]>,
    points: [[f64; 3]; 4],
    normal: [f64; 3],
    part: usize,
    color: usize,
) {
    let base = u16::try_from(verts.len()).expect("backpack vertex budget");
    let uv = [SWATCH_X[color] as f64 / 2048.0, SWATCH_Y as f64 / 2048.0];
    for pos in points {
        verts.push(Vertex {
            pos,
            normal,
            uv,
            part,
        });
    }
    tris.extend([[base, base + 1, base + 2], [base, base + 2, base + 3]]);
}

fn box_mesh(
    verts: &mut Vec<Vertex>,
    tris: &mut Vec<[u16; 3]>,
    lo: [f64; 3],
    hi: [f64; 3],
    part: usize,
    color: usize,
) {
    let [x, y, z] = lo;
    let [a, b, c] = hi;
    for (points, normal) in [
        (
            [[x, y, z], [x, y, c], [x, b, c], [x, b, z]],
            [-1.0, 0.0, 0.0],
        ),
        (
            [[a, y, z], [a, b, z], [a, b, c], [a, y, c]],
            [1.0, 0.0, 0.0],
        ),
        (
            [[x, y, z], [a, y, z], [a, y, c], [x, y, c]],
            [0.0, -1.0, 0.0],
        ),
        (
            [[x, b, z], [x, b, c], [a, b, c], [a, b, z]],
            [0.0, 1.0, 0.0],
        ),
        (
            [[x, y, z], [x, b, z], [a, b, z], [a, y, z]],
            [0.0, 0.0, -1.0],
        ),
        (
            [[x, y, c], [a, y, c], [a, b, c], [x, b, c]],
            [0.0, 0.0, 1.0],
        ),
    ] {
        face(verts, tris, points, normal, part, color);
    }
}

/// Append a compact tank, pump, three nozzle variants and two water streams.
/// Every variant has its own existing skinned bone; the pose hides inactive variants.
pub fn append(verts: &mut Vec<Vertex>, tris: &mut Vec<[u16; 3]>) {
    // Pump and blue tank, with a yellow protective frame and silver cap.
    box_mesh(
        verts,
        tris,
        [-0.17, -0.25, -0.12],
        [0.04, -0.15, 0.12],
        BODY,
        0,
    );
    box_mesh(
        verts,
        tris,
        [-0.15, -0.29, -0.09],
        [0.01, -0.245, 0.09],
        BODY,
        2,
    );
    box_mesh(
        verts,
        tris,
        [-0.19, -0.26, -0.04],
        [-0.165, -0.17, 0.04],
        BODY,
        1,
    );
    // The pair of downward Hover outlets, mounted either side of the pump.
    for (lo, hi) in [
        ([-0.02, -0.26, -0.19], [0.07, -0.15, -0.13]),
        ([-0.02, -0.26, 0.13], [0.07, -0.15, 0.19]),
    ] {
        box_mesh(verts, tris, lo, hi, HOVER, 1);
    }
    // Rocket's larger central downward outlet.
    box_mesh(
        verts,
        tris,
        [0.01, -0.29, -0.075],
        [0.10, -0.17, 0.075],
        ROCKET,
        1,
    );
    // Turbo's rear-facing outlet, distinct from the vertical nozzles.
    box_mesh(
        verts,
        tris,
        [-0.10, -0.37, -0.07],
        [-0.01, -0.28, 0.07],
        TURBO,
        1,
    );
    // Each nozzle's water stream has its own pose so switching never leaves a stray jet.
    for z in [-0.16, 0.16] {
        box_mesh(
            verts,
            tris,
            [0.07, -0.225, z - 0.013],
            [0.32, -0.195, z + 0.013],
            JETS,
            3,
        );
    }
    box_mesh(
        verts,
        tris,
        [0.10, -0.255, -0.035],
        [0.52, -0.205, 0.035],
        ROCKET_JET,
        3,
    );
    box_mesh(
        verts,
        tris,
        [-0.085, -0.68, -0.025],
        [-0.025, -0.37, 0.025],
        TURBO_JET,
        3,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_attachment_ancestors_are_rejected_before_asset_build() {
        // A rendered torso on bone 2 and an attachment on spare leaf bone 3.
        let mut parts = vec![2; BODY];
        parts[0] = 0;
        parts.push(3);
        assert!(safe_attachment_hierarchy(&[-1, 0, 1, 0], &parts));
        parts[BODY] = 1; // hiding this ancestor would also scale the torso
        assert!(!safe_attachment_hierarchy(&[-1, 0, 1, 0], &parts));
        parts[BODY] = 3;
        assert!(!safe_attachment_hierarchy(&[-1, 0, 1, 3], &parts)); // cyclic attachment
        assert!(!safe_attachment_hierarchy(&[-1, 9, 1, 0], &parts)); // invalid parent
    }
    #[test]
    fn geometry_fits_existing_armour_and_has_outward_faces() {
        let (mut verts, mut tris) = (Vec::new(), Vec::new());
        append(&mut verts, &mut tris);
        assert!(verts.len() <= 603); // measured free capacity in both vanilla chest LODs
        assert!(tris.len() * 3 <= 5592 - 2577);
        assert_eq!(verts.len(), 264);
        for t in tris {
            let [a, b, c] = t.map(|i| &verts[usize::from(i)]);
            let ab = std::array::from_fn::<_, 3, _>(|k| b.pos[k] - a.pos[k]);
            let ac = std::array::from_fn::<_, 3, _>(|k| c.pos[k] - a.pos[k]);
            let cross = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            assert!(cross.iter().zip(a.normal).map(|(x, n)| x * n).sum::<f64>() > 0.0);
            assert!(a.pos.iter().all(|x| x.is_finite()));
            assert!((BODY..=TURBO_JET).contains(&a.part));
            assert!((a.uv[1] - SWATCH_Y as f64 / 2048.0).abs() < 1e-8);
        }
    }
}
