use anyhow::{Context, Result, anyhow};
use headless_chrome::{Browser, LaunchOptions, Tab, browser::tab::ModifierKey};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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
        // Save sessions data to persist logins across runs
        let user_data_path = Some(PathBuf::from("./chrome_session_data_pvt"));
        crate::log_info!("Initializing browser...");
        if self.browser.is_none() {
            let browser = Browser::new(
                LaunchOptions::default_builder()
                    .headless(false)
                    .user_data_dir(user_data_path)
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
        crate::log_info!("Starting risk assessment for ticket: {}", ticket_id);
        let tab = self.get_tab()?;

        let ticket_url = format!("{}/browse/{}", self.config.base_url, ticket_id);
        self.count_nav += 1;
        crate::log_info!("Navigating #{} to: {}", self.count_nav, ticket_url);
        tab.navigate_to(&ticket_url)?;
        tab.wait_until_navigated()?;

        crate::log_info!("Verifying ticket page URL...");
        let login_username = {
            let trimmed = self.config.auth.username.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        };

        let microsoft_password = {
            let trimmed = self.config.auth.microsoft_password.trim();
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
            microsoft_password,
        )?;

        if is_on_correct_page {
            crate::log_info!("✅ Confirmed on correct ticket page: {}", ticket_id);

            self.open_risk_assessment_editor()?;

            if let Some(value) = &config.change_impact_assessment.security_controls_impact {
                crate::log_info!("Setting Security Controls Impact to '{}'.", value);
                self.select_dropdown_option(
                    &[
                        "security controls impact",
                        "security impact",
                        "security control impact",
                    ],
                    value,
                )?;
            } else {
                crate::log_warn!(
                    "No Security Controls Impact value provided in configuration; skipping field update"
                );
            }

            self.save_risk_assessment_changes()?;
            crate::log_info!("Risk assessment updates submitted.");
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
    fn click_button_save(&self) -> Result<bool> {
        let tab = self.tab()?;

        crate::log_info!("Findin save button ...");
        let button = tab.wait_for_element("button.css.-vl1vwyf")?;
        //let button = tab.wait_for_element("button[name='Edit form']")?;
        crate::log_info!("Button found, clicking... {:?}", button);
        button.click()?;
        tab.wait_until_navigated()?;
        Ok(true)
    }
    fn click_button_edit_form(&self) -> Result<bool> {
        let tab = self.tab()?;

        crate::log_info!("Waiting for 'Edit form' button to be present...");
        let button = tab.wait_for_element("._19itidpf")?;
        //let button = tab.wait_for_element("button[name='Edit form']")?;
        crate::log_info!("Button found, clicking... {:?}", button);
        button.click()?;
        tab.wait_until_navigated()?;
        Ok(true)
    }

    fn open_risk_assessment_editor(&self) -> Result<()> {
        crate::log_info!("Opening risk assessment edit form...");
        let clicked = self.click_button_edit_form()?;
        if clicked {
            thread::sleep(Duration::from_secs(2));
            Ok(())
        } else {
            crate::log_error!("Failed to open risk assessment edit form...");
            Err(anyhow!(
                "Could not find the 'Edit form' button in the risk assessment section"
            ))
        }
    }

    // TODO: Not working, needs a interactive debug to match elements
    fn select_dropdown_option(&self, field_keywords: &[&str], desired_value: &str) -> Result<()> {
        let tab = self.tab()?;
        let desired = desired_value.trim();
        if desired.is_empty() {
            return Err(anyhow!(
                "Desired value for dropdown {:?} may not be empty",
                field_keywords
            ));
        }

        let lowercase_keywords: Vec<String> =
            field_keywords.iter().map(|kw| kw.to_lowercase()).collect();

        let escape_css = |value: &str| value.replace('"', "\\\"");

        let mut input_element = None;
        for keyword in field_keywords {
            let escaped = escape_css(keyword);
            let selectors = [
                format!("input[aria-label*=\"{}\" i]", escaped),
                format!("input[data-testid*=\"{}\" i]", escaped),
            ];

            for selector in selectors {
                match tab.wait_for_element_with_custom_timeout(&selector, Duration::from_secs(3)) {
                    Ok(element) => {
                        crate::log_info!("Found dropdown input via selector '{}'", selector);
                        input_element = Some(element);
                        break;
                    }
                    Err(err) => {
                        crate::log_trace!("Selector '{}' not ready yet: {:#}", selector, err);
                    }
                }
            }

            if input_element.is_some() {
                break;
            }
        }

        if input_element.is_none() {
            let candidates = tab.find_elements("input[role=\"combobox\"]")?;
            for candidate in candidates {
                if let Some(label) = candidate.get_attribute_value("aria-label")? {
                    let label_lc = label.to_lowercase();
                    if lowercase_keywords.iter().any(|kw| label_lc.contains(kw)) {
                        crate::log_info!("Matched dropdown input via aria-label: {}", label);
                        input_element = Some(candidate);
                        break;
                    }
                }
            }
        }

        let input = input_element.with_context(|| {
            anyhow!(
                "Could not locate dropdown input for keywords {:?}",
                field_keywords
            )
        })?;

        input.scroll_into_view()?;
        input.click()?;

        let modifier_combos: [&[ModifierKey]; 2] = [&[ModifierKey::Ctrl], &[ModifierKey::Meta]];
        for modifiers in modifier_combos {
            if tab
                .press_key_with_modifiers("KeyA", Some(modifiers))
                .is_ok()
            {
                let _ = tab.press_key("Backspace");
                break;
            }
        }

        thread::sleep(Duration::from_millis(200));

        tab.send_character(desired)
            .with_context(|| format!("Failed to type '{}' into dropdown", desired))?;

        thread::sleep(Duration::from_millis(400));

        tab.press_key("Enter")
            .context("Failed to confirm dropdown selection with Enter")?;

        thread::sleep(Duration::from_millis(500));

        Ok(())
    }

    fn save_risk_assessment_changes(&self) -> Result<()> {
        let clicked = self.click_button_save()?;
        if clicked {
            crate::log_info!("Clicked save/update button to submit risk assessment changes");
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
