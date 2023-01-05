use serde::{
    de::{Error, Unexpected, Visitor},
    Deserialize, Deserializer,
};
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct TFSchema {
    pub format_version: String,
    pub provider_schemas: HashMap<String, TFProviderSchema>,
}

#[derive(Deserialize, Debug)]
pub struct TFProviderSchema {
    pub provider: TFBlockSchema,
    #[serde(default)]
    pub resource_schemas: HashMap<String, TFBlockSchema>,
    #[serde(default)]
    pub data_source_schemas: HashMap<String, TFBlockSchema>,
}

#[derive(Deserialize, Debug)]
pub struct TFBlockSchema {
    pub version: i64,
    pub block: TFBlock,
}

#[derive(Deserialize, Debug)]
pub struct TFBlock {
    #[serde(default)]
    pub attributes: HashMap<String, TFBlockAttribute>,
    #[serde(default)]
    pub block_types: HashMap<String, TFBlockType>,
    pub description: Option<String>,
}

// Terraform schemas only ever set the `required`, `optional`, `computed` and `sensitive` fields to
// `true` or don't set them at all.
#[derive(Deserialize, Debug)]
pub struct TFBlockAttribute {
    pub r#type: Option<TFType>,
    pub nested_type: Option<TFNestedType>,
    pub description: Option<String>,
    #[serde(default, deserialize_with = "terraform_bool")]
    pub required: bool,
    #[serde(default, deserialize_with = "terraform_bool")]
    pub optional: bool,
    #[serde(default, deserialize_with = "terraform_bool")]
    pub computed: bool,
    #[serde(default, deserialize_with = "terraform_bool")]
    pub sensitive: bool,
}

fn terraform_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let b = bool::deserialize(deserializer)?;
    if !b {
        Err(D::Error::invalid_value(Unexpected::Bool(b), &"true"))
    } else {
        Ok(b)
    }
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

#[derive(Deserialize, Debug)]
pub struct TFNestedType {
    pub attributes: HashMap<String, TFBlockAttribute>,
    pub nesting_mode: TFBlockNestingMode,
    pub min_items: Option<u32>,
    pub max_items: Option<u32>,
}

#[derive(Debug)]
pub enum TFType {
    Dynamic,
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
                    "dynamic" => Ok(TFType::String),
                    "string" => Ok(TFType::String),
                    "number" => Ok(TFType::Number),
                    "bool" => Ok(TFType::Bool),
                    _ => Err(serde::de::Error::unknown_variant(
                        v,
                        &["dynamic", "string", "number", "bool"],
                    )),
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let collection_type = seq
                    .next_element::<String>()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                match collection_type.as_str() {
                    "list" => Ok(TFType::List(Box::new(
                        seq.next_element::<TFType>()?
                            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?,
                    ))),
                    "map" => Ok(TFType::Map(Box::new(
                        seq.next_element::<TFType>()?
                            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?,
                    ))),
                    "set" => Ok(TFType::Set(Box::new(
                        seq.next_element::<TFType>()?
                            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?,
                    ))),
                    //TODO(vkleen): some providers use object types with implicitely optional
                    //fields; this doesn't seem to be documented anywhere in a machine readable
                    //format
                    "object" => Ok(TFType::Object(
                        seq.next_element::<HashMap<String, TFType>>()?
                            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?,
                    )),
                    "tuple" => Ok(TFType::Tuple(
                        seq.next_element::<Vec<TFType>>()?
                            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?,
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
