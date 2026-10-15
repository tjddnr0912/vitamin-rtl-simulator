//! §5.2 row 67a: the class scope. A class body binds its properties through the
//! enclosing scope's variable-keyed tables (V1–V10), so `parse_class_decl` snapshots
//! the scope after the class header and restores it at `endclass`, keeping a record
//! of what the body bound (`ClassRec`). Two places put a record back:
//!
//! - `extends B`: B's record is replayed into the derived class's body, so an
//!   inherited property stays in scope there.
//! - `h.randomize() with { … }`: the with-block's names resolve in the object's class
//!   first (IEEE 1800-2017 §18.7), so the record of `h`'s class is replayed around the
//!   block; when `h`'s class is not known, a block naming a name some visible class
//!   record binds is refused, and any other block parses where the call stands.
//!
//! A recorded binding names its TYPE by name (`var_struct[s] = "st2"`), and the
//! desugars re-read that name's tables where they run. A record replayed in another
//! scope — a `$unit` base into a module, or a base before a module typedef or import
//! of its property's type name — would read the type that scope binds to the name. So
//! each entry also records what those tables held for its types (`TypeView`, nested
//! member types included) and is re-bound only where they still hold the same; any
//! other entry is replayed as an unbind (loud, as before this row).

use super::*;

/// What a name-keyed desugar reads for one type name: the packed struct or union
/// layout (span-free members, nested member keys included), a per-instance layout, the
/// union flag, the enum labels and an unpacked record's members. Two views are equal
/// when a desugar through the name would build the same IR.
#[derive(Clone, PartialEq)]
pub(crate) struct TypeView {
    name: String,
    layout: Option<Vec<StructFieldLayout>>,
    sym: Option<SymStructLayout>,
    union: bool,
    labels: Option<Vec<(String, i64)>>,
    record: Option<Vec<StructMember>>,
}

/// One class property's entries in the variable-keyed binding tables (V1–V10) at
/// `endclass`, and the views of the types they name. All bindings empty = a property
/// no table binds (a vector, an int, …): replayed as an unbind, so an inherited plain
/// name hides the enclosing scope's struct variable of that name.
#[derive(Clone, Default)]
pub(crate) struct ClassBindState {
    var_struct: Option<String>,
    struct_scalar: bool,
    struct_1d_array: bool,
    struct_packed_array: bool,
    var_enum: Option<String>,
    packed_md: Option<Vec<Range>>,
    var_unpacked_struct: Option<String>,
    record_array: Option<String>,
    record_soa: Option<String>,
    var_class: Option<String>,
    views: Vec<TypeView>,
    /// The container the record was taken in (`Parser::cur_container`).
    container: Option<u64>,
}

impl ClassBindState {
    /// The type names the entry's bindings read (V1, V5, V7–V9).
    fn type_names(&self) -> Vec<String> {
        [
            &self.var_struct,
            &self.var_enum,
            &self.var_unpacked_struct,
            &self.record_array,
            &self.record_soa,
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect()
    }
}

/// What a class body bound, in binding order (first occurrence kept): its own
/// properties and the ones its `extends` replayed.
pub(crate) type ClassRec = Vec<(String, ClassBindState)>;

impl Parser<'_, '_> {
    /// A class property's declarators join the class record AFTER its declaration
    /// bound them — appended as bound, never derived from a difference of
    /// `local_decl_names` (which already holds every enclosing declaration's name, so a
    /// property shadowing one would drop out of the record).
    pub(crate) fn note_class_decl_names(&mut self, d: &NetVarDecl) {
        for n in &d.names {
            self.note_class_decl_name(&n.name.name);
        }
    }

    pub(crate) fn note_class_decl_name(&mut self, n: &str) {
        if let Some(v) = self.class_decl_names.as_mut() {
            v.push(n.to_string());
        }
    }

    /// The views of `roots` and of every nested member type their packed layouts
    /// name, transitively, in discovery order.
    fn type_views(&self, roots: Vec<String>) -> Vec<TypeView> {
        let mut seen: Vec<String> = Vec::new();
        let mut queue = roots;
        let mut out = Vec::new();
        while let Some(n) = queue.pop() {
            if seen.contains(&n) {
                continue;
            }
            seen.push(n.clone());
            let layout = self.struct_layouts.get(&n).map(|l| l.fields.clone());
            if let Some(fields) = &layout {
                for f in fields {
                    if let Some(k) = &f.8 {
                        queue.push(k.clone());
                    }
                }
            }
            out.push(TypeView {
                layout,
                sym: self.sym_struct_layouts.get(&n).cloned(),
                union: self.union_type_names.contains(&n),
                labels: self.enum_defs.get(&n).cloned(),
                record: self.unpacked_struct_layouts.get(&n).cloned(),
                name: n,
            });
        }
        out
    }

