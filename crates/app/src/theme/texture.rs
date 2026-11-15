//! Procedural surface textures (value-noise fBm, generated here: no image files). Each is a
//! tileable signed light map: dark ink where it shades, warm light where it lifts, transparent
//! where it leaves the surface's own colour alone, so one texture suits any base colour.

/// A texture kind a [`super::Surface`] can lay over its fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tex {
    /// Horizontal planks with grain, seams and end joints.
    Wood,
    /// Soft blotches and fibres.
    Parchment,
    /// Offset blocks with mortar and bevelled edges.
    Stone,
    /// Fine pebbled grain with creases.
    Leather,
}

impl Tex {
    pub const ALL: [Tex; 4] = [Tex::Wood, Tex::Parchment, Tex::Stone, Tex::Leather];

    /// Pixel size; it tiles at one point per pixel.
    pub fn size(self) -> (u32, u32) {
        match self {
            Tex::Wood => (1024, 512),
            _ => (512, 512),
        }
    }

    /// Width, height and straight-alpha RGBA8 bytes.
    pub fn rgba(self) -> (u32, u32, Vec<u8>) {
        let (w, h) = self.size();
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let v = match self {
                    Tex::Wood => wood(x, y, w),
                    Tex::Parchment => parchment(x, y),
                    Tex::Stone => stone(x, y),
                    Tex::Leather => leather(x, y),
                }
                .clamp(-1.0, 1.0);
                let (rgb, a) = if v < 0.0 { ([12, 6, 2], -v) } else { ([255, 236, 200], v) };
                out.extend_from_slice(&[rgb[0], rgb[1], rgb[2], (a * 255.0) as u8]);
            }
        }
        (w, h, out)
    }
}

fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0xff_ffff) as f32 / 0x100_0000 as f32
}

/// Value noise in 0..1 on a lattice that repeats every `px` × `py` cells.
fn noise(x: f32, y: f32, px: i32, py: i32, seed: u32) -> f32 {
    let (xf, yf) = (x.floor(), y.floor());
    let (ix, iy) = (xf as i32, yf as i32);
    let (fx, fy) = (x - xf, y - yf);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let at = |dx: i32, dy: i32| hash((ix + dx).rem_euclid(px), (iy + dy).rem_euclid(py), seed);
    let top = at(0, 0) + (at(1, 0) - at(0, 0)) * sx;
    let bottom = at(0, 1) + (at(1, 1) - at(0, 1)) * sx;
    top + (bottom - top) * sy
}

/// Fractal noise in roughly 0..1: `octaves` layers, each twice as fine and half as strong.
fn fbm(x: f32, y: f32, px: i32, py: i32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut f, mut total) = (0.0, 0.5, 1.0, 0.0);
    for o in 0..octaves {
        sum += amp * noise(x * f, y * f, px * f as i32, py * f as i32, seed + o);
        total += amp;
        amp *= 0.5;
        f *= 2.0;
    }
    sum / total
}

const PLANK: u32 = 64;

fn wood(x: u32, y: u32, w: u32) -> f32 {
    let plank = y / PLANK;
    let in_plank = y % PLANK;
    if in_plank < 2 {
        return -0.85;
    }
    if in_plank == 2 {
        return 0.12;
    }
    // A joint where one board ends and the next begins, somewhere along each plank.
    let joint = (plank * 389 + 211) % w;
    let dx = (x as i32 - joint as i32).rem_euclid(w as i32);
    if dx < 2 {
        return -0.75;
    }
    if dx == 2 {
        return 0.1;
    }
    let seed = plank * 31 + u32::from(x >= joint) * 7;
    let tone = (hash(plank as i32, (x >= joint) as i32, 99) - 0.5) * 0.3;
    let (fx, fy) = (x as f32, y as f32);
    // Grain: noise stretched along the board, warped into wavy streaks.
    let warp = fbm(fx / 128.0, fy / 32.0, (w / 128) as i32, 16, 3, seed);
    let streak = fbm(fx / 128.0, fy / 4.0 + warp * 5.0, (w / 128) as i32, 128, 4, seed + 11);
    let ring = (fy / 2.6 + warp * 22.0).sin() * 0.5 + 0.5;
    let rings = ring * ring * ring;
    tone + (streak - 0.5) * 0.9 - rings * rings * 0.28
}

fn parchment(x: u32, y: u32) -> f32 {
    let (fx, fy) = (x as f32, y as f32);
    let blotch = fbm(fx / 128.0, fy / 128.0, 4, 4, 5, 7);
    let fibre = noise(fx / 4.0, fy / 32.0, 128, 16, 8) - 0.5;
    let speck = if hash(x as i32, y as i32, 9) > 0.996 { -0.35 } else { 0.0 };
    (blotch - 0.5) * 1.3 + fibre * 0.18 + speck
}

fn stone(x: u32, y: u32) -> f32 {
    const BH: u32 = 64;
    const BW: u32 = 128;
    let row = y / BH;
    let sx = (x + (row % 2) * BW / 2) % 512;
    let (bx, by) = (sx % BW, y % BH);
    let block = (sx / BW) as i32 + row as i32 * 8;
    let edge = bx.min(BW - 1 - bx).min(by).min(BH - 1 - by);
    if edge < 3 {
        return -0.8;
    }
    let (fx, fy) = (x as f32, y as f32);
    let tone = (hash(block, 1, 5) - 0.5) * 0.25;
    let mut v = tone + (fbm(fx / 32.0, fy / 32.0, 16, 16, 4, 13) - 0.5) * 0.45;
    // Chiselled edges: lit along the top and left, shaded along the bottom and right.
    if edge < 6 {
        v += if by < 6 || bx < 6 { 0.2 } else { -0.28 };
    }
    // Soft pits and wear.
    let pit = fbm(fx / 16.0, fy / 16.0, 32, 32, 3, 17);
    v - ((0.36 - pit) * 3.0).clamp(0.0, 0.3)
}

fn leather(x: u32, y: u32) -> f32 {
    let (fx, fy) = (x as f32, y as f32);
    let broad = fbm(fx / 128.0, fy / 128.0, 4, 4, 3, 21) - 0.5;
    let grain = fbm(fx / 4.0, fy / 4.0, 128, 128, 2, 22) - 0.5;
    // Thin creases where a mid-scale noise crosses its middle.
    let crease = (1.0 - (fbm(fx / 64.0, fy / 64.0, 8, 8, 3, 23) - 0.5).abs() * 40.0).max(0.0);
    broad * 0.5 + grain * 0.22 - crease * 0.13
}

#[cfg(test)]
mod tests {
    use super::Tex;

    #[test]
    fn textures_are_deterministic_and_sized() {
        for t in Tex::ALL {
            let start = std::time::Instant::now();
            let (w, h, px) = t.rgba();
            eprintln!("{t:?} {w}x{h} in {:?}", start.elapsed());
            assert_eq!(px.len(), (w * h * 4) as usize);
            assert_eq!(px, t.rgba().2);
        }
    }
}
