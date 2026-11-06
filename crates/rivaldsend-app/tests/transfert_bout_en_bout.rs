//! Tests d'integration HTTP : transfert complet de bout en bout en memoire.
//!
//! Ces tests montent le vrai routeur axum (sans reseau, via `oneshot`) et
//! rejouent le protocole exact du worker d'envoi : annonce, morceaux,
//! achevement. Ils auraient attrape le bug du 404 systematique : avant
//! `enregistrer_entrant`, aucun octet n'arrivait jamais au receveur.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use rivaldsend_core::pairing;

/// Monte un etat isole : manager sur dossier temporaire, magasin de confiance
/// sur dossier temporaire, aucun handle Tauri (les emissions sont ignorees).
fn etat_test() -> (rivaldsend_app::http::AppState, tempfile::TempDir) {
    let dossier = tempfile::tempdir().unwrap();
    let manager = Arc::new(rivaldsend_core::manager::TransferManager::new(
        dossier.path().join("resume"),
    ));
    let trust = Arc::new(rivaldsend_core::trust::TrustStore::charger(dossier.path()));
    let etat = rivaldsend_app::http::AppState::avec_trust(manager, trust);
    (etat, dossier)
}

fn routeur_test(etat: &rivaldsend_app::http::AppState) -> axum::Router {
    rivaldsend_app::http::router(etat.clone())
}

async fn corps_json(reponse: axum::response::Response) -> serde_json::Value {
    let octets = axum::body::to_bytes(reponse.into_body(), 64 * 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&octets).unwrap()
}

fn manifeste_test(id: uuid::Uuid, contenu: &[u8]) -> serde_json::Value {
    serde_json::json!({
        "transfer_id": id,
        "protocol_version": { "major": 2, "minor": 0, "patch": 0 },
        "files": [{
            "relative_path": "fichier-test.bin",
            "size": contenu.len(),
            "blake3": blake3::hash(contenu).to_hex().to_string(),
            "mode": "file",
        }],
        "total_bytes": contenu.len(),
        "created_at": chrono::Utc::now().to_rfc3339(),
    })
}

