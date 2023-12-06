use std::collections::HashMap;

use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
pub struct GoSchema {
    pub computed_fields: Vec<FieldDescriptor>,
    pub schema: HashMap<String, Attribute>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Attribute {
    pub description: Option<String>,
    pub optional: bool,
    pub computed: bool,
    #[serde(rename = "type")]
    pub type_: Type,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldDescriptor {
    pub force: bool,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
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
    Object {
        open: bool,
        content: HashMap<String, Attribute>,
    },
    #[serde(deserialize_with = "transparent")]
    Dictionary {
        inner: Box<Type>,
        prefix: Vec<String>,
        computed_fields: Vec<FieldDescriptor>,
    },
}

// The very complex return type is required to make serde happy.
#[allow(clippy::type_complexity)]
fn transparent<'de, D>(deser: D) -> Result<(Box<Type>, Vec<String>, Vec<FieldDescriptor>), D::Error>
where
    D: Deserializer<'de>,
{
    <Box<Type> as Deserialize>::deserialize(deser).map(|inner| (inner, vec![], vec![]))
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

fn attribute_at_path<'a>(
    schema: &'a mut HashMap<String, Attribute>,
    path: &[String],
) -> Option<&'a mut Attribute> {
    let mut obj = schema;
    for p in path.split_last().map(|x| x.1).unwrap_or(&[]) {
        obj = obj.get_mut(p).and_then(|attr| match &mut attr.type_ {
            Type::Object { open: _, content } => Some(content),
            _ => None,
        })?;
    }
    obj.get_mut(path.last()?)
}

impl FieldDescriptor {
    fn split_at_first_wildcard(&self) -> (&[String], &[String]) {
        let first_wildcard = self
            .path
            .iter()
            .position(|x| x == "_")
            .unwrap_or(self.path.len());
        self.path.split_at(first_wildcard)
    }

    fn push_down(self, schema: &mut HashMap<String, Attribute>) -> Option<Self> {
        let (prefix, rest) = self.split_at_first_wildcard();
        let Some(attr) = attribute_at_path(schema, prefix) else {
            return Some(self);
        };
        match &mut attr.type_ {
            Type::Dictionary {
                inner: _,
                prefix: prev_prefix,
                computed_fields,
            } => {
                if prev_prefix.is_empty() {
                    *prev_prefix = prefix.to_vec();
                } else if prev_prefix != prefix {
                    return Some(self);
                }
                computed_fields.push(FieldDescriptor {
                    path: rest.into(),
                    ..self
                });
                None
            }
            _ => panic!("Wildcard in field path doesn't correspond to dictionary"),
        }
    }
}

impl GoSchema {
    pub fn push_down_computed_fields(self) -> Self {
        let Self {
            computed_fields,
            mut schema,
        } = self;
        Self {
            computed_fields: computed_fields
                .into_iter()
                .filter_map(|f| f.push_down(&mut schema))
                .collect(),
            schema,
        }
    }
}
