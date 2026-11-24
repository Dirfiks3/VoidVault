use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Login,
    Card,
    Note,
    ApiKey,
    Ssh,
}

impl Default for Category {
    fn default() -> Self {
        Self::Login
    }
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Card => "card",
            Self::Note => "note",
            Self::ApiKey => "apiKey",
            Self::Ssh => "ssh",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "login" => Ok(Self::Login),
            "card" => Ok(Self::Card),
            "note" => Ok(Self::Note),
            "apiKey" => Ok(Self::ApiKey),
            "ssh" => Ok(Self::Ssh),
            other => Err(AppError::new(format!("Неизвестная категория: {other}"))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KdfParams {
    pub algorithm: String,
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
    pub hash_len: usize,
    pub salt: String,
}

impl KdfParams {
    pub fn defaults(salt: String) -> Self {
        Self {
            algorithm: "argon2id".to_string(),
            memory_kib: 65_536,
            iterations: 3,
            parallelism: 4,
            hash_len: 32,
            salt,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFields {
    pub category: Category,
    pub title: String,
    pub service: String,
    pub username: String,
    pub password: String,
    pub totp_secret: String,
    pub note: String,
    pub favorite: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemInput {
    pub id: Option<String>,
    #[serde(flatten)]
    pub fields: ItemFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultItem {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(flatten)]
    pub fields: ItemFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TotpInfo {
    pub code: String,
    pub seconds_remaining: u64,
    pub period: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSummary {
    pub id: String,
    pub category: Category,
    pub title: String,
    pub service: String,
    pub username: String,
    pub favorite: bool,
    pub tags: Vec<String>,
    pub has_password: bool,
    pub has_totp: bool,
    pub totp: Option<TotpInfo>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub exists: bool,
    pub unlocked: bool,
    pub auto_lock_minutes: u64,
    pub item_count: usize,
    pub kdf: Option<KdfParams>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ImportMode {
    Merge,
    Replace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub mode: ImportMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordOptions {
    pub length: usize,
    pub uppercase: bool,
    pub lowercase: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_similar: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassphraseOptions {
    pub words: usize,
    pub separator: String,
    pub capitalize: bool,
    pub append_digit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedSecret {
    pub value: String,
    pub entropy_bits: f64,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardStatus {
    pub expires_in_seconds: u64,
}