    /// The class record: each bound name once (first occurrence), in binding order,
    /// with its V1–V10 entries and their type views as they stand at `endclass`.
    pub(crate) fn class_record(&self, names: Vec<String>) -> ClassRec {
        let mut seen = std::collections::HashSet::new();
        let mut rec = Vec::new();
        for n in names {
            if !seen.insert(n.clone()) {
                continue;
            }
            let mut st = ClassBindState {
                var_struct: self.var_struct.get(&n).cloned(),
                struct_scalar: self.struct_scalar_vars.contains(&n),
                struct_1d_array: self.struct_1d_array_vars.contains(&n),
                struct_packed_array: self.struct_packed_array_vars.contains(&n),
                var_enum: self.var_enum.get(&n).cloned(),
                packed_md: self.packed_md_params.get(&n).cloned(),
                var_unpacked_struct: self.var_unpacked_struct.get(&n).cloned(),
                record_array: self.record_array_vars.get(&n).cloned(),
                record_soa: self.record_soa_vars.get(&n).cloned(),
                var_class: self.var_class.get(&n).cloned(),
                views: Vec::new(),
                container: self.cur_container,
            };
            st.views = self.type_views(st.type_names());
            rec.push((n, st));
        }
        rec
    }

    /// Put a class record back: every name is unbound first (`unbind_struct_enum_name`
    /// — what the property's own declaration did — and `unbind_bindings`), then
    /// re-bound only where its types still read as recorded; a name whose type the
    /// current scope binds to something else stays unbound (loud). `note` pushes each
    /// name onto the class being parsed (an `extends` replay).
    pub(crate) fn replay_class_record(&mut self, rec: &ClassRec, note: bool) {
        for (n, st) in rec {
            self.unbind_struct_enum_name(n);
            self.unbind_bindings(n);
            if self.type_views(st.type_names()) == st.views {
                let mut st = st.clone();
                if st.container != self.cur_container {
                    // A record taken in another container — a `$unit` class replayed
                    // into a module (`extends` or a with-block) — is traffic this row
                    // opened (the container's start cleared the unit's leak, so it
                    // was refused before). An enum method's result there is 32 bits
                    // wide instead of the enum's (ROADMAP §2 enum-method-width; a
                    // module's own enum variable reads the same width), so its enum
                    // binding is withheld and `e.next()` stays refused as before.
                    st.var_enum = None;
                }
                self.bind_class_state(n, st);
            }
            if note {
                self.note_class_decl_name(n);
            }
        }
    }

    /// Install a recorded property's V1–V10 entries (the caller unbound the name).
    fn bind_class_state(&mut self, n: &str, st: ClassBindState) {
        if let Some(t) = st.var_struct {
            self.var_struct.insert(n.to_string(), t);
        }
        if st.struct_scalar {
            self.struct_scalar_vars.insert(n.to_string());
        }
        if st.struct_1d_array {
            self.struct_1d_array_vars.insert(n.to_string());
        }
        if st.struct_packed_array {
            self.struct_packed_array_vars.insert(n.to_string());
        }
        if let Some(t) = st.var_enum {
            self.var_enum.insert(n.to_string(), t);
        }
        if let Some(d) = st.packed_md {
            self.packed_md_params.insert(n.to_string(), d);
        }
        if let Some(t) = st.var_unpacked_struct {
            self.var_unpacked_struct.insert(n.to_string(), t);
        }
        if let Some(t) = st.record_array {
            self.record_array_vars.insert(n.to_string(), t);
        }
        if let Some(t) = st.record_soa {
            self.record_soa_vars.insert(n.to_string(), t);
        }
        if let Some(c) = st.var_class {
            self.var_class.insert(n.to_string(), c);
        }
    }

    /// `<call> with { … }` (IEEE 1800-2017 §18.7): the block's names resolve in the
    /// randomized object's class first, then where the call stands. A bare
    /// `randomize()` and `this.` / `super.randomize()` stand in a method of the class,
    /// whose scope is already the parse scope. For `h.randomize()` with `h` a handle of
    /// class `C` declared where the call stands (`var_class`), C's record is replayed
    /// around the block (a class declared later has none yet: the call's scope, as
    /// before this row); for any other receiver a block naming a name some visible
    /// class record binds is refused (it could be the object's property or the
    /// caller's declaration), and any other block parses where the call stands. Out of
    /// line: the statement parser that calls it recurses per nesting level.
    #[inline(never)]
    pub(crate) fn parse_with_constraints_in_class(&mut self, call: &HierPath) -> Vec<Expr> {
        let scope = self.enter_with_block_scope(call);
        let constraints = self.parse_with_constraints();
        if let Some(s) = scope {
            self.restore_scope_boxed(s);
        }
        constraints
    }

