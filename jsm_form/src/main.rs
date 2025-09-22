use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use jsm_form::{JsmFormClient, JsmConfig, FormData, RiskAssessmentConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::Value;
use std::io::{self, Write};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "jsm_form")]
#[command(about = "A CLI tool to automate JSM (Jira Service Management) web form completion")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize configuration file
    Init {
        /// Path to save the config file
        #[arg(short, long, default_value = "jsm_config.pvt.toml")]
        config: PathBuf,
    },
    /// Submit a form with the given data
    Submit {
        /// Path to the config file
        #[arg(short, long, default_value = "jsm_config.pvt.toml")]
        config: PathBuf,
        /// Form data as key=value pairs
        #[arg(short = 'd', long = "data")]
        data: Vec<String>,
        /// JSON file containing form data
        #[arg(short = 'j', long = "json")]
        json_file: Option<PathBuf>,
        /// TOML file containing form data
        #[arg(short = 't', long = "toml")]
        toml_file: Option<PathBuf>,
    },
    /// Complete risk assessment form for an existing ticket
    RiskAssessment {
        /// Path to the config file
        #[arg(short, long, default_value = "jsm_config.pvt.toml")]
        config: PathBuf,
        /// Ticket ID (e.g., ITH-66035)
        #[arg(short = 'i', long = "ticket-id")]
        ticket_id: String,
        /// TOML file containing risk assessment configuration
        #[arg(short = 't', long = "toml")]
        toml_file: PathBuf,
    },
    /// Analyze form structure (for debugging)
    Analyze {
        /// Path to the config file
        #[arg(short, long, default_value = "jsm_config.pvt.toml")]
        config: PathBuf,
    },
}

