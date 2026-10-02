//! Stable winding for Havok triangles reflected into SM64 space.
//! Never choose the solid side from Mario's position: crossing a plane or refreshing the
//! query must not turn the exterior of a rock into its interior.

pub fn surface_vertices(mut v: [[i32; 3]; 3], convex_center: Option<[f32; 3]>) -> Option<[[i32; 3]; 3]> {
    // er_to_sm reflects X, which reverses handedness.
    v.swap(1, 2);
    let a = v[0].map(|x| x as f64);
    let b = v[1].map(|x| x as f64);
    let c = v[2].map(|x| x as f64);
    let u: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
    let w: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
    let n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
    if n.iter().map(|x| x * x).sum::<f64>() < 1.0 {
        return None;
    }
    // Convex face indices and synthetic boxes need an explicit outward orientation.
    if let Some(mid) = convex_center {
        let dot: f64 = (0..3).map(|i| n[i] * ((a[i] + b[i] + c[i]) / 3.0 - mid[i] as f64)).sum();
        if dot < 0.0 {
            v.swap(1, 2);
        }
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normal(v: [[i32; 3]; 3]) -> [i32; 3] {
        let a: [i32; 3] = std::array::from_fn(|i| v[1][i] - v[0][i]);
        let b: [i32; 3] = std::array::from_fn(|i| v[2][i] - v[0][i]);
        [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
    }

    #[test]
    fn reflected_mesh_preserves_floor_ceiling_and_rock_exterior() {
        // A Havok upward floor after reflecting X must still be a floor.
        let floor = [[0, 0, 0], [0, 0, 100], [-100, 0, 0]];
        assert!(normal(surface_vertices(floor, None).unwrap())[1] > 0);
        let ceiling = [floor[0], floor[2], floor[1]];
        assert!(normal(surface_vertices(ceiling, None).unwrap())[1] < 0);
        // A leaning rock side keeps its outward normal across every query, including when
        // Mario's feet are below its plane or have penetrated it slightly.
        let rock = [[0, 0, 0], [0, 100, 100], [-20, 100, 0]];
        let result = surface_vertices(rock, None).unwrap();
        assert!(normal(result)[0] > 0);
        assert_eq!(surface_vertices(rock, None), Some(result));
    }

    #[test]
    fn convex_faces_are_outward_even_with_inconsistent_input_winding() {
        let top = [[-100, 100, -100], [100, 100, -100], [100, 100, 100]];
        for input in [top, [top[0], top[2], top[1]]] {
            assert!(normal(surface_vertices(input, Some([0.0; 3])).unwrap())[1] > 0);
        }
        let side = [[100, -100, -100], [100, 100, -100], [100, 100, 100]];
        assert!(normal(surface_vertices(side, Some([0.0; 3])).unwrap())[0] > 0);
        let bottom = top.map(|p| [p[0], -100, p[2]]);
        assert!(normal(surface_vertices(bottom, Some([0.0; 3])).unwrap())[1] < 0);
    }

    #[test]
    fn quantized_degenerate_triangles_are_rejected() {
        assert_eq!(surface_vertices([[0; 3]; 3], None), None);
        assert_eq!(surface_vertices([[0; 3], [1; 3], [2; 3]], None), None);
    }
}
