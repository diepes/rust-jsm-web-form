use anyhow::{Context, Result};
use headless_chrome::{Browser, LaunchOptions, Tab};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, debug, warn};

use crate::JsmConfig;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskAssessmentConfig {
    pub change_impact_assessment: ChangeImpactAssessmentConfig,
    pub change_risk_assessment: Option<ChangeRiskAssessmentConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangeImpactAssessmentConfig {
    pub security_controls_impact: Option<String>,
    pub performance_impact: Option<String>,
    pub availability_impact: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangeRiskAssessmentConfig {
    // Placeholder for future expansion
}

pub struct JsmWebClient {
    config: JsmConfig,
    browser: Option<Browser>,
}

impl JsmWebClient {
    pub fn new(config: JsmConfig) -> Self {
        Self {
            config,
            browser: None,
        }
    }

    fn get_tab(&mut self) -> Result<Arc<Tab>> {
        info!("Initializing browser...");
        if self.browser.is_none() {
            let browser = Browser::new(LaunchOptions::default_builder()
                .headless(false)  // Set to false for debugging
                .build()
                .context("Failed to build launch options")?)?;
            self.browser = Some(browser);
        }

        let browser = self.browser.as_ref().unwrap();
        let tab = browser.new_tab()?;
        
        // info!("Navigating to login page...");
        // let login_url = format!("{}/login.jsp", self.config.base_url);
        // tab.navigate_to(&login_url)?;
        // tab.wait_until_navigated()?;
        
        Ok(tab)
    }

    pub fn complete_risk_assessment(&mut self, ticket_id: &str, config: &RiskAssessmentConfig) -> Result<()> {
        info!("Starting risk assessment for ticket: {}", ticket_id);
        let tab = self.get_tab()?;
        
        info!("Navigating to ticket page...");
        let ticket_url = format!("{}/browse/{}", self.config.base_url, ticket_id);
        tab.navigate_to(&ticket_url)?;
        tab.wait_until_navigated()?;
        
        // Verify we're on the correct ticket page with a 30-second timeout
        info!("Verifying ticket page URL...");
        let is_on_correct_page = wait_for_ticket_page(&tab, &self.config.base_url, ticket_id, 45)?;
        
        if is_on_correct_page {
            info!("✅ Confirmed on correct ticket page: {}", ticket_id);
            info!("Pausing for 15 seconds to allow manual inspection of the ticket page...");
            std::thread::sleep(Duration::from_secs(15));
            
            // Simple placeholder implementation
            if let Some(value) = &config.change_impact_assessment.security_controls_impact {
                info!("Would set security controls impact to: {}", value);
            }
            
            info!("Risk assessment completed successfully");
            Ok(())
        } else {
            // We couldn't verify we're on the correct ticket page
            let current_url = tab.get_url();
            Err(anyhow::anyhow!(
                "Could not verify we're on the correct ticket page for {}.\nCurrent URL: {}\n\
                This may be due to a login page or other redirect.\n\
                Please try again after ensuring you're logged in.",
                ticket_id, current_url
            ))
        }
    }
}

/// Module-level function to complete risk assessment
pub fn complete_risk_assessment(config: &JsmConfig, ticket_id: &str, risk_config: &RiskAssessmentConfig) -> Result<()> {
    let mut client = JsmWebClient::new(config.clone());
    client.complete_risk_assessment(ticket_id, risk_config)
}

/// Check if we're on the expected ticket page
/// Returns true if the current URL matches the expected ticket URL pattern
fn is_on_ticket_page(url: &str, _base_url: &str, ticket_id: &str) -> bool {
    // This handles redirect variations but ensures the ticket ID is present
    url.contains(&format!("/browse/{}", ticket_id))
}

/// Wait until we're on the expected ticket page or timeout
fn wait_for_ticket_page(tab: &Arc<Tab>, base_url: &str, ticket_id: &str, timeout_secs: u64) -> Result<bool> {
    info!("Waiting to confirm we're on the correct ticket page...");
    let mut start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    
    // Check if we need to handle Microsoft login
    let mut current_url = tab.get_url();
    
    while start_time.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(1000));
        let new_url = tab.get_url();
            info!("Check new URL: {}", new_url);
        // First check if we've reached our destination
        if is_on_ticket_page(&new_url, base_url, ticket_id) {
            return Ok(true);
        }
        if new_url != current_url {
            info!("URL changed reset timeout. URL: {}", new_url);
            current_url = new_url.clone();
            start_time = std::time::Instant::now(); // Reset timeout on URL change
        }
        
    }
    // If we got here, we timed out
    info!("Could not verify we're on the ticket page. Current URL: {}", current_url);
    Ok(false)
}