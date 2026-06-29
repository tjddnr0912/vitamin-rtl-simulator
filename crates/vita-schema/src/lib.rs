//! vita-schema — structural schema-shape registry + blake3 hash (leaf; dep: blake3).
//!
//! `#[derive(SchemaHash)]` (in vita-artifact-derive) implements `SchemaShape` for
//! every participating sim-ir / hdl-ast type. The hash collapses the whole
//! type-reachability closure (acyclic DAG) into one 32-byte blake3 value.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// Implemented by every participating serde type. Children referenced by name,
/// never inlined (so a nested change flips the registry entry, hence the root).
pub trait SchemaShape {
    /// Rename-STABLE canonical key, `concat!(module_path!(),"::",Ident)`.
    fn schema_name() -> &'static str;
    /// LOCAL shape body only (this type's fields/variants + own serde attrs).
    /// Children appear as their `schema_name()` reference strings, not inlined.
    fn local_shape() -> &'static str;
    /// Register self then recurse into each distinct child (DFS, insert_once-guarded).
    fn register(reg: &mut ShapeRegistry);
}

/// Order-stable registry. No HashMap/HashSet anywhere (3-OS byte stability).
pub struct ShapeRegistry {
    entries: BTreeMap<&'static str, &'static str>, // schema_name -> local_shape
    visited: BTreeSet<&'static str>,
}

impl ShapeRegistry {
    pub fn new() -> Self {
        ShapeRegistry {
            entries: BTreeMap::new(),
            visited: BTreeSet::new(),
        }
    }

    /// Insert this type once. Returns false if already present (short-circuits DFS).
    /// Re-registration MUST carry an identical shape, else two types alias one name.
    pub fn insert_once(&mut self, name: &'static str, shape: &'static str) -> bool {
        if !self.visited.insert(name) {
            // plain assert (survives release builds) — a real collision is a correctness bug.
            assert_eq!(
                self.entries.get(name),
                Some(&shape),
                "SchemaHash name collision: {name} registered with two different shapes"
            );
            return false;
        }
        self.entries.insert(name, shape);
        true
    }

    /// Deterministic canonical string: sentinel + fqname-sorted `name=shape\n` lines.
    pub fn canonical_string(&self) -> String {
        let mut s = String::new();
        s.push_str("vita-schema-v1\n"); // format-version sentinel + non-empty baseline
        for (name, shape) in &self.entries {
            // BTreeMap iteration = fqname lexicographic = identical on every OS.
            s.push_str(name);
            s.push('=');
            s.push_str(shape);
            s.push('\n'); // literal 0x0A, never CRLF
        }
        s
    }
}

impl Default for ShapeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Drive the reachability closure from `T`, canonicalize, hash once.
pub fn schema_hash<T: SchemaShape>() -> [u8; 32] {
    let mut reg = ShapeRegistry::new();
    T::register(&mut reg);
    blake3::hash(reg.canonical_string().as_bytes()).into()
}

/// One-time cached hash for a fixed root. (blake3::hash is not a const fn, so the
/// value is produced on first access rather than as a literal const.)
pub fn cached_hash<T: SchemaShape>(cell: &'static OnceLock<[u8; 32]>) -> [u8; 32] {
    *cell.get_or_init(schema_hash::<T>)
}
