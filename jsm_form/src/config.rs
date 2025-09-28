use crate::{AuthConfig, JsmConfig};
use anyhow::Result;
use std::path::Path;

/// Load configuration from a file
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<JsmConfig> {
    let contents = std::fs::read_to_string(path)?;
    let config: JsmConfig = toml::from_str(&contents)?;
    Ok(config)
}

/// Save configuration to a file
pub fn save_config<P: AsRef<Path>>(config: &JsmConfig, path: P) -> Result<()> {
    let contents = toml::to_string_pretty(config)?;
    std::fs::write(path, contents)?;
    Ok(())
}

/// Create a default configuration template
pub fn create_default_config() -> JsmConfig {
    JsmConfig {
        org: "your-organization".to_string(),
        base_url: "https://your-organization.atlassian.net".to_string(),
        portal_id: 6,
        request_type_id: 73,
        auth: AuthConfig {
            username: "".to_string(),
            token_atlassian_api: "".to_string(),
            microsoft_password: "".to_string(),
        },
    }
}
