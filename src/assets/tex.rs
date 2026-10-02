//! Textures: Mario's albedo (SM64's textures and colours laid out for the FLVER's UVs), the chest
//! piece's TPF with Mario's textures, a minimal BC7 encoder, and the menu icon atlas patch.

use super::flver::{CELLS, INNER, PAD, TILE, mario_colors};
use super::model::MarioModel;

const SIZE: usize = 2048;
const ATLAS_W: usize = 64 * CELLS;

/// RGBA image, f32 channels 0..1
pub struct Image {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[f32; 4]>,
}

impl Image {
    pub fn new(w: usize, h: usize, fill: [f32; 4]) -> Self {
        Self { w, h, px: vec![fill; w * h] }
    }

    /// Area average by an integer factor.
    pub fn shrink(&self, f: usize) -> Image {
        let (w, h) = ((self.w / f).max(1), (self.h / f).max(1));
        let mut out = Image::new(w, h, [0.0; 4]);
        let (fx, fy) = (self.w / w, self.h / h);
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0.0f32; 4];
                for dy in 0..fy {
                    for dx in 0..fx {
                        let p = self.px[(y * fy + dy) * self.w + x * fx + dx];
                        for c in 0..4 {
                            acc[c] += p[c];
                        }
                    }
                }
                out.px[y * w + x] = acc.map(|v| v / (fx * fy) as f32);
            }
        }
        out
    }

    pub fn rgba8(&self) -> Vec<u8> {
        self.px.iter().flat_map(|p| p.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)).collect()
    }
}

/// Bilinear resample of an atlas cell (premultiplied, like Pillow does for RGBA).
fn cell_resized(atlas: &[u8], cell: usize, size: usize) -> Vec<[f32; 4]> {
    let at = |x: usize, y: usize| {
        let o = (y * ATLAS_W + cell * 64 + x) * 4;
        let a = atlas[o + 3] as f32 / 255.0;
        [atlas[o] as f32 / 255.0 * a, atlas[o + 1] as f32 / 255.0 * a, atlas[o + 2] as f32 / 255.0 * a, a]
    };
    let scale = 64.0 / size as f32;
    let axis = |i: usize| {
        let s = (i as f32 + 0.5) * scale - 0.5;
        let i0 = s.floor();
        let f = s - i0;
        let c = |v: f32| v.clamp(0.0, 63.0) as usize;
        (c(i0), c(i0 + 1.0), f)
    };
    let mut out = Vec::with_capacity(size * size);
    for y in 0..size {
        let (y0, y1, fy) = axis(y);
        for x in 0..size {
            let (x0, x1, fx) = axis(x);
            let (a, b, c, d) = (at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1));
            out.push(std::array::from_fn(|k| {
                (a[k] * (1.0 - fx) + b[k] * fx) * (1.0 - fy) + (c[k] * (1.0 - fx) + d[k] * fx) * fy
            }));
        }
    }
    out
}

/// 2048x2048: per SM64 colour a band of padded tiles (each atlas texture composited over the
/// colour), plus solid swatches for untextured triangles.
pub fn mario_albedo(model: &MarioModel) -> Image {
    let mut img = Image::new(SIZE, SIZE, [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0]);
    let cells: Vec<Vec<[f32; 4]>> = (0..CELLS).map(|i| cell_resized(&model.atlas, i, INNER)).collect();
    for (k, col) in mario_colors(model).iter().enumerate() {
        let c = col.map(|v| v as f32 / 255.0);
        for (i, cell) in cells.iter().enumerate() {
            for ty in 0..TILE {
                for tx in 0..TILE {
                    let inside = (PAD..PAD + INNER).contains(&tx) && (PAD..PAD + INNER).contains(&ty);
                    let rgb = if inside {
                        let p = cell[(ty - PAD) * INNER + tx - PAD];
                        [0, 1, 2].map(|ch| p[ch] + c[ch] * (1.0 - p[3]))
                    } else {
                        c
                    };
                    img.px[(k * 256 + ty) * SIZE + i * TILE + tx] = [rgb[0], rgb[1], rgb[2], 1.0];
                }
            }
        }
        for y in 1600..SIZE {
            for x in k * 340..k * 340 + 336 {
                img.px[y * SIZE + x] = [c[0], c[1], c[2], 1.0];
            }
        }
    }
    // Original backpack colours occupy a spare strip between Mario's texture bands and swatches.
    for (i, color) in super::fludd::COLORS.iter().enumerate() {
        let cx = super::fludd::SWATCH_X[i];
        let cy = super::fludd::SWATCH_Y;
        for y in cy - 16..cy + 16 {
            for x in cx - 16..cx + 16 {
                img.px[y * SIZE + x] = *color;
            }
        }
    }
    for (i, color) in super::cappy::COLORS.iter().enumerate() {
        let cx = super::cappy::SWATCH_X[i];
        let cy = super::cappy::SWATCH_Y;
        for y in cy - 16..cy + 16 {
            for x in cx - 16..cx + 16 {
                img.px[y * SIZE + x] = *color;
            }
        }
    }
    img
}

