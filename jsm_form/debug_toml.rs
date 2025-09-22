use serde_json::Value;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let toml_content = r#"
summary = "Test Issue"

[[customfield_10243]]
name = "Azure Virtual Server"

[[customfield_10243]]
name = "Azure Cloud"
"#;
    
    println!("Original TOML:");
    println!("{}", toml_content);
    println!("\n=== Parsing Process ===");
    
    // Parse as TOML value first, then convert to JSON value
    let toml_value: toml::Value = toml::from_str(&toml_content)?;
    println!("1. Parsed as TOML Value:");
    println!("{:#?}", toml_value);
    
    // Convert TOML value to JSON value
    let json_string = serde_json::to_string(&toml_value)?;
    println!("\n2. Converted to JSON string:");
    println!("{}", json_string);
    
    let json_value: Value = serde_json::from_str(&json_string)?;
    println!("\n3. Parsed as JSON Value:");
    println!("{:#}", json_value);
    
    if let Value::Object(map) = json_value {
        println!("\n4. Field extraction:");
        for (key, value) in &map {
            println!("Key: '{}' -> Value: {}", key, value);
            if key == "customfield_10243" {
                println!("  Affected services field details:");
                println!("  Type: {:?}", value);
                match value {
                    Value::Array(arr) => {
                        println!("  Array with {} items:", arr.len());
                        for (i, item) in arr.iter().enumerate() {
                            println!("    [{}]: {}", i, item);
                        }
                    }
                    _ => println!("  Not an array!"),
                }
            }
        }
    }
    
    Ok(())
}