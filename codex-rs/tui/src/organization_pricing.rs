//! Optional owner-supplied model price estimates; never a provider billing cap.
use anyhow::Result;
use anyhow::ensure;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Prices {
    provider: String,
    currency: String,
    source: String,
    /// Conservative owner quote using the highest applicable token rate.
    usd_per_million_total_tokens: BTreeMap<String, f64>,
}
impl Prices {
    pub(crate) fn load(home: &Path) -> Result<Self> {
        let file = codex_utils_path::open_relative_regular_file(
            home,
            Path::new("organization-prices.json"),
        )?;
        let metadata = file.metadata()?;
        ensure!(metadata.len() <= 65536, "price file exceeds 64 KiB");
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            ensure!(
                metadata.mode() & 0o077 == 0 && metadata.uid() == unsafe { libc::geteuid() },
                "prices must be private and owner controlled"
            );
        }
        let mut bytes = Vec::new();
        file.take(65537).read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= 65536, "price file grew beyond 64 KiB");
        let prices: Self = serde_json::from_slice(&bytes)?;
        ensure!(
            prices.currency == "USD"
                && !prices.provider.is_empty()
                && !prices.source.is_empty()
                && prices.source.len() <= 512
                && prices.usd_per_million_total_tokens.len() <= 128,
            "invalid price provenance"
        );
        ensure!(
            prices
                .usd_per_million_total_tokens
                .values()
                .all(|rate| rate.is_finite() && *rate >= 0.0 && *rate <= 1_000_000.0),
            "invalid token rate"
        );
        Ok(prices)
    }
    pub(crate) fn reported_estimate(
        &self,
        provider: &str,
        model: &str,
        tokens: i64,
    ) -> Option<String> {
        if provider != self.provider || tokens < 0 {
            return None;
        }
        let rate = *self.usd_per_million_total_tokens.get(model)?;
        Some(format!(
            "Owner reported-token cost estimate: ${:.6} USD · {provider}/{model} · source {}. Excludes unreported usage/non-token charges; not billing.",
            tokens as f64 * rate / 1_000_000.0,
            self.source
        ))
    }
    pub(crate) fn estimate(
        &self,
        provider: &str,
        model: &str,
        tokens: i64,
        ceiling: i64,
    ) -> Option<String> {
        if provider != self.provider || tokens < 0 || ceiling < 0 {
            return None;
        }
        let rate = *self.usd_per_million_total_tokens.get(model)?;
        Some(format!(
            "Owner model cost estimate: ${:.6} USD reported; ${:.6} USD token-ceiling estimate\nPrice basis: {provider}/{model}, ${rate} per million total tokens\nSource: {}\nEstimates exclude unreported usage and non-token charges; not a billing cap.\n",
            tokens as f64 * rate / 1_000_000.0,
            ceiling as f64 * rate / 1_000_000.0,
            self.source
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn write(home: &Path, rate: f64) {
        let p = home.join("organization-prices.json");
        std::fs::write(&p, serde_json::json!({"provider":"local","currency":"USD","source":"Owner test quote, not public provider pricing","usd_per_million_total_tokens":{"model":rate}}).to_string()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
    #[test]
    fn owner_estimate_preserves_provider_model_and_quote_provenance() {
        let home = tempfile::tempdir().unwrap();
        write(home.path(), 2.0);
        let prices = Prices::load(home.path()).unwrap();
        let estimate = prices
            .estimate("local", "model", 500_000, 1_000_000)
            .unwrap();
        assert!(estimate.contains("$1.000000 USD reported"));
        assert!(estimate.contains("$2.000000 USD token-ceiling estimate"));
        assert!(estimate.contains("Owner test quote"));
        assert!(prices.estimate("other", "model", 1, 2).is_none());
        assert!(prices.estimate("local", "other", 1, 2).is_none());
        assert!(prices.estimate("local", "model", -1, 2).is_none());
        write(home.path(), -1.0);
        assert!(Prices::load(home.path()).is_err());
    }
    #[test]
    fn unsafe_price_file_cannot_supply_a_quote() {
        let home = tempfile::tempdir().unwrap();
        assert!(Prices::load(home.path()).is_err());
        write(home.path(), 0.0);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let p = home.path().join("organization-prices.json");
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(Prices::load(home.path()).is_err());
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600)).unwrap();
            std::fs::rename(&p, home.path().join("actual-prices")).unwrap();
            std::os::unix::fs::symlink(home.path().join("actual-prices"), &p).unwrap();
            assert!(Prices::load(home.path()).is_err());
            std::fs::remove_file(&p).unwrap();
            std::fs::hard_link(home.path().join("actual-prices"), &p).unwrap();
            assert!(Prices::load(home.path()).is_err());
        }
    }
}
