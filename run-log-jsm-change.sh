#!/usr/bin/env bash
cd jsm_form

## Check that config files exist, ticket.toml and .jsm_token
# list of files to check
files=("ticket.toml" "jsm_config.pvt.toml")

for file in "${files[@]}"; do
  if [ ! -f "$file" ]; then
    echo "Error: $file not found! See README.md for setup instructions."
    exit 1
  fi
done

echo "# Using ticket.toml as input for change"

output="$( cargo run -- submit -t ticket.toml | tee /dev/tty )"

# Extract the Request ID from the output e.g. Request ID: ITH-66778
request_id="$( echo "$output" | grep 'Request ID: ' | awk -F 'Request ID: ' '{print $2}' )"
if [ -z "$request_id" ]; then
  echo "Error: Could not extract Request ID from output."
  exit 1
fi
echo "Extracted Request ID: $request_id"
sleep 2; echo

echo "# Running risk assessment for change $request_id"
cargo run -- risk-assessment -i "$request_id" -t ticket.toml
