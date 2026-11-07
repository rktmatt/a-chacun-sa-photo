# Rapport d'audit d'accessibilité RGAA 4.1.2 – À chacun sa photo

> **Statut : audit interne (autoévaluation), non certifiant.** Il a été réalisé le 7 novembre 2026 par le porteur du projet, avec l'aide d'un assistant IA (Claude). Il n'a pas été validé par un auditeur ou une auditrice indépendant·e. Les tests ont été faits sous macOS uniquement ; Windows et Linux n'ont pas été testés (voir « Limites »).
>
> Grilles (modèle officiel de la DINUM, à ouvrir avec LibreOffice) :
> - [grille-audit-rgaa-2026-11.ods](grille-audit-rgaa-2026-11.ods) : audit initial de la version 0.1.0 ;
> - [grille-audit-rgaa-2026-11-apres-corrections.ods](grille-audit-rgaa-2026-11-apres-corrections.ods) : contre-audit après les corrections.

## 1. Contexte

| | |
|---|---|
| Service audité | « À chacun sa photo », application de bureau hors ligne (Tauri 2, vue web WebKit sous macOS, WebView2 sous Windows, WebKitGTK sous Linux) |
| Versions | Audit initial : 0.1.0 (commit `0580268`). Contre-audit : 0.1.0 + corrections d'accessibilité du 7 novembre 2026 (voir `CHANGELOG.md`, section « Non publié ») |
| Date | 7 novembre 2026 |
| Référentiel | RGAA 4.1.2, 106 critères, niveaux A et AA du WCAG 2.1 |
| Réalisé par | Matthieu Roussel (porteur du projet), avec l'aide d'un assistant IA |
| Objectif | Préparer un pilote public et une diffusion sur les registres d'outils de l'Éducation nationale |

L'application tient sur un seul écran, qui change d'état. L'échantillon RGAA est donc fait **d'états d'écran** et non de pages web. Les pages « contact », « plan du site » et « mentions légales » n'existent pas dans une application hors ligne. Leur contenu (données, licence, aide clavier) se trouve dans « À propos ».

## 2. Échantillon

| N° | Écran | Contenu évalué |
|---|---|---|
| P01 | Ouverture de la classe | États « coffre-fort » et « ancienne classe illisible » |
| P02 | Phrase de passe | Création (Linux sans coffre-fort) et saisie, erreurs |
| P03 | Liste des élèves et gestion de la classe | Ajout, édition, suppression d'un élève, import CSV, sauvegarde/restauration, état « aucune photo » |
| P04 | Identification des photos | Photo, cadres (souris et clavier), zoom, bande de photos, masquage |
| P05 | Export des photos | Confirmation, choix du dossier, progression, message final |
| P06 | À propos | Données, aide clavier, licence, composants tiers |

## 3. Méthode et environnement de test

| Environnement | Ce qui a été testé |
|---|---|
| Code source | Lecture de `src/App.vue`, `src/style.css`, `index.html`, `src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs` |
| Chromium (panneau navigateur), interface servie par Vite, appels Tauri simulés | Tous les états de l'échantillon ; axe-core 4.10 (règles WCAG 2 A/AA et bonnes pratiques) ; ordre de tabulation et suivi du focus ; noms accessibles, rôles et zones ; contrastes (formule WCAG) ; texte à 200 % ; espacements (10.12) ; affichage à 640 et 320 px de large ; liens d'évitement ; erreurs de saisie |
| Application installée, macOS 26 (WebKit), version de débogage | Zoom du texte ⌘ + `=` / `-` / `0` jusqu'à 200 % ; affichage de l'écran d'ouverture et de l'espace de travail ; sélecteur de fichiers ; création et déplacement d'un cadre au clavier, focus visible ; coche ✓ de la bande de photos ; portée des raccourcis `+ - 0` ; erreur « nom en double » sous le champ ; menus et sélecteur de fichiers en français |
| VoiceOver (macOS 26) | Lecture de l'application avec une voix française (confirmée à l'écoute) |

