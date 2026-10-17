//! Vignettes des projets : rendu hors écran 4:3 de la vue courante
//! (caméra du projet), modèle seul — ni grille, ni poignées ; le squelette
//! n'est dessiné que s'il n'y a pas de maillage. La lecture des pixels et
//! l'encodage JPEG se font sur le fil des projets.

use bones_core::{Editor, Side, Viewport, side_of};
use bones_render::{BoneInstance, Renderer, Scene, bone_matrix};
use bones_ui::theme;
use glam::{Mat4, Vec4};

use crate::gpu::Gpu;
use crate::projects::{Rgba, ThumbnailSource};

/// Taille des vignettes, en pixels.
pub const SIZE: [u32; 2] = [640, 480];
/// Recadrage : la vignette montre le centre de la vue, agrandi d'autant
/// (le modèle remplit mieux la carte). Même caméra, champ plus étroit : la
/// transition vers l'éditeur reste exacte (voir `gallery::zoom`).
pub const CROP: f32 = 1.2;

/// Fond des vignettes : celui de la vue 3D, pour que la transition entre la
/// galerie et l'éditeur soit continue.
const BACKGROUND: [f64; 3] = theme::BACKGROUND;

/// Fond de la vue 3D en couleur d'interface (transitions de la galerie).
pub fn backdrop() -> egui::Color32 {
    let [r, g, b] = BACKGROUND.map(|c| (c * 255.0).round() as u8);
    egui::Color32::from_rgb(r, g, b)
}

/// Tableaux réutilisés d'une vignette à l'autre.
#[derive(Default)]
pub struct ThumbnailScene {
    joints: Vec<Mat4>,
    bones: Vec<BoneInstance>,
}

impl ThumbnailScene {
    fn build(&mut self, editor: &Editor) -> Scene<'_> {
        self.joints.clear();
        self.bones.clear();
        self.joints.extend(editor.skinning_matrices());
        let has_mesh = editor.rig.mesh.triangle_count() > 0;
        if !has_mesh {
            let world = editor.display_world();
            for (i, bone) in editor.rig.skeleton.bones().iter().enumerate() {
                let color = match side_of(&bone.name) {
                    Some(Side::Left) => theme::BONE_LEFT,
                    Some(Side::Right) => theme::BONE_RIGHT,
                    None => theme::BONE_CENTER,
                };
                let matrix = bone_matrix(world[i].to_matrix(), bone.length);
                self.bones.push(BoneInstance::new(
                    matrix,
                    Vec4::from_array(theme::to_linear(color)),
                ));
            }
        }
        let viewport = Viewport::new(SIZE[0] as f32, SIZE[1] as f32);
        let mut camera = editor.camera;
        camera.fov_y = 2.0 * ((camera.fov_y * 0.5).tan() / CROP).atan();
        Scene {
            view_proj: camera.view_proj(&viewport),
            eye: camera.eye(),
            viewport: [0.0, 0.0, viewport.width, viewport.height],
            background: BACKGROUND,
            joints: &self.joints,
            show_mesh: has_mesh,
            textured: editor.view.textures,
            weights: None,
            lines: &[],
            bones: &self.bones,
            overlay_lines: &[],
        }
    }
}

/// Rend la vignette de `editor` (soumise tout de suite au GPU) et retourne
/// de quoi la relire hors du fil principal. `None` si la surface n'est pas
/// lisible.
pub fn render(
    gpu: &Gpu,
    renderer: &mut Renderer,
    scene: &mut ThumbnailScene,
    editor: &Editor,
) -> Option<ThumbnailSource> {
    let (device, queue) = (&gpu.device, &gpu.queue);
    let revisions = editor.rig.revisions();
    renderer.set_mesh(device, queue, &editor.rig.mesh, revisions.mesh);
    renderer.set_skin(device, queue, &editor.rig.skin, revisions.skin);
    let readback = renderer.snapshot(device, queue, &scene.build(editor), SIZE)?;
    Some(Box::new(move || {
        let size = readback.size();
        readback.wait().map(|pixels| Rgba { size, pixels })
    }))
}
