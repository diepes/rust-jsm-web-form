use anyhow::{Context, Result};
use headless_chrome::{Tab, browser::tab::ModifierKey};
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn is_on_ticket_page(url: &str, ticket_id: &str) -> bool {
    url.contains(&format!("/browse/{}", ticket_id))
}

pub(crate) fn wait_for_ticket_page(
    tab: &Arc<Tab>,
    _base_url: &str,
    ticket_id: &str,
    timeout_secs: u64,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<bool> {
    crate::log_info!("Going through login steps ...");
    let mut start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    let mut current_url: String = "".to_string();
    let user = username.unwrap_or_default();
    let pass = password.unwrap_or_default();
    let mut atlassian_username_done = false;
    let mut account_continue_done = false;
    let mut microsoft_username_done = false;
    let mut microsoft_password_done = false;
    let mut warned_same_url = false;

    while start_time.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(5000));
        tab.wait_until_navigated()?;
        let new_url = tab.get_url();
        crate::log_info!("Check new URL: {}", new_url);

        if is_on_ticket_page(&new_url, ticket_id) {
            return Ok(true);
        }
        if new_url == current_url {
            if !warned_same_url && start_time.elapsed() > Duration::from_secs(10) {
                crate::log_warn!(
                    "Login URL has remained at {} for over 10 seconds; continuing to monitor in case manual action is required.",
                    new_url
                );
                warned_same_url = true;
            }
        }

        if new_url.starts_with("https://id.atlassian.com/") && new_url.contains("login") {
            if !atlassian_username_done {
                match try_fill_atlassian_username(tab, user) {
                    Ok(true) => {
                        crate::log_info!("Filled Atlassian username and triggered continue/login");
                        atlassian_username_done = true;
                        continue;
                    }
                    Ok(false) => {
                        crate::log_info!("Atlassian username field not ready yet; will retry...");
                    }
                    Err(err) => {
                        crate::log_warn!("Failed to auto-fill Atlassian username: {err:?}");
                        atlassian_username_done = true;
                    }
                }
            }
        } else if new_url.starts_with("https://id.atlassian.com/")
            && new_url.contains("join/user-access")
        {
            if !account_continue_done {
                match try_click_account_continue(tab, user) {
                    Ok(true) => {
                        crate::log_info!("Detected matching Atlassian account; clicked Continue");
                        account_continue_done = true;
                        continue;
                    }
                    Ok(false) => {
                        crate::log_info!(
                            "Account selection screen present but Continue button not clicked yet"
                        );
                    }
                    Err(err) => {
                        crate::log_warn!(
                            "Failed to auto-continue Atlassian account selection: {err:?}"
                        );
                        account_continue_done = true;
                    }
                }
            }
        } else if new_url
            .starts_with("https://login.microsoftonline.com/common/DeviceAuthTls/reprocess")
        {
            if !warned_same_url {
                crate::log_info!(
                    "Microsoft 2FA reprocess detected (URL: {}). Waiting for user to complete multi-factor authentication...",
                    new_url
                );
                print!(
                    "Please complete any required multi-factor authentication in the opened browser window. "
                );
                std::thread::sleep(Duration::from_millis(10000));
                warned_same_url = true;
            }
            continue;
        } else if new_url.starts_with("https://login.microsoftonline.com/") {
            if !microsoft_username_done {
                match try_fill_microsoft_username(tab, user) {
                    Ok(true) => {
                        crate::log_info!("Filled Microsoft login username and pressed Next");
                        microsoft_username_done = true;
                        continue;
                    }
                    Ok(false) => {
                        crate::log_info!(
                            "Microsoft login username field not ready yet; will retry..."
                        );
                    }
                    Err(err) => {
                        crate::log_warn!("Failed to auto-fill Microsoft username: {err:?}");
                        microsoft_username_done = true;
                    }
                }
            } else if !microsoft_password_done {
                match try_fill_microsoft_password(tab, pass) {
                    Ok(true) => {
                        crate::log_info!("Filled Microsoft password and submitted");
                        microsoft_password_done = true;
                        continue;
                    }
                    Ok(false) => {
                        crate::log_info!("Microsoft password field not ready yet; will retry...");
                    }
                    Err(err) => {
                        crate::log_warn!("Failed to auto-fill Microsoft password: {err:?}");
                        microsoft_password_done = true;
                    }
                }
            }
        }

        if new_url != current_url {
            crate::log_info!("URL changed; resetting timeout. URL: {}", new_url);
            current_url = new_url.clone();
            start_time = std::time::Instant::now();
            warned_same_url = false;
        }
    }

    crate::log_info!(
        "Could not verify we're on the ticket page. Current URL: {}",
        current_url
    );
    Ok(false)
}

