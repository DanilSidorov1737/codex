//! Inventory reads definitions and canonical observations; never connects or probes.
use anyhow::Result;
use anyhow::ensure;
use codex_core::config::Config;
use codex_model_provider_info::ModelProviderInfo;
use codex_protocol::organization_execution::OrganizationProviderEvidence;
use codex_protocol::organization_inventory::*;
use codex_state::organization::OrganizationStore;
use sha2::Digest;
use sha2::Sha256;

pub(crate) fn provider_evidence(
    id: &str,
    provider: &ModelProviderInfo,
) -> Result<OrganizationProviderEvidence> {
    let mut endpoint = provider
        .base_url
        .as_deref()
        .map(url::Url::parse)
        .transpose()?;
    if let Some(endpoint) = &mut endpoint {
        ensure!(
            matches!(endpoint.scheme(), "http" | "https") && endpoint.host_str().is_some(),
            "invalid provider endpoint"
        );
        let _ = endpoint.set_username("");
        let _ = endpoint.set_password(None);
        endpoint.set_query(None);
        endpoint.set_fragment(None);
    }
    let origin = endpoint
        .as_ref()
        .map(|url| url.origin().ascii_serialization());
    let basis = serde_json::to_vec(&(
        id,
        endpoint.as_ref().map(url::Url::as_str),
        provider.wire_api.to_string(),
        provider.requires_openai_auth,
    ))?;
    Ok(OrganizationProviderEvidence {
        id: id.into(),
        origin,
        endpoint_sha256: format!("{:x}", Sha256::digest(basis)),
    })
}

pub(crate) async fn inventory(
    config: &Config,
    store: &OrganizationStore,
) -> Result<OrganizationInventory> {
    let mut providers = Vec::new();
    for (id, provider) in &config.model_providers {
        let provider = if id == &config.model_provider_id {
            &config.model_provider
        } else {
            provider
        };
        let evidence = provider_evidence(id, provider)?;
        let observation = store.provider_observations(&evidence).await?;
        providers.push(OrganizationProviderCapability {
            id:id.clone(),selected:id==&config.model_provider_id,origin:evidence.origin,
            credential_requirement:provider.env_key.as_ref().map(|key|format!("Configured environment binding: {key}; values omitted")).unwrap_or_else(||if provider.requires_openai_auth {"Codex owner authentication; credentials remain on the host"} else {"Provider-specific host authentication; configuration does not prove readiness"}.into()),
            qualification:if observation.checked_results>0 {"checkedResultObserved"} else if observation.reported_model_requests>0 {"usageObserved"} else if observation.model_requests>0 {"requestRecorded"} else {"notTested"}.into(),
            observation,
        });
    }
    providers.sort_by(|a, b| a.id.cmp(&b.id));
    let mut tools = store.tool_observations().await?;
    for tool in &mut tools {
        tool.permissions=match tool.name.as_str() {
        "exec_command"=>"Explicit task grant, isolated filesystem and exact approval unless preauthorized; no agent network",
        "apply_patch"=>"Explicit output paths and native action approval; original project Apply is owner-only",
        _=>"Current task grant and host-enforced project/agent/roster boundaries; context cannot grant permissions",
    }.into();
    }
    let mut mcp_servers = Vec::new();
    for (name, server) in config.mcp_servers.get() {
        mcp_servers.push(OrganizationMcpCapability {name:name.clone(),enabled:server.enabled,available_to_tasks:false,qualification:"notTested".into(),permissions:"Not admitted to bounded organization tasks. Separate integration qualification and explicit authority are required.".into()});
    }
    mcp_servers.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(OrganizationInventory {providers,tools,mcp_servers,limitations:"Recorded observations qualify only their captured endpoint/model and native checks. Configuration alone does not prove compatibility. No credential values, headers, command arguments or endpoint paths/queries are returned. No probes, installs, fallback or external access are performed.".into()})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_evidence_redacts_credentials_and_detects_endpoint_changes() -> Result<()> {
        let mut provider = ModelProviderInfo {
            base_url: Some("https://username:password@example.com/v1?token=SECRET#fragment".into()),
            ..Default::default()
        };
        let evidence = provider_evidence("fixture", &provider)?;
        assert_eq!(evidence.origin.as_deref(), Some("https://example.com"));
        let json = serde_json::to_string(&evidence)?;
        for secret in ["username", "password", "SECRET", "fragment", "/v1"] {
            assert!(!json.contains(secret));
        }
        provider.base_url =
            Some("https://different:credential@example.com/v1?token=DIFFERENT".into());
        assert_eq!(
            provider_evidence("fixture", &provider)?,
            evidence,
            "credential changes must not appear in endpoint evidence"
        );
        provider.base_url = Some("https://example.com/v2".into());
        assert_ne!(
            provider_evidence("fixture", &provider)?.endpoint_sha256,
            evidence.endpoint_sha256
        );
        provider.base_url = Some("file:///tmp/private".into());
        assert!(provider_evidence("fixture", &provider).is_err());
        Ok(())
    }
}
