# Convention GitHub — RivaldSend

> Document de référence pour la gestion du versionnement, des branches et des livraisons du projet RivaldSend. Toute contribution doit respecter cette convention. La branche de travail par défaut est `develop`. La branche `main` est la branche de production et ne reçoit jamais de commit direct.

---

## 1. Principes généraux

1. Langue unique : tout message de commit, description de branche, titre de pull request et entrée de changelog est rédigé en français.
2. Branche de base : `develop`. C'est le point de départ de toute nouvelle branche. Aucune branche ne part de `main` sans autorisation explicite.
3. Branche de production : `main`. Protégée. Fusion uniquement depuis `develop` ou `hotfix/*` après validation complète.
4. Qualité avant livraison : aucun code n'arrive sur `main` sans tests au vert, revue de code et vérification des critères de sortie de phase.
5. Traçabilité : chaque décision structurante fait l'objet d'un ADR dans `protocol-spec/adr/`.

## 2. Modèle de branches

### 2.1 Branches permanentes

| Branche | Rôle | Protection | Source |
|---|---|---|---|
| `main` | Production. Code déployable. Tags de version. | Oui — pas de push direct, PR obligatoire, CI verte, 1 revue minimum | Fusion depuis `develop` ou `hotfix/*` |
| `develop` | Intégration. Base de tout développement. | Oui — pas de commit direct non vérifié, CI verte | Fusion depuis `feat/*`, `fix/*`, `chore/*` |

### 2.2 Branches temporaires

Toutes les branches temporaires partent de `develop` et y reviennent par pull request.

| Préfixe | Usage | Exemple | Durée de vie |
|---|---|---|---|
| `feat/` | Nouvelle fonctionnalité ou phase | `feat/p0-transfert-single-shot` | Supprimée après fusion |
| `fix/` | Correction de bug | `fix/reprise-offset-fsync` | Supprimée après fusion |
| `chore/` | Maintenance, outillage, dépendances | `chore/mise-a-jour-rustls` | Supprimée après fusion |
| `docs/` | Documentation seule | `docs/spec-v2.2` | Supprimée après fusion |
| `release/` | Préparation d'une version | `release/v0.1.0-mvp` | Fusionnée vers `main` + `develop` |
| `hotfix/` | Correctif urgent de production | `hotfix/v0.1.1-crash-tls` | Part de `main`, fusionnée vers `main` + `develop` |

Règles de nommage :
- minuscules, tirets comme séparateurs, pas d'accents ni d'espaces.
- court et explicite : `feat/p2-pairing-qr` et non `feat/nouvelle-fonctionnalite`.

### 2.3 Schéma du flux

```
main (production)  ──────────────────────────────────●──────────●─
                    \                               /          /
develop (base) ──────●──●──●──●──●──●──●──●──●──●──●──────────●─
                      \  \  \  \  \  \  \  \
feat/*                feat feat feat fix  chore docs release
```

Interdit :
- pousser directement sur `main`.
- créer une branche depuis `main` pour une fonctionnalité normale.
- fusionner une branche sans pull request.

## 3. Stratégie de versions

Le versionnement suit SemVer `MAJEURE.MINEURE.CORRECTIF` et la roadmap de la spécification.

| Version | Phase | Contenu | Branche de préparation |
|---|---|---|---|
| `v0.1.0` | P0 — Prototype | HTTP/TLS, transfert single-shot, BLAKE3 | `release/v0.1.0-p0` |
| `v0.2.0` | P1 — Fiabilité | Chunking, reprise, 12 scénarios toxiproxy, soak 1000 | `release/v0.2.0-p1` |
| `v0.3.0` | P2 — Produit desktop | mDNS, pairing PSK/QR, UI complète, dossiers base | `release/v0.3.0-p2` |
| `v1.0.0` | P3 — MVP commercial | Packaging signé, updater, install < 2 min | `release/v1.0.0-mvp` |
| `v1.1.0` | P4 — Linux + polish | .deb/AppImage, historique, i18n, dark mode | `release/v1.1.0-p4` |
| `v1.2.0` | P5 — Android | Foreground Service, SAF, Share Sheet | `release/v1.2.0-android` |
| `v1.3.0` | P6 — iOS | Keychain iOS, permission réseau local | `release/v1.3.0-ios` |
| `v2.0.0` | V2 | Mode invité, streaming Range, compression | `release/v2.0.0` |
| `v3.0.0` | V3 | Salles multi-appareils, sync, relay | `release/v3.0.0` |

