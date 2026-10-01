//! Memory explorer (dev tool): walks the physics world from CSHavokMan and logs every pointer
//! field that points at an object with a C++ class name (MSVC RTTI). Used to find Havok's live
//! world / bodies / shapes so we can read the game's real collision. Triggered with F9.

use std::collections::HashSet;
use std::fmt::Write as _;

use eldenring::cs::CSHavokMan;
use fromsoftware_shared::{FromStatic, UnknownPtr};
use windows::Win32::System::Memory::{MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_NOACCESS, VirtualQuery};


/// Readable regions VirtualQuery reported recently: (start, end, when). Checks inside them don't
/// ask Windows again for REGION_TTL. VirtualQuery made Mario mode run at ~20 fps on native
/// Windows (the collision and clutter scans check memory hundreds of times a frame); most
/// checks land in a few big heap regions.
static REGIONS: std::sync::Mutex<Vec<(usize, usize, std::time::Instant)>> = std::sync::Mutex::new(Vec::new());
const REGION_TTL: std::time::Duration = std::time::Duration::from_millis(500);
const MAX_REGIONS: usize = 64;

/// Memory check statistics (perf log): checks, VirtualQuery calls, time in VirtualQuery (ns).
pub static CHECKS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static QUERIES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static QUERY_NS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// True if `len` bytes at `addr` are committed, readable memory.
pub fn readable(addr: usize, len: usize) -> bool {
    use std::sync::atomic::Ordering::Relaxed;
    if addr < 0x10000 || addr % 8 != 0 {
        return false;
    }
    CHECKS.fetch_add(1, Relaxed);
    let now = std::time::Instant::now();
    let mut regions = REGIONS.lock().unwrap_or_else(|e| e.into_inner());
    regions.retain(|r| now.duration_since(r.2) < REGION_TTL);
    if regions.iter().any(|&(start, end, _)| addr >= start && addr.saturating_add(len) <= end) {
        return true;
    }
    let mut info = MEMORY_BASIC_INFORMATION::default();
    let n = unsafe { VirtualQuery(Some(addr as *const _), &mut info, size_of::<MEMORY_BASIC_INFORMATION>()) };
    QUERIES.fetch_add(1, Relaxed);
    QUERY_NS.fetch_add(now.elapsed().as_nanos() as u64, Relaxed);
    if n == 0 || info.State != MEM_COMMIT {
        return false;
    }
    if info.Protect.0 & (PAGE_NOACCESS.0 | PAGE_GUARD.0) != 0 {
        return false;
    }
    let start = info.BaseAddress as usize;
    let end = start + info.RegionSize;
    if regions.len() >= MAX_REGIONS {
        regions.remove(0);
    }
    regions.push((start, end, now));
    addr + len <= end
}

pub fn read_u64(addr: usize) -> Option<u64> {
    readable(addr, 8).then(|| unsafe { *(addr as *const u64) })
}

/// Class name of the object at `addr` (its first qword must be a vtable with RTTI).
pub fn class_of(addr: usize) -> Option<String> {
    let vt = read_u64(addr)? as usize;
    if !readable(vt.wrapping_sub(8), 16) {
        return None;
    }
    unsafe { UnknownPtr::from(addr) }.rtti_classname()
}

fn dump(out: &mut String, seen: &mut HashSet<usize>, addr: usize, name: &str, depth: u32, size: usize) {
    if !seen.insert(addr) {
        return;
    }
    let pad = "  ".repeat(depth as usize);
    let _ = writeln!(out, "{pad}== {name} @ {addr:#x}");
    let mut children = Vec::new();
    for off in (0..size).step_by(8) {
        let Some(v) = read_u64(addr + off) else { break };
        let v = v as usize;
        if let Some(cls) = class_of(v) {
            let _ = writeln!(out, "{pad}  +{off:#05x}: {v:#x} -> {cls}");
            children.push((v, cls));
        } else if let Some(inner) = read_u64(v).filter(|_| readable(v, 8)) {
            // pointer to a pointer to a class (arrays of objects are often like this)
            if let Some(cls) = class_of(inner as usize) {
                let _ = writeln!(out, "{pad}  +{off:#05x}: {v:#x} -> [{inner:#x} -> {cls}, ...]");
            }
        }
    }
    if depth < 3 {
        for (v, cls) in children {
            let interesting = cls.contains("hknp") || cls.contains("hkp") || cls.contains("Phys") || cls.contains("Hit");
            if interesting {
                dump(out, seen, v, &cls, depth + 1, 0x400);
            }
        }
    }
}

