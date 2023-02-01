use std::convert::TryFrom;
use std::rc::Rc;

use crate::intermediate::{self, FieldDescriptor, GoSchema, Providers, WithProviders};
use crate::nickel_builder as builder;
use nickel_lang::term::array::{Array, ArrayAttrs};
use nickel_lang::term::{Contract, MergePriority, RichTerm, Term};
use nickel_lang::types::{TypeF, Types};

fn type_contract(t: impl Into<Types>) -> Contract {
    Contract {
        types: t.into(),
        label: Default::default(),
    }
}

pub trait AsNickel {
    fn as_nickel(&self) -> RichTerm;
}

impl AsNickel for WithProviders<GoSchema> {
    fn as_nickel(&self) -> RichTerm {
        as_nickel_record(&self.data.schema)
            .path(["terraform", "required_providers"])
            .value(self.providers.as_nickel())
            .build()
    }
}

impl AsNickel for Providers {
    fn as_nickel(&self) -> RichTerm {
        use builder::*;
        Record::from(self.0.iter().map(|(name, provider)| {
            Field::name(name).value(Record::from([
                Field::name("source")
                    .priority(MergePriority::Bottom)
                    .value(Term::Str(provider.source.clone())),
                Field::name("version")
                    .priority(MergePriority::Bottom)
                    .value(Term::Str(provider.version.clone())),
            ]))
        }))
        .build()
    }
}

impl AsNickel for Vec<String> {
    fn as_nickel(&self) -> RichTerm {
        Term::Array(
            Array::new(
                self.iter()
                    .map(|s| RichTerm::from(Term::Str(s.clone())))
                    .collect::<Vec<_>>()
                    .into_boxed_slice()
                    .into(),
            ),
            ArrayAttrs::new(),
        )
        .into()
    }
}

impl AsNickel for Vec<FieldDescriptor> {
    fn as_nickel(&self) -> RichTerm {
        Term::Array(
            Array::new(
                self.iter()
                    .map(|x| x.as_nickel())
                    .collect::<Vec<_>>()
                    .into_boxed_slice()
                    .into(),
            ),
            ArrayAttrs::new(),
        )
        .into()
    }
}

impl AsNickel for FieldDescriptor {
    fn as_nickel(&self) -> RichTerm {
        use builder::*;

        let priority = Term::Enum(if self.force {
            "Force".into()
        } else {
            "Default".into()
        });
        Record::new()
            .field("prio")
            .value(priority)
            .field("path")
            .value(Term::Array(
                Array::new(Rc::from(
                    self.path
                        .iter()
                        .map(|s| RichTerm::from(Term::Str(s.clone())))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                )),
                ArrayAttrs::default(),
            ))
            .build()
    }
}

pub trait AsNickelField {
    fn as_nickel_field(
        &self,
        field: builder::Field<builder::Incomplete>,
    ) -> builder::Field<builder::Complete>;
}

impl AsNickelField for &intermediate::Attribute {
    fn as_nickel_field(
        &self,
        field: builder::Field<builder::Incomplete>,
    ) -> builder::Field<builder::Complete> {
        let intermediate::Attribute {
            description,
            optional,
            computed,
            type_,
        } = self;
        let (t, computed_fields) = type_.as_nickel_contracts();
        let field = field.some_doc(description.clone()).set_optional(*optional);
        let field = if let Some(fs) = computed_fields {
            field.contracts([t, fs].map(type_contract))
        } else {
            field.contract(type_contract(t))
        };
        if *computed {
            field
                .priority(MergePriority::Bottom)
                .value(Term::Var("TfNcl.undefined".into()))
        } else {
            field.no_value()
        }
    }
}

pub trait AsNickelContracts {
    fn as_nickel_contracts(&self) -> (Types, Option<Types>);
}

enum PrimitiveType {
    Dyn,
    Str,
    Num,
    Bool,
}

impl From<PrimitiveType> for RichTerm {
    fn from(t: PrimitiveType) -> Self {
        use nickel_lang::term::Term::Var;
        use PrimitiveType::*;
        match t {
            Dyn => Var("Dyn".into()).into(),
            Str => Var("Str".into()).into(),
            Num => Var("Num".into()).into(),
            Bool => Var("Bool".into()).into(),
        }
    }
}

impl AsNickelContracts for &intermediate::Type {
    fn as_nickel_contracts(&self) -> (Types, Option<Types>) {
        use intermediate::Type::*;
        use nickel_lang::mk_app;
        fn tfvar(inner: impl Into<RichTerm>) -> Types {
            Types(TypeF::Flat(mk_app!(
                Term::Var("TfNcl.Tf".into()),
                inner.into()
            )))
        }

        fn primitive(inner: PrimitiveType) -> (Types, Option<Types>) {
            (tfvar(inner), None)
        }

        match self {
            Dynamic => primitive(PrimitiveType::Dyn),
            String => primitive(PrimitiveType::Str),
            Number => primitive(PrimitiveType::Num),
            Bool => primitive(PrimitiveType::Bool),
            //TODO(vkleen): min and max should be represented as a contract
            //TODO(vkleen): tfvar wrapping is unclear
            List {
                min: _,
                max: _,
                content,
            } => (
                Types(TypeF::Array(Box::new(
                    content.as_ref().as_nickel_contracts().0,
                ))),
                None,
            ),
            Object(fields) => (
                Types(TypeF::Flat(
                    builder::Record::from(
                        fields
                            .iter()
                            .map(|(k, v)| v.as_nickel_field(builder::Field::name(k))),
                    )
                    .into(),
                )),
                None,
            ),
            Dictionary {
                inner,
                prefix,
                computed_fields,
            } => {
                let inner_contract = Types(TypeF::Dict(Box::new(
                    inner.as_ref().as_nickel_contracts().0,
                )));
                (
                    inner_contract,
                    Some(Types(TypeF::Flat(mk_app!(
                        Term::Var("TfNcl.ComputedFields".into()),
                        prefix.as_nickel(),
                        computed_fields.as_nickel()
                    )))),
                )
            }
        }
    }
}

fn as_nickel_record<K, V, It>(r: It) -> builder::Record
where
    K: AsRef<str>,
    V: AsNickelField,
    It: IntoIterator<Item = (K, V)>,
{
    builder::Record::from(
        r.into_iter()
            .map(|(k, v)| v.as_nickel_field(builder::Field::name(k))),
    )
}
