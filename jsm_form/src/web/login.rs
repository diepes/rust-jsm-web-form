use anyhow::{Context, Result};
use headless_chrome::Tab;
use serde_json::to_string;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use super::step::StepController;

pub(crate) fn is_on_ticket_page(url: &str, ticket_id: &str) -> bool {
    url.contains(&format!("/browse/{}", ticket_id))
}

pub(crate) fn wait_for_ticket_page(
    tab: &Arc<Tab>,
    _base_url: &str,
    ticket_id: &str,
    timeout_secs: u64,
    username: Option<&str>,
    step: &StepController,
) -> Result<bool> {
    info!("Waiting to confirm we're on the correct ticket page...");
    let mut start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    let mut current_url = tab.get_url();
    let mut atlassian_username_done = false;
    let mut account_continue_done = false;
    let mut microsoft_username_done = false;
    let mut atlassian_prompted = false;
    let mut account_prompted = false;
    let mut microsoft_prompted = false;
    let mut microsoft_inspect_prompted = false;

    while start_time.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(1000));
        let new_url = tab.get_url();
        info!("Check new URL: {}", new_url);

        if is_on_ticket_page(&new_url, ticket_id) {
            if step.enabled() {
                step.pause(&format!(
                    "Ticket page {ticket_id} reached at {new_url}. Inspect the page before continuing.",
                    ticket_id = ticket_id,
                    new_url = new_url
                ))?;
            }
            return Ok(true);
        }

        if let Some(user) = username {
            if new_url.starts_with("https://id.atlassian.com/") && new_url.contains("login") {
                if !atlassian_username_done {
                    if step.enabled() && !atlassian_prompted {
                        step.pause("Atlassian login detected. Inspect the username screen, then press Enter to auto-fill your username.")?;
                        atlassian_prompted = true;
                    }

                    match try_fill_atlassian_username(tab, user) {
                        Ok(true) => {
                            info!("Filled Atlassian username and triggered continue/login");
                            atlassian_username_done = true;
                            if step.enabled() {
                                atlassian_prompted = false;
                                step.pause("Submitted Atlassian username. Inspect the result (account chooser or redirect) before continuing.")?;
                            }
                            continue;
                        }
                        Ok(false) => {
                            info!("Atlassian username field not ready yet; will retry...");
                            if step.enabled() {
                                atlassian_prompted = false;
                            }
                        }
                        Err(err) => {
                            warn!("Failed to auto-fill Atlassian username: {err:?}");
                            atlassian_username_done = true;
                            if step.enabled() {
                                step.pause("Encountered an error while auto-filling the Atlassian username. Resolve manually if needed, then press Enter to continue monitoring.")?;
                            }
                        }
                    }
                }
            } else if new_url.starts_with("https://id.atlassian.com/") && new_url.contains("join/user-access") {
                if !account_continue_done {
                    if step.enabled() && !account_prompted {
                        step.pause("Atlassian account selection detected. Inspect the options, then press Enter to auto-click Continue for the configured user.")?;
                        account_prompted = true;
                    }

                    match try_click_account_continue(tab, user) {
                        Ok(true) => {
                            info!("Detected matching Atlassian account; clicked Continue");
                            account_continue_done = true;
                            if step.enabled() {
                                account_prompted = false;
                                step.pause("Clicked Continue on the Atlassian account chooser. Inspect the redirect before continuing.")?;
                            }
                            continue;
                        }
                        Ok(false) => {
                            info!("Account selection screen present but Continue button not clicked yet");
                            if step.enabled() {
                                account_prompted = false;
                            }
                        }
                        Err(err) => {
                            warn!("Failed to auto-continue Atlassian account selection: {err:?}");
                            account_continue_done = true;
                            if step.enabled() {
                                step.pause("Could not auto-click Continue. Finish the account selection manually, then press Enter to continue monitoring.")?;
                            }
                        }
                    }
                }
            } else if new_url.starts_with("https://login.microsoftonline.com/") {
                if !microsoft_username_done {
                    if step.enabled() && !microsoft_prompted {
                        step.pause("Microsoft login detected. Inspect the page, then press Enter to auto-fill the Microsoft username and press Next.")?;
                        microsoft_prompted = true;
                    }

                    match try_fill_microsoft_username(tab, user) {
                        Ok(true) => {
                            info!("Filled Microsoft login username and pressed Next");
                            microsoft_username_done = true;
                            if step.enabled() {
                                microsoft_prompted = false;
                                step.pause("Submitted Microsoft username. Complete password or MFA steps manually, then press Enter to continue monitoring for the redirect back to Jira.")?;
                            }
                            continue;
                        }
                        Ok(false) => {
                            info!("Microsoft login username field not ready yet; will retry...");
                            if step.enabled() {
                                microsoft_prompted = false;
                            }
                        }
                        Err(err) => {
                            warn!("Failed to auto-fill Microsoft username: {err:?}");
                            microsoft_username_done = true;
                            if step.enabled() {
                                step.pause("Error while auto-filling Microsoft username. Handle authentication manually, then press Enter to continue monitoring.")?;
                            }
                        }
                    }
                } else if step.enabled() && !microsoft_inspect_prompted {
                    step.pause("Microsoft authentication still in progress. Finish any remaining prompts manually, then press Enter to resume monitoring.")?;
                    microsoft_inspect_prompted = true;
                }
            }
        }

        if new_url != current_url {
            info!("URL changed; resetting timeout. URL: {}", new_url);
            current_url = new_url.clone();
            start_time = std::time::Instant::now();
            atlassian_prompted = false;
            account_prompted = false;
            microsoft_prompted = false;
            microsoft_inspect_prompted = false;
        }
    }

    info!("Could not verify we're on the ticket page. Current URL: {}", current_url);
    if step.enabled() {
        step.pause(&format!(
            "Timeout reached while waiting for ticket page. Current URL: {}. Inspect the browser and press Enter to finish.",
            current_url
        ))?;
    }
    Ok(false)
}