**Non testé** :
- Windows (WebView2) et NVDA ;
- Linux (WebKitGTK) et Orca ;
- restitution détaillée au lecteur d'écran : annonce des messages d'état (7.5), des cadres (7.1) et des boîtes de dialogue système.

## 4. Résultats

| Indicateur | Audit initial | Après corrections |
|---|---|---|
| **Taux global de conformité** (critères conformes / critères applicables) | **67,3 %** (37 C, 18 NC, 51 NA) | **100 %** (55 C, 0 NC, 51 NA) |
| Taux moyen par écran | 74,6 % | 100 % |
| P01 Ouverture | 83,3 % | 100 % |
| P02 Phrase de passe | 78,9 % | 100 % |
| P03 Élèves / gestion de la classe | 73,3 % | 100 % |
| P04 Identification des photos | 66,7 % | 100 % |
| P05 Export | 61,0 % | 100 % |
| P06 À propos | 84,4 % | 100 % |

**Niveau atteint** : sur l'environnement testé (macOS), les 55 critères applicables sont respectés. Ce résultat est celui d'une **autoévaluation**. Deux critères reposent surtout sur l'analyse du code et du DOM, faute de test complet au lecteur d'écran :
- 7.1 (cadres) ;
- 7.5 (messages d'état).

Points forts constatés :
- alternative clavier complète au dessin des cadres (Entrée, flèches, Maj+flèches, Suppr), documentée dans « À propos » et dans une aide reliée par `aria-describedby` ;
- boutons à icône tous nommés (`aria-label`) ;
- élèves sélectionnables exposés avec `aria-pressed` ;
- `<dialog>` natif pour « À propos » ;
- textes de l'interface bien contrastés (≥ 4,5:1) ;
- application entièrement en français, interface système comprise (menus, sélecteurs de fichiers, voix de VoiceOver) ;
- confirmations par boîtes de dialogue système ;
- aucune limite de temps ni contenu animé ;
- police Atkinson Hyperlegible ;
- respect de `prefers-reduced-motion`.

## 5. Non-conformités de l'audit initial (toutes corrigées)

| Priorité | Critère | Constat initial | Correction |
|---|---|---|---|
| 1 | 7.3 | « Ajouter des photos » et « Importer une liste » : un `<label>` autour d'un `<input type="file">` en `display: none`, inaccessible au clavier | Vrais `<button>` qui ouvrent un `<input type="file" hidden>` |
| 1 | 10.4 / 10.11 | Zoom de la vue web désactivé (`zoomHotkeysEnabled` absent) ; largeur minimale de 900 px ; noms d'élèves tronqués | `zoomHotkeysEnabled: true` et permission `core:webview:allow-set-webview-zoom` ; ⌘/Ctrl + `0` pour revenir à 100 % ; `minWidth` 640 et `minHeight` 560 ; noms qui passent à la ligne ; lignes de cahier dessinées par `background-size`, car WebKit n'agrandit pas les arrêts de dégradé en px ; barre photo qui passe à la ligne |
| 1 | 7.1 | Cadre `div role="button"` contenant les boutons ⇄ et × (`nested-interactive`) | Cadre focusable `role="group"` + `aria-roledescription="cadre"` ; boutons ⇄ et × placés à côté du cadre, dans un conteneur `.tag-box` |
| 1 | 7.5 | `<p role="status">` créé seulement avec son texte, et remplacé pendant l'export | Zone `role="status"` toujours présente, seul son texte change ; annonce unique de l'export ; un message répété est réannoncé |
| 2 | 12.8 | Focus perdu sur `<body>` après l'ouverture de la classe, l'édition d'un élève, × sur un cadre, l'export | Focus replacé à chaque fois : titre « Élèves », champ du nom, bouton « Modifier … », photo, « Exporter par élève », « Ajouter des photos » |
| 2 | 11.1 | Champs sans étiquette visible (`aria-label` + placeholder) | Étiquettes visibles, avec la contrainte « 8 caractères minimum » |
| 2 | 11.10 | Erreurs dans le bandeau du haut, non reliées au champ | Message sous le champ, `aria-invalid`, `aria-describedby`, focus sur le champ |
| 2 | 3.3 / 10.7 | Bordures des champs à 1,72:1 ; focus bleu sur la zone sombre à 2,07:1 ; bande de photos à 1,86:1 et 1,74:1 | Bordures des champs `#7a8a83` (3,63:1) ; focus jaune `#ffd23f` sur la zone sombre (8,19:1) ; bordures de la bande `#8a9196` (4,33:1) et bordure verte « identifiée » `#6fbf8f` (6,28:1) |
| 2 | 12.10 | Raccourcis `+ = - 0` actifs partout | Actifs seulement quand le focus est dans la zone photo |
| 2 | 9.2 / 12.6 | `<header>` placé dans `<main>` | Structure : liens d'évitement, `<header>`, zone d'état, `<main id="contenu">` ; titre de niveau 2 (masqué) pour la zone photo |
| 2 | 12.7 | Pas de lien d'évitement | « Aller à la photo » et « Aller à la liste des élèves » (« Aller au contenu » sur l'écran d'ouverture), visibles au focus |
| 3 | 3.1 | État « identifiée » de la bande de photos donné par la couleur seule | Coche ✓ visible ; la photo courante est sur fond blanc |
| 3 | 3.2 | Placeholders à 3,52:1 | Placeholders à 5,36:1 ; les champs ont désormais des étiquettes |
| 3 | 11.9 | Boutons « 100 % » et « Retirer » peu explicites | aria-label « 100 %, ajuster à la fenêtre » ; « Retirer la photo » |
| 3 | 8.7 | Licences en anglais sans `lang="en"` | `lang="en"` sur le bloc des licences |

Une correction a été ajoutée en cours de vérification (8.3 / 8.7, application macOS). VoiceOver lisait l'application avec une voix anglaise, parce que le paquet se déclarait en anglais (`CFBundleDevelopmentRegion = English`). Les menus (« File, Edit… ») et le sélecteur de fichiers (« Open, Cancel ») étaient aussi en anglais. Deux ajouts corrigent cela :
- `src-tauri/Info.plist`, qui déclare la langue `fr` ;
- un menu macOS en français, dont « Quitter » passe par l'avertissement des photos non exportées.

La voix française a été confirmée à l'écoute.

## 6. Dérogations

Aucune.

## 7. Limites et suite

Ces limites sont connues et acceptées. Le porteur du projet n'a pas d'accès à un poste Windows ou Linux, ni à un auditeur indépendant.

- **Windows et Linux non testés.** Les corrections reposent sur du HTML, du CSS et des ARIA standard, sans code propre à une plateforme. Il y a deux exceptions :
  - le zoom du texte, géré nativement par WebView2 sous Windows et par une fonction de Tauri sous Linux ;
  - le menu en français, propre à macOS.
  
  Le risque principal est donc le zoom sous Linux. Les vérifications à faire figurent dans `docs/recette.md` ; elles peuvent être confiées à un enseignant ou à un contributeur qui utilise ces systèmes.
- **Pas de test complet au lecteur d'écran.** Les critères 7.1 et 7.5 sont jugés conformes d'après le code, le DOM et axe-core. Leur restitution réelle par NVDA (Windows), Orca (Linux) et VoiceOver (annonce des messages et des cadres) reste à vérifier.
- **Pas de validation indépendante.** Ce rapport est une autoévaluation et ne vaut pas audit de conformité opposable. La déclaration d'accessibilité le dit explicitement et invite les utilisateurs à signaler tout défaut.
- **Sous macOS, Tab** ne parcourt les boutons et les liens (dont les liens d'évitement) que si « Navigation au clavier » est activée dans Réglages Système > Clavier. C'est un réglage du système, comme dans Safari ; le guide de l'enseignant et la recette le mentionnent.
- La déclaration d'accessibilité correspondante est [declaration-accessibilite.md](declaration-accessibilite.md). Il faut refaire cet audit, ou au moins la recette, à chaque version qui modifie l'interface.
