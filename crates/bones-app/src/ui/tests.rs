//! Tests de l'interface sans GPU : le studio des os modifie bien l'os
//! sélectionné quand on glisse ses valeurs.

use bones_core::{Editor, Mode, Rig, Viewport};
use bones_ui::theme;
use egui::{Color32, Event, FontId, PointerButton, RawInput, Rect, pos2, vec2};

use super::{Busy, Popover, UiState, show};
use crate::settings::Settings;

struct Harness {
    ctx: egui::Context,
    editor: Editor,
    state: UiState,
}

impl Harness {
    fn new() -> Self {
        let ctx = egui::Context::default();
        bones_ui::install(&ctx);
        let docs = std::env::temp_dir().join("bones-ui-tests");
        Self {
            ctx,
            editor: Editor::new(Rig::demo()),
            state: UiState::new(docs, Settings::default()),
        }
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1366.0, 1024.0))),
            events,
            ..Default::default()
        };
        let viewport = Viewport::new(1366.0, 1024.0);
        let busy = Busy {
            binding: false,
            jobs: 0,
        };
        let (editor, state) = (&mut self.editor, &mut self.state);
        let mut output = self.ctx.run_ui(input, |ui| {
            show(ui.ctx(), editor, state, &viewport, busy);
        });
        output.textures_delta.clear();
    }
}

fn button(pos: egui::Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}

#[test]
fn dragging_the_head_chip_moves_the_bone() {
    let mut h = Harness::new();
    h.editor.set_mode(Mode::Edit);
    let bone = h.editor.rig.skeleton.find("arm.l").expect("os du mannequin");
    h.editor.select(Some(bone));
    h.state.popover = Some(Popover::Outliner);
    h.state.popover_anchor = 212.0;
    h.state.bones_section = 2;
    for _ in 0..4 {
        h.frame(vec![]);
    }
    let before = h.editor.bone_info(bone).head.x;
    // Pastille « Head X » : première pastille de la ligne Head (voir la
    // capture), repérée par sa position dans le studio.
    let start = pos2(557.0, 260.0);
    h.frame(vec![Event::PointerMoved(start), button(start, true)]);
    for i in 1..=8 {
        h.frame(vec![Event::PointerMoved(start + vec2(8.0 * i as f32, 0.0))]);
    }
    h.frame(vec![button(start + vec2(64.0, 0.0), false)]);
    let after = h.editor.bone_info(bone).head.x;
    assert!(after - before > 0.08, "tête : {before} → {after}");
}

#[test]
fn reparent_messages_become_a_hint_others_a_toast() {
    let mut h = Harness::new();
    h.editor.set_mode(Mode::Edit);
    h.editor.edit_tool = bones_core::EditTool::Reparent;
    h.editor.status = Some("Weights ready".into());
    h.frame(vec![]);
    assert!(h.state.hint.is_none());
    h.editor.status = Some("arm.l: tap the new parent (or empty space for none)".into());
    h.frame(vec![]);
    assert!(h.state.hint.as_deref().is_some_and(|m| m.starts_with("arm.l")));
    h.editor.edit_tool = bones_core::EditTool::Select;
    h.frame(vec![]);
    assert!(h.state.hint.is_none());
}

fn key(key: egui::Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Default::default(),
    }
}

#[test]
fn transform_bar_follows_the_manipulator_context() {
    use super::transform_bar::available;
    use bones_core::{EditTool, PoseTool};
    let mut h = Harness::new();
    let bone = h.editor.rig.skeleton.find("arm.l").expect("os du mannequin");
    assert!(!available(&h.editor), "Pose sans sélection");
    h.editor.select(Some(bone));
    assert!(available(&h.editor));
    h.editor.pose_tool = PoseTool::Pin;
    assert!(!available(&h.editor), "outil Épingler");
    h.editor.set_mode(Mode::Edit);
    h.editor.select(None);
    assert!(available(&h.editor), "Édition : modèle entier");
    h.editor.edit_tool = EditTool::Roll;
    assert!(!available(&h.editor), "outils d'édition directe");
    h.editor.set_mode(Mode::AutoRig);
    assert!(!available(&h.editor));
}

#[test]
fn manipulator_keys_change_mode_and_are_saved() {
    use bones_core::manipulator::ManipulatorMode;
    let mut h = Harness::new();
    h.frame(vec![]);
    h.state.settings_dirty = false;
    for (k, mode) in [
        (egui::Key::W, ManipulatorMode::Translate),
        (egui::Key::E, ManipulatorMode::Rotate),
        (egui::Key::R, ManipulatorMode::Scale),
        (egui::Key::Q, ManipulatorMode::Combined),
    ] {
        h.frame(vec![key(k)]);
        assert_eq!(h.editor.manipulator.mode, mode);
        assert_eq!(h.state.settings.manipulator.mode, mode);
    }
    assert!(h.state.settings_dirty);
    let space = h.editor.manipulator.space;
    h.frame(vec![key(egui::Key::X)]);
    assert_ne!(h.editor.manipulator.space, space);
    assert_eq!(h.state.settings.manipulator.space, h.editor.manipulator.space);
}

