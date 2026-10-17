//! Matériau « verre dépoli » sombre (iOS) : la scène 3D floutée derrière un
//! panneau, assombrie par une teinte, avec un liseré d'un pixel. Aucun
//! dégradé : le flou vient de la scène elle-même.
//!
//! egui ne sait pas flouter ; l'application le fait sur le GPU. Le protocole,
//! à chaque image :
//! 1. avant l'interface, l'application publie ([`publish`]) les textures
//!    floutées dont elle dispose, une par emplacement ;
//! 2. l'interface peint ses panneaux ([`show`], [`paint`]) : le n-ième
//!    panneau de l'image occupe l'emplacement n et demande sa zone ;
//! 3. après l'interface, l'application relit les zones ([`take_requests`]),
//!    les floute dans l'image rendue et met les textures à jour avant que
//!    egui ne dessine.
//!
//! Chaque texture couvre la zone du panneau élargie de [`MARGIN`] (voir
//! [`region`]) : le flou y puise les couleurs voisines. Sans texture
//! (premier affichage, écran sans scène 3D), le panneau prend la couleur
//! unie des blocs sombres.

use egui::{
    Color32, Context, CornerRadius, Frame, Id, InnerResponse, Margin, Painter, Rect, Shape, Stroke,
    StrokeKind, TextureId, Ui, epaint::RectShape, pos2,
};

use crate::theme::{BAR, PANEL_BORDER};

/// Marge floutée autour d'un panneau, en points.
pub const MARGIN: f32 = 24.0;
/// Teinte sombre posée sur le flou (noir bleuté à 62 %, prémultiplié).
pub const TINT: Color32 = Color32::from_rgba_premultiplied(11, 11, 12, 158);

#[derive(Clone, Default)]
struct State {
    /// Textures publiées par l'application, par emplacement.
    textures: Vec<Option<TextureId>>,
    /// Zones demandées pendant la passe `pass` d'egui.
    requests: Vec<Rect>,
    pass: Option<u64>,
}

fn id() -> Id {
    Id::unique("bones-ui-glass")
}

/// Zone (en points) dont la texture de l'emplacement doit contenir le flou :
/// le panneau élargi de [`MARGIN`], limité à l'écran.
pub fn region(rect: Rect, screen: Rect) -> Rect {
    rect.expand(MARGIN).intersect(screen)
}

/// Application : textures floutées disponibles, avant l'interface.
pub fn publish(ctx: &Context, textures: Vec<Option<TextureId>>) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<State>(id()).textures = textures);
}

/// Application : zones demandées par la dernière passe de l'interface (une
/// nouvelle passe efface celles des passes précédentes).
pub fn take_requests(ctx: &Context) -> Vec<Rect> {
    ctx.data_mut(|d| std::mem::take(&mut d.get_temp_mut_or_default::<State>(id()).requests))
}

/// Formes du fond vitré de `rect` ; réserve l'emplacement suivant.
fn shapes(ctx: &Context, rect: Rect, radius: CornerRadius) -> Vec<Shape> {
    let pass = ctx.cumulative_pass_nr();
    let texture = ctx.data_mut(|d| {
        let state = d.get_temp_mut_or_default::<State>(id());
        if state.pass != Some(pass) {
            state.pass = Some(pass);
            state.requests.clear();
        }
        let slot = state.requests.len();
        state.requests.push(rect);
        state.textures.get(slot).copied().flatten()
    });
    let mut shapes = Vec::with_capacity(3);
    match texture {
        Some(texture) => {
            let r = region(rect, ctx.content_rect());
            let uv = |p: egui::Pos2| pos2((p.x - r.min.x) / r.width(), (p.y - r.min.y) / r.height());
            let uv = Rect::from_min_max(uv(rect.min), uv(rect.max));
            shapes.push(
                RectShape::filled(rect, radius, Color32::WHITE)
                    .with_texture(texture, uv)
                    .into(),
            );
            shapes.push(RectShape::filled(rect, radius, TINT).into());
        }
        None => shapes.push(RectShape::filled(rect, radius, BAR).into()),
    }
    shapes.push(RectShape::stroke(rect, radius, Stroke::new(1.0, PANEL_BORDER), StrokeKind::Inside).into());
    shapes
}

/// Peint un fond vitré dans `rect` (sous ce qui sera dessiné ensuite).
pub fn paint(painter: &Painter, rect: Rect, radius: impl Into<CornerRadius>) {
    painter.extend(shapes(painter.ctx(), rect, radius.into()));
}

/// Cadre vitré autour du contenu, comme un [`Frame`] : coins `radius`,
/// marges intérieures `margin`.
pub fn show<R>(
    ui: &mut Ui,
    radius: impl Into<CornerRadius>,
    margin: impl Into<Margin>,
    add: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let background = ui.painter().add(Shape::Noop);
    let inner = Frame::new().inner_margin(margin).show(ui, add);
    let shapes = shapes(ui.ctx(), inner.response.rect, radius.into());
    ui.painter().set(background, Shape::Vec(shapes));
    inner
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{RawInput, vec2};

    fn frame(ctx: &Context, paint_panel: bool) -> Vec<Rect> {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            if paint_panel {
                paint(
                    ui.painter(),
                    Rect::from_min_size(pos2(10.0, 10.0), vec2(100.0, 50.0)),
                    12,
                );
            }
        });
        output.textures_delta.clear();
        take_requests(ctx)
    }

    #[test]
    fn panels_are_requested_once_per_pass() {
        let ctx = Context::default();
        assert_eq!(frame(&ctx, true).len(), 1);
        assert_eq!(frame(&ctx, true).len(), 1, "pas d'accumulation entre images");
        assert!(frame(&ctx, false).is_empty());
    }

    #[test]
    fn region_is_expanded_and_clamped() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        let r = region(Rect::from_min_size(pos2(10.0, 100.0), vec2(100.0, 50.0)), screen);
        assert_eq!(r.min, pos2(0.0, 100.0 - MARGIN));
        assert_eq!(r.max, pos2(110.0 + MARGIN, 150.0 + MARGIN));
    }
}
