use crate::{FormData, JsmConfig};
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Remove any keys that are known to be configuration-only or not valid for the JSM REST API.
fn sanitize_request_fields(
    mut fields: std::collections::HashMap<String, serde_json::Value>,
) -> std::collections::HashMap<String, serde_json::Value> {
    // Keys we know should not be sent to the API
    const CONFIG_KEYS: [&str; 1] = ["risk_assessment"]; // extend as needed
    for k in CONFIG_KEYS {
        fields.remove(k);
    }
    fields
}

/// Submit form data to the JSM service desk using the REST API
pub async fn submit_form(client: &Client, config: &JsmConfig, form_data: FormData) -> Result<()> {
    // Use the Atlassian Service Desk REST API to create a customer request
    let create_request_url = format!("{}/rest/servicedeskapi/request", config.base_url);

    // Prepare the request payload according to Atlassian API format
    let cleaned_fields = sanitize_request_fields(form_data.fields);
    let request_payload = CreateRequestPayload {
        service_desk_id: config.portal_id,
        request_type_id: config.request_type_id,
        request_field_values: cleaned_fields,
        raise_on_behalf_of: None, // Current user
    };

    tracing::info!("Creating service desk request via API...");

    let response = client
        .post(&create_request_url)
        .basic_auth(&config.auth.username, Some(&config.auth.password))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&request_payload)
        .send()
        .await
        .context("Failed to submit service desk request")?;

    if response.status().is_success() {
        let response_body: CreateRequestResponse =
            response.json().await.context("Failed to parse response")?;

        tracing::info!("Service desk request created successfully!");
        tracing::info!("Request ID: {}", response_body.issue_key);
        tracing::info!(
            "Request URL: {}/browse/{}",
            config.base_url,
            response_body.issue_key
        );
        Ok(())
    } else {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();

        tracing::error!("Request creation failed with status: {}", status);
        tracing::error!("Error details: {}", error_body);

        if status == 400 {
            Err(anyhow::anyhow!(
                "Bad request: Check that your portal_id ({}) and request_type_id ({}) are correct, and that all required fields are provided.\nError details: {}",
                config.portal_id,
                config.request_type_id,
                error_body
            ))
        } else if status == 401 {
            Err(anyhow::anyhow!(
                "Authentication failed. Make sure you're using a valid API token as password."
            ))
        } else if status == 403 {
            Err(anyhow::anyhow!(
                "Access denied. You may not have permission to create requests in this service desk."
            ))
        } else {
            Err(anyhow::anyhow!(
                "Request creation failed with status: {} - {}",
                status,
                error_body
            ))
        }
    }
}

/// Payload for creating a service desk request via REST API
#[derive(Debug, Serialize)]
struct CreateRequestPayload {
    #[serde(rename = "serviceDeskId")]
    service_desk_id: u32,
    #[serde(rename = "requestTypeId")]
    request_type_id: u32,
    #[serde(rename = "requestFieldValues")]
    request_field_values: std::collections::HashMap<String, serde_json::Value>,
    #[serde(rename = "raiseOnBehalfOf", skip_serializing_if = "Option::is_none")]
    raise_on_behalf_of: Option<String>,
}

/// Response from creating a service desk request
#[derive(Debug, Deserialize)]
struct CreateRequestResponse {
    #[serde(rename = "issueId")]
    issue_id: String,
    #[serde(rename = "issueKey")]
    issue_key: String,
    #[serde(rename = "requestTypeId")]
    request_type_id: String,
    #[serde(rename = "serviceDeskId")]
    service_desk_id: String,
}
