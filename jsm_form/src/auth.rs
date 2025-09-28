use crate::AuthConfig;
use anyhow::{Context, Result};
use reqwest::Client;

/// Authenticate with the JSM instance using HTTP Basic Authentication
/// This method validates the credentials by making a test API call to the service desk
pub async fn authenticate(client: &Client, auth: &AuthConfig, base_url: &str) -> Result<()> {
    // For Atlassian Cloud instances, we use HTTP Basic Authentication with email:api_token
    // Test authentication by making a simple API call to get service desk info
    let test_url = format!("{}/rest/servicedeskapi/servicedesk", base_url);

    let response = client
        .get(&test_url)
        .basic_auth(&auth.username, Some(&auth.token_atlassian_api))
        .send()
        .await
        .context("Failed to test authentication")?;

    if response.status().is_success() {
        tracing::info!("Authentication successful");
        Ok(())
    } else {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();

        if status == 401 {
            Err(anyhow::anyhow!(
                "Authentication failed: Invalid credentials. Make sure you're using:\n\
                - Email address as username\n\
                - API token as password (not your account password)\n\
                Create an API token at: https://id.atlassian.com/manage-profile/security/api-tokens"
            ))
        } else if status == 403 {
            Err(anyhow::anyhow!(
                "Authentication successful but access denied. You may not have permission to access this service desk."
            ))
        } else {
            Err(anyhow::anyhow!(
                "Authentication failed with status: {} - {}",
                status,
                error_body
            ))
        }
    }
}
