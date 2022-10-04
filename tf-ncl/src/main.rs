use clap::Parser;
use std::{io::Read, path::PathBuf};

use tf_ncl::terraform::TFSchema;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_name = "FILE")]
    schema: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let opts = Args::parse();
    let schema_reader: Box<dyn Read> = match opts.schema {
        Some(path) => Box::new(std::fs::File::open(path)?),
        None => Box::new(std::io::stdin()),
    };
    let schema: TFSchema = serde_json::from_reader(schema_reader)?;
    for (provider, schema) in schema.provider_schemas {
        println!("{}", provider);
        println!("{:?}", schema.provider.block);
        println!();
        for (data_source, schema) in schema.data_source_schemas.iter().flatten() {
            println!("{}", data_source);
            for (n, a) in schema.block.attributes.iter().flatten() {
                println!("  {}", n);
                println!("    type: {:?}", a.r#type);
                println!("    required: {}", a.required);
                println!("    optional: {}", a.optional);
                println!("    computed: {}", a.computed);
                println!("    sensitive: {}", a.sensitive);
            }
        }
    }
    Ok(())
}
