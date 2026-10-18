//! Feuille d'application d'un préréglage : aperçu en bandeau (le mannequin
//! en grand), réglages
//! ([`ApplyOptions`] pour une animation ; intensité / miroir / additif /
//! « as key » pour une expression), bouton « Apply » et compte rendu
//! ([`ApplyReport`] : os non reconnus).

use std::time::Duration;

use bones_core::Editor;
use bones_core::presets::{ApplyMode, ApplyOptions, shot_for};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::card::Look;
use super::library::Entry;
use super::{Kind, PresetsState};

/// Compte rendu d'une application, pour les deux genres (animation et
/// expression), déjà réduit à ce que l'interface affiche.
#[derive(Default)]
pub struct Report {
    mapped: usize,
    unmapped: Vec<String>,
    warnings: Vec<String>,
}

/// Réglages de la feuille d'application (persistants entre deux ouvertures).
pub struct ApplyForm {
    options: ApplyOptions,
    /// Expressions : force, miroir (dans `options`), additif, pose-clé.
    intensity: f32,
    additive: bool,
    as_key: bool,
}

impl Default for ApplyForm {
    fn default() -> Self {
        Self {
            options: ApplyOptions::default(),
            intensity: 1.0,
            additive: false,
            as_key: true,
        }
    }
}

pub fn show(
    ui: &mut egui::Ui,
    editor: &mut Editor,
    p: &mut PresetsState,
    entry: Entry,
    kind: Kind,
    body: f32,
) {
    let look = Look::of(kind, ui.input(|i| i.time) as f32);
    egui::ScrollArea::vertical()
        .id_salt("preset-apply")
        .auto_shrink([false, false])
        .max_height(body)
        .show(ui, |ui| {
            if let Some(delay) = banner(ui, p, entry, look) {
                ui.ctx().request_repaint_after(delay);
            }
            ui.add_space(10.0);
            let info = p.library.info(entry);
            if !info.description.trim().is_empty() {
                ui.label(
                    egui::RichText::new(info.description.clone())
                        .size(14.0)
                        .color(theme::TEXT_DIM),
                );
                ui.add_space(8.0);
            }
            meta(ui, info);
            ui.add_space(10.0);
            match kind {
                Kind::Animations => animation_options(ui, &mut p.apply, info.looping),
                Kind::Expressions => expression_options(ui, &mut p.apply),
            }
            ui.add_space(14.0);
            if widgets::pill_button(ui, Some(Icon::Check), "Apply", true).clicked() {
                run(editor, p, entry, kind);
            }
            if let Some(report) = &p.report {
                ui.add_space(12.0);
                report_note(ui, report);
            }
        });
}

/// Bandeau d'aperçu en tête de la feuille : le mannequin en grand (ou le
/// squelette d'un préréglage d'animal). Retourne le délai avant la
/// prochaine image voulue.
fn banner(ui: &mut egui::Ui, p: &mut PresetsState, entry: Entry, look: Look) -> Option<Duration> {
    let preset = p.library.preset(entry);
    let staged = shot_for(preset).is_some();
    let height = if staged { BANNER } else { 150.0 };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(14), theme::WIDGET);
    if !staged {
        p.previews.warm(std::slice::from_ref(&preset), 1);
    }
    let look = Look {
        margin: look.margin + 0.04,
        ..look
    };
    super::card::thumbnail(ui, p, entry, rect.size(), rect, look)
}

/// Hauteur du bandeau quand le mannequin joue le préréglage.
const BANNER: f32 = 300.0;

/// Métadonnées : images, fréquence, boucle.
fn meta(ui: &mut egui::Ui, info: &bones_core::presets::PresetInfo) {
    let mut parts = Vec::new();
    if info.frames > 0 {
        parts.push(format!("{} frames", info.frames));
        parts.push(format!("{:.0} fps", info.fps));
    }
    parts.push(if info.looping {
        "loops".to_owned()
    } else {
        "one-shot".to_owned()
    });
    if info.root_motion {
        parts.push("root motion".to_owned());
    }
    ui.label(
        egui::RichText::new(parts.join("  ·  "))
            .size(13.0)
            .color(theme::TEXT_DIM),
    );
}

