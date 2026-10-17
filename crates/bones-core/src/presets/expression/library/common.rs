//! Expressions courantes et gestes isolés du visage (clins d'œil, clignement,
//! plissement, haussement de sourcils, moue, baiser, grimace, langue tirée).
//! Données CC0.

use super::super::builder::Expr;
use super::Spec;
use crate::presets::roles::RigType;

fn c(
    id: &'static str,
    name: &'static str,
    tags: &'static [&'static str],
    desc: &'static str,
    b: Expr,
) -> Spec {
    Spec::new(id, name, "Common", tags, desc, RigType::Humanoid, b.finish())
}

pub(super) fn all() -> Vec<Spec> {
    vec![
        blink(),
        wink(true),
        wink(false),
        eyes_closed(),
        squint(),
        brow_raise(),
        frown(),
        smirk(),
        mouth_open(),
        jaw_drop(),
        kiss(),
        sneer(),
        tongue_out(),
    ]
}

fn blink() -> Spec {
    let mut b = Expr::new();
    b.lids_close(1.0);
    c(
        "blink",
        "Blink",
        &["eyes", "close"],
        "Both eyes closed (blink).",
        b,
    )
}

fn wink(left: bool) -> Spec {
    let mut b = Expr::new();
    b.lid_close_one(left, 1.0);
    b.corner_up(left, 0.35);
    if left {
        c("wink.l", "Wink Left", &["eye", "wink"], "Left eye closed.", b)
    } else {
        c("wink.r", "Wink Right", &["eye", "wink"], "Right eye closed.", b)
    }
}

fn eyes_closed() -> Spec {
    let mut b = Expr::new();
    b.lids_close(0.85);
    c(
        "eyes_closed",
        "Eyes Closed",
        &["eyes", "rest"],
        "Eyes gently closed.",
        b,
    )
}

fn squint() -> Spec {
    let mut b = Expr::new();
    b.squint(0.9);
    c("squint", "Squint", &["eyes", "au7"], "Eyes narrowed.", b)
}

fn brow_raise() -> Spec {
    let mut b = Expr::new();
    b.brows_up(0.9);
    c(
        "brow_raise",
        "Brow Raise",
        &["brows", "au1", "au2"],
        "Both eyebrows raised.",
        b,
    )
}

fn frown() -> Spec {
    let mut b = Expr::new();
    b.brows_down(0.85);
    c(
        "frown",
        "Frown",
        &["brows", "au4"],
        "Brows drawn down and together.",
        b,
    )
}

fn smirk() -> Spec {
    let mut b = Expr::new();
    b.corner_up(true, 0.75);
    c(
        "smirk",
        "Smirk",
        &["mouth", "au12"],
        "Half smile, one corner raised.",
        b,
    )
}

fn mouth_open() -> Spec {
    let mut b = Expr::new();
    b.jaw_open(0.45);
    b.lips_part(0.6);
    c(
        "mouth_open",
        "Mouth Open",
        &["mouth", "jaw"],
        "Lips parted, jaw slightly open.",
        b,
    )
}

fn jaw_drop() -> Spec {
    let mut b = Expr::new();
    b.jaw_open(1.0);
    b.lips_part(0.5);
    c(
        "jaw_drop",
        "Jaw Drop",
        &["mouth", "jaw", "au26"],
        "Jaw dropped wide open.",
        b,
    )
}

fn kiss() -> Spec {
    let mut b = Expr::new();
    b.pucker(1.0);
    c(
        "kiss",
        "Pucker / Kiss",
        &["mouth", "au18"],
        "Lips pursed forward.",
        b,
    )
}

fn sneer() -> Spec {
    let mut b = Expr::new();
    b.nose_wrinkle(0.6);
    b.corner_up(true, 0.5);
    c(
        "sneer",
        "Sneer",
        &["mouth", "nose", "au10"],
        "Upper lip raised with a wrinkled nose.",
        b,
    )
}

fn tongue_out() -> Spec {
    let mut b = Expr::new();
    b.jaw_open(0.4);
    b.tongue_out(1.0);
    c(
        "tongue_out",
        "Tongue Out",
        &["tongue", "mouth"],
        "Tongue stuck out (needs tongue bones).",
        b,
    )
}
