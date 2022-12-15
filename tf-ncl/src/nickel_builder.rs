use std::convert::TryInto;

use nickel_lang::{
    parser::{
        uniterm::UniRecord,
        utils::{FieldPath, FieldPathElem},
    },
    term::{record::RecordAttrs, Contract, MergePriority, MetaValue, RichTerm, Term},
};

pub struct Incomplete();

pub struct Complete(Option<RichTerm>);

#[derive(Debug)]
pub struct Field<RB> {
    record: RB,
    path: FieldPath,
    metadata: Option<MetaValue>,
}

impl<A> Field<A> {
    pub fn doc(mut self, doc: impl AsRef<str>) -> Self {
        self.metadata = Some(MetaValue {
            doc: Some(doc.as_ref().into()),
            ..self.metadata.unwrap_or_default()
        });
        self
    }

    pub fn some_doc(mut self, some_doc: Option<impl AsRef<str>>) -> Self {
        if let Some(d) = some_doc {
            self = self.doc(d);
        }
        self
    }

    pub fn optional(mut self) -> Self {
        self.metadata = Some(MetaValue {
            opt: true,
            ..self.metadata.unwrap_or_default()
        });
        self
    }

    pub fn set_optional(mut self, opt: bool) -> Self {
        if self.metadata.is_none() && !opt {
            return self;
        }

        self.metadata = Some(MetaValue {
            opt,
            ..self.metadata.unwrap_or_default()
        });
        self
    }

    pub fn contract(mut self, contract: impl Into<Contract>) -> Self {
        self.metadata = self.metadata.or_else(|| Some(Default::default()));
        if let Some(mv) = self.metadata.as_mut() {
            mv.contracts.push(contract.into())
        }
        self
    }

    pub fn types(mut self, t: impl Into<Contract>) -> Self {
        self.metadata = Some(MetaValue {
            types: Some(t.into()),
            ..self.metadata.unwrap_or_default()
        });
        self
    }

    pub fn priority(mut self, priority: MergePriority) -> Self {
        self.metadata = Some(MetaValue {
            priority,
            ..self.metadata.unwrap_or_default()
        });
        self
    }
}

impl Field<Incomplete> {
    pub fn path<I, It>(path: It) -> Self
    where
        I: AsRef<str>,
        It: IntoIterator<Item = I>,
    {
        Field {
            record: Incomplete(),
            path: path
                .into_iter()
                .map(|e| FieldPathElem::Ident(e.as_ref().into()))
                .collect(),
            metadata: Default::default(),
        }
    }

    pub fn name(name: impl AsRef<str>) -> Self {
        Self::path([name])
    }

    pub fn no_value(self) -> Field<Complete> {
        Field {
            record: Complete(None),
            path: self.path,
            metadata: self.metadata,
        }
    }

    pub fn value(self, value: impl Into<RichTerm>) -> Field<Complete> {
        Field {
            record: Complete(Some(value.into())),
            path: self.path,
            metadata: self.metadata,
        }
    }
}

fn with_metadata(metadata: Option<MetaValue>, value: impl Into<RichTerm>) -> RichTerm {
    match metadata {
        Some(mv) => Term::MetaValue(MetaValue {
            value: Some(value.into()),
            ..mv
        })
        .into(),
        None => value.into(),
    }
}

impl Field<Complete> {
    pub fn with_record(self, r: Record) -> Record {
        let v = self.record;
        let f = Field {
            record: r,
            path: self.path,
            metadata: self.metadata,
        };
        match v {
            Complete(Some(v)) => f.value(v),
            Complete(None) => f.no_value(),
        }
    }
}

impl Field<Record> {
    pub fn no_value(mut self) -> Record {
        self.record.fields.push((
            self.path,
            Term::MetaValue(self.metadata.unwrap_or_default()).into(),
        ));
        self.record
    }

    pub fn value(mut self, value: impl Into<RichTerm>) -> Record {
        self.record
            .fields
            .push((self.path, with_metadata(self.metadata, value)));
        self.record
    }
}

#[derive(Debug)]
pub struct Record {
    fields: Vec<(FieldPath, RichTerm)>,
    attrs: RecordAttrs,
}

impl Record {
    pub fn new() -> Self {
        Record {
            fields: vec![],
            attrs: Default::default(),
        }
    }

