//! Une image : animation, interface (galerie ou éditeur), scène 3D, puis
//! rendu GPU. Sur la galerie, la passe 3D est sautée : effacement et
//! interface seulement.

use std::time::{Duration, Instant};

use crate::gallery::{self, GalleryAction};
use crate::manipulator;
use crate::running::Running;
use crate::session::Screen;
use crate::ui::{self, Busy, UiAction};

impl Running {
    pub fn redraw(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        let editing = self.session.screen == Screen::Editor;
        if editing {
            self.editor.update(dt);
        }
        self.poll_jobs();
        self.poll_projects();
        if let Some(job) = self.editor.bind_job() {
            self.jobs.bind(job);
        }

        // L'image GPU est réservée avant l'interface : si elle n'est pas
        // disponible, on ne construit rien (les textures d'egui produites par
        // une passe d'interface doivent toujours être envoyées au GPU).
        let Some(frame) = self.gpu.acquire() else {
            self.window.request_redraw();
            return;
        };

        // Interface.
        let editing = self.session.screen == Screen::Editor;
        let viewport = self.viewport();
        let busy = Busy {
            binding: self.jobs.is_binding(),
            jobs: self.jobs.running(),
        };
        let raw = self.egui_state.take_egui_input(&self.window);
        self.glass.publish(&self.egui_ctx);
        self.showcase.publish(&self.egui_ctx);
        let mut actions: Vec<UiAction> = Vec::new();
        let mut gallery_actions: Vec<GalleryAction> = Vec::new();
        let mut output = self.egui_ctx.run_pass(raw, |ctx| {
            if editing {
                actions = ui::show(ctx, &mut self.editor, &mut self.ui, &viewport, busy);
                manipulator::paint(ctx, &mut self.editor, &viewport);
            } else {
                let busy = self.session.busy.as_deref();
                gallery_actions = gallery::show(ctx, &mut self.session.gallery, busy);
                ui::show_toast(ctx, &mut self.ui);
            }
            if let Some(zoom) = &self.session.zoom {
                zoom.paint(ctx);
            }
        });
        self.egui_state
            .handle_platform_output(&self.window, output.platform_output);
        for action in actions {
            self.apply(action);
        }
        for action in gallery_actions {
            self.apply_gallery(action);
        }
        // Réglages enregistrés une fois le geste terminé (pas à chaque image
        // pendant qu'un curseur glisse).
        if !self.egui_ctx.input(|i| i.pointer.any_down()) {
            self.persist_settings();
        }
        self.repaint_at = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|v| v.repaint_delay)
            .filter(|d| *d < Duration::from_secs(3600))
            .map(|d| now + d);

        // Scène 3D (éditeur seulement).
        let target = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let size = self.gpu.size();
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("image") });
        if editing {
            let revisions = self.editor.rig.revisions();
            self.renderer
                .set_mesh(device, queue, &self.editor.rig.mesh, revisions.mesh);
            self.renderer
                .set_skin(device, queue, &self.editor.rig.skin, revisions.skin);
            let scene = self.scene.build(&mut self.editor, &viewport);
            self.renderer.prepare(device, queue, &scene);
            self.renderer.render(device, &mut encoder, &target, size);
            // Flou de la scène sous les panneaux vitrés (avant l'interface).
            self.glass.render(
                &self.egui_ctx,
                device,
                &mut encoder,
                &mut self.egui_renderer,
                &frame.texture,
            );
        }

        // Vignettes des galeries de préréglages, soumises avant l'image.
        self.showcase
            .render(&self.egui_ctx, device, queue, &mut self.egui_renderer);

        // Interface par-dessus (sur fond effacé dans la galerie).
        let ppp = output.pixels_per_point;
        let primitives = self.egui_ctx.tessellate(output.shapes, ppp);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: size,
            pixels_per_point: ppp,
        };
        for (id, deltas) in output.textures_delta.set.drain() {
            for delta in &deltas {
                self.egui_renderer.update_texture(device, queue, id, delta);
            }
        }
        let extra = self
            .egui_renderer
            .update_buffers(device, queue, &mut encoder, &primitives, &screen);
        let load = if editing {
            wgpu::LoadOp::Load
        } else {
            let [r, g, b, _] = gallery::BACKGROUND.to_normalized_gamma_f32();
            wgpu::LoadOp::Clear(wgpu::Color {
                r: r.into(),
                g: g.into(),
                b: b.into(),
                a: 1.0,
            })
        };
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.egui_renderer.render(&mut pass, &primitives, &screen);
        }
        for id in output.textures_delta.free.drain() {
            self.egui_renderer.free_texture(&id);
        }

        queue.submit(extra.into_iter().chain(std::iter::once(encoder.finish())));
        self.window.pre_present_notify();
        queue.present(frame);

        self.after_frame();
    }
}
