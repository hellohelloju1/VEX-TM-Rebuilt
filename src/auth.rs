use anyhow::{Result, anyhow};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
pub fn hash(password: &str) -> Result<String> {
    if password.is_empty() {
        return Ok(String::new());
    }
    let bytes: [u8; 16] = rand::random();
    let salt = SaltString::encode_b64(&bytes).map_err(|e| anyhow!("Password salt: {e}"))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow!("Password hash: {e}"))
}
pub fn verify(password: &str, hash: &str) -> bool {
    hash.is_empty()
        || PasswordHash::new(hash).is_ok_and(|h| {
            Argon2::default()
                .verify_password(password.as_bytes(), &h)
                .is_ok()
        })
}

pub fn remote_allowed(local: bool, password: Option<&str>, hash: &str) -> bool {
    local || (!hash.is_empty() && password.is_some_and(|p| verify(p, hash)))
}
