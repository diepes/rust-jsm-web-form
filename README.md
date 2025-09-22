# JSM Form Automation Tool

A Rust-based command-line tool to automate the completion of JSM (Jira Service Management) web forms.

## Features

- Automated authentication with JSM instances
- Form structure analysis and parsing
- Programmatic form submission
- Configuration file support
- Command-line interface for easy usage

## Installation

Make sure you have Rust installed, then:

```bash
git clone <repository-url>
cd jsm_form
cargo build --release
```

## Configuration

1. Initialize a configuration file:
```bash
cargo run -- init
```

2. Edit `jsm_config.pvt.toml` with your settings:
```toml
org = "your-organization"
base_url = "https://your-organization.atlassian.net"
portal_id = 6
request_type_id = 73

[auth]
username = ""  # Leave empty to be prompted
password = ""  # Leave empty to be prompted (secure input)
```

**Note:** You can leave the username and password fields empty in the config file. The tool will securely prompt you for these credentials when needed.

## Usage

### Submit form data from command line:
```bash
cargo run -- submit -d "summary=Test Issue" -d "description=This is a test issue"
```

### Submit form data from JSON file:
```bash
# Create data.json
{
  "summary": "Test Issue",
  "description": "This is a test issue",
  "priority": "Medium"
}

cargo run -- submit -j data.json
```

### Submit form data from TOML file:
```bash
# Create ticket.toml with required fields
cargo run -- submit -t ticket.toml

# Or combine TOML file with command line overrides
cargo run -- submit -t ticket.toml -d "summary=Override Summary"
```

Example `ticket.toml` for JSM Normal Change requests:
```toml
# Required fields
summary = "Deploy new application version 2.3.1"

# Required datetime fields (use ISO 8601 format with timezone)
customfield_10878 = "2025-09-23T14:00:00.000+1300"  # Planned start
customfield_10879 = "2025-09-23T16:00:00.000+1300"  # Planned end

# Optional fields
description = "This change deploys version 2.3.1 with bug fixes and improvements."

# Optional change management fields
customfield_10883 = """Implementation plan:
1. Stop application services at 14:00
2. Deploy new version from staging
3. Update configuration files
4. Restart services"""

customfield_10884 = """Backout plan:
1. Stop application services
2. Restore previous version from backup
3. Restart services"""
```

### Field Priority Order

When using multiple data sources, fields are loaded in the following priority order (later sources override earlier ones):

1. **TOML file** (using `-t ticket.toml`)
2. **JSON file** (using `-j data.json`)  
3. **Command line arguments** (using `-d key=value`)

Example combining all three:
```bash
# TOML file has summary="Base Summary"
# JSON file has summary="Updated Summary" 
# Command line overrides with summary="Final Summary"
cargo run -- submit -t base.toml -j updates.json -d "summary=Final Summary"
# Result: Uses "Final Summary" from command line
```

### Analyze form structure (for debugging):
```bash
cargo run -- analyze
```

## Security Notes

- **Recommended**: Leave credentials empty in config file and let the tool prompt you securely
- Password input is masked when prompted (not visible on screen)
- Store credentials securely if you choose to save them in config
- Consider using environment variables for sensitive data
- The tool maintains session cookies for authentication

## Form Field Discovery

The tool can discover form fields and their IDs using the `analyze` command:

```bash
cargo run -- analyze
```

This will show you available fields for your specific request type. Common JSM fields include:
- `summary` - Issue title (required)
- `description` - Issue description
- `customfield_XXXXX` - Custom fields specific to your JSM configuration

For JSM Normal Change requests, typical required fields are:
- `summary` - Change title
- `customfield_10878` - Planned start datetime
- `customfield_10879` - Planned end datetime

Use the analyze command to discover the exact field IDs for your JSM instance.

## Troubleshooting

- Ensure your credentials are correct
- Check that the portal_id and request_type_id match your JSM form URL
- Use the `analyze` command to debug form structure issues
- Enable debug logging with `RUST_LOG=debug`