#[test]
fn saved_manipulator_settings_reach_the_editor() {
    let mut h = Harness::new();
    h.state.settings.manipulator.snap = true;
    h.state.settings.manipulator.angle_step_deg = 30.0;
    h.editor.manipulator.invert_snap = true;
    h.frame(vec![]);
    assert!(h.editor.manipulator.snap);
    assert_eq!(h.editor.manipulator.angle_step_deg, 30.0);
    // L'inversion par Maj appartient au geste en cours, pas aux réglages.
    assert!(h.editor.manipulator.invert_snap);
    assert!(!h.state.settings.manipulator.invert_snap);
}

/// Clic (appui puis relâchement) au point `p`.
fn click(h: &mut Harness, p: egui::Pos2) {
    h.frame(vec![Event::PointerMoved(p), button(p, true)]);
    h.frame(vec![button(p, false)]);
}

/// Centre du segment `index` du contrôle de mode, aligné à droite de la barre
/// du haut (reproduit la géométrie de `widgets::segmented`).
fn segment_center(ctx: &egui::Context, labels: &[&str], index: usize) -> egui::Pos2 {
    let painter = egui::Painter::new(ctx.clone(), egui::LayerId::debug(), Rect::EVERYTHING);
    let widths: Vec<f32> = labels
        .iter()
        .map(|o| {
            let bold = painter.layout_no_wrap((*o).to_owned(), theme::semibold(14.0), Color32::WHITE);
            let reg = painter.layout_no_wrap((*o).to_owned(), FontId::proportional(14.0), Color32::WHITE);
            bold.size().x.max(reg.size().x) + 30.0
        })
        .collect();
    let total = widths.iter().sum::<f32>() + 6.0;
    let right = 1366.0 - 10.0;
    let x = right - total + 3.0 + widths[..index].iter().sum::<f32>() + widths[index] * 0.5;
    pos2(x, super::BAR_H * 0.5)
}

/// Zone du bandeau d'outils (collé au bord droit, centré verticalement).
fn toolbox_rect(h: &Harness) -> Rect {
    *h.state
        .rects
        .iter()
        .find(|r| (r.right() - 1366.0).abs() < 2.0 && r.top() > super::BAR_H && r.height() > 180.0)
        .expect("bandeau d'outils")
}

#[test]
fn mode_control_switches_to_weight_paint() {
    let mut h = Harness::new();
    // Deux images pour que les zones ancrées aux bords se stabilisent.
    h.frame(vec![]);
    h.frame(vec![]);
    let p = segment_center(&h.ctx, &["Pose", "Weights", "Edit", "Auto-Rig"], 1);
    click(&mut h, p);
    assert_eq!(h.editor.mode, Mode::WeightPaint);
}

#[test]
fn weight_paint_panels_build_without_panic() {
    let mut h = Harness::new();
    h.editor.set_mode(Mode::WeightPaint);
    for popover in [None, Some(Popover::WeightsTools), Some(Popover::Brush)] {
        h.state.popover = popover;
        h.state.popover_anchor = 1000.0;
        for _ in 0..3 {
            h.frame(vec![]);
        }
    }
    // Chaque section du studio de pinceaux.
    h.state.popover = Some(Popover::Brush);
    for section in 0..3 {
        h.state.brush_section = section;
        for _ in 0..3 {
            h.frame(vec![]);
        }
    }
    // Confirmation des poids automatiques.
    h.state.weights_tools.confirm_automatic = true;
    h.frame(vec![]);
    assert_eq!(h.editor.mode, Mode::WeightPaint);
    assert!(!h.state.rects.is_empty());
}

#[test]
fn sample_button_toggles_the_eyedropper() {
    use bones_core::weight_paint::PaintTool;
    let mut h = Harness::new();
    h.editor.set_mode(Mode::WeightPaint);
    h.frame(vec![]);
    h.frame(vec![]);
    // Bandeau gauche : le bouton pipette est sous les trois curseurs.
    let strip = *h
        .state
        .rects
        .iter()
        .find(|r| r.left() < 2.0 && r.height() > 300.0 && r.top() > super::BAR_H)
        .expect("bandeau gauche");
    assert_eq!(h.editor.weight_paint.brush.tool, PaintTool::Draw);
    // La pipette est le dernier bouton avant Annuler/Rétablir.
    let sample = pos2(strip.center().x, strip.bottom() - 120.0);
    click(&mut h, sample);
    assert_eq!(h.editor.weight_paint.brush.tool, PaintTool::Sample);
}

