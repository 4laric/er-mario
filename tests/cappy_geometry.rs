//! Geometry regressions without a ROM or game assets.
mod flver {
    pub struct Vertex {
        pub pos: [f64; 3],
        pub normal: [f64; 3],
        pub uv: [f64; 2],
        pub part: usize,
    }
}
#[path = "../src/assets/cappy.rs"]
mod cappy;
#[path = "../src/assets/fludd.rs"]
mod fludd;

#[test]
fn outward_winding_and_finite_mesh_fit_the_shared_attachment_budget() {
    let (mut verts, mut tris) = (Vec::new(), Vec::new());
    cappy::append(&mut verts, &mut tris);
    assert_eq!((verts.len(), tris.len()), (45, 40));
    for tri in &tris {
        let a = &verts[tri[0] as usize];
        let b = &verts[tri[1] as usize];
        let c = &verts[tri[2] as usize];
        let u = [0, 1, 2].map(|k| b.pos[k] - a.pos[k]);
        let v = [0, 1, 2].map(|k| c.pos[k] - a.pos[k]);
        let cross = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        assert!((0..3).map(|k| cross[k] * a.normal[k]).sum::<f64>() > 0.0);
        for index in tri {
            let vertex = &verts[*index as usize];
            assert_eq!(vertex.part, a.part);
            assert!(
                vertex
                    .pos
                    .iter()
                    .chain(vertex.normal.iter())
                    .chain(vertex.uv.iter())
                    .all(|x| x.is_finite())
            );
            assert!(vertex.uv.iter().all(|x| (0.0..1.0).contains(x)));
            assert!(matches!(vertex.part, cappy::CAP | cappy::EYES));
        }
    }
    fludd::append(&mut verts, &mut tris);
    // Actual inspected FLVER leaves 603 vertices beyond the original Mario model.
    // Reserve 192 more for replacing the head and separating its original hat.
    assert!(verts.len() + 192 < 603);
}

#[test]
fn new_palette_stays_clear_of_backpack_and_original_mario_cells() {
    assert_eq!(cappy::SWATCH_Y, fludd::SWATCH_Y);
    assert!(cappy::SWATCH_Y - 16 > 5 * 256 + 186);
    assert!(cappy::SWATCH_Y + 16 < 1600);
    for &x in &cappy::SWATCH_X {
        assert!(x + 16 < 2048);
        assert!(fludd::SWATCH_X.iter().all(|&other| x.abs_diff(other) > 32));
    }
    for pair in cappy::SWATCH_X.windows(2) {
        assert!(pair[0].abs_diff(pair[1]) > 32);
    }
    for color in cappy::COLORS {
        assert!(
            color
                .iter()
                .all(|x| x.is_finite() && (0.0..=1.0).contains(x))
        );
    }
}
