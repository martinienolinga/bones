//! Fil dédié aux accès disque des projets. Les travaux s'exécutent un par
//! un, dans l'ordre des demandes : un renommage ne croise jamais un
//! enregistrement du même fichier, une ouverture voit toujours la dernière
//! version enregistrée. Un enregistrement encore en attente pour un fichier
//! est remplacé par le plus récent (voir [`Queue`]).

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use super::tasks::{self, Dirs, Event, Task};

/// Réveil de la boucle d'événements quand un résultat est prêt.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// File d'attente avec regroupement.
#[derive(Default)]
pub struct Queue {
    tasks: VecDeque<Task>,
}

impl Queue {
    /// Ajoute un travail. Un enregistrement du même fichier déjà en attente
    /// est remplacé sur place ; une liste déjà demandée passe en fin de file
    /// (elle verra ainsi les travaux demandés entre-temps).
    pub fn push(&mut self, task: Task) {
        match &task {
            Task::Save(new) => {
                let queued = self
                    .tasks
                    .iter_mut()
                    .find(|t| matches!(t, Task::Save(old) if old.path == new.path));
                if let Some(slot) = queued {
                    *slot = task;
                    return;
                }
            }
            Task::List => self.tasks.retain(|t| !matches!(t, Task::List)),
            Task::ListImports => self.tasks.retain(|t| !matches!(t, Task::ListImports)),
            _ => {}
        }
        self.tasks.push_back(task);
    }

    pub fn pop(&mut self) -> Option<Task> {
        self.tasks.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Travaux en attente (tests).
    #[cfg(test)]
    pub fn iter(&self) -> impl Iterator<Item = &Task> {
        self.tasks.iter()
    }
}

#[derive(Default)]
struct State {
    queue: Queue,
    /// Un travail est en cours d'exécution.
    busy: bool,
    quit: bool,
}

type Shared = Arc<(Mutex<State>, Condvar)>;

pub struct Worker {
    shared: Shared,
    events: Receiver<Event>,
}

fn lock(shared: &Shared) -> MutexGuard<'_, State> {
    shared.0.lock().unwrap_or_else(|e| e.into_inner())
}

impl Worker {
    pub fn new(dirs: Dirs, wake: Waker) -> Self {
        let shared: Shared = Arc::default();
        let (sender, events) = channel();
        let thread_shared = shared.clone();
        std::thread::Builder::new()
            .name("projects".into())
            .spawn(move || run(&thread_shared, &dirs, &sender, &wake))
            .expect("création du fil des projets");
        Self { shared, events }
    }

    pub fn push(&self, task: Task) {
        lock(&self.shared).queue.push(task);
        self.shared.1.notify_all();
    }

    /// Résultats arrivés depuis le dernier appel.
    pub fn poll(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }

    /// Travaux en attente ou en cours.
    #[cfg(test)]
    pub fn is_busy(&self) -> bool {
        let state = lock(&self.shared);
        state.busy || !state.queue.is_empty()
    }

    /// Attend la fin de tous les travaux (passage en arrière-plan,
    /// fermeture) ; leurs résultats restent à relever avec [`Self::poll`].
    pub fn flush(&self) {
        let mut state = lock(&self.shared);
        while state.busy || !state.queue.is_empty() {
            state = self.shared.1.wait(state).unwrap_or_else(|e| e.into_inner());
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        lock(&self.shared).quit = true;
        self.shared.1.notify_all();
    }
}

fn run(shared: &Shared, dirs: &Dirs, sender: &Sender<Event>, wake: &Waker) {
    loop {
        let task = {
            let mut state = lock(shared);
            loop {
                if let Some(task) = state.queue.pop() {
                    state.busy = true;
                    break task;
                }
                if state.quit {
                    return;
                }
                state = shared.1.wait(state).unwrap_or_else(|e| e.into_inner());
            }
        };
        // Une erreur de programmation dans un travail (panique) ne doit pas
        // arrêter le fil : elle est signalée comme un échec.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tasks::run(task, dirs, &mut |event| {
                let _ = sender.send(event);
                wake();
            });
        }));
        if let Err(panic) = result {
            let message = panic
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "internal error".to_owned());
            let _ = sender.send(Event::Failed(message));
            wake();
        }
        lock(shared).busy = false;
        shared.1.notify_all();
    }
}
