//! A measurement entry, not a product surface (§5.2 Q-LOSSYSPELL, the plan's M1 on the
//! built tree): print the DESIGN record `Sidecars::lossy_spell` for one command line, as
//! the one-shot `vita` would compute it — every source with `read_to_string`, `-I` / `-D`
//! into the preprocessor, all sources one compilation unit, `-G NAME=VALUE` into the top
//! parameter overrides — through `elaborate::elaborate_located_params`, whose `run` writes
//! the record at its head. No production code reads the record; this is no CLI flag.
//!
//! Two modes, both through `Elaborator::run`:
//! - `full`: the front end accepts the design (`cli::frontend_sources_mapped`), so `vita`
//!   elaborates it: the whole unit is elaborated with the command's `--top` and `-G`.
//! - `items`: the front end refuses it (a parse error), so `vita` never elaborates it and
//!   no run of `vita` has a record. The record `run`'s head WOULD compute is printed: the
//!   parser's unit (preprocess, lex, `parse_with_warnings`, as `lossy_bits` reads it) is
//!   cut to its `TopItem::LossySpell` items — the only items the head's OR reads — and
//!   elaborated with the command's `-G`.
//!
//! In `full` mode the `items` value is computed too and printed as `items=<bits>`: equal
//! bits are the check that the head reads nothing else.
//!
//! `cargo run --locked -p cli --example lossy_design -- [-I<dir> | -I <dir>] [-D<n>[=v] |
//! -D <n>] [-G <n>=<v> | -G<n>=<v>] [--top <m>] [--timeout <n>] [+plusarg] <file>...`
//! prints one line: `DESIGN bits=<PUSLF> mode=<full|items> items=<PUSLF> parse_errors=<n>
//! elab_ok=<0|1> g=<count>`, or `NOTOKENS pp_errors=<n> lex_errors=<n>`. Bits are in the
//! order param_xz, unsized_xz, sign_kw_param, let_any, fill_xz.

use diag::{LogEvent, LogSink};

struct Quiet;
impl LogSink for Quiet {
    fn emit(&self, _: LogEvent) {}
}

fn bits(ls: hdl_ast::LossySpell) -> String {
    [
        ls.param_xz,
        ls.unsized_xz,
        ls.sign_kw_param,
        ls.let_any,
        ls.fill_xz,
    ]
    .iter()
    .map(|&b| if b { '1' } else { '0' })
    .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut incdirs = Vec::new();
    let mut defines = Vec::new();
    let mut gparams: Vec<(String, String)> = Vec::new();
    let mut tops: Vec<String> = Vec::new();
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
        } else if let Some(g) = a.strip_prefix("-G") {
            let tok = if g.is_empty() {
                it.next().unwrap_or_default()
            } else {
                g.to_string()
            };
            if let Some((n, v)) = tok.split_once('=') {
                gparams.push((n.to_string(), v.to_string()));
            }
        } else if a == "--top" {
            tops.extend(it.next());
        } else if a == "--timeout" {
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
    // The parser's unit, as `lossy_bits` reads it (whether or not the parse reported errors).
    let first = sources.first().map(|(n, _)| n.as_str()).unwrap_or("");
    let base = std::path::Path::new(first)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let pp = hdl_preprocess::preprocess_sources(base, &sources, &pre);
    if pp.has_errors() {
        println!("NOTOKENS pp_errors={} lex_errors=0", pp.diags.len());
        std::process::exit(1);
    }
    let (tokens, lex_errors) = hdl_lexer::lex(&pp.text);
    if !lex_errors.is_empty() {
        println!("NOTOKENS pp_errors=0 lex_errors={}", lex_errors.len());
        std::process::exit(1);
    }
    let (unit, parse_errors, _) = hdl_parser::parse_with_warnings(&tokens, &pp.text);
    let empty = std::collections::BTreeMap::new();
    let items_only = hdl_ast::SourceUnit {
        items: unit
            .iter()
            .flat_map(|u| u.items.iter())
            .filter(|i| matches!(i, hdl_ast::TopItem::LossySpell(_)))
            .cloned()
            .collect(),
        span: hdl_ast::Span { lo: 0, hi: 0 },
    };
    let (_, sc_items) = elaborate::elaborate_located_params(
        &items_only,
        &Quiet,
        &empty,
        &empty,
        -9,
        None,
        None,
        &gparams,
    );
    let items = bits(sc_items.lossy_spell);
    // `vita`'s own path: the front end, then the elaboration it would run.
    let (mode, design, elab_ok) = match cli::frontend_sources_mapped(&sources, &Quiet, &pre) {
        Some(((u, rt, _), _)) => {
            let roots = (!tops.is_empty()).then_some(tops.as_slice());
            let (ir, sc) = elaborate::elaborate_located_params(
                &u,
                &Quiet,
                &rt.unit_exp,
                &rt.prec_exp,
                rt.global_prec_exp,
                roots,
                None,
                &gparams,
            );
            ("full", bits(sc.lossy_spell), u8::from(ir.is_some()))
        }
        None => ("items", items.clone(), 0),
    };
    println!(
        "DESIGN bits={design} mode={mode} items={items} parse_errors={} elab_ok={elab_ok} g={}",
        parse_errors.len(),
        gparams.len()
    );
}
