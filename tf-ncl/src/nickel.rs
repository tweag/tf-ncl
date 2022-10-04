use crate::terraform::TFSchema;
use nickel_lang::parser::utils::{build_record, FieldPathElem};
use nickel_lang::term::{RecordAttrs, RichTerm, Term};

pub trait AsNickel {
    fn as_nickel(&self) -> RichTerm;
}

impl AsNickel for TFSchema {
    fn as_nickel(&self) -> RichTerm {
        let fields = self.provider_schemas.iter().map(|(k, _v)| {
            (
                FieldPathElem::Ident(k.into()),
                Term::Str("<placeholder>".to_owned()).into(),
            )
        });
        build_record(
            fields,
            RecordAttrs {
                open: true,
                ..Default::default()
            },
        )
        .into()
    }
}