    /// The class of a handle path `h` / `h.p.q` / `this.p`: `h`'s declared class
    /// (`var_class`), then each property's recorded class handle in turn. `None` when
    /// any step is not a known class handle.
    fn receiver_class(&self, recv: &[Ident]) -> Option<String> {
        let (first, rest) = recv.split_first()?;
        let mut class = self.var_class.get(&first.name).cloned()?;
        for seg in rest {
            let rec = self.class_records.get(&class)?;
            let (_, st) = rec.iter().find(|(n, _)| *n == seg.name)?;
            class = st.var_class.clone()?;
        }
        Some(class)
    }

    fn enter_with_block_scope(&mut self, call: &HierPath) -> Option<Box<ScopeSnapshot>> {
        let segs = &call.segments;
        if segs.len() < 2 {
            return None;
        }
        let mut recv = &segs[..segs.len() - 1];
        if matches!(recv[0].name.as_str(), "this" | "super") {
            // A method of the class: its properties are bound in the parse scope.
            recv = &recv[1..];
            if recv.is_empty() {
                return None;
            }
        }
        if let Some(c) = self.receiver_class(recv) {
            let rec = self.class_records.get(&c).cloned()?;
            let snap = self.snapshot_scope_boxed();
            self.replay_class_record(&rec, false);
            return Some(snap);
        }
        // The receiver's class is not known here. A block naming no name any visible
        // class record binds reads only the caller's scope, which is where it parses;
        // a block naming one could mean the object's property or the caller's
        // declaration, so it is refused (never unbound: an unbound caller constant
        // falls back to a silent value).
        if let Some(span) = self.with_block_names_a_class_property() {
            self.error_at(
                span,
                "a receiver whose class is known where the call stands, for an inline \
                 `randomize() with` block naming a class property (IEEE 1800-2017 §18.7)",
            );
        }
        None
    }

    /// The span of the first identifier in the `{ … }` block at the cursor that a
    /// visible class record binds.
    fn with_block_names_a_class_property(&self) -> Option<Span> {
        if self.class_records.is_empty() || self.peek() != Some(TokenKind::LBrace) {
            return None;
        }
        let mut depth = 0usize;
        for t in &self.toks[self.pos..] {
            match t.kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return None;
                    }
                }
                TokenKind::Word(WordKind::Ident) => {
                    let name = &self.src[t.span.clone()];
                    if self
                        .class_records
                        .values()
                        .any(|r| r.iter().any(|(n, _)| n == name))
                    {
                        return Some(Self::sp(&t.span));
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// A block opens a frame for the class-handle bindings its plain block-local
    /// declarations shadow (`block_body`). Out of line: `block_body` recurses per
    /// nesting level and its frame is the statement-depth budget.
    #[inline(never)]
    pub(crate) fn enter_block_handle_scope(&mut self) {
        self.block_handle_shadows.push(Vec::new());
    }

    /// At the block's end the shadowed class-handle bindings come back: a plain
    /// block-local `int it;` hides the module's `Item it` in the block only. (A
    /// plain declaration snapshots the block only when it shadows a packed
    /// multi-dimensional parameter, so the scope restore cannot be relied on here.)
    #[inline(never)]
    pub(crate) fn exit_block_handle_scope(&mut self) {
        if let Some(frame) = self.block_handle_shadows.pop() {
            for (n, c) in frame {
                self.var_class.insert(n, c);
            }
        }
    }

    /// Record the class-handle bindings a plain block-local declaration just dropped
    /// (`before` = `var_class` ahead of it) in the innermost block's frame.
    pub(crate) fn note_block_handle_shadows(
        &mut self,
        before: std::collections::HashMap<String, String>,
        d: &NetVarDecl,
    ) {
        let mut dropped = Vec::new();
        for n in &d.names {
            let name = &n.name.name;
            if let Some(c) = before.get(name) {
                if !self.var_class.contains_key(name) {
                    dropped.push((name.clone(), c.clone()));
                }
            }
        }
        if let Some(frame) = self.block_handle_shadows.last_mut() {
            frame.extend(dropped);
        }
    }
}
