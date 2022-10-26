use std::collections::HashMap;

use crate::terraform::{TFBlock, TFBlockAttribute, TFBlockType, TFSchema, TFType};
use nickel_lang::mk_record;
use nickel_lang::parser::utils::{build_record, FieldPathElem};
use nickel_lang::term::{Contract, MergePriority, MetaValue, RichTerm, Term};
use nickel_lang::types::{AbsType, Types};
use serde::Deserialize;

pub trait AsNickel {
    fn as_nickel(&self) -> RichTerm;
}

fn with_priority(prio: MergePriority, term: impl Into<RichTerm>) -> RichTerm {
    Term::MetaValue(MetaValue {
        priority: prio,
        value: Some(term.into()),
        ..Default::default()
    })
    .into()
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

        build_record(
            vec![
                (FieldPathElem::Ident("terraform".into()), {
                    let required_providers = providers.iter().map(|(name, provider)| {
                        (
                            FieldPathElem::Ident(name.into()),
                            mk_record! {
                                ("source", Term::Str(provider.source.clone())),
                                ("version", Term::Str(provider.version.clone()))
                            },
                        )
                    });
                    with_priority(MergePriority::Bottom, mk_record!{
                    ("required_providers", build_record(required_providers, Default::default()))
                }).into()
                }),
                (
                    FieldPathElem::Ident("provider".into()),
                    {
                        let provider_spec = build_record(
                            providers.iter().map(|(name, provider)| {
                                let schema = provider_schemas.get(&provider.source).unwrap();
                                (
                                    FieldPathElem::Ident(name.into()),
                                    Term::MetaValue(MetaValue {
                                        contracts: vec![type_contract(Types(AbsType::Array(
                                            Box::new(Types(AbsType::Flat(
                                                schema.provider.block.as_nickel(),
                                            ))),
                                        )))],
                                        opt: true,
                                        ..Default::default()
                                    })
                                    .into(),
                                )
                            }),
                            Default::default(),
                        );
                        Term::MetaValue(MetaValue {
                            contracts: vec![term_contract(provider_spec)],
                            opt: true,
                            ..Default::default()
                        })
                    }
                    .into(),
                ),
                (
                    FieldPathElem::Ident("resource".into()),
                    {
                        let resources = provider_schemas
                            .values()
                            .map(|v| v.resource_schemas.iter())
                            .flatten()
                            .map(|(k, v)| {
                                (
                                    FieldPathElem::Ident(k.into()),
                                    Term::MetaValue(MetaValue {
                                        doc: v.block.description.clone(),
                                        contracts: vec![dyn_record_contract(v.block.as_nickel())],
                                        opt: true,
                                        ..Default::default()
                                    })
                                    .into(),
                                )
                            });
                        Term::MetaValue(MetaValue {
                            contracts: vec![
                                term_contract(build_record(resources, Default::default())),
                                add_id_field_contract.clone(),
                            ],
                            opt: true,
                            ..Default::default()
                        })
                    }
                    .into(),
                ),
                (
                    FieldPathElem::Ident("data".into()),
                    {
                        let data_sources = provider_schemas
                            .values()
                            .map(|v| v.data_source_schemas.iter())
                            .flatten()
                            .map(|(k, v)| {
                                (
                                    FieldPathElem::Ident(k.into()),
                                    Term::MetaValue(MetaValue {
                                        doc: v.block.description.clone(),
                                        contracts: vec![dyn_record_contract(v.block.as_nickel())],
                                        opt: true,
                                        ..Default::default()
                                    })
                                    .into(),
                                )
                            });
                        Term::MetaValue(MetaValue {
                            contracts: vec![
                                term_contract(build_record(data_sources, Default::default())),
                                add_id_field_contract.clone(),
                            ],
                            opt: true,
                            ..Default::default()
                        })
                    }
                    .into(),
                ),
                (
                    FieldPathElem::Ident("output".into()),
                    {
                        let output_schema = mk_record!{
                            ("value", Term::MetaValue(MetaValue { contracts: vec![type_contract(Types(AbsType::Str()))], opt: true, ..Default::default() })),
                            ("description", Term::MetaValue(MetaValue { contracts: vec![type_contract(Types(AbsType::Str()))], opt: true, ..Default::default() })),
                            ("sensitive", Term::MetaValue(MetaValue { contracts: vec![type_contract(Types(AbsType::Bool()))], opt: true, ..Default::default() })),
                            ("depends_on", Term::MetaValue(MetaValue { contracts: vec![type_contract(Types(AbsType::Array(Box::new(Types(AbsType::Str())))))], opt: true, ..Default::default() }))
                        };
                        Term::MetaValue(MetaValue {
                            contracts: vec![dyn_record_contract(output_schema)],
                            opt: true,
                            ..Default::default()
                        })
                    }.into(),
                ),
            ],
            Default::default(),
        )
        .into()
    }
}

impl AsNickel for TFBlock {
    fn as_nickel(&self) -> RichTerm {
        let attribute_fields = self
            .attributes
            .iter()
            .map(|(k, v)| (FieldPathElem::Ident(k.into()), v.as_nickel()));
        let block_fields = self
            .block_types
            .iter()
            .map(|(k, v)| (FieldPathElem::Ident(k.into()), v.as_nickel()));
        build_record(attribute_fields.chain(block_fields), Default::default()).into()
    }
}

impl AsNickel for TFBlockAttribute {
    fn as_nickel(&self) -> RichTerm {
        Term::MetaValue(MetaValue {
            doc: self.description.clone(),
            opt: !self.required,
            contracts: vec![type_contract(self.r#type.as_nickel_type())],
            ..Default::default()
        })
        .into()
    }
}

impl AsNickel for TFBlockType {
    fn as_nickel(&self) -> RichTerm {
        fn wrap(t: &TFBlockType, nt: RichTerm) -> Types {
            use crate::terraform::TFBlockNestingMode::*;
            match t.nesting_mode {
                Single => nt.as_nickel_type(),
                List | Set => Types(AbsType::Array(Box::new(nt.as_nickel_type()))),
                Map => Types(AbsType::DynRecord(Box::new(nt.as_nickel_type()))),
            }
        }
        fn is_required(t: &TFBlockType) -> bool {
            t.min_items.iter().any(|&x| x >= 1)
        }

        Term::MetaValue(MetaValue {
            contracts: vec![type_contract(wrap(self, self.block.as_nickel()))],
            opt: !is_required(self),
            ..Default::default()
        })
        .into()
    }
}

pub trait AsNickelType {
    fn as_nickel_type(&self) -> Types;
}

impl AsNickelType for RichTerm {
    fn as_nickel_type(&self) -> Types {
        Types(AbsType::Flat(self.clone()))
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
            //TODO(vkleen): Maybe there should be a contract enforcing uniqueness here? Terraform
            //docs seem to indicate that they will implicitely throw away duplicates.
            TFType::Set(inner) => Types(AbsType::Array(Box::new(inner.as_nickel_type()))),
            TFType::Object(fields) => Types(AbsType::Flat(
                build_record(
                    fields.iter().map(|(k, v)| {
                        (
                            FieldPathElem::Ident(k.into()),
                            Term::MetaValue(MetaValue {
                                contracts: vec![type_contract(v.as_nickel_type())],
                                opt: true,
                                ..Default::default()
                            })
                            .into(),
                        )
                    }),
                    Default::default(),
                )
                .into(),
            )),
            TFType::Tuple(_) => unimplemented!(),
        }
    }
}
