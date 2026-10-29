//! package imports (explicit and wildcard type twins) and the package-binding type key — split out of `module_items.rs` (mechanical move, §5.2 row 66-0).

use super::*;

/// Copy each `prefix`-keyed twin in `map` (`p::st`) to its bare name (`st`) as a
/// wildcard import's per-name decision in `binds` says. A name with no decision is not
/// a type the package exports (a layout it keeps for a type it imported).
fn copy_wildcard_twins<V: Clone>(
    map: &mut std::collections::HashMap<String, V>,
    prefix: &str,
    binds: &std::collections::BTreeMap<String, WildcardBind>,
) {
    let twins: Vec<(String, V)> = map
        .iter()
        .filter_map(|(k, v)| k.strip_prefix(prefix).map(|b| (b.to_string(), v.clone())))
        .collect();
    for (b, v) in twins {
        match binds.get(&b) {
            Some(WildcardBind::Replace) => {
                map.insert(b, v);
            }
            Some(WildcardBind::IfAbsent) => {
                map.entry(b).or_insert(v);
            }
            Some(WildcardBind::Skip) | None => {}
        }
    }
}

impl Parser<'_, '_> {
    /// v7 P2-D: `import pkg::*;` / `import pkg::sym;` — the whole statement,
    /// INCLUDING a comma list (IEEE 1800 §26.8 `package_import_declaration` is
    /// `import package_import_item { , package_import_item } ;`). Each term
    /// becomes its own `ImportDecl`; every consumer is per-decl and
    /// order-independent, so N terms behave exactly like N statements.
    pub(crate) fn parse_import_decl_list(&mut self) -> Option<Vec<ImportDecl>> {
        self.bump(); // import
        let mut out = Vec::new();
        loop {
            // A malformed term aborts the STATEMENT: the caller's recovery
            // synchronises on the next item, and continuing the comma loop from
            // an unknown cursor position would report the rest as garbage.
            out.push(self.parse_import_term()?);
            if self.peek() == Some(TokenKind::Comma) {
                self.bump();
                continue;
            }
            break;
        }
        if !self.expect(TokenKind::Semi, "';'") {
            return None;
        }
        Some(out)
    }

    /// §4.5.434: a bare type name the compilation unit declared, not redeclared by
    /// this module — a wildcard import's twin of that name REPLACES it (§26.3).
    pub(crate) fn cu_type_overridable(&self, bare: &str) -> bool {
        self.cu_type_names.contains(bare) && !self.local_decl_names.contains(bare)
    }

    /// How a wildcard import binds `bare`, the bare name of one of its package's types —
    /// one answer for every type map, so a name the scope declares as one kind never
    /// gains another kind's layout from the package. At the unit scope every name in
    /// `cu_type_names` is the unit's own, so `Replace` is reached from a container only.
    pub(crate) fn wildcard_type_bind(&self, bare: &str) -> WildcardBind {
        if self.scope_type_names.contains(bare) {
            WildcardBind::Skip
        } else if self.cu_type_overridable(bare) {
            WildcardBind::Replace
        } else {
            WildcardBind::IfAbsent
        }
    }

