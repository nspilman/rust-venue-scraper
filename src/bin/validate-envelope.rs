use anyhow::{Context, Result};
use clap::Parser;
use jsonschema::JSONSchema;
use serde_json::Value;
use std::{fs, path::PathBuf};

/// Validate an envelope JSON file against Envelope v1 schema.
#[derive(Parser, Debug)]
#[command(name = "validate-envelope", version, about = "Validate envelope JSON against schema")] 
struct Cli {
    /// Path to the envelope JSON file to validate
    path: PathBuf,

    /// Optional path to a schema file (defaults to schemas/envelope.v1.json)
    #[arg(long)]
    schema: Option<PathBuf>,
}

fn load_json(path: &PathBuf) -> Result<Value> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let json: Value = serde_json::from_str(&data)
        .with_context(|| format!("Failed to parse JSON in {}", path.display()))?;
    Ok(json)
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let schema_path = args
        .schema
        .unwrap_or_else(|| PathBuf::from("schemas/envelope.v1.json"));

    let schema_json = load_json(&schema_path)?;
    let instance = load_json(&args.path)?;

    // jsonschema 0.17 expects a schema with 'static lifetime; leak the parsed schema for CLI lifetime
    let schema_static: &'static Value = Box::leak(Box::new(schema_json));

    let compiled = JSONSchema::options()
        .compile(schema_static)
        .context("Failed to compile JSON Schema")?;

    let result = compiled.validate(&instance);
    match result {
        Ok(_) => {
            println!("valid");
            Ok(())
        }
        Err(errors) => {
            eprintln!("invalid:");
            for error in errors {
                eprintln!("- {} at {}", error, error.instance_path);
            }
            std::process::exit(1)
        }
    }
}

