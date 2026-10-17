//! Menu des outils du menu « Weights » de Blender, appliqués à tout le
//! maillage ou à l'os peint. Les outils simples agissent d'un toucher ; les
//! outils réglables portent leurs paramètres et un bouton « Apply ». Les
//! poids automatiques, qui écrasent les poids peints, passent par une
//! confirmation.

use bones_core::{Editor, WeightsTool};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{header, note};
use crate::ui::UiState;

/// Réglages et état de confirmation des outils de poids.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub clean_threshold: f32,
    pub smooth_iterations: usize,
    pub smooth_factor: f32,
    pub levels_offset: f32,
    pub levels_gain: f32,
    pub limit_total: usize,
    pub quantize_steps: usize,
    /// Dialogue de confirmation des poids automatiques ouvert.
    pub confirm_automatic: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            clean_threshold: 0.1,
            smooth_iterations: 1,
            smooth_factor: 0.5,
            levels_offset: 0.0,
            levels_gain: 1.0,
            limit_total: 4,
            quantize_steps: 4,
            confirm_automatic: false,
        }
    }
}

/// Affiche le menu ; retourne vrai pour refermer le popover (outil appliqué).
pub fn show(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, max_height: f32) -> bool {
    header(ui, "Weights", None);
    ui.add_space(4.0);
    // L'outil choisi est appliqué après la liste, pour ne pas emprunter
    // l'éditeur dans les fermetures imbriquées des lignes.
    let mut chosen: Option<WeightsTool> = None;
    let mut open_confirm = false;
    egui::ScrollArea::vertical()
        .max_height((max_height - 90.0).max(200.0))
        .show(ui, |ui| {
            let wt = &mut state.weights_tools;
            widgets::section_title(ui, "All vertices");
            widgets::group(ui, |g| {
                if g.value_row("Normalize All", "").clicked() {
                    chosen = Some(WeightsTool::NormalizeAll);
                }
                g.custom(52.0, |ui| {
                    label(ui, "Limit Total");
                    right(ui, |ui| {
                        let go = widgets::chip_button(ui, "Apply", false).clicked();
                        ui.add_space(6.0);
                        widgets::stepper(ui, &mut wt.limit_total, 1..=8);
                        if go {
                            chosen = Some(WeightsTool::LimitTotal {
                                max: wt.limit_total as u8,
                            });
                        }
                    });
                });
            });

            widgets::section_title(ui, "Active bone");
            widgets::group(ui, |g| {
                if g.value_row("Invert", "1 − w").clicked() {
                    chosen = Some(WeightsTool::Invert);
                }
                g.custom(52.0, |ui| {
                    label(ui, "Clean");
                    right(ui, |ui| {
                        let go = widgets::chip_button(ui, "Apply", false).clicked();
                        ui.add_space(6.0);
                        widgets::drag_chip(ui, &mut wt.clean_threshold, 0.0..=1.0, 0.005, 2, "", 72.0);
                        if go {
                            chosen = Some(WeightsTool::Clean {
                                threshold: wt.clean_threshold,
                            });
                        }
                    });
                });
                g.custom(52.0, |ui| {
                    label(ui, "Smooth");
                    right(ui, |ui| {
                        let go = widgets::chip_button(ui, "Apply", false).clicked();
                        ui.add_space(6.0);
                        widgets::stepper(ui, &mut wt.smooth_iterations, 1..=10);
                        if go {
                            chosen = Some(WeightsTool::Smooth {
                                iterations: wt.smooth_iterations as u32,
                                factor: wt.smooth_factor,
                            });
                        }
                    });
                });
                g.custom(52.0, |ui| {
                    label(ui, "Quantize");
                    right(ui, |ui| {
                        let go = widgets::chip_button(ui, "Apply", false).clicked();
                        ui.add_space(6.0);
                        widgets::stepper(ui, &mut wt.quantize_steps, 2..=10);
                        if go {
                            chosen = Some(WeightsTool::Quantize {
                                steps: wt.quantize_steps as u32,
                            });
                        }
                    });
                });
            });
            widgets::slider_row_with(ui, &mut wt.levels_offset, -1.0..=1.0, "Levels offset", |ui, v| {
                widgets::drag_chip(ui, v, -1.0..=1.0, 0.005, 2, "", 72.0)
            });
            widgets::slider_row_with(ui, &mut wt.levels_gain, 0.0..=4.0, "Levels gain", |ui, v| {
                widgets::drag_chip(ui, v, 0.0..=4.0, 0.01, 2, "×", 72.0)
            });
            if widgets::pill_button(ui, Some(Icon::Adjust), "Apply levels", false).clicked() {
                chosen = Some(WeightsTool::Levels {
                    offset: wt.levels_offset,
                    gain: wt.levels_gain,
                });
            }

            widgets::section_title(ui, "Symmetry");
            widgets::group(ui, |g| {
                if g.value_row("Mirror", ".l ↔ .r").clicked() {
                    chosen = Some(WeightsTool::Mirror);
                }
            });

            widgets::section_title(ui, "Rebind");
            open_confirm = widgets::group(ui, |g| {
                g.icon_row(
                    Icon::Weights,
                    "Automatic weights",
                    Some("Recompute from the skeleton"),
                    None,
                )
                .clicked()
            });
            note(ui, "Automatic weights replace everything you painted.");
        });
    if let Some(tool) = chosen {
        editor.apply_weights_tool(tool);
    }
    if open_confirm {
        state.weights_tools.confirm_automatic = true;
    }
    chosen.is_some() || open_confirm
}

/// Libellé de ligne personnalisée (17 pt).
fn label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(17.0).color(theme::TEXT_LIST));
}

/// Contenu aligné à droite d'une ligne personnalisée.
fn right<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add)
        .inner
}

/// Dialogue centré confirmant les poids automatiques ; retourne sa zone.
pub fn confirm_dialog(ctx: &egui::Context, editor: &mut Editor, state: &mut UiState) -> Option<egui::Rect> {
    if !state.weights_tools.confirm_automatic {
        return None;
    }
    let area = egui::Area::new(egui::Id::unique("automatic-confirm"))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -40.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            theme::panel_frame().show(ui, |ui| {
                ui.set_width(320.0);
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Automatic weights").font(theme::semibold(20.0)));
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("This overwrites every weight you painted.")
                            .size(14.0)
                            .color(theme::TEXT_DIM),
                    );
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        if widgets::pill_button(ui, None, "Cancel", false).clicked() {
                            state.weights_tools.confirm_automatic = false;
                        }
                        if widgets::pill_button(ui, Some(Icon::Weights), "Replace", true).clicked() {
                            editor.apply_weights_tool(WeightsTool::Automatic);
                            state.weights_tools.confirm_automatic = false;
                        }
                    });
                    ui.add_space(2.0);
                });
            })
        });
    Some(area.response.rect)
}
