//! Barre contextuelle du manipulateur 3D, comme celle de l'outil
//! Transformer de Procreate : les quatre jeux de poignées (déplacer,
//! tourner, mettre à l'échelle, tout), le repère (global / local), le
//! magnétisme et un bouton « Options » qui ouvre une petite fenêtre (pas du
//! magnétisme, affichage et taille du manipulateur).
//!
//! Elle n'apparaît que là où le manipulateur agit : en Pose avec un os
//! sélectionné (outils Move et Rotate), en Édition avec l'outil Adjust.

use bones_core::manipulator::{ManipulatorMode, TransformSpace};
use bones_core::{EditTool, Editor, Mode, PoseTool};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{EDGE, UiState};

/// État propre à la barre : fenêtre d'options ouverte et abscisse de son
/// bouton (pointe de l'encoche).
#[derive(Default)]
pub struct BarState {
    pub options: bool,
    anchor: f32,
}

const MODES: [(ManipulatorMode, Icon, &str); 4] = [
    (ManipulatorMode::Translate, Icon::Move, "Move (W)"),
    (ManipulatorMode::Rotate, Icon::Rotate, "Rotate (E)"),
    (ManipulatorMode::Scale, Icon::Scale, "Scale (R)"),
    (
        ManipulatorMode::Combined,
        Icon::Transform,
        "Move, rotate and scale (Q)",
    ),
];

/// Vrai si le manipulateur a un objet dans le contexte courant (mêmes
/// règles que `bones-core`).
pub fn available(editor: &Editor) -> bool {
    match editor.mode {
        Mode::Pose => editor.pose_tool != PoseTool::Pin && editor.selected.is_some(),
        Mode::Edit => editor.edit_tool == EditTool::Select,
        Mode::AutoRig | Mode::WeightPaint => false,
        Mode::Object => editor.selected_object.is_some(),
    }
}

/// Dessine la barre au-dessus de l'ordonnée `bottom` (haut de la frise, ou
/// bas de l'écran) ; retourne les zones occupées.
pub fn show(ctx: &egui::Context, editor: &mut Editor, state: &mut UiState, bottom: f32) -> Vec<egui::Rect> {
    let screen = ctx.content_rect();
    let lift = screen.bottom() - bottom + 8.0;
    let frame = theme::bar_frame().inner_margin(egui::Margin::symmetric(8, 4));
    let mut options_x = None;
    let open = state.transform.options;
    let bar = egui::Area::new(egui::Id::unique("transform-bar"))
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -lift])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    options_x = tabs(ui, editor, open);
                });
            })
        })
        .response
        .rect;
    if let Some(x) = options_x {
        state.transform.options = !state.transform.options;
        state.transform.anchor = x;
    }
    let mut rects = vec![bar];
    if state.transform.options {
        let rect = options(ctx, editor, state, bar.top());
        let tapped_outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !rect.contains(p) && !bar.contains(p))
        });
        if tapped_outside {
            state.transform.options = false;
        }
        rects.push(rect);
    }
    rects
}

/// Onglets de la barre ; retourne l'abscisse du bouton « Options » s'il a
/// été touché.
fn tabs(ui: &mut egui::Ui, editor: &mut Editor, options_open: bool) -> Option<f32> {
    let m = editor.manipulator;
    for (mode, icon, hint) in MODES {
        let active = m.visible && m.mode == mode;
        if widgets::bar_tab(ui, icon, mode.label(), active)
            .on_hover_text(hint)
            .clicked()
        {
            editor.set_manipulator_mode(mode);
        }
    }
    widgets::separator_dot(ui);
    // Sans os sélectionné (modèle entier), le repère est toujours global.
    let (icon, hint) = match m.space {
        TransformSpace::World => (Icon::Globe, "World axes — tap for the bone's axes (X)"),
        TransformSpace::Local => (Icon::Local, "Bone axes — tap for world axes (X)"),
    };
    let space = ui
        .add_enabled_ui(editor.selected.is_some(), |ui| {
            widgets::bar_tab(ui, icon, m.space.label(), false)
        })
        .inner;
    if space.on_hover_text(hint).clicked() {
        editor.toggle_manipulator_space();
    }
    if widgets::bar_tab(ui, Icon::Magnet, "Snap", m.snap)
        .on_hover_text("Snapping (hold ⇧ to invert)")
        .clicked()
    {
        editor.toggle_manipulator_snap();
    }
    widgets::separator_dot(ui);
    let button = widgets::bar_tab(ui, Icon::Adjust, "Options", options_open);
    button.clicked().then_some(button.rect.center().x)
}

/// Fenêtre d'options posée au-dessus de la barre, encoche vers le bas.
fn options(ctx: &egui::Context, editor: &mut Editor, state: &mut UiState, bar_top: f32) -> egui::Rect {
    const WIDTH: f32 = 380.0;
    const NOTCH: f32 = 10.0;
    let screen = ctx.content_rect();
    let left = (state.transform.anchor - WIDTH * 0.5).clamp(EDGE, (screen.right() - WIDTH - EDGE).max(EDGE));
    let area = egui::Area::new(egui::Id::unique("transform-options"))
        .pivot(egui::Align2::LEFT_BOTTOM)
        .fixed_pos(egui::pos2(left, bar_top - NOTCH - 4.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let panel = theme::panel_frame().show(ui, |ui| {
                ui.set_width(WIDTH - 40.0);
                ui.spacing_mut().item_spacing.y = 8.0;
                content(ui, editor);
            });
            let rect = panel.response.rect;
            let x = state
                .transform
                .anchor
                .clamp(rect.left() + 30.0, rect.right() - 30.0);
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(x + 12.0, rect.bottom() - 1.0),
                    egui::pos2(x, rect.bottom() + NOTCH),
                    egui::pos2(x - 12.0, rect.bottom() - 1.0),
                ],
                theme::PANEL,
                egui::Stroke::NONE,
            ));
            rect
        });
    area.inner.with_max_y(area.inner.bottom() + NOTCH)
}

fn content(ui: &mut egui::Ui, editor: &mut Editor) {
    ui.label(egui::RichText::new("Transform").font(theme::semibold(24.0)));
    let m = &mut editor.manipulator;
    widgets::section_title(ui, "Snapping");
    widgets::group(ui, |g| {
        g.switch_row(
            &mut m.snap,
            "Snapping",
            Some("Angles, moves and scale go by steps"),
        );
    });
    let chip = |suffix: &'static str, decimals: usize, speed: f64, range: std::ops::RangeInclusive<f32>| {
        move |ui: &mut egui::Ui, v: &mut f32| {
            widgets::drag_chip(ui, v, range.clone(), speed, decimals, suffix, 96.0)
        }
    };
    widgets::slider_row_with(
        ui,
        &mut m.angle_step_deg,
        1.0..=90.0,
        "Angle step",
        chip("°", 0, 0.2, 1.0..=90.0),
    );
    widgets::slider_row_with(
        ui,
        &mut m.distance_step,
        0.005..=0.5,
        "Distance step",
        chip(" m", 3, 0.001, 0.001..=10.0),
    );
    widgets::slider_row_with(
        ui,
        &mut m.scale_step,
        0.01..=1.0,
        "Scale step",
        chip("", 2, 0.005, 0.01..=10.0),
    );
    widgets::note(ui, "Hold ⇧ while dragging to invert snapping.");
    widgets::section_title(ui, "Gizmo");
    widgets::group(ui, |g| {
        g.switch_row(&mut m.visible, "Show gizmo", Some("Handles on the selection"));
    });
    widgets::slider_row(ui, &mut m.size, 48.0..=160.0, "Size", "pt");
}
