//! Small original flask; metres in native hand axes, with -X pointing toward its mouth.
use super::flver::Vertex;

pub const FLASK: usize = 30;
pub const FLASK_MOUTH: [f32; 3] = [-0.17, 0.0, 0.0];

/// Six-sided amber bottle, narrow neck and red stopper, using existing atlas swatches.
pub fn append(verts: &mut Vec<Vertex>, tris: &mut Vec<[u16; 3]>) {
    for (lo, hi, radius, pixel) in [
        (-0.10, 0.07, 0.065, 192.0),
        (-0.15, -0.10, 0.025, 192.0),
        (-0.17, -0.15, 0.032, 1984.0),
    ] {
        let uv = [pixel / 2048.0, 1536.0 / 2048.0];
        let point = |x, angle: f64| [x, radius * angle.cos(), radius * angle.sin()];
        for i in 0..6 {
            let a = i as f64 * std::f64::consts::TAU / 6.0;
            let b = (i + 1) as f64 * std::f64::consts::TAU / 6.0;
            let mid = (a + b) * 0.5;
            let base = u16::try_from(verts.len()).expect("flask vertex budget");
            for pos in [point(lo, a), point(lo, b), point(hi, b), point(hi, a)] {
                verts.push(Vertex { pos, normal: [0.0, mid.cos(), mid.sin()], uv, part: FLASK });
            }
            tris.extend([[base, base + 1, base + 2], [base, base + 2, base + 3]]);
        }
        for (x, sign) in [(lo, -1.0), (hi, 1.0)] {
            let base = u16::try_from(verts.len()).expect("flask vertex budget");
            for pos in std::iter::once([x, 0.0, 0.0]).chain((0..6).map(|i| point(x, i as f64 * std::f64::consts::TAU / 6.0))) {
                verts.push(Vertex { pos, normal: [sign, 0.0, 0.0], uv, part: FLASK });
            }
            for i in 0..6 {
                let a = base + 1 + i;
                let b = base + 1 + (i + 1) % 6;
                tris.push(if sign < 0.0 { [base, b, a] } else { [base, a, b] });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flask_fits_remaining_native_budget_and_has_valid_geometry() {
        let (mut verts, mut tris) = (Vec::new(), Vec::new());
        append(&mut verts, &mut tris);
        assert_eq!(verts.len(), 114);
        assert_eq!(tris.len(), 72);
        assert!(verts.len() <= 157);
        assert!(verts.iter().all(|v| v.part == FLASK && v.pos.iter().chain(v.normal.iter()).all(|x| x.is_finite())));
        for tri in tris {
            let p = tri.map(|i| verts[i as usize].pos);
            let a: [f64; 3] = std::array::from_fn(|i| p[1][i] - p[0][i]);
            let b: [f64; 3] = std::array::from_fn(|i| p[2][i] - p[0][i]);
            let cross = [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]];
            assert!(cross.iter().map(|v| v*v).sum::<f64>() > 1e-12);
        }
    }
}
