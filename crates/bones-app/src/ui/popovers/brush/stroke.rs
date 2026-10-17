//! Section « Stroke » : réglages du trait (espacement des touches,
//! accumulation à l'aérographe, faces avant seulement) et effet de la
//! pression du crayon, comme les options de trait de Blender.

use bones_core::Editor;
use bones_ui::widgets;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor) {
    let brush = &mut editor.weight_paint.brush;
    widgets::section_title(ui, "Strength");
    widgets::slider_row_with(ui, &mut brush.strength, 0.0..=1.0, "Strength", |ui, v| {
        widgets::drag_chip(ui, v, 0.0..=1.0, 0.01, 2, "", 72.0)
    });
    widgets::note(
        ui,
        "Apple Pencil pressure drives strength; this value is the full force for finger and mouse.",
    );

    widgets::section_title(ui, "Spacing");
    widgets::slider_row(ui, &mut brush.spacing, 0.02..=1.0, "Dab spacing", "");
    widgets::note(
        ui,
        "Gap between dabs along a stroke, as a fraction of the brush size.",
    );

    widgets::section_title(ui, "Behaviour");
    widgets::group(ui, |g| {
        g.switch_row(
            &mut brush.accumulate,
            "Airbrush",
            Some("Dabs build up as you paint over"),
        );
        g.switch_row(
            &mut brush.front_faces_only,
            "Front faces only",
            Some("Skip faces turned away from the camera"),
        );
    });

    widgets::section_title(ui, "Apple Pencil pressure");
    widgets::group(ui, |g| {
        g.switch_row(&mut brush.pressure_strength, "Pressure → strength", None);
        g.switch_row(&mut brush.pressure_radius, "Pressure → size", None);
    });
}
