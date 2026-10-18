//! Écrans de l'application et cycle de vie des projets.
//!
//! L'app démarre toujours sur la galerie (écran d'accueil, comme
//! Procreate) ; l'éditeur n'est atteint qu'en ouvrant ou en créant un
//! projet, et son bouton « Gallery » y ramène.
//!
//! - ouvrir : lecture en tâche de fond (fil des projets), puis la vignette
//!   grandit jusqu'au plein écran et l'éditeur apparaît dessous ;
//! - enregistrement automatique : 2 s après la dernière modification, doigt
//!   levé, avec une vignette fraîche (rendu GPU, lecture et encodage hors du
//!   fil principal) ; jamais deux enregistrements du même fichier à la
//!   fois ;
//! - retour à la galerie : enregistrement lancé, la vignette fraîche
//!   rétrécit jusqu'à sa carte dès qu'elle est prête ;
//! - passage en arrière-plan ou fermeture : enregistrement, puis attente de
//!   la fin des écritures.
//!
//! [`actions`] : demandes de la galerie ; [`events`] : résultats du fil des
//! projets ; [`screens`] : passages entre la galerie et l'éditeur.

mod actions;
mod events;
mod screens;

#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use winit::event_loop::EventLoopProxy;

use crate::UserEvent;
use crate::gallery::Gallery;
use crate::gallery::zoom::Zoom;
use crate::input::Gestures;
use crate::projects::{self, OpenProject, Task, Template, Waker, Worker};
use crate::running::Running;
use crate::thumbnail::{self, ThumbnailScene};

/// Attente maximale de la vignette fraîche avant d'afficher la galerie.
const RETURN_WAIT: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Gallery,
    Editor,
}

pub struct Session {
    pub screen: Screen,
    pub gallery: Gallery,
    pub worker: Worker,
    /// Projet de l'éditeur (gardé en mémoire après le retour à la galerie :
    /// le rouvrir est immédiat).
    pub current: Option<OpenProject>,
    pub zoom: Option<Zoom>,
    /// Retour à la galerie demandé à cet instant, en attente de la vignette.
    returning: Option<Instant>,
    /// Création ou import en cours (pastille de la galerie).
    pub busy: Option<String>,
    thumbnails: ThumbnailScene,
    /// Écran de l'image précédente (gestes remis à zéro au changement).
    last_screen: Screen,
}

impl Session {
    /// Galerie à l'écran, liste des projets demandée ; au premier lancement
    /// (pas de dossier des projets), un projet « Mannequin » est créé.
    pub fn new(docs: &std::path::Path, proxy: EventLoopProxy<UserEvent>) -> Self {
        let proxy = Mutex::new(proxy);
        let wake: Waker = Arc::new(move || {
            if let Ok(proxy) = proxy.lock() {
                let _ = proxy.send_event(UserEvent::JobDone);
            }
        });
        Self::start(docs, wake)
    }

    /// Démarrage, `wake` réveillant la boucle d'événements.
    fn start(docs: &std::path::Path, wake: Waker) -> Self {
        let dirs = projects::dirs(docs);
        let first_launch = !dirs.projects.exists();
        let worker = Worker::new(dirs, wake.clone());
        if first_launch {
            worker.push(create(Template::Mannequin, false));
        }
        worker.push(Task::List);
        let busy = first_launch.then(|| format!("Creating “{}”…", Template::Mannequin.name()));
        Self {
            screen: Screen::Gallery,
            gallery: Gallery::new(wake),
            worker,
            current: None,
            zoom: None,
            returning: None,
            busy,
            thumbnails: ThumbnailScene::default(),
            last_screen: Screen::Gallery,
        }
    }

    /// Prochain réveil nécessaire (enregistrement automatique, retour).
    pub fn deadline(&self) -> Option<Instant> {
        let autosave = self.current.as_ref().and_then(|p| p.autosave.deadline());
        let returning = self.returning.map(|t| t + RETURN_WAIT);
        autosave.into_iter().chain(returning).min()
    }
}

