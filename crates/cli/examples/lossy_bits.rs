//! A measurement entry, not a product surface (§5.2 Q-LOSSYSPELL, the plan's M1): print the
//! parser's `TopItem::LossySpell` bits for one file list, read the way the one-shot `vita`
//! reads it — every source with `read_to_string`, `-I` / `-D` into the preprocessor, all
//! sources one compilation unit — then the steps of `cli::frontend_sources_mapped`
//! (`hdl_preprocess::preprocess_sources`, `hdl_lexer::lex`, `hdl_parser::parse_with_warnings`).
//! The bits are printed whether or not the parse reported errors: the four token bits are a
//! function of the whole token vector; `sign_kw_param` holds what the parse reached. No
//! production code reads the record; this is no CLI flag.
//!
//! `cargo run --locked -p cli --example lossy_bits -- [-I<dir> | -I <dir>] [-D<n>[=v] | -D <n>]
//! [--top <m>] [--timeout <n>] [+plusarg] <file>...` prints one line:
//! `BITS param_xz=<0|1> unsized_xz=<0|1> sign_kw_param=<0|1> let_any=<0|1> fill_xz=<0|1>
//! items=<0|1> parse_errors=<n>`, or `NOTOKENS pp_errors=<n> lex_errors=<n>` when the
//! preprocessor or the lexer failed (no token vector; the first message on stderr).

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut incdirs = Vec::new();
    let mut defines = Vec::new();
    let mut files = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == "-I" {
            incdirs.extend(it.next().map(std::path::PathBuf::from));
        } else if let Some(d) = a.strip_prefix("-I") {
            incdirs.push(std::path::PathBuf::from(d));
        } else if let Some(d) = a.strip_prefix("-D") {
            let tok = if d.is_empty() {
                it.next().unwrap_or_default()
            } else {
                d.to_string()
            };
            defines.push(match tok.split_once('=') {
                Some((n, v)) => (n.to_string(), v.to_string()),
                None => (tok, String::new()),
            });
        } else if a == "--top" || a == "--timeout" {
            it.next();
        } else if !a.starts_with('+') {
            files.push(a);
        }
    }
    let mut sources = Vec::new();
    for f in &files {
        match std::fs::read_to_string(f) {
            Ok(s) => sources.push((f.clone(), s)),
            Err(e) => {
                println!("NOTOKENS pp_errors=1 lex_errors=0");
                eprintln!("cannot read '{f}': {e}");
                std::process::exit(1);
            }
        }
    }
    let pre = hdl_preprocess::PreOpts {
        incdirs,
        cli_defines: defines,
        ..hdl_preprocess::PreOpts::default()
    };
    // As `frontend_sources_mapped`: includes resolve from the first source's directory.
    let first = sources.first().map(|(n, _)| n.as_str()).unwrap_or("");
    let base = std::path::Path::new(first)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let pp = hdl_preprocess::preprocess_sources(base, &sources, &pre);
    if pp.has_errors() {
        println!("NOTOKENS pp_errors={} lex_errors=0", pp.diags.len());
        if let Some(d) = pp.diags.first() {
            eprintln!("{d:?}");
        }
        std::process::exit(1);
    }
    let (tokens, lex_errors) = hdl_lexer::lex(&pp.text);
    if !lex_errors.is_empty() {
        println!("NOTOKENS pp_errors=0 lex_errors={}", lex_errors.len());
        std::process::exit(1);
    }
    let (unit, parse_errors, _) = hdl_parser::parse_with_warnings(&tokens, &pp.text);
    let found: Vec<hdl_ast::LossySpell> = unit
        .iter()
        .flat_map(|u| u.items.iter())
        .filter_map(|i| match i {
            hdl_ast::TopItem::LossySpell(ls) => Some(*ls),
            _ => None,
        })
        .collect();
    let b = found.first().copied().unwrap_or_default();
    let f = |x: bool| u8::from(x);
    println!(
        "BITS param_xz={} unsized_xz={} sign_kw_param={} let_any={} fill_xz={} items={} \
         parse_errors={}",
        f(b.param_xz),
        f(b.unsized_xz),
        f(b.sign_kw_param),
        f(b.let_any),
        f(b.fill_xz),
        found.len(),
        parse_errors.len()
    );
}
