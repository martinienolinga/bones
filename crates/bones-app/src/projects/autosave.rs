//! Enregistrement automatique : un projet modifié est enregistré une fois
//! l'édition au repos depuis [`DELAY`] (aucune nouvelle modification, aucun
//! doigt posé), jamais pendant un geste.

use std::time::{Duration, Instant};

/// Délai de repos avant l'enregistrement automatique.
pub const DELAY: Duration = Duration::from_secs(2);

/// Suivi du compteur de modifications de l'éditeur
/// (`Editor::edit_count`).
#[derive(Clone, Debug)]
pub struct Autosave {
    /// Compteur au dernier enregistrement (lancé).
    saved: u64,
    /// Dernière valeur observée du compteur.
    seen: u64,
    /// Instant où le compteur a changé pour la dernière fois.
    changed_at: Option<Instant>,
}

impl Autosave {
    /// Projet tout juste ouvert ou enregistré au compteur `edits`.
    pub fn new(edits: u64) -> Self {
        Self {
            saved: edits,
            seen: edits,
            changed_at: None,
        }
    }

    /// Des modifications n'ont pas encore été enregistrées.
    pub fn is_dirty(&self, edits: u64) -> bool {
        edits != self.saved
    }

    /// À appeler à chaque image : vrai quand il faut enregistrer maintenant.
    pub fn due(&mut self, edits: u64, pointer_down: bool, now: Instant) -> bool {
        if edits != self.seen {
            self.seen = edits;
            self.changed_at = Some(now);
        }
        if !self.is_dirty(edits) {
            self.changed_at = None;
            return false;
        }
        let since = *self.changed_at.get_or_insert(now);
        !pointer_down && now.duration_since(since) >= DELAY
    }

    /// Un enregistrement du compteur `edits` vient d'être lancé.
    pub fn saved(&mut self, edits: u64) {
        self.saved = edits;
        self.seen = edits;
        self.changed_at = None;
    }

    /// Prochain enregistrement prévu (pour réveiller la boucle d'événements).
    pub fn deadline(&self) -> Option<Instant> {
        self.changed_at.map(|t| t + DELAY)
    }
}
