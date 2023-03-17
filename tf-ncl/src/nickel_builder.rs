use nickel_lang::{
    identifier::Ident,
    parser::utils::{build_record, FieldPathElem},
    position::TermPos,
    term::{
        record::{self, FieldMetadata, RecordAttrs, RecordData},
        LabeledType, MergePriority, RichTerm, Term,
    },
    types::{self, EnumRows, RecordRows, TypeF},
};

type StaticPath = Vec<Ident>;

pub struct Incomplete();

pub struct Complete(Option<RichTerm>);

#[derive(Debug)]
pub struct Field<RB> {
    record: RB,
    path: StaticPath,
    metadata: FieldMetadata,
}

pub struct Types(pub TypeF<Box<Types>, RecordRows, EnumRows>);

impl From<Types> for TypeF<Box<Types>, RecordRows, EnumRows> {
    fn from(value: Types) -> Self {
        value.0
    }
}

fn add_positions(t: Types) -> types::Types {
    types::Types {
        types: t
            .0
            .map(|ty| Box::new(add_positions(*ty)), |rrow| rrow, |erow| erow),
        pos: TermPos::None,
    }
}

impl<A> Field<A> {
    pub fn doc(self, doc: impl AsRef<str>) -> Self {
        self.some_doc(Some(doc))
    }

    pub fn some_doc(mut self, some_doc: Option<impl AsRef<str>>) -> Self {
        self.metadata.doc = some_doc.map(|d| d.as_ref().to_owned());
        self
    }

    pub fn optional(self) -> Self {
        self.set_optional(true)
    }

    pub fn set_optional(mut self, opt: bool) -> Self {
        self.metadata.opt = opt;
        self
    }

    pub fn not_exported(self) -> Self {
        self.set_not_exported(true)
    }

    pub fn set_not_exported(mut self, not_exported: bool) -> Self {
        self.metadata.not_exported = not_exported;
        self
    }

    pub fn contract(
        mut self,
        contract: impl Into<TypeF<Box<Types>, RecordRows, EnumRows>>,
    ) -> Self {
        self.metadata.annotation.contracts.push(LabeledType {
            types: add_positions(Types(contract.into())),
            label: Default::default(),
        });
        self
    }

    pub fn contracts<I>(mut self, contracts: I) -> Self
    where
        I: IntoIterator<Item = Types>,
    {
        self.metadata
            .annotation
            .contracts
            .extend(contracts.into_iter().map(|c| LabeledType {
                types: add_positions(c),
                label: Default::default(),
            }));
        self
    }

    pub fn types(mut self, t: impl Into<TypeF<Box<Types>, RecordRows, EnumRows>>) -> Self {
        self.metadata.annotation.types = Some(LabeledType {
            types: add_positions(Types(t.into())),
            label: Default::default(),
        });
        self
    }

    pub fn priority(mut self, priority: MergePriority) -> Self {
        self.metadata.priority = priority;
        self
    }

    pub fn metadata(mut self, metadata: FieldMetadata) -> Self {
        self.metadata = metadata;
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
            path: path.into_iter().map(|e| e.as_ref().into()).collect(),
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
            record::Field {
                value: None,
                metadata: self.metadata,
                ..Default::default()
            },
        ));
        self.record
    }

    pub fn value(mut self, value: impl Into<RichTerm>) -> Record {
        self.record.fields.push((
            self.path,
            record::Field {
                value: Some(value.into()),
                metadata: self.metadata,
                ..Default::default()
            },
        ));
        self.record
    }
}

#[derive(Debug)]
pub struct Record {
    fields: Vec<(StaticPath, record::Field)>,
    attrs: RecordAttrs,
}

