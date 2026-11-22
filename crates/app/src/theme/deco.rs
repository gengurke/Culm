//! What a theme draws over its header band ([`BandDeco`]).

use super::{BAND_H, BandDeco, pal, t};
use rune::{El, el};

/// The current theme's [`BandDeco`] as a layer over the band (none for [`BandDeco::None`]): a
/// fixed scatter, the same every frame (a tiny LCG).
pub fn band_deco() -> Option<El> {
    let (deco, p) = (t().skin.band_deco, pal());
    let mut layer = el()
        .key("band/stars")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .clip_rounded(0.0)
        .a11y_hidden();
    let mut seed: u32 = 0x9e37_79b9;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    match deco {
        BandDeco::None => return None,
        BandDeco::Stars => {
            for _ in 0..28 {
                let (x, y, size, a) = (
                    next() * 1600.0,
                    next() * BAND_H,
                    if next() > 0.85 { 2.0 } else { 1.0 },
                    0.18 + next() * 0.55,
                );
                layer = layer.child(
                    el().absolute(x, y)
                        .size(size, size)
                        .radius(size / 2.0)
                        .fill(p.text.alpha(a)),
                );
            }
        }
        BandDeco::Embers => {
            // Sparks thin out and fade as they rise from the bottom edge.
            for _ in 0..26 {
                let (x, rise, size) = (
                    next() * 1600.0,
                    next() * next(),
                    if next() > 0.8 { 3.0 } else { 2.0 },
                );
                let a = 0.85 - rise * 0.75;
                let spark = p.warning.mix(p.accent_hi, next());
                layer = layer.child(
                    el().absolute(x, BAND_H - 6.0 - rise * BAND_H * 0.9)
                        .size(size, size)
                        .radius(size / 2.0)
                        .fill(spark.alpha(a))
                        .shadow(0.0, 6.0, spark.alpha(a * 0.8)),
                );
            }
        }
    }
    Some(layer)
}
