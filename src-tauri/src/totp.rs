use crate::{
    error::{err, AppResult},
    models::TotpInfo,
};
use base32::Alphabet;
use std::time::{SystemTime, UNIX_EPOCH};
use totp_rs::{Algorithm, Secret, TOTP};

const PERIOD: u64 = 30;
const DIGITS: usize = 6;

fn decode_base32(secret: &str) -> AppResult<Vec<u8>> {
    let cleaned: String = secret
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();

    if cleaned.is_empty() {
        return Err(crate::error::AppError::new("TOTP-секрет пуст"));
    }

    let padded = if cleaned.len() % 8 == 0 {
        cleaned.clone()
    } else {
        format!("{}{}", cleaned, "=".repeat(8 - cleaned.len() % 8))
    };

    base32::decode(Alphabet::RFC4648 { padding: true }, &padded)
        .or_else(|| base32::decode(Alphabet::RFC4648 { padding: false }, &cleaned))
        .ok_or_else(|| crate::error::AppError::new("Некорректный Base32 TOTP-секрет"))
}

pub fn generate(secret: &str) -> AppResult<TotpInfo> {
    let bytes = decode_base32(secret)?;
    let raw = Secret::Raw(bytes).to_bytes().map_err(err)?;
    let totp = TOTP::new(Algorithm::SHA1, DIGITS, 1, PERIOD, raw).map_err(err)?;
    let code = totp.generate_current().map_err(err)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let seconds_remaining = PERIOD - (now % PERIOD);

    Ok(TotpInfo {
        code,
        seconds_remaining,
        period: PERIOD,
    })
}
