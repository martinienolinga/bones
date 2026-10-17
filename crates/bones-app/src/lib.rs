//! Application PaintSharp Bones : fenêtre winit, rendu wgpu et interface egui.
//!
//! Sur iPad, `run` ne rend jamais la main (winit lance `UIApplicationMain`).
//! Sur macOS et Linux, la même application sert au développement.

mod app;
mod clip;
mod files;
mod frame;
mod gizmo;
mod glass;
mod gpu;
mod input;
mod jobs;
mod manipulator;
mod running;
mod scene;
mod settings;
mod ui;

use winit::event_loop::EventLoop;

/// Événements envoyés à la boucle principale depuis les tâches de fond.
#[derive(Debug)]
pub enum UserEvent {
    /// Une tâche de fond est terminée : il faut relever les résultats.
    JobDone,
}

pub fn run() {
    #[cfg(not(target_os = "ios"))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("création de la boucle d'événements");
    let mut app = app::App::new(event_loop.create_proxy());
    event_loop.run_app(&mut app).expect("boucle d'événements");
}
