use serde::{de::Visitor, Deserialize, Deserializer};
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct TFSchema {
    pub format_version: String,
    pub provider_schemas: HashMap<String, TFProviderSchema>,
}

#[derive(Deserialize, Debug)]
pub struct TFProviderSchema {
    pub provider: TFBlockSchema,
    pub resource_schemas: Option<HashMap<String, TFBlockSchema>>,
    pub data_source_schemas: Option<HashMap<String, TFBlockSchema>>,
}

#[derive(Deserialize, Debug)]
pub struct TFBlockSchema {
    pub version: i64,
    pub block: TFBlock,
}

#[derive(Deserialize, Debug)]
pub struct TFBlock {
    pub attributes: Option<HashMap<String, TFBlockAttribute>>,
    pub block_types: Option<HashMap<String, TFBlockType>>,
    pub description: Option<String>,
}

// The Default::default() for bool is false
#[derive(Deserialize, Debug)]
pub struct TFBlockAttribute {
    pub r#type: TFType,
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub computed: bool,
    #[serde(default)]
    pub sensitive: bool,
}

#[derive(Deserialize, Debug)]
pub struct TFBlockType {
    pub nesting_mode: TFBlockNestingMode,
    pub min_items: Option<u32>,
    pub max_items: Option<u32>,
    pub block: TFBlock,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TFBlockNestingMode {
    Single,
    List,
    Set,
    Map,
}

#[derive(Debug)]
pub enum TFType {
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
                    //TODO(vkleen): some providers use object types with implicitely optional
                    //fields; this doesn't seem to be documented anywhere in a machine readable
                    //format
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
