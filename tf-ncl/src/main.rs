use clap::Parser;
use pretty::{BoxAllocator, BoxDoc, Pretty};
use std::{
    io::{stdout, Read},
    path::PathBuf,
};
use tf_ncl::{
    nickel::{AsNickel, IntoWithProviders, Providers},
    terraform::{AddMetaArguments, TFSchema},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_name = "PROVIDERS")]
    providers: PathBuf,
    #[arg(value_name = "TERRAFORM-SCHEMA")]
    schema: Option<PathBuf>,
}

fn get_providers(opts: &Args) -> anyhow::Result<Providers> {
    Ok(serde_json::from_reader(std::fs::File::open(
        &opts.providers,
    )?)?)
}

fn get_schema(opts: &Args) -> anyhow::Result<TFSchema> {
    let schema_reader: Box<dyn Read> = if let Some(path) = &opts.schema {
        Box::new(std::fs::File::open(path)?)
    } else {
        Box::new(std::io::stdin())
    };

    let mut schema: TFSchema = serde_json::from_reader(schema_reader)?;
    schema.add_metaarguments();
    Ok(schema)
}

fn main() -> anyhow::Result<()> {
    let opts = Args::parse();

    let providers = get_providers(&opts)?;
    let schema = get_schema(&opts)?;

    let pretty_ncl_schema: BoxDoc = schema
        .with_providers(providers)
        .as_nickel()
        .pretty(&BoxAllocator)
        .into_doc();
    pretty_ncl_schema.render(80, &mut stdout())?;
    println!("");

    Ok(())
}