Chaque release produit :
- un tag annoté `vX.Y.Z` sur `main`.
- une entrée dans `CHANGELOG.md` au format Keep a Changelog.
- un binaire signé et un SBOM.

## 4. Convention de commits

### 4.1 Format obligatoire

```
type(portée): description courte à l'impératif présent

[corps détaillé si nécessaire]

[BREAKING CHANGE: description si rupture]
```

- `type` : voir table ci-dessous.
- `portée` : crate ou domaine (`proto`, `core`, `app`, `ui`, `ci`, `docs`, `securite`, `build`).
- `description` : en français, minuscule, sans point final, max 72 caractères, à l'impératif présent.

### 4.2 Types autorisés

| Type | Usage |
|---|---|
| `feat` | Nouvelle fonctionnalité |
| `fix` | Correction de bug |
| `docs` | Documentation uniquement |
| `style` | Mise en forme, formatage, sans changement logique |
| `refactor` | Refactorisation sans changement fonctionnel |
| `perf` | Amélioration de performance |
| `test` | Ajout ou correction de tests |
| `build` | Système de build, dépendances |
| `ci` | Intégration continue |
| `chore` | Maintenance, outillage |
| `revert` | Annulation d'un commit précédent |

### 4.3 Exemples conformes

```
feat(core): implémente la reprise symétrique avec persistance fsync
fix(proto): rejette les chemins contenant un traversal parent
docs(spec): clarifie la politique de chunking par paliers
refactor(app): factorise la construction du routeur axum
test(core): ajoute les 12 scénarios de coupure toxiproxy
chore(ci): ajoute cargo audit et cargo deny au pipeline
build(deps): met à jour rustls vers 0.23.12
perf(core): réutilise les buffers BytesMut pour réduire les allocations
```

### 4.4 Exemples non conformes

```
fix bug -> manque type et portée, pas en français conventionnel
Feat(Core): Ajout Fonctionnalité. -> majuscule, point final, accent mal placé
feat: wip -> description vide
update -> type inconnu
```

### 4.5 Règles senior

- Un commit = une intention. Pas de commit fourre-tout.
- Taille visée : < 400 lignes de diff hors lockfiles. Au-delà, découper.
- Messages clairs pour une relecture dans 6 mois sans contexte.
- Pas de `fixup!`, `wip`, `tmp` sur `develop` ou `main`. Squash avant fusion.
- Jamais `--no-verify`. Corriger la cause du hook qui échoue.

## 5. Flux de travail détaillé

### 5.1 Développer une fonctionnalité

```bash
git checkout develop
git pull origin develop
git checkout -b feat/ma-fonctionnalite
# ... développement ...
git add <fichiers concernés>
git commit -m "feat(portée): description claire de la fonctionnalité"
git push -u origin feat/ma-fonctionnalite
# Ouvrir une pull request vers develop
```

Exigences avant fusion vers `develop` :
- `cargo build --all-targets` au vert.
- `cargo clippy --all-targets -- -D warnings` au vert.
- `cargo test` ou `cargo nextest run` au vert.
- `cargo audit` et `cargo deny check` au vert.
- `pnpm lint` et `pnpm build` au vert si frontend touché.

### 5.2 Livrer une version vers production

