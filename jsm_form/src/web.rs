use anyhow::{Context, Result};
use headless_chrome::{Browser, LaunchOptions, Tab};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

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
        let login_username = {
            let trimmed = self.config.auth.username.trim();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        };

        let is_on_correct_page = wait_for_ticket_page(
            &tab,
            &self.config.base_url,
            ticket_id,
            45,
            login_username,
        )?;
        
        if is_on_correct_page {
            info!("✅ Confirmed on correct ticket page: {}", ticket_id);

            self.open_risk_assessment_editor(&tab)?;

            if let Some(value) = &config.change_impact_assessment.security_controls_impact {
                info!("Setting Security Controls Impact to '{}'.", value);
                self.select_dropdown_option(
                    &tab,
                    &["security controls impact", "security impact", "security control impact"],
                    value,
                )?;
            } else {
                warn!("No Security Controls Impact value provided in configuration; skipping field update");
            }

            self.save_risk_assessment_changes(&tab)?;
            info!("Risk assessment updates submitted.");
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

    fn click_button_with_text(&self, tab: &Arc<Tab>, candidate_texts: &[&str]) -> Result<bool> {
        let texts_json = serde_json::to_string(candidate_texts)?;
        let script = format!(
            r#"(function() {{
                const targets = {}.map(t => t.toLowerCase().trim());
                const elements = Array.from(document.querySelectorAll('button, [role="button"], a[role="button"]'));
                for (const target of targets) {{
                    const match = elements.find(el => (el.innerText || el.textContent || '').trim().toLowerCase() === target);
                    if (match) {{
                        match.click();
                        return target;
                    }}
                }}
                return '';
            }})()"#,
            texts_json
        );

        let result = tab
            .evaluate(&script, false)
            .context("Failed to evaluate JavaScript to click button")?;
        let return_value = result.value.and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or_default();
        Ok(!return_value.is_empty())
    }

    fn open_risk_assessment_editor(&self, tab: &Arc<Tab>) -> Result<()> {
        info!("Opening risk assessment edit form...");
        let clicked = self.click_button_with_text(tab, &["Edit form", "Edit Form", "Edit risk assessment"])?;
        if clicked {
            std::thread::sleep(Duration::from_secs(2));
            Ok(())
        } else {
            error!("Failed to open risk assessment edit form...");
            Err(anyhow::anyhow!("Could not find the 'Edit form' button in the risk assessment section"))
        }
    }

    fn select_dropdown_option(&self, tab: &Arc<Tab>, field_keywords: &[&str], desired_value: &str) -> Result<()> {
        let keywords_json = serde_json::to_string(field_keywords)?;
        let value_json = serde_json::to_string(desired_value)?;

        let open_script = format!(
            r#"(function() {{
                const keywords = {}.map(k => k.toLowerCase());
                const allElements = Array.from(document.querySelectorAll('[aria-label], [data-testid], label, button, [role="combobox"], select, span, div'));
                function textFor(el) {{
                    return (el.getAttribute('aria-label') || el.getAttribute('data-testid') || el.innerText || el.textContent || '').trim().toLowerCase();
                }}
                let target = null;
                for (const el of allElements) {{
                    const text = textFor(el);
                    if (!text) continue;
                    if (keywords.some(k => text.includes(k))) {{
                        target = el;
                        break;
                    }}
                }}
                if (!target) {{
                    return "field-not-found";
                }}
                const clickable = target.matches('button, [role="button"], [role="combobox"], select') ? target : target.closest('button, [role="button"], [role="combobox"], select');
                if (!clickable) {{
                    return "clickable-not-found";
                }}
                clickable.click();
                return "clicked";
            }})()"#,
            keywords_json
        );

        let open_result = tab
            .evaluate(&open_script, false)
            .context("Failed to evaluate script to open dropdown")?;
        let open_status = open_result
            .value
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "".to_string());

        if open_status != "clicked" {
            return Err(anyhow::anyhow!(
                "Could not locate or open dropdown for field keywords {:?} (status: {})",
                field_keywords,
                open_status
            ));
        }

        std::thread::sleep(Duration::from_millis(750));

        let select_script = format!(
            r#"(function() {{
                const desired = {}.toLowerCase();
                const optionElements = Array.from(document.querySelectorAll('[role="option"], li[role="option"], select option'));
                for (const element of optionElements) {{
                    const text = (element.innerText || element.textContent || '').trim();
                    if (!text) continue;
                    if (text.toLowerCase() === desired) {{
                        element.click();
                        if (element instanceof HTMLOptionElement) {{
                            const select = element.parentElement;
                            if (select) {{
                                select.value = element.value;
                                select.dispatchEvent(new Event('change', {{ bubbles: true }}));
                            }}
                        }}
                        return "selected";
                    }}
                }}
                return "option-not-found";
            }})()"#,
            value_json
        );

        let select_result = tab
            .evaluate(&select_script, false)
            .context("Failed to evaluate script to pick dropdown option")?;
        let select_status = select_result
            .value
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "".to_string());

        if select_status != "selected" {
            return Err(anyhow::anyhow!(
                "Could not select value '{}' for field keywords {:?} (status: {})",
                desired_value,
                field_keywords,
                select_status
            ));
        }

        Ok(())
    }

    fn save_risk_assessment_changes(&self, tab: &Arc<Tab>) -> Result<()> {
        let clicked = self.click_button_with_text(tab, &["Save", "Update", "Done", "Close"])?;
        if clicked {
            info!("Clicked save/update button to submit risk assessment changes");
            std::thread::sleep(Duration::from_secs(2));
            Ok(())
        } else {
            Err(anyhow::anyhow!("Could not find a save/update button after editing the risk assessment"))
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
fn wait_for_ticket_page(
    tab: &Arc<Tab>,
    base_url: &str,
    ticket_id: &str,
    timeout_secs: u64,
    username: Option<&str>,
) -> Result<bool> {
    info!("Waiting to confirm we're on the correct ticket page...");
    let mut start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    let mut current_url = tab.get_url();
    let mut atlassian_username_done = false;
    let mut microsoft_username_done = false;

    while start_time.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(1000));
        let new_url = tab.get_url();
        info!("Check new URL: {}", new_url);

        if is_on_ticket_page(&new_url, base_url, ticket_id) {
            return Ok(true);
        }

        if let Some(user) = username {
            if new_url.starts_with("https://id.atlassian.com/") && new_url.contains("login") {
                if !atlassian_username_done {
                    match try_fill_atlassian_username(tab, user) {
                        Ok(true) => {
                            info!("Filled Atlassian username and triggered continue/login");
                            atlassian_username_done = true;
                            continue;
                        }
                        Ok(false) => {
                            info!("Atlassian username field not ready yet; will retry...");
                        }
                        Err(err) => {
                            warn!("Failed to auto-fill Atlassian username: {err:?}");
                            atlassian_username_done = true;
                        }
                    }
                }
            } else if new_url.starts_with("https://id.atlassian.com/") && new_url.contains("join/user-access") {
                match try_click_account_continue(tab, user) {
                    Ok(true) => {
                        info!("Detected matching Atlassian account; clicked Continue");
                        continue;
                    }
                    Ok(false) => {
                        info!("Account selection screen present but Continue button not clicked yet");
                    }
                    Err(err) => {
                        warn!("Failed to auto-continue Atlassian account selection: {err:?}");
                    }
                }
            } else if new_url.starts_with("https://login.microsoftonline.com/") {
                if !microsoft_username_done {
                    match try_fill_microsoft_username(tab, user) {
                        Ok(true) => {
                            info!("Filled Microsoft login username and pressed Next");
                            microsoft_username_done = true;
                            continue;
                        }
                        Ok(false) => {
                            info!("Microsoft login username field not ready yet; will retry...");
                        }
                        Err(err) => {
                            warn!("Failed to auto-fill Microsoft username: {err:?}");
                            microsoft_username_done = true;
                        }
                    }
                }
            }
        }

        if new_url != current_url {
            info!("URL changed; resetting timeout. URL: {}", new_url);
            current_url = new_url.clone();
            start_time = std::time::Instant::now();
        }
    }

    info!("Could not verify we're on the ticket page. Current URL: {}", current_url);
    Ok(false)
}

