use clap::Parser;
use pretty::{BoxAllocator, BoxDoc, Pretty};
use std::{
    io::{stdout, Read},
    path::PathBuf,
};
use tf_ncl::{nickel::AsNickel, terraform::TFSchema};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
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
    let schema: TFSchema = serde_json::from_reader(schema_reader)?;
    let pretty_ncl_schema: BoxDoc = schema.as_nickel().pretty(&BoxAllocator).into_doc();
    pretty_ncl_schema.render(80, &mut stdout())?;
    println!("");

    Ok(())
}
