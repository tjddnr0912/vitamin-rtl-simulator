//! V-PRIM belt-and-suspenders: the SimIr canonical string must contain none of the
//! platform-variant tokens. The derive reject arm is the primary guard; this catches
//! any aliased re-introduction.
use vita_schema::{SchemaShape, ShapeRegistry};

#[test]
fn no_usize_isize_float_tokens() {
    let mut reg = ShapeRegistry::new();
    sim_ir::SimIr::register(&mut reg);
    let canon = reg.canonical_string();
    for tok in ["usize", "isize", "f32", "f64"] {
        assert!(
            !canon.contains(tok),
            "forbidden token `{tok}` in SimIr schema"
        );
    }
}