/// Scans an object for hkArray-like fields (pointer, i32 size, i32 capacity) and reports which
/// element offsets hold pointers to Havok shape objects.
fn arrays(out: &mut String, addr: usize, size: usize) {
    let _ = writeln!(out, "\n== arrays in object @ {addr:#x}");
    for off in (0..size).step_by(8) {
        let (Some(ptr), Some(meta)) = (read_u64(addr + off), read_u64(addr + off + 8)) else { break };
        let (ptr, count) = (ptr as usize, (meta & 0xffff_ffff) as i64);
        if !(1..=200_000).contains(&count) || !readable(ptr, 8) {
            continue;
        }
        // look at the first 0x400 bytes of the array data for shape pointers
        let mut hits = Vec::new();
        for eoff in (0..0x400).step_by(8) {
            let Some(v) = read_u64(ptr + eoff) else { break };
            if let Some(cls) = class_of(v as usize) {
                if cls.contains("Shape") || cls.contains("Body") || cls.contains("Motion") {
                    hits.push(format!("{eoff:#x}:{cls}"));
                }
            }
        }
        if !hits.is_empty() {
            let _ = writeln!(out, "  +{off:#05x}: data {ptr:#x} count {count} | {}", hits.join(" "));
        }
    }
}

fn hexdump(out: &mut String, addr: usize, len: usize) {
    for row in (0..len).step_by(16) {
        let mut line = format!("    {row:#05x}:");
        let mut floats = String::new();
        for w in (0..16).step_by(4) {
            let Some(q) = read_u64(addr + row + (w & !7)) else { return };
            let v = if w % 8 == 0 { q as u32 } else { (q >> 32) as u32 };
            line += &format!(" {v:08x}");
            floats += &format!(" {:>10.3}", f32::from_bits(v));
        }
        let _ = writeln!(out, "{line}   |{floats}");
    }
}

/// Every hkArray-like field of an object, with element samples in several interpretations.
fn object_arrays(out: &mut String, addr: usize, size: usize, depth: u32) {
    let pad = "  ".repeat(depth as usize);
    for off in (0..size).step_by(8) {
        let (Some(ptr), Some(meta)) = (read_u64(addr + off), read_u64(addr + off + 8)) else { break };
        let (ptr, count) = (ptr as usize, (meta & 0xffff_ffff) as i64);
        let cap = ((meta >> 32) & 0x3fff_ffff) as i64;
        if (1..=5_000_000).contains(&count) && cap >= count && readable(ptr, 16) {
            let q = |i: usize| read_u64(ptr + i * 8).unwrap_or(0);
            let _ = writeln!(
                out,
                "{pad}  +{off:#05x}: ARRAY data {ptr:#x} count {count} cap {cap} | u64: {:x} {:x} {:x} | u32: {:x} {:x} {:x} {:x} | u16: {:x} {:x} {:x} {:x}",
                q(0), q(1), q(2),
                q(0) as u32, (q(0) >> 32) as u32, q(1) as u32, (q(1) >> 32) as u32,
                q(0) as u16, (q(0) >> 16) as u16, (q(0) >> 32) as u16, (q(0) >> 48) as u16,
            );
        } else if let Some(cls) = class_of(ptr) {
            let _ = writeln!(out, "{pad}  +{off:#05x}: {ptr:#x} -> {cls}");
            if depth < 2 && !cls.contains("Allocator") {
                let _ = writeln!(out, "{pad}  -- inside {cls}:");
                object_arrays(out, ptr, 0x200, depth + 1);
            }
        }
    }
}

