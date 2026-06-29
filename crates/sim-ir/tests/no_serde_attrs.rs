//! 16 §312: every frozen type carries only empty `#[]` attr slots. Root-walk the
//! whole SimIr closure so new M3 types are covered automatically.
use vita_schema::{SchemaShape, ShapeRegistry};

#[test]
fn frozen_types_are_attr_free() {
    let mut reg = ShapeRegistry::new();
    sim_ir::SimIr::register(&mut reg);
    for line in reg.canonical_string().lines().skip(1) {
        let (name, shape) = line.split_once('=').unwrap();
        // every `#[...]` slot must be empty `#[]`
        let mut i = 0;
        let b = shape.as_bytes();
        while let Some(p) = shape[i..].find("#[") {
            let open = i + p + 2;
            assert!(
                b.get(open) == Some(&b']'),
                "{name} carries a serde attr; frozen cluster must be attr-free: {shape}"
            );
            i = open + 1;
        }
    }
}
