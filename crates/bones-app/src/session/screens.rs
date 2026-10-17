//! Passages entre la galerie et l'éditeur, avec les transitions de
//! `gallery::zoom`.

use std::time::Instant;

use crate::gallery::layout::{BAR_H, GRID_TOP, Grid};
use crate::gallery::zoom::{self, Zoom};
use crate::projects::Task;
use crate::running::Running;
use crate::thumbnail;

use super::Screen;

impl Running {
    /// Bouton « Gallery » de l'éditeur : enregistrement lancé, galerie
    /// affichée dès que la vignette fraîche est prête.
    pub fn leave_editor(&mut self) {
        self.editor.playing = false;
        self.ui.reset_for_project();
        if self.save_current(false) {
            self.session.returning = Some(Instant::now());
            if let Some(project) = &self.session.current {
                self.session.gallery.library.bring_to_front(&project.path);
            }
        } else {
            self.show_gallery();
        }
    }

    /// Affiche la galerie ; la vignette du projet quitté rétrécit jusqu'à
    /// sa carte.
    pub(super) fn show_gallery(&mut self) {
        self.session.returning = None;
        self.session.screen = Screen::Gallery;
        let gallery = &mut self.session.gallery;
        gallery.reset();
        gallery.scroll_to_top = true;
        self.session.worker.push(Task::List);
        let Some(project) = &self.session.current else {
            return;
        };
        let (Some(index), Some(texture)) = (
            gallery.library.index(&project.path),
            gallery.thumbs.texture(&project.path).cloned(),
        ) else {
            return;
        };
        let screen = self.egui_ctx.content_rect();
        let card = Grid::new(screen.width()).thumb_rect(egui::pos2(screen.left(), BAR_H + GRID_TOP), index);
        if card.bottom() < screen.bottom() {
            let view = zoom::in_view(screen, &texture, thumbnail::CROP);
            self.session.zoom = Some(Zoom::close(texture, view, card, thumbnail::backdrop()));
            gallery.zooming = Some(project.path.clone());
        }
    }

    /// Passe à l'éditeur pour le projet courant, en agrandissant sa vignette
    /// depuis sa carte quand c'est possible ; `preview` : vignette en pleine
    /// résolution (sinon celle de la carte).
    pub(super) fn enter_editor(&mut self, preview: Option<egui::TextureHandle>) {
        self.ui.reset_for_project();
        let gallery = &mut self.session.gallery;
        let path = self.session.current.as_ref().map(|p| p.path.clone());
        let card = path.as_ref().and_then(|p| gallery.thumb_rect(p));
        let texture = preview.or_else(|| path.as_ref().and_then(|p| gallery.thumbs.texture(p).cloned()));
        match (path, card, texture) {
            (Some(path), Some(card), Some(texture)) => {
                let screen = self.egui_ctx.content_rect();
                let view = zoom::in_view(screen, &texture, thumbnail::CROP);
                self.session.zoom = Some(Zoom::open(texture, card, view, thumbnail::backdrop()));
                gallery.opening = None;
                gallery.zooming = Some(path);
            }
            _ => {
                gallery.opening = None;
                self.session.screen = Screen::Editor;
            }
        }
    }
}