/// Attempt to fill the Atlassian username on the login page
/// Returns true if successful, false if the field wasn't found, or an error if something went wrong
fn try_fill_atlassian_username(tab: &Arc<Tab>, username: &str) -> Result<bool> {
    let username_json = serde_json::to_string(username)?;
    let script = format!(
        r#"(function() {{
            const usernameField = document.querySelector('input[name="username"], input#username, input[type="email"]');
            if (!usernameField) {{
                return "not-found";
            }}
            usernameField.focus();
            usernameField.value = {username};
            usernameField.dispatchEvent(new Event('input', {{ bubbles: true }}));
            usernameField.dispatchEvent(new Event('change', {{ bubbles: true }}));

            const nextButton = document.querySelector('button[type="submit"], button#login-submit, button[data-testid="next-button"]');
            if (nextButton) {{
                nextButton.click();
                return "filled-and-submitted";
            }}
            return "filled";
        }})()"#,
        username = username_json
    );

    let result = tab
        .evaluate(&script, false)
        .context("Failed to evaluate JavaScript to fill username")?;

    let status = result
        .value
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    Ok(matches!(status.as_str(), "filled" | "filled-and-submitted"))
}

/// Attempt to fill the Microsoft username on the login page
/// Returns true if successful, false if the field wasn't found, or an error if something went wrong
fn try_fill_microsoft_username(tab: &Arc<Tab>, username: &str) -> Result<bool> {
    let username_json = serde_json::to_string(username)?;
    let script = format!(
        r#"(function() {{
            const input = document.querySelector('input[name="loginfmt"], input#i0116, input[type="email"]');
            if (!input) {{
                return "not-found";
            }}
            input.focus();
            input.value = {username};
            input.dispatchEvent(new Event('input', {{ bubbles: true }}));
            input.dispatchEvent(new Event('change', {{ bubbles: true }}));

            const nextButton = document.querySelector('#idSIButton9, button[type="submit"], input[type="submit"]');
            if (nextButton) {{
                nextButton.click();
                return "submitted";
            }}
            return "filled";
        }})()"#,
        username = username_json
    );

    let result = tab
        .evaluate(&script, false)
        .context("Failed to evaluate JavaScript to fill Microsoft username")?;

    let status = result
        .value
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    Ok(matches!(status.as_str(), "filled" | "submitted"))
}

fn try_click_account_continue(tab: &Arc<Tab>, username: &str) -> Result<bool> {
    let username_json = serde_json::to_string(&username.to_lowercase())?;
    let script = format!(
        r#"(function() {{
            const email = {username};
            const bodyText = (document.body.innerText || document.body.textContent || '').toLowerCase();
            if (!bodyText.includes(email)) {{
                return "email-not-found";
            }}
            const buttons = Array.from(document.querySelectorAll('button'));
            const continueButton = buttons.find(btn => (btn.innerText || btn.textContent || '').trim().toLowerCase() === 'continue');
            if (continueButton) {{
                continueButton.click();
                return "clicked";
            }}
            return "button-not-found";
        }})()"#,
        username = username_json
    );

    let result = tab
        .evaluate(&script, false)
        .context("Failed to evaluate JavaScript to click account continue")?;

    let status = result
        .value
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    Ok(status == "clicked")
}