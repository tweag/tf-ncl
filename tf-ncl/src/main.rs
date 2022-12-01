use clap::Parser;
use core::fmt;
use pretty::{BoxAllocator, BoxDoc, DocBuilder, Pretty};
use std::{
    io::{self, stdout, Read},
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

struct RenderableSchema<'a>(BoxDoc<'a>);

impl<'a> RenderableSchema<'a> {
    fn render(&self, f: &mut impl io::Write) -> anyhow::Result<()> {
        write!(f,
"let remove_if_exists = fun key r =>
      if builtin.is_record r && record.has_field key r
      then record.remove key r
      else r
    in
let maybe_record_map = fun f v =>
      if builtin.is_record v
      then record.map f v
      else v
    in
let addIdField__ = fun l x =>
      x |> record.map (fun res_type r =>
      r |> record.map (fun name r => r & {{ \"id\" | force = \"${{%{{res_type}}.%{{name}}.id}}\" }}))
    in
{{
    Config = {{
        config | Schema,
        renderable_config = mkConfig config,
        ..
    }},
    Schema = 
{schema},
    mkConfig | Schema -> {{_: Dyn}}
             = (maybe_record_map (fun k v =>
                v |> maybe_record_map (fun res_type v =>
                  v |> maybe_record_map (fun res_name v =>
                    v |> remove_if_exists \"id\")))),
}}", schema = self)?;
        Ok(())
    }
}

impl<'a> From<DocBuilder<'a, BoxAllocator>> for RenderableSchema<'a> {
    fn from(d: DocBuilder<'a, BoxAllocator>) -> Self {
        RenderableSchema(d.into_doc())
    }
}

impl<'a> fmt::Display for RenderableSchema<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.render_fmt(80, f)
    }
}

fn main() -> anyhow::Result<()> {
    let opts = Args::parse();

    let providers = get_providers(&opts)?;
    let schema = get_schema(&opts)?;

    let doc: RenderableSchema = schema
        .with_providers(providers)
        .as_nickel()
        .pretty(&BoxAllocator)
        .into();

    doc.render(&mut stdout())
}
