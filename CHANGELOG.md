# Journal des modifications

Format inspiré de [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/). Versions selon [SemVer](https://semver.org/lang/fr/).

## [Non publié]

### Ajouté
- Repli par phrase de passe quand l’ordinateur n’a pas de coffre-fort (fréquent sous Linux), avec explications pas à pas.
- Écran « À propos » : version, fonctionnement hors ligne, raccourcis clavier, licences des composants tiers.
- Dessin des cadres au clavier (Entrée, flèches, Maj + flèches, Suppr) pour l’accessibilité.
- « Visage inconnu » : cadre toujours flouté, pour un enfant hors de la classe.
- Modification et suppression d’un élève ; effacement de la classe en fin d’année.
- Import de la liste des élèves depuis un fichier texte ou un export tableur / ONDE.
- Sauvegarde et restauration de la liste des élèves, chiffrées par une phrase de passe.
- Zoom sur la photo, réattribution d’un cadre, retrait d’une photo de la session.
- Choix du masquage : flou ou aplat opaque.
- Avertissement avant de quitter si des photos identifiées n’ont pas été exportées.
- Documentation en français : guide de l’enseignant, kit RGPD (fiche de registre, modèle d’autorisation, note au DPO).
- Tests automatiques des règles de floutage, d’export et d’import de la liste (`npm test`), et recette manuelle avant
  publication (`docs/recette.md`).
- Pré-audit d’accessibilité RGAA 4.1.2 (`docs/rgaa/`) : grille complétée, rapport avec corrections priorisées, projet de
  déclaration d’accessibilité.
- Agrandissement du texte de l’application avec Ctrl/⌘ + `=` / `-` (Ctrl/⌘ + `0` pour revenir à 100 %) ; la fenêtre peut être réduite à 640 px de large.
- Mises à jour des dépendances par Dependabot (npm, cargo, GitHub Actions) avec délai de 7 jours ; sur GitHub, les pull
  requests ne lancent que les vérifications et les tests, et la CI vérifie que la liste des licences tierces est à jour.

### Corrigé (accessibilité)
- « Ajouter des photos » et « Importer une liste » sont désormais utilisables au clavier.
- Les boutons × et ⇄ ne sont plus imbriqués dans le cadre : un cadre est annoncé comme « cadre » par les lecteurs d’écran.
- Les messages d’état (bandeau, fin d’export, erreurs) sont lus par les lecteurs d’écran, y compris quand le même
  message revient.
- Retirer un cadre avec × rend le focus à la photo.
- Liens d’évitement « Aller à la photo » et « Aller à la liste des élèves », en-tête sorti de la zone principale.
- Étiquettes visibles sur tous les champs ; erreurs de saisie affichées sous le champ concerné, qui reçoit le focus.
- Le focus clavier n’est plus perdu après l’ouverture de la classe, la modification d’un élève, l’export ou l’effacement.
- Contrastes renforcés (bordures des champs, focus jaune sur la zone photo, bande de photos) ; coche ✓ sur les photos
  identifiées.
- Les touches `+` `-` `0` ne zooment la photo que lorsque le focus est sur la zone photo.
- Les noms d’élèves longs passent à la ligne au lieu d’être tronqués.
- macOS : l’application se déclare en français (`Info.plist`). VoiceOver lit donc l’interface avec une voix française,
  et les sélecteurs de fichiers sont en français. Les menus sont traduits (« Édition », « Fenêtre »), et « Quitter »
  affiche l’avertissement des photos non exportées.

### Modifié
- L’accord de diffusion est désormais **non autorisé par défaut**.
- Chaque export crée un nouveau dossier daté et n’écrase jamais un fichier existant.
- Tous les messages d’erreur sont en français.
- Identifiant de l’application : `fr.achacunsaphoto`.

### Sécurité
- Un cadre dont l’élève n’existe plus est flouté au lieu d’être laissé net.
- Les données supprimées de la liste des élèves sont écrasées dans le fichier (`secure_delete`).
- Mise à jour des dépendances signalées par des alertes de sécurité : source-map-js 1.2.2 (embarqué) ; outils de
  développement Vite 6.4.6, Vitest 5.1, vue-tsc 3.3 (tinypool et minimatch ne sont plus installés).

## [0.1.0]

Première version d’essai : liste des élèves chiffrée, import JPEG/PNG/HEIC, cadres manuels, export par élève avec
floutage des enfants non autorisés et suppression des métadonnées.