    /// One `pkg::sym` / `pkg::*` term, no `import` keyword and no `;`.
    pub(crate) fn parse_import_term(&mut self) -> Option<ImportDecl> {
        let start = self.cur_span();
        let pkg = self.ident()?;
        if !self.expect(TokenKind::ColonColon, "'::'") {
            return None;
        }
        let item = if self.peek() == Some(TokenKind::Star) {
            self.bump();
            None
        } else {
            Some(self.ident()?)
        };
        // Bring the package's TYPE names into the current bare scope at parse time
        // (type names are parse-resolved). A package body's bare typedefs are now
        // unit-scoped (`restore_scope_unit` drops them, keeping only the `pkg::t`
        // twins), so `import p::t;` / `import p::*;` must copy the scoped twin back
        // to its bare name for a later `t x;` to parse. Value/const imports are
        // still handled at elaborate from the returned `ImportDecl`.
        let prefix = format!("{}::", pkg.name);
        match &item {
            Some(name) => {
                let scoped = format!("{prefix}{}", name.name);
                let bare = name.name.clone();
                // IEEE §26.3: the name is now bound in this scope, so a later wildcard
                // import here leaves it alone (`wildcard_type_bind`).
                self.scope_type_names.insert(bare.clone());
                self.note_type_rebound(&bare);
                // §5.2 row 36: the import is the name's newest registration.
                let td_from = self.typedefs.get(&scoped).map(|i| i.bind);
                let un_from = self.imported_record_bind(&scoped);
                if let Some(v) = self.typedefs.get(&scoped).cloned() {
                    self.typedefs.insert(bare.clone(), v);
                }
                if let Some(v) = self.struct_layouts.get(&scoped).cloned() {
                    self.struct_layouts.insert(bare.clone(), v);
                }
                if let Some(v) = self.enum_defs.get(&scoped).cloned() {
                    self.enum_defs.insert(bare.clone(), v);
                }
                // G6: UNPACKED struct typedefs live only in `unpacked_struct_layouts`
                // (never in `typedefs`), so a bare `rec_t r;` after `import p::rec_t`
                // needs the scoped twin copied here too, exactly like the packed kinds.
                if let Some(v) = self.unpacked_struct_layouts.get(&scoped).cloned() {
                    self.unpacked_struct_layouts.insert(bare.clone(), v);
                }
                self.restamp_imported_type(&bare, td_from, un_from);
                if self.union_type_names.contains(&scoped) {
                    self.union_type_names.insert(bare);
                }
            }
            None => {
                // Wildcard `import p::*` — copy every type p exports, `p::X`, to bare
                // `X`, as `wildcard_type_bind` decides ONCE per name for every map:
                // never over a type this scope binds itself (§5.2 row 1), over a
                // unit-scope binding from a container (§4.5.434), else only where the
                // map has no entry (an earlier wildcard's binding stays). p exports a
                // type when it has a `typedefs` twin (every packed kind, an enum, a
                // type parameter) or an unpacked-struct twin. A `p::X` key only in
                // `struct_layouts` / `enum_defs` / `union_type_names` is the layout of
                // a type p IMPORTED, kept for its own variables' replay
                // (`pkg_binding_type_key`); p does not re-export it (IEEE §26.3), so no
                // bare name learns it here — with a unit `typedef st`, `import p::*`
                // wrote base's layout under the unit's `st` (`12 2 2 003` where all
                // three oracles print `12 4 8 0ff`).
                let offered: std::collections::BTreeSet<String> = self
                    .typedefs
                    .keys()
                    .chain(self.unpacked_struct_layouts.keys())
                    .filter_map(|k| k.strip_prefix(&prefix).map(str::to_string))
                    .collect();
                let binds: std::collections::BTreeMap<String, WildcardBind> = offered
                    .iter()
                    .map(|b| (b.clone(), self.wildcard_type_bind(b)))
                    .collect();
                for (b, bind) in &binds {
                    if *bind == WildcardBind::Replace {
                        self.note_type_rebound(b);
                    }
                }
                // §5.2 row 36: which of a bound name's entries this import writes, and
                // their package order — the import is the name's newest registration.
                let stamps: Vec<(String, Option<u64>, Option<u64>)> = binds
                    .iter()
                    .filter(|(_, bind)| {
                        matches!(bind, WildcardBind::Replace | WildcardBind::IfAbsent)
                    })
                    .map(|(b, bind)| {
                        let writes = |present: bool| *bind == WildcardBind::Replace || !present;
                        let scoped = format!("{prefix}{b}");
                        let td = self
                            .typedefs
                            .get(&scoped)
                            .filter(|_| writes(self.typedefs.contains_key(b)))
                            .map(|i| i.bind);
                        let un = self
                            .imported_record_bind(&scoped)
                            .filter(|_| writes(self.unpacked_struct_layouts.contains_key(b)));
                        (b.clone(), td, un)
                    })
                    .collect();
                copy_wildcard_twins(&mut self.typedefs, &prefix, &binds);
                copy_wildcard_twins(&mut self.struct_layouts, &prefix, &binds);
                copy_wildcard_twins(&mut self.enum_defs, &prefix, &binds);
                // G6: wildcard-copy UNPACKED struct typedefs too (see the explicit arm).
                copy_wildcard_twins(&mut self.unpacked_struct_layouts, &prefix, &binds);
                for (b, td, un) in stamps {
                    self.restamp_imported_type(&b, td, un);
                }
                let un: Vec<String> = self
                    .union_type_names
                    .iter()
                    .filter_map(|k| k.strip_prefix(&prefix).map(|b| b.to_string()))
                    .filter(|b| {
                        matches!(
                            binds.get(b),
                            Some(WildcardBind::Replace | WildcardBind::IfAbsent)
                        )
                    })
                    .collect();
                self.union_type_names.extend(un);
            }
        }
        // §3 ⑤: replay the package's struct/enum NAME bindings so a member access /
        // `'{…}` / enum method on an imported package variable or parameter desugars
        // here as it did in the package. A WILDCARD replays with `or_insert` (a
        // same-named local or earlier binding wins, like the type copies above). An
        // EXPLICIT `import r::X` WINS over a prior wildcard binding of `X` (IEEE
        // §26.8 — elaborate already gives it the VALUE), so it first DROPS whatever
        // `X` was bound to and then installs r's binding if r has one: without the
        // drop, `import q::*; import r::P;` read r's value through q's layout
        // (`P=122 lo=2 hi=12`, no simulator's answer), and a non-struct `r::P`
        // would keep desugaring `P.lo` against q's struct.
        // Only a binding a WILDCARD made is dropped (`wildcard_bound`): a local
        // declaration that shares the name — a struct variable named like a
        // package TYPE being imported explicitly, say — keeps its binding.
        if let Some(i) = &item {
            if self.wildcard_bound.remove(&i.name) {
                self.unbind_struct_enum_name(&i.name);
            }
        }
        if let Some(pb) = self.pkg_bindings.get(&pkg.name).cloned() {
            let explicit = item.is_some();
            let wanted = |n: &str| item.as_ref().is_none_or(|i| i.name == n);
            // The names whose `var_struct` binding THIS replay installed. The
            // shape sets below follow it, never the type NAME alone: a local
            // scalar `st_t P;` declared before `import p::*` (p exports `st_t
            // P[2]`) has the same type key, and a name-keyed gate put that scalar
            // into `struct_1d_array_vars` (review B3 — latent until a member can
            // itself be a struct; it would have diverted the scalar `'{…}` to the
            // per-element desugar).
            let mut landed: std::collections::HashSet<String> = std::collections::HashSet::new();
            for (n, ty) in pb.var_struct.clone() {
                if wanted(&n) {
                    if explicit {
                        self.var_struct.insert(n.clone(), ty);
                        self.wildcard_bound.remove(&n);
                        landed.insert(n);
                    } else if !self.var_struct.contains_key(&n)
                        && !self.local_decl_names.contains(&n)
                    {
                        self.var_struct.insert(n.clone(), ty);
                        self.wildcard_bound.insert(n.clone());
                        landed.insert(n);
                    }
                }
            }
            // The `'{…}` desugar sets follow the binding that actually landed: a
            // name whose `var_struct` entry stayed someone else's (a local struct
            // array or scalar, a union, an earlier import) keeps its own shape.
            for n in pb.struct_scalar.clone() {
                if landed.contains(&n) {
                    self.struct_scalar_vars.insert(n);
                }
            }
            for n in pb.struct_1d_array.clone() {
                if landed.contains(&n) {
                    self.struct_1d_array_vars.insert(n);
                }
            }
            for (n, ty) in pb.var_enum {
                if wanted(&n) {
                    if explicit {
                        self.var_enum.insert(n.clone(), ty);
                        self.wildcard_bound.remove(&n);
                    } else if !self.var_enum.contains_key(&n) && !self.local_decl_names.contains(&n)
                    {
                        self.var_enum.insert(n.clone(), ty);
                        self.wildcard_bound.insert(n);
                    }
                }
            }
            // §3 ⑤ ⓐ: a multi-dimensional packed parameter's dims cross the import
            // under the same rules as `var_enum` (explicit wins; a wildcard never
            // binds over a local declaration or an earlier binding).
            for (n, dims) in pb.packed_md {
                if wanted(&n) {
                    if explicit {
                        self.packed_md_params.insert(n.clone(), dims);
                        self.wildcard_bound.remove(&n);
                    } else if !self.packed_md_params.contains_key(&n)
                        && !self.local_decl_names.contains(&n)
                    {
                        self.packed_md_params.insert(n.clone(), dims);
                        self.wildcard_bound.insert(n);
                    }
                }
            }
            // §3 ⑤ ⓓ: the package's literal-valued constants, same rules. Two
            // wildcards exporting the same name with DIFFERENT values make the name
            // ambiguous (IEEE §26.3) — the entry is dropped so the read declines
            // (loud), never the first package's value.
            for (n, v) in pb.consts {
                if wanted(&n) {
                    if explicit {
                        self.const_locals.insert(n.clone(), v);
                        self.wildcard_bound.remove(&n);
                    } else if self.local_decl_names.contains(&n) {
                        // a local declaration wins over a wildcard
                    } else if let Some(prev) = self.const_locals.get(&n).copied() {
                        if prev != v && self.wildcard_bound.contains(&n) {
                            self.const_locals.remove(&n);
                        }
                    } else {
                        self.const_locals.insert(n.clone(), v);
                        self.wildcard_bound.insert(n);
                    }
                }
            }
        }
        Some(ImportDecl {
            pkg,
            item,
            span: start.to(self.prev_span()),
        })
    }

    /// The stable key a package's struct/enum NAME binding records for type `ty`
    /// (see the `pkg_bindings` capture in `parse_module_like`): the `pkg::ty` twin,
    /// registered from the bare layout/labels in scope when this package did not
    /// define `ty` itself (it imported it). An already-scoped `ty` is returned as is.
    pub(crate) fn pkg_binding_type_key(&mut self, pkg: &str, ty: &str) -> String {
        if ty.contains("::") {
            return ty.to_string();
        }
        let scoped = format!("{pkg}::{ty}");
        if !self.struct_layouts.contains_key(&scoped) {
            if let Some(l) = self.struct_layouts.get(ty).cloned() {
                self.struct_layouts.insert(scoped.clone(), l);
                if self.union_type_names.contains(ty) {
                    self.union_type_names.insert(scoped.clone());
                }
            }
        }
        if !self.enum_defs.contains_key(&scoped) {
            if let Some(e) = self.enum_defs.get(ty).cloned() {
                self.enum_defs.insert(scoped.clone(), e);
            }
        }
        if self.struct_layouts.contains_key(&scoped) || self.enum_defs.contains_key(&scoped) {
            scoped
        } else {
            ty.to_string()
        }
    }
}
