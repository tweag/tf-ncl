use crate::terraform::{TFBlock, TFBlockAttribute, TFSchema, TFType};
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

        let required_providers = provider_schemas.iter().map(|(k, _v)| {
            (
                FieldPathElem::Ident(provider_name.into()),
                mk_record! {("source", Term::Str(k.to_string()))},
            )
        });

        mk_record! {
            ("terraform", with_priority(MergePriority::Bottom, mk_record!{
                ("required_providers", build_record(required_providers, Default::default()))
            })),
            ("provider", mk_record!{
                (provider_name, contract_metavalue(term_contract(provider_schemas.values().next().unwrap().provider.block.as_nickel())))
            })
        }
    }
}

impl AsNickel for TFBlock {
    fn as_nickel(&self) -> RichTerm {
        let attribute_fields = self
            .attributes
            .iter()
            .flatten()
            .map(|(k, v)| (FieldPathElem::Ident(k.into()), v.as_nickel()));
        build_record(attribute_fields, Default::default()).into()
    }
}

impl AsNickel for TFBlockAttribute {
    fn as_nickel(&self) -> RichTerm {
        Term::MetaValue(MetaValue {
            doc: self.description.clone(),
            contracts: vec![type_contract(self.r#type.as_nickel_type())],
            opt: !self.required,
            ..Default::default()
        })
        .into()
    }
}

pub trait AsNickelType {
    fn as_nickel_type(&self) -> Types;
}

impl AsNickelType for TFType {
    fn as_nickel_type(&self) -> Types {
        Types(match self {
            TFType::String => AbsType::Str(),
            TFType::Number => AbsType::Num(),
            TFType::Bool => AbsType::Bool(),
            TFType::List(inner) => AbsType::Array(Box::new(inner.as_nickel_type())),
            TFType::Map(inner) => AbsType::DynRecord(Box::new(inner.as_nickel_type())),
            //TODO(vkleen): Maybe there should be a contract enforcing uniqueness here? Terraform
            //docs seem to indicate that they will implicitely throw away duplicates.
            TFType::Set(inner) => AbsType::Array(Box::new(inner.as_nickel_type())),
            TFType::Object(_) => todo!(),
            TFType::Tuple(_) => todo!(),
        })
    }
}
