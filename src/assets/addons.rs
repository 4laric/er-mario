//! Small original flask; metres in native hand axes, with -X pointing toward its mouth.
use super::flver::Vertex;

pub const FLASK: usize = 30;
pub const BOARD: usize = 31;
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
            for pos in (0..6).map(|i| point(x, i as f64 * std::f64::consts::TAU / 6.0)) {
                verts.push(Vertex { pos, normal: [sign, 0.0, 0.0], uv, part: FLASK });
            }
            for i in 1..5 {
                let a = base + i;
                let b = base + i + 1;
                tris.push(if sign < 0.0 { [base, b, a] } else { [base, a, b] });
            }
        }
    }
    append_board(verts, tris);
}

/// Original compact deck and four low-poly wheels, in -X up / +Y forward axes.
fn append_board(verts: &mut Vec<Vertex>, tris: &mut Vec<[u16; 3]>) {
    let base = u16::try_from(verts.len()).expect("board vertex budget");
    for x in [-0.035, 0.0] {
        for y in [-0.32, 0.32] {
            for z in [-0.13, 0.13] {
                let n: [f64; 3] = [if x < 0.0 { -1.0 } else { 1.0 }, y / 0.32, z / 0.13];
                verts.push(Vertex { pos: [x,y,z], normal: n.map(|v| v / 3.0f64.sqrt()), uv: [192.0/2048.0,1536.0/2048.0], part: BOARD });
            }
        }
    }
    for t in [[0,1,3],[0,3,2],[4,6,7],[4,7,5],[0,4,5],[0,5,1],[2,3,7],[2,7,6],[0,2,6],[0,6,4],[1,5,7],[1,7,3]] {
        tris.push(t.map(|i| base+i));
    }
    for y in [-0.20,0.20] {
        for z in [-0.14,0.14] {
            let base = u16::try_from(verts.len()).expect("board wheel budget");
            for n in [[-1.0,0.0,0.0],[0.0,1.0,0.0],[1.0,0.0,0.0],[0.0,-1.0,0.0],[0.0,0.0,-1.0],[0.0,0.0,1.0]] {
                verts.push(Vertex {pos:[0.05+n[0]*0.05,y+n[1]*0.05,z+n[2]*0.025], normal:n, uv:[576.0/2048.0,1536.0/2048.0], part:BOARD});
            }
            for i in 0..4 {
                tris.extend([[base+4,base+i,base+(i+1)%4],[base+5,base+(i+1)%4,base+i]]);
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
        assert_eq!(verts.iter().filter(|v| v.part == FLASK).count(), 108);
        assert_eq!(verts.iter().filter(|v| v.part == BOARD).count(), 32);
        assert_eq!(verts.len(), 140);
        assert_eq!(tris.len(), 104);
        assert!(verts.len() <= 145); // actual armour1184 minus native heads/Mario/FLUDD1039
        assert!(verts.iter().all(|v| matches!(v.part, FLASK | BOARD) && v.pos.iter().chain(v.normal.iter()).all(|x| x.is_finite())));
        for tri in tris {
            let p = tri.map(|i| verts[i as usize].pos);
            let a: [f64; 3] = std::array::from_fn(|i| p[1][i] - p[0][i]);
            let b: [f64; 3] = std::array::from_fn(|i| p[2][i] - p[0][i]);
            let cross = [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]];
            assert!(cross.iter().map(|v| v*v).sum::<f64>() > 1e-12);
            let normals: [f64;3] = std::array::from_fn(|i| tri.iter().map(|&v| verts[v as usize].normal[i]).sum());
            assert!(cross.iter().zip(normals).map(|(a,b)| a*b).sum::<f64>() > 0.0);
        }
    }
}
