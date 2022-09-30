use serde::Deserialize;
use std::{collections::HashMap, io::Read, path::PathBuf};

use clap::Parser;
use serde_json::Value;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_name = "FILE")]
    schema: Option<PathBuf>,
}

#[derive(Deserialize, Debug)]
struct TFSchema {
    format_version: String,
    provider_schemas: HashMap<String, TFProviderSchema>,
}

#[derive(Deserialize, Debug)]
struct TFProviderSchema {
    provider: Value,
    resource_schemas: Option<HashMap<String, Value>>,
    data_source_schemas: Option<HashMap<String, Value>>,
}

fn main() -> anyhow::Result<()> {
    let opts = Args::parse();
    let schema_reader: Box<dyn Read> = match opts.schema {
        Some(path) => Box::new(std::fs::File::open(path)?),
        None => Box::new(std::io::stdin()),
    };
    let schema: TFSchema = serde_json::from_reader(schema_reader)?;
    for provider_schema in schema.provider_schemas.values() {
        println!("{:?}", provider_schema.data_source_schemas);
    }
    Ok(())
}
