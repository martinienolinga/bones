//! Effets de dessin partagés par les widgets : dégradé d'accent (bleu →
//! indigo), lueurs radiales et reflet supérieur des surfaces. Les dégradés
//! sont de minuscules textures créées une fois, puis appliquées à des
//! rectangles arrondis anticrénelés d'egui.

use std::sync::Arc;

use egui::{
    Color32, ColorImage, Context, CornerRadius, Id, Painter, Pos2, Rect, Shape, TextureHandle,
    TextureOptions, epaint::RectShape, pos2, vec2,
};

use crate::theme::{ACCENT, ACCENT_2, mix};

struct Gradients {
    accent: TextureHandle,
    glow: TextureHandle,
    sheen: TextureHandle,
}

fn gradients(ctx: &Context) -> Arc<Gradients> {
    let id = Id::unique("bones-ui-gradients");
    if let Some(g) = ctx.data(|d| d.get_temp::<Arc<Gradients>>(id)) {
        return g;
    }
    let accent = ColorImage::new(
        [64, 1],
        (0..64).map(|x| mix(ACCENT, ACCENT_2, x as f32 / 63.0)).collect(),
    );
    let glow = ColorImage::new(
        [64, 64],
        (0..64 * 64)
            .map(|i| {
                let (x, y) = ((i % 64) as f32 - 31.5, (i / 64) as f32 - 31.5);
                let d = ((x * x + y * y).sqrt() / 32.0).min(1.0);
                let a = ((1.0 - d) * (1.0 - d) * 255.0) as u8;
                Color32::from_white_alpha(a)
            })
            .collect(),
    );
    let sheen = ColorImage::new(
        [1, 32],
        (0..32)
            .map(|y| {
                let t = y as f32 / 31.0;
                Color32::from_white_alpha(((1.0 - t / 0.6).max(0.0) * 30.0) as u8)
            })
            .collect(),
    );
    let g = Arc::new(Gradients {
        accent: ctx.load_texture("bones-ui-accent", accent, TextureOptions::LINEAR),
        glow: ctx.load_texture("bones-ui-glow", glow, TextureOptions::LINEAR),
        sheen: ctx.load_texture("bones-ui-sheen", sheen, TextureOptions::LINEAR),
    });
    ctx.data_mut(|d| d.insert_temp(id, g.clone()));
    g
}

/// Prépare les textures de dégradés.
pub fn install(ctx: &Context) {
    gradients(ctx);
}

/// Rectangle arrondi rempli du dégradé d'accent (bleu → indigo).
pub fn accent_fill(painter: &Painter, rect: Rect, radius: impl Into<CornerRadius>, opacity: f32) {
    let g = gradients(painter.ctx());
    let uv = Rect::from_min_max(pos2(0.0, 0.5), pos2(1.0, 0.5));
    let tint = Color32::from_white_alpha((opacity.clamp(0.0, 1.0) * 255.0) as u8);
    painter.add(Shape::Rect(
        RectShape::filled(rect, radius, tint).with_texture(g.accent.id(), uv),
    ));
}

/// Reflet doux sur le haut d'une surface (impression de matière).
pub fn sheen(painter: &Painter, rect: Rect, radius: impl Into<CornerRadius>) {
    let g = gradients(painter.ctx());
    let uv = Rect::from_min_max(pos2(0.5, 0.0), pos2(0.5, 1.0));
    painter.add(Shape::Rect(
        RectShape::filled(rect, radius, Color32::WHITE).with_texture(g.sheen.id(), uv),
    ));
}

/// Lueur radiale de couleur `color` (l'alpha règle l'intensité).
pub fn glow(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    let g = gradients(painter.ctx());
    let rect = Rect::from_center_size(center, vec2(radius, radius) * 2.0);
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    painter.image(g.glow.id(), rect, uv, color);
}
