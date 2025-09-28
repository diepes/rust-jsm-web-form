use anyhow::{Context, Result, anyhow};
use headless_chrome::{Browser, LaunchOptions, Tab};
use serde_json::to_string;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::JsmConfig;

use super::login;
use super::types::RiskAssessmentConfig;

pub struct JsmWebClient {
    config: JsmConfig,
    browser: Option<Browser>,
    tab: Option<Arc<Tab>>,
    count_nav: usize,
}

impl JsmWebClient {
    pub fn new(config: JsmConfig) -> Self {
        Self {
            config,
            browser: None,
            tab: None,
            count_nav: 0,
        }
    }

    fn get_tab(&mut self) -> Result<Arc<Tab>> {
        if let Some(tab) = &self.tab {
            return Ok(Arc::clone(tab));
        }

        info!("Initializing browser...");
        if self.browser.is_none() {
            let browser = Browser::new(
                LaunchOptions::default_builder()
                    .headless(false)
                    .build()
                    .context("Failed to build launch options")?,
            )?;
            self.browser = Some(browser);
        }

        let browser = self.browser.as_ref().unwrap();
        let tab = browser.new_tab()?;
        self.tab = Some(Arc::clone(&tab));

        Ok(tab)
    }

    fn tab(&self) -> Result<Arc<Tab>> {
        self.tab.as_ref().cloned().context(
            "Browser tab not initialized. Call get_tab() before interacting with the page.",
        )
    }

    pub fn complete_risk_assessment(
        &mut self,
        ticket_id: &str,
        config: &RiskAssessmentConfig,
    ) -> Result<()> {
        info!("Starting risk assessment for ticket: {}", ticket_id);
        let tab = self.get_tab()?;

        let ticket_url = format!("{}/browse/{}", self.config.base_url, ticket_id);
        self.count_nav += 1;
        info!("Navigating #{} to: {}", self.count_nav, ticket_url);
        tab.navigate_to(&ticket_url)?;
        tab.wait_until_navigated()?;

        info!("Verifying ticket page URL...");
        let login_username = {
            let trimmed = self.config.auth.username.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        };

        let is_on_correct_page = login::wait_for_ticket_page(
            &tab,
            &self.config.base_url,
            ticket_id,
            45,
            login_username,
        )?;

        if is_on_correct_page {
            info!("✅ Confirmed on correct ticket page: {}", ticket_id);

            self.open_risk_assessment_editor()?;

            if let Some(value) = &config.change_impact_assessment.security_controls_impact {
                info!("Setting Security Controls Impact to '{}'.", value);
                self.select_dropdown_option(
                    &[
                        "security controls impact",
                        "security impact",
                        "security control impact",
                    ],
                    value,
                )?;
            } else {
                warn!(
                    "No Security Controls Impact value provided in configuration; skipping field update"
                );
            }

            self.save_risk_assessment_changes()?;
            info!("Risk assessment updates submitted.");
            Ok(())
        } else {
            let current_url = tab.get_url();
            Err(anyhow!(
                "Could not verify we're on the correct ticket page for {}.\nCurrent URL: {}\n\
                This may be due to a login page or other redirect.\n\
                Please try again after ensuring you're logged in.",
                ticket_id,
                current_url
            ))
        }
    }

    fn click_button_with_text(&self, candidate_texts: &[&str]) -> Result<bool> {
        let tab = self.tab()?;
        let texts_json = to_string(candidate_texts)?;
        let script = format!(
            r#"(function() {{
                const targets = {}.map(t => t.toLowerCase().trim());
                const elements = Array.from(document.querySelectorAll('button, [role=\"button\"], a[role=\"button\"]'));
                for (const target of targets) {{
                    const match = elements.find(el => (el.innerText || el.textContent || '').trim().toLowerCase() === target);
                    if (match) {{
                        match.click();
                        return target;
                    }}
                }}
                return '';
            }})"#,
            texts_json
        );

        let result = tab
            .evaluate(&script, false)
            .context("Failed to evaluate JavaScript to click button")?;
        let return_value = result
            .value
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        Ok(!return_value.is_empty())
    }

    fn open_risk_assessment_editor(&self) -> Result<()> {
        info!("Opening risk assessment edit form...");
        let clicked =
            self.click_button_with_text(&["Edit form", "Edit Form", "Edit risk assessment"])?;
        if clicked {
            thread::sleep(Duration::from_secs(2));
            Ok(())
        } else {
            error!("Failed to open risk assessment edit form...");
            Err(anyhow!(
                "Could not find the 'Edit form' button in the risk assessment section"
            ))
        }
    }

    fn select_dropdown_option(&self, field_keywords: &[&str], desired_value: &str) -> Result<()> {
        let tab = self.tab()?;
        let keywords_json = to_string(field_keywords)?;
        let value_json = to_string(desired_value)?;

        let open_script = format!(
            r#"(function() {{
                const keywords = {}.map(k => k.toLowerCase());
                const allElements = Array.from(document.querySelectorAll('[aria-label], [data-testid], label, button, [role=\"combobox\"], select, span, div'));
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
                const clickable = target.matches('button, [role=\"button\"], [role=\"combobox\"], select') ? target : target.closest('button, [role=\"button\"], [role=\"combobox\"], select');
                if (!clickable) {{
                    return "clickable-not-found";
                }}
                clickable.click();
                return "clicked";
            }})"#,
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
            return Err(anyhow!(
                "Could not locate or open dropdown for field keywords {:?} (status: {})",
                field_keywords,
                open_status
            ));
        }

        thread::sleep(Duration::from_millis(750));

        let select_script = format!(
            r#"(function() {{
                const desired = {}.toLowerCase();
                const optionElements = Array.from(document.querySelectorAll('[role=\"option\"], li[role=\"option\"], select option'));
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
            }})"#,
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
            return Err(anyhow!(
                "Could not select value '{}' for field keywords {:?} (status: {})",
                desired_value,
                field_keywords,
                select_status
            ));
        }

        Ok(())
    }

    fn save_risk_assessment_changes(&self) -> Result<()> {
        let clicked = self.click_button_with_text(&["Save", "Update", "Done", "Close"])?;
        if clicked {
            info!("Clicked save/update button to submit risk assessment changes");
            thread::sleep(Duration::from_secs(2));
            Ok(())
        } else {
            Err(anyhow!(
                "Could not find a save/update button after editing the risk assessment"
            ))
        }
    }
}

pub fn complete_risk_assessment(
    config: &JsmConfig,
    ticket_id: &str,
    risk_config: &RiskAssessmentConfig,
) -> Result<()> {
    let mut client = JsmWebClient::new(config.clone());
    client.complete_risk_assessment(ticket_id, risk_config)
}
