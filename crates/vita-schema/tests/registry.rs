//! Determinism / dedup / collision behaviour, exercised with hand-written impls
//! (no derive dependency — this crate is pure runtime).
use vita_schema::{schema_hash, SchemaShape, ShapeRegistry};

// Leaf type: `demo::Flags` newtype(u8).
struct Flags;
impl SchemaShape for Flags {
    fn schema_name() -> &'static str {
        "demo::Flags"
    }
    fn local_shape() -> &'static str {
        "repr=@#[]newtype(#[]u8)"
    }
    fn register(reg: &mut ShapeRegistry) {
        reg.insert_once(Self::schema_name(), Self::local_shape());
    }
}

// Parent type referencing Flags by name, twice (dedup must intern once).
struct Pair;
impl SchemaShape for Pair {
    fn schema_name() -> &'static str {
        "demo::Pair"
    }
    fn local_shape() -> &'static str {
        "repr=@#[]struct{#[]a:demo::Flags,#[]b:demo::Flags}"
    }
    fn register(reg: &mut ShapeRegistry) {
        if reg.insert_once(Self::schema_name(), Self::local_shape()) {
            <Flags as SchemaShape>::register(reg);
            <Flags as SchemaShape>::register(reg); // second call is a no-op (dedup)
        }
    }
}

#[test]
fn canonical_string_is_sorted_and_self_identifying() {
    let mut reg = ShapeRegistry::new();
    Pair::register(&mut reg);
    let s = reg.canonical_string();
    // sentinel first, then fqname-sorted entries, each `name=shape`, '\n' separated.
    assert_eq!(
        s,
        "vita-schema-v1\n\
         demo::Flags=repr=@#[]newtype(#[]u8)\n\
         demo::Pair=repr=@#[]struct{#[]a:demo::Flags,#[]b:demo::Flags}\n"
    );
}

#[test]
fn hash_is_stable_and_order_independent() {
    let h1 = schema_hash::<Pair>();
    let h2 = schema_hash::<Pair>();
    assert_eq!(h1, h2);
    // Hashing the same canonical string must equal blake3 of that string.
    let mut reg = ShapeRegistry::new();
    Pair::register(&mut reg);
    let expect: [u8; 32] = blake3::hash(reg.canonical_string().as_bytes()).into();
    assert_eq!(h1, expect);
}

#[test]
#[should_panic(expected = "collision")]
fn name_collision_with_differing_shape_panics() {
    let mut reg = ShapeRegistry::new();
    reg.insert_once("demo::Flags", "repr=@#[]newtype(#[]u8)");
    reg.insert_once("demo::Flags", "repr=@#[]newtype(#[]u16)"); // same name, diff shape -> panic
}
