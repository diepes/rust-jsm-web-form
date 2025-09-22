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

The tool automatically discovers form fields by parsing the JSM form HTML. Common JSM fields include:
- `summary` - Issue title
- `description` - Issue description  
- `priority` - Issue priority
- `components` - Components affected
- Custom fields specific to your JSM configuration

## Troubleshooting

- Ensure your credentials are correct
- Check that the portal_id and request_type_id match your JSM form URL
- Use the `analyze` command to debug form structure issues
- Enable debug logging with `RUST_LOG=debug`