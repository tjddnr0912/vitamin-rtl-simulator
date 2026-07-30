//! functions / tasks — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// True when the cursor is on `automatic` and the NEXT token begins a
    /// declaration — a net/var kind keyword (`int`, `logic`, …) or a
    /// (possibly user-typedef) identifier. Routes a block-local `automatic
    /// <type> <name>;` lifetime override (GAP-D, IEEE §6.21) through the
    /// decl-collection path instead of the statement region. Mirrors the
    /// keyword set of `net_var_kind`.
    pub(crate) fn lifetime_prefixes_decl(&self) -> bool {
        use Kw::*;
        match self.peek_at(1) {
            Some(TokenKind::Word(WordKind::Keyword(k))) => matches!(
                k,
                Wire | Tri
                    | Wand
                    | Triand
                    | Wor
                    | Trior
                    | Tri0
                    | Tri1
                    | Supply0
                    | Supply1
                    | Trireg
                    | Uwire
                    | Reg
                    | Logic
                    | Integer
                    | Real
                    | Realtime
                    | Time
                    | Event
                    | String
                    | Bit
                    | Byte
                    | Shortint
                    | Int
                    | Longint
            ),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent) => true,
            _ => false,
        }
    }

    /// Returns the parsed `FunctionDef` plus a `is_void` flag. A `function void`
    /// in module/package scope is task-equivalent (statement-called, output
    /// formals, control flow) — the module-item caller converts it to a `TaskDef`
    /// to reuse the full task machinery. Class methods ignore the flag (a void
    /// method is just a frame-function whose result is discarded at the call).
    pub(crate) fn parse_function_def(&mut self) -> (FunctionDef, bool) {
        let start = self.cur_span();
        self.bump(); // 'function'
        let automatic = self.eat_kw(Kw::Automatic);
        // N7/SV: a return-type KIND keyword (`logic`/`reg`/`bit`/`int`/`byte`/
        // `shortint`/`longint`) — `function int f` / `function logic [7:0] g`.
        // `integer`/`real`/`realtime`/`time` stay in `opt_param_type` below.
        // 2-state atoms imply a fixed signed range; `int` maps to the 32-bit
        // signed `Integer` return path (exact width/sign).
        let mut signed = false;
        let mut range = None;
        let mut ret_type = ParamType::Implicit;
        let mut ret_two_state = false;
        let mut ret_string = false;
        let is_void = self.eat_kw(Kw::Void);
        if is_void {
            // `function void f(...)`: no return value. In module/package scope the
            // caller converts to a TaskDef (task-equivalent); inside a class it is a
            // frame-function whose result is discarded. ret_type stays Implicit with
            // no range (the slot is never read). No AST shape change (IR-0).
        } else if self.at_kw(Kw::String) {
            // G4: `function [automatic] string f(...)` — string return type. `range`/
            // `ret_type` stay default; elaborate keys off `ret_string` to make the
            // return net a `NetKind::String` and treat a call as a string operand.
            self.bump();
            ret_string = true;
        } else {
            let kw_kind = match self.peek() {
                Some(TokenKind::Word(WordKind::Keyword(
                    k @ (Kw::Logic
                    | Kw::Reg
                    | Kw::Bit
                    | Kw::Int
                    | Kw::Byte
                    | Kw::Shortint
                    | Kw::Longint),
                ))) => Some(k),
                _ => None,
            };
            if let Some(k) = kw_kind {
                self.bump(); // the kind keyword
                             // int/byte/shortint/longint/bit are 2-state integral return types.
                ret_two_state =
                    matches!(k, Kw::Int | Kw::Byte | Kw::Shortint | Kw::Longint | Kw::Bit);
                match k {
                    // `int` is 32-bit SIGNED 2-state (defaults signed).
                    Kw::Int => {
                        ret_type = ParamType::Integer;
                        signed = true;
                    }
                    Kw::Byte => {
                        range = Some(Self::dec_range(7));
                        signed = true;
                    }
                    Kw::Shortint => {
                        range = Some(Self::dec_range(15));
                        signed = true;
                    }
                    Kw::Longint => {
                        range = Some(Self::dec_range(63));
                        signed = true;
                    }
                    _ => {} // logic/reg/bit: width from an explicit range below
                }
                // An explicit trailing `unsigned` must override the atom default.
                if let Some(s) = self.opt_signed() {
                    signed = s;
                }
                if range.is_none() {
                    range = self.opt_range();
                }
                // §4.5.156 (§3 全 site): `int` is the only non-vector kw-kind that can reach a
                // USER range here (byte/shortint/longint carry a forced width; logic/reg/bit are
                // vectors) — reject `function int [7:0] f`.
                if matches!(k, Kw::Int) && range.is_some() {
                    self.reject_packed_dims_on_nonvector(NetVarKind::Int, true);
                }
            } else if let Some(info) = self.peek_block_typedef_decl() {
                // A user-defined type name as the return type: `function b_t f;`
                // (the `<typedef_name> <function_name>` shape — same disambiguation
                // as a block-local decl). Map the typedef's resolved type onto the
                // return fields, mirroring the built-in-keyword arm above.
                self.eat_scope_qualifier(); // optional `pkg::` before the type name
                self.bump(); // the typedef name
                             // The function return-type fields carry one packed dimension
                             // (`range`); a multi-dim packed typedef (`typedef logic [3:0][7:0]
                             // m_t`) cannot be represented, so loud-reject rather than silently
                             // return only the first dimension's width (correct-or-loud).
                if !info.packed.is_empty() {
                    self.error("a multi-dimension packed type as a function return type");
                }
                signed = info.signed;
                range = info.range.clone();
                ret_two_state = matches!(
                    info.kind,
                    NetVarKind::Bit
                        | NetVarKind::Byte
                        | NetVarKind::Int
                        | NetVarKind::Shortint
                        | NetVarKind::Longint
                );
                // `int`/`integer` carry their 32-bit width via ParamType::Integer;
                // `real`/`realtime` MUST map onto ParamType::Real/Realtime — that enum is the
                // only place the return's realness is recorded, so leaving it `Implicit` made
                // `typedef real myreal; function myreal f(…)` return an INTEGER: the value was
                // rounded (and, through a frame call's return temp, came back 0.0), silently.
                // Every other kind sizes from `range` (a named atom with no explicit range
                // gets its fixed atom width here).
                if matches!(info.kind, NetVarKind::Int | NetVarKind::Integer) {
                    ret_type = ParamType::Integer;
                } else if info.kind == NetVarKind::Real {
                    ret_type = ParamType::Real;
                } else if info.kind == NetVarKind::Realtime {
                    ret_type = ParamType::Realtime;
                } else if range.is_none() {
                    if let Some(w) = Self::atom_member_width(info.kind) {
                        range = Some(Self::dec_range(w - 1));
                    }
                }
            } else {
                // return-type signedness/range/type, V2005 order: [signed] [range] [type]
                let sign_kw = self.opt_signed();
                range = self.opt_range();
                ret_type = self.opt_param_type();
                // `integer` defaults SIGNED; an explicit qualifier wins.
                signed = sign_kw.unwrap_or(matches!(ret_type, ParamType::Integer));
            }
            // a second `signed` after an integer-ish return is tolerated.
            signed = signed || self.opt_signed().unwrap_or(false);
        }
        let name = self.ident().unwrap_or_else(|| Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        // EXT2-C: scope struct-port `var_struct` bindings to this function (see
        // `parse_task_def`) — snapshot before the ports, restore after the body.
        let tf_scope = self.snapshot_scope();
        let mut ports = self.opt_tf_port_paren_list();
        self.expect(TokenKind::Semi, "';' after function header");
        let (body_decls, body_enums, body) = self.tf_body(BlockEnd2::Endfunction, &mut ports);
        self.restore_scope(tf_scope);
        self.expect(
            TokenKind::Word(WordKind::Keyword(Kw::Endfunction)),
            "'endfunction'",
        );
        self.opt_block_label(); // optional `: name` after endfunction → discard
        (
            FunctionDef {
                automatic,
                signed,
                range,
                ret_type,
                ret_two_state,
                ret_string,
                name,
                ports,
                body_decls,
                body_enums,
                body: Box::new(body),
                span: start.to(self.prev_span()),
            },
            is_void,
        )
    }

    /// `task [automatic] name [(tf_ports)] ; {body_decl} body_stmt endtask`
    pub(crate) fn parse_task_def(&mut self) -> TaskDef {
        let start = self.cur_span();
        self.bump(); // 'task'
        let automatic = self.eat_kw(Kw::Automatic);
        let name = self.ident().unwrap_or_else(|| Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        // EXT2-C: scope struct-port `var_struct` bindings to this task — snapshot
        // before the port list, restore after the body — so a struct port name never
        // leaks into the module or another tf scope (a stale binding would silently
        // mis-desugar `name.field` elsewhere).
        let tf_scope = self.snapshot_scope();
        let mut ports = self.opt_tf_port_paren_list();
        self.expect(TokenKind::Semi, "';' after task header");
        let (body_decls, body_enums, body) = self.tf_body(BlockEnd2::Endtask, &mut ports);
        self.restore_scope(tf_scope);
        self.expect(TokenKind::Word(WordKind::Keyword(Kw::Endtask)), "'endtask'");
        self.opt_block_label();
        TaskDef {
            automatic,
            name,
            ports,
            body_decls,
            body_enums,
            body: Box::new(body),
            span: start.to(self.prev_span()),
        }
    }

    /// Optional ANSI tf-port list `( tf_port {, tf_port} )`. Returns `[]` if there
    /// is no `(` (non-ANSI form — ports come from body input/output decls instead).
    /// Empty `()` ⇒ `[]`. Direction AND type are sticky across comma-grouped
    /// names (a bare `, name` inherits both — see `parse_tf_port`).
    pub(crate) fn opt_tf_port_paren_list(&mut self) -> Vec<TfPort> {
        let mut ports = Vec::new();
        if self.peek() != Some(TokenKind::LParen) {
            return ports;
        }
        self.bump(); // '('
        if self.peek() == Some(TokenKind::RParen) {
            self.bump();
            return ports;
        }
        let mut inherited = PortDir::Input;
        let mut inherited_type: TfPortType = (None, false, None, None, None);
        loop {
            let before = self.pos;
            let (port, dir, ty, unpacked_struct) = self.parse_tf_port(inherited, &inherited_type);
            inherited = dir;
            inherited_type = ty;
            // R5: an unpacked-struct port expands to its N member formals; every
            // other port pushes one (byte-identical to the pre-R5 path).
            match unpacked_struct {
                Some(tyname) => {
                    let members = self.unpacked_struct_member_ports(&port, &tyname);
                    ports.extend(members);
                }
                None => ports.push(port),
            }
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "')' closing tf-port list");
        ports
    }

    pub(crate) fn parse_tf_port(
        &mut self,
        inherited: PortDir,
        inherited_type: &TfPortType,
    ) -> (TfPort, PortDir, TfPortType, Option<String>) {
        let start = self.cur_span();
        // N2: SV `ref` (and `const ref`) formal direction. `ref` is NOT a keyword in
        // our lexer — it lexes as a plain identifier — so match it textually here, in
        // the direction slot only (a formal can never be *named* `ref`). Map to INOUT
        // (copy-in / copy-out): for straight-line synthesizable code with no aliasing
        // this is observationally identical to true pass-by-reference, and it is the
        // natural spelling for the in-place block helpers (`ref logic [31:0] H[0:7]`).
        // A `const ref` is input-only; copy-out of its unmodified value is harmless, so
        // it maps to inout as well. iverilog has no `ref` support (sorry) — hand-IEEE.
        // Only consume a leading `const` when it actually precedes `ref`.
        if self.at_ident_kw("const") && self.peek_at(1) == Some(TokenKind::Word(WordKind::Ident)) {
            // Peek the token after `const`: consume the pair only if it is `ref`.
            let save = self.pos;
            self.bump(); // tentatively consume `const`
            if !self.at_ident_kw("ref") {
                self.pos = save; // not `const ref` — restore
            }
        }
        let (dir, dir_present) = if self.at_ident_kw("ref") {
            self.bump();
            (PortDir::Inout, true)
        } else {
            match self.peek() {
                Some(TokenKind::Word(WordKind::Keyword(Kw::Input))) => {
                    self.bump();
                    (PortDir::Input, true)
                }
                Some(TokenKind::Word(WordKind::Keyword(Kw::Output))) => {
                    self.bump();
                    (PortDir::Output, true)
                }
                Some(TokenKind::Word(WordKind::Keyword(Kw::Inout))) => {
                    self.bump();
                    (PortDir::Inout, true)
                }
                _ => (inherited, false), // bare `, b` continues the previous direction
            }
        };
        let mut net_or_var = self.net_var_kind();
        if net_or_var.is_some() {
            self.bump();
        }
        let explicit_signed = self.opt_signed();
        let mut range = self.opt_range();
        // §4.5.156 (§3 全 site): reject an inline packed range on a non-vector tf-port
        // kind (`task t(byte [7:0] a)`); a typedef-name port is resolved below and was
        // validated at its own decl.
        if let Some(k) = net_or_var {
            self.reject_packed_dims_on_nonvector(k, range.is_some());
        }
        // A tf-port type given as a user-defined type name (`task t(byte_t a)`).
        // Resolve a SIMPLE typedef (vector / enum / atom) to its kind/sign/range,
        // exactly as a built-in keyword type would; a packed struct/union typedef
        // additionally returns its type name so the port var is layout-bound (EXT2-C;
        // class / multi-dim-packed = honest-loud, handled inside the helper).
        let mut typedef_signed: Option<bool> = None;
        let mut struct_name: Option<String> = None;
        let mut unpacked_struct: Option<String> = None;
        // r18 (E1): an ENUM-typedef formal (`input e_t m`) — bind the port NAME to its
        // enum type in `var_enum` (below, after the name is parsed) so `m.name()`/
        // `m.next()` desugar in the body exactly like a module-scope enum var. The type
        // name must be captured BEFORE `try_tf_port_typedef` consumes the token; the
        // resolver returns only the underlying vector kind, not the enum-ness.
        let mut enum_name: Option<String> = None;
        if net_or_var.is_none() && range.is_none() {
            let tname = self.type_name_key();
            let is_enum = self.enum_defs.contains_key(&tname);
            if let Some((k, s, r, sn, usn)) = self.try_tf_port_typedef() {
                net_or_var = Some(k);
                range = r;
                typedef_signed = Some(s);
                struct_name = sn;
                unpacked_struct = usn;
                if is_enum {
                    enum_name = Some(tname);
                }
            }
        }
        // A port carries its own type when a direction keyword OR any explicit type
        // token is present; otherwise (a bare `, name`) it inherits the previous
        // type (INCLUDING its struct-ness). The resolved type then propagates on.
        let type_present = net_or_var.is_some() || range.is_some() || explicit_signed.is_some();
        let (net_or_var, signed, range, struct_name, enum_name) = if dir_present || type_present {
            (
                net_or_var,
                explicit_signed
                    .or(typedef_signed)
                    .unwrap_or_else(|| atom_default_signed(net_or_var)),
                range,
                struct_name,
                enum_name,
            )
        } else {
            inherited_type.clone()
        };
        let name = self.ident().unwrap_or_else(|| Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        // IEEE §13.3: unpacked-array formal dims follow the NAME
        // (`input logic [63:0] words [0:7]`). Parsed here so the port list no
        // longer stops at the trailing `[` (was E2002 → a 6-error cascade); the
        // dims ride `TfPort.unpacked` for elaborate to lower / loud-classify.
        let mut unpacked = Vec::new();
        while self.at_dim_start() {
            match self.parse_dim() {
                Some(d) => unpacked.push(d),
                None => break,
            }
        }
        // EXT2-C: bind a struct/union port NAME to its layout so `name.field`
        // desugars in the body (scoped to this tf by `parse_function_def`/
        // `parse_task_def`). A bare continuation `, b` inherits the struct name too.
        if let Some(sn) = &struct_name {
            if !name.name.is_empty() {
                self.bind_tf_port_struct(&name.name, sn);
            }
        }
        // r18 (E1): bind an enum-typed port NAME to its enum type so `name.name()`/
        // `.next()`/`.first` desugar in the body (scoped to this tf by the snapshot/
        // restore around the port list + body). A bare continuation `, b` inherits the
        // enum name through `inherited_type`, exactly like the struct binding above.
        if let Some(en) = &enum_name {
            if !name.name.is_empty() {
                self.var_enum.insert(name.name.clone(), en.clone());
            }
        }
        // IEEE §13.5.3: an ANSI tf-port may carry a default argument value
        // (`int b = 10`), used when a call omits the trailing actual.
        let default = if self.eat(TokenKind::Eq) {
            Some(self.expr(0))
        } else {
            None
        };
        let port = TfPort {
            dir,
            net_or_var,
            signed,
            range,
            name,
            unpacked,
            default,
            span: start.to(self.prev_span()),
        };
        let next_type = (
            port.net_or_var,
            port.signed,
            port.range.clone(),
            struct_name,
            enum_name,
        );
        (port, dir, next_type, unpacked_struct)
    }

    /// Body of a function/task: a decl prefix (net/var decls AND — for the non-ANSI
    /// form — input/output/inout formal decls, hoisted into `ports`), then exactly
    /// ONE body statement (usually a `begin … end`), up to the endfunction/endtask
    /// closer. `ports` is appended to for non-ANSI formals.
    pub(crate) fn tf_body(
        &mut self,
        end: BlockEnd2,
        ports: &mut Vec<TfPort>,
    ) -> (Vec<NetVarDecl>, Vec<TypedefDecl>, Stmt) {
        let mut body_decls = Vec::new();
        // Body-local `typedef enum` nodes (round-5 Gap B) — the function/task has a
        // `body_enums` AST slot to carry them to elaborate for label-constant
        // registration. Alias/struct/union body typedefs need no carrier.
        let mut body_enums: Vec<TypedefDecl> = Vec::new();
        // Body-local typedef DEFINITIONs are lexically scoped (see `block_body`):
        // snapshot at the first one, restore when the body ends.
        let mut typedef_scope: Option<ScopeSnapshot> = None;
        while !self.at_eof() && !self.at_tf_end(end) {
            if matches!(
                self.peek(),
                Some(TokenKind::Word(WordKind::Keyword(
                    Kw::Input | Kw::Output | Kw::Inout
                )))
            ) {
                // non-ANSI formal: `input [7:0] a, b;` → one TfPort per name.
                let before = self.pos;
                self.parse_tf_port_decl_into(ports);
                if self.pos == before {
                    self.bump();
                }
                continue;
            }
            // A body-local typedef DEFINITION (`typedef logic [3:0] t;`).
            if self.at_kw(Kw::Typedef) {
                if typedef_scope.is_none() {
                    typedef_scope = Some(self.snapshot_scope());
                }
                let before = self.pos;
                // allow_enum: a function/task body can carry an enum's labels
                // (via `body_enums`) to elaborate, so accept & collect it here.
                if let Some(td) = self.parse_body_typedef_def(true) {
                    body_enums.push(td);
                }
                if self.pos == before {
                    self.bump();
                }
                continue;
            }
            // B4: a per-decl lifetime override `automatic <kind> <name>;` (only
            // `automatic` — `static` is not a reserved word). The keyword precedes
            // a normal var decl OR a user-typedef decl; consume it and stamp the
            // lifetime on the decl. `lifetime_prefixes_decl` gates the same kind
            // set as `block_body`'s GAP-D branch, so `automatic int unsigned x;`
            // and `automatic my_t s;` both work in a function/task body.
            if self.at_kw(Kw::Automatic) && self.lifetime_prefixes_decl() {
                self.bump(); // 'automatic'
                let before = self.pos;
                if self.net_var_kind().is_some() {
                    if let Some(mut d) = self.parse_net_var(false) {
                        // function/task body decl: no net delay
                        d.lifetime = Some(true);
                        body_decls.push(d);
                    }
                } else if let Some(info) = self.peek_block_typedef_decl() {
                    if typedef_scope.is_none() {
                        typedef_scope = Some(self.snapshot_scope());
                    }
                    if let Some(mut d) = self.parse_typed_decl(info) {
                        d.lifetime = Some(true);
                        body_decls.push(d);
                    }
                }
                if self.pos == before {
                    self.bump();
                }
                continue;
            }
            if self.net_var_kind().is_some() {
                let before = self.pos;
                if let Some(d) = self.parse_net_var(false) {
                    // function/task body decl: no net delay
                    body_decls.push(d);
                }
                if self.pos == before {
                    self.bump();
                }
                continue;
            }
            // A function/task body decl using a user-defined type name
            // (`my_enum_t s; byte_t b;`), same `<typedef_name> <ident>` shape as a
            // block-local decl (see `block_body`). This writes the VAR-name-keyed
            // maps, so — exactly as in `block_body` — it triggers the scope
            // snapshot: a function-local struct var (`inner_s x;`) whose name
            // shadows an outer one must not leak its layout binding past the body
            // (a function need NOT do a struct field assign to clobber it, so the
            // frame-call-subset E3009 does not cover this).
            if let Some(info) = self.peek_block_typedef_decl() {
                if typedef_scope.is_none() {
                    typedef_scope = Some(self.snapshot_scope());
                }
                let before = self.pos;
                if let Some(d) = self.parse_typed_decl(info) {
                    body_decls.push(d);
                }
                if self.pos == before {
                    self.bump();
                }
                continue;
            }
            // §4.5.192: a packable UNPACKED-struct scalar body-local (`rec_t p;`) — the
            // type is in `unpacked_struct_layouts`, not `typedefs`, so it misses the
            // typedef branch above. Lowered to a packed-vector local + scalar-struct
            // registration so `p.field` desugars like a packed-struct member.
            {
                let before = self.pos;
                if let Some(d) = self.parse_body_unpacked_struct_local() {
                    if typedef_scope.is_none() {
                        typedef_scope = Some(self.snapshot_scope());
                    }
                    body_decls.push(d);
                    continue;
                }
                if self.pos != before {
                    // consumed tokens but produced no decl (a loud error path) — do not
                    // re-parse them as a statement; advance the loop.
                    continue;
                }
                // Family B (r17): a NON-packable unpacked-struct scalar body-local at
                // the tf-body TOP LEVEL (e.g. `np_t r;` where `np_t` has a `string`
                // member). `parse_body_unpacked_struct_local`'s PACKABLE gate returned
                // `None` without consuming, and the plain-typedef branch above missed
                // it (unpacked structs live in `unpacked_struct_layouts`, not
                // `typedefs`). Route it through the SAME per-member path the begin/end
                // block-local already uses (`try_block_unpacked_struct_decl` →
                // `parse_unpacked_struct_decl`): each member gets its own frame slot
                // (a `string` member → `NetKind::String` heap slot, integral members →
                // scalar slots), so `r.count`/`r.name` desugar to `$unp$r$field`
                // exactly as at module scope. Array/dyn/loud forms inherit
                // `parse_unpacked_struct_decl`'s existing correct-or-loud behavior,
                // identical to the block-local path. Snapshot BEFORE the parse mutates
                // `var_unpacked_struct` so `restore_scope` at body end is leak-free.
                if let Some(tyname) = self.peek_unpacked_struct_decl() {
                    if typedef_scope.is_none() {
                        typedef_scope = Some(self.snapshot_scope());
                    }
                    if let Some(member_decls) = self.parse_unpacked_struct_decl(tyname) {
                        body_decls.extend(member_decls);
                    }
                    continue;
                }
            }
            break; // first non-decl token starts the body statement
        }
        let body = if self.at_tf_end(end) {
            Stmt::Null(self.cur_span()) // empty body: `function f; endfunction`
        } else {
            // SV: a function/task body may hold MULTIPLE statements with no
            // explicit `begin`/`end` (`function f; a=1; b=2; endfunction`). Collect
            // them all (until the closer) and wrap in an implicit sequential block.
            // A SINGLE statement is returned bare — byte-identical to the V2005
            // one-statement form, so every existing design is unaffected.
            let start = self.cur_span();
            let mut stmts = Vec::new();
            while !self.at_eof() && !self.at_tf_end(end) {
                let before = self.pos;
                stmts.push(self.parse_statement());
                if self.pos == before {
                    self.bump(); // guarantee forward progress
                }
            }
            if stmts.len() == 1 {
                stmts.pop().unwrap()
            } else {
                Stmt::Block {
                    label: None,
                    decls: Vec::new(),
                    stmts,
                    span: start.to(self.prev_span()),
                }
            }
        };
        // Drop body-local typedefs (restore outer scope) after the body is parsed.
        if let Some(scope) = typedef_scope {
            self.restore_scope(scope);
        }
        (body_decls, body_enums, body)
    }

    /// True at the `endfunction`/`endtask` closer.
    pub(crate) fn at_tf_end(&self, end: BlockEnd2) -> bool {
        match end {
            BlockEnd2::Endfunction => self.at_kw(Kw::Endfunction),
            BlockEnd2::Endtask => self.at_kw(Kw::Endtask),
        }
    }

    /// Non-ANSI formal decl `input [r] a, b;` → one TfPort per name, appended.
    pub(crate) fn parse_tf_port_decl_into(&mut self, ports: &mut Vec<TfPort>) {
        let dir = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::Output))) => {
                self.bump();
                PortDir::Output
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Inout))) => {
                self.bump();
                PortDir::Inout
            }
            _ => {
                self.bump();
                PortDir::Input
            }
        };
        let mut net_or_var = self.net_var_kind();
        if net_or_var.is_some() {
            self.bump();
        }
        let mut signed = self.signed_eff(net_or_var);
        let mut range = self.opt_range();
        // §4.5.156 (§3 全 site): reject an inline packed range on a non-vector non-ANSI
        // tf-port kind; a typedef-name port is resolved below (validated at its decl).
        if let Some(k) = net_or_var {
            self.reject_packed_dims_on_nonvector(k, range.is_some());
        }
        // A non-ANSI tf-port type given as a user-defined type name
        // (`input byte_t a;` / `input cfg_t c;`) — resolve a SIMPLE typedef, or a
        // packed struct/union (EXT2-C), exactly as the ANSI path.
        let mut struct_name: Option<String> = None;
        let mut unpacked_struct: Option<String> = None;
        if net_or_var.is_none() && range.is_none() {
            if let Some((k, s, r, sn, usn)) = self.try_tf_port_typedef() {
                net_or_var = Some(k);
                signed = s;
                range = r;
                struct_name = sn;
                unpacked_struct = usn;
            }
        }
        loop {
            let n_start = self.cur_span();
            let Some(name) = self.ident() else { break };
            // IEEE §13.3: a non-ANSI formal may be an unpacked array too
            // (`input logic [63:0] words [0:7];`). Per-name dims (a comma list can
            // mix `a, mem [0:3]`), mirroring the ANSI path.
            let mut unpacked = Vec::new();
            while self.at_dim_start() {
                match self.parse_dim() {
                    Some(d) => unpacked.push(d),
                    None => break,
                }
            }
            // Bind a struct/union port name to its layout (scoped by the enclosing
            // tf's snapshot/restore); `input cfg_t a, b;` binds every name.
            if let Some(sn) = &struct_name {
                self.bind_tf_port_struct(&name.name, sn);
            }
            let port = TfPort {
                dir,
                net_or_var,
                signed,
                range: range.clone(),
                name,
                unpacked,
                default: None, // non-ANSI formals have no default (ANSI-only, §13.5.3)
                span: n_start.to(self.prev_span()),
            };
            // R5: an unpacked-struct formal expands to its N member ports (per name in
            // a comma list); every other formal appends one (byte-identical pre-R5).
            match &unpacked_struct {
                Some(tyname) => {
                    let members = self.unpacked_struct_member_ports(&port, tyname);
                    ports.extend(members);
                }
                None => ports.push(port),
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';' after tf-port declaration");
    }

    /// GAP-D helper: parse a block-local declaration carrying an explicit
    /// `automatic` lifetime override (`automatic <type> <name>;`, IEEE §6.21),
    /// stamping `lifetime = Some(true)`. Split out of `block_body` and marked
    /// `#[inline(never)]` so its locals never enlarge that hot recursive frame
    /// (the MAX_STMT_DEPTH budget is frame-sized — see the call site). Snapshots
    /// `scope` before a user-type decl, exactly like the inline typedef-var
    /// branch, so a block-local shadow does not leak its binding out of the block.
    #[inline(never)]
    pub(crate) fn parse_automatic_block_decl(
        &mut self,
        scope: &mut Option<Box<ScopeSnapshot>>,
    ) -> Option<NetVarDecl> {
        self.bump(); // 'automatic' — `static` is not reserved, so only this reaches here
        if self.net_var_kind().is_some() {
            let mut d = self.parse_net_var(false)?;
            d.lifetime = Some(true);
            Some(d)
        } else if let Some(info) = self.peek_block_typedef_decl() {
            if scope.is_none() {
                *scope = Some(Box::new(self.snapshot_scope()));
            }
            let mut d = self.parse_typed_decl(info)?;
            d.lifetime = Some(true);
            Some(d)
        } else {
            None
        }
    }
}
