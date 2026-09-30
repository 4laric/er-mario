//! Moving collision (lifts, doors, pushed props) as libsm64 surface objects, so Mario rides and
//! bumps into them like SM64 platforms.
//!
//! After each static collision query the bodies it used are watched; the first time one of them
//! moves it is taken out of the static collision and becomes a surface object, updated every tick.

use std::collections::HashMap;

use eldenring::position::HavokPosition;
use glam::{Quat, Vec3};

use crate::havok_col::HavokCollision;
use crate::{collision, log, sm64, worker};

/// Most triangles per moving object (lift platforms are small; skip anything map-sized).
const MAX_TRIS: usize = 3000;
/// Stop tracking objects this far from Mario (SM64 units).
const FORGET: f32 = 4000.0;

struct Tracked {
    id: u32,
    /// rotation when the object was built (its triangles are baked in that pose)
    q0: Quat,
    /// its triangles (body space), to measure how far Mario is from the nearest one
    mesh: std::sync::Arc<crate::havok_col::Mesh>,
}

#[derive(Default)]
pub struct Moving {
    /// static bodies near Mario and their transform when last queried
    watch: HashMap<u32, (Vec3, Quat, usize)>,
    tracked: HashMap<u32, Tracked>,
}

fn sm(origin: [f32; 3], p: Vec3) -> Vec3 {
    Vec3::from(collision::er_to_sm(origin, &HavokPosition(p.x, p.y, p.z, 0.0)))
}

impl Moving {
    /// After a static query: remember where the bodies it used are now.
    pub fn watch_query(&mut self, h: &HavokCollision) {
        self.watch.clear();
        for &i in &h.last_bodies {
            if let Some((p, q)) = h.transform(i) {
                self.watch.insert(i, (p, q, h.shape_of(i)));
            }
        }
    }

    /// The world origin shifted (every body jumped): take a fresh snapshot instead of seeing motion.
    pub fn rewatch(&mut self, h: &HavokCollision) {
        self.watch_query(h);
    }

    /// Every tick. Returns true when the static collision must be rebuilt (a body became dynamic or
    /// went back to static).
    pub fn update(&mut self, h: &mut HavokCollision, origin: [f32; 3], mario: [f32; 3]) -> bool {
        let mut rebuild = false;
        // newly moving bodies -> surface objects
        let moved: Vec<u32> = self
            .watch
            .iter()
            .filter(|(i, (p0, q0, shape))| {
                !self.tracked.contains_key(i)
                    && h.shape_of(**i) == *shape
                    && h.transform(**i).is_some_and(|(p, q)| p.distance(*p0) > 0.01 || q.dot(*q0).abs() < 0.99999)
            })
            .map(|(i, _)| *i)
            .collect();
        for i in moved {
            self.watch.remove(&i);
            let (Some(mesh), Some((p, q))) = (h.mesh_of(i), h.transform(i)) else { continue };
            if mesh.tris().len() > MAX_TRIS {
                continue;
            }
            let center = sm(origin, p);
            let m = Vec3::from(mario);
            // convex shapes: every face points away from the middle (built once, so it must not depend
            // on where Mario happens to be)
            let convex_middle = h.is_convex(i).then(|| {
                let local = mesh.tris().iter().flatten().copied().sum::<Vec3>() / (mesh.tris().len() * 3) as f32;
                sm(origin, q * local + p)
            });
            let mut surfaces = Vec::with_capacity(mesh.tris().len());
            for t in mesh.tris() {
                let w = t.map(|v| sm(origin, q * v + p));
                let n = (w[1] - w[0]).cross(w[2] - w[1]);
                if n.length_squared() < 1.0 {
                    continue;
                }
                let n = n.normalize();
                // face each triangle the way Mario meets it (like the static collision)
                let want = if let Some(mid) = convex_middle {
                    (w[0] + w[1] + w[2]) / 3.0 - mid
                } else if n.y > 0.2 {
                    Vec3::Y
                } else if n.y < -0.2 {
                    -Vec3::Y
                } else {
                    let c = (w[0] + w[1] + w[2]) / 3.0;
                    Vec3::new(m.x - c.x, 0.0, m.z - c.z)
                };
                let local = w.map(|v| {
                    let l = (v - center).round();
                    [l.x as i32, l.y as i32, l.z as i32]
                });
                let mut s = sm64::SM64Surface::grass(local);
                if n.dot(want) < 0.0 {
                    s.vertices.swap(1, 2);
                }
                surfaces.push(s);
            }
            if surfaces.is_empty() {
                continue;
            }
            let n = surfaces.len();
            let transform = sm64::SM64ObjectTransform { position: center.into(), euler_rotation: [0.0; 3] };
            let id = worker::call("object create", move |_| {
                let object = sm64::SM64SurfaceObject { transform, surface_count: surfaces.len() as u32, surfaces: surfaces.as_ptr() };
                unsafe { sm64::sm64_surface_object_create(&object) }
            });
            if let Some(id) = id {
                log(format!("moving: body #{i} is moving, now a surface object ({n} triangles)"));
                self.tracked.insert(i, Tracked { id, q0: q, mesh: mesh.clone() });
                h.exclude.insert(i);
                rebuild = true;
            }
        }
        // move tracked objects along with their bodies
        let mut moves = Vec::new();
        let mut forget = Vec::new();
        for (i, t) in &self.tracked {
            let Some((p, q)) = h.transform(*i) else {
                log(format!("moving: body #{i} gone (no transform), dropping"));
                forget.push(*i);
                continue;
            };
            let center = sm(origin, p);
            // outside the body's whole bounding sphere (+40 m)? Lifts are e.g. 258 m tall cylinders whose
            // top is the platform, so neither the origin nor the vertices say where Mario can touch it
            let dist = (center.distance(Vec3::from(mario)) - t.mesh.radius() / crate::SCALE).max(0.0);
            if dist > FORGET {
                log(format!("moving: body #{i} {dist:.0} units away, dropping"));
                forget.push(*i);
                continue;
            }
            // rotation since creation, mirrored into SM64 space; SM64 objects turn about Y (yaw)
            let d = q * t.q0.inverse();
            let d = Quat::from_xyzw(d.x, -d.y, -d.z, d.w);
            let f = d * Vec3::Z;
            let yaw = f.x.atan2(f.z).to_degrees();
            moves.push((t.id, sm64::SM64ObjectTransform { position: center.into(), euler_rotation: [0.0, yaw, 0.0] }));
        }
        if !moves.is_empty() {
            worker::call("object move", move |_| {
                for (id, transform) in &moves {
                    unsafe { sm64::sm64_surface_object_move(*id, transform) };
                }
            });
        }
        for i in forget {
            if let Some(t) = self.tracked.remove(&i) {
                let id = t.id;
                worker::call("object delete", move |_| unsafe { sm64::sm64_surface_object_delete(id) });
                h.exclude.remove(&i);
                rebuild = true;
            }
        }
        rebuild
    }

    /// Deletes every surface object (Mario is going away).
    pub fn clear(&mut self, h: &mut HavokCollision) {
        let ids: Vec<u32> = self.tracked.drain().map(|(i, t)| {
            h.exclude.remove(&i);
            t.id
        }).collect();
        self.watch.clear();
        if !ids.is_empty() {
            worker::call("object delete", move |_| {
                for id in ids {
                    unsafe { sm64::sm64_surface_object_delete(id) };
                }
            });
        }
    }
}