fn animation_options(ui: &mut egui::Ui, form: &mut ApplyForm, looping: bool) {
    let o = &mut form.options;
    widgets::section_title(ui, "How to add it");
    let mut mode = match o.mode {
        ApplyMode::Replace => 0,
        ApplyMode::Insert => 1,
        ApplyMode::Blend => 2,
    };
    if widgets::segmented(ui, &["Replace", "Insert", "Blend"], &mut mode).changed() {
        o.mode = [ApplyMode::Replace, ApplyMode::Insert, ApplyMode::Blend][mode];
    }
    ui.add_space(10.0);
    widgets::section_title(ui, "Playback");
    value_slider(ui, &mut o.speed, 0.25..=3.0, "Speed");
    if looping {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Loops").size(17.0).color(theme::TEXT_LIST));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut loops = o.loops.max(1) as usize;
                if widgets::stepper(ui, &mut loops, 1..=16).changed() {
                    o.loops = loops as u32;
                }
            });
        });
    }
    ui.add_space(6.0);
    widgets::switch_row(ui, &mut o.mirror, "Mirror", "Swap left and right");
    widgets::switch_row(ui, &mut o.in_place, "In place", "Remove forward travel");
}

/// Réglages d'une expression : force, miroir, additif, écriture d'une clé.
fn expression_options(ui: &mut egui::Ui, form: &mut ApplyForm) {
    widgets::section_title(ui, "Expression");
    value_slider(ui, &mut form.intensity, 0.0..=1.5, "Intensity");
    ui.add_space(6.0);
    widgets::switch_row(ui, &mut form.options.mirror, "Mirror", "Swap left and right");
    widgets::switch_row(
        ui,
        &mut form.additive,
        "Additive",
        "Add on top of the current face pose",
    );
    widgets::switch_row(
        ui,
        &mut form.as_key,
        "Apply as key",
        "Set a keyframe at the current frame",
    );
}

/// Curseur dont la pastille montre la valeur réelle en multiplicateur
/// (« 1.00× ») plutôt que le pourcentage de la course.
fn value_slider(ui: &mut egui::Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>, label: &str) {
    widgets::slider_row_with(ui, value, range, label, |ui, v| {
        widgets::value_chip(ui, &format!("{:.2}×", *v))
    });
}

/// Applique le préréglage (animation ou expression) et mémorise le compte
/// rendu.
pub(super) fn run(editor: &mut Editor, p: &mut PresetsState, entry: Entry, kind: Kind) {
    let preset = p.library.preset(entry);
    let name = preset.info.name.clone();
    let outcome = match kind {
        Kind::Animations => editor.apply_preset(preset, &p.apply.options).map(|r| Report {
            mapped: r.mapped.len(),
            unmapped: r.unmapped,
            warnings: r.warnings,
        }),
        Kind::Expressions => {
            let f = &p.apply;
            editor
                .apply_expression(preset, f.intensity, f.options.mirror, f.additive, f.as_key)
                .map(|r| Report {
                    mapped: r.applied.len(),
                    unmapped: r.skipped,
                    warnings: Vec::new(),
                })
        }
    };
    match outcome {
        Ok(report) => {
            let n = report.unmapped.len();
            p.notify(if n == 0 {
                format!("Applied “{name}”")
            } else {
                format!("Applied “{name}” — {n} bones skipped")
            });
            p.report = Some(report);
        }
        Err(e) => p.notify(format!("Error: {e}")),
    }
}

/// Compte rendu : os posés, os ignorés et avertissements.
fn report_note(ui: &mut egui::Ui, report: &Report) {
    widgets::section_title(ui, "Result");
    widgets::group(ui, |g| {
        g.value_row("Bones posed", &report.mapped.to_string());
        if !report.unmapped.is_empty() {
            g.value_row("Skipped", &report.unmapped.len().to_string());
        }
    });
    for warning in &report.warnings {
        super::manage::hook_note(ui, warning);
    }
    if !report.unmapped.is_empty() {
        super::manage::hook_note(ui, &format!("Skipped: {}", report.unmapped.join(", ")));
    }
}
