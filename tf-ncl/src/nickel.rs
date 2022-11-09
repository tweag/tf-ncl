use crate::intermediate::{self, Schema};
use crate::nickel_builder as builder;
use nickel_lang::term::{Contract, MergePriority, RichTerm, Term};
use nickel_lang::types::{AbsType, Types};

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

pub trait AsNickel {
    fn as_nickel(&self) -> RichTerm;
}

impl AsNickel for Schema {
    fn as_nickel(&self) -> RichTerm {
        // TODO(vkleen): This is an evil hack until we have a better term construction method
        let add_id_field_contract = term_contract(Term::Var("addIdField__".into()));
        let required_providers = self.providers.iter().map(|(name, provider)| {
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

        let provider = builder::Record::from(self.providers.iter().map(|(name, provider)| {
            builder::Field::name(name)
                .optional()
                .contract(type_contract(Types(AbsType::Array(Box::new(Types(
                    AbsType::Flat(as_nickel_record(&provider.configuration)),
                ))))))
                .no_value()
        }));

        let resource = as_nickel_record(self.providers.values().flat_map(|p| p.resources.iter()));
        let data = as_nickel_record(self.providers.values().flat_map(|p| p.data_sources.iter()));

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
                .contract(add_id_field_contract)
                .no_value(),
            builder::Field::name("output")
                .optional()
                .contract(dyn_record_contract(output))
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

impl AsNickelType for &intermediate::Type {
    fn as_nickel_type(&self) -> Types {
        use intermediate::Type::*;
        match self {
            Dynamic => Types(AbsType::Dyn()),
            String => Types(AbsType::Str()),
            Number => Types(AbsType::Num()),
            Bool => Types(AbsType::Bool()),
            //TODO(vkleen): min and max should be represented as a contract
            List {
                min: _,
                max: _,
                content,
            } => Types(AbsType::Array(Box::new(content.as_ref().as_nickel_type()))),
            Object(fields) => Types(AbsType::Flat(
                builder::Record::from(
                    fields
                        .iter()
                        .map(|(k, v)| v.as_nickel_field(builder::Field::name(k))),
                )
                .into(),
            )),
            Dictionary(inner) => Types(AbsType::DynRecord(Box::new(
                inner.as_ref().as_nickel_type(),
            ))),
        }
    }
}

fn as_nickel_record<K, V, It>(r: It) -> RichTerm
where
    K: AsRef<str>,
    V: AsNickelField,
    It: IntoIterator<Item = (K, V)>,
{
    builder::Record::from(
        r.into_iter()
            .map(|(k, v)| v.as_nickel_field(builder::Field::name(k))),
    )
    .build()
}
