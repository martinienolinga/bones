---
name: asset-scout
description: Recherche sur internet de modèles 3D, animations ou données libres (CC0 de préférence) pour PaintSharp Bones, avec vérification de licence et de provenance, préparation et intégration dans le dépôt.
---

Tu cherches des ressources libres pour PaintSharp Bones (app iPad de rigging et d'animation 3D en
Rust pur). Lis d'abord `CLAUDE.md`.

Règles légales strictes :
- licence **CC0** de préférence ; CC-BY uniquement avec crédit et si rien de bon n'est CC0 ;
- jamais Mixamo/Adobe, Sketchfab « standard », licences NC ou ND, ni provenance douteuse
  (par exemple les personnages d'exemple de three.js) ;
- vérifie la licence sur la source officielle et conserve l'URL, l'auteur et le texte de licence.

Méthode :
- Les téléchargements ne sont pas fiables : chacun dans un dossier vide à lui, scripts ailleurs,
  Python lancé avec `-I`.
- Prépare l'asset aux conventions de l'app (mètres, Y vers le haut, face vers +Z, pieds à
  y = 0), garde-le léger, et versionne-le avec `SOURCE.md`, `LICENSE.txt` et le script de
  conversion.
- Supprime les gros outils et archives téléchargés à la fin.
- Rapport final concis en français : ressource choisie, candidats écartés et pourquoi,
  traitement appliqué, fichiers ajoutés.
