//! Poids des objets déformés en tâche de fond : même file que le corps
//! (`Editor::bind_job`), résultat routé par le jeton de la clé.

use crate::editor::{BindJob, Editor};
use crate::scene::ObjectId;
use crate::{Skin, next_revision};

/// Demandes gardées au plus (une demande dont le résultat ne revient
/// jamais ne doit pas faire grossir la liste sans fin).
const MAX_PENDING: usize = 64;

impl Editor {
    /// Prochaine liaison d'objet à lancer (aucune pendant un trait de
    /// peinture : les poids du corps changent encore).
    pub(in crate::editor) fn object_bind_job(&mut self) -> Option<BindJob> {
        if self.paint.stroke.is_some() {
            return None;
        }
        let skip: Vec<_> = self.objects.binds.iter().map(|(_, id, s)| (*id, *s)).collect();
        let input = self.rig.object_bind_input(self.bind_method, &skip)?;
        let token = next_revision();
        if self.objects.binds.len() >= MAX_PENDING {
            self.objects.binds.remove(0);
        }
        self.objects.binds.push((token, input.id, input.stamp));
        Some(BindJob {
            mesh: input.mesh.clone(),
            skeleton: self.rig.skeleton.clone(),
            method: self.bind_method,
            key: (self.rig.revisions().skeleton, token),
            object: Some(input),
        })
    }

    /// Installe le résultat d'une liaison d'objet ; retourne les poids s'ils
    /// ne sont pas destinés à un objet (ils sont alors ceux du corps).
    pub(in crate::editor) fn finish_object_bind(&mut self, skin: Skin, key: (u64, u64)) -> Option<Skin> {
        let Some(i) = self.objects.binds.iter().position(|(t, ..)| *t == key.1) else {
            return Some(skin);
        };
        let (_, id, stamp) = self.objects.binds.remove(i);
        if self.rig.apply_object_skin(id, skin, stamp) {
            let name = self.rig.object(id).map(|o| o.name.clone()).unwrap_or_default();
            self.status = Some(format!("Weights ready: {name}"));
        }
        None
    }

    /// Force un nouveau calcul des poids automatiques d'un objet.
    pub fn rebind_object(&mut self, id: ObjectId) {
        self.objects.binds.retain(|(_, o, _)| *o != id);
        self.rig.invalidate_object_bind(id);
    }

    /// Un calcul de poids d'objet est en attente ou en cours.
    pub fn objects_binding(&self) -> bool {
        !self.objects.binds.is_empty() || !self.rig.stale_objects().is_empty()
    }
}
