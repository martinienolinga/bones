//! Planches-contacts des préréglages (outil de vérification visuelle, tests
//! seulement) : images d'un cycle en vue de profil et de face, os en bâtons
//! (gauche en bleu, droite en rouge), sol gradué tous les 25 cm.
//!
//! `BONES_PRESET_SHEETS=dossier cargo test -p bones-core --lib sheets -- --ignored`

use glam::{Vec2, Vec3};
use image::{Rgb, RgbImage};

use crate::presets::roles::{Lr, RoleMap};
use crate::{Clip, Pose, Skeleton, head_tail};

const CELL: u32 = 240;

fn line(img: &mut RgbImage, a: Vec2, b: Vec2, color: Rgb<u8>, width: i32) {
    let steps = (b - a).abs().max_element().ceil().max(1.0) as i32;
    for i in 0..=steps {
        let p = a.lerp(b, i as f32 / steps as f32);
        for dx in -width / 2..=width / 2 {
            for dy in -width / 2..=width / 2 {
                let (x, y) = (p.x as i32 + dx, p.y as i32 + dy);
                if x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
                    img.put_pixel(x as u32, y as u32, color);
                }
            }
        }
    }
}

/// Dessine une planche : `columns` images régulièrement espacées sur le
/// clip, une ligne de profil et une ligne de face (caméra qui suit la racine
/// horizontalement).
pub(crate) fn sheet(skeleton: &Skeleton, roles: &RoleMap, clip: &Clip, columns: u32) -> RgbImage {
    let mut img = RgbImage::from_pixel(CELL * columns, CELL * 3, Rgb([250, 250, 247]));
    let basis = roles.frame.basis();
    let to_canon = |p: Vec3| basis.inverse() * p;
    let rest = skeleton.rest_world();
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for (w, b) in rest.iter().zip(skeleton.bones()) {
        let (h, t) = head_tail(w, b.length);
        lo = lo.min(to_canon(h).min(to_canon(t)));
        hi = hi.max(to_canon(h).max(to_canon(t)));
    }
    // Cadrage sur la hauteur et la longueur du corps (pas l'envergure).
    let size = (hi.y - lo.y).max(hi.z - lo.z).max(1e-3) * 1.35;
    let scale = CELL as f32 / size;
    let ground = roles.frame.ground;
    let root = roles.root().unwrap_or(crate::BoneId(0));
    let center = to_canon(rest[root.index()].translation);
    let mut pose = Pose::rest(skeleton);
    let last = clip.frame_count.saturating_sub(1).max(1) as f32;
    for c in 0..columns {
        let frame = last * c as f32 / (columns - 1).max(1) as f32;
        clip.sample_into(frame, &mut pose);
        let world = pose.world(skeleton);
        let follow = to_canon(world[root.index()].translation) - center;
        // Profil, face, dessus.
        for (row, view) in [(0u32, 2usize), (1, 0), (2, 1)] {
            let ox = (c * CELL) as f32 + CELL as f32 * 0.5;
            let oy = (row * CELL) as f32 + CELL as f32 * if view == 1 { 0.5 } else { 0.85 };
            let project = |p: Vec3| {
                let base = if view == 1 { (lo + hi) * 0.5 } else { center };
                let q = to_canon(p) - Vec3::new(follow.x, 0.0, follow.z) - Vec3::new(base.x, ground, base.z);
                let h = if view == 2 { q.z } else { -q.x };
                let v = if view == 1 { q.z } else { q.y };
                Vec2::new(ox + h * scale, oy - v * scale)
            };
            // Sol gradué (les graduations défilent avec le déplacement).
            let gy = if view == 1 { ((row + 1) * CELL) as f32 - 2.0 } else { oy };
            line(&mut img, Vec2::new((c * CELL) as f32, gy), Vec2::new(((c + 1) * CELL) as f32, gy), Rgb([180, 180, 170]), 1);
            let along = if view == 2 { follow.z } else { -follow.x };
            let first = ((along - size) / 0.25).floor() as i32;
            for k in first..first + (2.0 * size / 0.25) as i32 + 2 {
                let x = ox + (k as f32 * 0.25 - along) * scale;
                if x > (c * CELL) as f32 && x < ((c + 1) * CELL) as f32 {
                    line(&mut img, Vec2::new(x, gy), Vec2::new(x, gy + 6.0), Rgb([150, 150, 140]), 1);
                }
            }
            for (i, (w, b)) in world.iter().zip(skeleton.bones()).enumerate() {
                let (h, t) = head_tail(w, b.length);
                let side = roles.roles[i].and_then(|r| r.chain.side());
                let color = match side {
                    Some(Lr::Left) => Rgb([40, 90, 220]),
                    Some(Lr::Right) => Rgb([220, 60, 50]),
                    None if roles.roles[i].is_some() => Rgb([30, 30, 30]),
                    None => Rgb([150, 150, 150]),
                };
                line(&mut img, project(h), project(t), color, 2);
            }
        }
        let x = ((c + 1) * CELL) as f32 - 0.5;
        line(&mut img, Vec2::new(x, 0.0), Vec2::new(x, (3 * CELL) as f32), Rgb([220, 220, 215]), 1);
    }
    img
}