#[test]
fn toolbox_toggles_weight_options() {
    let mut h = Harness::new();
    h.editor.set_mode(Mode::WeightPaint);
    h.frame(vec![]);
    h.frame(vec![]);
    let tb = toolbox_rect(&h);
    // Première cellule : « X-Mirror » (haut du bandeau, sous la marge de 8 pt).
    let x_mirror = pos2(tb.center().x, tb.top() + 36.0);
    assert!(!h.state.settings.weight_paint.mirror_x);
    click(&mut h, x_mirror);
    assert!(
        h.state.settings.weight_paint.mirror_x,
        "la bascule X-Mirror est enregistrée"
    );
}

#[test]
fn view_buttons_form_a_grid_left_of_the_cube() {
    let mut h = Harness::new();
    h.frame(vec![]);
    h.frame(vec![]);
    // Guide de rotation : 116 pt, ancré à 76 pt du bord droit, 60 pt du haut.
    let cube = Rect::from_min_size(pos2(1366.0 - 76.0 - 116.0, 60.0), vec2(116.0, 116.0));
    let grid = *h
        .state
        .rects
        .iter()
        .find(|r| (r.right() - (cube.left() - 8.0)).abs() < 1.0)
        .expect("grille des boutons à gauche du guide");
    assert!(
        (grid.center().y - cube.center().y).abs() < 1.0,
        "centrée : {grid:?}"
    );
    assert!(
        (grid.width() - grid.height()).abs() < 2.0,
        "deux par ligne : {grid:?}"
    );
    // Cellules de 44 pt dans une marge de 6 pt : verrou en haut à gauche,
    // vue orthographique en haut à droite.
    let lock = pos2(grid.left() + 28.0, grid.top() + 28.0);
    let ortho = pos2(grid.right() - 28.0, grid.top() + 28.0);
    let (locked, ortho_before) = (h.editor.camera.locked, h.editor.camera.orthographic);
    click(&mut h, ortho);
    assert_ne!(h.editor.camera.orthographic, ortho_before);
    click(&mut h, lock);
    assert_ne!(h.editor.camera.locked, locked);
}

#[test]
fn weight_slider_lowers_the_brush_weight() {
    let mut h = Harness::new();
    h.editor.set_mode(Mode::WeightPaint);
    h.frame(vec![]);
    h.frame(vec![]);
    let strip = *h
        .state
        .rects
        .iter()
        .find(|r| r.left() < 2.0 && r.height() > 300.0 && r.top() > super::BAR_H)
        .expect("bandeau gauche");
    assert_eq!(h.editor.weight_paint.brush.weight, 1.0);
    // Premier curseur (Weight) : glisser le pouce vers le bas réduit le poids.
    let start = pos2(strip.center().x, strip.top() + 120.0);
    h.frame(vec![Event::PointerMoved(start), button(start, true)]);
    for i in 1..=6 {
        h.frame(vec![Event::PointerMoved(start + vec2(0.0, 10.0 * i as f32))]);
    }
    h.frame(vec![button(start + vec2(0.0, 60.0), false)]);
    assert!(
        h.editor.weight_paint.brush.weight < 0.95,
        "poids après glissement : {}",
        h.editor.weight_paint.brush.weight
    );
}

#[test]
fn brush_library_picks_a_tool() {
    use bones_core::weight_paint::PaintTool;
    let mut h = Harness::new();
    h.editor.set_mode(Mode::WeightPaint);
    h.state.popover = Some(Popover::Brush);
    h.state.popover_anchor = 1000.0;
    h.state.brush_section = 0;
    for _ in 0..5 {
        h.frame(vec![]);
    }
    let pop = *h.state.rects.last().expect("studio de pinceaux");
    assert_eq!(h.editor.weight_paint.brush.tool, PaintTool::Draw);
    // Deuxième vignette de la grille d'outils : « Blur ».
    let blur = pos2(pop.left() + 440.0, pop.top() + 158.0);
    click(&mut h, blur);
    assert_eq!(h.editor.weight_paint.brush.tool, PaintTool::Blur);
}

#[test]
fn weights_tools_menu_applies_and_closes() {
    let mut h = Harness::new();
    h.editor.set_mode(Mode::WeightPaint);
    h.state.popover = Some(Popover::WeightsTools);
    h.state.popover_anchor = 1000.0;
    for _ in 0..5 {
        h.frame(vec![]);
    }
    let pop = *h.state.rects.last().expect("menu d'outils");
    // Première ligne : « Normalize All » (applique l'outil et referme le menu).
    let row = pos2(pop.left() + 60.0, pop.top() + 150.0);
    click(&mut h, row);
    assert!(
        h.state.popover.is_none(),
        "la ligne applique l'outil et ferme le menu"
    );
}
