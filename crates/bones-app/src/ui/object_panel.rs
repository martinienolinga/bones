//! Panneau du mode Objet (bas de l'écran, comme l'auto-rig) : palette des
//! primitives, import d'un maillage en objet, et résumé de l'objet
//! sélectionné (nom, suppression, duplication, accès aux propriétés). Les
//! propriétés détaillées et la hiérarchie vivent dans l'outliner unifié.

use std::path::Path;

use bones_core::Editor;
use bones_core::mesh::Primitive;
use bones_core::scene::ObjectRef;
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{EDGE, Popover, UiAction, UiState, block};

/// Glyphe de chaque primitive.
fn primitive_icon(p: Primitive) -> Icon {
    match p {
        Primitive::Cube => Icon::Cube,
        Primitive::Sphere => Icon::Sphere,
        Primitive::Cylinder => Icon::Cylinder,
        Primitive::Cone => Icon::Cone,
        Primitive::Plane => Icon::Plane,
        Primitive::Torus => Icon::Torus,
    }
}

/// Affiche le panneau ; retourne sa zone (et celle de la liste d'import).
pub fn show(
    ctx: &egui::Context,
    editor: &mut Editor,
    state: &mut UiState,
    actions: &mut Vec<UiAction>,
) -> Vec<egui::Rect> {
    let docs = state.docs_dir.clone();
    let bar = block(
        ctx,
        "object-panel",
        egui::Align2::CENTER_BOTTOM,
        [0.0, -EDGE],
        theme::panel_frame(),
        |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                heading(ui);
                divider(ui);
                for p in Primitive::ALL {
                    if widgets::tool_cell(ui, primitive_icon(p), p.label(), false).clicked() {
                        editor.add_primitive(p);
                    }
                }
                divider(ui);
                let open = state.object_import;
                if widgets::tool_cell(ui, Icon::Import, "Import", open).clicked() {
                    state.object_import = !open;
                }
                divider(ui);
                selected(ui, editor, state);
            });
        },
    );
    let mut rects = vec![bar];
    if state.object_import {
        rects.push(import_list(ctx, state, &docs, actions, bar.top()));
    }
    rects
}

/// Titre « Object » avec son glyphe d'accent.
fn heading(ui: &mut egui::Ui) {
    let (glyph, _) = ui.allocate_exact_size(egui::vec2(30.0, 48.0), egui::Sense::hover());
    bones_ui::icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(glyph.center(), egui::vec2(26.0, 26.0)),
        Icon::Object,
        theme::ACCENT,
    );
    ui.label(egui::RichText::new("Object").font(theme::semibold(18.0)));
}

fn divider(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 48.0), egui::Sense::hover());
    ui.painter().line_segment(
        [
            rect.center_top() + egui::vec2(0.0, 8.0),
            rect.center_bottom() - egui::vec2(0.0, 8.0),
        ],
        egui::Stroke::new(0.5, theme::HAIRLINE),
    );
}

/// Résumé de l'objet sélectionné : nom, propriétés, copie, suppression.
fn selected(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState) {
    let id = editor.selected_extra();
    let name = match editor.selected_object {
        Some(ObjectRef::Body) => editor.rig.body.name.clone(),
        Some(ObjectRef::Extra(i)) => editor.rig.object(i).map(|o| o.name.clone()).unwrap_or_default(),
        None => "No selection".to_owned(),
    };
    let color = if id.is_some() {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    ui.add_sized(
        [120.0, 48.0],
        egui::Label::new(egui::RichText::new(name).size(16.0).color(color)).truncate(),
    );
    ui.add_enabled_ui(id.is_some(), |ui| {
        if widgets::tool_cell(
            ui,
            Icon::Adjust,
            "Properties",
            state.popover == Some(Popover::Outliner),
        )
        .clicked()
        {
            // Ouvre l'outliner sur la section Scène (propriétés + hiérarchie).
            state.popover = Some(Popover::Outliner);
            state.popover_anchor = ui.ctx().content_rect().center().x;
            state.bones_section = 0;
        }
        if widgets::tool_cell(ui, Icon::Duplicate, "Duplicate", false).clicked()
            && let Some(id) = id
        {
            let _ = editor.duplicate_object(id, true);
        }
        if widgets::tool_cell(ui, Icon::Delete, "Delete", false).clicked()
            && let Some(id) = id
        {
            let _ = editor.delete_object(id);
        }
    });
}

/// Liste flottante des maillages importables (dossier Documents et Inbox).
fn import_list(
    ctx: &egui::Context,
    state: &mut UiState,
    docs: &Path,
    actions: &mut Vec<UiAction>,
    bar_top: f32,
) -> egui::Rect {
    const WIDTH: f32 = 420.0;
    let files: Vec<crate::files::ImportFile> = crate::files::importable(docs)
        .into_iter()
        .filter(|f| f.format.is_some())
        .collect();
    let area = egui::Area::new(egui::Id::unique("object-import"))
        .pivot(egui::Align2::CENTER_BOTTOM)
        .fixed_pos(egui::pos2(ctx.content_rect().center().x, bar_top - 12.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            theme::panel_frame().show(ui, |ui| {
                ui.set_width(WIDTH);
                ui.label(egui::RichText::new("Import as object").font(theme::semibold(20.0)));
                ui.add_space(6.0);
                if files.is_empty() {
                    widgets::note(
                        ui,
                        "Drop glTF, FBX, OBJ, STL, PLY or BVH files in the app's Documents folder.",
                    );
                } else {
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        for f in &files {
                            let badge = f.format.map(|fmt| fmt.extension().to_uppercase());
                            if widgets::list_row(ui, Icon::Mesh, &f.name, None, badge.as_deref()).clicked() {
                                actions.push(UiAction::ImportObject(f.path.clone()));
                                state.object_import = false;
                            }
                        }
                    });
                }
            });
        })
        .response
        .rect;
    let rect = area;
    let outside = ctx.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !rect.contains(p) && p.y < bar_top)
    });
    if outside {
        state.object_import = false;
    }
    rect
}
