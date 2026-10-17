---
description: Vérifie tout le dépôt avant un commit (format, clippy, tests, cible iPad)
---

Lance, dans cet ordre, depuis la racine du dépôt :

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo check -p bones-app --target aarch64-apple-ios`

Résume en français : ce qui passe, et pour chaque échec le fichier, la ligne et la cause. Si des
sous-agents travaillent en même temps, indique quels échecs viennent de leurs fichiers en cours
plutôt que des modifications à committer. Ne corrige rien sans qu'on te le demande.
