//! Demandes de la galerie, confiées au fil des projets.

use bones_core::Rig;
use bones_core::io::Imported;

use crate::gallery::GalleryAction;
use crate::projects::Task;
use crate::running::Running;

use super::create;

impl Running {
    pub fn apply_gallery(&mut self, action: GalleryAction) {
        let session = &mut self.session;
        let worker = &session.worker;
        match action {
            GalleryAction::Open(path) => {
                if session.current.as_ref().is_some_and(|p| p.path == path) {
                    // Projet quitté à l'instant : encore en mémoire.
                    self.enter_editor(None);
                } else {
                    worker.push(Task::Load(path));
                }
            }
            GalleryAction::New(template) => {
                session.busy = Some(format!("Creating “{}”…", template.name()));
                worker.push(create(template, true));
            }
            GalleryAction::Import(file) => match file.format {
                Some(_) => {
                    session.busy = Some(format!("Importing “{}”…", file.name));
                    let (path, name) = (file.path, file.name.clone());
                    let build = Box::new(move || match bones_core::io::import(&path) {
                        Ok(Imported::Rig(rig)) => Ok(rig),
                        Ok(Imported::Mesh(mesh)) => Ok(Rig::new(name, mesh)),
                        Err(e) => Err(e.to_string()),
                    });
                    worker.push(Task::Create {
                        name: file.name,
                        build,
                        open: true,
                    });
                }
                None => {
                    session.busy = Some(format!("Importing “{}”…", file.name));
                    worker.push(Task::Adopt(file.path));
                }
            },
            GalleryAction::ListImports => worker.push(Task::ListImports),
            GalleryAction::Rename(path, name) => {
                if let Some(project) = session.current.as_mut().filter(|p| p.path == path) {
                    project.renaming = true;
                }
                worker.push(Task::Rename(path, name));
            }
            GalleryAction::Duplicate(paths) => {
                for path in paths {
                    worker.push(Task::Duplicate(path));
                }
            }
            GalleryAction::Delete(paths) => {
                for path in paths {
                    worker.push(Task::Delete(path));
                }
            }
        }
    }
}
