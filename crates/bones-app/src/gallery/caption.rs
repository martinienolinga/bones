//! Légende d'une carte : nom semi-gras (toucher pour le renommer en place)
//! et résumé gris, centrés sous la vignette.

use bones_ui::theme;
use egui::text::{LayoutJob, TextWrapping};
use egui::{Color32, CornerRadius, Rect, Sense, vec2};

use super::library::{Card, subtitle};
use super::{Gallery, GalleryAction};

/// Nom (ou champ de renommage) et résumé, centrés sous la vignette.
pub fn show(
    ui: &mut egui::Ui,
    gallery: &mut Gallery,
    card: &Card,
    thumb: Rect,
    caption: Rect,
    actions: &mut Vec<GalleryAction>,
) {
    let title_rect = Rect::from_center_size(
        egui::pos2(thumb.center().x, thumb.bottom() + 21.0),
        vec2(caption.width() - 12.0, 26.0),
    );
    if gallery.rename.as_ref().is_some_and(|r| r.path == card.path) {
        // Champ à la place du nom ; le résumé s'efface pendant la saisie.
        let field = Rect::from_center_size(
            title_rect.center() + vec2(0.0, 2.0),
            vec2(thumb.width() + 24.0, 30.0),
        );
        rename_field(ui, gallery, field, actions);
        return;
    } else {
        let galley = elided(
            ui,
            &card.name,
            theme::semibold(15.0),
            theme::TEXT_LIST,
            title_rect.width(),
        );
        let text_rect = Rect::from_center_size(title_rect.center(), galley.size() + vec2(12.0, 8.0));
        let title = ui.interact(
            text_rect,
            egui::Id::unique(("card-title", &card.path)),
            Sense::click(),
        );
        let color = if title.is_pointer_button_down_on() {
            theme::TEXT_DIM
        } else {
            theme::TEXT_LIST
        };
        ui.painter()
            .galley(title_rect.center() - galley.size() * 0.5, galley, color);
        if title.clicked() && !gallery.is_selecting() && gallery.is_idle() {
            gallery.menu = None;
            gallery.start_rename(card.path.clone());
        }
    }
    if let Some(info) = &card.info {
        let galley = elided(
            ui,
            &subtitle(info),
            egui::FontId::proportional(13.0),
            theme::TEXT_DIM,
            caption.width(),
        );
        let pos = egui::pos2(thumb.center().x, thumb.bottom() + 41.0) - galley.size() * 0.5;
        ui.painter().galley(pos, galley, theme::TEXT_DIM);
    }
}

fn elided(
    ui: &egui::Ui,
    text: &str,
    font: egui::FontId,
    color: Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = TextWrapping::truncate_at_width(width);
    ui.painter().layout_job(job)
}

/// Champ de renommage : tout le nom sélectionné à l'ouverture ; Entrée ou
/// toucher ailleurs valide, Échap annule.
fn rename_field(ui: &mut egui::Ui, gallery: &mut Gallery, rect: Rect, actions: &mut Vec<GalleryAction>) {
    let Some(rename) = &mut gallery.rename else { return };
    let id = egui::Id::unique(("rename", &rename.path));
    let field = rect;
    ui.painter()
        .rect_filled(field, CornerRadius::same(8), theme::WIDGET);
    let edit = egui::TextEdit::singleline(&mut rename.text)
        .id(id)
        .font(theme::semibold(15.0))
        .text_color(theme::TEXT)
        .align(egui::Align2::CENTER_CENTER)
        .frame(egui::Frame::NONE)
        .margin(egui::Margin::symmetric(8, 5))
        .desired_width(field.width() - 16.0)
        .char_limit(80);
    let output = ui
        .scope_builder(egui::UiBuilder::new().max_rect(field), |ui| {
            ui.spacing_mut().interact_size.y = field.height();
            edit.show(ui)
        })
        .inner;
    if std::mem::take(&mut rename.focus) {
        output.response.request_focus();
        let mut state = output.state;
        let all = egui::text::CCursorRange::two(
            egui::text::CCursor::new(0),
            egui::text::CCursor::new(rename.text.chars().count()),
        );
        state.cursor.set_char_range(Some(all));
        state.store(ui.ctx(), id);
        return;
    }
    if output.response.lost_focus() {
        let cancelled = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let Some(rename) = gallery.rename.take() else {
            return;
        };
        let name = rename.text.trim().to_owned();
        let unchanged = gallery.library.get(&rename.path).is_some_and(|c| c.name == name);
        if !cancelled && !name.is_empty() && !unchanged {
            gallery.library.rename(&rename.path, &name);
            actions.push(GalleryAction::Rename(rename.path, name));
        }
    }
}
