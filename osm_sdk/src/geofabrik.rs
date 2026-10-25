//! Geofabrik access: published MD5 checksums, snapshot versions, downloads.
//!
//! Version semantics (verified against the live service): `*-latest.osm.pbf`
//! is a stable alias that 307-redirects to a dated immutable file
//! (`california-261024.osm.pbf`, YYMMDD). The dated filename is the
//! de-facto version string. Checksum files sit next to the archive in
//! md5sum text format: `<32-hex>  <filename>` (filename may be the dated
//! variant, not the `-latest` one).

use crate::error::SdkError;
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Parse an md5sum-format file: returns the lowercase hex digest.
/// Accepts `\n`-terminated or bare text, ` *` or two-space separators,
/// and the dated-filename variant.
pub fn parse_md5_file(text: &str) -> Result<String, SdkError> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .ok_or_else(|| SdkError::Other(anyhow::anyhow!("empty md5 file")))?;
    let hex = line
        .split_whitespace()
        .next()
        .ok_or_else(|| SdkError::Other(anyhow::anyhow!("md5 file has no digest")))?;
    let lower = hex.to_ascii_lowercase();
    if lower.len() != 32 || !lower.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
        return Err(SdkError::Other(anyhow::anyhow!(
            "md5 digest is not 32 hex chars: {hex:?}"
        )));
    }
    Ok(lower)
}

/// Extract the `YYMMDD` version from a dated Geofabrik filename/URL.
/// Returns `None` for undated (`-latest`) names.
pub fn version_from_dated_filename(location: &str) -> Option<String> {
    const DIGITS: &[u8] = b"0123456789";
    let marker = ".osm.pbf";
    let idx = location.rfind(marker)?;
    if idx < 6 {
        return None;
    }
    let candidate = &location[idx - 6..idx];
    if !candidate.bytes().all(|c| DIGITS.contains(&c)) {
        return None;
    }
    Some(candidate.to_string())
}

/// Shared exponential back-off schedule: 1s, 2s, 4s, ... capped at 30s.
pub(crate) fn backoff_delays(retries: u32) -> Vec<Duration> {
    (0..retries)
        .map(|i| Duration::from_secs(1u64 << i.min(5)))
        .map(|d| d.min(Duration::from_secs(30)))
        .collect()
}

pub struct Geofabrik {
    /// Follows redirects — for checksum fetches and downloads.
    http: reqwest::Client,
    /// Does not follow redirects — reads the `Location` header for the
    /// current dated snapshot.
    no_redirect: reqwest::Client,
    /// Test/mirror hook: replace this URL prefix in every requested URL.
    url_rewrite: Option<(String, String)>,
}

impl Geofabrik {
    pub fn new() -> Self {
        let ua = concat!("logos-osm/", env!("CARGO_PKG_VERSION"));
        Self {
            http: reqwest::Client::builder()
                .user_agent(ua)
                .timeout(Duration::from_secs(600))
                .build()
                .expect("static client config"),
            no_redirect: reqwest::Client::builder()
                .user_agent(ua)
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(30))
                .build()
                .expect("static client config"),
            url_rewrite: None,
        }
    }

    /// Point every Geofabrik URL at another base (wiremock in tests, a
    /// mirror in front of a flaky link). Only the prefix is swapped.
    pub fn set_url_rewrite(&mut self, from: impl Into<String>, to: impl Into<String>) {
        self.url_rewrite = Some((from.into(), to.into()));
    }

    fn resolve(&self, url: &str) -> String {
        match &self.url_rewrite {
            Some((from, to)) if url.starts_with(from.as_str()) => {
                format!("{to}{}", &url[from.len()..])
            }
            _ => url.to_string(),
        }
    }

    /// Geofabrik's published MD5 for the snapshot at `pbf_url`.
    pub async fn fetch_checksum(&self, pbf_url: &str) -> Result<String, SdkError> {
        let url = self.resolve(&crate::regions::md5_url(pbf_url));
        let text = self
            .http
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        parse_md5_file(&text)
    }

    /// Current snapshot version for `pbf_url`: the `YYMMDD` dated filename
    /// the `-latest` alias redirects to. Falls back to the `Last-Modified`
    /// date (`yyyymmdd`) when no dated name is present.
    pub async fn current_version(&self, pbf_url: &str) -> Result<String, SdkError> {
        let resp = self.no_redirect.get(self.resolve(pbf_url)).send().await?;
        if let Some(loc) = resp.headers().get(reqwest::header::LOCATION) {
            if let Some(v) = version_from_dated_filename(loc.to_str().unwrap_or("")) {
                return Ok(v);
            }
        }
        if resp.status().is_success() {
            if let Some(v) = version_from_dated_filename(resp.url().path()) {
                return Ok(v);
            }
            // 200 without a dated name: the response itself carries
            // Last-Modified — no separate HEAD needed.
            if let Some(lm) = resp.headers().get(reqwest::header::LAST_MODIFIED) {
                if let Ok(s) = lm.to_str() {
                    if let Some(v) = version_from_http_date(s) {
                        return Ok(v);
                    }
                }
            }
        }
        Err(SdkError::Other(anyhow::anyhow!(
            "cannot determine snapshot version for {pbf_url}"
        )))
    }

    /// Stream `url` to `dest`, retrying transient failures with exponential
    /// back-off. Returns the byte count.
    pub async fn download(&self, url: &str, dest: &Path, retries: u32) -> Result<u64, SdkError> {
        let delays = backoff_delays(retries);
        let mut last_err: Option<SdkError> = None;
        for (attempt, delay) in delays.into_iter().enumerate() {
            if attempt > 0 {
                tokio::time::sleep(delay).await;
            }
            match self.download_once(&self.resolve(url), dest).await {
                Ok(n) => return Ok(n),
                Err(e) => {
                    tracing::warn!(attempt, url, error = %e, "download attempt failed");
                    last_err = Some(e);
                }
            }
        }
        let e = last_err.expect("at least one attempt");
        Err(SdkError::RetriesExhausted {
            op: "geofabrik download",
            retries,
            message: e.to_string(),
        })
    }

    async fn download_once(&self, url: &str, dest: &Path) -> Result<u64, SdkError> {
        let resp = self.http.get(url).send().await?.error_for_status()?;
        let mut file = tokio::fs::File::create(dest).await?;
        let mut n: u64 = 0;
        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            n += chunk.len() as u64;
        }
        file.flush().await?;
        Ok(n)
    }
}