    pub fn field(self, name: impl AsRef<str>) -> Field<Record> {
        Field {
            record: self,
            path: vec![FieldPathElem::Ident(name.as_ref().into())],
            metadata: None,
        }
    }

    pub fn fields<I, It>(mut self, fields: It) -> Self
    where
        I: Into<Field<Complete>>,
        It: IntoIterator<Item = I>,
    {
        for f in fields {
            self = f.into().with_record(self)
        }
        self
    }

    pub fn path<It, I>(self, path: It) -> Field<Record>
    where
        I: AsRef<str>,
        It: IntoIterator<Item = I>,
    {
        Field {
            record: self,
            path: path
                .into_iter()
                .map(|e| FieldPathElem::Ident(e.as_ref().into()))
                .collect(),
            metadata: None,
        }
    }

    pub fn attrs(mut self, attrs: RecordAttrs) -> Self {
        self.attrs = attrs;
        self
    }

    #[allow(clippy::needless_update)]
    pub fn open(mut self) -> Self {
        self.attrs = RecordAttrs {
            open: true,
            ..self.attrs
        };
        self
    }

    pub fn build(self) -> RichTerm {
        UniRecord {
            fields: self.fields,
            tail: None,
            attrs: self.attrs,
            pos: Default::default(),
            pos_ellipsis: Default::default(),
        }
        .try_into()
        .unwrap()
    }
}

impl Default for Record {
    fn default() -> Self {
        Self::new()
    }
}

impl<I, It> From<It> for Record
where
    I: Into<Field<Complete>>,
    It: IntoIterator<Item = I>,
{
    fn from(f: It) -> Self {
        Record::new().fields(f)
    }
}

impl From<Record> for RichTerm {
    fn from(val: Record) -> Self {
        val.build()
    }
}

#[cfg(test)]
mod tests {
    use nickel_lang::{
        parser::utils::{build_record, elaborate_field_path, FieldPathElem},
        term::RichTerm,
        types::{TypeF, Types},
    };

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn trivial() {
        let t: RichTerm = Record::new()
            .field("foo")
            .value(Term::Str("bar".into()))
            .into();
        assert_eq!(
            t,
            build_record(
                vec![(
                    FieldPathElem::Ident("foo".into()),
                    Term::Str("bar".into()).into()
                )],
                Default::default()
            )
            .into()
        );
    }

    #[test]
    fn from_iter() {
        let t: RichTerm = Record::from([
            Field::name("foo").value(Term::Null),
            Field::name("bar").value(Term::Null),
        ])
        .into();
        assert_eq!(
            t,
            build_record(
                vec![
                    (FieldPathElem::Ident("foo".into()), Term::Null.into()),
                    (FieldPathElem::Ident("bar".into()), Term::Null.into()),
                ],
                Default::default()
            )
            .into()
        );
    }

    #[test]
    fn some_doc() {
        let t: RichTerm = Record::from([
            Field::name("foo").some_doc(Some("foo")).no_value(),
            Field::name("bar").some_doc(None as Option<&str>).no_value(),
            Field::name("baz").doc("baz").no_value(),
        ])
        .into();
        assert_eq!(
            t,
            build_record(
                vec![
                    (
                        FieldPathElem::Ident("foo".into()),
                        Term::MetaValue(MetaValue {
                            doc: Some("foo".into()),
                            ..Default::default()
                        })
                        .into()
                    ),
                    (
                        FieldPathElem::Ident("bar".into()),
                        Term::MetaValue(MetaValue {
                            ..Default::default()
                        })
                        .into()
                    ),
                    (
                        FieldPathElem::Ident("baz".into()),
                        Term::MetaValue(MetaValue {
                            doc: Some("baz".into()),
                            ..Default::default()
                        })
                        .into()
                    )
                ],
                Default::default()
            )
            .into()
        );
    }

    #[test]
    fn fields() {
        let t: RichTerm = Record::new()
            .fields([
                Field::name("foo").value(Term::Str("foo".into())),
                Field::name("bar").value(Term::Str("bar".into())),
            ])
            .into();
        assert_eq!(
            t,
            build_record(
                vec![
                    (
                        FieldPathElem::Ident("foo".into()),
                        Term::Str("foo".into()).into()
                    ),
                    (
                        FieldPathElem::Ident("bar".into()),
                        Term::Str("bar".into()).into()
                    ),
                ],
                Default::default()
            )
            .into()
        );
    }