/// Dumps the first compressed-mesh body: raw body bytes and the shape's arrays.
fn bodies(out: &mut String, world: usize) {
    let Some(data) = read_u64(world + 0x28) else { return };
    let count = read_u64(world + 0x30).unwrap_or(0) as u32 as usize;
    let data = data as usize;
    let mut shown = 0;
    let mut kinds = std::collections::BTreeMap::<String, u32>::new();
    for i in 0..count.min(4000) {
        let body = data + i * 0xb0;
        let Some(shape) = read_u64(body + 0x60) else { continue };
        let Some(cls) = class_of(shape as usize) else { continue };
        *kinds.entry(cls.clone()).or_default() += 1;
        if cls.contains("CompressedMesh") && shown < 2 {
            shown += 1;
            let _ = writeln!(out, "\n== body #{i} @ {body:#x} shape {shape:#x} ({cls})");
            hexdump(out, body, 0xb0);
            if let Some(md) = read_u64(shape as usize + 0x48) {
                let _ = writeln!(out, "  mesh data header:");
                hexdump(out, md as usize, 0x60);
                let secs = read_u64(md as usize + 0x60).unwrap_or(0) as usize;
                let _ = writeln!(out, "  sections[0..2]:");
                hexdump(out, secs, 0xc0);
            }
            let _ = writeln!(out, "  shape object:");
            hexdump(out, shape as usize, 0x100);
            object_arrays(out, shape as usize, 0x100, 0);
        }
    }
    let _ = writeln!(out, "\nshape kinds over {count} body slots: {kinds:?}");
}

fn f32_at(addr: usize) -> f32 {
    let q = read_u64(addr & !7).unwrap_or(0);
    f32::from_bits(if addr % 8 == 0 { q as u32 } else { (q >> 32) as u32 })
}

fn u32_at(addr: usize) -> u32 {
    let q = read_u64(addr & !7).unwrap_or(0);
    if addr % 8 == 0 { q as u32 } else { (q >> 32) as u32 }
}

