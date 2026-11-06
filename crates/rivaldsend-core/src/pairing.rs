use hkdf::Hkdf;
use sha2::Sha256;
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// Alphabet des codes d'appairage : 32 symboles sans 0/O/1/I, pour éviter
/// toute confusion de lecture quand le code est recopié depuis un écran.
pub const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// Longueur du code, en symboles.
pub const LONGUEUR_CODE: usize = 6;

/// Entropie d'un code (`32^6`), en bits. Un code n'est valide que le temps
/// d'un appairage : c'est un PRF a usage unique, pas un secret de long terme.
pub const ENTROPIE_CODE_BITS: u32 = 30;

/// Normalise un code saisi ou scanne : retire le tiret d'affichage, met en
/// majuscules. Point d'entree unique, Appariage, worker d'envoi et API HTTP
/// doivent tous passer par ici.
pub fn normaliser(code: &str) -> String {
    code.trim().replace('-', "").to_ascii_uppercase()
}

/// Verifie le format d'un code : 6 symboles de [`ALPHABET`], ou 6 chiffres
/// pour rester compatible avec les codes numeriques plus anciens.
pub fn code_valide(code: &str) -> bool {
    let n = normaliser(code);
    n.len() == LONGUEUR_CODE
        && (n.bytes().all(|c| c.is_ascii_digit()) || n.bytes().all(|c| ALPHABET.contains(&c)))
}

/// Genere un code depuis une source cryptographique.
///
/// `Uuid::new_v4()` tire 16 octets d'un CSPRNG du systeme ; on n'en utilise
/// qu'une partie, avec rejet des valeurs hors alphabet pour ne pas
/// introduire de biais modulo.
pub fn generer_code() -> String {
    let mut code = String::with_capacity(LONGUEUR_CODE);
    // 16 octets par tour : largement suffisant pour 6 symboles. La boucle
    // refait un tirage si tous les octets ont ete rejetes.
    while code.len() < LONGUEUR_CODE {
        for octet in Uuid::new_v4().into_bytes() {
            // Rejet des valeurs hors borne : tirage sans biais modulo.
            let borne = 256 / ALPHABET.len() as u16 * ALPHABET.len() as u16;
            if octet as u16 >= borne {
                continue;
            }
            let idx = (octet as u16 % ALPHABET.len() as u16) as usize;
            code.push(ALPHABET[idx] as char);
            if code.len() == LONGUEUR_CODE {
                break;
            }
        }
    }
    code
}

/// Formate un code pour l'affichage : `AB39XZ` -> `AB3-9XZ`.
pub fn code_affiche(code: &str) -> String {
    let n = normaliser(code);
    if n.len() == LONGUEUR_CODE {
        format!("{}-{}", &n[..3], &n[3..])
    } else {
        n
    }
}

/// Derive la cle de transfert a partir d'un secret (octets) et du sel du
/// transfert. Le secret est soit le code d'appairage (compatibilite), soit
/// le secret de paire etabli lors de l'appairage.
pub fn derive_psk_bytes(secret: &[u8], salt: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(salt), secret);
    let mut okm = [0u8; 32];
    // Un HKDF-SHA256 de 32 octets ne peut pas echouer : la longueur de sortie
    // est fixe et inferieure a 255 * hash_len.
    hk.expand(b"rivaldsend-psk", &mut okm)
        .expect("derive_psk_bytes : longueur de sortie HKDF invalide");
    okm
}

/// Derive la PSK d'un transfert depuis un code d'appairage. Chemin
/// historique, conserve pour les pairs qui ne sont pas (encore) apparies.
pub fn derive_psk(code: &str, salt: &[u8]) -> [u8; 32] {
    derive_psk_bytes(normaliser(code).as_bytes(), salt)
}

/// Derive le secret partage d'une paire a partir du code d'appairage.
///
/// Les deux appareils possedent le code (l'un l'affiche, l'autre le scanne ou
/// le saisit) et recuperent chacun leur `device_id` via `/v1/identity`. En
/// triant les deux identifiants, l'ordre n'a plus d'importance et les deux
/// cotes obtiennent le meme secret sans jamais le faire transiter sur le
/// reseau.
pub fn derive_pair_secret(code: &str, device_a: &str, device_b: &str) -> [u8; 32] {
    let (a, b) = if device_a <= device_b {
        (device_a, device_b)
    } else {
        (device_b, device_a)
    };
    let ikm = format!("rivaldsend-pair-v1|{}|{}|{}", normaliser(code), a, b);
    let hk = Hkdf::<Sha256>::new(Some(b"rivaldsend-pair-v1"), ikm.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"rivaldsend-pair-secret", &mut okm)
        .expect("derive_pair_secret : longueur de sortie HKDF invalide");
    okm
}

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).unwrap_u8() == 1
}

