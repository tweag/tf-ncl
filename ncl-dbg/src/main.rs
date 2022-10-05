use nickel_lang_utilities::parse;

fn main() -> anyhow::Result<()> {
    let ncl_term = parse(r"
        { test | | Bool
        }
    ").unwrap();
    println!("{:?}", ncl_term);
    Ok(())
}
