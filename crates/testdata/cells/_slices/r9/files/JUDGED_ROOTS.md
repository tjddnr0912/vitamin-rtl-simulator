# Row 9 — judged roots (opus-judge 2026-10-07, frozen vita f4d778f2, HEAD 1d23be89)

Family A = OpenTitan a3490b42 (S/row9/A-opentitan/REPORT.md, repros/, patched/, patchlog.txt)
Family B = VeeR EL2 0169d669 / EH1 d04b1c7a (S/row9/B-veer/REPORT.md, repros/<id>/t.sv, el2w.patch, eh1w.patch, bin/)
Family C = alexforencich axis 48ff7a7e / pcie 25156a9a / uart 1b867e53 / i2c a65be404 (S/row9/C-alex/repros, hand/, run/, scripts/)
Judge raw runs: S/row9/judge/runs/<id>/{vita,iv,s2v,vl}.out, v1/, probes/, grid4/

S = structural, L = local, O = other. parser = hdl-parser/src, elab = elaborate/src.

| id | fam | judged | S/L | code site | mechanism | closes with | held row |
|---|---|---|---|---|---|---|---|
| V1 | B | V | S (+L loud) | const_bound.rs:44 tier-3 gate ast_selfwidths_all_known → const_select.rs:721 → select_base_at_declared :612 (dwidth>64, signed None); sinks packed.rs:1882, const_bound.rs:185; engine eval/binops.rs:702, sim-ir selfwidth.rs:248 unwrap_or(1) | arithmetic over a select of a >64-bit parameter in a bound/width/count declines; consumer gets width 1 / count 0, no diagnostic (SILENT; EL2 fetch addr 00000000 vs 80000000) | V+B (REMAINING_WORK:22 "bare >64-bit select" blocked by declaring-scope fold; §4.5.560 reverted); silent half local: sinks refuse | sibling 1–4 |
| R11 | A | B | S | typedef range folded at caller prefix `[in t.$func$mp::inv]`; ROADMAP "A typedef's named dims re-resolve at each use; BLOCKED (declaring-scope fold)"; §3.b pkg-text-open | `typedef logic [W-1:0] m_t` as a package function formal type resolved where pkg W unbound; prim_mubi_pkg → all 19 IPs | B stage (window needs AE, AC) ; untried local alt: fold package typedef dims at declaration | sibling 1–4 |
| R26 | A | B | S | package.rs:826-869 → const_fn.rs:1383 const_fn_def | package parameter's call cannot see own-package or imported routines (module import folds) | own-routine half local (pkg-param-own-fn OPEN); imported half = row 2 | DIRECT row 2 |
| R25 | A | V | S | package.rs:928-938 (label fold Option<i64>); §2 row 15 BLOCKED | enum label `'z` not representable in i64 label store | V stage | sibling (AE) |
| R28 | A | V | S | const_fn.rs:1255 (w>64 → None); "Above 64 bits the decline is deliberate; HELD" | 80-bit enum labels | V stage | none |
| R30 | A | V | S | cond_names.rs:120-129 param_site (untyped → Other); string-literal-condition-residue BLOCKED | untyped string header param invisible to generate-condition reader | V+B | sibling 1–4 |
| R04 | A | T | S | parser typedefs.rs:551-566; structs/mod.rs:45-83 | `s_t [2:0]` ok as variable, refused as alias/member/localparam/parameter/return — each lane parses type itself | T stage | sibling row 8 |
| R05 | A | T | S | parser struct_sel.rs:726/876/913 const_lit | member part-select remapped at parse time needs literal bounds; `tl_i.a_address[RegAw-1:2]` all 19 IPs | T stage (member layout carried to elaborate) | none |
| R01b | A | T | S | parser typedefs.rs:780-797; names_an_overridable (no call leaf) | member width over `vbits()` not evaluable at parse time | T stage | none |
| R21(+R33) | A | T | S | parser struct_sel.rs:94-98 | symbolic layout: whole-member read ok; member sub-select and $bits(type) refused | T stage | none |
| R14 | A | T | S | parser type_params.rs:901-908; ROADMAP ⑤ HELD | type parameter carried only as T$w/T$s; enum type no carrier | T stage | sibling row 6 |
| V3 | B | T(+V) | S | parser typedefs.rs:780-797 | member width `[pt.W-1:0]` over struct-typed package param (2294 bits) | T stage (+ wide value) | none |
| R01a | A | V | L | parser expr_primary.rs:33 try_const_index, :656 cw_eval | parser fold lacks >>, <<, ** | add arms | none |
| R29 | A | V | L | const_wide.rs:599-602 under fold_concat_parts:1169 | zero-count replication in a larger concat declines whole concat | local | none |
| R31 | A | V | L | expr_special.rs:705 bits_of_selfdet no select arm | $bits({e.f,e.bus}), $bits(addr[5:4]) | local (= V4 edit) | none |
| V4 | B | V | L | const_fn.rs:423 → expr_special.rs:705 | $bits(sig[5:4]) in an override | local (= R31) | none |
| R10 | A | B | L | net_util.rs:762 prescan_net_bits (body decls only) | $bits(<ANSI port>) | local | none |
| B1 | B | B | L | ports.rs:258-268 | modport expression as interface actual | local | none |
| B2 | B | B | L | ports.rs:276-285 (iface_insts instances only) | forwarded interface port → implicit 1-bit net | local | none |
| R27 | A | T | L | ports.rs:745-750 child_len>1 vs netdecl.rs:792 | one-element array port wired as scalar | local | none |
| T1 | B | T | L | struct_sel.rs:726/876/913 | const_lit cannot read literal-valued localparam | use const_bound | none |
| T2 | B | T | L | expr_ctx.rs:148 → lvalue.rs:572 | member sub-select as output-port actual (CA LHS works) | local | none |
| A other | A | other | L | R02 anon nested struct packed (all reg_pkgs); R06 '{N{0}} in pattern; R19 package export; R23 a[i][k].member; R24 T'{…}; R08 '{default:v} to fn-local unpacked array (frame); R09 {a,b}= in framed fn; R17 sub-array/pattern as unpacked port actual (ports.rs:441) | parser / lowering | local | none |
| B other | B | other | L | O1 module-item `assert #0`; O2 covergroup local `cg_t cg=new();`; O3 `#()` empty param list; O4 `if` in constraint; O5 DPI import (was non-goal; owner 2026-10-07: now a goal); O6 assoc index `[bit[31:0]]`/`[int unsigned]`; O7/O8 packed_lval.rs:345-370 (2-D var index + part-select; non-zero-LSB elem + part-select); O9 CA-called fn writes select of return var; O10 part-select driver beside whole driver on a wire (E3001 vs oracles); O11 concat target in frame fn; O12 $sformatf nested; O13 $readmemh into assoc array; O14 hierarchical unpacked-array write; O15 std::randomize() with {dist}; O16 ast_query.rs:562 generate-loop always_combs each writing own bit of shared vector → F4016 delta limit (iverilog/verilator any=11); O17 '{…} into unpacked slice; O18 cont_array.rs:369 cascade of O8 (false E3018) | parser / lowering / engine | local/engine | none |
| C-R01 | C | split | — | §2-N-1 SPLIT | t0: CA x→value wakes always @* (vita = verilator 00; iverilog x). IEEE §4.7 + §9.2.2.2.2: both legal | not a defect | none |
| C-R02 | C | other | L | packed.rs:1903-1906 | reversed part-select in a constant-false procedural `if` refused (generate twin runs); blocks axis_switch/axis_arb_mux/axis_ram_switch S_COUNT=1, pcie_us_axi_dma(_wr)/master(_rd) 64/128 | local | none |
| X01 | A | V | L | instance.rs:131 (fill sized at 32) | `'1` enum label not sized to its 3-bit base (overlay-only find) | local | none |