/// BC1 with a full mip chain (box-filtered), as stored in a DDS body.
fn bc1_mips(img: &Image, mips: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut level = Image { w: img.w, h: img.h, px: img.px.clone() };
    for m in 0..mips {
        if m > 0 {
            level = if level.w >= 2 && level.h >= 2 { level.shrink(2) } else { level };
        }
        let fmt = texpresso::Format::Bc1;
        let mut buf = vec![0u8; fmt.compressed_size(level.w, level.h)];
        fmt.compress(&level.rgba8(), level.w, level.h, texpresso::Params::default(), &mut buf);
        out.extend_from_slice(&buf);
    }
    out
}

fn u16s(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// Replaces every UTF-16 occurrence of `old` with `new` (same length).
pub fn rename(blob: &mut [u8], old: &str, new: &str) {
    let (o, n) = (u16s(old), u16s(new));
    debug_assert_eq!(o.len(), n.len());
    let mut i = 0;
    while i + o.len() <= blob.len() {
        if blob[i..i + o.len()] == o[..] {
            blob[i..i + o.len()].copy_from_slice(&n);
            i += o.len();
        } else {
            i += 1;
        }
    }
}

fn i32_at(d: &[u8], o: usize) -> usize {
    i32::from_le_bytes(d[o..o + 4].try_into().unwrap()) as usize
}

fn u32_at(d: &[u8], o: usize) -> usize {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap()) as usize
}

struct TpfTexture {
    name: String,
    off: usize,
    size: usize,
    mips: usize,
}

fn tpf_textures(t: &[u8]) -> Vec<TpfTexture> {
    (0..i32_at(t, 8))
        .map(|i| {
            let a = 0x10 + i * 0x14;
            TpfTexture {
                off: i32_at(t, a),
                size: i32_at(t, a + 4),
                mips: t[a + 10] as usize,
                name: super::bnd4::wstr(t, i32_at(t, a + 12)).unwrap_or_default(),
            }
        })
        .collect()
}

fn dds_header(dds: &[u8]) -> usize {
    if &dds[84..88] == b"DX10" { 148 } else { 128 }
}

/// Mario's albedo, a flat matte normal map and an empty metal mask, in the chest piece's own
/// sizes and formats (full textures "X_a", low-detail ones "X_a_l").
pub fn build_tpf(tpf: &[u8], albedo: &Image) -> Result<Vec<u8>, String> {
    let mut t = tpf.to_vec();
    for tex in tpf_textures(tpf) {
        let base = tex.name.strip_suffix("_l").unwrap_or(&tex.name);
        let dds = &tpf[tex.off..tex.off + tex.size];
        let head = dds_header(dds);
        let (h, w) = (u32_at(dds, 12), u32_at(dds, 16));
        let body = if base.ends_with("_a") && !base.ends_with("rope_a") {
            if w > SIZE || SIZE % w != 0 || w != h {
                return Err(format!("unexpected albedo size {w}x{h}"));
            }
            let img = if w == SIZE { albedo.shrink(1) } else { albedo.shrink(SIZE / w) };
            bc1_mips(&img, tex.mips)
        } else if base.ends_with("_n") {
            // BC7 mode 6, constant (128, 128, 50, 254): flat normal, low shine, no detail grime
            let mut bits: u128 = 1 << 6;
            for (k, c7) in [64u128, 64, 64, 64, 25, 25, 127, 127].into_iter().enumerate() {
                bits |= c7 << (7 + 7 * k);
            }
            bits.to_le_bytes().repeat((tex.size - head) / 16)
        } else if base.ends_with("_m") {
            vec![0; tex.size - head]
        } else {
            continue;
        };
        if body.len() != tex.size - head {
            return Err(format!("texture {} size mismatch ({} vs {})", tex.name, body.len(), tex.size - head));
        }
        t[tex.off + head..tex.off + tex.size].copy_from_slice(&body);
    }
    Ok(t)
}