/// Decodes a hknpCompressedMeshShapeData into triangles (shape-local space).
/// Layout guesses (Havok hkcdStaticMeshTree), verified against the counts in the log:
///   data+0x30 domain min (vec4), +0x40 domain max (vec4)
///   data+0x60 sections (0x60 each), +0x70 primitives (u8 x4), +0x80 shared index (u16),
///   +0x90 packed vertices (u32 11/11/10), +0xa0 shared vertices (u64 21/21/22)
pub fn decode_mesh(data: usize, log: &mut String) -> Vec<[[f32; 3]; 3]> {
    // arrays are validated once as a whole; absurd counts mean the layout guess is wrong
    let arr = |off: usize, elem: usize, max: usize| -> Option<(usize, usize)> {
        let p = read_u64(data + off)? as usize;
        let n = u32_at(data + off + 8) as usize;
        (n <= max && (n == 0 || readable(p, n * elem))).then_some((p, n))
    };
    let (Some((sections, nsec)), Some((prims, nprims)), Some((sidx, nsidx)), Some((packed, npacked)), Some((shared, nshared))) = (
        arr(0x60, 0x60, 10_000),
        arr(0x70, 4, 2_000_000),
        arr(0x80, 2, 2_000_000),
        arr(0x90, 4, 2_000_000),
        arr(0xa0, 8, 2_000_000),
    ) else {
        let _ = writeln!(log, "mesh data {data:#x}: implausible array header, skipped");
        return Vec::new();
    };
    let dmin = [f32_at(data + 0x30), f32_at(data + 0x34), f32_at(data + 0x38)];
    let dmax = [f32_at(data + 0x40), f32_at(data + 0x44), f32_at(data + 0x48)];
    let _ = writeln!(log, "mesh data {data:#x}: domain {dmin:?}..{dmax:?} sections {nsec} prims {nprims} sidx {nsidx} packed {npacked} shared {nshared}");
    let mut tris = Vec::new();
    let (mut sum_prims, mut sum_shared) = (0, 0);
    for si in 0..nsec {
        let sec = sections + si * 0x60;
        let _ = nprims;
        let off = [f32_at(sec + 0x30), f32_at(sec + 0x34), f32_at(sec + 0x38)];
        let scale = [f32_at(sec + 0x3c), f32_at(sec + 0x40), f32_at(sec + 0x44)];
        let first_packed = u32_at(sec + 0x48) as usize;
        let sv = u32_at(sec + 0x4c);
        let pr = u32_at(sec + 0x50);
        let (sv_start, sv_count) = ((sv >> 8) as usize, (sv & 0xff) as usize);
        let (pr_start, pr_count) = ((pr >> 8) as usize, (pr & 0xff) as usize);
        let num_packed = u32_at(sec + 0x58) as usize & 0xff;
        sum_prims += pr_count;
        sum_shared += sv_count;
        if si < 3 {
            let _ = writeln!(log, "  section {si}: off {off:?} scale {scale:?} firstPacked {first_packed} shared {sv_start}+{sv_count} prims {pr_start}+{pr_count} numPacked {num_packed}");
        }
        let vert = |i: usize| -> Option<[f32; 3]> {
            if i < num_packed {
                if first_packed + i >= npacked {
                    return None;
                }
                let v = unsafe { *((packed + (first_packed + i) * 4) as *const u32) };
                Some([
                    off[0] + scale[0] * (v & 0x7ff) as f32,
                    off[1] + scale[1] * ((v >> 11) & 0x7ff) as f32,
                    off[2] + scale[2] * (v >> 22) as f32,
                ])
            } else {
                let k = sv_start + i - num_packed;
                if k >= nsidx {
                    return None;
                }
                let si16 = unsafe { *((sidx + k * 2) as *const u16) } as usize;
                if si16 >= nshared {
                    return None;
                }
                let v = unsafe { *((shared + si16 * 8) as *const u64) };
                let f = |bits: u64, max: f64, a: f32, b: f32| a + (b - a) * (bits as f64 / max) as f32;
                Some([
                    f(v & 0x1f_ffff, 0x1f_ffff as f64, dmin[0], dmax[0]),
                    f((v >> 21) & 0x1f_ffff, 0x1f_ffff as f64, dmin[1], dmax[1]),
                    f(v >> 42, 0x3f_ffff as f64, dmin[2], dmax[2]),
                ])
            }
        };
        for p in pr_start..(pr_start + pr_count).min(nprims) {
            let idx = unsafe { *((prims + p * 4) as *const u32) }.to_le_bytes().map(|b| b as usize);
            let (Some(a), Some(b), Some(c)) = (vert(idx[0]), vert(idx[1]), vert(idx[2])) else { continue };
            tris.push([a, b, c]);
            if idx[3] != idx[2] {
                if let Some(d) = vert(idx[3]) {
                    tris.push([a, c, d]);
                }
            }
        }
    }
    let _ = writeln!(log, "  sum section prims {sum_prims} (array {nprims}), sum shared {sum_shared} (index array {nsidx}), tris {}", tris.len());
    tris
}

/// Decodes all compressed-mesh bodies near `center` into an OBJ file for inspection.
fn export_near(out: &mut String, world: usize, center: [f32; 3]) {
    let data = read_u64(world + 0x28).unwrap_or(0) as usize;
    let count = u32_at(world + 0x30) as usize;
    let mut obj = String::new();
    let mut nv = 0usize;
    let mut logged = 0;
    let mut bodies_used = 0;
    for i in 0..count.min(4000) {
        let body = data + i * 0xb0;
        let Some(shape) = read_u64(body + 0x60) else { continue };
        let shape = shape as usize;
        if !class_of(shape).is_some_and(|c| c.contains("CompressedMesh")) {
            continue;
        }
        let Some(mdata) = read_u64(shape + 0x48) else { continue };
        let t = [f32_at(body + 0x30), f32_at(body + 0x34), f32_at(body + 0x38)];
        let q = glam::Quat::from_xyzw(f32_at(body + 0x80), f32_at(body + 0x84), f32_at(body + 0x88), f32_at(body + 0x8c));
        let mut dummy = String::new();
        let tris = decode_mesh(mdata as usize, if logged < 3 { &mut *out } else { &mut dummy });
        logged += 1;
        let mut used = false;
        for tri in tris {
            let w = tri.map(|v| {
                let p = q.mul_vec3(glam::Vec3::from(v)) + glam::Vec3::from(t);
                [p.x, p.y, p.z]
            });
            let near = w.iter().any(|p| (p[0] - center[0]).abs() < 40.0 && (p[2] - center[2]).abs() < 40.0 && (p[1] - center[1]).abs() < 30.0);
            if near {
                used = true;
                for p in w {
                    obj += &format!("v {} {} {}\n", p[0], p[1], p[2]);
                }
                obj += &format!("f {} {} {}\n", nv + 1, nv + 2, nv + 3);
                nv += 3;
            }
        }
        if used {
            bodies_used += 1;
        }
    }
    let _ = writeln!(out, "\nexported {} triangles from {bodies_used} bodies near {center:?}", nv / 3);
    let _ = std::fs::write(crate::paths::file("near.obj"), obj);
}

