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
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    generate_at(secret, now)
}

fn generate_at(secret: &str, unix_seconds: u64) -> AppResult<TotpInfo> {
    let bytes = decode_base32(secret)?;
    let raw = Secret::Raw(bytes).to_bytes().map_err(err)?;
    let totp = TOTP::new(Algorithm::SHA1, DIGITS, 1, PERIOD, raw).map_err(err)?;
    let code = totp.generate(unix_seconds);

    let seconds_remaining = PERIOD - (unix_seconds % PERIOD);

    Ok(TotpInfo {
        code,
        seconds_remaining,
        period: PERIOD,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rfc_secret() -> String {
        base32::encode(
            Alphabet::RFC4648 { padding: false },
            b"12345678901234567890",
        )
    }

    #[test]
    fn matches_rfc_6238_sha1_vectors() {
        let secret = rfc_secret();
        let vectors = [
            (59, "287082"),
            (1_111_111_109, "081804"),
            (1_111_111_111, "050471"),
            (1_234_567_890, "005924"),
            (2_000_000_000, "279037"),
        ];
        for (time, expected) in vectors {
            let info = generate_at(&secret, time).unwrap();
            assert_eq!(info.code, expected, "time: {time}");
            assert_eq!(info.period, 30);
        }
    }

    #[test]
    fn countdown_never_exceeds_period() {
        for time in [0, 1, 29, 30, 31, 1_234_567_890] {
            let info = generate_at(&rfc_secret(), time).unwrap();
            assert!((1..=PERIOD).contains(&info.seconds_remaining));
        }
    }

    #[test]
    fn accepts_lowercase_and_spaced_secret() {
        let secret = rfc_secret();
        let spaced = secret
            .chars()
            .enumerate()
            .map(|(index, ch)| {
                if index % 4 == 0 {
                    format!(" {ch}")
                } else {
                    ch.to_lowercase().to_string()
                }
            })
            .collect::<String>();
        assert_eq!(
            generate_at(&spaced, 59).unwrap().code,
            generate_at(&secret, 59).unwrap().code
        );
    }

    #[test]
    fn rejects_empty_and_invalid_secret() {
        assert!(generate_at("", 0).is_err());
        assert!(generate_at("!!!", 0).is_err());
    }
}
