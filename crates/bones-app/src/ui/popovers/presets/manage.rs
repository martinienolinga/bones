//! Gestion des préréglages utilisateur : liste des fichiers importables
//! ([`View::Import`]) et menu contextuel d'un préréglage (appui long) —
//! renommer, dupliquer, exporter, supprimer. Les opérations fichier passent
//! par [`bones_core::presets`].

use std::path::Path;

use bones_core::presets;
use bones_ui::icons::{self, Icon};
use bones_ui::{theme, widgets};
use egui::vec2;

use super::library::{self, Entry, Importable};
use super::{PresetsState, View, presets_dir};
use crate::files;

/// Note discrète, repère pour les fonctions en attente d'API (expressions).
pub fn hook_note(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        let (dot, _) = ui.allocate_exact_size(vec2(14.0, 16.0), egui::Sense::hover());
        ui.painter().circle_filled(dot.center(), 2.5, theme::TEXT_FAINT);
        ui.label(egui::RichText::new(text).size(13.0).color(theme::TEXT_FAINT));
    });
}

/// Vue « Import » : fichiers d'animation des Documents et de la boîte de
/// réception, un par ligne.
pub fn import(ui: &mut egui::Ui, p: &mut PresetsState, docs: &Path, body: f32) {
    if p.imports.is_none() {
        p.imports = Some(library::importable(docs));
    }
    let list = p.imports.as_ref().unwrap();
    if list.is_empty() {
        ui.add_space((body * 0.28).max(30.0));
        ui.vertical_centered(|ui| {
            let (g, _) = ui.allocate_exact_size(vec2(40.0, 40.0), egui::Sense::hover());
            icons::paint(ui.painter(), g, Icon::Import, theme::TEXT_FAINT);
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Nothing to import").font(theme::semibold(18.0)));
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Drop .kb3da, BVH, glTF, GLB or FBX files into the app's Documents.")
                    .size(13.0)
                    .color(theme::TEXT_DIM),
            );
        });
        return;
    }
    let files: Vec<(String, String, std::path::PathBuf)> = list
        .iter()
        .map(|f: &Importable| (f.name.clone(), f.ext.to_uppercase(), f.path.clone()))
        .collect();
    let mut chosen = None;
    egui::ScrollArea::vertical()
        .id_salt("preset-import")
        .auto_shrink([false, false])
        .max_height(body)
        .show(ui, |ui| {
            widgets::group(ui, |g| {
                for (name, ext, path) in &files {
                    if g.icon_row(Icon::Motion, name, None, Some(ext)).clicked() {
                        chosen = Some(path.clone());
                    }
                }
            });
            hook_note(
                ui,
                "Imported motions are saved to your presets, then retargeted on apply.",
            );
        });
    if let Some(path) = chosen {
        run_import(p, docs, &path);
    }
}

fn run_import(p: &mut PresetsState, docs: &Path, path: &Path) {
    match presets::import_file(path) {
        Ok(presets) if !presets.is_empty() => {
            let dir = presets_dir(docs);
            let mut saved = 0;
            for preset in &presets {
                if presets::user::save(preset, &dir).is_ok() {
                    saved += 1;
                }
            }
            p.refresh(docs);
            p.view_slot = Some(View::Grid);
            p.notify(format!("Imported {saved} preset(s)"));
        }
        Ok(_) => p.notify("No animation found in this file"),
        Err(e) => p.notify(format!("Error: {e}")),
    }
}

/// Menu contextuel d'un préréglage utilisateur (appui long) et sa feuille de
/// renommage.
pub fn context_menu(ui: &mut egui::Ui, p: &mut PresetsState, docs: &Path) {
    if p.rename.is_some() {
        rename_sheet(ui, p, docs);
        return;
    }
    let Some(menu) = &p.menu else { return };
    let (entry, pos, confirm) = (menu.entry, menu.pos, menu.confirm_delete);
    let screen = ui.ctx().content_rect();
    let left = pos.x.min(screen.right() - 230.0);
    let top = pos.y.min(screen.bottom() - 240.0);
    let area = egui::Area::new(egui::Id::unique("preset-menu"))
        .fixed_pos(egui::pos2(left, top))
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            theme::panel_frame()
                .inner_margin(egui::Margin::same(6))
                .show(ui, |ui| {
                    ui.set_width(220.0);
                    if widgets::list_row(ui, Icon::Edit, "Rename", None, None).clicked() {
                        let name = p.library.info(entry).name.clone();
                        p.rename = Some((entry, name));
                    }
                    if widgets::list_row(ui, Icon::Duplicate, "Duplicate", None, None).clicked() {
                        act(p, docs, entry, Action::Duplicate);
                    }
                    if widgets::list_row(ui, Icon::Export, "Export", None, None).clicked() {
                        act(p, docs, entry, Action::Export);
                    }
                    let delete = if confirm { "Confirm delete" } else { "Delete" };
                    if widgets::list_row(ui, Icon::Delete, delete, None, None).clicked() {
                        if confirm {
                            act(p, docs, entry, Action::Delete);
                        } else if let Some(m) = &mut p.menu {
                            m.confirm_delete = true;
                        }
                    }
                });
        });
    let rect = area.response.rect;
    let outside = ui
        .ctx()
        .input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|pt| !rect.contains(pt)));
    if outside {
        p.menu = None;
    }
}

enum Action {
    Duplicate,
    Export,
    Delete,
}

fn act(p: &mut PresetsState, docs: &Path, entry: Entry, action: Action) {
    let Some(path) = p.library.path(entry).map(Path::to_path_buf) else {
        p.menu = None;
        return;
    };
    let message = match action {
        Action::Duplicate => presets::user::duplicate(&path).map(|_| "Duplicated".to_owned()),
        Action::Delete => presets::user::delete(&path).map(|_| "Deleted".to_owned()),
        Action::Export => {
            let name = p.library.info(entry).name.clone();
            let out = files::free_path(docs, &name, presets::EXTENSION);
            presets::export_file(p.library.preset(entry), &out).map(|_| format!("Exported “{name}”"))
        }
    };
    match message {
        Ok(msg) => {
            p.refresh(docs);
            p.notify(msg);
        }
        Err(e) => p.notify(format!("Error: {e}")),
    }
    p.menu = None;
}

/// Feuille de renommage d'un préréglage utilisateur.
fn rename_sheet(ui: &mut egui::Ui, p: &mut PresetsState, docs: &Path) {
    let screen = ui.ctx().content_rect();
    let mut done = false;
    egui::Area::new(egui::Id::unique("preset-rename"))
        .fixed_pos(screen.center() - vec2(180.0, 80.0))
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            theme::panel_frame()
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    ui.set_width(328.0);
                    ui.label(egui::RichText::new("Rename preset").font(theme::semibold(18.0)));
                    ui.add_space(10.0);
                    if let Some((_, text)) = &mut p.rename {
                        let edit = egui::TextEdit::singleline(text).desired_width(f32::INFINITY);
                        ui.add(edit).request_focus();
                    }
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if widgets::cancel_button(ui, "Cancel").clicked() {
                            p.rename = None;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if widgets::pill_button(ui, None, "Save", true).clicked() {
                                done = true;
                            }
                        });
                    });
                });
        });
    if done
        && let Some((entry, text)) = p.rename.take()
        && let Some(path) = p.library.path(entry).map(Path::to_path_buf)
    {
        match presets::user::rename(&path, &text) {
            Ok(_) => {
                p.refresh(docs);
                p.notify("Renamed");
            }
            Err(e) => p.notify(format!("Error: {e}")),
        }
    }
}
