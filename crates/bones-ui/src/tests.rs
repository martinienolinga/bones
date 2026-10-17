//! Tests de rendu : icônes (SVG valides, planche contact) et widgets, dans
//! un contexte egui sans GPU.

use egui::{Color32, RawInput, Rect, pos2, vec2};

use crate::{icons, theme, widgets};

fn run(mut ui_fn: impl FnMut(&mut egui::Ui)) -> egui::FullOutput {
    let ctx = egui::Context::default();
    crate::install(&ctx);
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1024.0, 768.0))),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| ui_fn(ui));
    // Pas de GPU ici : les textures (polices, icônes) ne sont pas envoyées.
    output.textures_delta.clear();
    output
}

#[test]
fn every_icon_is_valid_and_visible() {
    assert_eq!(icons::all().len(), 48);
    for &icon in icons::all() {
        let pixels = icons::rasterize(icon, 40);
        let opaque = pixels.chunks_exact(4).filter(|p| p[3] > 128).count();
        assert!(opaque > 60, "{icon:?} est presque vide ({opaque} pixels)");
    }
}

/// Planche contact des icônes (64 et 40 px) écrite en PPM si la variable
/// `BONES_ICON_SHEET` donne un dossier : sert à la revue visuelle.
#[test]
fn icon_contact_sheet() {
    let Some(dir) = std::env::var_os("BONES_ICON_SHEET") else {
        return;
    };
    let (tile, cols) = (96usize, 9usize);
    let rows = icons::all().len().div_ceil(cols);
    let (w, h) = (cols * tile, rows * tile * 2);
    let mut img = vec![[0x1c_u8, 0x1c, 0x1e]; w * h];
    for (i, &icon) in icons::all().iter().enumerate() {
        for (band, px) in [(0usize, 64usize), (1, 40)] {
            let pixels = icons::rasterize(icon, px as u32);
            let x0 = (i % cols) * tile + (tile - px) / 2;
            let y0 = band * rows * tile + (i / cols) * tile + (tile - px) / 2;
            for y in 0..px {
                for x in 0..px {
                    let s = &pixels[(y * px + x) * 4..][..4];
                    let d = &mut img[(y0 + y) * w + x0 + x];
                    for c in 0..3 {
                        d[c] = (s[c] as u32 + d[c] as u32 * (255 - s[3] as u32) / 255).min(255) as u8;
                    }
                }
            }
        }
    }
    let mut ppm = format!("P6 {w} {h} 255\n").into_bytes();
    ppm.extend(img.iter().flatten());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(std::path::Path::new(&dir).join("sheet.ppm"), ppm).unwrap();
}

#[test]
fn widgets_render() {
    let mut value = 3.0;
    let mut frame = 12.0;
    let mut mode = 1;
    let mut on = true;
    let output = run(|ui| {
        widgets::tool_button(ui, icons::Icon::Bone, true, "Os");
        widgets::tool_cell(ui, icons::Icon::Ik, "IK", false);
        widgets::text_button(ui, "Galerie", false);
        widgets::separator_dot(ui);
        widgets::segmented(ui, &["Pose", "Édition", "Auto-Rig"], &mut mode);
        widgets::vertical_slider(ui, &mut value, 1.0..=6.0, "Chaîne IK");
        widgets::vertical_slider_with_icon(ui, &mut value, 1.0..=6.0, icons::Icon::Chain, "Chaîne IK");
        widgets::key_strip(ui, &mut frame, 120, &[0, 12, 60], 64.0);
        widgets::switch_row(ui, &mut on, "Poser", "IK et rotation");
        widgets::pill_button(ui, Some(icons::Icon::Save), "Enregistrer", true);
        widgets::list_row(ui, icons::Icon::Export, "Exporter", Some("glTF"), Some("GLB"));
        widgets::section_title(ui, "Section");
        let mut r = 22.0;
        widgets::slider_row(ui, &mut r, 10.0..=48.0, "Rayon", "pt");
        icons::paint(
            ui.painter(),
            Rect::from_min_size(pos2(10.0, 10.0), vec2(24.0, 24.0)),
            icons::Icon::Pin,
            Color32::WHITE,
        );
    });
    assert!(!output.shapes.is_empty());
    // Sans interaction, rien ne change.
    assert_eq!((value, frame, mode, on), (3.0, 12.0, 1, true));
    let _ = theme::ACCENT;
}
