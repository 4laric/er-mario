//! Native cap/head regressions without a ROM or game assets.
#![allow(dead_code)]
mod model {
    #[derive(Clone, Copy)]
    pub struct Tri {
        pub part: i32,
        pub local: [[f32; 3]; 3],
        pub normal: [[f32; 3]; 3],
        pub uv: [[f32; 2]; 3],
        pub color: [[f32; 3]; 3],
        pub world: [[f32; 3]; 3],
    }
    pub struct MarioModel {
        pub tris: Vec<Tri>,
        pub peace: Vec<Tri>,
    }
}
fn log(_: impl AsRef<str>) {}
#[path = "../src/assets/bnd4.rs"]
mod bnd4;
#[path = "../src/assets/cappy.rs"]
mod cappy;
#[path = "../src/assets/fludd.rs"]
mod fludd;
#[path = "../src/assets/flver.rs"]
mod flver;

fn triangle(part: i32, texture: Option<usize>) -> model::Tri {
    let uv = texture
        .map(|cell| {
            [
                [(cell as f32 + 0.2) / 11.0, 0.2],
                [(cell as f32 + 0.8) / 11.0, 0.2],
                [(cell as f32 + 0.2) / 11.0, 0.8],
            ]
        })
        .unwrap_or([[1.0, 1.0]; 3]);
    model::Tri {
        part,
        local: [[10.0, 20.0, 30.0], [11.0, 20.0, 30.0], [10.0, 21.0, 30.0]],
        normal: [[0.0, 0.0, 1.0]; 3],
        uv,
        color: [[1.0, 0.0, 0.0]; 3],
        world: [[10.0, 20.0, 30.0], [11.0, 20.0, 30.0], [10.0, 21.0, 30.0]],
    }
}

#[test]
fn native_geometry_is_preserved_and_rest_and_flight_never_show_two_heads() {
    let cap = triangle(cappy::CAP as i32, Some(0));
    let face = triangle(3, None);
    let mut hair = face;
    hair.local[0][0] = 24.0;
    let capped = [triangle(1, None), face, cap];
    let native = cappy::native_heads(&capped, &[hair]).unwrap();
    assert_eq!(native.len(), 5);
    let original = &native.iter().find(|t| t.part == cappy::CAP as i32).unwrap();
    assert_eq!(original.local, cap.local);
    assert_eq!(original.normal, cap.normal);
    assert_eq!(original.uv, cap.uv);
    let rest_cap = &native.iter().filter(|t| t.part == 3).nth(1).unwrap();
    assert_eq!(rest_cap.local, cap.local);
    assert_eq!(rest_cap.uv, cap.uv);
    let bare = &native
        .iter()
        .find(|t| t.part == cappy::BARE_HEAD as i32)
        .unwrap();
    assert_eq!(bare.local, hair.local);
    assert_eq!(cappy::shown_parts(false), (true, false, false));
    assert_eq!(cappy::shown_parts(true), (false, true, true));
    assert_eq!(cappy::shown_parts(false), (true, false, false));
    assert_eq!(flver::PART_BONES.len(), 30);
}

#[test]
fn missing_native_head_or_hat_fails_instead_of_generating_an_incomplete_model() {
    let face = triangle(3, None);
    let cap = triangle(cappy::CAP as i32, None);
    assert!(cappy::native_heads(&[face], &[face]).is_none());
    assert!(cappy::native_heads(&[cap], &[face]).is_none());
    assert!(cappy::native_heads(&[face, cap], &[]).is_none());
}

#[test]
fn bare_head_eyes_share_all_four_native_blink_variants() {
    for cell in 5..=8 {
        let face = triangle(3, None);
        let eye = triangle(3, Some(cell));
        let cap = triangle(cappy::CAP as i32, Some(0));
        let model = model::MarioModel {
            tris: cappy::native_heads(&[face, eye, cap], &[face, eye]).unwrap(),
            peace: Vec::new(),
        };
        let (verts, tris) = flver::mario_vertices(&model);
        let count = |part| {
            tris.iter()
                .filter(|t| verts[t[0] as usize].part == part)
                .count()
        };
        assert_eq!(count(3), 2); // original capped face and its native hat
        assert_eq!(count(cappy::CAP), 1); // original cap, animated only in flight
        assert_eq!(count(cappy::BARE_HEAD), 1); // native bare face, no duplicate eye patch
        for part in 16..20 {
            assert_eq!(count(part), 1);
        }
        let eye_uv: Vec<_> = (16..20)
            .map(|part| verts.iter().find(|v| v.part == part).unwrap().uv)
            .collect();
        for pair in eye_uv.windows(2) {
            assert!((pair[1][0] - pair[0][0] - flver::TILE as f64 / flver::TEX).abs() < 1e-6);
            assert!((pair[1][1] - pair[0][1]).abs() < 1e-6);
        }
        assert!(
            verts
                .iter()
                .flat_map(|v| v.pos.iter().chain(v.normal.iter()).chain(v.uv.iter()))
                .all(|x| x.is_finite())
        );
        assert!(tris.iter().flatten().all(|&i| (i as usize) < verts.len()));
    }
}
