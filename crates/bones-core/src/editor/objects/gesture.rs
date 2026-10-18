//! Gestes du mode Objet : sélection au doigt (lancer de rayon sur les
//! maillages tels qu'affichés), déplacement dans le plan de l'écran, et
//! simple toucher.

use glam::{Mat4, Vec2, Vec3};
use rayon::prelude::*;

use crate::editor::Editor;
use crate::editor::drag::Drag;
use crate::scene::pick::{ray_mesh, ray_model};
use crate::scene::{ObjectId, ObjectRef};
use crate::{Skin, Transform, Viewport};

/// Positions déformées par skinning linéaire (comme le shader et
/// `Skin::deform_into`), après le placement `model`.
fn skinned_positions(positions: &[Vec3], model: Mat4, skin: &Skin, matrices: &[Mat4]) -> Vec<Vec3> {
    positions
        .par_iter()
        .enumerate()
        .map(|(i, p)| {
            let p = model.transform_point3(*p);
            let vw = skin.weights.get(i).copied().unwrap_or_default();
            let (mut sum, mut total) = (Vec3::ZERO, 0.0);
            for (bone, w) in vw.iter() {
                if let Some(m) = matrices.get(bone.index()) {
                    sum += w * m.transform_point3(p);
                    total += w;
                }
            }
            if total < 1e-6 { p } else { sum / total }
        })
        .collect()
}

impl Editor {
    /// Élément visible et non verrouillé sous le doigt (le plus proche de
    /// la caméra), corps compris.
    pub fn pick_object(&self, pos: Vec2, vp: &Viewport) -> Option<ObjectRef> {
        let ray = self.camera.ray(pos, vp);
        let matrices = self.skinning_matrices();
        let mut best: Option<(f32, ObjectRef)> = None;
        let mut consider = |t: Option<f32>, item: ObjectRef| {
            if let Some(t) = t
                && best.is_none_or(|(b, _)| t < b)
            {
                best = Some((t, item));
            }
        };
        for draw in self.object_draws() {
            if self.rig.object(draw.id).is_none_or(|o| o.locked) {
                continue;
            }
            let mesh = &draw.mesh;
            let t = match &draw.skin {
                None => ray_model(&ray, draw.model, &mesh.positions, &mesh.indices),
                Some(s) => {
                    let placed = skinned_positions(&mesh.positions, draw.model, &s.skin, &matrices);
                    ray_mesh(ray.origin, ray.dir, &placed, &mesh.indices)
                }
            };
            consider(t, ObjectRef::Extra(draw.id));
        }
        let body = &self.rig.body;
        if body.visible && !body.locked && self.view.mesh && self.rig.mesh.triangle_count() > 0 {
            let mesh = &self.rig.mesh;
            let t = if self.rig.skin.weights.len() == mesh.vertex_count() {
                let placed = skinned_positions(&mesh.positions, Mat4::IDENTITY, &self.rig.skin, &matrices);
                ray_mesh(ray.origin, ray.dir, &placed, &mesh.indices)
            } else {
                ray_mesh(ray.origin, ray.dir, &mesh.positions, &mesh.indices)
            };
            consider(t, ObjectRef::Body);
        }
        best.map(|(_, item)| item)
    }

    /// Début de geste en mode Objet : un objet touché est sélectionné et
    /// suit le doigt ; sinon (corps ou vide) le geste revient à la caméra et
    /// un simple toucher sélectionnera ce qui se trouve dessous.
    pub(in crate::editor) fn object_down(&mut self, pos: Vec2, vp: &Viewport, _radius: f32) -> bool {
        self.objects.last_down = Some((pos, *vp));
        let Some(ObjectRef::Extra(id)) = self.pick_object(pos, vp) else {
            return false;
        };
        let Some(start) = self.object_display_world(id) else {
            return false;
        };
        self.selected_object = Some(ObjectRef::Extra(id));
        self.checkpoint();
        let grab = self.plane_point(pos, vp, start.translation);
        self.drag = Some(Drag::Object { id, start, grab });
        true
    }

    /// Objet glissé : translation dans le plan de l'écran passant par lui.
    pub(in crate::editor) fn object_drag(
        &mut self,
        id: ObjectId,
        start: &Transform,
        grab: Vec3,
        pos: Vec2,
        vp: &Viewport,
    ) {
        let mut world = *start;
        world.translation += self.plane_point(pos, vp, start.translation) - grab;
        self.set_object_world(id, &world);
    }

    /// Place un objet en monde (os tels qu'affichés) en recalculant sa
    /// transformation relative au parent.
    pub(super) fn set_object_world(&mut self, id: ObjectId, world: &Transform) {
        let Some(parent) = self.rig.object(id).map(|o| o.parent()) else {
            return;
        };
        let local = self
            .rig
            .parent_world(parent, &self.display_world())
            .inverse()
            .mul(world);
        let _ = self.rig.set_object_local(id, local);
    }

    /// Simple toucher sans objet saisi : sélectionne ce qui est dessous
    /// (le corps) ou désélectionne.
    pub(in crate::editor) fn object_tap(&mut self) {
        let under = self.objects.last_down.take();
        self.selected_object = under.and_then(|(p, vp)| self.pick_object(p, &vp));
    }
}
