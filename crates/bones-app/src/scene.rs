//! Construction de la scène à dessiner à partir de l'état de l'éditeur.
//! Les tableaux sont réutilisés d'une image à l'autre (aucune allocation en
//! régime établi).

use bones_core::{Editor, Handle, Mode, Viewport, side_of};
use bones_render::{BoneInstance, LineVertex, Scene, bone_matrix, handle_matrix};
use bones_ui::theme;
use glam::{Mat4, Vec3, Vec4};

use crate::clip::clip_segment;

#[derive(Default)]
pub struct SceneBuilder {
    joints: Vec<Mat4>,
    bones: Vec<BoneInstance>,
    lines: Vec<LineVertex>,
    overlay: Vec<LineVertex>,
    view_proj: Mat4,
    /// Unité de la grille (puissance de 10 adaptée à la taille du modèle),
    /// recalculée seulement quand le maillage change.
    grid_unit: (u64, f32),
}

fn color(c: egui::Color32) -> Vec4 {
    Vec4::from_array(theme::to_linear(c))
}

impl SceneBuilder {
    pub fn build(&mut self, editor: &mut Editor, viewport: &Viewport) -> Scene<'_> {
        self.joints.clear();
        self.bones.clear();
        self.lines.clear();
        self.overlay.clear();

        self.view_proj = editor.camera.view_proj(viewport);

        self.joints.extend(editor.skinning_matrices());
        if editor.view.grid {
            let revision = editor.rig.revisions().mesh;
            if self.grid_unit.0 != revision {
                self.grid_unit = (revision, grid_unit(&editor.rig.mesh));
            }
            self.grid(self.grid_unit.1);
        }
        if editor.view.bones {
            self.skeleton(editor);
        }
        self.handles(editor);
        if editor.mode == Mode::Pose {
            self.trajectory(editor);
            if editor.view.center_of_mass {
                self.center_of_mass(editor);
            }
        }

