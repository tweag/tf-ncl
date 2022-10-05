use crate::terraform::{TFBlock, TFBlockAttribute, TFSchema, TFType};
use nickel_lang::identifier::Ident;
use nickel_lang::parser::utils::{build_record, FieldPathElem};
use nickel_lang::stdlib::contract;
use nickel_lang::term::make::{op1, var};
use nickel_lang::term::{Contract, MergePriority, MetaValue, RichTerm, Term, UnaryOp};
use nickel_lang::types::{AbsType, Types};
use nickel_lang::{mk_app, mk_record};

pub trait AsNickel {
    fn as_nickel(&self, lib_import: &RichTerm) -> RichTerm;
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
    fn as_nickel(&self, lib_import: &RichTerm) -> RichTerm {
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
                (provider_name, contract_metavalue(term_contract(provider_schemas.values().next().unwrap().provider.block.as_nickel(lib_import))))
            })
        }
    }
}

impl AsNickel for TFBlock {
    fn as_nickel(&self, lib_import: &RichTerm) -> RichTerm {
        let attribute_fields = self
            .attributes
            .iter()
            .flatten()
            .map(|(k, v)| (FieldPathElem::Ident(k.into()), v.as_nickel(lib_import)));
        build_record(attribute_fields, Default::default()).into()
    }
}

fn from_lib(lib_import: &RichTerm, i: &str) -> RichTerm {
    op1(UnaryOp::StaticAccess(Ident::new(i)), lib_import.clone())
}

impl AsNickel for TFBlockAttribute {
    fn as_nickel(&self, lib_import: &RichTerm) -> RichTerm {
        let mv = MetaValue {
            doc: self.description.clone(),
            contracts: vec![term_contract(self.r#type.as_nickel(lib_import))],
            ..Default::default()
        };
        Term::MetaValue(if self.required {
            mv
        } else {
            MetaValue {
                contracts: vec![term_contract(mk_app!(
                    from_lib(lib_import, "Nullable"),
                    self.r#type.as_nickel(lib_import)
                ))],
                opt: true,
                priority: MergePriority::Bottom,
                value: Some(Term::Null.into()),
                ..mv
            }
        })
        .into()
    }
}

impl AsNickel for TFType {
    fn as_nickel(&self, lib_import: &RichTerm) -> RichTerm {
        match self {
            TFType::String => var("Str"),
            TFType::Number => var("Num"),
            TFType::Bool => var("Bool"),
            TFType::List(inner) => mk_app!(var("Array"), inner.as_nickel(lib_import)),
            TFType::Map(inner) => mk_app!(
                from_lib(lib_import, "dyn_record"),
                inner.as_nickel(lib_import)
            ),
            //TODO(vkleen): Maybe there should be a contract enforcing uniqueness here? Terraform
            //docs seem to indicate that they will implicitely throw away duplicates.
            TFType::Set(inner) => mk_app!(var("Array"), inner.as_nickel(lib_import)),
            TFType::Object(_) => todo!(),
            TFType::Tuple(_) => todo!(),
        }
    }
}
