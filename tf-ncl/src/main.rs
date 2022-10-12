use anyhow::anyhow;
use clap::Parser;
use nickel_lang::{
    mk_record,
    program::Program,
    serialize::ExportFormat,
    term::{RichTerm, Term},
};
use pretty::{BoxAllocator, BoxDoc, Pretty};
use std::{
    io::{stdout, Read},
    path::{Path, PathBuf},
    process,
};
use tf_ncl::{
    nickel::AsNickel,
    terraform::{AddMetaArguments, TFSchema},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_name = "FILE")]
    providers: Option<PathBuf>,
}

//const REQUIRED_PROVIDERS: &str = include_str!("./required_providers.ncl");

//TODO(vkleen): Error handling!
fn eval_providers(ps: impl Read) -> anyhow::Result<Term> {
    let mut p = Program::new_from_source(ps, "<providers>")?;
    println!("{:#?}", p.query(Some("aws.source".to_string())));
    let res = p.eval_full().map(Term::from);
    if let Err(e) = res {
        p.report(e);
        process::exit(1);
    } else {
        Ok(res.unwrap())
    }
}

//TODO(vkleen): Error handling!
fn required_providers_stanza(providers: Term) -> anyhow::Result<String> {
    Ok(nickel_lang::serialize::to_string(
        ExportFormat::Json,
        &mk_record!(("terraform", mk_record!(("required_providers", providers)))),
    )
    .unwrap())
}

fn main() -> anyhow::Result<()> {
    let opts = Args::parse();
    let providers_reader: Box<dyn Read> = if let Some(path) = opts.providers {
        Box::new(std::fs::File::open(path)?)
    } else {
        Box::new(std::io::stdin())
    };

    let providers = eval_providers(providers_reader)?;
    println!("{}", required_providers_stanza(providers)?);

    //let mut schema: TFSchema = serde_json::from_reader(schema_reader)?;
    //schema.add_metaarguments();
    //let pretty_ncl_schema: BoxDoc = (opts.provider, schema)
    //    .as_nickel()
    //    .pretty(&BoxAllocator)
    //    .into_doc();
    //pretty_ncl_schema.render(80, &mut stdout())?;
    //println!("");
    Ok(())
}
