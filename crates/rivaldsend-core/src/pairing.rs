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