Counts: structural V4 B2 T6; local V/B/T 10; other 28; held-row direct hits 1 (R26→row 2).

Pages (28 blocked): 8 open with local+other only (EH1 + 7 alexforencich modules); 20 need structural (19 OpenTitan + EL2).
OpenTitan: every page has S{R04 R05 R11} + O{R08 R09} + L{R01a R10}; extras per IP: uart/gpio/i2c +S{R01b} +O{R02 R06}; pattgen, sysrst_ctrl +O{R02}; rv_timer +S{R01b} +L{R27} +O{R02 R06 R17}; aon_timer +S{R01b R26 R28} +L{R29} +O{R02 R06}; edn +S{R14 R25} +O{R02 R17}; entropy_src +S{R14 R26 R28 R30} +L{R27 R29} +O{R02}; csrng +S{R14 R25 R26 R28} +L{R29} +O{R02}; hmac, usbdev +S{R21} +O{R02} (hmac +O{R24}); aes +S{R14 R25 R26 R28 R30} +L{R29} +O{R02 R24}; kmac +S{R14 R21 R25 R26 R28} +L{R29} +O{R02 R24}; otbn +S{R01b R14 R21 R25 R26 R28} +L{R29 R31} +O{R02 R24}; spi_host, spi_device +S{R01b R21} +O{R02 R06 R19}; adc_ctrl +O{R02 R23}; rv_plic none extra.
EL2: S{V1 V3} L{T1 B1 B2} O{O2–O16}. EH1: L{T1 T2 V4} O{O1 O6 O8 O9 O11 O12 O13 O16 O17 O18}.
V→B alone opens 0/20 (T roots everywhere). With T as per-consumer slices, B then opens 12 OpenTitan pages.

V1 other silent shapes (probe p4, sv2v→iverilog + verilator agree): `f[pt.HI-1:0]`, `f[0 +: pt.HI+1]`, `{(pt.HI+1){1'b1}}` → 0, `f[WP[7:0]+1:0]` over a plain 72-bit parameter, `f[pt.HI*2:pt.LO]`, same inside a function body, net declaration assignment. Correct: bare-member bounds/counts, declaration ranges, for/repeat bounds.

Admission candidates (vita = oracle byte-identical):
- verilog-axis axis_switch 4×4 + 4× axis_fifo, hand tb NF=3000: CYCD=45268d3005e4fe9d, trace md5 4614ce43345864ce; iverilog 7.79 s, vita 5.00 s (S/row9/C-alex/hand/tb_axis.v)
- verilog-i2c master+slave NCYC=900000: md5 726994eae6ef80b9; iverilog 8.83 s, vita 6.37 s (hand/tb_i2c.v)
- verilog-uart loopback N=20000: ACC=5c207af72615bf37 CYCD=70fa3a6cafdf8426; iverilog 6.63 s, vita 4.81 s (hand/tb_uart.v)
- verilog-pcie dma_if_pcie_us 512 generic tb: needs in-tb DIGEST + protocol stimulus
- OpenTitan prims tb (11 upstream files): digest 2958736d2b4a80a9 = verilator ×5 = sv2v→iverilog; vita 54.1 s vs verilator 0.38 s
- EH1 after local roots: verilator x-invariant 66/66, vita 7.4 s, hello_world 443/1362 (needs t0 reset-edge tb fix W22/E22)
- EL2: not admissible (verilator not x-invariant)
Mutation-moves-digest check (corpus contract rule 5) not yet run for any candidate.