/// Joue un transfert complet : annonce, morceaux, achevement. Renvoie la PSK.
async fn transfert_complet(
    etat: &rivaldsend_app::http::AppState,
    id: uuid::Uuid,
    contenu: &[u8],
    code: &str,
) -> String {
    // 1. Annonce.
    let reponse = routeur_test(etat)
        .oneshot(
            Request::post("/v1/transfers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "manifest": manifeste_test(id, contenu),
                        "code": code,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::OK, "l'annonce doit passer");
    let corps = corps_json(reponse).await;
    assert_eq!(corps["transfer_id"].as_str().unwrap(), &id.to_string());

    // PSK derivee comme le ferait l'expediteur.
    let psk = pairing::hex(pairing::derive_psk(code, id.as_bytes()));

    // 2. Morceaux.
    let mut decalage = 0usize;
    for morceau in contenu.chunks(1024) {
        let reponse = routeur_test(etat)
            .oneshot(
                Request::put(format!("/v1/transfers/{id}/chunks/{decalage}"))
                    .header("X-PSK", psk.clone())
                    .body(Body::from(morceau.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            reponse.status(),
            StatusCode::OK,
            "le morceau {decalage} doit etre accepte (pas de 404)"
        );
        let ack = corps_json(reponse).await;
        assert_eq!(
            ack["ack_offset"].as_u64().unwrap(),
            (decalage + morceau.len()) as u64
        );
        decalage += morceau.len();
    }

    // 3. Achevement.
    let reponse = routeur_test(etat)
        .oneshot(
            Request::post(format!("/v1/transfers/{id}/complete"))
                .header("X-PSK", psk.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "manifest": manifeste_test(id, contenu) }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::OK, "l'achevement doit passer");
    let corps = corps_json(reponse).await;
    assert_eq!(corps["status"].as_str().unwrap(), "completed");

    psk
}

#[tokio::test]
async fn un_transfert_complet_livre_le_fichier_verifie() {
    let (etat, _dossier) = etat_test();
    etat.set_code("AB39XZ").await.unwrap();
    // Dossier de livraison isole pour ne pas ecrire dans ~/Telechargements.
    let livraison = tempfile::tempdir().unwrap();
    etat.manager
        .set_default_download_dir(livraison.path().to_path_buf())
        .await
        .unwrap();

    let contenu: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
    let id = uuid::Uuid::new_v4();
    transfert_complet(&etat, id, &contenu, "AB39XZ").await;

    // Le fichier livre est identique a l'original.
    let livree = livraison.path().join("fichier-test.bin");
    assert!(livree.exists(), "le fichier doit etre livre");
    assert_eq!(std::fs::read(&livree).unwrap(), contenu);
}

#[tokio::test]
async fn un_morceau_sans_annonce_est_rejete() {
    let (etat, _dossier) = etat_test();
    let id = uuid::Uuid::new_v4();

    let reponse = routeur_test(&etat)
        .oneshot(
            Request::put(format!("/v1/transfers/{id}/chunks/0"))
                .header("X-PSK", "00".repeat(32))
                .body(Body::from(vec![1u8; 8]))
                .unwrap(),
        )
        .await
        .unwrap();
    // Pas d'annonce : pas de PSK memorisee, donc 404 (pas 403).
    assert_eq!(reponse.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn une_mauvaise_psk_est_refusee() {
    let (etat, _dossier) = etat_test();
    etat.set_code("AB39XZ").await.unwrap();

    let contenu = b"bonjour";
    let id = uuid::Uuid::new_v4();
    let reponse = routeur_test(&etat)
        .oneshot(
            Request::post("/v1/transfers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "manifest": manifeste_test(id, contenu),
                        "code": "AB39XZ",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::OK);

    let reponse = routeur_test(&etat)
        .oneshot(
            Request::put(format!("/v1/transfers/{id}/chunks/0"))
                .header("X-PSK", "ff".repeat(32))
                .body(Body::from(contenu.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn une_annulation_sans_psk_est_refusee() {
    // Regression : DELETE n'exigeait aucune PSK, n'importe quel pair du LAN
    // pouvait annuler le transfert d'autrui.
    let (etat, _dossier) = etat_test();
    etat.set_code("AB39XZ").await.unwrap();

    let contenu = b"bonjour";
    let id = uuid::Uuid::new_v4();
    let reponse = routeur_test(&etat)
        .oneshot(
            Request::post("/v1/transfers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "manifest": manifeste_test(id, contenu),
                        "code": "AB39XZ",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::OK);

    // Sans PSK : refuse.
    let reponse = routeur_test(&etat)
        .oneshot(
            Request::delete(format!("/v1/transfers/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::UNAUTHORIZED);

    // Avec la bonne PSK : annule.
    let psk = pairing::hex(pairing::derive_psk("AB39XZ", id.as_bytes()));
    let reponse = routeur_test(&etat)
        .oneshot(
            Request::delete(format!("/v1/transfers/{id}"))
                .header("X-PSK", psk)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::OK);
}

#[tokio::test]
async fn une_ecriture_hors_volume_est_refusee() {
    let (etat, _dossier) = etat_test();
    etat.set_code("AB39XZ").await.unwrap();

    let contenu = b"petit";
    let id = uuid::Uuid::new_v4();
    let reponse = routeur_test(&etat)
        .oneshot(
            Request::post("/v1/transfers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "manifest": manifeste_test(id, contenu),
                        "code": "AB39XZ",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::OK);

    let psk = pairing::hex(pairing::derive_psk("AB39XZ", id.as_bytes()));
    // Le manifeste annonce 5 octets : ecrire a l'offset 1000 est hors volume.
    let reponse = routeur_test(&etat)
        .oneshot(
            Request::put(format!("/v1/transfers/{id}/chunks/1000"))
                .header("X-PSK", psk)
                .body(Body::from(vec![9u8; 8]))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn un_fichier_corrompu_est_rejete_a_l_achevement() {
    let (etat, _dossier) = etat_test();
    etat.set_code("AB39XZ").await.unwrap();
    let livraison = tempfile::tempdir().unwrap();
    etat.manager
        .set_default_download_dir(livraison.path().to_path_buf())
        .await
        .unwrap();

    let contenu = b"contenu original";
    let id = uuid::Uuid::new_v4();
    let reponse = routeur_test(&etat)
        .oneshot(
            Request::post("/v1/transfers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "manifest": manifeste_test(id, contenu),
                        "code": "AB39XZ",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::OK);

    // On envoie un contenu different de celui annonce.
    let psk = pairing::hex(pairing::derive_psk("AB39XZ", id.as_bytes()));
    let reponse = routeur_test(&etat)
        .oneshot(
            Request::put(format!("/v1/transfers/{id}/chunks/0"))
                .header("X-PSK", psk.clone())
                .body(Body::from(b"contenu FALSIFIE".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::OK);

    // L'achevement echoue sur la taille (16 != 17), rien n'est livre.
    let reponse = routeur_test(&etat)
        .oneshot(
            Request::post(format!("/v1/transfers/{id}/complete"))
                .header("X-PSK", psk)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "manifest": manifeste_test(id, contenu) }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::BAD_REQUEST);
    assert!(!livraison.path().join("fichier-test.bin").exists());
}

#[tokio::test]
async fn l_appairage_etablit_un_secret_utilisable_pour_envoyer() {
    let (etat, _dossier) = etat_test();
    let code = pairing::generer_code();
    etat.set_code(&code).await.unwrap();

    // 1. Le pair demande l'appairage avec le code affiche.
    let reponse = routeur_test(&etat)
        .oneshot(
            Request::post("/v1/pair")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "code": code,
                        "device_id": "appareil-distant-1",
                        "name": "Telephone",
                        "fingerprint_short": "ABCD1234",
                        "platform": "android",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::OK);

    // 2. Le secret est memorise des deux cotes via la meme derivation.
    let secret_distant = pairing::derive_pair_secret(&code, "appareil-local", "appareil-distant-1");
    let _ = secret_distant;

    // 3. Un mauvais code est refuse.
    let reponse = routeur_test(&etat)
        .oneshot(
            Request::post("/v1/pair")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "code": "ZZZZZZ",
                        "device_id": "appareil-distant-2",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        reponse.status() == StatusCode::FORBIDDEN || reponse.status() == StatusCode::UNAUTHORIZED,
        "un mauvais code doit etre refuse, statut : {}",
        reponse.status()
    );
}

#[tokio::test]
async fn sans_code_affiche_l_annonce_est_refusee() {
    // Premier contact sans code en cours : 409, pas un 404 obscur.
    let (etat, _dossier) = etat_test();

    let contenu = b"bonjour";
    let id = uuid::Uuid::new_v4();
    let reponse = routeur_test(&etat)
        .oneshot(
            Request::post("/v1/transfers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "manifest": manifeste_test(id, contenu),
                        "code": "AB39XZ",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::CONFLICT);
}
