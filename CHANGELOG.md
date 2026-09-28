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

### Modifié
- L’accord de diffusion est désormais **non autorisé par défaut**.
- Chaque export crée un nouveau dossier daté et n’écrase jamais un fichier existant.
- Tous les messages d’erreur sont en français.
- Identifiant de l’application : `fr.achacunsaphoto`.

### Sécurité
- Un cadre dont l’élève n’existe plus est flouté au lieu d’être laissé net.
- Les données supprimées de la liste des élèves sont écrasées dans le fichier (`secure_delete`).

## [0.1.0]

Première version d’essai : liste des élèves chiffrée, import JPEG/PNG/HEIC, cadres manuels, export par élève avec
floutage des enfants non autorisés et suppression des métadonnées.
