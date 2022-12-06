use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    fmt::Display,
};

use serde::Deserialize;

use crate::terraform::{TFBlock, TFBlockAttribute, TFBlockSchema, TFBlockType, TFSchema, TFType};

#[derive(Debug)]
pub struct Schema {
    pub providers: HashMap<String, Provider>,
}

#[derive(Debug)]
pub struct Provider {
    pub source: String,
    pub version: String,
    pub configuration: HashMap<String, Attribute>,
    pub data_sources: HashMap<String, Attribute>,
    pub resources: HashMap<String, Attribute>,
}

#[derive(Debug)]
pub struct Attribute {
    pub description: Option<String>,
    pub optional: bool,
    pub interpolation: InterpolationStrategy,
    pub type_: Type,
}

#[derive(Debug)]
pub enum InterpolationStrategy {
    Nickel,
    Terraform { force: bool },
}

#[derive(Debug)]
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

impl IntoWithProviders for TFSchema {
    fn with_providers(self, providers: Providers) -> WithProviders<Self> {
        WithProviders {
            providers,
            data: self,
        }
    }
}

/// Terraform required_providers needs to be a bijection between local name and provider source
/// Returns the map provider_source -> (local_name, version) if possible.
/// TODO(vkleen) make a proper error type
fn invert_providers(schema: Providers) -> Result<HashMap<String, (String, String)>, ()> {
    let mut r = HashMap::with_capacity(schema.0.len());
    for (local_name, provider_config) in schema.0.into_iter() {
        if r.contains_key(&provider_config.source) {
            return Err(());
        }
        r.insert(
            provider_config.source,
            (local_name, provider_config.version),
        );
    }
    Ok(r)
}

fn make_configuration(provider: TFBlockSchema) -> Result<HashMap<String, Attribute>, ()> {
    provider.try_into()
}

fn make_data_sources(
    schemas: HashMap<String, TFBlockSchema>,
) -> Result<HashMap<String, Attribute>, ()> {
    Ok(values_try_into(schemas)
        .collect::<Result<HashMap<String, Attribute>, ()>>()?
        .into_iter()
        .map(|(k, v)| (k, v.add_common().into_dictionary()))
        .collect())
}

fn make_resources(
    schemas: HashMap<String, TFBlockSchema>,
) -> Result<HashMap<String, Attribute>, ()> {
    Ok(values_try_into(schemas)
        .collect::<Result<HashMap<String, Attribute>, ()>>()?
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                v.add_common()
                    .add_lifecycle()
                    .add_provisioner()
                    .into_dictionary(),
            )
        })
        .collect())
}

impl Attribute {
    fn into_dictionary(self) -> Attribute {
        Attribute {
            type_: Type::Dictionary(Box::new(self.type_)),
            ..self
        }
    }
}

trait MetaArguments {
    fn add_lifecycle(self) -> Self;
    fn add_common(self) -> Self;
    fn add_provisioner(self) -> Self;
}

impl MetaArguments for Attribute {
    fn add_lifecycle(self) -> Self {
        let Attribute { type_, .. } = self;
        let type_ = match type_ {
            Type::Object(inner) => Type::Object(inner.add_lifecycle()),
            _ => type_,
        };
        Attribute { type_, ..self }
    }

    fn add_common(self) -> Self {
        let Attribute { type_, .. } = self;
        let type_ = match type_ {
            Type::Object(inner) => Type::Object(inner.add_common()),
            _ => type_,
        };
        Attribute { type_, ..self }
    }

    fn add_provisioner(self) -> Self {
        let Attribute { type_, .. } = self;
        let type_ = match type_ {
            Type::Object(inner) => Type::Object(inner.add_provisioner()),
            _ => type_,
        };
        Attribute { type_, ..self }
    }
}

