use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Provider {
    pub source: String,
    pub version: String,
    pub configuration: HashMap<String, Attribute>,
    pub data_sources: HashMap<String, Attribute>,
    pub resources: HashMap<String, Attribute>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GoSchema(pub HashMap<String, Attribute>);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Attribute {
    pub description: Option<String>,
    pub optional: bool,
    pub interpolation: InterpolationStrategy,
    #[serde(rename = "type")]
    pub type_: Type,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum InterpolationStrategy {
    Nickel,
    Terraform { force: bool },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Type {
    Dynamic,
    String,
    Number,
    Bool,
    List {
        min: Option<u32>,
        max: Option<u32>,
        content: Box<Type>,
    },
    Object(HashMap<String, Attribute>),
    Dictionary(Box<Type>),
}

#[derive(Deserialize, Debug)]
pub struct Providers(pub HashMap<String, ProviderConfig>);

#[derive(Deserialize, Debug)]
pub struct ProviderConfig {
    pub source: String,
    pub version: String,
}

pub struct WithProviders<T> {
    pub providers: Providers,
    pub data: T,
}

pub trait IntoWithProviders
where
    Self: Sized,
{
    fn with_providers(self, providers: Providers) -> WithProviders<Self>;
}

impl<T> IntoWithProviders for T {
    fn with_providers(self, providers: Providers) -> WithProviders<Self> {
        WithProviders {
            providers,
            data: self,
        }
    }
}