pub fn run() {
    let mut out = String::new();
    match unsafe { CSHavokMan::instance() } {
        Ok(havok) => {
            let addr = havok as *const CSHavokMan as usize;
            let mut seen = HashSet::new();
            dump(&mut out, &mut seen, addr, "CSHavokMan", 0, 0x200);
            // hknpWorld lives at CSPhysWorld + 0x8
            let world = read_u64(addr + 0x98).and_then(|pw| read_u64(pw as usize + 0x8)).unwrap_or(0) as usize;
            if class_of(world).is_some_and(|c| c == "hknpWorld") {
                arrays(&mut out, world, 0x1000);
                bodies(&mut out, world);
                if let Some(p) = (unsafe { eldenring::cs::WorldChrMan::instance() }).ok().and_then(|w| w.main_player.as_ref()) {
                    let pos = p.chr_ins.modules.physics.position;
                    let _ = writeln!(out, "\nplayer at {:?}", (pos.0, pos.1, pos.2));
                    // raw sections of the first mesh for layout checking
                    export_near(&mut out, world, [pos.0, pos.1, pos.2]);
                }
            } else {
                let _ = writeln!(out, "hknpWorld not at CSPhysWorld+8");
            }
        }
        Err(e) => {
            let _ = writeln!(out, "no CSHavokMan: {e:?}");
        }
    }
    let _ = std::fs::write(crate::paths::file("explore.log"), out);
    crate::log("explorer: wrote explore.log");
}

/// Debug: an object's first `size` bytes as 8-byte words, with class names for pointers to
/// polymorphic objects, and one level into those (their first 0x80 bytes).
pub fn dump_object(addr: usize, size: usize, name: &str) -> String {
    let mut out = format!("== {name} @ {addr:#x} ({})\n", class_of(addr).unwrap_or_default());
    for off in (0..size).step_by(8) {
        let Some(v) = read_u64(addr + off) else { break };
        let v = v as usize;
        let cls = class_of(v);
        let f = f32::from_bits(v as u32);
        let _ = writeln!(out, "  +{off:#05x}: {v:#018x} {}{}", cls.clone().map(|c| format!("-> {c}")).unwrap_or_default(), if cls.is_none() && f.is_finite() && f.abs() > 1e-4 && f.abs() < 1e5 { format!(" (f32 {f:.3})") } else { String::new() });
        if let Some(c) = cls {
            for o2 in (0..0x80).step_by(8) {
                let Some(w) = read_u64(v + o2) else { break };
                let w = w as usize;
                let _ = writeln!(out, "      {c} +{o2:#04x}: {w:#018x} {}", class_of(w).unwrap_or_default());
            }
        }
    }
    out
}

/// Debug: a CSRagdollIns's per-body flags (the list at +0xF8; bit 2 = that body ragdolls and
/// collides with the map), with +0xF0 and the excluded body at +0x1F4.
pub fn ragdoll_flags(r: usize) -> String {
    let arr = read_u64(r + 0xF8).unwrap_or(0) as usize;
    let f0 = read_u64(r + 0xF0).unwrap_or(0) as u32;
    let excl = read_u64(r + 0x1F0).map(|v| (v >> 32) as i32).unwrap_or(-1);
    let mut flags = Vec::new();
    if readable(arr, 32 * 8) {
        for i in 0..32 {
            flags.push(format!("{:x}", read_u64(arr + i * 8).unwrap_or(0)));
        }
    }
    format!("ragdoll flags @ {arr:#x}: +0xf0 {f0:#x}, excluded body {excl}, per body [{}]", flags.join(" "))
}