/// Encode une cle en hexadecimal minuscule (transportable en en-tete HTTP).
pub fn hex(psk: [u8; 32]) -> String {
    let mut sortie = String::with_capacity(64);
    for octet in psk {
        sortie.push_str(&format!("{octet:02x}"));
    }
    sortie
}

/// Decode un hexadecimal de 64 caracteres en 32 octets.
pub fn depuis_hex(texte: &str) -> Option<[u8; 32]> {
    let propre = texte.trim();
    if propre.len() != 64 || !propre.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let mut sortie = [0u8; 32];
    for (i, octet) in sortie.iter_mut().enumerate() {
        *octet = u8::from_str_radix(&propre[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(sortie)
}

/// Identifiant de pair stable, derive de l'adresse servie. Utilise comme cle
/// de cache et comme identifiant dans le magasin de confiance : survit au
/// redemarrage, contrairement a une cle d'annonce mDNS.
pub fn id_de_pair(ip: &str, port: u16) -> String {
    format!("peer-{ip}:{port}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn psk_is_deterministic_per_code_and_salt() {
        let a = derive_psk("123456", b"salt-1");
        assert_eq!(a, derive_psk("123456", b"salt-1"));
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn psk_differs_on_code_or_salt_change() {
        let base = derive_psk("123456", b"salt-1");
        assert_ne!(base, derive_psk("654321", b"salt-1"));
        assert_ne!(base, derive_psk("123456", b"salt-2"));
    }

    #[test]
    fn constant_time_eq_cases() {
        assert!(constant_time_eq(b"pair", b"pair"));
        assert!(!constant_time_eq(b"pair", b"paix"));
        assert!(!constant_time_eq(b"pair", b"pair!"));
    }

    #[test]
    fn le_code_genere_ressemble_a_un_code_affiche() {
        for _ in 0..200 {
            let code = generer_code();
            assert_eq!(code.len(), LONGUEUR_CODE, "longueur : {code}");
            assert!(
                code.bytes().all(|c| ALPHABET.contains(&c)),
                "hors alphabet : {code}"
            );
            // Le formatage doit etre un aller-retour stable.
            assert_eq!(normaliser(&code_affiche(&code)), code);
        }
    }

    #[test]
    fn le_code_genere_est_variable() {
        // 200 tirages sur 32^6 : le risque de collision complete est
        // negligible, on verifie juste qu'on ne rend pas toujours le meme.
        let premier = generer_code();
        assert!(
            (0..50).any(|_| generer_code() != premier),
            "generer_code rend une valeur constante"
        );
    }

    #[test]
    fn validation_du_format_de_code() {
        assert!(code_valide("AB39XZ"));
        assert!(code_valide("AB3-9XZ"), "le tiret d'affichage est tolere");
        assert!(code_valide("ab39xz"), "la casse est ignoree");
        assert!(code_valide("  AB39XZ  "), "les espaces sont ignores");
        assert!(code_valide("123456"), "format numerique historique");
        assert!(!code_valide(""));
        assert!(!code_valide("ABC"));
        assert!(!code_valide("AB39XZO"), "trop long");
        assert!(!code_valide("AB39X0"), "0 exclu de l'alphabet");
        assert!(!code_valide("AB39X1"), "1 exclu de l'alphabet");
        assert!(!code_valide("AB39XI"), "I exclu de l'alphabet");
        assert!(!code_valide("AB39XO"), "O exclu de l'alphabet");
    }

    #[test]
    fn le_secret_de_paire_est_symetrique() {
        // Quel que soit l'ordre dans lequel chaque appareil se designe,
        // les deux obtiennent le meme secret.
        let a = derive_pair_secret("AB39XZ", "appareil-a", "appareil-b");
        let b = derive_pair_secret("AB39XZ", "appareil-b", "appareil-a");
        assert_eq!(a, b);
    }

    #[test]
    fn le_secret_de_paire_change_avec_le_code_ou_les_paires() {
        let base = derive_pair_secret("AB39XZ", "a", "b");
        assert_ne!(base, derive_pair_secret("AB39XK", "a", "b"));
        assert_ne!(base, derive_pair_secret("AB39XZ", "a", "c"));
    }

    #[test]
    fn hex_aller_retour() {
        let secret = derive_pair_secret("AB39XZ", "a", "b");
        let encode = hex(secret);
        assert_eq!(encode.len(), 64);
        assert_eq!(depuis_hex(&encode), Some(secret));
        assert_eq!(depuis_hex("trop court"), None);
        assert_eq!(depuis_hex(&"z".repeat(64)), None);
    }
}
