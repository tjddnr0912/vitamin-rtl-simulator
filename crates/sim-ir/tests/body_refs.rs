//! F1 guard (final-review): machine-enforce the body-ref == registry-key invariant.
//!
//! The derive renders each child reference as the *spelled* field path, while the
//! registry key is `module_path!()::Ident`. For the frozen closure these agree only
//! because every cross-type field is spelled `sim_ir::Foo` and all types live at the
//! crate root (`extern crate self as sim_ir`). A future field spelled bare (`flags:
//! ProcFlags`), via `crate::`, or through a `use`-alias would emit a body reference
//! that never appears as a registry key — a silent dangling reference. These two
//! checks make that an explicit failure.
use vita_schema::{SchemaShape, ShapeRegistry};

/// (key, body) for every registry entry, from the canonical string.
fn entries() -> Vec<(String, String)> {
    let mut reg = ShapeRegistry::new();
    sim_ir::SimIr::register(&mut reg);
    reg.canonical_string()
        .lines()
        .skip(1) // drop the "vita-schema-v1" sentinel
        .filter_map(|l| {
            l.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect()
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Every `sim_ir::Ident` token appearing in a body must be a registry key.
/// Catches a typo'd or wrong-crate reference (e.g. `sim_ir::ProcFlag`).
#[test]
fn every_sim_ir_ref_is_a_registry_key() {
    let entries = entries();
    let keys: std::collections::BTreeSet<&str> = entries.iter().map(|(k, _)| k.as_str()).collect();
    const PAT: &str = "sim_ir::";
    for (name, body) in &entries {
        let mut rest = body.as_str();
        while let Some(p) = rest.find(PAT) {
            let after = &rest[p + PAT.len()..];
            let ident: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            let token = format!("{PAT}{ident}");
            assert!(
                keys.contains(token.as_str()),
                "body of {name} references `{token}` which is not a registry key (dangling ref)"
            );
            rest = &after[ident.len()..];
        }
    }
}

/// No bare user-type ident (a registry short-name like `ProcFlags`) may appear in a
/// body un-prefixed by `sim_ir::`. Catches a bare/`crate::`/aliased spelling that
/// would silently diverge from the child's schema_name key.
#[test]
fn no_bare_user_type_refs_in_bodies() {
    let entries = entries();
    let short_idents: Vec<String> = entries
        .iter()
        .map(|(k, _)| k.rsplit("::").next().unwrap().to_string())
        .collect();
    for (name, body) in &entries {
        let bytes = body.as_bytes();
        for ident in &short_idents {
            for (pos, _) in body.match_indices(ident.as_str()) {
                // whole-word occurrence?
                let before_word = pos > 0 && is_word_byte(bytes[pos - 1]);
                let end = pos + ident.len();
                let after_word = end < bytes.len() && is_word_byte(bytes[end]);
                if before_word || after_word {
                    continue; // substring of a larger identifier, not a standalone ref
                }
                // Type-position only. A user-type reference always immediately follows
                // ':' (named field / the second ':' of `sim_ir::`), '<' (generic arg),
                // or ',' (tuple/multi-generic arg). An enum VARIANT NAME follows ']'
                // (the closing of its `#[]` attr slot) — e.g. WaitCause::Expr renders
                // `#[]Expr{..}`, where `Expr` equals the type short-name but is a variant,
                // not a reference. Skip non-type positions so variant names never trip.
                if pos == 0 || !matches!(bytes[pos - 1], b':' | b'<' | b',') {
                    continue;
                }
                let fq = pos >= 8 && &body[pos - 8..pos] == "sim_ir::";
                assert!(
                    fq,
                    "body of {name} has bare user-type ref `{ident}` (must be sim_ir::{ident}); body={body}"
                );
            }
        }
    }
}
