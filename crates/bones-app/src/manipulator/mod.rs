//! Dessin du manipulateur 3D (flèches, anneaux, carrés de plan, cubes
//! d'échelle) en aplats, sans dégradé, sur un calque placé au-dessus de la
//! vue 3D et sous les panneaux de l'interface.
//!
//! La géométrie (pixels physiques) vient de `bones-core` ; ce module ne
//! choisit que les couleurs, les épaisseurs et la mise en évidence de la
//! partie survolée ou manipulée.

use std::cell::RefCell;

use bones_core::manipulator::{Axis, Element, Geometry, Kind, Part, Tint};
use bones_core::{Editor, Viewport};
use egui::{Color32, Pos2, Shape, Stroke};

const X: Color32 = Color32::from_rgb(0xF2, 0x38, 0x5A);
const Y: Color32 = Color32::from_rgb(0x8C, 0xD1, 0x3A);
const Z: Color32 = Color32::from_rgb(0x2D, 0x9C, 0xFF);
const VIEW: Color32 = Color32::from_rgb(0x8E, 0x8E, 0x93);
const BALL: Color32 = Color32::from_rgb(0xD1, 0xD1, 0xD6);
const HANDLE: Color32 = Color32::from_rgb(0x64, 0xD2, 0xFF);
const NEUTRAL: Color32 = Color32::from_rgb(0xF2, 0xF2, 0xF7);
/// Partie en cours de manipulation.
const ACTIVE: Color32 = Color32::from_rgb(0xFF, 0xD6, 0x0A);

/// Épaisseur des traits, en points.
const STROKE: f32 = 2.5;
/// Opacité des moitiés arrière des anneaux.
const BACK_ALPHA: f32 = 0.28;
/// Opacité des autres parties pendant une manipulation.
const IDLE_ALPHA: f32 = 0.35;
/// Rayon de survol, en points.
const HOVER_RADIUS: f32 = 14.0;

thread_local! {
    /// Tampons réutilisés d'une image à l'autre.
    static GEOMETRY: RefCell<Geometry> = RefCell::new(Geometry::default());
}

/// Dessine le manipulateur de l'éditeur (s'il y en a un) et transmet à
/// l'éditeur l'échelle de l'interface et l'état de la touche Maj
/// (magnétisme inversé).
pub fn paint(ctx: &egui::Context, editor: &mut Editor, viewport: &Viewport) {
    let ppp = ctx.pixels_per_point();
    editor.pixels_per_point = ppp;
    editor.manipulator.invert_snap = ctx.input(|i| i.modifiers.shift);
    GEOMETRY.with_borrow_mut(|g| {
        if !editor.manipulator_geometry(viewport, g) {
            return;
        }
        let active = editor.manipulator_active();
        let hovered = if active.is_none() {
            hover(ctx, g, ppp)
        } else {
            None
        };
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::unique("manipulator"),
        ));
        for e in g.elements() {
            draw(&painter, g, e, ppp, active, hovered);
        }
        if let Some((text, at)) = editor.manipulator_readout() {
            readout(&painter, &text, Pos2::new(at.x / ppp, at.y / ppp));
        }
    });
}

/// Partie sous le pointeur, s'il n'est pas sur l'interface.
fn hover(ctx: &egui::Context, g: &Geometry, ppp: f32) -> Option<Part> {
    let pos = ctx.input(|i| i.pointer.hover_pos())?;
    if ctx
        .layer_id_at(pos)
        .is_some_and(|l| l.order != egui::Order::Background)
    {
        return None;
    }
    g.pick(glam::Vec2::new(pos.x, pos.y) * ppp, HOVER_RADIUS * ppp)
}

fn base_color(tint: Tint) -> Color32 {
    match tint {
        Tint::Axis(Axis::X) => X,
        Tint::Axis(Axis::Y) => Y,
        Tint::Axis(Axis::Z) => Z,
        Tint::View => VIEW,
        Tint::Handle => HANDLE,
        Tint::Ball => BALL,
        Tint::Trackball => Color32::from_white_alpha(14),
        Tint::Neutral => NEUTRAL,
    }
}

fn draw(
    painter: &egui::Painter,
    g: &Geometry,
    e: &Element,
    ppp: f32,
    active: Option<Part>,
    hovered: Option<Part>,
) {
    let is_active = active.is_some() && e.part == active;
    let is_hovered = hovered.is_some() && e.part == hovered;
    let mut color = base_color(e.tint);
    let mut width = STROKE * e.width;
    if is_active {
        color = match e.tint {
            Tint::Trackball => Color32::from_white_alpha(26),
            _ => ACTIVE,
        };
    } else if is_hovered {
        color = match e.tint {
            Tint::Trackball => Color32::from_white_alpha(24),
            _ => color.lerp_to_gamma(Color32::WHITE, 0.4),
        };
        width *= 1.35;
    } else if active.is_some() {
        color = color.gamma_multiply(IDLE_ALPHA);
    }
    if e.back {
        color = color.gamma_multiply(BACK_ALPHA);
    }
    let points: Vec<Pos2> = g
        .points(e)
        .iter()
        .map(|p| Pos2::new(p.x / ppp, p.y / ppp))
        .collect();
    let stroke = Stroke::new(width, color);
    painter.add(match e.kind {
        Kind::Line => Shape::line(points, stroke),
        Kind::Loop => Shape::closed_line(points, stroke),
        Kind::Fill => {
            // Boule centrale : liseré discret pour rester lisible sur fond clair.
            let outline = match e.tint {
                Tint::Ball => Stroke::new(1.0, Color32::from_black_alpha(70)),
                _ => Stroke::NONE,
            };
            Shape::convex_polygon(points, color, outline)
        }
    });
}

/// Valeur courante (distance, angle ou facteur) près du doigt.
fn readout(painter: &egui::Painter, text: &str, at: Pos2) {
    let galley = painter.layout_no_wrap(text.to_owned(), bones_ui::theme::semibold(13.0), NEUTRAL);
    let pos = at + egui::vec2(18.0, -34.0);
    let rect = egui::Rect::from_min_size(pos, galley.size()).expand2(egui::vec2(8.0, 4.0));
    painter.rect_filled(rect, rect.height() * 0.5, Color32::from_black_alpha(190));
    painter.galley(pos, galley, NEUTRAL);
}
