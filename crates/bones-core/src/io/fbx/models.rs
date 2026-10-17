//! Hiérarchie des modèles FBX (nœuds de scène) et leurs transformations de
//! repos.

use std::collections::HashMap;

use glam::DMat4;

use super::document::{Document, Props};
use super::transform::{LocalParts, geometric};

pub(super) struct ModelInfo {
    pub name: String,
    pub subclass: String,
    pub parent: Option<i64>,
    pub parts: LocalParts,
    pub geometric: DMat4,
    pub rest_global: DMat4,
}

pub(super) struct Models {
    pub info: HashMap<i64, ModelInfo>,
    /// Modèles, parents avant enfants.
    pub order: Vec<i64>,
}

impl Models {
    pub fn read(doc: &Document) -> Self {
        let mut info: HashMap<i64, ModelInfo> = doc
            .of_class("Model")
            .map(|o| {
                let props = Props::of(&o.node);
                let parent = doc.parents_of(o.id, "Model").first().map(|p| p.id);
                (
                    o.id,
                    ModelInfo {
                        name: o.name.clone(),
                        subclass: o.subclass.clone(),
                        parent,
                        parts: LocalParts::new(&props),
                        geometric: geometric(&props),
                        rest_global: DMat4::IDENTITY,
                    },
                )
            })
            .collect();

        // Ordre topologique : racines (triées pour un résultat stable), puis enfants.
        let mut children: HashMap<Option<i64>, Vec<i64>> = HashMap::new();
        for (id, m) in &info {
            let parent = m.parent.filter(|p| info.contains_key(p));
            children.entry(parent).or_default().push(*id);
        }
        for list in children.values_mut() {
            list.sort_unstable();
        }
        let mut order = Vec::with_capacity(info.len());
        let mut stack: Vec<i64> = children.get(&None).cloned().unwrap_or_default();
        stack.reverse();
        while let Some(id) = stack.pop() {
            order.push(id);
            if let Some(c) = children.get(&Some(id)) {
                stack.extend(c.iter().rev());
            }
        }

        for &id in &order {
            let parent_global = info[&id]
                .parent
                .and_then(|p| info.get(&p))
                .map(|p| p.rest_global)
                .unwrap_or(DMat4::IDENTITY);
            let m = info.get_mut(&id).expect("known model");
            m.rest_global = parent_global * m.parts.rest();
        }
        Models { info, order }
    }

    /// Transformations globales avec des locales modifiées (animation).
    pub fn globals(&self, local: impl Fn(i64, &ModelInfo) -> DMat4) -> HashMap<i64, DMat4> {
        let mut out: HashMap<i64, DMat4> = HashMap::with_capacity(self.order.len());
        for &id in &self.order {
            let m = &self.info[&id];
            let parent = m
                .parent
                .and_then(|p| out.get(&p))
                .copied()
                .unwrap_or(DMat4::IDENTITY);
            out.insert(id, parent * local(id, m));
        }
        out
    }

    /// Ancêtre le plus proche appartenant à `set`.
    pub fn nearest_in(&self, id: i64, set: &HashMap<i64, usize>) -> Option<usize> {
        let mut cur = self.info.get(&id)?.parent;
        while let Some(p) = cur {
            if let Some(j) = set.get(&p) {
                return Some(*j);
            }
            cur = self.info.get(&p)?.parent;
        }
        None
    }
}
