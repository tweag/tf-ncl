use serde::{de::Visitor, Deserialize, Deserializer};
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
    provider: TFBlockSchema,
    resource_schemas: Option<HashMap<String, Value>>,
    data_source_schemas: Option<HashMap<String, TFBlockSchema>>,
}

#[derive(Deserialize, Debug)]
struct TFBlockSchema {
    version: i64,
    block: TFBlock,
}

#[derive(Deserialize, Debug)]
struct TFBlock {
    attributes: Option<HashMap<String, TFBlockAttribute>>,
    block_types: Option<Value>,
    description: Option<String>,
}

// The Default::default() for bool is false
#[derive(Deserialize, Debug)]
struct TFBlockAttribute {
    r#type: TFType,
    description: Option<String>,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    computed: bool,
    #[serde(default)]
    sensitive: bool,
}

#[derive(Debug)]
enum TFType {
    String,
    Number,
    Bool,
    List(Box<TFType>),
    Map(Box<TFType>),
    Set(Box<TFType>),
    Object(HashMap<String, TFType>),
    Tuple(Vec<TFType>),
}

impl<'de> Deserialize<'de> for TFType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TFTypeVisitor();
        impl<'de> Visitor<'de> for TFTypeVisitor {
            type Value = TFType;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a terraform type")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "string" => Ok(TFType::String),
                    "number" => Ok(TFType::Number),
                    "bool" => Ok(TFType::Bool),
                    _ => Err(serde::de::Error::unknown_variant(
                        v,
                        &["string", "number", "bool"],
                    )),
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let collection_type = seq
                    .next_element::<String>()?
                    .ok_or(serde::de::Error::invalid_length(0, &self))?;
                match collection_type.as_str() {
                    "list" => Ok(TFType::List(Box::new(
                        seq.next_element::<TFType>()?
                            .ok_or(serde::de::Error::invalid_length(1, &self))?,
                    ))),
                    "map" => Ok(TFType::Map(Box::new(
                        seq.next_element::<TFType>()?
                            .ok_or(serde::de::Error::invalid_length(1, &self))?,
                    ))),
                    "set" => Ok(TFType::Set(Box::new(
                        seq.next_element::<TFType>()?
                            .ok_or(serde::de::Error::invalid_length(1, &self))?,
                    ))),
                    "object" => Ok(TFType::Object(
                        seq.next_element::<HashMap<String, TFType>>()?
                            .ok_or(serde::de::Error::invalid_length(1, &self))?,
                    )),
                    "tuple" => Ok(TFType::Tuple(
                        seq.next_element::<Vec<TFType>>()?
                            .ok_or(serde::de::Error::invalid_length(1, &self))?,
                    )),
                    v => Err(serde::de::Error::unknown_variant(
                        v,
                        &["list", "map", "set", "object", "tuple"],
                    )),
                }
            }
        }
        deserializer.deserialize_any(TFTypeVisitor())
    }
}

fn main() -> anyhow::Result<()> {
    let opts = Args::parse();
    let schema_reader: Box<dyn Read> = match opts.schema {
        Some(path) => Box::new(std::fs::File::open(path)?),
        None => Box::new(std::io::stdin()),
    };
    let schema: TFSchema = serde_json::from_reader(schema_reader)?;
    for (provider, schema) in schema.provider_schemas {
        println!("{}", provider);
        println!("{:?}", schema.provider.block);
        println!();
        for (data_source, schema) in schema.data_source_schemas.iter().flatten() {
            println!("{}", data_source);
            for (n, a) in schema.block.attributes.iter().flatten() {
                println!("  {}", n);
                println!("    type: {:?}", a.r#type);
                println!("    required: {}", a.required);
                println!("    optional: {}", a.optional);
                println!("    computed: {}", a.computed);
                println!("    sensitive: {}", a.sensitive);
            }
        }
    }
    Ok(())
}
