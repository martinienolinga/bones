//! Gros plan automatique sur la tête à l'étape « Face » de l'auto-rig : la
//! caméra glisse vers une vue de face cadrée sur la tête, entre la barre du
//! haut et le panneau du bas, puis revient au cadrage d'avant quand on quitte
//! l'étape (retour à « Body », autre mode, « Generate rig »).
//!
//! Règle de retour : le cadrage d'avant est toujours rétabli, même si
//! l'utilisateur a bougé la caméra pendant l'étape « Face » — un gros plan
//! de la tête ne sert à rien pour les marqueurs du corps, et l'on retrouve
//! ainsi exactement la vue qu'on avait quittée. Un geste de caméra pendant
//! une transition l'interrompt aussitôt (le doigt a toujours le dernier mot).
//!
//! Le cadrage est un état de vue : il ne passe pas par l'historique
//! d'annulation. Le calcul (région de la tête, cadrage exact dans la zone
//! utile) est dans `bones-core` ([`Editor::face_camera`]).

use bones_core::{Editor, OrbitCamera, Viewport};

use super::BAR_H;

/// Durée d'une transition (secondes).
const DURATION: f64 = 0.35;
/// Marges latérales (bandeaux d'outils) et écart aux panneaux, en points.
const SIDE: f32 = 96.0;
const GAP: f32 = 16.0;
/// Part de la hauteur utile laissée libre au-dessus et au-dessous de la tête.
const COMFORT: f32 = 0.08;

/// État du gros plan, gardé d'une image à l'autre.
#[derive(Default)]
pub struct FaceView {
    /// Étape « Face » affichée à l'image précédente.
    active: bool,
    /// Cadrage d'avant le gros plan, rétabli à la sortie.
    restore: Option<OrbitCamera>,
    /// Révision du maillage : un nouveau modèle est recadré.
    mesh: u64,
    transition: Option<Transition>,
}

#[derive(Clone, Copy)]
struct Transition {
    from: OrbitCamera,
    to: OrbitCamera,
    start: f64,
    /// Retour au cadrage d'avant.
    back: bool,
    /// Dernière caméra écrite : si elle a changé, c'est un geste.
    written: OrbitCamera,
}

impl FaceView {
    /// Projet ouvert ou refermé : rien à rétablir.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Sortie immédiate du gros plan, sans transition (retour à la galerie :
    /// la vignette et la prochaine ouverture montrent le modèle entier).
    pub fn settle(&mut self, editor: &mut Editor) {
        let back = self.transition.take().filter(|t| t.back).map(|t| t.to);
        if let Some(previous) = self.restore.take().or(back) {
            editor.camera = editor.camera.blend(&previous, 1.0);
        }
        self.active = false;
    }

    /// À appeler à chaque image, avant de dessiner les marqueurs du visage.
    /// `active` : l'étape « Face » est affichée ; `panel_top` : haut du
    /// panneau du bas (points).
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        editor: &mut Editor,
        active: bool,
        viewport: &Viewport,
        panel_top: Option<f32>,
    ) {
        let now = ctx.input(|i| i.time);
        if self
            .transition
            .is_some_and(|t| !editor.camera.same_view(&t.written))
        {
            self.transition = None;
        }
        let mesh = editor.rig.revisions().mesh;
        let remodeled = std::mem::replace(&mut self.mesh, mesh) != mesh;
        if active && (!self.active || remodeled) {
            // Retour en cours : le cadrage à rétablir reste celui d'origine.
            let base = match self.transition {
                Some(t) if t.back && !remodeled => t.to,
                _ => editor.camera,
            };
            let safe = safe_area(ctx, panel_top);
            if let Some(close_up) = editor.face_camera(viewport, &safe) {
                self.restore = Some(base);
                self.start(ctx, editor, close_up, false, now);
            }
        } else if !active
            && self.active
            && let Some(previous) = self.restore.take()
        {
            self.start(ctx, editor, previous, true, now);
        }
        self.active = active;
        self.step(ctx, editor, now);
    }

    fn start(&mut self, ctx: &egui::Context, editor: &Editor, to: OrbitCamera, back: bool, now: f64) {
        // L'alignement animé du guide de rotation s'efface devant celui-ci.
        crate::gizmo::cancel_animation(ctx);
        self.transition = Some(Transition {
            from: editor.camera,
            to,
            start: now,
            back,
            written: editor.camera,
        });
    }

    /// Fait avancer la transition ; une image est redemandée tant qu'elle dure.
    fn step(&mut self, ctx: &egui::Context, editor: &mut Editor, now: f64) {
        let Some(t) = &mut self.transition else { return };
        let k = ((now - t.start) / DURATION).clamp(0.0, 1.0) as f32;
        // Départ et arrivée en douceur, comme les alignements du guide.
        let ease = k * k * (3.0 - 2.0 * k);
        let pose = t.from.blend(&t.to, ease);
        // Seul le point de vue bouge : verrou et projection restent ceux de
        // l'utilisateur.
        editor.camera = OrbitCamera {
            target: pose.target,
            distance: pose.distance,
            yaw: pose.yaw,
            pitch: pose.pitch,
            ..editor.camera
        };
        t.written = editor.camera;
        if k >= 1.0 {
            self.transition = None;
        } else {
            ctx.request_repaint();
        }
    }
}

/// Zone utile de l'écran (pixels physiques) : sous la barre du haut, au-dessus
/// du panneau du bas, entre les bandeaux, avec une marge de confort.
fn safe_area(ctx: &egui::Context, panel_top: Option<f32>) -> Viewport {
    let screen = ctx.content_rect();
    let ppp = ctx.pixels_per_point();
    let top = screen.top() + BAR_H + GAP;
    let bottom = panel_top.unwrap_or(screen.bottom() - 120.0) - GAP;
    let margin = COMFORT * (bottom - top).max(0.0);
    let (top, bottom) = (top + margin, bottom - margin);
    Viewport {
        x: (screen.left() + SIDE) * ppp,
        y: top * ppp,
        width: (screen.width() - 2.0 * SIDE).max(1.0) * ppp,
        height: (bottom - top).max(1.0) * ppp,
    }
}
