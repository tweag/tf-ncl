use crate::terraform::{TFBlock, TFBlockAttribute, TFBlockType, TFSchema, TFType};
use nickel_lang::mk_record;
use nickel_lang::parser::utils::{build_record, FieldPathElem};
use nickel_lang::term::{Contract, MergePriority, MetaValue, RichTerm, Term};
use nickel_lang::types::{AbsType, Types};

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

fn contract_metavalue(contract: impl Into<Contract>) -> RichTerm {
    Term::MetaValue(MetaValue {
        contracts: vec![contract.into()],
        ..Default::default()
    })
    .into()
}

impl AsNickel for (String, TFSchema) {
    fn as_nickel(&self) -> RichTerm {
        let provider_name = &self.0;
        let provider_schemas = &self.1.provider_schemas;
        //TODO(vkleen): figure out how to best map provider URLs to names
        assert!(provider_schemas.len() == 1);
        let provider_schema = provider_schemas.values().next().unwrap();

        build_record(vec![
            (FieldPathElem::Ident("terraform".into()), {
                let required_providers = provider_schemas.iter().map(|(k, _v)| {
                    (
                        FieldPathElem::Ident(provider_name.into()),
                        mk_record! {("source", Term::Str(k.to_string()))},
                    )
                });
                with_priority(MergePriority::Bottom, mk_record!{
                    ("required_providers", build_record(required_providers, Default::default()))
                }).into()
            }),
            (FieldPathElem::Ident("provider".into()), mk_record!{
                (provider_name, contract_metavalue(term_contract(provider_schema.provider.block.as_nickel())))
            }.into()),
            (FieldPathElem::Ident("resource".into()), {
                let resources = provider_schema.resource_schemas.iter().flatten().map(|(k, v)| {
                    (FieldPathElem::Ident(k.into()), Term::MetaValue(MetaValue {
                        doc: v.block.description.clone(),
                        types: Some(dyn_record_contract(v.block.as_nickel())),
                        opt: true,
                        ..Default::default()
                    }).into())
                });
                contract_metavalue(term_contract(build_record(resources, Default::default())))
            }.into()),

            (FieldPathElem::Ident("data".into()), {
                let resources = provider_schema.data_source_schemas.iter().flatten().map(|(k, v)| {
                    (FieldPathElem::Ident(k.into()), Term::MetaValue(MetaValue {
                        doc: v.block.description.clone(),
                        types: Some(dyn_record_contract(v.block.as_nickel())),
                        opt: true,
                        ..Default::default()
                    }).into())
                });
                contract_metavalue(term_contract(build_record(resources, Default::default())))
            }.into()),
        ], Default::default()).into()
    }
}

impl AsNickel for TFBlock {
    fn as_nickel(&self) -> RichTerm {
        let attribute_fields = self
            .attributes
            .iter()
            .flatten()
            .map(|(k, v)| (FieldPathElem::Ident(k.into()), v.as_nickel()));
        let block_fields = self
            .block_types
            .iter()
            .flatten()
            .map(|(k, v)| (FieldPathElem::Ident(k.into()), v.as_nickel()));
        build_record(attribute_fields.chain(block_fields), Default::default()).into()
    }
}

impl AsNickel for TFBlockAttribute {
    fn as_nickel(&self) -> RichTerm {
        Term::MetaValue(MetaValue {
            doc: self.description.clone(),
            opt: !self.required,
            types: Some(type_contract(self.r#type.as_nickel_type())),
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
            types: Some(type_contract(wrap(self, self.block.as_nickel()))),
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
            TFType::String => Types(AbsType::Str()),
            TFType::Number => Types(AbsType::Num()),
            TFType::Bool => Types(AbsType::Bool()),
            TFType::List(inner) => Types(AbsType::Array(Box::new(inner.as_nickel_type()))),
            TFType::Map(inner) => Types(AbsType::DynRecord(Box::new(inner.as_nickel_type()))),
            //TODO(vkleen): Maybe there should be a contract enforcing uniqueness here? Terraform
            //docs seem to indicate that they will implicitely throw away duplicates.
            TFType::Set(inner) => Types(AbsType::Array(Box::new(inner.as_nickel_type()))),
            TFType::Object(fields) => Types(AbsType::StaticRecord(Box::new(fields.iter().fold(
                Types(AbsType::RowEmpty()),
                |acc, (k, t)| {
                    Types(AbsType::RowExtend(
                        k.into(),
                        Some(Box::new(t.as_nickel_type())),
                        Box::new(acc),
                    ))
                },
            )))),
            TFType::Tuple(_) => unimplemented!(),
        }
    }
}