// ---- BC7 ---------------------------------------------------------------------------------------

const BC7_WEIGHTS: [f64; 16] = [0., 4., 9., 13., 17., 21., 26., 30., 34., 38., 43., 47., 51., 55., 60., 64.];

/// Minimal BC7 encoder (mode 6 only: RGBA endpoints 7 bits + p-bit, 16 index levels).
pub fn bc7_mode6(rgba: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h);
    for by in 0..h / 4 {
        for bx in 0..w / 4 {
            let px: [[f64; 4]; 16] =
                std::array::from_fn(|i| std::array::from_fn(|c| rgba[((by * 4 + i / 4) * w + bx * 4 + i % 4) * 4 + c] as f64));
            let lo: [f64; 4] = std::array::from_fn(|c| px.iter().map(|p| p[c]).fold(f64::MAX, f64::min));
            let hi: [f64; 4] = std::array::from_fn(|c| px.iter().map(|p| p[c]).fold(f64::MIN, f64::max));
            let q = |v: f64| (v / 2.0).round_ties_even().clamp(0.0, 127.0) as u128;
            let (mut e0, mut e1) = (lo.map(q), hi.map(q));
            let (f0, f1) = (e0.map(|v| v as f64 * 2.0), e1.map(|v| v as f64 * 2.0));
            let d: [f64; 4] = std::array::from_fn(|c| f1[c] - f0[c]);
            let dd: f64 = d.iter().map(|v| v * v).sum();
            let mut idx: [u128; 16] = std::array::from_fn(|i| {
                let t = if dd > 0.0 { (0..4).map(|c| (px[i][c] - f0[c]) * d[c]).sum::<f64>() / dd.max(1.0) } else { 0.0 };
                let t = t.clamp(0.0, 1.0);
                let mut best = 0;
                for k in 1..16 {
                    if (t - BC7_WEIGHTS[k] / 64.0).abs() < (t - BC7_WEIGHTS[best] / 64.0).abs() {
                        best = k;
                    }
                }
                best as u128
            });
            // the first pixel's index has only 3 bits: swap the endpoints where it would need the 4th
            if idx[0] >= 8 {
                std::mem::swap(&mut e0, &mut e1);
                idx = idx.map(|i| 15 - i);
            }
            let mut bits: u128 = 1 << 6;
            for c in 0..4 {
                bits |= e0[c] << (7 + 14 * c) | e1[c] << (14 + 14 * c);
            }
            bits |= idx[0] << 65;
            for (k, &i) in idx.iter().enumerate().skip(1) {
                bits |= i << (64 + 4 + 4 * (k - 1));
            }
            out.extend_from_slice(&bits.to_le_bytes());
        }
    }
    out
}

// ---- menu icons --------------------------------------------------------------------------------

const ICON_ATLAS: &str = "SB_Icon_04";
const SPRITE: usize = 160;
/// icon -> (x, y) of its 160x160 sprite in SB_Icon_04 (the unused icons 13580-13583)
pub const ICON_RECTS: [(usize, usize); 4] = [(1312, 820), (1312, 656), (1312, 492), (1312, 328)];

/// Paints the icons (cap, overalls, gloves, shoes; 160x160 RGBA) into the menu atlas texture.
pub fn patch_icon_atlas(tpf: &[u8], icons: &[Vec<u8>; 4]) -> Result<Vec<u8>, String> {
    let tex = tpf_textures(tpf).into_iter().find(|t| t.name == ICON_ATLAS).ok_or("icon atlas not found")?;
    let dds = &tpf[tex.off..tex.off + tex.size];
    let (h, w) = (u32_at(dds, 12), u32_at(dds, 16));
    // a wrong header read once made a gigantic image: check everything before touching it
    if (w, h) != (4096, 2048) || &dds[84..88] != b"DX10" || u32_at(dds, 128) != 98 || tex.size - 148 < w * h {
        return Err("icon atlas is not the expected texture".into());
    }
    let mut t = tpf.to_vec();
    let data = tex.off + 148;
    let row_blocks = w / 4;
    for (icon, &(x, y)) in icons.iter().zip(ICON_RECTS.iter()) {
        let blocks = bc7_mode6(icon, SPRITE, SPRITE);
        let row = SPRITE / 4 * 16;
        for j in 0..SPRITE / 4 {
            let start = data + ((y / 4 + j) * row_blocks + x / 4) * 16;
            t[start..start + row].copy_from_slice(&blocks[j * row..(j + 1) * row]);
        }
    }
    Ok(t)
}