/// Création d'un projet d'après un modèle.
fn create(template: Template, open: bool) -> Task {
    Task::Create {
        name: template.name().to_owned(),
        build: Box::new(move || Ok(template.rig())),
        open,
    }
}

impl Running {
    /// Rend une vignette de l'éditeur courant pour un enregistrement.
    fn thumbnail(&mut self) -> Option<projects::ThumbnailSource> {
        thumbnail::render(
            &self.gpu,
            &mut self.renderer,
            &mut self.session.thumbnails,
            &self.editor,
        )
    }

    /// Enregistre le projet ouvert s'il le faut (vignette comprise).
    fn save_current(&mut self, force: bool) -> bool {
        let needed = self
            .session
            .current
            .as_ref()
            .is_some_and(|p| !p.renaming && (force || p.needs_save(&self.editor)));
        if !needed {
            return false;
        }
        let thumb = self.thumbnail();
        let Some(project) = &mut self.session.current else {
            return false;
        };
        let task = project.save_task(&self.editor, thumb);
        self.session.worker.push(Task::Save(Box::new(task)));
        true
    }

    /// Après chaque image : enregistrement automatique, fin des transitions.
    pub fn after_frame(&mut self) {
        let now = Instant::now();
        if self.session.screen == Screen::Editor && self.session.returning.is_none() {
            // Jamais d'instantané pendant un geste (son étape d'annulation
            // peut encore être abandonnée).
            let pointer_down = self.gestures.is_active()
                || self.editor.is_dragging()
                || self.egui_ctx.input(|i| i.pointer.any_down());
            let due = self
                .session
                .current
                .as_mut()
                .is_some_and(|p| p.autosave_due(&self.editor, pointer_down, now));
            if due {
                self.save_current(true);
            }
        }
        if self.session.returning.is_some_and(|t| now - t >= RETURN_WAIT) {
            self.show_gallery();
        }
        let time = self.egui_ctx.input(|i| i.time);
        if let Some(zoom) = &self.session.zoom {
            if zoom.opening && zoom.moved(time) && self.session.screen == Screen::Gallery {
                self.session.screen = Screen::Editor;
                self.session.gallery.zooming = None;
                self.session.gallery.opening = None;
            }
            if zoom.finished(time) {
                self.session.zoom = None;
                self.session.gallery.zooming = None;
            }
        }
        if self.session.screen != self.session.last_screen {
            // Un toucher commencé sur l'autre écran n'appartient pas à
            // celui-ci.
            self.session.last_screen = self.session.screen;
            self.gestures = Gestures::default();
        }
    }

    /// Alerte mémoire d'iOS : sur la galerie, le projet quitté (déjà
    /// enregistré) n'est plus gardé en mémoire, ni ses tampons GPU.
    pub fn release_memory(&mut self) {
        self.showcase.release(&mut self.egui_renderer);
        let saved = self
            .session
            .current
            .as_ref()
            .is_none_or(|p| !p.needs_save(&self.editor));
        if self.session.screen != Screen::Gallery || self.session.returning.is_some() || !saved {
            return;
        }
        self.session.current = None;
        self.editor = bones_core::Editor::new(bones_core::Rig::new("", bones_core::Mesh::default()));
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        let revisions = self.editor.rig.revisions();
        self.renderer
            .set_mesh(device, queue, &self.editor.rig.mesh, revisions.mesh);
    }

    /// Relit la liste des projets (retour au premier plan).
    pub fn refresh_projects(&mut self) {
        self.session.worker.push(Task::List);
        self.window.request_redraw();
    }

    /// Passage en arrière-plan ou fermeture : enregistre le projet ouvert
    /// (le GPU est encore utilisable à ce moment-là sur iPad) et attend la
    /// fin de toutes les écritures.
    pub fn save_and_wait(&mut self) {
        self.persist_settings();
        if self.editor.is_dragging() {
            // Geste interrompu par le système : il est annulé.
            self.editor.pointer_cancel();
        }
        self.gestures = Gestures::default();
        // Aussi depuis la galerie : une retouche faite juste avant le retour
        // (pendant l'attente de la vignette) reste à enregistrer.
        self.save_current(false);
        self.session.worker.flush();
        self.poll_projects();
    }
}