        let camera = &editor.camera;
        Scene {
            view_proj: camera.view_proj(viewport),
            eye: camera.eye(),
            viewport: [viewport.x, viewport.y, viewport.width, viewport.height],
            background: theme::BACKGROUND,
            joints: &self.joints,
            show_mesh: editor.view.mesh,
            textured: editor.view.textures,
            weight_bone: editor.selected.filter(|_| editor.view.weights).map(|b| b.0),
            lines: &self.lines,
            bones: &self.bones,
            overlay_lines: &self.overlay,
        }
    }

    /// Ajoute un segment, réduit à sa partie visible (voir [`clip_segment`]).
    fn line(&mut self, a: Vec3, b: Vec3, c: Vec4, overlay: bool) {
        let Some((a, b)) = clip_segment(&self.view_proj, a, b) else {
            return;
        };
        let target = if overlay {
            &mut self.overlay
        } else {
            &mut self.lines
        };
        target.push(LineVertex::new(a, c));
        target.push(LineVertex::new(b, c));
    }

    /// Grille au sol : pas d'un quart d'unité, lignes majeures à chaque unité
    /// (1 m pour un personnage, 100 pour un modèle en centimètres…).
    fn grid(&mut self, unit: f32) {
        let minor = Vec4::new(0.35, 0.35, 0.37, 0.25);
        let major = Vec4::new(0.45, 0.45, 0.48, 0.45);
        let extent = 5.0 * unit;
        for i in -20..=20 {
            let t = i as f32 * 0.25 * unit;
            let c = if i == 0 {
                continue;
            } else if i % 4 == 0 {
                major
            } else {
                minor
            };
            self.line(Vec3::new(t, 0.0, -extent), Vec3::new(t, 0.0, extent), c, false);
            self.line(Vec3::new(-extent, 0.0, t), Vec3::new(extent, 0.0, t), c, false);
        }
        self.line(
            Vec3::new(-extent, 0.0, 0.0),
            Vec3::new(extent, 0.0, 0.0),
            Vec4::new(0.8, 0.25, 0.25, 0.8),
            false,
        );
        self.line(
            Vec3::new(0.0, 0.0, -extent),
            Vec3::new(0.0, 0.0, extent),
            Vec4::new(0.25, 0.4, 0.85, 0.8),
            false,
        );
    }

    fn skeleton(&mut self, editor: &Editor) {
        let skeleton = &editor.rig.skeleton;
        let world = editor.display_world();
        for (i, bone) in skeleton.bones().iter().enumerate() {
            let id = bones_core::BoneId(i as u32);
            let c = if editor.selected == Some(id) {
                theme::BONE_SELECTED
            } else if editor.mode == Mode::Pose && editor.rig.pins.iter().any(|p| p.bone == id) {
                theme::BONE_PINNED
            } else {
                match side_of(&bone.name) {
                    Some(bones_core::Side::Left) => theme::BONE_LEFT,
                    Some(bones_core::Side::Right) => theme::BONE_RIGHT,
                    None => theme::BONE_CENTER,
                }
            };
            self.bones.push(BoneInstance::new(
                bone_matrix(world[i].to_matrix(), bone.length),
                color(c),
            ));
        }

        // Fantômes (onion skin) aux clés voisines, dessinés après les os réels.
        if editor.mode == Mode::Pose && editor.view.ghosts {
            let ghost = color(theme::GHOST);
            for pose in editor.ghosts() {
                for (bone, w) in skeleton.bones().iter().zip(&pose) {
                    self.bones
                        .push(BoneInstance::new(bone_matrix(w.to_matrix(), bone.length), ghost));
                }
            }
        }
    }

    fn handles(&mut self, editor: &Editor) {
        let size = editor.camera.distance * 0.022;
        let world = editor.display_world();
        for (handle, p) in editor.handles() {
            let (c, s) = match handle {
                Handle::IkTarget(_) => (theme::IK_HANDLE, size * 1.3),
                Handle::IkPole(i) => {
                    // Trait entre le coude/genou et son pôle.
                    if let Some(limb) = editor.rig.limbs.get(i) {
                        let joint = world[limb.lower.index()].translation;
                        self.line(
                            joint,
                            p,
                            color(theme::POLE_HANDLE) * Vec4::new(1.0, 1.0, 1.0, 0.6),
                            true,
                        );
                    }
                    (theme::POLE_HANDLE, size)
                }
                Handle::Marker(..) => (theme::MARKER, size * 1.2),
            };
            self.bones.push(BoneInstance::new(handle_matrix(p, s), color(c)));
        }
        // Repère des épingles : petite croix à la cible.
        if editor.mode == Mode::Pose {
            let c = color(theme::BONE_PINNED);
            for pin in editor.rig.pins.clone() {
                self.cross(pin.target, size, c);
            }
        }
    }

    fn trajectory(&mut self, editor: &mut Editor) {
        if !editor.view.trajectory {
            return;
        }
        let current = editor.current_frame() as usize;
        let base = color(theme::TRAJECTORY);
        let Some(points) = editor.trajectory().map(<[Vec3]>::to_vec) else {
            return;
        };
        for (i, w) in points.windows(2).enumerate() {
            // Plus opaque près de l'image courante.
            let d = (i as f32 - current as f32).abs();
            let alpha = (1.0 - d / 40.0).clamp(0.25, 1.0);
            self.line(w[0], w[1], base * Vec4::new(1.0, 1.0, 1.0, alpha), true);
        }
        if let Some(p) = points.get(current) {
            self.cross(*p, editor.camera.distance * 0.012, base);
        }
    }

    fn center_of_mass(&mut self, editor: &mut Editor) {
        let Some(com) = editor.center_of_mass() else {
            return;
        };
        let c = Vec4::new(1.0, 0.85, 0.2, 1.0);
        let s = editor.camera.distance * 0.02;
        self.cross(com, s, c);
        let ground = Vec3::new(com.x, 0.0, com.z);
        self.line(com, ground, c * Vec4::new(1.0, 1.0, 1.0, 0.5), true);
        self.cross(ground, s * 1.5, c);
    }

    fn cross(&mut self, p: Vec3, s: f32, c: Vec4) {
        self.line(p - Vec3::X * s, p + Vec3::X * s, c, true);
        self.line(p - Vec3::Y * s, p + Vec3::Y * s, c, true);
        self.line(p - Vec3::Z * s, p + Vec3::Z * s, c, true);
    }
}

/// Puissance de 10 la plus proche de la taille du modèle.
fn grid_unit(mesh: &bones_core::Mesh) -> f32 {
    let size = mesh
        .bounds()
        .map(|(lo, hi)| (hi - lo).max_element())
        .unwrap_or(1.0)
        .max(1e-3);
    10f32.powf(size.log10().round())
}
