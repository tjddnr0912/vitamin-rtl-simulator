(§5.2 Q-DECLTYPE · rank — F · source W/review/structure-2026-11-04/REPORT.txt S7 item 2) (one declaration-type record, from the AST to elaborate's nets):
* slot: the first open row of §5.2, ahead of Q-MEMBSRC (C2) and 29 (C3) (owner, 2026-11-05)
* slot: it absorbs Q-PARAM2STATE, which is its first slice (Q-PARAM2STATE keeps its doc and witnesses)
* slot: C2 Q-MEMBSRC and C3 29 wait on it through Q-PARAM2STATE
* slot: the type and package rows after it (46, 24, 68 with 23, 43, 39, 42) build on it
* slot: an AST / SchemaHash change, so it runs alone (D12)
* symptom: a declaration's type facts are spread over 9 AST structs (`hdl-ast/src/lib.rs:308`, `:643`, `:660`, `:683`, `:708`, `:836`, `:1361`, `:2103`, `:2159`), parser tables and 7+ per-NetId elaborate maps written in 7 files from 56 `add_net` sites
* evidence: 3c5b846f ("census found six" net-creating sites), d544b301 (`ParamDecl.signed` folds an explicit `unsigned` away)
* evidence: the residual inline twin `elaborate/src/netdecl.rs:745-766` of `elaborate/src/array_geom.rs:419-435`
* fix: `hdl_ast::DataType { kind, signing: Implicit|Signed|Unsigned, packed: Vec<Range>, unpacked: Vec<Dim> }` embedded in each declaration
* fix: in elaborate one `NetShape { declared (msb,lsb) i64, packed dims, unpacked dims, kind, two_state }` in a single `BTreeMap<u32, NetShape>` written only by `add_net(name, NetVar, NetShape)`, so a site cannot forget a fact
* note: size (grep counts, not effort): 9 AST structs (a `.vu` schema bump), 65 `signed` constructions in hdl-parser, 56 `add_net` sites in 26 files, ~143 reads of the 7 maps (`packed_dims` 68, `intro_kind` 29, `array_dims` 18, `neg_lsb` 9, `asc_lsb` 7, `unpacked` 7, `decl_range` 5)
* note: a refactor row is verified by unchanged output — the full gate, corpus and cells byte-identical to PRE — not by page cells
