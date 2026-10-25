//! Shared error type for the OSM SDK.

#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("region not in predefined set: {0}")]
    UnknownRegion(String),

    #[error("geofabrik index does not resolve region {0}")]
    IndexMismatch(String),

    #[error("regions overlap: {0} is contained in {1}")]
    Overlap(String, String),

    #[error("checksum mismatch for {region}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        region: String,
        expected: String,
        actual: String,
    },

    #[error("registry error {code}: {message}")]
    Registry { code: u32, message: String },

    #[error("registry validation failed for {region}: code {code}")]
    InvalidEntry { region: String, code: u32 },

    #[error("batch too big: {0} > 72")]
    BatchTooBig(usize),

    #[error("{op} failed after {retries} retries: {message}")]
    RetriesExhausted {
        op: &'static str,
        retries: u32,
        message: String,
    },

    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl SdkError {
    pub fn registry(code: u32, message: impl Into<String>) -> Self {
        Self::Registry { code, message: message.into() }
    }
}
