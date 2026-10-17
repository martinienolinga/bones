//! Visèmes pour la synchronisation labiale, d'après les formes de bouche
//! standard de Preston Blair (*Advanced Animation*, 1947). Chaque visème
//! mappe mâchoire, lèvres et langue. Données CC0.

use super::super::builder::Expr;
use super::Spec;
use crate::presets::roles::RigType;

fn v(id: &'static str, name: &'static str, desc: &'static str, b: Expr) -> Spec {
    Spec::new(
        id,
        name,
        "Visemes",
        &["lipsync", "viseme"],
        desc,
        RigType::Humanoid,
        b.finish(),
    )
}

pub(super) fn all() -> Vec<Spec> {
    vec![
        v(
            "viseme_rest",
            "Viseme Rest",
            "Closed, neutral mouth.",
            Expr::new(),
        ),
        ai(),
        ee(),
        uu(),
        oo(),
        consonants(),
        fv(),
        ll(),
        mbp(),
        wq(),
        rr(),
        th(),
    ]
}

fn ai() -> Spec {
    let mut b = Expr::new();
    b.jaw_open(0.55);
    b.lips_part(0.35);
    v("ai", "AI", "Open \"ah\" / \"eye\".", b)
}

fn ee() -> Spec {
    let mut b = Expr::new();
    b.widen_mouth(0.6);
    b.jaw_open(0.2);
    b.lips_part(0.3);
    v("e", "E", "Wide \"ee\".", b)
}

fn uu() -> Spec {
    let mut b = Expr::new();
    b.pucker(0.7);
    b.jaw_open(0.2);
    v("u", "U", "Rounded \"oo\".", b)
}

fn oo() -> Spec {
    let mut b = Expr::new();
    b.pucker(0.5);
    b.jaw_open(0.5);
    b.lips_part(0.2);
    v("o", "O", "Open rounded \"oh\".", b)
}

fn consonants() -> Spec {
    let mut b = Expr::new();
    b.jaw_open(0.2);
    b.lips_part(0.35);
    b.widen_mouth(0.2);
    v("cdgknstxyz", "CDGKNSTXYZ", "Teeth-close consonants.", b)
}

fn fv() -> Spec {
    let mut b = Expr::new();
    b.lower_lip_up(1.0);
    b.lips_part(0.2);
    v("fv", "FV", "Lower lip to upper teeth.", b)
}

fn ll() -> Spec {
    let mut b = Expr::new();
    b.jaw_open(0.3);
    b.lips_part(0.3);
    b.tongue_up(1.0);
    v("l", "L", "Tongue tip to the roof.", b)
}

fn mbp() -> Spec {
    let mut b = Expr::new();
    b.lips_press(1.0);
    v("mbp", "MBP", "Pressed, closed lips.", b)
}

fn wq() -> Spec {
    let mut b = Expr::new();
    b.pucker(1.0);
    b.jaw_open(0.15);
    v("wq", "WQ", "Strong pucker \"w\".", b)
}

fn rr() -> Spec {
    let mut b = Expr::new();
    b.pucker(0.35);
    b.jaw_open(0.2);
    b.tongue_up(0.3);
    v("r", "R", "Slightly rounded \"r\".", b)
}

fn th() -> Spec {
    let mut b = Expr::new();
    b.jaw_open(0.3);
    b.lips_part(0.3);
    b.tongue_out(0.5);
    v("th", "TH", "Tongue between the teeth.", b)
}
