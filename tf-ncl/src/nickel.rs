use crate::intermediate::{self, GoSchema, Providers, WithProviders};
use crate::nickel_builder as builder;
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
            type_,
        } = self;
        field
            .some_doc(description.clone())
            .set_optional(*optional)
            .contract(type_contract(type_.as_nickel_type()))
            .no_value()
    }
}

pub trait AsNickelType {
    fn as_nickel_type(&self) -> Types;
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

impl AsNickelType for &intermediate::Type {
    fn as_nickel_type(&self) -> Types {
        use intermediate::Type::*;
        fn tfvar(inner: impl Into<RichTerm>) -> Types {
            use nickel_lang::mk_app;
            Types(TypeF::Flat(mk_app!(
                Term::Var("TfNcl.Tf".into()),
                inner.into()
            )))
        }

        match self {
            Dynamic => tfvar(PrimitiveType::Dyn),
            String => tfvar(PrimitiveType::Str),
            Number => tfvar(PrimitiveType::Num),
            Bool => tfvar(PrimitiveType::Bool),
            //TODO(vkleen): min and max should be represented as a contract
            //TODO(vkleen): tfvar wrapping is unclear
            List {
                min: _,
                max: _,
                content,
            } => Types(TypeF::Array(Box::new(content.as_ref().as_nickel_type()))),
            Object(fields) => Types(TypeF::Flat(
                builder::Record::from(
                    fields
                        .iter()
                        .map(|(k, v)| v.as_nickel_field(builder::Field::name(k))),
                )
                .into(),
            )),
            Dictionary(inner) => Types(TypeF::Dict(Box::new(inner.as_ref().as_nickel_type()))),
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