impl Default for Geofabrik {
    fn default() -> Self {
        Self::new()
    }
}

/// `Sun, 25 Oct 2026 00:51:09 GMT` → `20261025`.
pub(crate) fn version_from_http_date(s: &str) -> Option<String> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }
    let day = parts[1];
    let mon = MONTHS.iter().position(|m| *m == parts[2])? + 1;
    let year = parts[3];
    if year.len() != 4
        || !year.bytes().all(|c| c.is_ascii_digit())
        || !day.bytes().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    Some(format!("{year}{mon:02}{:0>2}", day.parse::<u32>().unwrap_or(0)))
}

/// MD5 of a file, lowercase hex (import-time verification).
pub async fn md5_file(path: &Path) -> Result<String, SdkError> {
    use md5::Digest;
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = md5::Md5::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// SHA-256 of a file, lowercase hex (storage integrity cross-checks).
pub async fn sha256_file(path: &Path) -> Result<String, SdkError> {
    use sha2::Digest;
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_md5_file_variants() {
        assert_eq!(
            parse_md5_file("d72ae8dde254945f12adc92386fde236  berlin-latest.osm.pbf\n").unwrap(),
            "d72ae8dde254945f12adc92386fde236"
        );
        assert_eq!(
            parse_md5_file("35cff894271b8f56ddb81911dcffe089  california-261024.osm.pbf").unwrap(),
            "35cff894271b8f56ddb81911dcffe089"
        );
        assert_eq!(
            parse_md5_file("35cff894271b8f56ddb81911dcffe089 *california-latest.osm.pbf\n").unwrap(),
            "35cff894271b8f56ddb81911dcffe089"
        );
        assert!(parse_md5_file("nothex  file").is_err());
        assert!(parse_md5_file("").is_err());
    }

    #[test]
    fn version_from_dated_filename() {
        assert_eq!(
            super::version_from_dated_filename(
                "https://download.geofabrik.de/north-america/us/california-261024.osm.pbf"
            ),
            Some("261024".into())
        );
        assert_eq!(
            super::version_from_dated_filename(
                "https://download.geofabrik.de/north-america/us/california-latest.osm.pbf"
            ),
            None
        );
    }

    #[test]
    fn http_date_parsing() {
        assert_eq!(
            version_from_http_date("Sun, 25 Oct 2026 00:51:09 GMT"),
            Some("20261025".into())
        );
        assert_eq!(version_from_http_date("garbage"), None);
    }

    #[test]
    fn backoff_shape() {
        let d: Vec<u64> = backoff_delays(4).iter().map(|d| d.as_secs()).collect();
        assert_eq!(d, vec![1, 2, 4, 8]);
        assert_eq!(backoff_delays(8).last().unwrap().as_secs(), 30);
    }
}
