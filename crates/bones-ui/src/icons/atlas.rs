//! Atlas d'icônes : chaque SVG est rastérisé une seule fois par `resvg`, à
//! la densité de pixels de l'écran, dans une unique texture egui.
//!
//! Deux tailles sont préparées (20 et 32 pt) pour rester nettes sans mipmaps :
//! une icône est toujours réduite d'au plus ~35 %, jamais agrandie. Mémoire :
//! environ 1 Mo sur un iPad Retina. L'atlas n'est refait que si la densité
//! de pixels change.

use std::sync::Arc;

use egui::{Color32, ColorImage, Context, Id, Painter, Rect, TextureHandle, TextureOptions, pos2, vec2};
use rayon::prelude::*;
use resvg::{tiny_skia, usvg};

use super::{ALL, Icon};

/// Tailles préparées, en points.
const BUCKETS: [f32; 2] = [20.0, 32.0];
/// Icônes par ligne de l'atlas.
const COLUMNS: usize = 15;
/// Marge transparente autour de chaque icône (évite les débordements du
/// filtrage linéaire).
const GUTTER: usize = 2;

struct Atlas {
    pixels_per_point: f32,
    texture: TextureHandle,
    /// Coordonnées de texture : `uv[taille][icône]`.
    uv: [Vec<Rect>; 2],
}

fn id() -> Id {
    Id::unique("bones-ui-icon-atlas")
}

/// Rastérise une icône en `px` × `px` pixels (RGBA prémultiplié).
pub fn rasterize(icon: Icon, px: u32) -> Vec<u8> {
    let px = px.max(1);
    let mut pixmap = tiny_skia::Pixmap::new(px, px).expect("taille d'icône valide");
    match usvg::Tree::from_str(&icon.svg(), &usvg::Options::default()) {
        Ok(tree) => {
            let scale = px as f32 / tree.size().width();
            resvg::render(
                &tree,
                tiny_skia::Transform::from_scale(scale, scale),
                &mut pixmap.as_mut(),
            );
        }
        Err(e) => debug_assert!(false, "SVG invalide pour {icon:?} : {e}"),
    }
    pixmap.take()
}

fn build(ctx: &Context, ppp: f32) -> Atlas {
    let sizes = BUCKETS.map(|pt| (pt * ppp).round().max(8.0) as usize);
    let rows = ALL.len().div_ceil(COLUMNS);
    let cell = |s: usize| s + 2 * GUTTER;
    let width = COLUMNS * cell(sizes[1]);
    let height = rows * (cell(sizes[0]) + cell(sizes[1]));
    let mut pixels = vec![0u8; width * height * 4];

    // Rastérisation parallèle (une tâche par icône et par taille).
    let jobs: Vec<(usize, usize)> = (0..2).flat_map(|b| (0..ALL.len()).map(move |i| (b, i))).collect();
    let images: Vec<Vec<u8>> = jobs
        .par_iter()
        .map(|&(b, i)| rasterize(ALL[i], sizes[b] as u32))
        .collect();

    let mut uv: [Vec<Rect>; 2] = [Vec::new(), Vec::new()];
    for (&(b, i), image) in jobs.iter().zip(&images) {
        let size = sizes[b];
        let top = if b == 0 { 0 } else { rows * cell(sizes[0]) };
        let (x0, y0) = (
            (i % COLUMNS) * cell(size) + GUTTER,
            top + (i / COLUMNS) * cell(size) + GUTTER,
        );
        for row in 0..size {
            let dst = ((y0 + row) * width + x0) * 4;
            pixels[dst..dst + size * 4].copy_from_slice(&image[row * size * 4..(row + 1) * size * 4]);
        }
        let (w, h) = (width as f32, height as f32);
        uv[b].push(Rect::from_min_size(
            pos2(x0 as f32 / w, y0 as f32 / h),
            vec2(size as f32 / w, size as f32 / h),
        ));
    }

    let image = ColorImage::from_rgba_premultiplied([width, height], &pixels);
    let texture = ctx.load_texture("bones-ui-icons", image, TextureOptions::LINEAR);
    Atlas {
        pixels_per_point: ppp,
        texture,
        uv,
    }
}

fn get(ctx: &Context) -> Arc<Atlas> {
    let ppp = ctx.pixels_per_point();
    if let Some(atlas) = ctx.data(|d| d.get_temp::<Arc<Atlas>>(id()))
        && atlas.pixels_per_point == ppp
    {
        return atlas;
    }
    let atlas = Arc::new(build(ctx, ppp));
    ctx.data_mut(|d| d.insert_temp(id(), atlas.clone()));
    atlas
}

pub fn prepare(ctx: &Context) {
    get(ctx);
}

pub fn paint(painter: &Painter, rect: Rect, icon: Icon, tint: Color32) {
    let atlas = get(painter.ctx());
    let size = rect.width().min(rect.height());
    let bucket = usize::from(size > BUCKETS[0] + 0.5);
    let target = Rect::from_center_size(rect.center(), vec2(size, size));
    painter.image(atlas.texture.id(), target, atlas.uv[bucket][icon as usize], tint);
}
