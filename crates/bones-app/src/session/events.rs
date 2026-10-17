//! Résultats du fil des projets : vignettes fraîches, enregistrements,
//! ouvertures, créations, opérations sur les fichiers, listes.

use crate::files;
use crate::projects::{Event, OpenProject, Task};
use crate::running::Running;

use super::Screen;

impl Running {
    pub fn poll_projects(&mut self) {
        for event in self.session.worker.poll() {
            self.on_project_event(event);
        }
    }

    fn on_project_event(&mut self, event: Event) {
        let session = &mut self.session;
        let gallery = &mut session.gallery;
        match event {
            Event::Thumbnail { path, image } => {
                gallery.thumbs.install(&self.egui_ctx, &path, image);
                let leaving = session.current.as_ref().is_some_and(|p| p.path == path);
                if session.returning.is_some() && leaving {
                    self.show_gallery();
                }
            }
            Event::Saved { path, result, .. } => match result {
                Ok((info, modified)) => {
                    gallery.library.saved(&path, info, modified);
                    gallery.thumbs.confirm(&path, modified);
                }
                Err(e) => self
                    .ui
                    .notify(format!("Couldn't save “{}”: {e}", files::stem(&path))),
            },
            Event::Loaded {
                path,
                result,
                preview,
            } => {
                if gallery.opening.as_ref() != Some(&path) {
                    return; // ouverture abandonnée
                }
                match result {
                    Ok(loaded) => {
                        let info = loaded.info;
                        let has_thumbnail = loaded.thumbnail.is_some();
                        let mut editor = loaded.snapshot.into_editor();
                        let name = files::stem(&path);
                        editor.rig.name = name.clone();
                        session.current =
                            Some(OpenProject::new(path, name, info.created, &editor, has_thumbnail));
                        self.editor = editor;
                        let options = egui::TextureOptions::LINEAR;
                        let preview = preview.map(|image| self.egui_ctx.load_texture("zoom", image, options));
                        self.enter_editor(preview);
                    }
                    Err(e) => {
                        gallery.opening = None;
                        self.ui
                            .notify(format!("Couldn't open “{}”: {e}", files::stem(&path)));
                    }
                }
            }
            Event::Created { open, result } => {
                session.busy = None;
                session.worker.push(Task::List);
                match result {
                    Ok(created) if open => {
                        let editor = *created.editor;
                        let name = created.info.name.clone();
                        session.current = Some(OpenProject::new(
                            created.path,
                            name,
                            created.info.created,
                            &editor,
                            false,
                        ));
                        self.editor = editor;
                        self.ui.reset_for_project();
                        session.gallery.opening = None;
                        session.screen = Screen::Editor;
                    }
                    Ok(created) => {
                        // Projet d'exemple : vignette rendue tout de suite.
                        let thumb = crate::thumbnail::render(
                            &self.gpu,
                            &mut self.renderer,
                            &mut session.thumbnails,
                            &created.editor,
                        );
                        let mut project = OpenProject::new(
                            created.path,
                            created.info.name,
                            created.info.created,
                            &created.editor,
                            false,
                        );
                        let task = project.save_task(&created.editor, thumb);
                        session.worker.push(Task::Save(Box::new(task)));
                    }
                    Err(e) => self.ui.notify(format!("Couldn't create the project: {e}")),
                }
            }
            Event::Renamed { from, result } => {
                match result {
                    Ok(to) => {
                        gallery.library.moved(&from, &to);
                        gallery.thumbs.moved(&from, &to);
                        if let Some(project) = session.current.as_mut().filter(|p| p.path == from) {
                            project.name = files::stem(&to);
                            project.path = to;
                            self.editor.rig.name = project.name.clone();
                        }
                    }
                    Err(e) => self.ui.notify(format!("Couldn't rename: {e}")),
                }
                if let Some(project) = &mut session.current {
                    project.renaming = false;
                }
                session.worker.push(Task::List);
            }
            Event::Duplicated(result) | Event::Adopted(result) => {
                if let Err(e) = result {
                    self.ui.notify(format!("Error: {e}"));
                }
                session.busy = None;
                session.worker.push(Task::List);
            }
            Event::Deleted { path, result } => {
                gallery.thumbs.forget(&path);
                if session.current.as_ref().is_some_and(|p| p.path == path) {
                    session.current = None;
                }
                if let Err(e) = result {
                    self.ui.notify(format!("Couldn't delete: {e}"));
                }
                session.worker.push(Task::List);
            }
            Event::Listed(entries) => gallery.library.set_entries(entries),
            Event::Imports(files) => gallery.imports = Some(files),
            Event::Failed(message) => {
                session.busy = None;
                gallery.opening = None;
                session.worker.push(Task::List);
                self.ui.notify(format!("Error: {message}"));
            }
        }
    }
}