pub(crate) fn try_fill_atlassian_username(tab: &Arc<Tab>, username: &str) -> Result<bool> {
    let username_json = to_string(username)?;
    let script = format!(
        r#"(function() {{
            const targetUsername = {username}.toLowerCase();

            function normalise(text) {{
                return (text || '').trim().toLowerCase();
            }}

            const buttons = Array.from(document.querySelectorAll('button, [role=\"button\"]'));

            for (const button of buttons) {{
                const buttonText = normalise(button.innerText || button.textContent);
                if (!buttonText) {{
                    continue;
                }}

                const dataTestId = normalise(button.getAttribute('data-test-id'));
                const container = button.closest('[data-testid], [role], div, form, main');
                const containerText = normalise(container ? container.innerText || container.textContent : '');
                const relatesToUser =
                    dataTestId.includes(targetUsername) ||
                    containerText.includes(targetUsername) ||
                    buttonText.includes(targetUsername);

                const isContinue = buttonText === 'continue' || buttonText.startsWith('sign in');
                if (relatesToUser && isContinue) {{
                    button.click();
                    return "clicked-account";
                }}

                if (!relatesToUser && dataTestId && dataTestId.includes('account-item') && containerText.includes(targetUsername)) {{
                    button.click();
                    return "clicked-account";
                }}
            }}

            const useAnother = buttons.find(btn => normalise(btn.innerText || btn.textContent).includes('use another account'));
            if (useAnother) {{
                useAnother.click();
                return "opened-use-another";
            }}

            const usernameField = document.querySelector('input[data-testid=\"username\"], input[name=\"username\"], input#username, input[type=\"email\"]');
            if (!usernameField) {{
                return "not-found";
            }}
            usernameField.focus();
            usernameField.value = {username};
            usernameField.dispatchEvent(new Event('input', {{ bubbles: true }}));
            usernameField.dispatchEvent(new Event('change', {{ bubbles: true }}));

            const nextButton = document.querySelector('button[data-testid=\"login-submit-idf-testid\"], button[type=\"submit\"], button#login-submit, button[data-testid=\"next-button\"]');
            if (nextButton) {{
                nextButton.click();
                return "filled-and-submitted";
            }}
            return "filled";
        }})"#,
        username = username_json
    );

    let result = tab
        .evaluate(&script, false)
        .context("Failed to evaluate JavaScript to fill username")?;

    let status = result
        .value
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    match status.as_str() {
        "filled" | "filled-and-submitted" | "clicked-account" => Ok(true),
        "opened-use-another" => {
            info!("Triggered 'Use another account' on Atlassian login; waiting for username field to appear.");
            Ok(false)
        }
        "not-found" => Ok(false),
        other => {
            if !other.is_empty() {
                warn!("Unhandled Atlassian login status: {other}");
            }
            Ok(false)
        }
    }
}

pub(crate) fn try_fill_microsoft_username(tab: &Arc<Tab>, username: &str) -> Result<bool> {
    let username_json = to_string(username)?;
    let script = format!(
        r#"(function() {{
            const input = document.querySelector('input[name=\"loginfmt\"], input#i0116, input[type=\"email\"]');
            if (!input) {{
                return "not-found";
            }}
            input.focus();
            input.value = {username};
            input.dispatchEvent(new Event('input', {{ bubbles: true }}));
            input.dispatchEvent(new Event('change', {{ bubbles: true }}));

            const nextButton = document.querySelector('#idSIButton9, button[type=\"submit\"], input[type=\"submit\"]');
            if (nextButton) {{
                nextButton.click();
                return "submitted";
            }}
            return "filled";
        }})"#,
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

pub(crate) fn try_click_account_continue(tab: &Arc<Tab>, username: &str) -> Result<bool> {
    let username_json = to_string(&username.to_lowercase())?;
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
        }})"#,
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