fn elaborate_field_path(
    path: StaticPath,
    content: record::Field,
) -> (FieldPathElem, record::Field) {
    let mut it = path.into_iter();
    let fst = it.next().unwrap();

    let content = it.rev().fold(content, |acc, id| {
        record::Field::from(RichTerm::new(
            Term::Record(RecordData {
                fields: [(id, acc)].into(),
                ..Default::default()
            }),
            TermPos::None,
        ))
    });

    (FieldPathElem::Ident(fst), content)
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
            path: vec![name.as_ref().into()],
            metadata: Default::default(),
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
            path: path.into_iter().map(|e| e.as_ref().into()).collect(),
            metadata: Default::default(),
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
        parser::utils::{build_record, FieldPathElem},
        term::{RichTerm, TypeAnnotation},
        types::{TypeF, Types},
    };

    use pretty_assertions::assert_eq;

    use super::*;

    fn term(t: Term) -> record::Field {
        record::Field::from(RichTerm::new(t, TermPos::None))
    }

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
                    term(Term::Str("bar".to_owned()))
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
                    (FieldPathElem::Ident("foo".into()), term(Term::Null)),
                    (FieldPathElem::Ident("bar".into()), term(Term::Null)),
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
                        record::Field {
                            metadata: FieldMetadata {
                                doc: Some("foo".into()),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    ),
                    (FieldPathElem::Ident("bar".into()), Default::default()),
                    (
                        FieldPathElem::Ident("baz".into()),
                        record::Field {
                            metadata: FieldMetadata {
                                doc: Some("baz".into()),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
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
                        term(Term::Str("foo".into()))
                    ),
                    (
                        FieldPathElem::Ident("bar".into()),
                        term(Term::Str("bar".into()))
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
                        record::Field {
                            metadata: FieldMetadata {
                                opt: true,
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    ),
                    (
                        FieldPathElem::Ident("bar".into()),
                        record::Field {
                            metadata: FieldMetadata {
                                opt: true,
                                ..Default::default()
                            },
                            ..Default::default()
                        }
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
                        vec!["terraform".into(), "required_providers".into()],
                        term(build_record(
                            vec![
                                (FieldPathElem::Ident("foo".into()), term(Term::Null)),
                                (FieldPathElem::Ident("bar".into()), term(Term::Null))
                            ],
                            Default::default()
                        ))
                    ),
                    elaborate_field_path(
                        vec![
                            "terraform".into(),
                            "required_providers".into(),
                            "foo".into()
                        ],
                        term(Term::Str("hello world!".into()))
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
                    record::Field {
                        metadata: FieldMetadata {
                            priority: MergePriority::Top,
                            ..Default::default()
                        },
                        ..Default::default()
                    }
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
            .contract(TypeF::String)
            .no_value()
            .into();
        assert_eq!(
            t,
            build_record(
                vec![(
                    FieldPathElem::Ident("foo".into()),
                    record::Field {
                        metadata: FieldMetadata {
                            annotation: TypeAnnotation {
                                contracts: vec![LabeledType {
                                    types: Types {
                                        types: TypeF::String,
                                        pos: TermPos::None
                                    },
                                    label: Default::default()
                                }],
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        ..Default::default()
                    }
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
            .contract(TypeF::String)
            .types(TypeF::Number)
            .optional()
            .not_exported()
            .no_value()
            .into();
        assert_eq!(
            t,
            build_record(
                vec![(
                    FieldPathElem::Ident("foo".into()),
                    record::Field {
                        metadata: FieldMetadata {
                            doc: Some("foo?".into()),
                            opt: true,
                            priority: MergePriority::Bottom,
                            not_exported: true,
                            annotation: TypeAnnotation {
                                types: Some(LabeledType {
                                    types: Types {
                                        types: TypeF::Number,
                                        pos: TermPos::None
                                    },
                                    label: Default::default()
                                }),
                                contracts: vec![LabeledType {
                                    types: Types {
                                        types: TypeF::String,
                                        pos: TermPos::None
                                    },
                                    label: Default::default()
                                }],
                            },
                        },
                        ..Default::default()
                    }
                )],
                Default::default()
            )
            .into()
        );
    }
}
