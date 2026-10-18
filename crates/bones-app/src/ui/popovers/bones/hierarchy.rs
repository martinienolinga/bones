//! Sections « Parent » (choix du nouveau parent de l'os dans une liste à
//! cocher) et « Limbs » (bascules IK / FK des membres, comme Auto-Rig Pro).
//! L'arbre du squelette vit désormais dans l'outliner unifié ([`super::scene`]).

use bones_core::editor::BoneInfo;
use bones_core::{Editor, LimbMode};
use bones_ui::{theme, widgets};

use crate::ui::popovers::note;

pub fn parent(ui: &mut egui::Ui, editor: &mut Editor, info: &BoneInfo) {
    widgets::section_title(ui, "New parent");
    let candidates = editor.parent_candidates(info.id);
    let skeleton = &editor.rig.skeleton;
    let picked = widgets::group(ui, |g| {
        let mut picked = None;
        if g.check_row("None (root)", info.parent.is_none()).clicked() {
            picked = Some(None);
        }
        for c in candidates {
            if g.check_row(&skeleton.bone(c).name, info.parent == Some(c))
                .clicked()
            {
                picked = Some(Some(c));
            }
        }
        picked
    });
    if let Some(parent) = picked.filter(|p| *p != info.parent) {
        editor.set_bone_parent(info.id, parent, info.connected);
    }
    note(
        ui,
        "The bone and its children are kept. With “Connected” on, its head \
         jumps to the new parent's tail. Tip: the Parent tool does this in the view.",
    );
}

pub fn limbs(ui: &mut egui::Ui, editor: &mut Editor) {
    if editor.rig.limbs.is_empty() {
        note(ui, "This rig has no IK limbs (Auto-Rig creates them).");
        return;
    }
    widgets::section_title(ui, "IK / FK");
    let mut toggled = None;
    widgets::group(ui, |g| {
        for (i, limb) in editor.rig.limbs.iter().enumerate() {
            let mut index = usize::from(limb.mode == LimbMode::Fk);
            g.custom(52.0, |ui| {
                ui.label(egui::RichText::new(&limb.name).size(17.0).color(theme::TEXT_LIST));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::segmented(ui, &["IK", "FK"], &mut index).changed() {
                        toggled = Some(i);
                    }
                });
            });
        }
    });
    if let Some(i) = toggled {
        editor.toggle_limb_mode(i);
    }
}
