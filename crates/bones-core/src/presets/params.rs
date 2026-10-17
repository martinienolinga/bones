//! Paramètres des générateurs procéduraux : valeurs nommées, bornées par
//! leur description ([`ParamSpec`]) pour que l'interface affiche des
//! curseurs et qu'un fichier reçu ne puisse pas imposer de valeur absurde.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Description d'un paramètre (curseur de l'interface).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamSpec {
    pub key: &'static str,
    /// Nom affiché (anglais).
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    /// Pas du curseur (1 pour un entier ou une bascule 0/1, 0 si continu).
    pub step: f32,
}

impl ParamSpec {
    pub const fn new(key: &'static str, label: &'static str, min: f32, max: f32, default: f32) -> Self {
        Self {
            key,
            label,
            min,
            max,
            default,
            step: 0.0,
        }
    }

    /// Bascule 0/1.
    pub const fn toggle(key: &'static str, label: &'static str, default: bool) -> Self {
        Self {
            key,
            label,
            min: 0.0,
            max: 1.0,
            default: if default { 1.0 } else { 0.0 },
            step: 1.0,
        }
    }

    /// Valeur ramenée dans les bornes (valeur par défaut si non finie).
    pub fn clamp(&self, v: f32) -> f32 {
        if !v.is_finite() {
            return self.default;
        }
        let v = v.clamp(self.min, self.max);
        if self.step > 0.0 {
            (v / self.step).round() * self.step
        } else {
            v
        }
    }
}

/// Valeurs choisies ; un paramètre absent prend sa valeur par défaut.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Params(pub BTreeMap<String, f32>);

impl Params {
    pub fn new() -> Self {
        Self::default()
    }

    /// Construit à partir de paires (clé, valeur).
    pub fn of(values: &[(&str, f32)]) -> Self {
        Self(values.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect())
    }

    pub fn set(&mut self, key: &str, value: f32) {
        self.0.insert(key.to_owned(), value);
    }

    /// Recouvre ces valeurs par celles de `other`.
    pub fn merged(&self, other: &Params) -> Params {
        let mut out = self.clone();
        out.0.extend(other.0.iter().map(|(k, v)| (k.clone(), *v)));
        out
    }

    /// Valeur d'un paramètre décrit dans `specs` (bornée).
    pub fn get(&self, specs: &[ParamSpec], key: &str) -> f32 {
        let Some(spec) = specs.iter().find(|s| s.key == key) else {
            debug_assert!(false, "paramètre inconnu : {key}");
            return 0.0;
        };
        self.0.get(key).map_or(spec.default, |v| spec.clamp(*v))
    }

    pub fn flag(&self, specs: &[ParamSpec], key: &str) -> bool {
        self.get(specs, key) >= 0.5
    }
}
