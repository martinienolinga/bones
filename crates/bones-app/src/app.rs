//! Cycle de vie de l'application (winit) et aiguillage des événements.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::window::{Window, WindowId};

use crate::UserEvent;
use crate::running::Running;

pub struct App {
    proxy: EventLoopProxy<UserEvent>,
    running: Option<Running>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self { proxy, running: None }
    }
}

fn window_attributes() -> winit::window::WindowAttributes {
    let attrs = Window::default_attributes().with_title("PaintSharp Bones");
    #[cfg(target_os = "ios")]
    let attrs = {
        use winit::platform::ios::{ScreenEdge, WindowAttributesExtIOS};
        attrs
            .with_prefers_status_bar_hidden(true)
            .with_prefers_home_indicator_hidden(true)
            // Les gestes système (centre de contrôle, accueil) ne volent pas
            // les glissements commencés au bord de l'écran.
            .with_preferred_screen_edges_deferring_system_gestures(ScreenEdge::ALL)
    };
    #[cfg(not(target_os = "ios"))]
    let attrs = attrs.with_inner_size(winit::dpi::LogicalSize::new(1366.0, 1024.0));
    attrs
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(window_attributes())
                .expect("création de la fenêtre"),
        );
        self.running = Some(Running::new(window, self.proxy.clone()));
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn user_event(&mut self, _: &ActiveEventLoop, event: UserEvent) {
        if let (UserEvent::JobDone, Some(r)) = (event, &mut self.running) {
            r.window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(r) = &mut self.running else { return };
        let consumed = r.egui_event(&event);

        match event {
            WindowEvent::CloseRequested => {
                r.autosave();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => r.gpu.resize(size.width, size.height),
            WindowEvent::RedrawRequested => r.redraw(),
            WindowEvent::Touch(t) => {
                let pos = glam::Vec2::new(t.location.x as f32, t.location.y as f32);
                match t.phase {
                    TouchPhase::Started => r.touch_start(t.id, pos),
                    TouchPhase::Moved => r.touch_move(t.id, pos),
                    TouchPhase::Ended => r.touch_end(t.id, false),
                    TouchPhase::Cancelled => r.touch_end(t.id, true),
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                r.mouse_move(glam::Vec2::new(position.x as f32, position.y as f32));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pan = matches!(button, MouseButton::Right | MouseButton::Middle);
                match state {
                    ElementState::Pressed => r.mouse_down(pan),
                    ElementState::Released => r.mouse_up(),
                }
            }
            WindowEvent::MouseWheel { delta, .. } if !consumed => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                r.mouse_wheel(lines);
            }
            _ => {}
        }
        r.window.request_redraw();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(r) = &mut self.running else { return };
        if r.wants_continuous_redraw() {
            r.window.request_redraw();
        }
        event_loop.set_control_flow(match r.next_repaint() {
            Some(deadline) => ControlFlow::WaitUntil(deadline),
            None => ControlFlow::Wait,
        });
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        // Passage en arrière-plan sur iPad : on sauvegarde le travail.
        if let Some(r) = &mut self.running {
            r.autosave();
        }
    }
}
