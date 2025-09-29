//! JSM Form Automation Library
//!
//! This library provides functionality to automate completion of JSM (Jira Service Management) web forms.

pub mod auth;
pub mod config;
pub mod error;
pub mod form;
pub mod logging;
pub mod web;

use anyhow::Result;
use serde::{Deserialize, Serialize};

// Re-export web automation types
pub use web::{ChangeImpactAssessmentConfig, ChangeRiskAssessmentConfig, RiskAssessmentConfig};

/// Configuration for the JSM form automation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsmConfig {
    /// Organization name (used to construct Atlassian URLs)
    pub org: String,
    /// Base URL of the JSM instance
    pub base_url: String,
    /// Portal ID
    pub portal_id: u32,
    /// Request type ID
    pub request_type_id: u32,
    /// Authentication credentials
    pub auth: AuthConfig,
}

/// Authentication configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    /// Username for authentication
    pub username: String,
    /// API Token for authentication
    pub token_atlassian_api: String,
    /// Password used for Microsoft login flow
    #[serde(default)]
    pub microsoft_password: String,
}

/// Form data to be submitted
#[derive(Debug, Deserialize, Serialize)]
pub struct FormData {
    /// Map of field names to values (supports strings, arrays, objects, etc.)
    pub fields: std::collections::HashMap<String, serde_json::Value>,
}

/// Main JSM form client
pub struct JsmFormClient {
    config: JsmConfig,
    client: reqwest::Client,
}

impl JsmFormClient {
    /// Create a new JSM form client
    pub fn new(config: JsmConfig) -> Self {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Authenticate with the JSM instance
    pub async fn authenticate(&self) -> Result<()> {
        auth::authenticate(&self.client, &self.config.auth, &self.config.base_url).await
    }

    /// Submit form data to the JSM form
    pub async fn submit_form(&self, form_data: FormData) -> Result<()> {
        form::submit_form(&self.client, &self.config, form_data).await
    }
}
