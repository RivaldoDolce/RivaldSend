# Changelog

Toutes les modifications notables de ce projet seront documentées dans ce fichier.

Le format est basé sur [Keep a Changelog](https://keepachangelog.com/fr/1.0.0/) et le projet adhère à [Semantic Versioning](https://semver.org/lang/fr/).

## [Non publié]

### Ajouté
- Workspace 3 crates (proto/core/app) avec contrat IPC strict (zéro byte de fichier)
- Transfert single-shot et chunking par paliers (16 Mo -> 4 Mo) avec BLAKE3
- Endpoints HTTP/1.1 axum : /v1/negotiate, /v1/transfers, /v1/transfers/{id}/chunks, /v1/transfers/{id}/resume, /v1/transfers/{id}/complete
- Validation stricte des chemins, normalisation NFC, anti path traversal, limites manifestes
- Pairing PSK dérivé HKDF-SHA256, comparaison à temps constant, pinning Ed25519 (keystore OS)
- UI premium React 18 + Vite + Tailwind : DropZone, PeerList, Progress 4Hz, Pairing QR/code, Historique, Paramètres
- Thèmes clair/sombre, i18n FR/EN, design tokens CSS variables, WCAG AA
- Configuration Tauri 2.x, capabilities minimales, CSP stricte

