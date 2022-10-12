use clap::Parser;
use pretty::{BoxAllocator, BoxDoc, Pretty};
use std::{
    io::{stdout, Read},
    path::PathBuf,
};
use tf_ncl::{
    nickel::{AsNickel, ProviderNameVersion},
    terraform::{AddMetaArguments, TFSchema},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_name = "PROVIDER-NAME")]
    provider: String, //TODO(vkleen): This is not going to work for schemas with multiple providers
    #[arg(value_name = "PROVIDER-VERSION")]
    provider_version: Option<String>, //TODO(vkleen): This is not going to work for schemas with multiple providers
    #[arg(value_name = "FILE")]
    schema: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let opts = Args::parse();
    let schema_reader: Box<dyn Read> = if let Some(path) = opts.schema {
        Box::new(std::fs::File::open(path)?)
    } else {
        Box::new(std::io::stdin())
    };

    let mut schema: TFSchema = serde_json::from_reader(schema_reader)?;
    schema.add_metaarguments();
    let pretty_ncl_schema: BoxDoc =
        ProviderNameVersion::new(opts.provider, opts.provider_version, schema)
            .as_nickel()
            .pretty(&BoxAllocator)
            .into_doc();
    pretty_ncl_schema.render(80, &mut stdout())?;
    println!("");

    Ok(())
}