pub(crate) fn try_fill_atlassian_username(tab: &Arc<Tab>, username: &str) -> Result<bool> {
    if username.trim().is_empty() {
        crate::log_warn!("No Atlassian username provided; skipping auto-fill");
        return Ok(false);
    }

    const SELECTORS: &[&str] = &[
        "input[data-testid=\"username\"]",
        "input[name=\"username\"]",
        "input#username",
        "input[type=\"email\"]",
    ];

    let mut field = None;
    for selector in SELECTORS {
        match tab.wait_for_element_with_custom_timeout(selector, Duration::from_secs(5)) {
            Ok(element) => {
                crate::log_info!(
                    "Found Atlassian username field with selector '{}'; focusing",
                    selector
                );
                field = Some(element);
                break;
            }
            Err(err) => {
                crate::log_info!("Username selector '{}' not ready yet: {:#}", selector, err);
            }
        }
    }

    let Some(element) = field else {
        return Ok(false);
    };

    element.scroll_into_view()?;
    element.click()?;

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

    tab.send_character(username)
        .context("Failed to type Atlassian username")?;
    tab.press_key("Enter")
        .context("Failed to submit Atlassian username")?;

    Ok(true)
}

pub(crate) fn try_fill_microsoft_username(tab: &Arc<Tab>, username: &str) -> Result<bool> {
    if username.trim().is_empty() {
        crate::log_warn!("No Microsoft username provided; skipping auto-fill");
        return Ok(false);
    } else {
        crate::log_info!("Filling Microsoft username: {}", username);
    }

    const SELECTORS: &[&str] = &[
        "input[name=\"loginfmt\"]",
        "input#i0116",
        "input[type=\"email\"]",
    ];

    let mut field = None;
    for selector in SELECTORS {
        match tab.wait_for_element_with_custom_timeout(selector, Duration::from_secs(5)) {
            Ok(element) => {
                crate::log_info!(
                    "Found Microsoft username field with selector '{}'; focusing",
                    selector
                );
                field = Some(element);
                break;
            }
            Err(err) => {
                crate::log_info!(
                    "Microsoft username selector '{}' not ready yet: {:#}",
                    selector,
                    err
                );
            }
        }
    }

    let Some(element) = field else {
        return Ok(false);
    };

    element.scroll_into_view()?;
    element.click()?;

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

    tab.send_character(username)
        .context("Failed to type Microsoft username")?;

    if tab.press_key("Enter").is_err() {
        if let Ok(button) =
            tab.wait_for_element_with_custom_timeout("#idSIButton9", Duration::from_secs(2))
        {
            crate::log_info!("Clicking Microsoft Next button directly");
            button.scroll_into_view()?;
            button.click()?;
        }
    }

    Ok(true)
}

pub(crate) fn try_fill_microsoft_password(tab: &Arc<Tab>, password: &str) -> Result<bool> {
    if password.trim().is_empty() {
        crate::log_warn!("No Microsoft password provided; skipping auto-fill");
        return Ok(false);
    }

    const SELECTORS: &[&str] = &[
        "input[name=\"passwd\"]",
        "input#i0118",
        "input[type=\"password\"]",
    ];

    let mut field = None;
    for selector in SELECTORS {
        match tab.wait_for_element_with_custom_timeout(selector, Duration::from_secs(5)) {
            Ok(element) => {
                crate::log_info!(
                    "Found Microsoft password field with selector '{}'; focusing",
                    selector
                );
                field = Some(element);
                break;
            }
            Err(err) => {
                crate::log_info!(
                    "Microsoft password selector '{}' not ready yet: {:#}",
                    selector,
                    err
                );
            }
        }
    }

    let Some(element) = field else {
        return Ok(false);
    };

    element.scroll_into_view()?;
    element.click()?;

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

    tab.send_character(password)
        .context("Failed to type Microsoft password")?;

    if tab.press_key("Enter").is_err() {
        if let Ok(button) =
            tab.wait_for_element_with_custom_timeout("#idSIButton9", Duration::from_secs(3))
        {
            crate::log_info!("Clicking Microsoft sign-in button directly");
            button.scroll_into_view()?;
            button.click()?;
        } else {
            return Ok(false);
        }
    }

    Ok(true)
}

pub(crate) fn try_click_account_continue(tab: &Arc<Tab>, username: &str) -> Result<bool> {
    if username.trim().is_empty() {
        crate::log_warn!("No Atlassian username provided; skipping continue button automation");
        return Ok(false);
    }

    let lowercase_username = username.to_lowercase();

    let body_contains_user = tab
        .wait_for_element_with_custom_timeout("body", Duration::from_secs(2))
        .ok()
        .and_then(|body| body.get_inner_text().ok())
        .map(|text| text.to_lowercase().contains(&lowercase_username))
        .unwrap_or(false);

    if !body_contains_user {
        return Ok(false);
    }

    let buttons = tab.find_elements("button")?;
    for button in buttons {
        let text = button.get_inner_text().unwrap_or_default();
        if text.trim().eq_ignore_ascii_case("continue") {
            button.scroll_into_view()?;
            button.click()?;
            return Ok(true);
        }
    }

    Ok(false)
}