impl MetaArguments for HashMap<String, Attribute> {
    fn add_lifecycle(mut self) -> Self {
        self.extend([(
            "lifecycle".to_string(),
            Attribute {
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                description: None,
                type_: Type::Object(
                [
                    ("create_before_destroy".to_string(), Attribute {
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        description: Some("By default, when Terraform must change a resource argument that cannot be updated in-place due to remote API limitations, Terraform will instead destroy the existing object and then create a new replacement object with the new configured arguments.

The create_before_destroy meta-argument changes this behavior so that the new replacement object is created first, and the prior object is destroyed after the replacement is created.".to_string()),
                            type_: Type::Bool
                        }),
                    ("prevent_destroy".to_string(), Attribute {
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        description: Some("This meta-argument, when set to true, will cause Terraform to reject with an error any plan that would destroy the infrastructure object associated with the resource, as long as the argument remains present in the configuration.".to_string()),
                        type_: Type::Bool
                    }),
                    ("ignore_changes".to_string(), Attribute {
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        description: Some(r#"By default, Terraform detects any difference in the current settings of a real infrastructure object and plans to update the remote object to match configuration.

The ignore_changes feature is intended to be used when a resource is created with references to data that may change in the future, but should not affect said resource after its creation. In some rare cases, settings of a remote object are modified by processes outside of Terraform, which Terraform would then attempt to "fix" on the next run. In order to make Terraform share management responsibilities of a single object with a separate process, the ignore_changes meta-argument specifies resource attributes that Terraform should ignore when planning updates to the associated remote object."#.to_string()),
                        type_: Type::List { min: None, max: None, content: Box::new(Type::String) }
                    }),
                    ("replace_triggered_by".to_string(), Attribute {
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        description: Some(r#"Replaces the resource when any of the referenced items change. Supply a list of expressions referencing managed resources, instances, or instance attributes. When used in a resource that uses count or for_each, you can use count.index or each.key in the expression to reference specific instances of other resources that are configured with the same count or collection."#.to_string()),
                        type_: Type::List { min: None, max: None, content: Box::new(Type::String) }
                    }),
                ]
                .into()),
            })]);
        self
    }

    fn add_common(mut self) -> Self {
        self.extend([
            ("depends_on".to_string(), Attribute {
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                description: Some("Use the depends_on meta-argument to handle hidden resource or module dependencies that Terraform cannot automatically infer. You only need to explicitly specify a dependency when a resource or module relies on another resource's behavior but does not access any of that resource's data in its arguments.".to_string()),
                type_: Type::List { min: None, max: None, content: Box::new(Type::String) }
            }),
            ("provider".to_string(), Attribute {
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                description: Some("The provider meta-argument specifies which provider configuration to use for a resource, overriding Terraform's default behavior of selecting one based on the resource type name. Its value should be an unquoted <PROVIDER>.<ALIAS> reference.".to_string()),
                type_: Type::String
            }),
        ]);
        self
    }

    fn add_provisioner(mut self) -> Self {
        self.extend([(
            "provisioner".to_string(),
            Attribute {
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                description: None,
                type_: Type::Dictionary(Box::new(Type::Dynamic)),
            },
        )]);
        self
    }
}

impl TryFrom<WithProviders<TFSchema>> for Schema {
    /// TODO(vkleen) make a proper error type
    type Error = ();

    fn try_from(s: WithProviders<TFSchema>) -> Result<Self, Self::Error> {
        let mut provider_cfgs = invert_providers(s.providers)?;
        let mut providers = HashMap::with_capacity(provider_cfgs.len());
        for (source, schema) in s.data.provider_schemas.into_iter() {
            let (local_name, version) = provider_cfgs.remove(&source).ok_or(())?;
            providers.insert(
                local_name,
                Provider {
                    source,
                    version,
                    configuration: make_configuration(schema.provider)?,
                    data_sources: make_data_sources(schema.data_source_schemas)?,
                    resources: make_resources(schema.resource_schemas)?,
                },
            );
        }
        Ok(Schema { providers })
    }
}

impl TryFrom<TFBlockAttribute> for Attribute {
    /// TODO(vkleen) make a proper error type
    type Error = ();

    fn try_from(val: TFBlockAttribute) -> Result<Self, Self::Error> {
        let (optional, interpolation) = {
            assert!(!matches!(
                (val.optional, val.required, val.computed),
                (false, false, false) | (true, true, _) | (_, true, true)
            ));
            match (val.optional, val.required, val.computed) {
                (true, false, false) => Ok((true, InterpolationStrategy::Nickel)),
                (false, true, false) => Ok((false, InterpolationStrategy::Nickel)),
                (false, false, true) => {
                    //TODO(vkleen) Once interpolation of computed fields is properly handled,
                    //these fields should no longer be optional
                    Ok((true, InterpolationStrategy::Terraform { force: true }))
                }
                (true, false, true) => {
                    //TODO(vkleen) Once interpolation of computed fields is properly handled,
                    //these fields should no longer be optional
                    Ok((true, InterpolationStrategy::Terraform { force: false }))
                }
                _ => Err(()),
            }
        }?;

        Ok(Attribute {
            description: val.description,
            optional,
            interpolation,
            type_: val.r#type.try_into()?,
        })
    }
}

impl TryFrom<TFBlockType> for Attribute {
    type Error = ();
    fn try_from(val: TFBlockType) -> Result<Self, Self::Error> {
        Ok(Attribute {
            description: val.block.description.clone(),
            optional: true,
            ///TODO(vkleen) this isn't right
            interpolation: InterpolationStrategy::Nickel,
            type_: val.try_into()?,
        })
    }
}

impl TryFrom<TFBlockType> for Type {
    type Error = ();
    fn try_from(val: TFBlockType) -> Result<Self, Self::Error> {
        use crate::terraform::TFBlockNestingMode::*;
        match val.nesting_mode {
            Single => Self::try_from(val.block),
            List | Set => Ok(Type::List {
                min: val.min_items,
                max: val.max_items,
                content: Box::new(val.block.try_into()?),
            }),
            Map => Ok(Type::Dictionary(Box::new(val.block.try_into()?))),
        }
    }
}

impl TryFrom<TFBlock> for Type {
    type Error = ();
    fn try_from(value: TFBlock) -> Result<Self, Self::Error> {
        Ok(Attribute::try_from(value)?.type_)
    }
}

impl TryFrom<TFType> for Type {
    type Error = ();
    fn try_from(val: TFType) -> Result<Self, Self::Error> {
        match val {
            TFType::Dynamic => Ok(Type::Dynamic),
            TFType::String => Ok(Type::String),
            TFType::Number => Ok(Type::Number),
            TFType::Bool => Ok(Type::Bool),
            TFType::List(inner) | TFType::Set(inner) => Ok(Type::List {
                min: None,
                max: None,
                content: Box::new(Type::try_from(*inner)?),
            }),
            TFType::Map(inner) => Ok(Type::Dictionary(Box::new(Type::try_from(*inner)?))),
            TFType::Object(inner) => {
                let inner: Result<HashMap<_, _>, _> = inner
                    .into_iter()
                    .map(|(k, v)| {
                        Ok((
                            k,
                            Attribute {
                                description: None,
                                optional: true,
                                /// Terraform does not provide a machine readable specification
                                /// for which attributes in object types are optional
                                interpolation: InterpolationStrategy::Nickel,
                                type_: v.try_into()?,
                            },
                        ))
                    })
                    .collect();
                Ok(Type::Object(inner?))
            }
            TFType::Tuple(_) => Err(()),
        }
    }
}

fn values_try_into<I, K, V, O>(x: I) -> impl Iterator<Item = Result<(K, O), V::Error>>
where
    K: Display,
    V: TryInto<O>,
    I: IntoIterator<Item = (K, V)>,
{
    x.into_iter().map(|(n, v)| v.try_into().map(|rv| (n, rv)))
}

impl TryFrom<TFBlockSchema> for Attribute {
    type Error = ();
    fn try_from(value: TFBlockSchema) -> Result<Self, Self::Error> {
        Self::try_from(value.block)
    }
}

impl TryFrom<TFBlock> for Attribute {
    type Error = ();
    fn try_from(value: TFBlock) -> Result<Self, Self::Error> {
        Ok(Attribute {
            description: value.description.clone(),
            optional: true,
            interpolation: InterpolationStrategy::Nickel,
            type_: Type::Object(value.try_into()?),
        })
    }
}

impl TryFrom<TFBlockSchema> for HashMap<String, Attribute> {
    type Error = ();
    fn try_from(value: TFBlockSchema) -> Result<Self, Self::Error> {
        Self::try_from(value.block)
    }
}

impl TryFrom<TFBlock> for HashMap<String, Attribute> {
    type Error = ();
    fn try_from(value: TFBlock) -> Result<Self, Self::Error> {
        let attribute_fields = values_try_into(value.attributes);
        let block_fields = values_try_into(value.block_types);
        attribute_fields.chain(block_fields).collect()
    }
}