/// Prompt for credentials if not set in config
fn ensure_credentials(config: &mut JsmConfig) -> Result<()> {
    // Check and prompt for username
    if config.auth.username.is_empty() || config.auth.username == "your-username" {
        print!("Enter username: ");
        io::stdout().flush()?;
        let mut username = String::new();
        io::stdin().read_line(&mut username)?;
        config.auth.username = username.trim().to_string();
        
        if config.auth.username.is_empty() {
            return Err(anyhow::anyhow!("Username cannot be empty"));
        }
    }

    // Check and prompt for password
    if config.auth.password.is_empty() || config.auth.password == "your-password" {
        let password = rpassword::prompt_password("Enter password: ")?;
        if password.is_empty() {
            return Err(anyhow::anyhow!("Password cannot be empty"));
        }
        config.auth.password = password;
    }

    println!("Credentials configured for user: {}", config.auth.username);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { config } => {
            let default_config = jsm_form::config::create_default_config();
            jsm_form::config::save_config(&default_config, &config)?;
            println!("Configuration file created at: {}", config.display());
            println!("Please edit the file with your credentials and settings.");
        }
        
        Commands::Submit { config, data, json_file, toml_file } => {
            let mut config = jsm_form::config::load_config(&config)?;
            
            // Ensure credentials are provided
            ensure_credentials(&mut config)?;
            
            let client = JsmFormClient::new(config);
            
            // Authenticate first
            println!("Authenticating...");
            client.authenticate().await?;
            println!("Authentication successful!");
            
            // Prepare form data
            let mut fields: HashMap<String, Value> = HashMap::new();
            
            // Load from TOML file if provided
            if let Some(toml_path) = toml_file {
                let toml_content = std::fs::read_to_string(&toml_path)
                    .with_context(|| format!("Failed to read TOML file: {}", toml_path.display()))?;
                
                // Parse as TOML value first, then convert to JSON value
                let toml_value: toml::Value = toml::from_str(&toml_content)
                    .with_context(|| format!("Failed to parse TOML file: {}", toml_path.display()))?;
                
                // Convert TOML value to JSON value
                let json_string = serde_json::to_string(&toml_value)
                    .context("Failed to convert TOML to JSON")?;
                let json_value: Value = serde_json::from_str(&json_string)
                    .context("Failed to parse converted JSON")?;
                
                if let Value::Object(map) = json_value {
                    fields.extend(map);
                    println!("Loaded {} fields from TOML file: {}", fields.len(), toml_path.display());
                } else {
                    return Err(anyhow::anyhow!("TOML file must contain an object at the root level"));
                }
            }
            
            // Load from JSON file if provided (will override TOML fields with same keys)
            if let Some(json_path) = json_file {
                let json_content = std::fs::read_to_string(&json_path)
                    .with_context(|| format!("Failed to read JSON file: {}", json_path.display()))?;
                let json_value: Value = serde_json::from_str(&json_content)
                    .with_context(|| format!("Failed to parse JSON file: {}", json_path.display()))?;
                
                if let Value::Object(map) = json_value {
                    let json_field_count = map.len();
                    fields.extend(map);
                    println!("Loaded {} additional fields from JSON file: {}", json_field_count, json_path.display());
                } else {
                    return Err(anyhow::anyhow!("JSON file must contain an object at the root level"));
                }
            }
            
            // Add command line data (will override file fields with same keys)
            for item in data {
                if let Some((key, value)) = item.split_once('=') {
                    fields.insert(key.to_string(), Value::String(value.to_string()));
                } else {
                    eprintln!("Warning: Invalid data format '{}', expected 'key=value'", item);
                }
            }
            
            let form_data = FormData { fields };
            
            if form_data.fields.is_empty() {
                eprintln!("No form data provided. Use -d key=value, -j data.json, or -t data.toml");
                std::process::exit(1);
            }
            
            println!("Submitting form with {} fields...", form_data.fields.len());
            client.submit_form(form_data).await?;
            println!("Form submitted successfully!");
        }
        
        Commands::RiskAssessment { config, ticket_id, toml_file } => {
            let mut config = jsm_form::config::load_config(&config)?;
            
            // Ensure credentials are provided
            ensure_credentials(&mut config)?;
            
            // Load risk assessment configuration from TOML file
            let toml_content = std::fs::read_to_string(&toml_file)
                .with_context(|| format!("Failed to read TOML file: {}", toml_file.display()))?;
            
            // Parse the entire TOML file first
            let toml_value: toml::Value = toml::from_str(&toml_content)
                .with_context(|| format!("Failed to parse TOML file: {}", toml_file.display()))?;
            
            // Extract the risk_assessment section
            let risk_assessment_section = toml_value.get("risk_assessment")
                .context("Missing 'risk_assessment' section in TOML file")?;
            
            // Convert the risk_assessment section to RiskAssessmentConfig
            let risk_config: RiskAssessmentConfig = risk_assessment_section.clone().try_into()
                .with_context(|| format!("Failed to parse risk assessment configuration from TOML file: {}", toml_file.display()))?;
            
            println!("Completing risk assessment for ticket: {}", ticket_id);
            jsm_form::web::complete_risk_assessment(&config, &ticket_id, &risk_config)?;
            println!("Risk assessment completed successfully!");
        }
        
        Commands::Analyze { config } => {
            let mut config = jsm_form::config::load_config(&config)?;
            
            // Ensure credentials are provided
            ensure_credentials(&mut config)?;
            
            let client = JsmFormClient::new(config.clone());
            
            println!("Authenticating...");
            client.authenticate().await?;
            println!("Authentication successful!");
            
            println!("Analyzing form structure for service desk {} and request type {}...", 
                     config.portal_id, config.request_type_id);
            
            // Get request type details to understand the required fields
            let request_type_url = format!(
                "{}/rest/servicedeskapi/servicedesk/{}/requesttype/{}", 
                config.base_url, config.portal_id, config.request_type_id
            );
            
            println!("Fetching request type details from: {}", request_type_url);
            
            let response = reqwest::Client::new()
                .get(&request_type_url)
                .basic_auth(&config.auth.username, Some(&config.auth.password))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if response.status().is_success() {
                let body = response.text().await?;
                println!("Request type details:");
                println!("{}", body);
            } else {
                println!("Failed to get request type details: {}", response.status());
                let error_body = response.text().await.unwrap_or_default();
                println!("Error: {}", error_body);
            }
            
            // Also try to get field information
            let fields_url = format!(
                "{}/rest/servicedeskapi/servicedesk/{}/requesttype/{}/field", 
                config.base_url, config.portal_id, config.request_type_id
            );
            
            println!("\nFetching field details from: {}", fields_url);
            
            let fields_response = reqwest::Client::new()
                .get(&fields_url)
                .basic_auth(&config.auth.username, Some(&config.auth.password))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if fields_response.status().is_success() {
                let fields_body = fields_response.text().await?;
                println!("Available fields:");
                println!("{}", fields_body);
            } else {
                println!("Failed to get field details: {}", fields_response.status());
                let fields_error = fields_response.text().await.unwrap_or_default();
                println!("Error: {}", fields_error);
            }
        }
    }
    
    Ok(())
}