    #[test]
    fn fields_metadata() {
        let t: RichTerm = Record::new()
            .fields([
                Field::name("foo").optional().no_value(),
                Field::name("bar").optional().no_value(),
            ])
            .into();
        assert_eq!(
            t,
            build_record(
                vec![
                    (
                        FieldPathElem::Ident("foo".into()),
                        Term::MetaValue(MetaValue {
                            opt: true,
                            ..Default::default()
                        })
                        .into()
                    ),
                    (
                        FieldPathElem::Ident("bar".into()),
                        Term::MetaValue(MetaValue {
                            opt: true,
                            ..Default::default()
                        })
                        .into()
                    ),
                ],
                Default::default()
            )
            .into()
        );
    }

    #[test]
    fn overriding() {
        let t: RichTerm = Record::new()
            .path(vec!["terraform", "required_providers"])
            .value(Record::from([
                Field::name("foo").value(Term::Null),
                Field::name("bar").value(Term::Null),
            ]))
            .path(vec!["terraform", "required_providers", "foo"])
            .value(Term::Str("hello world!".into()))
            .into();
        assert_eq!(
            t,
            build_record(
                vec![
                    elaborate_field_path(
                        vec![
                            FieldPathElem::Ident("terraform".into()),
                            FieldPathElem::Ident("required_providers".into())
                        ],
                        build_record(
                            vec![
                                (FieldPathElem::Ident("foo".into()), Term::Null.into()),
                                (FieldPathElem::Ident("bar".into()), Term::Null.into())
                            ],
                            Default::default()
                        )
                        .into()
                    ),
                    elaborate_field_path(
                        vec![
                            FieldPathElem::Ident("terraform".into()),
                            FieldPathElem::Ident("required_providers".into()),
                            FieldPathElem::Ident("foo".into())
                        ],
                        Term::Str("hello world!".into()).into()
                    )
                ],
                Default::default()
            )
            .into()
        );
    }

    #[test]
    fn open_record() {
        let t: RichTerm = Record::new().open().into();
        assert_eq!(t, build_record(vec![], RecordAttrs { open: true }).into());
    }

    #[test]
    fn prio_metadata() {
        let t: RichTerm = Record::new()
            .field("foo")
            .priority(MergePriority::Top)
            .no_value()
            .into();
        assert_eq!(
            t,
            build_record(
                vec![(
                    FieldPathElem::Ident("foo".into()),
                    Term::MetaValue(MetaValue {
                        doc: None,
                        types: None,
                        contracts: vec![],
                        opt: false,
                        priority: MergePriority::Top,
                        value: None,
                    })
                    .into()
                )],
                Default::default()
            )
            .into()
        );
    }

    #[test]
    fn contract() {
        let t: RichTerm = Record::new()
            .field("foo")
            .contract(Contract {
                types: Types(TypeF::Str),
                label: Default::default(),
            })
            .no_value()
            .into();
        assert_eq!(
            t,
            build_record(
                vec![(
                    FieldPathElem::Ident("foo".into()),
                    Term::MetaValue(MetaValue {
                        contracts: vec![Contract {
                            types: Types(TypeF::Str),
                            label: Default::default()
                        }],
                        ..Default::default()
                    })
                    .into()
                )],
                Default::default()
            )
            .into()
        );
    }

    #[test]
    fn exercise_metadata() {
        let t: RichTerm = Record::new()
            .field("foo")
            .priority(MergePriority::Bottom)
            .doc("foo?")
            .contract(Contract {
                types: Types(TypeF::Str),
                label: Default::default(),
            })
            .types(Contract {
                types: Types(TypeF::Num),
                label: Default::default(),
            })
            .optional()
            .no_value()
            .into();
        assert_eq!(
            t,
            build_record(
                vec![(
                    FieldPathElem::Ident("foo".into()),
                    Term::MetaValue(MetaValue {
                        doc: Some("foo?".into()),
                        types: Some(Contract {
                            types: Types(TypeF::Num),
                            label: Default::default(),
                        }),
                        contracts: vec![Contract {
                            types: Types(TypeF::Str),
                            label: Default::default()
                        }],
                        opt: true,
                        priority: MergePriority::Bottom,
                        value: None,
                    })
                    .into()
                )],
                Default::default()
            )
            .into()
        );
    }
}
