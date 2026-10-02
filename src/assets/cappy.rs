//! Original Cappy eyes and interior lining, attached to the player's original cap.
//! Head-local metres: -X up, +Y front, Z across. No third-party add-on assets.
use super::flver::Vertex;

pub const CAP: usize = 28;
pub const EYES: usize = 29;
pub const CENTER: [f32; 3] = [-0.475, 0.0, 0.0];
pub const SWATCH_Y: usize = 1536;
pub const SWATCH_X: [usize; 3] = [1728, 1856, 1984];
pub const COLORS: [[f32; 4]; 3] = [
    [1.0, 1.0, 1.0, 1.0],
    [0.015, 0.02, 0.025, 1.0],
    [0.22, 0.02, 0.015, 1.0],
];

fn oval(
    verts: &mut Vec<Vertex>,
    tris: &mut Vec<[u16; 3]>,
    center: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    normal: [f64; 3],
    part: usize,
    color: usize,
) {
    const SIDES: usize = 8;
    let base = u16::try_from(verts.len()).expect("Cappy vertex budget");
    let uv = [SWATCH_X[color] as f64 / 2048.0, SWATCH_Y as f64 / 2048.0];
    verts.push(Vertex {
        pos: center,
        normal,
        uv,
        part,
    });
    for i in 0..SIDES {
        let theta = i as f64 * std::f64::consts::TAU / SIDES as f64;
        let pos = [0, 1, 2].map(|k| center[k] + u[k] * theta.cos() + v[k] * theta.sin());
        verts.push(Vertex {
            pos,
            normal,
            uv,
            part,
        });
    }
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let forward = (0..3).map(|k| cross[k] * normal[k]).sum::<f64>() > 0.0;
    for i in 0..SIDES {
        let a = base + 1 + i as u16;
        let b = base + 1 + ((i + 1) % SIDES) as u16;
        tris.push(if forward { [base, a, b] } else { [base, b, a] });
    }
}

pub fn append(verts: &mut Vec<Vertex>, tris: &mut Vec<[u16; 3]>) {
    // A small lining closes the crown's interior; it does not span the long brim.
    oval(
        verts,
        tris,
        [-0.305, -0.06, 0.0],
        [0.0, 0.14, 0.0],
        [0.0, 0.0, 0.245],
        [1.0, 0.0, 0.0],
        CAP,
        2,
    );
    for z in [-0.09, 0.09] {
        oval(
            verts,
            tris,
            [-0.575, 0.28, z],
            [0.058, 0.0, 0.0],
            [0.0, 0.0, 0.045],
            [0.0, 1.0, 0.0],
            EYES,
            0,
        );
        oval(
            verts,
            tris,
            [-0.580, 0.283, z],
            [0.036, 0.0, 0.0],
            [0.0, 0.0, 0.024],
            [0.0, 1.0, 0.0],
            EYES,
            1,
        );
    }
}
