//! Vue indexée d'un arbre FBX 7.x : objets par identifiant, connexions et
//! propriétés (`Properties70`).

use std::collections::HashMap;

use fbxcel::low::v7400::AttributeValue;
use fbxcel::tree::v7400::{NodeHandle, Tree};
use glam::DVec3;

/// Objet FBX (`Model`, `Geometry`, `Material`…).
pub(super) struct Object<'a> {
    pub id: i64,
    pub name: String,
    /// Nom du nœud : « Model », « Geometry »…
    pub class: &'a str,
    /// Sous-classe : « Mesh », « LimbNode », « Skin », « Cluster »…
    pub subclass: String,
    pub node: NodeHandle<'a>,
}

#[derive(Clone)]
pub(super) struct Link {
    pub id: i64,
    /// Propriété ciblée pour les connexions « OP ».
    pub property: Option<String>,
}

pub(super) struct Document<'a> {
    pub objects: HashMap<i64, Object<'a>>,
    /// Pour chaque parent, ses enfants dans l'ordre des connexions.
    pub children: HashMap<i64, Vec<Link>>,
    /// Pour chaque enfant, ses parents.
    pub parents: HashMap<i64, Vec<Link>>,
    pub global_settings: Option<NodeHandle<'a>>,
}

/// Nom lisible : « Nom\0\u{1}Classe » → « Nom » (et préfixe « Classe:: » retiré).
fn clean_name(raw: &str) -> String {
    let name = raw.split('\0').next().unwrap_or(raw);
    name.rsplit("::").next().unwrap_or(name).to_owned()
}

impl<'a> Document<'a> {
    pub fn new(tree: &'a Tree) -> Self {
        let root = tree.root();
        let mut objects = HashMap::new();
        if let Some(objs) = root.first_child_by_name("Objects") {
            for node in objs.children() {
                let attrs = node.attributes();
                let Some(id) = attrs.first().and_then(as_i64) else {
                    continue;
                };
                let name = attrs
                    .get(1)
                    .and_then(|a| a.get_string())
                    .map(clean_name)
                    .unwrap_or_default();
                let subclass = attrs
                    .get(2)
                    .and_then(|a| a.get_string())
                    .unwrap_or_default()
                    .to_owned();
                objects.insert(
                    id,
                    Object {
                        id,
                        name,
                        class: node.name(),
                        subclass,
                        node,
                    },
                );
            }
        }
        let mut children: HashMap<i64, Vec<Link>> = HashMap::new();
        let mut parents: HashMap<i64, Vec<Link>> = HashMap::new();
        if let Some(conns) = root.first_child_by_name("Connections") {
            for c in conns.children_by_name("C") {
                let a = c.attributes();
                let (Some(child), Some(parent)) = (a.get(1).and_then(as_i64), a.get(2).and_then(as_i64))
                else {
                    continue;
                };
                let property = a.get(3).and_then(|p| p.get_string()).map(str::to_owned);
                children.entry(parent).or_default().push(Link {
                    id: child,
                    property: property.clone(),
                });
                parents
                    .entry(child)
                    .or_default()
                    .push(Link { id: parent, property });
            }
        }
        Document {
            objects,
            children,
            parents,
            global_settings: root.first_child_by_name("GlobalSettings"),
        }
    }

    /// Enfants d'un objet appartenant à la classe `class`.
    pub fn children_of(&self, id: i64, class: &str) -> Vec<&Object<'a>> {
        self.links(&self.children, id, class)
    }

    /// Parents d'un objet appartenant à la classe `class`.
    pub fn parents_of(&self, id: i64, class: &str) -> Vec<&Object<'a>> {
        self.links(&self.parents, id, class)
    }

    fn links(&self, map: &HashMap<i64, Vec<Link>>, id: i64, class: &str) -> Vec<&Object<'a>> {
        map.get(&id)
            .into_iter()
            .flatten()
            .filter_map(|l| self.objects.get(&l.id))
            .filter(|o| o.class == class)
            .collect()
    }

    /// Enfants reliés par une propriété précise (connexion « OP »).
    pub fn children_with_property(&self, id: i64, property: &str) -> Vec<&Object<'a>> {
        self.children
            .get(&id)
            .into_iter()
            .flatten()
            .filter(|l| l.property.as_deref() == Some(property))
            .filter_map(|l| self.objects.get(&l.id))
            .collect()
    }

    pub fn of_class(&self, class: &str) -> impl Iterator<Item = &Object<'a>> {
        self.objects.values().filter(move |o| o.class == class)
    }
}

pub(super) fn as_i64(a: &AttributeValue) -> Option<i64> {
    match a {
        AttributeValue::I64(v) => Some(*v),
        AttributeValue::I32(v) => Some(*v as i64),
        AttributeValue::I16(v) => Some(*v as i64),
        _ => None,
    }
}

pub(super) fn as_f64(a: &AttributeValue) -> Option<f64> {
    match a {
        AttributeValue::F64(v) => Some(*v),
        AttributeValue::F32(v) => Some(*v as f64),
        AttributeValue::I32(v) => Some(*v as f64),
        AttributeValue::I64(v) => Some(*v as f64),
        AttributeValue::I16(v) => Some(*v as f64),
        AttributeValue::Bool(v) => Some(*v as i32 as f64),
        _ => None,
    }
}

/// Propriétés `Properties70` d'un nœud : nom → valeurs.
pub(super) struct Props<'a>(HashMap<&'a str, &'a [AttributeValue]>);

impl<'a> Props<'a> {
    pub fn of(node: &NodeHandle<'a>) -> Self {
        let mut map = HashMap::new();
        if let Some(p70) = node.first_child_by_name("Properties70") {
            for p in p70.children_by_name("P") {
                let a = p.attributes();
                if let Some(name) = a.first().and_then(|n| n.get_string()) {
                    map.insert(name, a.get(4..).unwrap_or(&[]));
                }
            }
        }
        Props(map)
    }

    pub fn f64(&self, name: &str) -> Option<f64> {
        self.0.get(name)?.first().and_then(as_f64)
    }

    pub fn i64(&self, name: &str) -> Option<i64> {
        self.0
            .get(name)?
            .first()
            .and_then(|a| as_i64(a).or_else(|| as_f64(a).map(|f| f as i64)))
    }

    pub fn vec3(&self, name: &str) -> Option<DVec3> {
        let v = self.0.get(name)?;
        Some(DVec3::new(
            v.first().and_then(as_f64)?,
            v.get(1).and_then(as_f64)?,
            v.get(2).and_then(as_f64)?,
        ))
    }
}

/// Premier attribut d'un enfant nommé (tableau, nombre, chaîne…).
pub(super) fn child_attr<'a>(node: &NodeHandle<'a>, name: &str) -> Option<&'a AttributeValue> {
    node.first_child_by_name(name)?.attributes().first()
}
