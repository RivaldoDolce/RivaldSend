use hkdf::Hkdf;
use sha2::Sha256;
use subtle::ConstantTimeEq;
pub fn derive_psk(code: &str, salt: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(salt), code.as_bytes());
    let mut okm = [0u8; 32];
    let _ = hk.expand(b"rivaldsend-psk", &mut okm);
    okm
}
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).unwrap_u8() == 1
}
pub fn generate_code() -> String {
    let id = uuid::Uuid::new_v4().as_u128();
    format!("{:06}", id % 1_000_000)
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
    fn generated_code_is_six_digits() {
        for _ in 0..50 {
            let code = generate_code();
            assert_eq!(code.len(), 6);
            assert!(code.bytes().all(|c| c.is_ascii_digit()));
        }
    }
}
