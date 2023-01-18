use clap::Parser;
use core::fmt;
use pretty::{BoxAllocator, BoxDoc, Pretty};
use std::{
    io::{self, stdout, Read},
    path::PathBuf,
};
use tf_ncl::{
    intermediate::{GoSchema, IntoWithProviders, Providers, WithProviders},
    nickel::AsNickel,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_name = "REQUIRED-PROVIDERS")]
    providers: PathBuf,
    #[arg(value_name = "TERRAFORM-SCHEMA")]
    schema: Option<PathBuf>,
}

fn get_providers(opts: &Args) -> anyhow::Result<Providers> {
    Ok(serde_json::from_reader(std::fs::File::open(
        &opts.providers,
    )?)?)
}

fn get_schema(opts: &Args) -> anyhow::Result<GoSchema> {
    let schema_reader: Box<dyn Read> = if let Some(path) = &opts.schema {
        Box::new(std::fs::File::open(path)?)
    } else {
        Box::new(std::io::stdin())
    };

    Ok(serde_json::from_reader(schema_reader)?)
}

struct RenderableSchema<'a> {
    schema: BoxDoc<'a>,
    providers: BoxDoc<'a>,
}

impl<'a> RenderableSchema<'a> {
    fn render(&self, f: &mut impl io::Write) -> anyhow::Result<()> {
        let tfncl_lib = include_str!("../../ncl/lib.ncl");

        write!(
            f,
            "{{
    Config = {{
        config | Schema,
        renderable_config = TfNcl.mkConfig config,
        ..
    }},
    Schema =
{schema},
    TfNcl = {tfncl_lib} & {{
        mkConfig | Schema -> {{_: Dyn}}
                 = fun v => v |> TfNcl.resolve_provider_computed,
    }},
    required_providers = {required_providers}
}}",
            schema = Display(&self.schema),
            required_providers = Display(&self.providers)
        )?;
        Ok(())
    }
}

impl<'a> From<WithProviders<GoSchema>> for RenderableSchema<'a> {
    fn from(s: WithProviders<GoSchema>) -> Self {
        RenderableSchema {
            schema: s.as_nickel().pretty(&BoxAllocator).into_doc(),
            providers: s.providers.as_nickel().pretty(&BoxAllocator).into_doc(),
        }
    }
}

struct Display<'a, 'b>(&'b BoxDoc<'a>);

impl<'a, 'b> fmt::Display for Display<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.render_fmt(80, f)
    }
}

fn main() -> anyhow::Result<()> {
    let opts = Args::parse();

    let providers = get_providers(&opts)?;
    let go_schema = get_schema(&opts)?;

    let doc: RenderableSchema = go_schema.with_providers(providers).into();

    doc.render(&mut stdout())
}