```bash
git checkout develop
git pull origin develop
git checkout -b release/v1.0.0-mvp
# bump version, CHANGELOG.md, vérifications finales
git commit -m "chore(release): prépare la version v1.0.0 mvp"
git push -u origin release/v1.0.0-mvp
# PR release/v1.0.0-mvp -> main
# Après validation :
git checkout main
git merge --no-ff release/v1.0.0-mvp
git tag -a v1.0.0 -m "version v1.0.0 mvp commercial"
git push origin main --tags
git checkout develop
git merge --no-ff main
git push origin develop
```

### 5.3 Correctif urgent de production

```bash
git checkout main
git pull origin main
git checkout -b hotfix/v1.0.1-correction-critique
# ... correctif minimal ...
git commit -m "fix(app): corrige le plantage au démarrage sur réseau public"
git push -u origin hotfix/v1.0.1-correction-critique
# PR vers main + cherry-pick ou fusion vers develop
```

## 6. Pull requests

- Titre en français, format commit conventionnel.
- Description : contexte, solution, comment tester, risques, ADR lié si applicable.
- Taille : viser < 400 lignes. Au-delà, justifier ou découper.
- Checks obligatoires : CI verte sur Windows, macOS, Linux.
- Revue : 1 approbation minimum. L'auteur ne s'auto-approuve pas.
- Squash ou rebase selon la propreté de l'historique. Préférer squash pour les branches feat avec commits intermédiaires brouillons.

Modèle de description :

```
Contexte : ...
Changements :
- ...
Tests :
- [ ] cargo test au vert
- [ ] cargo clippy au vert
- [ ] tests manuels : ...
Risques : ...
ADR : ...
```

## 7. Protection des branches

| Règle | `main` | `develop` |
|---|---|---|
| Push direct | Interdit | Interdit sans PR si > 1 contributeur |
| PR obligatoire | Oui | Oui (recommandé) |
| CI verte obligatoire | Oui | Oui |
| Revue requise | 1 minimum | 1 recommandée |
| Résolution de conversation | Oui | Oui |
| Force push | Interdit | Interdit |

Configuration GitHub à appliquer dans Settings > Branches > Branch protection rules.

## 8. Tests avant production

Aucune fusion vers `main` sans :

1. Tests unitaires et d'intégration au vert (`proto` ≥ 80%, `core` ≥ 80%).
2. Soak test 1000 transferts si P1 atteint.
3. 12 scénarios de coupure toxiproxy au vert si P1 atteint.
4. Audit de sécurité : `cargo audit`, `cargo deny`, `pnpm audit`.
5. Vérification manuelle : install → transfert en < 2 min sur machine propre.

## 9. Changelog

Fichier `CHANGELOG.md` à la racine, format Keep a Changelog :

```markdown
## [1.0.0] - 2026-09-03
### Ajouté
- Transfert single-shot avec BLAKE3
### Corrigé
- Rejet du path traversal
```

Chaque PR qui modifie le comportement utilisateur met à jour le changelog.

## 10. Commandes de référence

```bash
git checkout develop && git pull
git checkout -b feat/nouvelle-fonctionnalite
git commit -m "feat(core): ajoute le chunking par paliers"
git push -u origin feat/nouvelle-fonctionnalite

git log --oneline --graph --all -20
git diff develop...feat/ma-branche
cargo build --all-targets && cargo clippy --all-targets -- -D warnings
cargo nextest run
```

## 11. Rôle de l'agent ingénieur senior

L'agent qui opère sur ce dépôt :
- travaille exclusivement depuis `develop`.
- crée les branches nécessaires, rédige les commits en français, ouvre les PR.
- ne touche jamais à `main` sans autorisation explicite du propriétaire.
- committe dès qu'une unité de travail cohérente est terminée et testée.
- documente toute décision hors spec par un ADR.
- maintient ce fichier à jour à chaque évolution du flux.

---
*Dernière mise à jour : 2026-09-03 — Branche de référence : `develop`*
