# Déclaration d'accessibilité – À chacun sa photo

> À mettre à jour à chaque version qui modifie l'interface. Avant de publier, remplacer `[version]` par le numéro de la version publiée, et vérifier que la recette (`docs/recette.md`) a été passée sur cette version.

Matthieu Roussel, porteur du projet « À chacun sa photo », s'engage à rendre ce logiciel accessible, conformément à l'article 47 de la loi n° 2005-102 du 11 février 2005.

Cette déclaration d'accessibilité s'applique à l'application « À chacun sa photo », version [version].

## État de conformité

« À chacun sa photo » est **totalement conforme** au Référentiel général d'amélioration de l'accessibilité (RGAA), version 4.1.2, d'après un audit interne réalisé sous macOS (voir « Établissement de cette déclaration » et ses limites).

## Résultats des tests

L'audit interne (autoévaluation), réalisé le 7 novembre 2026 par le porteur du projet avec l'aide d'un assistant IA, révèle que :

- 100 % des critères du RGAA version 4.1.2 sont respectés (55 critères applicables sur 106) ;
- le taux moyen de conformité de l'application s'élève à 100 %.

Un premier passage sur la version 0.1.0 avait relevé 67,3 % de critères respectés. Les 18 critères non conformes ont été corrigés avant la publication de cette déclaration. Le détail figure dans le [rapport d'audit](rapport-audit-rgaa.md).

## Contenus non accessibles

### Non-conformités

Aucune non-conformité connue à la date de la déclaration.

### Dérogations pour charge disproportionnée

Aucune.

### Contenus non soumis à l'obligation d'accessibilité

- Les photos importées par l'enseignant : ce sont des contenus de tiers, et l'application ne décrit pas leur contenu.

## Établissement de cette déclaration

Cette déclaration a été établie le 7 novembre 2026. Elle repose sur un audit interne du 7 novembre 2026, qui n'a pas été validé par un auditeur ou une auditrice indépendant·e.

### Limites de l'évaluation

- L'application a été testée sous macOS uniquement. Les versions Windows (WebView2) et Linux (WebKitGTK) utilisent la même interface, mais elles n'ont pas été testées.
- Avec VoiceOver, seule la lecture de l'application en français a été vérifiée. L'annonce des messages d'état et des cadres a été évaluée d'après le code. Elle n'a pas été écoutée avec un lecteur d'écran.

Si vous constatez un défaut, en particulier sous Windows ou avec NVDA, merci de le signaler (voir « Retour d'information et contact »).

### Technologies utilisées

HTML5, CSS, JavaScript (Vue 3), dans une vue web système (Tauri 2).

### Environnement de test

| Agent utilisateur | Technologie d'assistance |
|---|---|
| WebKit, application installée (macOS 26) | VoiceOver (macOS 26), clavier, zoom du texte |
| Chromium (interface servie localement, à des fins de test) | Clavier, axe-core 4.10 |

### Outils pour évaluer l'accessibilité

- axe-core 4.10 ;
- inspection du code ;
- calcul des contrastes (formule WCAG) ;
- tests manuels au clavier, en zoom à 200 % et à 320 px de large ;
- VoiceOver.

### Écrans de l'application ayant fait l'objet de la vérification

1. Ouverture de la classe
2. Phrase de passe (création et saisie)
3. Liste des élèves et gestion de la classe
4. Identification des photos
5. Export des photos
6. À propos

## Retour d'information et contact

Si vous n'arrivez pas à accéder à un contenu ou à une fonctionnalité, vous pouvez contacter le porteur du projet. Il vous orientera vers une alternative accessible ou vous donnera le contenu sous une autre forme :

- en ouvrant un ticket sur le dépôt du projet : https://github.com/rktmatt/a-chacun-sa-photo/issues

## Voies de recours

Si vous constatez un défaut d'accessibilité qui vous empêche d'accéder à un contenu ou à une fonctionnalité, que vous nous le signalez et que vous ne parvenez pas à obtenir une réponse de notre part, vous êtes en droit de faire parvenir vos doléances ou une demande de saisine au Défenseur des droits :

- écrire un message au Défenseur des droits (formulaire en ligne sur defenseurdesdroits.fr) ;
- contacter le délégué du Défenseur des droits dans votre région ;
- envoyer un courrier par la poste (gratuit, ne pas mettre de timbre) : Défenseur des droits, Libre réponse 71120, 75342 Paris CEDEX 07.
