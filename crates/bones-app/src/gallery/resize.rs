//! Réduction des vignettes à leur taille d'affichage (mémoire des
//! textures) : moyennes 2 × 2 tant que l'image fait plus du double de la
//! cible, puis interpolation bilinéaire.

/// Image RGBA réduite à la largeur `target` (jamais agrandie).
pub fn to_width(width: u32, height: u32, rgba: Vec<u8>, target: u32) -> (u32, u32, Vec<u8>) {
    let (mut w, mut h, mut px) = (width as usize, height as usize, rgba);
    let target = target.max(1) as usize;
    if px.len() != w * h * 4 || w <= target {
        return (width, height, px);
    }
    while w >= 2 * target && h >= 2 {
        (w, h, px) = halve(w, h, &px);
    }
    if w > target {
        let th = ((h * target) as f32 / w as f32).round().max(1.0) as usize;
        px = bilinear(w, h, &px, target, th);
        (w, h) = (target, th);
    }
    (w as u32, h as u32, px)
}

fn halve(w: usize, h: usize, px: &[u8]) -> (usize, usize, Vec<u8>) {
    let (nw, nh) = (w / 2, h / 2);
    let mut out = vec![0u8; nw * nh * 4];
    for y in 0..nh {
        for x in 0..nw {
            for c in 0..4 {
                let at = |dx: usize, dy: usize| u32::from(px[((2 * y + dy) * w + 2 * x + dx) * 4 + c]);
                let sum = at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1);
                out[(y * nw + x) * 4 + c] = ((sum + 2) / 4) as u8;
            }
        }
    }
    (nw, nh, out)
}

fn bilinear(w: usize, h: usize, px: &[u8], tw: usize, th: usize) -> Vec<u8> {
    let mut out = vec![0u8; tw * th * 4];
    let (sx, sy) = (w as f32 / tw as f32, h as f32 / th as f32);
    for y in 0..th {
        let fy = ((y as f32 + 0.5) * sy - 0.5).clamp(0.0, (h - 1) as f32);
        let (y0, ty) = (fy as usize, fy.fract());
        let y1 = (y0 + 1).min(h - 1);
        for x in 0..tw {
            let fx = ((x as f32 + 0.5) * sx - 0.5).clamp(0.0, (w - 1) as f32);
            let (x0, tx) = (fx as usize, fx.fract());
            let x1 = (x0 + 1).min(w - 1);
            for c in 0..4 {
                let at = |x: usize, y: usize| f32::from(px[(y * w + x) * 4 + c]);
                let top = at(x0, y0) + (at(x1, y0) - at(x0, y0)) * tx;
                let bottom = at(x0, y1) + (at(x1, y1) - at(x0, y1)) * tx;
                out[(y * tw + x) * 4 + c] = (top + (bottom - top) * ty).round() as u8;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_small_images_and_shrinks_large_ones() {
        let image = vec![100u8; 64 * 48 * 4];
        let (w, h, px) = to_width(64, 48, image.clone(), 100);
        assert_eq!((w, h, px.len()), (64, 48, image.len()));
        let (w, h, px) = to_width(64, 48, image.clone(), 16);
        assert_eq!((w, h), (16, 12));
        assert!(px.iter().all(|&v| v == 100), "couleur unie conservée");
        let (w, h, px) = to_width(640, 480, vec![7; 640 * 480 * 4], 378);
        assert_eq!((w, h, px.len()), (378, 284, 378 * 284 * 4));
    }

    #[test]
    fn halving_averages_blocks() {
        // 2 × 2 pixels noir/blanc → un pixel gris.
        let px = [0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255];
        let (w, h, out) = halve(2, 2, &px);
        assert_eq!((w, h), (1, 1));
        assert_eq!(out, [128, 128, 128, 255]);
    }
}
