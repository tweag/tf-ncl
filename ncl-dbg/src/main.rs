use std::io::Cursor;

use nickel_lang::term::RichTerm;
use nickel_lang_utilities::{parse, eval};
use pretty::{Pretty, BoxAllocator, DocBuilder};

fn pretty(rt: &RichTerm) -> String {
    let allocator = BoxAllocator;
    let mut ret = Vec::new();
    let mut rt_pretty = Cursor::new(&mut ret);

    let doc: DocBuilder<_, ()> = rt.clone().pretty(&allocator);
    doc.render(80, &mut rt_pretty).unwrap();
    String::from_utf8_lossy(&ret).into_owned()
}

fn main() -> anyhow::Result<()> {
    let ncl_term = r#"
        let Contract = { foo | Num, opt | Str | optional } in
        let value | Contract = { foo = 1 } in
        record.has_field "opt" value
    "#;
    let parsed = parse(ncl_term).unwrap();
    println!("{:#?}", parsed.clone().without_pos());
    println!("{}", pretty(&parsed));
    println!("{:#?}", eval(ncl_term));
    Ok(())
}
