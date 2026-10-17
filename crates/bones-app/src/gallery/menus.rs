//! Fenêtres de la galerie : menu « + » (nouveau projet), fichiers à
//! importer, menu d'une carte (Rename, Duplicate, Delete).

use bones_ui::icons::{self, Icon};
use bones_ui::{theme, widgets};
use egui::{Align2, CornerRadius, FontId, Rect, Sense, Stroke, pos2, vec2};

use super::popover::{self, header};
use super::{Gallery, GalleryAction, Menu};
use crate::files::ImportFile;
use crate::projects::Template;

const IDS: [&str; 3] = ["gallery-new", "gallery-import", "gallery-card"];

pub fn show(ctx: &egui::Context, gallery: &mut Gallery, actions: &mut Vec<GalleryAction>) {
    let Some(menu) = gallery.menu.clone() else {
        for id in IDS {
            popover::reset(ctx, id);
        }
        return;
    };
    if popover::scrim(ctx, "gallery-menu-scrim", 0.0) {
        gallery.menu = None;
        return;
    }
    match menu {
        Menu::New(anchor) => {
            popover::show(ctx, IDS[0], anchor, 380.0, 20, |ui| {
                new_project(ui, gallery, anchor, actions)
            });
        }
        Menu::Import(anchor) => {
            let room = ctx.content_rect().bottom() - anchor.bottom();
            popover::show(ctx, IDS[1], anchor, 420.0, 20, |ui| {
                import(ui, gallery, room, actions)
            });
        }
        Menu::Card(path, anchor) => {
            popover::show(ctx, IDS[2], anchor, 250.0, 6, |ui| {
                card_menu(ui, gallery, &path, actions)
            });
        }
    }
}

fn new_project(ui: &mut egui::Ui, gallery: &mut Gallery, anchor: Rect, actions: &mut Vec<GalleryAction>) {
    header(ui, "New project");
    let picked = widgets::group(ui, |g| {
        let rows = [
            (Icon::Pose, "Mannequin", "Rigged character, ready to pose"),
            (Icon::Bone, "Empty project", "A single root bone, no mesh"),
            (Icon::Import, "Import 3D model…", "glTF, FBX, OBJ, STL, PLY, BVH"),
        ];
        let mut picked = None;
        for (i, (icon, title, subtitle)) in rows.into_iter().enumerate() {
            if g.icon_row(icon, title, Some(subtitle), None).clicked() {
                picked = Some(i);
            }
        }
        picked
    });
    match picked {
        Some(0) => new(gallery, Template::Mannequin, actions),
        Some(1) => new(gallery, Template::Empty, actions),
        Some(_) => {
            gallery.imports = None;
            gallery.menu = Some(Menu::Import(anchor));
            actions.push(GalleryAction::ListImports);
        }
        None => {}
    }
}

fn new(gallery: &mut Gallery, template: Template, actions: &mut Vec<GalleryAction>) {
    gallery.menu = None;
    actions.push(GalleryAction::New(template));
}

fn import(ui: &mut egui::Ui, gallery: &mut Gallery, room: f32, actions: &mut Vec<GalleryAction>) {
    header(ui, "Import");
    let Some(files) = gallery.imports.clone() else {
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.add(egui::Spinner::new().size(18.0).color(theme::TEXT_DIM));
            ui.label(
                egui::RichText::new("Looking for files…")
                    .size(15.0)
                    .color(theme::TEXT_DIM),
            );
        });
        ui.add_space(8.0);
        return;
    };
    let (models, projects): (Vec<&ImportFile>, Vec<&ImportFile>) =
        files.iter().partition(|f| f.format.is_some());
    let mut picked = None;
    egui::ScrollArea::vertical()
        .max_height((room - 220.0).clamp(160.0, 520.0))
        .show(ui, |ui| {
            if files.is_empty() {
                widgets::group(ui, |g| {
                    g.value_row("No files to import", "");
                });
            }
            for (title, list) in [("3D models", &models), ("Projects", &projects)] {
                if list.is_empty() {
                    continue;
                }
                widgets::section_title(ui, title);
                widgets::group(ui, |g| {
                    for file in list {
                        let (icon, kind, badge) = describe(file);
                        if g.icon_row(icon, &file.name, Some(&kind), Some(&badge)).clicked() {
                            picked = Some((*file).clone());
                        }
                    }
                });
            }
        });
    note(
        ui,
        "Put glTF, GLB, FBX, OBJ + MTL, STL, PLY or BVH files and their textures in \
         “On My iPad › Bones” with the Files app. A 3D model becomes a new project.",
    );
    if let Some(file) = picked {
        gallery.menu = None;
        actions.push(GalleryAction::Import(file));
    }
}

/// Glyphe, nature et pastille d'un fichier importable.
fn describe(file: &ImportFile) -> (Icon, String, String) {
    match file.format {
        Some(format) => {
            // « Binary glTF (Blender, Unity…) » : seul le nom du format.
            let label = format.label();
            let kind = label.split_once(" (").map_or(label, |(kind, _)| kind);
            let icon = if format == bones_core::Format::Bvh {
                Icon::Trajectory
            } else {
                Icon::Import
            };
            (icon, kind.to_owned(), format.extension().to_uppercase())
        }
        None => (
            Icon::Gallery,
            "PaintSharp Bones project".to_owned(),
            "KBRIG".to_owned(),
        ),
    }
}

fn note(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(16, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(13.0).color(theme::TEXT_DIM));
        });
}

/// Menu d'une carte, comme un menu contextuel d'iOS : libellé à gauche,
/// glyphe à droite, « Delete » en rouge.
fn card_menu(
    ui: &mut egui::Ui,
    gallery: &mut Gallery,
    path: &std::path::Path,
    actions: &mut Vec<GalleryAction>,
) {
    ui.spacing_mut().item_spacing.y = 0.0;
    let rows = [
        (Icon::Edit, "Rename", false),
        (Icon::Duplicate, "Duplicate", false),
        (Icon::Delete, "Delete", true),
    ];
    for (i, (icon, label, danger)) in rows.into_iter().enumerate() {
        if menu_row(ui, icon, label, danger, i > 0).clicked() {
            gallery.menu = None;
            gallery.long_pressed = None;
            match i {
                0 => gallery.start_rename(path.to_owned()),
                1 => actions.push(GalleryAction::Duplicate(vec![path.to_owned()])),
                _ => gallery.confirm = Some(vec![path.to_owned()]),
            }
        }
    }
}

fn menu_row(ui: &mut egui::Ui, icon: Icon, label: &str, danger: bool, separator: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::click());
    let painter = ui.painter();
    if separator {
        let y = rect.top();
        painter.line_segment(
            [pos2(rect.left() + 12.0, y), pos2(rect.right() - 12.0, y)],
            Stroke::new(0.5, theme::HAIRLINE),
        );
    }
    let press = ui.ctx().animate_bool_with_time(
        response.id.with("press"),
        response.is_pointer_button_down_on(),
        0.1,
    );
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.1);
    let t = (0.5 * hover + press).min(1.0);
    if t > 0.0 {
        painter.rect_filled(rect, CornerRadius::same(12), theme::WIDGET.gamma_multiply(t));
    }
    let color = if danger { theme::DANGER } else { theme::TEXT_LIST };
    painter.text(
        pos2(rect.left() + 16.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(17.0),
        color,
    );
    let glyph = Rect::from_center_size(pos2(rect.right() - 26.0, rect.center().y), vec2(20.0, 20.0));
    icons::paint(
        painter,
        glyph,
        icon,
        if danger { theme::DANGER } else { theme::ICON },
    );
    response
}
