use nickel_lang_utilities::{parse, eval};

fn main() -> anyhow::Result<()> {
    let ncl_term = r#"
        contract."$dyn_record" Str
    "#;
    println!("{:#?}", parse(ncl_term));
    println!("{:#?}", eval(ncl_term));
    Ok(())
}
