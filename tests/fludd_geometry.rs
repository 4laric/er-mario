//! Standalone authored-geometry checks; no ROM, game archive or Windows SDK required.
//! rustc --edition=2024 --test tests/fludd_geometry.rs -o fludd_geometry_tests.exe
mod flver {
    pub struct Vertex {
        pub pos: [f64; 3],
        pub normal: [f64; 3],
        pub uv: [f64; 2],
        pub part: usize,
    }
}
#[path = "../src/assets/fludd.rs"]
mod fludd;

#[test]
fn swatches_do_not_overlap_mario_textures_or_each_other() {
    assert!(fludd::SWATCH_Y - 16 > 5 * 256 + 186);
    assert!(fludd::SWATCH_Y + 16 < 1600);
    for color in fludd::COLORS {
        assert!(
            color
                .into_iter()
                .all(|c| c.is_finite() && (0.0..=1.0).contains(&c))
        );
    }
    for xs in fludd::SWATCH_X.windows(2) {
        assert!(xs[0] + 16 < xs[1] - 16);
    }
}
