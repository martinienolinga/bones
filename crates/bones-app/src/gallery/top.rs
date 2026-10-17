//! Barre de la galerie : grand titre à gauche ; à droite, la capsule
//! « Select · Import » et le bouton rond « + ». En mode sélection : nombre
//! de projets cochés, puis « Duplicate · Delete · Done ».

use bones_ui::icons::Icon;
use bones_ui::theme;
use bones_ui::widgets::{self, CapsuleItem, CapsuleTone};

use super::layout::{self, BAR_H};
use super::{BACKGROUND, Gallery, GalleryAction, Menu};

/// Titre de l'app.
pub const TITLE: &str = "PaintSharp Bones";

pub fn show(ui: &mut egui::Ui, gallery: &mut Gallery, actions: &mut Vec<GalleryAction>) {
    let screen = ui.ctx().content_rect();
    let bar = egui::Rect::from_min_size(screen.min, egui::vec2(screen.width(), BAR_H));
    ui.painter().rect_filled(bar, 0, BACKGROUND);
    let margin = layout::margin(screen.width());
    let inner = bar
        .shrink2(egui::vec2(margin, 0.0))
        .translate(egui::vec2(0.0, 4.0));

    let mut left = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    left.label(
        egui::RichText::new(TITLE)
            .font(theme::semibold(34.0))
            .color(theme::TEXT),
    );
    if let Some(selection) = &gallery.selection {
        left.add_space(14.0);
        let text = match selection.len() {
            0 => "Select projects".to_owned(),
            1 => "1 project selected".to_owned(),
            n => format!("{n} projects selected"),
        };
        left.label(egui::RichText::new(text).size(17.0).color(theme::TEXT_DIM));
    }

    let mut right = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    right.spacing_mut().item_spacing.x = 10.0;
    if gallery.is_selecting() {
        selection_buttons(&mut right, gallery, actions);
    } else {
        main_buttons(&mut right, gallery, actions);
    }
}

fn main_buttons(ui: &mut egui::Ui, gallery: &mut Gallery, actions: &mut Vec<GalleryAction>) {
    let plus_open = matches!(gallery.menu, Some(Menu::New(_)));
    let plus = widgets::round_button(ui, Icon::Plus, plus_open, "New project");
    if plus.clicked() {
        gallery.menu = (!plus_open).then_some(Menu::New(plus.rect));
    }
    let import_open = matches!(gallery.menu, Some(Menu::Import(_)));
    let any = !gallery.library.cards().is_empty();
    let items = [
        CapsuleItem::new("Select").enabled(any),
        CapsuleItem::new("Import").tone(if import_open {
            CapsuleTone::Active
        } else {
            CapsuleTone::Plain
        }),
    ];
    let capsule = widgets::capsule(ui, &items);
    #[cfg(test)]
    {
        gallery.buttons = capsule.rects.clone();
        gallery.buttons.push(plus.rect);
    }
    match capsule.clicked {
        Some(0) => {
            gallery.menu = None;
            gallery.rename = None;
            gallery.selection = Some(Vec::new());
        }
        Some(1) if import_open => gallery.menu = None,
        Some(1) => {
            gallery.imports = None;
            gallery.menu = Some(Menu::Import(capsule.rects[1]));
            actions.push(GalleryAction::ListImports);
        }
        _ => {}
    }
}

fn selection_buttons(ui: &mut egui::Ui, gallery: &mut Gallery, actions: &mut Vec<GalleryAction>) {
    let selected = gallery.selection.clone().unwrap_or_default();
    let any = !selected.is_empty();
    let items = [
        CapsuleItem::new("Duplicate").enabled(any),
        CapsuleItem::new("Delete").enabled(any).tone(CapsuleTone::Danger),
        CapsuleItem::new("Done").tone(CapsuleTone::Accent),
    ];
    let capsule = widgets::capsule(ui, &items);
    #[cfg(test)]
    {
        gallery.buttons = capsule.rects.clone();
    }
    match capsule.clicked {
        Some(0) => {
            actions.push(GalleryAction::Duplicate(selected));
            gallery.selection = Some(Vec::new());
        }
        Some(1) => gallery.confirm = Some(selected),
        Some(2) => gallery.selection = None,
        _ => {}
    }
}
