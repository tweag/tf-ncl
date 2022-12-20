//! A builder interface for constructing Nickel record terms
use nickel_lang::{
    identifier::Ident,
    parser::utils::{build_record, FieldPathElem},
    term::{
        record::{RecordAttrs, RecordData},
        Contract, MergePriority, MetaValue, RichTerm, Term,
    },
};

type StaticPath = Vec<Ident>;

/// A marker type for tracking a [Field] that is not yet completely specified.
pub struct Incomplete();

/// A markertype for tracking a [Field] that is finalized with or without a value.
pub struct Complete(Option<RichTerm>);

/// This a builder for a single record field. The generic paramter `RB` is used to track whether
/// the field has been completely constructed and whether it has been associated with a record
/// builder yet.
#[derive(Debug)]
pub struct Field<RB> {
    record: RB,
    path: StaticPath,
    metadata: Option<MetaValue>,
}

impl<A> Field<A> {
    /// Set `doc` metadata for this Nickel record field. See also [Field::some_doc].
    pub fn doc(mut self, doc: impl AsRef<str>) -> Self {
        self.metadata = Some(MetaValue {
            doc: Some(doc.as_ref().into()),
            ..self.metadata.unwrap_or_default()
        });
        self
    }

    /// [Option]ally set `doc` metadata for this Nickel record field. See also [Field::doc].
    pub fn some_doc(mut self, some_doc: Option<impl AsRef<str>>) -> Self {
        if let Some(d) = some_doc {
            self = self.doc(d);
        }
        self
    }

    /// Set Nickel metadata to mark this field as optional if the surrounding record is interpreted
    /// as a contract. See also [Field::set_optional].
    pub fn optional(mut self) -> Self {
        self.metadata = Some(MetaValue {
            opt: true,
            ..self.metadata.unwrap_or_default()
        });
        self
    }

    /// Determine if this field should be regarded as optional if the surrounding record is
    /// interpreted as a contract. See also [Field::optional]
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

    /// Attach a Nickel `contract` of type [Contract] to this field.
    pub fn contract(mut self, contract: impl Into<Contract>) -> Self {
        self.metadata = self.metadata.or_else(|| Some(Default::default()));
        if let Some(mv) = self.metadata.as_mut() {
            mv.contracts.push(contract.into())
        }
        self
    }

    /// Attach a static Nickel type `t` to this field.
    pub fn types(mut self, t: impl Into<Contract>) -> Self {
        self.metadata = Some(MetaValue {
            types: Some(t.into()),
            ..self.metadata.unwrap_or_default()
        });
        self
    }

    /// Set the merge priority of this field to `priority`.
    pub fn priority(mut self, priority: MergePriority) -> Self {
        self.metadata = Some(MetaValue {
            priority,
            ..self.metadata.unwrap_or_default()
        });
        self
    }
}

impl Field<Incomplete> {
    /// Construct an incomplete field which is not yet associated with a specific record builder
    /// with path `path`.
    pub fn path<I, It>(path: It) -> Self
    where
        I: AsRef<str>,
        It: IntoIterator<Item = I>,
    {
        Field {
            record: Incomplete(),
            path: path.into_iter().map(|e| e.as_ref().into()).collect(),
            metadata: Default::default(),
        }
    }

    /// Construct an incomplete field which is not yet associated with a specific record builder
    /// with name `name`.
    pub fn name(name: impl AsRef<str>) -> Self {
        Self::path([name])
    }

    /// Finalize this field without assigning a value. This is useful for building up record
    /// contracts.
    pub fn no_value(self) -> Field<Complete> {
        Field {
            record: Complete(None),
            path: self.path,
            metadata: self.metadata,
        }
    }

    /// Finalize this field with `value`.
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
    /// Associate a finalized field builder with a [Record].
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
    /// Finalize this field without assigning a value. This is useful for building record
    /// contracts.
    pub fn no_value(mut self) -> Record {
        self.record.fields.push((
            self.path,
            Term::MetaValue(self.metadata.unwrap_or_default()).into(),
        ));
        self.record
    }

    /// Finalize this field with `value`.
    pub fn value(mut self, value: impl Into<RichTerm>) -> Record {
        self.record
            .fields
            .push((self.path, with_metadata(self.metadata, value)));
        self.record
    }
}

/// This is a builder for a Nickel record as a [RichTerm].
#[derive(Debug)]
pub struct Record {
    fields: Vec<(StaticPath, RichTerm)>,
    attrs: RecordAttrs,
}

impl Record {
    pub fn new() -> Self {
        Record {
            fields: vec![],
            attrs: Default::default(),
        }
    }

    /// Create a field with name `name`. Returns a [Field] builder which can be turned back into a
    /// record builder using [Field<Record>::value] or [Field<Record>::no_value] as appropriate.
    pub fn field(self, name: impl AsRef<str>) -> Field<Record> {
        Field {
            record: self,
            path: vec![name.as_ref().into()],
            metadata: None,
        }
    }

    /// Create multiple fields from `fields`. Each item of the iterator should be a completed
    /// [Field] builder, e.g.
    /// ``
    /// Field::name("foo").no_value()
    /// ``
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

    /// Create a field under the field name path `path`. Use this to construct a record of the form
    /// ``
    /// { foo = { bar = "baz" } } ~ { foo.bar = "baz" }
    /// ``
    pub fn path<It, I>(self, path: It) -> Field<Record>
    where
        I: AsRef<str>,
        It: IntoIterator<Item = I>,
    {
        Field {
            record: self,
            path: path.into_iter().map(|e| e.as_ref().into()).collect(),
            metadata: None,
        }
    }

    /// Set the `attrs` field of the resulting record to `attrs`. This can be used to construct an
    /// open record contract by using `RecordAttrs { open: true }`. See also [Record::open].
    pub fn attrs(mut self, attrs: RecordAttrs) -> Self {
        self.attrs = attrs;
        self
    }

    /// Construct an open record. Equivalent to setting the Nickel record attributes to
    /// `RecordAttrs { open: true }`
    #[allow(clippy::needless_update)]
    pub fn open(mut self) -> Self {
        self.attrs = RecordAttrs {
            open: true,
            ..self.attrs
        };
        self
    }

    /// Finalize the builder and return the resulting Nickel record as a [RichTerm]
    pub fn build(self) -> RichTerm {
        fn elaborate_field_path(path: StaticPath, content: RichTerm) -> (FieldPathElem, RichTerm) {
            let mut it = path.into_iter();
            let fst = it.next().unwrap();

            let content = it.rev().fold(content, |acc, id| {
                Term::Record(RecordData::with_fields([(id, acc)].into())).into()
            });

            (FieldPathElem::Ident(fst), content)
        }

        let elaborated = self
            .fields
            .into_iter()
            .map(|(path, rt)| elaborate_field_path(path, rt))
            .collect::<Vec<_>>();
        build_record(elaborated, self.attrs).into()
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
