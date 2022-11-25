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

// The Default::default() for bool is false
// Terraform schemas only ever set the `required`, `optional`, `computed` and `sensitive` fields to
// `true` or don't set them at all.
#[derive(Deserialize, Debug)]
pub struct TFBlockAttribute {
    pub r#type: TFType,
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

pub trait AddMetaArguments {
    fn add_metaarguments(&mut self);
}

impl AddMetaArguments for TFSchema {
    fn add_metaarguments(&mut self) {
        for s in self.provider_schemas.values_mut() {
            s.add_metaarguments();
        }
    }
}

impl AddMetaArguments for TFProviderSchema {
    fn add_metaarguments(&mut self) {
        self.provider.block.attributes.insert(
            "alias".to_string(),
            TFBlockAttribute {
                r#type: TFType::String,
                description: None,
                required: false,
                optional: true,
                computed: false,
                sensitive: false,
            },
        );
        for s in self.resource_schemas.values_mut() {
            add_resource_metaarguments(&mut s.block)
        }
        for s in self.data_source_schemas.values_mut() {
            add_data_source_metaarguments(&mut s.block)
        }
    }
}

fn add_resource_metaarguments(res: &mut TFBlock) {
    add_common_metaarguments(res);
    add_lifecycle_metaarguments(res);
}

fn add_data_source_metaarguments(res: &mut TFBlock) {
    add_common_metaarguments(res);
}

fn add_lifecycle_metaarguments(res: &mut TFBlock) {
    res.block_types.extend(vec![
            ("lifecycle".to_string(), TFBlockType {
                nesting_mode: TFBlockNestingMode::Single,
                min_items: None,
                max_items: None,
                block: TFBlock {
                    attributes: [
                        ("create_before_destroy".to_string(), TFBlockAttribute {
                            r#type: TFType::Bool,
                            description: Some("By default, when Terraform must change a resource argument that cannot be updated in-place due to remote API limitations, Terraform will instead destroy the existing object and then create a new replacement object with the new configured arguments.

The create_before_destroy meta-argument changes this behavior so that the new replacement object is created first, and the prior object is destroyed after the replacement is created.".to_string()),
                            required: false,
                            optional: true,
                            computed: false,
                            sensitive: false
                        }),
                        ("prevent_destroy".to_string(), TFBlockAttribute {
                            r#type: TFType::Bool,
                            description: Some("This meta-argument, when set to true, will cause Terraform to reject with an error any plan that would destroy the infrastructure object associated with the resource, as long as the argument remains present in the configuration.".to_string()),
                            required: false,
                            optional: true,
                            computed: false,
                            sensitive: false
                        }),
                        ("ignore_changes".to_string(), TFBlockAttribute {
                            r#type: TFType::List(Box::new(TFType::String)),
                            description: Some(r#"By default, Terraform detects any difference in the current settings of a real infrastructure object and plans to update the remote object to match configuration.

The ignore_changes feature is intended to be used when a resource is created with references to data that may change in the future, but should not affect said resource after its creation. In some rare cases, settings of a remote object are modified by processes outside of Terraform, which Terraform would then attempt to "fix" on the next run. In order to make Terraform share management responsibilities of a single object with a separate process, the ignore_changes meta-argument specifies resource attributes that Terraform should ignore when planning updates to the associated remote object."#.to_string()),
                            required: false,
                            optional: true,
                            computed: false,
                            sensitive: false
                        }),
                        ("replace_triggered_by".to_string(), TFBlockAttribute {
                            r#type: TFType::List(Box::new(TFType::String)),
                            description: Some(r#"Replaces the resource when any of the referenced items change. Supply a list of expressions referencing managed resources, instances, or instance attributes. When used in a resource that uses count or for_each, you can use count.index or each.key in the expression to reference specific instances of other resources that are configured with the same count or collection."#.to_string()),
                            required: false,
                            optional: true,
                            computed: false,
                            sensitive: false
                        }),
                    ].into(),
                    block_types: [
                    ].into(),
                    description: None
                },
            }),
        ]);
}

fn add_common_metaarguments(res: &mut TFBlock) {
    res.attributes.extend(vec![
            ("depends_on".to_string(), TFBlockAttribute {
                r#type: TFType::List(Box::new(TFType::String)),
                description: Some("Use the depends_on meta-argument to handle hidden resource or module dependencies that Terraform cannot automatically infer. You only need to explicitly specify a dependency when a resource or module relies on another resource's behavior but does not access any of that resource's data in its arguments.".to_string()),
                required: false,
                optional: true,
                computed: false,
                sensitive: false,
            }),
            ("provider".to_string(), TFBlockAttribute{
                r#type: TFType::String,
                description: Some("The provider meta-argument specifies which provider configuration to use for a resource, overriding Terraform's default behavior of selecting one based on the resource type name. Its value should be an unquoted <PROVIDER>.<ALIAS> reference.".to_string()),
                required: false,
                optional: true,
                computed: false,
                sensitive: false,
            }),
        ]);
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
