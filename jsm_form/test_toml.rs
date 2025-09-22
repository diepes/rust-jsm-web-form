use serde_json::Value;

fn main() {
    let toml_content = r#"
summary = "Test Issue"
customfield_10243 = ["Customer Portal", "Web Application", "Azure"]
description = "This is a test"
"#;
    
    let toml_value: Value = toml::from_str(&toml_content).unwrap();
    println!("Parsed TOML: {:#}", toml_value);
    
    if let Value::Object(map) = toml_value {
        for (key, value) in map {
            println!("Key: {}, Value: {}, Type: {}", key, value, match value {
                Value::String(_) => "String",
                Value::Array(_) => "Array",
                Value::Object(_) => "Object",
                Value::Number(_) => "Number",
                Value::Bool(_) => "Bool",
                Value::Null => "Null",
            });
        }
    }
}