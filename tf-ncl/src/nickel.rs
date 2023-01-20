use crate::intermediate::{self, GoSchema, Providers, Schema, WithProviders};
use crate::nickel_builder as builder;
use nickel_lang::term::{Contract, MergePriority, RichTerm, Term};
use nickel_lang::types::{TypeF, Types};

fn term_contract(term: impl Into<RichTerm>) -> Contract {
    type_contract(Types(TypeF::Flat(term.into())))
}

fn dict_contract(term: impl Into<RichTerm>) -> Contract {
    type_contract(Types(TypeF::Dict(Box::new(Types(TypeF::Flat(
        term.into(),
    ))))))
}

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
        as_nickel_record(&self.data.0)
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

impl AsNickel for Schema {
    fn as_nickel(&self) -> RichTerm {
        use builder::*;
        // TODO(vkleen): This is an evil hack until we have a better term construction method
        let add_id_field_contract = term_contract(Term::Var("addIdField__".into()));
        let required_providers = self.providers.iter().map(|(name, provider)| {
            Field::name(name).value(Record::from([
                Field::name("source")
                    .priority(MergePriority::Bottom)
                    .value(Term::Str(provider.source.clone())),
                Field::name("version")
                    .priority(MergePriority::Bottom)
                    .value(Term::Str(provider.version.clone())),
            ]))
        });

        let provider = Record::from(self.providers.iter().map(|(name, provider)| {
            Field::name(name)
                .optional()
                .contract(type_contract(Types(TypeF::Array(Box::new(Types(
                    TypeF::Flat(as_nickel_record(&provider.configuration).build()),
                ))))))
                .no_value()
        }));

        let resource = as_nickel_record(self.providers.values().flat_map(|p| p.resources.iter()));
        let data = as_nickel_record(self.providers.values().flat_map(|p| p.data_sources.iter()));

        let output = Record::from([
            Field::name("value")
                .optional()
                .contract(type_contract(Types(TypeF::Str)))
                .no_value(),
            Field::name("description")
                .optional()
                .contract(type_contract(Types(TypeF::Str)))
                .no_value(),
            Field::name("sensitive")
                .optional()
                .contract(type_contract(Types(TypeF::Bool)))
                .no_value(),
            Field::name("depends_on")
                .optional()
                .contract(type_contract(Types(TypeF::Array(Box::new(Types(
                    TypeF::Str,
                ))))))
                .no_value(),
        ]);

        let variable = Record::from([
            Field::name("default")
                .optional()
                .contract(type_contract(Types(TypeF::Dyn)))
                .no_value(),
            Field::name("description")
                .optional()
                .contract(type_contract(Types(TypeF::Str)))
                .no_value(),
            Field::name("sensitive")
                .optional()
                .contract(type_contract(Types(TypeF::Bool)))
                .no_value(),
            Field::name("type")
                .optional()
                .contract(type_contract(Types(TypeF::Str)))
                .no_value(),
            Field::name("nullable")
                .optional()
                .contract(type_contract(Types(TypeF::Bool)))
                .no_value(),
        ]);

        Record::from([
            Field::name("terraform")
                .contract(term_contract(
                    Record::from([
                        Field::name("required_providers")
                            .contract(dict_contract(Record::from([
                                Field::name("source")
                                    .contract(type_contract(Types(TypeF::Str)))
                                    .no_value(),
                                Field::name("version")
                                    .contract(type_contract(Types(TypeF::Str)))
                                    .no_value(),
                            ])))
                            .no_value(),
                        Field::name("backend")
                            .contract(type_contract(Types(TypeF::Dyn)))
                            .optional()
                            .no_value(),
                    ])
                    .build(),
                ))
                .value(Record::from([
                    Field::name("required_providers").value(Record::from(required_providers))
                ])),
            Field::name("provider")
                .optional()
                .contract(term_contract(provider))
                .no_value(),
            Field::name("resource")
                .optional()
                .contract(term_contract(resource))
                .contract(add_id_field_contract.clone())
                .no_value(),
            Field::name("data")
                .optional()
                .contract(term_contract(data))
                .contract(add_id_field_contract)
                .no_value(),
            Field::name("output")
                .optional()
                .contract(dict_contract(output))
                .no_value(),
            Field::name("variable")
                .optional()
                .contract(dict_contract(variable))
                .no_value(),
        ])
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
            ///TODO(vkleen) Handle interpolation properly
                interpolation: _,
            type_,
        } = self;
        field
            .some_doc(description.clone())
            .set_optional(*optional)
            .contract(type_contract(type_.as_nickel_type()))
            .no_value()
    }
}

impl AsNickelField for &intermediate::Type {
    fn as_nickel_field(
        &self,
        field: builder::Field<builder::Incomplete>,
    ) -> builder::Field<builder::Complete> {
        field
            .contract(type_contract(self.as_nickel_type()))
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
