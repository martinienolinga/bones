//! Carte d'un projet : vignette aux coins arrondis (ni ombre ni dégradé),
//! nom et résumé centrés dessous (voir `caption`). Toucher la vignette ouvre
//! le projet (ou la coche en mode sélection) ; appui long ou clic droit :
//! menu de la carte.

use std::time::Duration;

use bones_ui::{theme, widgets};
use egui::{Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, vec2};

use super::layout::fit;
use super::library::Card;
use super::{Gallery, GalleryAction, Menu};

/// Durée d'appui qui ouvre le menu de la carte.
pub const LONG_PRESS: f64 = 0.5;
/// Arrondi des vignettes.
const RADIUS: u8 = 7;

pub fn show(
    ui: &mut egui::Ui,
    gallery: &mut Gallery,
    card: &Card,
    thumb: Rect,
    caption: Rect,
    ppp: f32,
    actions: &mut Vec<GalleryAction>,
) {
    let id = egui::Id::unique(("card", &card.path));
    let response = ui.interact(thumb, id, Sense::click());
    interact(ui, gallery, card, thumb, &response, actions);
    gallery.shown.insert(card.path.clone(), thumb);
    if !ui.is_rect_visible(caption) {
        return;
    }

    let ctx = ui.ctx().clone();
    let dragging = ui.input(|i| i.pointer.is_decidedly_dragging());
    let press = ctx.animate_bool_with_time(
        id.with("press"),
        response.is_pointer_button_down_on() && !dragging,
        0.12,
    );
    let selected = gallery.is_selected(&card.path);
    let sel = ctx.animate_bool_with_time(id.with("sel"), selected, 0.14);
    let selecting = ctx.animate_bool_with_time(id.with("mode"), gallery.is_selecting(), 0.18);

    // Vignette, légèrement réduite sous le doigt.
    let width = (thumb.width() * ppp).round() as u32;
    let texture = gallery.thumbs.get(&card.path, card.modified, width).cloned();
    let size = texture.as_ref().map_or(thumb.size(), |t| t.size_vec2());
    let image = scale(fit(thumb, size), 1.0 - 0.035 * press);
    let painter = ui.painter();
    let radius = CornerRadius::same(RADIUS);
    if gallery.zooming.as_ref() == Some(&card.path) {
        // Vignette en transition : le zoom la dessine.
        super::caption::show(ui, gallery, card, thumb, caption, actions);
        return;
    }
    match &texture {
        Some(texture) => {
            let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            let shape =
                egui::epaint::RectShape::filled(image, radius, Color32::WHITE).with_texture(texture.id(), uv);
            painter.add(shape);
        }
        None => {
            painter.rect_filled(image, radius, theme::PANEL);
        }
    }
    if press > 0.0 {
        painter.rect_filled(image, radius, Color32::from_black_alpha((70.0 * press) as u8));
    }
    if sel > 0.0 {
        let stroke = Stroke::new(3.0, theme::ACCENT.gamma_multiply(sel));
        painter.rect_stroke(
            image.expand(2.0),
            CornerRadius::same(RADIUS + 3),
            stroke,
            StrokeKind::Outside,
        );
    }
    if selecting > 0.0 {
        let r = 12.0 * (0.7 + 0.3 * selecting);
        let center = image.right_bottom() - vec2(18.0, 18.0);
        let mut faded = painter.clone();
        faded.multiply_opacity(selecting);
        widgets::check_badge(&faded, center, r, sel);
    }
    if gallery.opening.as_ref() == Some(&card.path) {
        painter.rect_filled(image, radius, Color32::from_black_alpha(110));
        let spinner = Rect::from_center_size(image.center(), vec2(30.0, 30.0));
        egui::Spinner::new()
            .size(30.0)
            .color(Color32::WHITE)
            .paint_at(ui, spinner);
    }

    super::caption::show(ui, gallery, card, thumb, caption, actions);
}

fn scale(rect: Rect, factor: f32) -> Rect {
    Rect::from_center_size(rect.center(), rect.size() * factor)
}

/// Toucher, appui long, clic droit.
fn interact(
    ui: &egui::Ui,
    gallery: &mut Gallery,
    card: &Card,
    thumb: Rect,
    response: &egui::Response,
    actions: &mut Vec<GalleryAction>,
) {
    let path = &card.path;
    let held = response.is_pointer_button_down_on()
        && ui.input(|i| {
            !i.pointer.is_decidedly_dragging()
                && i.pointer
                    .press_start_time()
                    .is_some_and(|t| i.time - t >= LONG_PRESS)
        });
    if response.is_pointer_button_down_on() {
        ui.ctx().request_repaint_after(Duration::from_millis(40));
    }
    let long_press = held && gallery.long_pressed.as_ref() != Some(path);
    if (long_press || response.secondary_clicked()) && !gallery.is_selecting() && gallery.is_idle() {
        gallery.long_pressed = long_press.then(|| path.clone());
        gallery.rename = None;
        gallery.menu = Some(Menu::Card(path.clone(), thumb));
    }
    if response.clicked() {
        if gallery.long_pressed.take().as_ref() == Some(path) {
            return; // fin de l'appui long : le menu est déjà ouvert
        }
        if gallery.is_selecting() {
            gallery.toggle_selected(path);
        } else if gallery.is_idle() {
            gallery.rename = None;
            gallery.opening = Some(path.clone());
            actions.push(GalleryAction::Open(path.clone()));
        }
    }
}
