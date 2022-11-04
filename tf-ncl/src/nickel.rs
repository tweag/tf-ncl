use std::collections::HashMap;

use crate::nickel_builder as builder;
use crate::terraform::{TFBlock, TFBlockAttribute, TFBlockType, TFSchema, TFType};
use nickel_lang::term::{Contract, MergePriority, RichTerm, Term};
use nickel_lang::types::{AbsType, Types};
use serde::Deserialize;

pub trait AsNickel {
    fn as_nickel(&self) -> RichTerm;
}

fn term_contract(term: impl Into<RichTerm>) -> Contract {
    type_contract(Types(AbsType::Flat(term.into())))
}

fn dyn_record_contract(term: impl Into<RichTerm>) -> Contract {
    type_contract(Types(AbsType::DynRecord(Box::new(Types(AbsType::Flat(
        term.into(),
    ))))))
}

fn type_contract(t: impl Into<Types>) -> Contract {
    Contract {
        types: t.into(),
        label: Default::default(),
    }
}

#[derive(Deserialize, Debug)]
pub struct Providers(HashMap<String, ProviderConfig>);

#[derive(Deserialize, Debug)]
pub struct ProviderConfig {
    source: String,
    version: String,
}

pub struct WithProviders<T> {
    providers: Providers,
    data: T,
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

impl AsNickel for WithProviders<TFSchema> {
    fn as_nickel(&self) -> RichTerm {
        let providers = &self.providers.0;
        let provider_schemas = &self.data.provider_schemas;

        //TODO(vkleen): This is an evil hack until we have a better term construction method
        let add_id_field_contract = term_contract(Term::Var("addIdField__".into()));

        let required_providers = providers.iter().map(|(name, provider)| {
            builder::Field::name(name)
                .priority(MergePriority::Bottom)
                .value(builder::Record::from([
                    builder::Field::name("source")
                        .priority(MergePriority::Bottom)
                        .value(Term::Str(provider.source.clone())),
                    builder::Field::name("version")
                        .priority(MergePriority::Bottom)
                        .value(Term::Str(provider.version.clone())),
                ]))
        });

        let provider = builder::Record::from(providers.iter().map(|(name, provider)| {
            let schema = provider_schemas.get(&provider.source).unwrap();
            builder::Field::name(name)
                .optional()
                .contract(type_contract(Types(AbsType::Array(Box::new(Types(
                    AbsType::Flat(schema.provider.block.as_nickel()),
                ))))))
                .no_value()
        }));

        let resource = builder::Record::from(
            provider_schemas
                .values()
                .map(|v| v.resource_schemas.iter())
                .flatten()
                .map(|(k, v)| {
                    builder::Field::name(k)
                        .optional()
                        .some_doc(v.block.description.as_ref())
                        .contract(dyn_record_contract(v.block.as_nickel()))
                        .no_value()
                }),
        );

        let data = builder::Record::from(
            provider_schemas
                .values()
                .map(|v| v.data_source_schemas.iter())
                .flatten()
                .map(|(k, v)| {
                    builder::Field::name(k)
                        .optional()
                        .some_doc(v.block.description.as_ref())
                        .contract(dyn_record_contract(v.block.as_nickel()))
                        .no_value()
                }),
        );

        let output = builder::Record::from([
            builder::Field::name("value")
                .optional()
                .contract(type_contract(Types(AbsType::Str())))
                .no_value(),
            builder::Field::name("description")
                .optional()
                .contract(type_contract(Types(AbsType::Str())))
                .no_value(),
            builder::Field::name("sensitive")
                .optional()
                .contract(type_contract(Types(AbsType::Bool())))
                .no_value(),
            builder::Field::name("depends_on")
                .optional()
                .contract(type_contract(Types(AbsType::Array(Box::new(Types(
                    AbsType::Str(),
                ))))))
                .no_value(),
        ]);

        builder::Record::from([
            builder::Field::path(["terraform", "required_providers"])
                .priority(MergePriority::Bottom)
                .value(builder::Record::from(required_providers)),
            builder::Field::name("provider")
                .optional()
                .contract(term_contract(provider))
                .no_value(),
            builder::Field::name("resource")
                .optional()
                .contract(term_contract(resource))
                .contract(add_id_field_contract.clone())
                .no_value(),
            builder::Field::name("data")
                .optional()
                .contract(term_contract(data))
                .contract(add_id_field_contract.clone())
                .no_value(),
            builder::Field::name("output")
                .contract(dyn_record_contract(output))
                .no_value(),
        ])
        .build()
    }
}

fn to_fields<K, V, A>(r: A) -> impl Iterator<Item = builder::Field<builder::Complete>>
where
    K: AsRef<str>,
    V: AsNickelField,
    A: Iterator<Item = (K, V)>,
{
    r.map(|(k, v)| v.as_nickel_field(builder::Field::name(k)))
}

impl AsNickel for TFBlock {
    fn as_nickel(&self) -> RichTerm {
        builder::Record::from(
            to_fields(self.attributes.iter()).chain(to_fields(self.block_types.iter())),
        )
        .build()
    }
}

pub trait AsNickelField {
    fn as_nickel_field(
        &self,
        field: builder::Field<builder::Incomplete>,
    ) -> builder::Field<builder::Complete>;
}

impl<A: AsNickelField> AsNickelField for &A {
    fn as_nickel_field(
        &self,
        field: builder::Field<builder::Incomplete>,
    ) -> builder::Field<builder::Complete> {
        (*self).as_nickel_field(field)
    }
}

impl AsNickelField for TFBlockAttribute {
    fn as_nickel_field(
        &self,
        field: builder::Field<builder::Incomplete>,
    ) -> builder::Field<builder::Complete> {
        field
            .some_doc(self.description.as_ref())
            .set_optional(!self.required)
            .contract(type_contract(self.r#type.as_nickel_type()))
            .no_value()
    }
}

impl AsNickelField for TFBlockType {
    fn as_nickel_field(
        &self,
        field: builder::Field<builder::Incomplete>,
    ) -> builder::Field<builder::Complete> {
        fn wrap(t: &TFBlockType, nt: RichTerm) -> Types {
            use crate::terraform::TFBlockNestingMode::*;
            match t.nesting_mode {
                Single => nt.into_nickel_type(),
                List | Set => Types(AbsType::Array(Box::new(nt.into_nickel_type()))),
                Map => Types(AbsType::DynRecord(Box::new(nt.into_nickel_type()))),
            }
        }

        fn is_required(t: &TFBlockType) -> bool {
            t.min_items.iter().any(|&x| x >= 1)
        }

        field
            .set_optional(!is_required(self))
            .contract(type_contract(wrap(self, self.block.as_nickel())))
            .no_value()
    }
}

pub trait AsNickelType {
    fn as_nickel_type(&self) -> Types;
}

pub trait IntoNickelType {
    fn into_nickel_type(self) -> Types;
}

impl AsNickelType for RichTerm {
    fn as_nickel_type(&self) -> Types {
        self.clone().into_nickel_type()
    }
}

impl IntoNickelType for RichTerm {
    fn into_nickel_type(self) -> Types {
        Types(AbsType::Flat(self))
    }
}

impl AsNickelType for TFType {
    fn as_nickel_type(&self) -> Types {
        match self {
            TFType::Dynamic => Types(AbsType::Dyn()),
            TFType::String => Types(AbsType::Str()),
            TFType::Number => Types(AbsType::Num()),
            TFType::Bool => Types(AbsType::Bool()),
            TFType::List(inner) => Types(AbsType::Array(Box::new(inner.as_nickel_type()))),
            TFType::Map(inner) => Types(AbsType::DynRecord(Box::new(inner.as_nickel_type()))),
            // TODO(vkleen): Maybe there should be a contract enforcing uniqueness here? Terraform
            // docs seem to indicate that they will implicitely throw away duplicates.
            TFType::Set(inner) => Types(AbsType::Array(Box::new(inner.as_nickel_type()))),
            TFType::Object(fields) => Types(AbsType::Flat(
                builder::Record::from(fields.iter().map(|(k, v)| {
                    // TODO(vkleen): optional() might not be correct, but terraform
                    // providers seem to be inconsistent about which fields are required
                    builder::Field::name(k)
                        .optional()
                        .contract(type_contract(v.as_nickel_type()))
                        .no_value()
                }))
                .into(),
            )),
            TFType::Tuple(_) => unimplemented!(),
        }
    }
}
