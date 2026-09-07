# ROADMAP — open work (vitamin)

Forward-only. Completed slices live in [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md) (`#### 4.5.<N>`, index at the top); Phase A–D execution records in [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md); the pre-2026-07-16 text in [ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md). Section numbers (§0–§9) are stable: tests and CLAUDE.md cite them. Row numbers in §2 and circled numbers in §3 are never reused.

Baseline counts (tests, format_version, MsgCode) are kept in CLAUDE.md only.

Operating rule: when a slice lands, append its record to the ARCHIVE and delete the item here; a residue survives as its own row. New findings are added as one row in the matching section. Every row states: symptom with repro and oracle values, root cause and code site, fix shape, prerequisite, oracle status.

## Summary

| order | § | track | open items | oracle | notes |
|---:|---|---|---:|---|---|
| 1 | §2 | silent-wrong (correctness) | see §2 tables | 2-oracle unless marked | fills LOOPROMPT slots 2·3; walls and oracle splits are listed but not started |
| 2 | §3 | loud → correct-support | see §3 tables | mostly ✓ | fills LOOPROMPT slot 1; §0 T2 residues are the cheaper end of the same ladder |
| 3 | §6 | G2 observability (OBS) | 6 stages | internal 3-way | orthogonal to correctness; parallelizable |
| 4 | §5 | performance | residues only | measured | below the correctness ladder; codegen / 2-state storage / cycle mode rejected |
| — | §4 | SVA honest-loud | 6 | mostly none | hand-IEEE when started |
| — | §7 | conditional / long-term | 4 | — | trigger-gated |
| — | §8 | non-goals | 1 | — | permanent |

Priority principle (time-invariant): ① CRITICAL silent-wrong with an oracle > ② loud→supported with an oracle > ③ honest-loud promotion whose prerequisite is met > ④ G2 OBS. Performance does not enter this ladder. "No oracle" is not a reason to defer: implement from the LRM and pin by hand.

## 0. correct-support 승격 큐

비목표(의도적 loud): fixed 배열 `new[]` · multi-dim partial 인덱스 `s[0]`(둘 다 iverilog도 거부) · cross-type SoA whole-element 복사 · generate case 의 real scrutinee(§4.5.243 핀).

### iverilog defects (vita is IEEE-correct) — oracle disqualifiers, regression-pinned

| # | repro | iverilog | vita |
|---|---|---|---|
| ① | `string s[5]; s[0]="abcdefg"` 원소 `.len()` | 5(배열 크기) | 7 |
| ② | 동시 fork 활성화가 automatic string 배열 공유 | `A!` | `A!!` |
| ③ | 같은 fd 에 `$fmonitor` 두 번 | 누적(자기 싱글턴 `$monitor` 와 모순) | destination 별 replace |
| ④ | 빈 string 배열 원소 `%s` | 공백 1칸 | 빈 문자열 |
| ⑤ | `$clog2(4'sd7+4'sd1)` (§20.8.1 = 인자 자기 폭의 비트 패턴) | 32 | 3(verilator 도 3 · §4.5.343) |
| ⑥ | `$itor(64'h1_0000_0008)` (unsigned·signed `longint` 둘 다 8 ⇒ 부호 축 아님) | 8 | 4294967304(verilator 동일) |
| ⑦ | `s="ab"` 일 때 `s<"ab"`·`s<"aa"`·`s<"zz"` | 전부 1(동시 참 불가) | `0 0 1`(verilator 동일) |

### T2 residues (each its own slice)

| id | symptom · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle |
|---|---|---|---|---|
| 8ⓐ | 암시 변환 `logic [R-1:0]`·`{R{1'b1}}`, loud | — | 비목표 | 분열(폭은 verilator 거부/iverilog 3, count 는 iverilog 거부/verilator 3) |
| 8ⓑ | 무타입 localparam 의 real 값, loud | §6.20.2 상 real 파라미터 | 반올림 = §4.5.232 가 철회한 silent-wrong | 2-oracle |
| 8ⓒ | 실수 override `#(.R(2.5))`, loud | override 채널이 i64 | 채널을 real 로 | 2-oracle |
| 8ⓓ | `1.0/0.0`, loud | real 도메인이 비유한을 의도적 거절 | 의도적 | 2-oracle |
| 8ⓔ | `R<<1`(실수 미정의 연산자), loud | — | 비목표 | 분열(iverilog 거부/verilator 6) |
| 8ⓕ | `localparam time T = R*2.0`, loud | `param_decl_width_opt` 가 `time` 을 무타입 가지로 흘린다 | 8자리 공유 ⇒ census 후 분기 | 2-oracle |
| 8ⓖ | 상수함수 본문의 `$rtoi`, loud | 모듈 스코프 resolver ⇒ shadow 위험 | env 아는 walk 로 이동 | 2-oracle |
| 8ⓗ | `int'(real'(R))` 중첩, loud | — | 명시 변환 경계 확장 | 2-oracle |
| 10ⓐ | `parameter` 라벨은 접으면 안 된다 — override 가 라벨 값을 바꾼다(`m #(.K(9))` 에서 iverilog 10/`first=9`), 파서는 override 전에 돈다 | 파서 `const_locals` 라벨 폴드 | enum-method desugar 를 elaborate 로(아키텍처) | 2-oracle |
| 10ⓑ | `localparam L = 8'h5` 라벨이 안 접혀 enum 이 `enum_defs` 에 안 들어가고 모든 메서드가 "hierarchical function call" loud | `const_locals` 가 decimal 만 기록 | 그 표는 generate 인덱스와 공유 ⇒ 생산자를 넓히면 다른 소비자가 움직인다(별도 항목). 절단 필요 리터럴(unsized `'h1FFFFFFFF`·mis-sized `4'hFF`) 거부는 유지(두 오라클도 거부) | 2-oracle |
| 11 | 음수 range bound 잔여 = PART select `x[1:-2]`(정직한 loud) · 포트/formal(warn+clamp) | 바운드 접기가 unsigned | 포트 비대칭은 의도적 opt-in | 2-oracle |
| 14-a | `-pvalue+<name>=<val>` 미구현(`grep -rn pvalue crates/` = 0건) | — | `-G` 별칭이므로 argv 파싱만 | n/a |
| 14-b | `-P<path>=<val>`(계층 경로) 미구현 | defparam 이 direct-child 한정 | 같은 제약을 물려받는다 | n/a |
| 14-c | 서로 다른 `-G` 로 만든 두 `.velab` 이 헤더 128바이트 동일, 게이트가 구분 못 함(값은 맞다·위험은 provenance) | `-G` 가 RULE-V upstream 다이제스트에 안 들어간다; 섞었더니 안 바뀐 `.vu` 에 `vrun --upstream` 이 `E9003 digest changed` 거짓 stale | 헤더 자기 필드(doc-14 §RULE B) = `format_version` bump | n/a |
| 13 | `case (x) inside {…}` loud | — | hand-IEEE + 내부 차분 | no-oracle(iverilog 13.0 이 `case inside`/`inside` op/array reduction 거부) |

## 0-B. 소형 follow-on (loud 유지)

- `void'(getnext())` void-cast of output-formal fn · frame-formal array 를 nested hier 로 forward(OUTPUT/INOUT) · param/call leaf size-cast `8'(P*a)`(§4.5.212 잔여).
- fork-in-frame 잔여(§4.5.214, Minor/safe): `fork_arms_self_contained` resolve-time 재-walk 중복 제거 · 공유 `enter_task_frame` arm comment · forking task 를 호출하는 fork arm 의 elaborate-time reject(현재 F4004 tie-cap runtime guard 로 안전, clean E3009 가 명확) · same-instant zero-delay sibling visibility differential-미검증.

## 0-C. 대형 항목 착수 판단표 (크기 재추정 금지)

| 항목 | 비용 | payoff | 선행조건 / 함정 |
|---|---|---|---|
| A. 파일위치 함수군(`$ftell`/`$fseek`/`$rewind`/`$ferror`) | format_version bump 확정 | 中 | 신규 `SysFuncId` = frozen-root 변경 → SimIr 스키마해시·canonical·RON 골든 재핀 + 전 `.velab` 무효화. 사이드카 우회 불가(겹치는 기존 id 없음, 실측). `$feof`/`$fgetc`/`$ungetc` 만 쓰면 bump 없이 가능 |
| B. literal 파싱 공유 크레이트(§4.5.234 안 ①) | 中~大(559줄 이동+어댑터+전 리터럴 재검증) | 小 — 거부 형태는 절단 리터럴(`4'hFF` in `[3:0]`)·unsized+`s` 인데 iverilog 도 절단 거부; 얻는 것은 두-술어 위험 제거 | `literal.rs` 가 `sim_ir::{BitPacked,ConstRepr,ConstVal}` 의존 → 옮기면 hdl-parser 가 sim-ir 을 본다(레이어링 역전). 분해 = digit→bits(중립)/ConstVal 패킹(IR) |

순서 A > B: A 는 format bump 가치가 있을 때, B 는 두-술어 제거가 우선순위를 얻을 때만.

## 1. 착수 우선순위 — 원칙만 (현재 큐 = §5.2)

1. 오라클 있는 CRITICAL silent-wrong (§2) — 최우선. 정확성이 이 저장소의 최상위 원칙이다.
2. 오라클 있는 loud→supported (§3 · additive=저위험). "오라클이 없다" 는 미루는 이유가 아니다 — 없으면 hand-IEEE 로 짓는다.
3. 전제조건 충족된 honest-loud 승격 (§0 · §4~§5).
4. G2 OBS 슬라이스 (§6).

성능은 이 사다리 위에 올라오지 않는다. 새 큐를 여기 만들지 마라.

T4(기회 슬라이스): 함수 지역 배열 원소 쓰기 514 ns vs iverilog 24 ns(ARCHIVE_PHASE_A-D §5.0-b).

## 2-N. 신규 silent-wrong (verilog-axi census 부산물)

| id | symptom · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle |
|---|---|---|---|---|
| 2-N-1 | verilog-axi 미승격: `m_axi_awvalid`/`m_axi_wvalid`/`m_axi_arvalid` 가 리셋 직후 iverilog x, vita 0 — 123,166 사이클 중 29(`XC=29` vs `XC=0`, N=200 불변·다이제스트 불변). 기능 일치(같은 사이클 완료)·vita 가 낙관적이라 x 전파 버그를 가린다 | 크로스바가 register slice 에 computed 와이어로 도달(`int_s_axi_wready[m] = int_axi_wready[w_select_reg*S_COUNT+m] \|\| w_drop_reg`); vita 만 t0 이벤트. vita 규칙 = 계산하는 구동자는 초기 상태를 갖고 비트를 옮기는 구동자는 안 갖는다(`sim_engine::alias::copy_nets`) | 두 번째 오라클 없이 쫓지 않는다; 승격 = 다이제스트가 x-사이클을 안 세거나 oracle-split 판정 | oracle-split: `assign w = a \| b` → ivl c=1 이지만 `a & b` → c=0(피연산자·값 동일); `pr & 1'b1`·`~(~pr)`·`{pr}`·`1'b1 ? pr : 1'b0` 는 접고 `pr \| 1'b0`·`pr ^ 1'b0` 는 안 접는다 = elaborator 가 멈추는 지점. verilator 도 아님: `a=10 b=11` vs iverilog·vita `a=x b=x` |
| 2-N-2 | FST 가 `$dumpvars` 스냅샷을 잃는다: 초기화만 다른 두 설계가 byte-identical 473-byte `.fst`(전 신호 `x`, exit 0); VCD 가 다른 24 설계가 동일 FST ⇒ 파형 differential 오라클 불가 | 없는 time step 이 아니다 — time 0 을 lazily 열어도(arm 1회 발화) `xxxxxxxx` 불변 | 값이 fst-writer 의 per-var INITIAL value 로 흡수 ⇒ 다음은 그 라이브러리의 initial-value API. 투기적 변경 revert | n/a |

t0-event 잔여(pre-existing · PRE == POST · 의도적 보류):

- `assign w = 1'bx;`(값이 `x` 인 computed driver 도) vita t0 이벤트, iverilog 아님 — vita driven-net 기본 `z`, iverilog 사실상 `x`.
- 절단 copy `wire [3:0] w; assign w = r8;` — iverilog collapse, vita 는 computed(widening 은 양쪽 발화). `assign #1 w = r;` — iverilog 0, vita 2. multi-driver·2-구동자 `wand`/`wor` — iverilog 0, vita 1(단일 구동자는 copy 규칙으로 해결). concat lvalue `assign {x,y} = …` 는 single-chunk 게이트로 제외.
- vita dirty 채널은 NET 단위, iverilog collapse 는 BIT 단위 ⇒ `bus[1]` 상수 구동자가 vita 에서 `bus[0]` 독자를 깨운다.
- oracle-SPLIT `wire w; assign w = 1'b1; reg r = w;` = iverilog `z`, verilator·vita `1`(§6.8 은 procedure 앞만 정하고 continuous assignment 는 procedure 가 아니다) ⇒ 핀만.
- `buf b1(o1, zin)` vita `x` / iverilog `z`(이웃 `assign o2 = zin;` 은 양쪽 `z`); LRM 표는 x, `oracle_split_rulings.rs` 핀 — `buf` 는 bit move 가 아니라 IEEE 1364 §7.3 z→x 강제.

## 2-R residue

- 리포트 ③ 미착수: 미사용 패키지 함수도 프레임화되고 같은 원인이 인스턴스마다 보고된다(사용성 축).

## 2. Silent-wrong 잔여

Row numbers are cited from tests (`§2 row 7/14/21/25/27/33`, `§2 🆕 I/L/M/N`); never reuse a number. Resolved rows are in ROADMAP_ARCHIVE 「§2 표 해소 행」: 1 · 1d · 1e · 1f · 🆕A · 2 · 2b · 3 · 4 · 6 · 8 · 8b · 9 · 11 · 12 · 13 · 18 · 20 · 21 · 22 · 27 · 28 · 33 · 🆕C · 🆕D · 🆕E · 🆕G · 🆕K.

WALL(provenance) — rows 14 · 15 · 16 · 25 · 26 · 30 · 🆕 F stop in one place: `const_wide.rs`'s `fold_bits_at` decides an expression's sign NODE-LOCALLY (`sg = ls && rs`) where §11.8.1 makes the whole region unsigned if ANY operand is, and a module-scope initializer folds through the width-UNLIMITED `const_eval_in_scope` while a function local's declared width lives in `envw`. Routing a DECLARED-width/sign target through `eval_const_assign` works (row 14: 27 divergent → 3 of 44 cells) but the shared walk is not correct on its own terms — §11.4.10 makes a shift's RIGHT operand self-determined and unsigned, and the i64-lane bound was on the TARGET only — so each attempt was built, measured and reverted. `param_declared_width_provenance.rs` pins the reverted state, the prerequisites and the cells a fix must not move.

WALL(AST self-width) — the size-cast cluster below (width probe, `ir_bits_of` fallbacks, real × fill, prim cast) needs a tree-wide AST pass answering a node's self width WITHOUT lowering it. §4.5.346 proved that pass already stands INSIDE a cast (`const_self_width` + `const_signed_env`).

| row | status | symptom · repro · oracle values | root cause · code site | fix shape · prerequisite / wall |
|---|---|---|---|---|
| 🆕 B | BLOCKED(sign provenance) | ⓐ `localparam [31:0] L1=(B>>>2)+8'd0` = 4294967276, runtime twin 44 (oracles 44); net size polluted too (`logic [((B>>>2)+8'd0)-1:0] bus` 22 bits vs 44) · ⓑ `case (b>>>2)` with an unsigned label: vita `eq236`, oracles `eq44` | ⓐ `const_fn.rs:162` `AShr => Some(a >> b)` has no sign in its signature; wide twin `const_wide.rs:308` uses the left-operand rule · ⓑ `stmt_flow.rs:~605` wraps the lowered scrutinee in an outer `$unsigned`, whose argument is self-determined | ⓐ the right rule is one file over, `const_fn_width.rs:427`; WALL(provenance) · ⓑ re-lower with `lower_size_ctx_entry(scrutinee, w, ext=false)`, wrapper kept as FALLBACK (6 `case`/`casez`/`casex` cells; `case (b/c)` 1 → 3, `b%c` 2 → 1); prerequisite = sign provenance told apart from a default (`expr_self_signed`'s catch-all is not a fact for calls, non-whitelisted sysfuncs, constants folded from them) |
| 3b | BLOCKED(field-key map) | class-property ascending/negative bound normalisation has nowhere to be recorded (PRE == POST) | class fields are not nets (`ClassField` → heap slot); the map is keyed by NetId | prerequisite = a field-key normalisation map · 1 oracle (iverilog dies on an assertion) and the minimal repro is loud for another reason (`C c = new();`) |
| 5 | LOUD | `s = string'(24'h610062);` → iverilog `len=2`·`s=="ab"` / vita E2002 | parser | §3, not §2 · the NUL-stripping report does not parse in vita, so that axis needs another repro |
| 7 | OPEN | a parent `initial` READING a child net at t0 sees X: `initial s = 8'hEE` in a child read as `r = u1.s;` → oracles `ee`, vita `xx`; declaration init, `always @*`, fork-arm and delayed child `initial` are correct | process rank is not recorded | rank on `push_process`; `sim_ir::Process` is frozen ⇒ sidecar ⇒ format bump, both backends · zero corpus demand |
| 10 | OPEN | after `import pk::*;` a `K[31:24]` of a >64-bit package parameter is `11`, oracles `dd` (`pk::K[31:24]` = `dd`, a `$bits`-sized net = 128 are correct) | a wide import binds into `wide_param_bits`; `param_sel_range`'s `walk_scopes_key` does not look there | widening it returns staleness to the ~10 binders of the second map ⇒ what `bind_param_value` made unrepresentable; the halves are different machinery |
| 14 | WALL(provenance) | `localparam logic signed [7:0] NM = -8'sd2; localparam logic [63:0] X = NM ^ 64'h0;` = `fffffffffffffffe`, oracles `00000000000000fe`; the same expression over a FUNCTION LOCAL folds `00…fe` · the routing also fixes `localparam logic [7:0] M = (P + 8'd100) % 8'd7` (P=200: 6, oracles 2), `pk::PA ^ 64'h0`, `int S = (D - C) / 2` (2147483641), the generate-scope `time NM` shadow (2c for 12c), the 65-bit-leaf `/ %` | a module-scope initializer folds through width-UNLIMITED `const_eval_in_scope` | route a DECLARED-width/sign target through `eval_const_assign`; the gate must demand provenance of every LEAF (`param_meta` is a DEFAULT for an untyped parameter, ABSENT for a `time` one), decline above the i64 lane, refuse an unsized FILL operand, answer THREE-valued in the scope walk, and NOT route the PACKAGE binder (row 26) · prerequisite = a width-aware walk correct on its own terms: §11.4.10 shift count (`16'hFF01 << 3'b101` = 0, 30 correct→wrong + 36 loud→wrong, reachable through a constant FUNCTION ⇒ its own row, closes first) and an i64 bound not on the TARGET only (83 cells) |
| 15 | BLOCKED(2-state field) | an OVERRIDE carrying a sized x/z literal loses the unknown plane: `#(.K(8'b1010_010x))` onto `parameter logic [7:0] K` binds `10100100` at exit 0 where oracles keep the x; `8'bzzzzz1z0` binds `11111110`. Five cells, every channel | `params.rs`'s i64-lane test reads only VALUE bits (`bp_get(..).0`); the sibling `fill` arm declines with `fill_is_unknown` | `bp_any_unknown` was shipped and reverted (76 CORRECT cells go loud: a 2-STATE declaration converts x and z to 0) · prerequisite = record the parameter's 2-state-ness (`hdl-parser/src/params.rs` computes `var_kind` and DROPS it; an `hdl-ast` field + SchemaHash re-pin, parser-only), which also closes z→0 (today 1) · the ORIGINAL headline (an unknown plane in the narrow store, 22 loud cells, ~40 sites, demand 0) is separate and stacks on row 14; above bit 64 what survives is z, not x |
| 16 | ORACLE-SPLIT | 12 override cells: 5 diverge from iverilog, but verilator sides with VITA on 4 (`-64'd1`, `<ones> + 64'd1`, `<ones> << 4`) and `~32'd0` is a 3-way split | that is row 17 | a fix would "correct" one side of a live split ⇒ row 21 did NOT thread the context through `override_bits` · the only 2-oracle sub-case = operands already ≥ the target width: `#(.K(~128'd0))` vita `00…00ffffffffffffffff` vs oracles all-ones |
| 17 | ORACLE-SPLIT | `leaf #(.K(32'd0 - 32'd1))` on `parameter logic [127:0] K`: iverilog `ffff…ffff`, verilator `0000…0000ffffffff`, vita `0000000000000000ffffffff_ffffffff` (zero-extends from 64, the i64 lane's width) | — | do not chase: vita matches NEITHER and §6.20.2 does not settle it · neighbours are not split (`64'hFFFF_FFFF_FFFF_FFFF + 64'd0` zero-extends in all three, `-(64'sd1)` sign-extends in all three) |
| 19 | PERF | a 2-D / 3-D / packed element as a continuous-assign LHS costs ~10× on BOTH backends; against 50.0 ns for a 1-D unpacked element: 2-D `arr[0:15][0:3]` 546.7 ns native / 675.8 vm, 3-D 829.2 / 967.5, packed `logic [63:0][31:0]` 410.8 / 441.7 | not located; native/vm ratio 0.81–0.93 ⇒ SHARED PLUMBING | needs its own census; the report's and §4.5.382's diagnoses were both refuted on the 1-D axis |
| 23 | LOUD | `clocking cb; input a_b;` beside `clocking cb_a; input b;` is legal (verilator `R1=17 R2=34`) and vita refuses it with ``net/variable `top.__clk_cb_a_b` redeclared`` at exit 1 | the `__clk_`/`__clkout_` mangling in `sva_clocking.rs:727`/`:657` | correct→loud, verilator is the accept/reject oracle ⇒ §3; a new sigil must be taught to the VCD and FST filters · the naming half is a one-token fix (`:745` computes the instance-qualified `alias`, `:750` re-formats it without `fq`; the `[in …]` suffix is `lvalue.rs:179`) |
| 24 | DO-NOT-START (row 34) | 24a CLOBBER (silent-wrong, exit 0, verilator oracle): a signal merely DECLARED as a clocking output is destroyed to `x` or frozen — `vita x,171,171,171,171,171` vs `verilator 170,171,172,173,174,175`, 4 cells, one across a module boundary · 24b one-cycle LAG, 4 cells | 24a `init_diag.rs::clocking_commit_plan` (~1202), OUTPUT phase unconditional; the INPUT phase is correct · 24b §14.16 skew `#0` in Re-NBA, a scheduler-REGION question with no anchor | 24a = a written flag produced at the write site, `out_pairs` grows a third field riding `SimOpts` out-of-band (no format bump) · corpus demand 0 |
| 25 | WALL(provenance) | `parameter P = 5` + `#(.P(32'hF0F0F0F0))` binds a SIGNED 32-bit −252645136 (`P < 0` = 1, `%0d` negative); `parameter Q = 8'sd1` + `#(.Q(32'hDEADBEEF))` is `ef` with `$bits(Q)` 8 — oracles bind the override's own type (§6.20.2) | `params.rs::param_decl_width_opt`'s literal arm answers the DEFAULT's literal type even when `default_binds == false`; `ResolvedOverride` carries `signed` but no `width` | the producer patch is kept (`scratchpad/r29/row25/producer.patch`, 317 lines + 5 tests; fixed 7,982 of 122,774 cells and serv's `\|WITH_CSR`) but three rounds each found a NEW correct→silent edge (`defparam` with a NAME rhs, a `time` parameter with a DECIMAL default, `$signed(64'h…)` resized down) ⇒ reverted; the size-cast slice ships `param_type_guessed` DECLINING every guessed type · prerequisite = a parent-side resolver that DECLINES on meta-less names and answers `$signed/$unsigned` by operand, carried through every channel including `defparam`; a fill onto an untyped parameter binds `(1, false)` |
| 26 | WALL(provenance) | routing the PACKAGE binder through the width-aware fold is a net loss: 8,748 package-consumer designs, 1,233 correct→silent-wrong against 714 fixed, plus one correct→loud | it makes `pk::X`'s stored value canonical (i64 −2 for `logic signed [7:0] PA = 8'hFE`) while every consumer still folds through the width-unlimited walk and sign-extends it | prerequisite is on the CONSUMER side: `every_name_has_a_declared_width` does not enumerate `ExprKind::PkgScoped`, an imported constant is in no provenance set — closing both turns those 1,233 cells correct · until then the identical text answers `00…fe` in a module and `ff…fe` in a package |
| 30 | WALL(§11.8.1 sign) | `localparam logic [127:0] C = '1 ^ 1'b0;` → vita `…00000000ffffffff`, oracles 128 ones; the same text as `r = …`, `assign c = …` and through a port prints 128 ones. 165 of 264 cells (22 operator forms × {32,33,64,65,96,128} × {`logic`,`logic signed`}), 0 splits, all four binder copies; the old band ("wrong from 33 up") is a property of its operands — `logic [7:0] A = '1 >> 2` is `ff`, oracles `3f`, 82 more cells at widths 1..31 | the wide fold's fill arm plus node-local region sign | the 4-piece fix works (`fold_bits_at` fill arm folds at `ctx` when `ctx>0` and not x/z · `param_i64_fill_at_declared` ahead of the i64 walk in all four binders · a LOCAL predicate with a `Cast` arm · a `fill_width_survived_the_fold` guard): 778 FIXED / 0 new-silent / 0 new-loud over 1,622 cells — reverted; §11.6.1 evaluates at `max(ctx, every self-determined operand's width)` so freezing the fill at the LEAF is wrong (150 cells), the ROUTING predicate cannot serve as the guard (104 NEW-LOUD; a decline at `param_bits_at_declared` is `E3009`), and of 504 PRE-loud→value cells 215 are silently wrong on the SIGN axis (`localparam logic [7:0] B = ($signed(4'hF)+1) \| 8'h00;` `00` vs oracles' `10`, no fill anywhere) ⇒ PREREQUISITE = §11.8.1 region sign in the wide fold; ACCEPT set = "correct a value, never create one" |
| 🆕 F | WALL(§11.8.1 sign) | a narrow SIGNED operand is ZERO-extended in an unsigned context: `localparam logic [7:0] A = 8'hFF - (-1'sb1);` is `fe` and `8'hF0 \| (-1'sb1)` is `f1`; oracles `00` and `ff`. 24 cells at widths 2..32, both sign declarations (no fill involved) | §11.8.2 reinterprets each operand at the EXPRESSION's sign; `const_wide.rs`'s bitwise/arith arms compute `cs = ls && rs` then `resize_bits(.., cs)` for BOTH | same root as row 30's prerequisite — file the fix once |
| 🆕 H | OPEN | ⓐ `(&4'b110x)`·`(\|4'b101x)` 이 바운드에서 1비트 clamp — 두 오라클 3/4 · 8칸(`^`/`~^` 는 분열) · ⓑ ascending `parameter [0:3] P` · lo≠0 `parameter [7:4] P` 의 `\|P` 바운드가 loud(두 오라클 4) · ⓒ `localparam E = 4'hF \| 4'h0; wire [(&E)+2:0]` loud(두 오라클 4) · ⓓ `localparam R = ~(\|4'b1010);` 는 의도적 loud(두 오라클 `0`·`$bits` 1) · ⓔ const-fn 본문 지역 대입 후 리덕션(`t = a[5:0]; return (\|t)+2;`) loud(오라클 3) | ⓐ `fold_self_bits` 리덕션 arm 이 unknown 하나에도 decline · ⓑ `narrow_param_bits` 가 `lo != 0 \|\| ascending` 거절 · ⓔ 본문 지역 폭이 envw 에 없다 | ⓑ 리덕션 전용 레이아웃-무관 resolver · ⓒ WALL(provenance) · ⓓ 그 클래스가 닫히면 `param_init_kept_loud` 삭제 |
| 🆕 I | OPEN | ⓐ another process's read in the same delta: `initial #1 v = 8'hA5;` declared before `initial #1 $display(c);` — oracles `a5`, vita `00` · ⓒ word-alias residue: a RUNTIME index `m[k]` (oracles `a5`, PRE == POST `xx`); `m[1][2]`, an `integer` array into an unsigned copy, `m[32'hFFFFFFFE]` / `m[64'd0 - 64'd2]` are splits (vita E4002 / W4029); not folded: a GENVAR index `assign cw[g] = m[g-2]` (iverilog `a5 5a`), an all-`z` driver beside a partial/delayed driver (E3001), a `force`d copy after `release` · ⓔ `bit [7:0] c; assign c = v;` excluded and unexercised (vita refuses `bit` copy destinations, `E-ELAB-LVALUE-KIND`) · ⓖ a callee body is ONE set of expressions ⇒ marked only for roots EVERY calling process writes (3 of 4 cells); still computed, all oracle SPLITS: an array-WORD-target copy `assign c[0] = v[0]` (iverilog `a5` / verilator `00`), a sign-EXTENDING copy `[15:0] <= signed [7:0]` (iverilog `ffa5` / verilator `0000` for a `wire` destination — for a `logic [15:0]` destination BOTH oracles `ffa5`, vita `xxxx`: pre-existing 2-oracle, review A §4.5.442), a zero-extending / truncating / concat copy, a partial slice `v[3:0]` (iverilog `x` / verilator `5`), `v[7 -: 8]` (iverilog `0` / verilator `4294967295`, vita = verilator); pre-existing silent (2-oracle): a full-range select of an ARRAY WORD `assign c = m[1][7:0]` stays `x` where both oracles read the word (`copy_alias`'s `Select` arm takes a flat net only; review B, §4.5.442); an `always_comb` whose only read is inside a called task runs at HEAD (`a5` = verilator; iverilog `xx` with "no sensitivities", split) | ⓐ a §5.4.1 race kept on the settle's value ON PURPOSE (the store-side forward broke picorv32 / UDP / keccak parity) · ⓕ interpreter/VM take the extension sign from the slot (255), the native path from the node · ⓖ callee reads are not in the sensitivity derivation | ⓖ the copy's declared sign is re-stamped on the aliased read in `eval_core` and `read_scalar_words` (§4.5.442); a mismatch ANYWHERE in a chain disables the tail below it |
| 🆕 J | LOUD | ⓑ `wire [('1)+2:0] x;` — oracles 2, vita loud · ⓒ `$bits('1 & 4'hF)` — oracles 4, vita loud (`bits_of_selfdet` has no operator arms) · ⓓ `{'1, 1'b0}` — illegal (§11.4.12), oracles lenient (2), vita loud; keep · ⓔ `v['1]` — split (iverilog 0, verilator 1), vita = iverilog · ⓕ `'1 * 2'd2` / `'1 + 1'b1` / `4'd8 - '1` widths — verilator 2/1/4, iverilog 3/2/5, values agree, vita = verilator · ⓖ `localparam U = '1; localparam Y = U + 4'd1;` → `$bits(Y)` 32, oracles 4/5 | ⓖ fill-INDEPENDENT (`localparam U = 1;` shows the same 32), the row-14 value-inferred tail (`min_signed_bits(v).max(32)`) | ⓐ (a fill as LEFT operand under a TYPED declaration, 11 cells) is row 30 · ⓖ WALL(provenance) |
| 🆕 M | LOUD | ⓐ `m #(.P('1 ^ 1'b0)) u();` onto `parameter logic [39:0] P` — vita 32 bits (`00ffffffff`), iverilog target-sized (`ffffffffff`), verilator ONE bit (`0000000001`); 40 pre-existing + 9 split cells · ⓑ `(\|'1)` and `{('1 ^ 1'b0)}` in a constant stay loud · ⓔ `cover property (… (a \|-> b))` is loud (`cover.rs` has its own sequence-only grammar) · ⓕ verilator prints NO failure for `a \|-> b and b \|-> a` where §16.12.8 fails at the first failing operand (vita t=35) — not an oracle for property-level `and` | ⓐ the parent folds the override before the target's width is known (`resolve_param_overrides` → `ovr_by_name`) | ⓐ prerequisite = a target-typed override evaluation (row 17's axis) · residue: a select of an ascending or value-sized hierarchical parameter stays loud; `$bits` of a hierarchical string is loud (split 1/16); an override CARRYING past the operands' top bit (`~`, `+`, `<<`, unary minus, `?:`) stays loud; a decimal / `-(64'sd1)` override of an untyped 128-bit-default parameter keeps 128 bits (row 25's i64 half) |
| 🆕 N | OPEN | still open: VCD `$scope` names the block `gi[0]` / `genblk1[0]` (iverilog `begin gi` / `begin genblk1`); a task declared in an unnamed block is a split (iverilog `top.genblk1.t`, verilator `top.genblk1.genblk1.t`, vita loud); a user block named `genblk1` beside an implicit one is not disambiguated (iverilog `genblk01`, verilator refuses); `%m` in a CONCURRENT `assert property` action block omits the assertion label (`top.nb` for verilator's `top.nb.ap`; iverilog refuses concurrent assertions ⇒ 1-oracle) — the label dies in the parser: `hdl_ast::Stmt::ConcurrentAssert` has no `label` field, and adding one to that frozen SchemaHash type flips the root hash (an IMMEDIATE labelled assert already carries its label, verilator's side of a live split); the leniency `gi[0].x` on a conditional scope (oracles reject); a class method called from a generate-block process / a frame names its INSTANCE since §4.5.441 (a label inside the method is kept, verilator; iverilog drops it — split); a `$unit` class prints `top.C.show` (split); a package class, and a class method calling a module task / `$strobe` in a class, are loud; a parameterized class prints `C__8` (verilator `C__N8`); an ELABORATE-time diagnostic in a class spells `[in $class$C$m]`; a package function is `top.pf` (iverilog `p::pf` / verilator `p.pf`, split); `--hier-tree` / `--inst-paths` list no generate scopes | the class table is global and its declaring INSTANCE unknown, so the CALLING scope is prefixed | beside it (loud): an instance ARRAY of a PORTLESS module (`ch w[1:0]()`) is refused with "child has non-ANSI ports" — a false reason on a valid design |
| 🆕 O | OPEN | a RUNTIME read of an outer ARRAY shadowed by a generate-scope scalar `localparam` reads the OUTER array's element: `logic [31:0] ROTA[0:3];` with `generate if (1) begin : g localparam int ROTA = 99; … $display(ROTA[1])` prints `20` (the element just written through `top.ROTA[1]`) where both oracles print `1` — bit 1 of the inner scalar 99. The WHOLE-name read `ROTA` is `99` in all three, so vita contradicts itself in one `$display`; the CONST twin and the WRITE twin are both correct ⇒ runtime READ only (§4.5.443 measured) | `arrays.rs:41`'s `expr_array_chain` calls `lookup_net_scoped`, which walks `symbols` ONLY — a generate-scope `localparam` is bound into `params`, so the walk sails past the inner binding and finds the outer array net | the const lane's twin `const_array_ref_of_base` (`const_array.rs:107`) already has the guard: `walk_scopes_key` over `symbols ∪ params`, answering only when the INNERMOST binding is the array (§4.5.416). Copy it · 2-oracle (use a `logic [7:0]` array — iverilog refuses `int`/unpacked ARRAY PARAMETERS) · size S |
| 🆕 L | LOUD | ⓐ `localparam real P = 3; $bits(P)` — vita 32 / verilator 64 / iverilog 1; §6.12 makes `real` 64 bits so vita's 32 is a silent-wrong on an oracle-SPLIT axis (scoped `p::P` answers 64 — the two `$bits` paths disagree) · ⓑ `$bits` of a string parameter — vita + iverilog 16 (§6.16), verilator 64: vita = LRM, keep · ⓒ `localparam real Q = 1.5; localparam W = Q * 2;` — E3009, oracles 3.0 · ⓓ a 2-state struct's `'{…}` in a constant is loud (`w'(longint'(e))` has no const-fold arm); a 4-state struct's folds · ⓔ a fill inside a `'{…}` in a constant is loud · ⓕ a string or `real` package parameter through `import p::*` — E3010 / E3009 (scoped `p::S` works) · ⓖ `m #(.X('{1'b0, 5'd7}))` of a struct-typed header parameter — E3009, verilator 21 · ⓗ `p::v.a` — E2002 (the struct desugar keys on the bare first segment) · ⓙ `gather_local_decl_names` omits functions, tasks, genvars, instance names, typedef names, array parameters, generate-block contents, and a package importing a package passes an EMPTY set · ⓚ an `import` inside a generate BLOCK is loud E3009 unless redundant; per-scope application is the remaining work · ⓛ `union packed` containing an anonymous `struct packed` fails to parse; `import`/`localparam` in a function body is loud · ⓠ `logic [1:0] i; F[i*4+3:i*4]` → `0000` and `F[i*4 +: 0]` → `0` in silence (verilator refuses; illegal SV) · ⓡ a block-local variable shadowing a wildcard-imported package VARIABLE is read as the local AFTER its block (`SX` → `11`, oracles `a5`) · ⓢ a header parameter redeclared in the body answers the body declaration · ⓣ `c #(.A(x), .A(y)) u();` is accepted, last wins · ⓤ an x/z WRITE into a 2-state member of a 4-state packed struct keeps the x/z — `o.q = 4'bx1z0;` vita `xx1z0x`, iverilog `x0100x` (§7.2.1); the parser knows the member is 2-state (`StructFieldLayout.5`), the write path does not squash · ⓦ residue (loud): a package function whose body reads a package constant outside the i64 interpreter (real / string / array / enum); an INTERFACE importing a package function into a range bound (`apply_import_const_funcs` is wired for modules and packages only — PRE was a silent 1-bit net, both oracles 8, now loud); an `import` inside a generate block (unapplied, loud twice) · ⓧ a compilation-unit `import` after the module still applies, and a package constant used before its declaration folds (iverilog rejects both) · ⓨ the declared-range gate's span dedup reports a generate loop's bad bound once (iverilog three times) · ⓩ residue (§4.5.443 closed the descending / ascending declared range in the wide fold): a NEGATIVE declared LSB is still read positionally — `localparam logic [3:-4] A = 8'h3C; {A[3:0], A[3:0]}` is `cc`, both oracles `33` (PRE == POST; `param_range`'s lo is a `u32`) · loud beside it: runtime `$size(P)` of a scalar parameter, a multi-packed ELEMENT array parameter, a >64-bit base's select (`logic [191:64] A; A[127:64]`, both oracles fold), a whole-NAME read of a shifted or ascending parameter in a >64-bit concatenation (`logic [11:4] A; logic [79:0] L = {A, 72'h0}` — the zero-LSB twin folds; `narrow_param_bits` declines the NAME, which is what keeps the structural select arm sound) · (aa) 324-cell residue: UNTYPED `localparam G = C + D` stays i64 (iverilog 16 / verilator 0, split); enum labels are the same split (vita = verilator); `$clog2(C+D)` in an untyped declaration folds the 32-bit sum (oracles the 4-bit 0); a `byte` operand in a bound (`[Y+Y:0]`, `Y = 100`) reads the LRM/iverilog 57 where verilator and PRE read 201 | ⓩ residue: `param_range` stores `lo: u32`, so a negative declared LSB is not representable | ⓩ residue = widen `param_range`'s lo to `i64` at its producer and every consumer (`param_sel_range` · `norm_offset_for_range` · `const_norm_bit`); measured, not filed as urgent (demand 0) · (aa) the elaborate VALUE lane is row 14's wall · also recorded: the `endpackage` export of `packed_md_params` has no `local_decl_names` filter; `foreach` over a multi-dim packed formal is loud with a misleading enum-method diagnostic |
| 31 | PERF | all 126 pure-family cells are already three-way correct in all nine positions; `assign p1 = $signed(a)*$signed(b)` measured 603 evaluations against 201 for `a*b` (3.0×), same for `a >> $clog2(8)`; demand 22 continuous assigns / 29 occurrences in ibex and verilog-ethernet | — | do not start as a §2 item; re-filed to §5.2 rank 4 · the STATE half is do-not-chase: of 32 wrong cells only 6 are arbitrable (26 have both oracles constant but disagreeing, ivl `z` vs vlt `0`; 4 split on constancy) and the decline makes the one reachable hazard WORSE (2,406,546 settle spins) |
| 32 | LOUD | real, 6 cells, but the run ends with `F4004` and exit 1; the residue is one extra `$display` line | `frame_eval.rs::run_frame_call_with`, one funnel | → §3 tail, ~10–20 product lines · vita's own TASK path already bails without committing the caller's lvalue and matches iverilog; verilator is disqualified because it prints after a TOP-LEVEL `$finish` too |
| 34 | DO-NOT-START | rows 23/24 (clocking): 36 silent-wrong cells and startable, still excluded — 1-oracle only (iverilog 13 cannot parse `clocking`) and demand is zero twice over (no corpus row; all 16 `endclocking` files are inside interfaces, which vita refuses through an unrelated gate) | — | worse than recorded: the signal is destroyed PERMANENTLY, and with a 2-state declaration it clobbers to `0,0,0,…` — a plausible value with no `x` anywhere |

### Size cast / signedness

- 너비 프로브가 진단을 두 번 낸다(값은 정확): 좁힘 판정용 lowering 이 진단과 사이드테이블 등록을 전부 실행한다 — `8'(a >> pk::nope)` E3009 ×2, `2'({u1.nope} % 4)` E3010 ×2, 죽은 노드가 직렬화돼 깊이 32 중첩에서 `.velab` 5.7×(5884 B vs 1035 B). WALL(AST self-width).
- `w = ir_bits_of(plain)` 은 노드의 폭이라 캐스트 피연산자 전체의 self 폭을 못 본다 — `2'((s8>>u3)*s16)` vita `11` / 두 오라클 `01`, 11칸.
- 폭을 모를 때는 어느 기본값도 옳지 않다(시도·되돌림): `2'(u1.mem[0] % 4)`·`2'(u1.k[7:0] % 4)` vita `xx` / iverilog `11`, `2'(s % 4)`(`string s="A"`) `xx` / hand-IEEE `01`. `ir_bits_of` 가 `None` 인 곳 = 지연-hier 읽기 · `string` 넷 · string 을 내는 `SysFunc` 가족. `unwrap_or(u32::MAX)` 는 더 나쁘다(`4'('0 / {u1.k})` `0000` → `xxxx` · `.velab` 22.4× · RSS 10 MB → 1.1 GB · 회귀 41칸) ⇒ WALL(AST self-width).
- 4-state 좁힘이 x 를 떨어뜨린다: `a=8'bxxxx_0011` 에서 `2'(a+1)`·`2'(a*2)`·`2'(-a)`·`2'(a-1)` 이 전부 known / iverilog `xx`(`<<`·`&` 는 4-state 에서도 폐쇄, 4,116칸 발산 0).
- A size cast over a FUNCTION-CALL leaf evaluates at self width (2-oracle): `ast_ctx_signed` answers `None` for a call, so `64'(f(1) - 40)` is `00000000ffffffd0` for the oracles' `ffffffffffffffd0`, 16 of 720 cells. Fix = give `expr_self_signed`'s `_ => false` (21 callers) the declared return type. Residue: a DYNAMIC/queue/associative element's sign is invisible to the classifier; a HIERARCHICAL or class-member operand keeps the pre-slice classifier; a `time` constant from a guessed parameter declines.
- 크기 캐스트의 real 이 fill 과 만나면 조용하다(오라클 ✓): 반대편이 fill ∧ real 원천이 평범한 real 넷이 아님(`parameter real`·real 리터럴·real 반환·`$signed(r)`·`$realtime`·`$sqrt(r)`) ∧ 연산자가 real 비전파(`& | ^ << >> >>> %`) 면 퍼널에 안 들어간다 — 288칸 중 84칸(`4'(RP ^ '0)`·`4'($sqrt(r) & '1)` exit 0, iverilog 는 전부 거부). 자물쇠 둘(`ast_ctx_signed` = `None`, `expr_is_real` 의 `Binary` 팔에 비트/시프트/`%` 없음) ⇒ CLASS · WALL(AST self-width).
- `$signed(real)`/`$unsigned(real)` 이 위치 의존적이다: 캐스트 안 15자리는 거부, 7자리는 exit 0(`$signed(r)*2` → 15 · `%0d`/`%0f` · int/real 대입); iverilog 는 전부 거부. 곁: 2인자 `$signed(r, u)` 를 조용히 받는다.
- prim cast 가 타깃 폭을 문맥결정 피연산자에 안 내린다(오라클 ✓): `a=8'hFF` 에서 `int'(a*a)` `00000001` / iverilog `0000fe01`, `shortint'(a*a)` `00000001` / `fffffe01`. `lower_prim_cast` 는 `lower_ctx_or_plain`(fill 만); 그대로 배선하면 `refuse_real_size_operand` 가 `int'(r)` 를 loud 로 만든다 ⇒ WALL(AST self-width).
- 캐스트의 문맥폭이 안쪽 자기결정 노드에서 멈춘다(두 오라클 ✓): `64'(-16'(u16))` `000000000000fffb` / `fffffffffffffffb`, `8'(s4 * 4'(s8))` `…f9` / `…09`, `16'(s8 + 4'(u8))` `000c` / `010c`; 무중첩 `64'(-u16)` 정상 ⇒ 트리거는 중첩 캐스트·`$signed`/`$unsigned` 노드. 10,368칸 중 143칸.
- 넓히는 캐스트가 불순한 피연산자의 부호 수정을 못 받는다: `extend_to` 의 부호 fill 이 피연산자를 두 번 부르므로 `16'(f())`·`int'(f())` 는 무부호 답 유지(오라클 `fffd`/`fffffffd`) ⇒ 한 번만 부르는 4-state 보존 확장 또는 callee 순수성 술어.
- 캐스트/인라인의 확장 부호가 미러에서 온다 — signed 클래스 필드는 순수·반복 가능한데도 못 받는다: `function signed [63:0] fw; fw = c.sf;`(`8'hAB`) `00…ab` / hand-IEEE `ff…ab`.
- 캐스트가 원소의 부호를 청구하지 못하는 나머지 철자들: `unpacked_elem_signed` 는 base 가 단일 세그먼트 ident 일 때만 청구 — 전부 `40'(x[0]*1)` 에서 vita `00000000fd` / iverilog `fffffffffd`: 다차원 `g[i][j]` · `pk::pm[0]` · 프레임 로컬 배열 · dyn/queue 원소 · 인터페이스 배열 원소. 패키지 철자가 급하다 — `arrays.rs` 는 `pkg::arr[i]` arm 을 이미 갖고 있어 분류기가 자기 lowering 리졸버와 어긋나 있고 한 설계에서 `pm[0]` 정답 · `pk::pm[0]` 오답. 곁: `16'(u1.sarr[0])` `000000000000fff9` / iverilog `fffffffffffffff9`.
- A FILL override (`'1`/`'0`, `#()` or `-G`) binds at the default's width; both oracles bind ONE bit: `#(.P('1))` onto `parameter P = 5` is `-1 bits=32` in vita and `1 bits=1` in both oracles (§6.20.2). Fix = a fill onto an UNTYPED parameter binds `(1, false)`.
- fill override 가 타깃 폭이 아니라 32비트로 접히는 자리 셋(오라클 ✓) — `param_decl_width` 가 `None` 인 형태: ⓐ >64비트(`parameter [127:0] K` + `'1` → `0000…ffffffffffffffff`, iverilog 128비트 전부 1; "wide 파라미터 OVERRIDE 는 loud" 불변식의 구멍) ⓑ `time`(`#(.T('1))` 4294967295 / 18446744073709551615) ⓒ untyped(§12.2.2 — `#(.K(64'hDEADBEEF))` −559038737 / 3735928559) ⓓ `real`(`#(.R('1))` 와 `-G R='1` 이 `4294967295.0` / `1.0`). 한 뿌리 = 한 슬라이스; 세 채널(`#()`·`defparam`·`-G`)의 현재 일치를 깨지 마라.
- A `time` parameter with a DECIMAL default forwards as 32-bit unsigned: `parameter time T = 1 << 40` has no `param_meta`, so `#(.P(T))` types it `(32, unsigned)` via `const_self_width`'s `map_or(32)` and truncates 2^40 to 0; oracles bind 64 bits. Fix = a typed (`time`/`integer`/`int`) declaration records its type as meta even for a non-literal default.

### Constant domain (i64)

- i64 상수 도메인이 오버플로에서 거절한다 — 언어는 문맥 폭에서 wrap 한다(CLASS · 2-오라클 합치): `3037000500 * 3037000500` 두 오라클 145474192 / vita loud · `64'h7FFF… + 64'd1` 0 / loud · `3 ** 40` 689956897 / loud. 문맥 폭 없이 모듈러로 접지 마라(mod 2^64 는 ≤64비트 문맥에서만 옳고 `localparam [127:0] P = 3 ** 41` 이 이미 잘린 값을 zero-extend). 선행조건 = 폭-인식 모듈 스코프 fold; vita 런타임은 전부 정확하다.
- 64비트 상수의 부호 없는 값이 i64 도메인에서 음수로 읽힌다(2-오라클 합치): `localparam L = (64'hFFFFFFFF00000000 > 0) ? 111 : 222;` vita 222 / 두 오라클 111(`parameter [63:0] BIG` 철자도 같다). 뿌리 = `const_eval_i64_lit` 의 64비트 재해석 arm(range 검사가 비교 위치엔 없다); 닫히면 `a_placement_that_does_not_fit_the_i64_domain_declines` 도 연다.
- 폭이 정확히 64 인 비교는 조용히 틀린다(2-오라클 합치): `((64'd1 - 64'd2) > 64'd0)` 두 오라클 1 / vita 0 — `masking = ctx_w > 0 && ctx_w < 64` 의 off-by-one.
- module-scope `localparam` 의 `/`·`%`·`>>>` 는 선언 폭이 있어도 부호를 잃는다: `% 64'd10` 18446744073709551615(ivl 5) · `/ 64'd10` 0(ivl 1844674407370955161) · `>>> 4` 18446744073709551615(ivl 1152921504606846975); `>>` 는 정확 ⇒ 표 14 와 한 항목, 따로 착수하지 마라.
- >64비트는 의도적 decline(`w == 64` 만 unsigned): 두 방향이 서로 반대로 틀린다 — `(64'hFFFF…FFFF + 65'd1) > 64'hFFFF…FFFF` 는 signed 읽기가, `((65'd1-65'd2) > 65'd0)` 은 unsigned 읽기가 맞다(오라클 각 1) ⇒ 추측 금지. 핀 = `const_unsigned_at_sixty_four.rs::above_sixty_four_bits_keeps_the_pre_slice_answer`.
- `*` 의 64비트 unsigned 오버플로는 loud(`64'h8000…0000 * 64'd2` 두 오라클 0 / vita 거부, `checked_mul`). 문맥 폭이 정확히 64일 때만 wrap.
- untyped `localparam` 의 거대 `**` 는 iverilog 가 행(hang) 한다(오라클 부재): `localparam L = 3 ** (64'd0 - 64'd8);` 10분 100% CPU ⇒ verilator 단독 판정.
- placement/캐스트 fold 잔여(honest-loud): carry 연산이 든 concat(`{4'd2,(4'd1+4'd1)}` iverilog 34) · concat 안의 x/z · prim/signing 캐스트(`int'(7)` iverilog 7) · 지역변수에서 온 replication count. carry-free folder 확장 금지 — 해석기 자신의 폭-인식 걷기로 라우팅.
- wrap 하는 SIZE 의 const-도메인 셀은 decline(loud E3009 · iverilog 1): `const_eval_cast` 의 절단 fold 는 무제한 operand fold 위에서 unsound(`4'((4'd8+4'd8)/4'd3)` SV 0 vs 절단 5). 곁: real-반환 const fn 본문 폭(`f = 4'd15+4'd1` → 16.0, self-det 0.0 이 정답). WALL(AST self-width).
- decl-init 호출 사슬이 깊이 캡 64 에 걸린다(correct→loud): 상수함수 70개 사슬이 iverilog 71 / PRE 71 / POST loud — 대안(미구현) = 실행 중인 함수로 재진입할 때만 한 레벨 과금.
- 4-state 지역변수의 무초기화 기본값이 0 이다: `integer x; g = x + 1;` vita 1 / iverilog `x`(2-state `int x;` 는 정답).
- packed 차원 곱이 u32 를 넘으면 패닉(진단 없음): `bit [65535:0][65535:0] tt;` 가 `attempt to multiply with overflow` — 넷 할당 자리.
- 선언 폭 모델이 셋인데 하나만 packed 를 본다: `const_decl_wsign`(곱) · `const_bound.rs::decl_is_wide`(첫 차원만) · `ast_kind_range_width`; 지금은 건전하나 폭을 줄이는 차원 규칙이 생기면 조용히 깨진다.
- 파라미터 선언 fold 가 네 벌(오라클 ✓): 정본 `params.rs::bind_one_param` 밖의 셋이 각자 다르게 빠뜨린다 — `instance.rs`(override 없음) · `generate.rs` · `package.rs`. generate/package 는 fill 기본값을 선언 폭으로 안 접고(`parameter [63:0] Q = '1` → `00000000ffffffff`), `package.rs` 는 `param_range` 를 기록하지 않고(`parameter [15:8] P` 의 부분선택이 `x`) `string`/`real` 을 라우팅하지 않는다. CLASS.

### Index sealing

- queue·dynamic 배열의 인덱스에는 봉인이 없다 — 상수도 넷도(오라클 ✓): 256엔트리 `int q[$]` 에서 `q[-8'sd1]`·`q[s8]`(−1)이 진단 없이 원소 255(iverilog 기본값 `0`), `int d[]` 도 같다. 쓰기 쪽은 loud(W4020) = read/write 비대칭이고 `dynarr.rs` 가 `seal_index_unsigned` 를 안 부른다. verilator 는 2의 거듭제곱 크기에서 마스킹하므로 오라클이 아니다.
- 함수 호출 인덱스는 어느 봉인에도 못 온다(오라클 ✓): `arr[fneg(0)]`(`-8'sd1`)이 조용히 원소 255 / iverilog `xx` — 봉인이 `Call` 을 반복 가능성에서 거절.
- `$bits` 를 상수 인덱스 식에 쓰면 `x` 를 읽는다(오라클 ✓): `m[$bits(m[0]) - 4'sd9]` `xx` / iverilog `10`(리터럴·`$clog2` 철자는 정상).
- 평범한 벡터의 비트/부분선택은 음수 상수 인덱스를 무부호로 읽는다(오라클 ✓): `pv[-2'sd1]` `01`·`pv[3'sd7 -: 2]` `3` / iverilog `0X`·`x`. 별도 퍼널(`sealed_signed_index`/`norm_sub_k`).

### Inline / frame binds

- 인라인 경로가 선언 폭을 본문 안으로 안 내린다 — 프레임 경로는 내린다(오라클 ✓): `function [31:0] fh(input [7:0] x); fh = fld * x;`(8'hFF)가 static `00000001`, `automatic` `0000fe01` = iverilog. `lower_ctx_or_plain(rhs, ctx_w)` 는 fill 만 크기를 준다.
- 프레임 인자 바인드가 §11.6.1 확장 부호를 안 쓴다(오라클 ✓): `8'shf7` → `000000f7` / iverilog `0000fff7`. 세 퍼널 공유(프레임 함수·태스크·클래스 메서드) = CLASS, 1,920칸 중 24칸; 넷 대입과 포트 연결은 정답이라 자리는 바인드.
- `expr_is_repeatable` 이 배열 원소를 거절해 `f(mem[i])` 가 바인드를 못 받는다(오라클 ✓): `gs(arr[2])` `00…f7` / iverilog `ff…f7`. 필요한 것은 반복 가능성이 아니라 부작용 없는 중복.
- 계층 참조·클래스필드 actual 은 선언 폭을 못 받는다: 지어진 32 라 `trusted_self_width` 가 `None` → 바인드가 사퇴하고 결과가 actual 폭으로 나온다(`gs={x,x}` 에 `hi.hv` → 8비트). generate 스코프 이름도 같다.
- `cast_operand_is_real` 의 AST 절반이 bare 단일 세그먼트만 본다(오라클 ✓): `pa(f(0))` 4(정답) · `pa(p::f(0))` 는 f64 페이로드가 2-state formal 로(`c.cm()` 도) — 넓히면 호출부 8곳.
- 인라인 body-local 의 2-state 선언은 x/z 를 안 떨어뜨린다(오라클 ✓): `bit [7:0] b; b = x;` `x7` / iverilog `07`; `fold_straight_line` 에 2-state 단계만 없다 — 바인드와 한 자리로.
- 인라인 바인드의 폭 결정이 `ir_bits_of` 의 지어진 폭을 믿는다: 클래스 필드가 32 를 답해 절단/확장 판정이 뒤집힌다 — `i16(c.bu)`(8비트 필드) `xxc3`. 창은 `필드폭 < formal폭 < 32`; 정본 `canonical_self_width`.
- `real` rhs 는 §10.7 을 건너뛴다 — `resize_inline_assign` 의 `expr_is_real` early-return(`f = r + x*x` `013b` / iverilog `3b`).
- `!trusted_w` 카브아웃 아래는 아직 샌다(설계상 트레이드): `fh = c.big + 1'b1;`(40비트 필드)가 대상 폭에 따라 `00000000` vs `0000010000000000`.
- 곁: actual 이 formal 보다 넓으면 절단하지 않는다(의도적): `f(8'hFF)` `ff` / iverilog `0f`; `{f(8'h02){1'b1}}` 0 / `f`.
- The verbatim inline actual's mirror is wrong on its own path: an 8-bit signed frame-call actual into a 16-bit signed formal (`fs16_add(g(-16))` → `00f0`, oracles `fff0`) — `bind_formal_actual` widens by the actual's MIRROR sign (`Call ⇒ false`).
- 바인딩 자리는 넷이 아니라 아홉 — 다섯이 남았다(2-오라클 · `f(300.0)`→`input byte` 가 300, 오라클 44): ⓐ output formal 을 가진 frame 함수 ⓑ 계층 task 호출(인자가 `inline_task.rs` 에서 formal 폭 없이 미리 lowering) ⓒ 계층 함수 호출 ⓓ class 메서드/task ⓔ class 생성자. ⓑⓒ 는 구조가 다르다.
- `expr_is_repeatable` decline 이 남기는 조용한 기본값(2-오라클): 사용자 `Call`(`f(rfn(3))`) · real 배열/큐 원소 · 화이트리스트 밖 SysFunc(`$sqrt`·`$itor`·`$bitstoreal`) · `p::rf(...)`. `$random` 은 decline 이 옳다.
- `time` 선언의 명시 `signed` 한정자가 버려진다(2-오라클): `input time signed k` 에서 `k/2` 오라클 −4 / vita 9223372036854775804 — `kind_signedness` 가 `time`→unsigned 하드코딩.
- 범위 밖 real 의 정수 클램프: `real rv = 1.0e300; byte'(rv)` 두 오라클 0 / vita −1(`±inf`·NaN 포함).
- `int'($random*1.0)` 의 draw 횟수(둘 다 틀렸고 값이 바뀌었다): `lower_prim_cast` 에 `expr_is_repeatable` 게이트가 없어 캐스트당 4회 draw(iverilog 1회).

### Real

- `automatic`(framed) real 함수를 직접 피연산자로 쓰면 넓힌다(2-오라클): `fa(1) + (-s)` 두 오라클 −7 / vita 9; 곁 `{fa(1), 1'b0}` 이 조용히 통과. 공유 규칙의 `Call` arm 이 그 형태에 도달하지 않는다.
- package/class 함수도 같은 구멍: `p::one() + (-s)` · `c.getr() + (-s)` 두 오라클 −7 / vita 9.
- 나머지 변환 경계는 문맥-결정: `real r; r = (-s);` −8.0 / 8.0, `r = (s+s)` 0.0 / −16.0 ⇒ Binary·Ternary 는 닫혔고 단순 대입은 남았다.
- real-반환 const fn 의 본문은 §2 가 아니라 §3: `localparam real R = f();` 에 `E3009 … not a foldable constant expression`(iverilog 0.000000) = honest-loud.
- `$realtobits`/`$bitstoreal` 이 64비트 아닌 인자를 조용히 받는다(iverilog "requires a 64-bit argument"); vita 는 저64비트를 답한다.

### Ranges / bounds / selects

- 파라미터의 PART-SELECT 를 폭 바운드로 쓰면 조용히 1비트(오라클 ✓ iverilog): `localparam logic [31:0] W = 32'hdeadbeef; logic [W[7:0]-1:0] v;` `$bits(v)=1` / iverilog 239. 전체 파라미터는 정상 ⇒ part-select 가 상수 바운드 도메인에 안 닿는다.
- const 배열을 가리는 안쪽 스칼라 — GAP-G 의 shadow 검사가 첫 가지에만 없다(오라클 하나, verilator): generate 안쪽 `localparam int ROT = 99;` 가 `localparam int ROT [0:3]` 을 가리면 `logic [ROT[1]:0] v` 가 vita `$bits=21` / verilator 2. `const_array_vals_of_base` 의 첫 가지가 `walk_scopes_key` 히트에서 바로 반환해 둘째 가지의 inner-wins 검사를 건너뛴다; 모듈 스코프 철자는 정확.
- loud 잔여(오라클은 답한다): >64비트 파라미터 셀렉트(`wide_param_bits` 에 비트가 있고 i64 `params` 엔 없다) · 헤더 파라미터가 다른 헤더 파라미터의 셀렉트로 기본값을 받는 형태 · `#(.N(W[7:0]))` override · `defparam` · struct 멤버 폭(파서 갭) · 클래스 속성.
- self-referential 반환 range 는 스택 오버플로(오라클 없음 — iverilog 도 abort): `function [f():0] f();` — `const_fn_ret_wsign` 이 call 깊이를 안 끈다. 처방 = `depth + 1` 한 줄.

### Class fields

- `ir_bits_of` 가 클래스 필드의 폭을 핸들넷에서 읽는다: 진짜 폭은 `class_field_widths` 사이드카에만 있어 틀린 `Some(32)`(`16'(c.sb)` `xxxd` / hand-IEEE `fffd`). CLASS, 정본 `canonical_self_width`. 두 번째 증상: 캐스트 폭이 지어진 32 와 같으면 `Ordering::Equal` 이 resize 를 건너뛰어 캐스트가 사라진다 — `32'(c.s8)` `fd`(`fffffffd`) · `32'(c.s8 + ua[0])` `fa`(`000001fa`).
- 오름차순 음수 bound 가 클래스 속성에서만 클램프된다(오라클 하나 — verilator 4비트, iverilog assertion 사망): `class C; logic [-3:0] q;` 가 W3056 + exit 0 로 틀린 값(표 3b). 더 값싼 절반: 정규화 안 된 클래스 필드 셀렉트는 lsb≠0 인 `logic [7:1] q` 에서도 깨져 있다.
- packed 음수 low bound dim 의 비트 선택은 좌표를 만들지 못한다: `logic [-3:0][1:0] x; x[-3]` — 옳은 좌표 `(lo+size-1) - idx` 의 부호 있는 뺄셈을 `dim_coord` 의 오름차순 arm 이 안 짓는다(두 빌드 모두 loud). 전체 값과 `$bits` 는 정확.

### Scoping / imports / block-locals

- block-local 선언이 같은 이름의 IMPORT 된 package 변수를 clobber 한다(2-오라클 합치): `import pk::*` 뒤 `begin : blk integer pv; pv = 99; end` 이후 `pk::pv` 가 vita 99 / 두 오라클 5 — bare name 으로 모듈 net 에 flatten 하는 v1 모델이 import alias 와 같은 칸에 앉는다.
- 파라미터와 넷을 같은 이름으로 선언하면 vita 만 받는다(두 오라클 모두 거부): `localparam N = 7; logic [3:0] N;` 를 vita 는 받고 파라미터로 해석한다(`r=7`). vita 가 지어낸 확장이라 loud 화는 사다리 하강이 아니고, shadow 규칙의 `!params` 절이 유일하게 관측되는 자리라 불변임을 핀해 뒀다(`block_local_shadows_param.rs`).
- queue·assoc 원소의 part-select 쓰기가 조용히 사라진다(verilator 오라클 · iverilog 구문 거부): `q[0][15:8]=8'h0F;` → verilator `ffff0fff` / vita `ffffffff`; dyn(`q[]`) 철자는 맞는다 = write-twin 갭.
- 폭 0 인덱스 part-select 를 조용히 받는다: `parameter P = 0; t[i +: P] = …` 를 iverilog 는 거부, vita 는 exit 0(§3 성격).

### Delays / events

- 런타임 변수 지연(2-오라클): `assign #(dv) y = a;` dv=5 면 오라클 5 / vita 0 — 엔진이 서스펜션 시점에 평가해야 한다. `#(D, dv)` 부분 fold 도 같은 뿌리(rise 만 접히고 fall 이 rise 값). 핀 = `structural_delay_scope_fold.rs::a_runtime_variable_delay_is_still_zero_delay_and_still_quiet`.
- 사이즈드 음수 리터럴이 새 규칙에 도달하지 못한다: `#(-4'd1)` 은 `const_delay_ticks` 안의 `const_eval_u32` 가 32비트 `wrapping_neg`(두 오라클 15)인데 파라미터 쌍둥이 `parameter [3:0] Q = -4'd1` 는 15 = 두 철자가 갈린다. 고침은 `const_delay_u64` 의 `_` arm 이 `Unary` 를 declines 하는 한 줄이지만 `#(-5000000000)` 의 폭 판정을 오라클로 먼저 재야 한다.
- zero-rise 사이드카 거래(`#(ZERO_PARAM, F)` 의 fall 이 버려진다 · `#(0,F)` 는 맞다): 뿌리는 엔진 — `Some(0)` 은 CA 를 delayed 레인으로 보내고 zero-tick write 가 Postponed 리전 뒤에 착지한다(두 오라클 1 / vita 0) ⇒ silent↔silent 맞바꿈 금지로 보류; zero-tick lag 를 고치면 둘 다 열린다.
- `TimeLit` 이 식의 루트가 아니면 안 접힌다: `#(5ns + 2ns)`·`#(2*5ns)`·`#(2.5ns)` 두 오라클 8/11/4 / vita 0.
- 지연 CA 의 인덱스 안 함수 호출(`assign #(D) y = arr[h(a)];`)이 `native_eval/compile.rs` 에서 패닉(exit 101) — iverilog 는 돈다.
- cont-assign 만 구동하는 wire 가 t=0 에 가짜 이벤트를 낸다(iverilog 1오라클 · verilator 미조회 ⇒ 3-오라클 census 필수): `wire b; assign #5 b = a;` 에서 b 가 z 로 시작해 t=0 settle 의 z→x 가 `always @(b)` 를 깨운다; iverilog 는 t=0 이벤트가 없다. `assign d = c ^ 1'b0;` 도 같다 ⇒ 초기값 도메인 문제.

### Diagnostics / artifacts

- 기본 백엔드의 Mul 체인이 밑수를 n 번 재-lower 해 진단을 n 배로 낸다: `r <= m[idx] ** 16;` 이 interp/native E4002 2건, bytecode 8건 + "further suppressed". 값은 안 틀리지만 8-cap 을 먹어 뒤쪽 진단을 지운다.
- `coverpoint_domain` 의 Pow arm 이 정본과 어긋난다: `max(lw,rw)`/`ls && rs` 로 접는데 정본은 LHS 폭·base 부호. 영향 = 커버리지 auto-bin 개수뿐.
- `--obs-dir` 의 run.json 이 `-G` 를 안 싣는다: `-G W=9` 와 `-G W=100` 의 run.json 이 타임스탬프 말고 동일 — 효과가 다른 설계인 유일한 플래그에 OBS rail 이 눈멀어 있다(§6 OBS-1 과 같이).
- `%h` 가 1비트 미지 EXPRESSION 결과를 `x` 로, iverilog 는 `X` 로 찍는다: `$display("%h", ^a)`; 같은 값의 1비트 NET 은 양쪽 다 `x` 라 iverilog 가 일관되지 않고 IEEE §21.2.1.3 은 vita 편. 215설계 중 17칸.

### Performance (open, recorded)

- A continuous assign whose RHS contains ANY `Expr::Call` or `Expr::SysFunc` is re-evaluated 6.00× per input change instead of 1.00× (300,001 evals for 50,000 iterations vs 50,001; the same call in an `always @*` is 1.00×). Root = `levelize::expr_is_pure_of_nets` (`levelize.rs:329`), whose `E::SysFunc{..} | E::Call{..} | E::ArrayItem{..} => false` arm sets `dirty_ok=false` and drops the assign into `ca_always`. `$unsigned(src) ^ …` 1.89×, `… ^ 128'($bits(src))` 4.90×; the inliner trips it (`resize_inline_assign` seals with `$signed`/`$unsigned`, `inline_fn.rs:631,655`) so an inlined function measured 1.98× SLOWER (0.158 s vs 0.080 s). Fix order: (1) a per-`SysFuncId` ALLOW-list, `_`-free exhaustive (~79 variants, ~22 impure); (2) the dep set for `Expr::Call` — `expr_nets`' Call arm (`levelize.rs:161`) walks only the ARGS, so the reject is SOUND today; prize 5.95×; (3) then the body cost (2.33× ceiling). Certification moves the DIAGNOSTIC stream (a pure RHS `errors=5`, the same RHS in a no-op `$unsigned` `errors=9`) and must be adjudicated first. No `pure` flag on `FuncDef`/`SimIr` (frozen); computable out-of-band.
- `coerce_two_state` names its operand once per TARGET BIT and the engine walks that DAG as a tree: `byte'` 8, `int'` 32, `longint'` 64, `int'(int'(x))` 1024 against iverilog's 1; the discriminator is 2-state-ness, not width (`integer'` and `int'` differ by 27×). The `expr_may_be_unknown` guard in `lower_prim_cast` took 1024 → 32. Still wrong: `int'(f())` names `f` 32 times because a `Call` is conservatively unknown, and a WIDENING cast over a call fans out to the wider width ⇒ needs the `expr_is_repeatable` gate.
- Coercing at the OPERAND's width instead of the TARGET's took the repro 69.6 s → 6.7 s (10.4×), ping count 32 → 4 (the hand-written `{28'd0, nb}` control is 2.76 s). Still open — the residue is the 4 surviving terms plus the frame call, now the LARGER half. Not shipped: a per-bit skip fires 0 times on 41 cast cells; the third caller (`inline_fn.rs:396`) has no resize in front of it, so narrowing would change the value. The reorder's own silent-wrong: `ir_bits_of` answers `None` for a deferred hierarchical reference (also a `string` net, string-producing system functions, the `pop`/array-reduction family) and the caller FABRICATES 32 — `longint'(u1.w40)` with `logic [39:0] w40` is `0000001234567800` in iverilog and PRE, the unguarded reorder printed `0000000034567800` ⇒ take it only where the width is a DECLARED fact.
- 4-state actual → 2-state formal 강제가 런타임 O(선언폭)(3백엔드 동일: `byte` 12.8× `shortint` 23.8× `int` 46.4×). 진짜 수정 = x/z→0 IR 프리미티브(format bump) 또는 엔진 memoize; 완화 둘(per-query 메모·노드 예산)은 개선 0 으로 반증(비용은 바인드 개수에 있고 영속 캐시는 in-place 패치와 충돌).
- 상수 도메인의 비교/논리/삼항조건 fold 가 ~3배 느리다(값은 정답): `const_int_selfdet` 이 트리를 두 번 더 걷는다 = 피연산자당 6 walk vs 옛 2. 병리적: 1,500 localparam × 60항 0.35 → 1.00 s · 이중 generate-for 3.14 → 11.73 s · 컨트롤 1.00×; 현실 설계에선 안 보인다(picorv32 0.030 → 0.030 s). 처방 ⓐ 폭·부호 walk 융합 ⓑ generate-for bound 의 genvar-free 부분식 메모.
- `==?`/`!=?` 좌편향 체인은 여전히 2^depth: 깊이 22 에서 30 s → 79 s. 값은 정확.

### Oracle splits (recorded, not chased)

- untyped localparam 의 정수 init 폭: `localparam L = 4'd15 + 4'd1` 이 vita 16 = iverilog 16 / verilator 0.
- iverilog 는 64비트 unsigned `%` 에서 자기모순: `64'hFFFFFFFFFFFFFFFF % 64'd10` = 5 인데 `(64'd0 - 64'd1) % 64'd10` = 1 ⇒ 그 축의 오라클로 `(0-1)` 철자를 쓰지 마라.
- 크로스스코프 t0 decl-init race(양쪽 §6.8 합법) · 런타임 구성 `-0.0` 표시 · iverilog 자인 결함들(expression-force "evaluated once" 등).
- `$stime` 의 부호: `16'($stime)` 이 t=0x8000 에서 vita/verilator 5.050 `00008000`, iverilog 13.0 `ffff8000`; IEEE 1364-2005 §17.7.2 = "returns an unsigned integer that is a 32-bit time" 이고 vita 는 캐스트 밖에서도 무부호(`q = $stime` at t=2^31 → `0000000080000000`).
- Mutual recursion across two packages in a constant function (`p::f(4)` ↔ `q::g`): vita 10 = hand-IEEE (4+3+2+1+0), verilator 8, iverilog cannot parse it (review A, §4.5.440).
- `#(.S("str"))` 가 적용되기 전에 W3056 을 한 번 낸다(값은 정답): 부모 쪽 숫자 fold 가 먼저 실패해 "override 는 상수가 아니다; 기본값 유지" 를 찍고 그 다음 string 채널이 적용한다.

## 3. Loud→supported 후보 (전부 loud=안전 · additive)

코퍼스 10/10 에 거절 0 ⇒ 워크로드 코퍼스는 더 이상 §3 착수 순서를 정하지 않는다. §3 의 남은 줄은 §2 정확성 큐 뒤에 선다.

### 3.a Numbered open items

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| ③ⓐ | `&&`/`\|\|` 우항 · `?:` arm 의 파일읽기 호출 hoist 불가 | 건너뛸 수 있는 평가(§11.4.7/§11.4.11) · `hoist/general.rs` guard block | `guarded_hoist` 적용 + fd 상태 순서 증명 | iverilog | — |
| ③ⓑ | `while`/`for` 조건의 호출 | 한 번 hoist 는 한 번만 읽는다 | 루프 본문 안으로 재작성(`lower_shortcircuit_cond` 모양) | iverilog | — |
| ③ⓒ | `$feof` 가 살아남는 문장 전부 거절 · `x = $feof(fd)*10 + $fgetc(fd)` EOF 근처 vita 9 / iverilog −1(파일 중간은 일치) | `$feof` 는 파일 위치를 읽는데 hoist 가 변이를 앞으로 옮긴다 · arm 마다 순서 규약이 다름(`assign_seq` rhs→인덱스 · `Case` scrutinee→라벨 · 태스크 인자열) | 선행조건 = `order_walk` 급 순서 판정기 | iverilog | — |
| ③ⓓ | 별칭을 이름으로 못 붙이는 읽기(`m.a` · `p::v` · `Shape::NoHoist` 자식) + 호출이 ref 를 쓰면 fail-closed 거절 | 겹침을 루트 이름으로 판정 | net 동일성으로 판정 | iverilog | — |
| ⑤ | ibex 페이지 19→13 = `export "DPI-C"`(12, loud by design) + ibex_core:2481 `cs_registers_i.g_pmp_csrs[i_region].x`(1) | 파서가 상수 인덱스만 세그먼트 이름에 접는다; genvar 는 elaborate 만 안다 | path segment 에 인덱스 식 = AST 모양 ⇒ DEEP | verilator | DEEP |
| ⑤ⓐ | multi-packed 파라미터: `$size`/`$left`/`$dimensions` · 값으로서의 `'{…}` · 그런 타입의 ARRAY 파라미터 · `import p::*; import q::*` 가 둘 다 `P` export | `packed_md.rs` flat 재작성 밖 | 소비자별 arm | verilator (iverilog: "packed array parameters are not supported yet") | — |
| ⑤ⓒ | header array parameter: NESTED(2-D) override 패턴 · 원소 >64bit override/whole-array default · `defparam` · interface-header array parameter · override 로서의 `'{default: v}` · 헤더 뒤 BODY `localparam` 이 원소 폭을 이름 부르는 형태 | `array_param_twin`/`const_array_override_vals` 밖 | 채널 확장 | verilator-value | — |
| ⑤ⓔ | element select: MULTI-PACKED element(`logic [1:0][3:0] A[2]` 의 `A[1][0]`·whole-element read·`$size(A,2)`) · concat/replication count 안의 ascending·non-zero-LSB element · `p::S[1].b` · 런타임 `$size` · 원소 밖 select · UNTYPED child param 의 element-select override | 원소 capture 가 declines(PRE 동일) | 도메인 확장 | verilator-value | — |
| ⑤ⓕ | unpacked-array typedef residue (§4.5.445 supported the declaration, the ANSI port, package / `$unit` / block-local / generate scope, chained alias, decl-init, dynamic `[]` / queue `[$]` / assoc `[string]` dims and an interface member): still loud — a function RETURN type (1-oracle, verilator only) · a tf-port FORMAL · a NON-ANSI port · a `parameter` of that type · `parameter type T = a_t` (both oracles `$bits` 32) · `$bits(a_t)` of the bare TYPE name (both oracles 32; the parse-time width table has no dim product) · dims on BOTH the typedef and the declarator (`a_t y [0:1]`, a live oracle SPLIT on dimension ORDER — iverilog `$size(y,1)=4 $size(y,2)=2`, verilator `2` / `4`, and iverilog contradicts its own answer for the identical explicit type) | each consumer reads `TypeInfo` and has no slot for unpacked dims; every one of them now DECLINES on `!info.unpacked.is_empty()` rather than binding the element type | per-consumer, each its own slice; the split row is do-not-start | 2-oracle except the function return type (1) and the declarator-dims split (0) | S each |
| ⑤ⓓ | nested struct member: 비-fill 비-0 `v` 의 `default: v` · `o.i.e.name()` · packed ARRAY 멤버 `in_t [1:0] i` · UNPACKED record 안의 packed struct · `u.c.perms.q` · `$bits(pkg::T)` · `o.i[1+:2] = …` · 멤버 폭이 `1 << 3`/`8'd5`/전방참조 localparam/header `parameter`(overridable=correct-loud) | 파서 flat layout 표가 받는 소스 종류 밖 | 소비자별 확장 | 2-oracle (`default: v` = verilator whole / iverilog 거절) | — |
| ⑤ | CU-scope: unit-scope VARIABLE/net · class 본문의 unit enum label · unit 상수 사이 전방참조 · `$unit::t` · `assign` 이 구동하는 enum-typed output port(E3018) | 파서 unit-scope 클론 밖 | 항목별 | 2-oracle (전방참조는 split, vita 는 iverilog 편) | — |
| ⑤ | parser/preprocessor: multi-dim packed formal 이 동시에 unpacked array(`logic [1:0][3:0] a [2]`) · 32비트 미만 파라미터의 based-literal 값이 parse-time 표 밖 · 비-ANSI `<type> [dims]` 포트 · dims 를 가진 atom typedef · dims 를 가진 SIGNED typedef element · 이름 없는/중복 `define formal | 파서 flat 재작성 · `define 인자 파서 | 표/재작성 확장 | 2-oracle / split (verilator lenient) | — |
| ⑤ | `parameter type` 의 struct/enum/union/real/string/class 또는 다차원 default·override | `T$w`/`T$s` 두 값 파라미터 desugar 로 표현 불가 | loud by design | 2-oracle | — |
| ⑧ | 함수 본문의 시스템 함수 거절 — `assign m = f()` 안의 `$random`/`$time` 이 매 패스 다시 뽑힌다 | 씨앗은 넷이 아니라 `levelize::func_read_deps` 가 이름 부를 수 없다 | 넷 아닌 상태를 의존 집합에 표현 | 2-oracle(둘 다 얼린다) | — |
| ⑧ | 도달한 `$finish` 다음 문장이 실행된다(`$fatal` 과 같은 동작) | `SimState::frame_end_is_loud` 의 경계가 문장 | 경계를 식 수준으로 | iverilog 는 멈춘다 | — |
| ⑧ | output formal 을 가진 함수는 `Terminator::Call` 로 라우팅돼 `$finish` 를 수행(exit 0) — 같은 구문이 formal 방향에 따라 두 답 | 라우팅이 formal 방향에 갈린다 | 라우팅 통일 | iverilog 는 구문 자체를 거부 | — |
| ⑧ | 카운터를 이월하는 함수의 잔여 불일치는 규칙이 아니라 평가 한 번 | vita 의 t0 추가 settle 패스 | 이 가족의 정직한 인증 선행조건 | 2-oracle | — |
| ⑨ | `import pk::*;` 뒤 바레 string/real 파라미터 이름 loud(fold 성공, import 바인딩만 없음) | `apply_import_consts` 가 `params`(i64)로만 재바인딩 | string/real 사이드맵에 같은 대우 = 라우팅 아닌 배관, 호출부 둘. 핀 = `string_const_domain.rs` · `real_params.rs` | 2-oracle | — |
| ⑨ | `generate if (P::R > 1.0)` real 조건 loud | `const_real.rs` 에 `PkgScoped` arm 없음 | arm 추가 | 2-oracle | small |
| ⑬ | 서브루틴 본문 안 배열 접근이 CALL 문에 귀속 | tier-3 arena 가 RECORD 만 하고 caller 문장 경계에서 drain ⇒ callee StmtId 를 모른다 | 두 번째 `cur_stmt` 원본 또는 `Rc<Cell>` 공유. 트랩: publish 를 넣으면 interp 가 `d.sv:6`, native 가 무위치 ⇒ 백엔드 일치를 택했다 | — | — |
| ⑬ | terminator 조건(`if (mem[i])`) · cont-assign settle · t0 arm · delayed-CA apply drain 은 무위치 | 블록 마지막 문장 뒤 평가라 `cur_stmt` 를 NO_STMT 로 지운다 | 틀린 줄보다 무위치가 낫다(의도) | — | — |
| ⑬ | W4022 · W4028 · delta-limit · RunRange · W4020 · W4029/W4007 인스턴스 경로 = `location: None` | sid 없는 진단군은 접근 문장 키가 엔진에 없다 | `cur_stmt` + `stmt_diag_meta`(배관 존재) · §4.5.249 `SpanResolver` + StmtId→span 사이드카 | — | — |
| ⑭ | call-tree-to-task 미출하: 인라인된 서브루틴은 0 calls 로 보고돼 "free" 로 읽힌다 | 호출을 두 방식으로 lower — call seam frame body, elaborate-time INLINE splice(`inline_task.rs`/`inline_fn.rs`; 14.39 s inline vs 0.35 s frame) | 선행조건 = 인라인 사이트→caller 의 elaborate-time 기록(`Sidecars::func_names` 존재, 선언 `file:line:col` 쌍둥이만 없음) | — | — |
| ⑭ | 리포터는 ~440 cycle/s 를 원하고 20.4 를 잰다 = 21x scheduler/executor | 관측성 아님 | Phase D codegen + arena — §5.1 에서 추적 | — | — |

### 3.b Small residues

**Parser accept**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| specify | `specify … endspecify` 블록 자체가 E2002(`specparam` 은 모듈 상수로 수용) | 파서 미수용 | `specparam` hoist + 경로지연/타이밍체크 폐기. 선행 = `hdl_parser::parse` 에 경고 채널이 없어 조용히 버리면 `$setup` 이 loud→silent ⇒ `ModuleItem` 마커 + elaborate `W3056` | iverilog | — |
| case-inside | `case (x) inside {…}`(§12.5.4) = E2002 | 파서 미수용 | hand-IEEE `==?` + 내부차분 | no-oracle | — |
| based-ws | `64'sh FFFF` = lexer reject | 렉서 | 허용 | iverilog 허용 | minor |
| tf-localparam | `task automatic t; localparam int K = 3;` → `E2002 expected statement, found keyword 'localparam'`(IEEE §6.20 허용) | 파서 statement 위치 | 선언 수용 | iverilog | small |
| R30-1 | 빠진 패키지 → E2002 7줄, 패키지 이름 0줄 | tf-port 의 `IDENT::IDENT` 를 파서가 타입으로 못 받는다 | 타입으로 받고 elaborate 가 "unknown package" ⇒ 1줄 | — | 파서 |
| enum-label | `enum bit[3:0] {A=8'hFF}` → `enum_defs` 미등록 → `.first`/`.next`/`.name` 全 E3010/E3009; out-of-range 검증 skip 으로 silent-truncate | `const_lit` 이 unsized-decimal 만 fold | const_lit 확장 또는 elaborate-time 검사 | iverilog reject | — |
| md-packed-write | md-packed nested part-select WRITE: ascending/non-zero-lsb leaf · genvar-index `x[g][m:l]`(over-reject) · const-OOB packed idx = silent no-op | §4.5.145 는 descending zero-lsb leaf 한정 | leaf 기하 확장 | — | — |
| misc-parse | 음수-LSB 멤버 sub-select · generate 내 `import` · package 자기-func init · SYS-READ hier-element dest · hier-write sentinel panic→loud · EXT2-A2c `logic[1:0][7:0] PK` · EXT2-NAP `'{k:v}` | — | hand-IEEE + 내부차분 | no-oracle | — |

**Constants / parameters**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| wide-override | wide(>64bit) 파라미터 OVERRIDE loud — `#(.K(128'h…))` 거절 | override 채널이 i64/string 뿐 | `ResolvedOverride` 에 wide 슬롯; 자식 폭을 모르는 부모에서 접으므로 원문을 넘겨 자식 폭에서 재폴딩 | — | — |
| §3.3 | wide `localparam` part-select fold — `{A[127:64], 64'h0}` | 인덱스 fold 가 필요한 별개 arm | arm 추가 | iverilog 는 접는다 | — |
| real-fold | `localparam/parameter real` 의 `+`·`*`·`/`·`-`·`**` 全 E3009 "not foldable"; `$clog2(real-lit)` 동근 | `const_eval_in_scope` = i64-only | real f64 arithmetic 추가 | iverilog folds | broad |
| xz-fill-param | `localparam logic [W] P = 'x` 가 0 bind(x 소실) → `P==0`·`P+1`·`P ==? pat` 全 divergent | `fill_to_i64`/`fill_literal_const` | 상수 도메인에 x/z | — | broad |
| compound-==? | `==?` fold 잔여 = unsized x/z 패턴 · negative-signed LHS · non-literal RHS · param override 비상수(W3056→error) · longint MIN fold(package) · loud-message 품질 2건 | §4.5.146 은 sized 패턴만 | 확장 | — | — |
| defparam-iface | `ifc a(); defparam a.D = 255;` → `W3056 … matched no instance` + 기본값 유지(iverilog `d=ff`, vita `d=8`) | `defparams` 소비가 `elaborate_instance` 에만 · `iface_inst.rs` 는 자기 `overrides` 만 | `defparams.remove(path)` 를 정본 바인더에서 병합 | iverilog | small |
| neg-ascending | `reg [-33:-2]` → `$bits` vita 1 / iverilog 32, loud `W3056`. 하강 `[-2:-33]`·혼합 `[3:-2]` 정상 | `array_geom.rs` `allow_neg_lsb` opt-in | 그 조합을 opt-in 경로에 | iverilog | — |
| neg-bound-part | 음수 bound net PART select: `q[-3 +: 2]`·`q[-1 -: 2]` 정확, `[msb:lsb]` 만 막힘. 쓰기 비대칭 — `x[-3:-2]=…` 조용히 정확, `x[-1:0]=…` 은 "out of order" 라는 사실과 다른 방향 진단으로 loud | 바운드 fold 가 unsigned | `const_bound_signed` | verilator | — |
| neg-elem-bound | `logic [-3:0] q[$]` W3056 클램프(verilator `q[0][-3]`=1) | 원소 net 이 `elaborate_netvar_decl_inner` early-`continue` 경로라 선언 사이드맵에 안 닿는다 | 사이드맵에 닿게 | verilator | — |
| §3.1(c) | `always_comb` 구동 변수의 선언 초기화자 무경고 — verilator `MULTIDRIVEN` error · xrun `*E,MULAXX` · iverilog 실행(2:1) | elaborate 층 · 감도 리스트 합성이 그 집합을 안다 | W2004 급 경고 한 줄 | split(lint) | small |
| aes§2 | 인라이너 판별자가 `automatic` 만이 아니다 — plain 3/5 · `automatic`·`for`·`if`·`case` 1/5 · `p::f()` 2/5 | 인라이너 판별자 | 인라이너 확장 | 실측 | — |

**Subroutine / frame**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| §3.11 | `function automatic` 인라인 — "비-재귀 automatic 은 인라인과 동일" 은 측정으로 반증(전 스위트 15 실패 · `$random` 두 번) | 인라인 확장이 피연산자를 두 번째로 이름 부른다 | ⓐ 한 번만 이름 부르기(callee-purity 술어) · ⓑ codegen 개방(`is_codegen_able` 의 `Terminator::Call` 거부 = §5 T1/T2) | — | — |
| static-local | `function integer f(input integer x); integer s; begin s = s + x; f = s; end` → E3010 `undeclared net/variable top.s` | 블록-로컬 flatten 이 definite-assignment 요구; 제어 흐름이 있으면 프레임이 되어 정상 ⇒ 갭은 "직선 본문 + 읽고-쓰는 static 로컬" 뿐 | flatten 에 read-before-write 슬롯 | iverilog 는 X 로 실행 | — |
| frame-oob | 프레임-로컬 배열 OOB 읽기에 E4002 없음(모듈 배열은 E4002+exit 1) | 프레임 로컬 배열은 packed 슬롯이라 array-word 개념이 없다 | elaborate 가 슬롯의 원래 기하를 남긴다 | vita 내부 비일관 | — |
| 2seg-call | output-formal 호출 왼쪽의 `c.m()`·`u.size()`·`t.size()`·`ci.get_coverage()` loud. 의도된 교환: PRE 는 조용히 틀렸다(`t.o` 로 `q=12`, 정답 11) | `order_walk` opacity 를 `callee_body_cannot_touch`(단일 세그먼트 전용)로 답할 수 없다 | 클래스 메서드/패키지 함수 본문 리졸버 | iverilog | — |
| dyn-formal-pos | dyn-formal 호출 불가 자리 = `&&`/`\|\|` 우변 · 다른 호출의 인자 · select/lvalue 인덱스 · `case` scrutinee · `repeat` 카운트 · cast/replicate 피연산자. 지원 7 / loud 10(9건 iverilog PASS = false-loud) | 좁은 hoister(`hoist_dyn_formal_calls`)와 범용 hoister(`shape()`)가 다른 위치 집합 | 범용 hoister 흡수(`__t = f(arr)`). 함정: stand-down 이 `frame_fn_lowering` 이라 프레임 함수 본문만 남았고 거기선 hoist 가 정상 ⇒ call-kind 단위로 쪼갠다 | iverilog 9/10 | — |
| pkg-default | package 함수 default 인자 스코프 — `default_binding_matches_decl_scope` 가 `tf_decl_scope` 와 비교하나 package 함수는 import 한 모듈 프리픽스로 기록 | 심볼별 선언 스코프 미기록 | 심볼별 선언 스코프 기록 | iverilog | — |
| fgets-rhs | `return $fgets(line, fd);` = E3009 "…only as the direct rhs of a blocking assignment" | 라우팅 술어와 로워링이 같은 형태에 키잉 | 둘을 함께 넓힌다 | iverilog | — |
| V3/V4 | `wait(<frame-local>)` · `repeat(<non-const>) @`(hidden counter 가 SHARED net) · NBA-to-frame-local · `fork`/`disable fork`/`wait fork`-in-task | per-activation repeat counter · in-frame fork machinery 부재 | 각각 별개 슬라이스 | iverilog | — |
| frame-array | frame-local array 의 multi-dim · non-zero-based · non-simple-element · whole-copy(`b=a`) · `foreach` · NBA-elem · `'{…}` init | §4.5.169 범위 밖 | 확장 | iverilog | — |
| V2A/V5 | automatic/recursive task 의 dyn-array formal(frame formal 이 scalar slot) · FUNCTION dyn-array local(`&self` 실행기가 `new[]`=`&mut` heap 실행 불가) · recursion/concurrent · multi-dim/packed/non-bit-vector element | `&self` 실행기 | handle-in-slot · per-activation heap stash | iverilog | — |
| foreach-fn | FUNCTION + `foreach`-on-dyn-formal loud | framed function dyn formal 미지원 | function-frame dyn-formal 슬라이스 | iverilog | — |
| r16-exec | ① dyn local/formal 을 쓰는 recursion ② string/real/class-handle element 의 dyn formal ③ unwritten output formal = IEEE §13.5.2 empty copy-out | — | per-activation heap stash | iverilog 는 by-ref = 非준수 | — |
| re-forward | FUNCTION 이 자기 dyn-formal 을 재전달(`return sum(c)`) | framed function formal 이 heap-resident 라 `dyn_array_actual_net` 이 못 resolve | mutual-recursion soundness hole ⇒ frame-route 시 guard | iverilog | — |
| hier-task | output/inout/array/string formal(cross-boundary copy-out) · STATIC task hier-call · nested-in-frame-body hier enable(`task_calls_func` transitivity) · generate-block 내 hier task call | — | frame⊂inline 동등화 | iverilog | large |
| array-formal | non-zero-base descending 배열 formal · hier-task OUTPUT/INOUT 배열 formal · frame-formal 배열을 nested hier 로 forward · 재전달 · non-zero-LSB 원소 · 2-D/signed/task array formal | — | — | iverilog | hard |
| blk-automatic | block-local automatic lifetime(`automatic int j=k*10`) | per-activation storage = deep block-local-flatten | — | iverilog 도 거부 | deep |
| misc-sub | `q.min()[0]` · `x.name().len()` · pkg TASK statement call · method/ctor NAME-default class-scope · G4 string-return frame call | — | — | no-oracle | — |

**System tasks & file I/O**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| plusargs-%0d | `$value$plusargs` 의 `%0d` 폭-지정 스펙 과잉거부 — iverilog 는 받는다(값 5), vita E3009 | 스펙 파서가 폭 수식자를 안 벗긴다 | 폭 수식자 제거 — `exec::plusargs::effect` 의 conv 문자 추출과 한 철자(풀리면 `'0'` 이 %s 로 읽히는 함정) | iverilog | small |
| writemem-local | `task automatic t; reg [7:0] loc[0:1]; … $writememh("x.txt", loc);` = E3009 "a whole unpacked-array formal has no value here" | 두 백엔드 동일 pre-existing | 여는 슬라이스는 seam 도 함께 — `read_task_net` 이 이 거부를 도달 불가 논거로 아레나를 맨손으로 읽는다; 핀 = `writemem_targets_the_seam_cannot_own_are_refused_before_the_backend` | iverilog 는 파일을 쓴다 | — |
| filepos | `$ftell`/`$sscanf` = E3009 "unsupported system function in expression", `$fseek` = W3056 warn+skip | expression 문맥의 side-effect sysfunc | statement-form desugar 확장 | iverilog 동작(`A=6 B=0 C=6 D=0`, `$sscanf`→`2 12 34`) | — |
| fmonitor | `$fmonitor`/`$fstrobe` = W3056 skip = 파일출력 silent drop(warned) | `FmtCapture` 에 fd 가 없다 | `FmtCapture` 에 `fd:Option<u32>` + strobe drain 을 `file_write` 라우팅. format bump 필요 · STDIN read 는 결정성 설계 | — | 전용 슬라이스 |
| $typename | enum / packed struct 가 base 타입으로 렌더(`logic[1:0]`; IEEE §20.6.1 은 `enum{...}`) | 렌더 한정 | 렌더 확장 · 핀 `typename_pins.rs` | no-oracle | 값 무영향 |
| %p-ⓐ | UNPACKED STRUCT 와 `string sa[2]` 가 DECLARATION 에서 E3010 ⇒ 렌더할 net 이 없다 | 선언 갭 | 그 feature 아래로 재filing | verilator | — |
| %p-ⓒ | 기록된 발산 둘: NEGATIVE assoc key(vita 는 IEEE §7.9.4 SIGNED key 순, verilator 는 hex 정렬; `-1` 이 64비트) · `real` unpacked array(verilator 는 원소 0 만, QUEUE 는 정확 = 자기모순 ⇒ vita 는 verilator 자신의 재귀 규칙) | — | 핀 유지 | verilator only | — |
| sformatf | `$sformatf` 를 ternary arm / 단락 우변 / `$monitor`·`$strobe` 인자 / 태스크 인자에 두는 형태 | `eval` 의 `SysFuncId::Sformatf` arm 이 포맷 문자열을 무시 | `format_args_str`/`render_template` 을 리더 제네릭으로 올려 `EvalCtx` 에서 쓰면 통째로 닫히고 statement-level hoist 은퇴 | — | — |
| ext-round20 | ① §4.11 의 미격리 79 건 ② 감싸는 같은-이름 쌍은 shadowing 이라 loud, 모듈 넷과 이름이 겹치는 블록 로컬도 그대로 | — | — | — | — |

**Nets / timing**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| E3001-delayed | `assign #(D) bus = en ? d : 1'bz;` 둘 이상 겹친 tri-state 버스가 exit 1(E3001) | `check_whole_net_multidriver` 가 "드라이버 중 하나라도 delayed 면 4-state wire 해석 대상 아님" 으로 `md_nets` 를 그대로 비춘다 | 엔진 `md_nets` 가 delayed 드라이버를 해석. 트랩: 렌즈의 "PRE 는 맞았다" BLOCKING 은 자기 프로브의 10 ns 격자가 버려진 2 ns 지연을 못 본 것이라 철회됐다 | 2-oracle (1 ns: `t=11 bus=1` / iverilog `bus=z`) | — |
| E3001-overlap | 같은-범위 part-select 쌍(`assign z8[3:0]=…` ×2)·delayed+plain 겹침을 iverilog 는 비트 단위로 해상(`zzzz0xx1`), vita 는 E3001 | 비트 단위 드라이버 맵 부재 | 비트 단위 드라이버 맵(§2 ⓑ 인프라) 선결 | iverilog | — |
| hier-event | ``always @(`TOP.a_uVDC.RTRIM_I)`` — 읽기는 이미 동작 ⇒ sensitivity 등록만 | 패치 대상이 `Process.sensitivity.edges[i].net` 이고 그 프로세스는 아직 push 되지 않았다 | (proc_idx, edge_idx) 예약 후 instance 확정 시 패치하는 새 lane | iverilog | — |
| xproc-disable | cross-process `disable` | 미지원 | "suspend 상태가 아닌 대상의 `disable` 은 no-op" 만으로 그 라이브러리 통과. 경계: suspend 중인 대상까지 무시하면 silent-wrong — 활성이면 loud | iverilog | — |
| timescale | partial-timescale 진단(`W-PARSE-TIMESCALE-PARTIAL`/`E-PP-TIMESCALE-PARTIAL`) — 일부 모듈만 선언 시 무진단 1ns/1ns(전무 케이스만 W1017) | 배선 부재 | doc-08 §15 설계 · `rt.default_used` 존재 — 배선만 | — | small |
| deep | t0 race · `@(*)` decl-init wake · runtime `==?` pattern · inline body NON-fill context-width · modport 방향 강제 · force part-select · assoc key/clocking array-output word0 · 음수 range bound 의 PART select(§2) | — | — | — | deep |

**Diagnostics quality**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| E3009-anchor | `E3010`/`E3009` file:line 비일관 — 붙는 자리(`d_trunc.v:3:20`)도, 계층 경로만 나오는 자리도 있다 | 앵커 미전달 | `diag::SpanResolver` 존재 ⇒ 앵커를 안 넘기는 호출 지점 전수가 범위 | — | — |
| error_at | 앵커와 `found` 가 다른 토큰 — `g[w].u.q` 는 앵커 `w`, 메시지 `found '.'` | `error_at` 은 더 이른 노드에, `found` 는 커서 토큰 | R29-1 이 별개 필드로 분리 ⇒ correct-but-confusing. 사이트 10곳 | — | — |
| #9 | `velab -L`(worklib 병합) 경로에 위치 없음 | 각 CU 스팬이 자기 확장 버퍼(0부터)를 인덱싱해 좌표공간이 겹친다; 틀린 CU 맵이면 틀린 file:line ⇒ `None` 유지 | 병합 시 스팬 오프셋 재작성(AST 전체 walk) | — | — |
| cli-lib | `cargo test -p cli --no-default-features --lib` 이 E0004 로 죽는다(pre-existing) | lib 테스트 타깃이 dev-dep 링크로 sim-engine 의 `oracle` 만 되살리고 cli feature 는 꺼진 채라 `backend_name` 의 `#[cfg(feature="oracle")]` arm 둘이 잘린다 | cli dev-dep 을 `default-features = false` 로 잡거나 두 crate 의 `oracle` 을 하나로. CI 는 못 본다 ⇒ 그 명령에 `-p cli` 를 더하지 마라 | — | — |
| EXT2-DOC | 문서 stale(CLI-ref · lang-ref · system-tasks · explain) | — | — | — | — |

**Strings / heap**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| paren-select | 괄호 base 의 string byte select 가 조용히 0 — `(p)[0]`=0 vs `p[0]`=119(`(v)[0]`·`(v)[3:0]` 은 정확) | `string_index_read` base gate 가 `Ident\|BitSelect` 만 `matches!` 하고 `Paren` 을 unwrap 안 함 → width-0 handle 의 packed bit-select 로 낙하 | gate 에 `Paren` unwrap | no-oracle | 한 gate |
| real-part-write | `real x; x[3:0] = 4'hF` → 값 불변·무진단 | §4.5.220 이 dyn `real` ELEMENT 를 loud 화 ⇒ scalar 가 뒤처진 비대칭 | scalar 도 loud | iverilog 거부("can not select part of real") | — |
| reduction-init | `string s = $sformatf("%0d", arr.sum());` 가 E3009 "unsupported hierarchical function call arr.sum"(`q.size()`·`.len()`·`.substr()`·`.name()` 은 동작) | t0 pre-sweep 경로의 갭 | pre-sweep 경로에 reduction | — | — |
| string-array | §0 T1: FIXED string array decl-init(`string s[2]='{"a","b"}`) · fixed array 런타임 인덱스/`foreach` · `string q[$]` · `string s[2][2]` · 계층 `u.s[0]` · frame-local string array(static task=E3018 · function/automatic=E3009) · dyn element byte select `d[0][0]` | fixed 는 element-net 표현 때문에 const-index 전용 | — | iverilog ✓ | T1 |
| inline-string | static task inline string local(`hoist_inline_task_locals`) = Wire→E3018 | inline 경로 = frame-slot 아님 · 순진 String 화 시 str_bytes twin 미적용 | 별개 inline-string-storage 슬라이스 | — | — |
| string-misc | substr-actual `s[i]` · `s[i:j]` · `s[i].len()` · whole-element read(`x=arr[i]`) · record array-of-record · queue/assoc 의 string·real 요소 · string queue · block-local queue decl · `u.q[0]` 계층 read | — | — | — | — |

**VCD / real conversion**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| vcd | cosmetic encoding 차이(decode 동일): ① vita full-width(`bxxxxxxxx`) vs iverilog strip(`bx`·`b0`) ② t=0 초기덤프 = `$dumpvars` 에 pre-assign X + `#0` change vs settled 값 ③ logic 절차구동 시 vita `wire` vs iverilog `reg` · `int`=`reg` vs `integer` ④ real size `64` vs `1` ⑤ `parameter` 미덤프 | elaborate packed-md `NetVar.lsb` stale(VCD helper 서 flat fallback 우회) | — | iverilog | 큰 golden churn |
| x→real | X-bearing integral→real — vita 는 whole X → `0.0`, iverilog 는 per-bit X→0(`4'bxx01`→1). `$itor`/`$sqrt`/`$pow`/real-`**` 공통 | `real_arg` = `to_i128_signed().unwrap_or(0)` | per-bit 변환 | iverilog | non-silent |
| wide→real | width>128 정수→real 은 여전히 `0.0`(65..=128 은 수정됨) | `to_i128_signed` 가 128 비트까지 | word-grid f64 근사 | — | 초희귀 |

### 3.c Intentionally loud (not gaps)

| id | reason |
|---|---|
| §3.1 DPI-C · `export "DPI-C" function` | 영구 비목표 |
| `$value$plusargs` in an arbitrary expression | `ok = $value$plusargs(…)` 와 `if ($value$plusargs(…))` 는 이미 동작; 남은 `$display("%0d", $value$plusargs(…))` 류는 side-effect sysfunc 패밀리의 설계(single-eval 보장을 위한 statement-form lower)라 desugar 할 statement 가 없어 loud 가 정답 |
| `%p` of `int a[0:0]` | `sim_ir::NetVar` 는 `array_len` 만 나르고 scalar 도 1, `unpacked_array_nets` 는 엔진에 안 닿는다 ⇒ 받으면 중괄호 없이 원소만 exit 0 으로 찍는다. 선행조건 = IR 의 array-ness 또는 새 사이드카 + format bump |
| unpacked 배열 포트의 방향 불일치(`[0:3]` ↔ `[3:0]`) | IEEE §7.6 은 원소를 위치로 짝지어 flat-index 연결이 순서를 뒤집는다(vita 4 / iverilog 1); 구현하려면 위치↔인덱스 매핑을 `wire_array_port` 와 배열 대입이 한 철자로 써야 한다 |
| 커버리지를 증명할 수 없는 고정 배열 채움(계산 인덱스·조건부 쓰기·불완전 집합) | 규칙이지 갭이 아니다; 진단이 이유를 직접 말한다 |
| 한 프레임 본문 식에서 같은 dyn-formal 함수를 2회 또는 자기 재귀 호출 | 마커 슬롯이 하나 |
| 블록이 블록을 감싸며 같은 이름을 재선언하는 shadowing | 초기화자를 가진 static 블록 로컬 둘이 한 이름이면 평탄화된 넷 하나에 pre-arm 초기화가 둘 걸려 뒤가 앞을 덮는다(iverilog 7/9 → 9/9) |
| `$readmem*` child-vs-parent `initial` 경쟁 | IEEE §4.7 이 `initial` 순서를 nondeterministic 으로 두고 두 오라클이 반대로 쓴다 — iverilog `aa bb cc dd`, verilator `01 02 03 04`, vita == verilator ⇒ ORACLE SPLIT |
| `$readmemh` into a `wire` array | iverilog 거부 · verilator 수용 ⇒ split; vita 는 local/hierarchical parity 로 수용 |
| header default 가 body import 의 상수를 이름 부르는 것 | split — iverilog 거절, verilator fold |
| iverilog 13.0 결함 2건(vita 가 IEEE 정답) | ① 루프 본문 블록이 로컬을 선언하면 `break` 가 `continue` 처럼 동작 ② `case` item 안 `continue` 에서 `vthread.cc` assertion abort |
| `%u`/`%z` · `%l` | 양쪽 다 문서화된 선택(vita 무출력 · iverilog raw 바이트) · `%l` 은 cosmetic |

## 4. SVA / 검증 honest-loud 잔여

- empty-match `##0`/unbounded `##[m:$]` 융합 — 오라클 부재 — 선행 = §16.9.2.1 불연속.
- N2c full sequence local var(중첩 attempt 각자 데이터=L급) — 단일-capture 는 지원 — 선행 = 중첩 attempt 데이터 모델.
- later-antecedent read · outer-`|=>` prop-ref skew 고급형 — 오라클 없음 — 선행 = 2-cycle·중첩·cross-clock census.
- SVA-QUAD collapse default-flip — `VITA_SVA_COLLAPSE` opt-in 상태 — 선행 = full-VCD 골든 audit.
- N4 clocking 잔여 = skew 값 자체(블록 전역 `default input/output SKEW`(IEEE §14.3)는 파싱·적용된다) — `output #0` 은 iverilog 가 `clocking` 을 파싱 못 하고 verilator 는 Observed 리전 샘플이라 앵커가 없다 — 선행 = hand-IEEE §14.11/§14.16. `input #0`/`#N`/`##N` 은 다른 리전이라 loud 유지.
- class: down-cast `Derived'(base)` · real→longint cast · base-shadow `Base'(d).v` · cast-as-receiver `(B'(d)).foo()` — 선행 = `$cast` 타입가드.

## 5. perf / 하드닝

Performance axis: diminishing returns reached; performance ranks below the correctness ladder.
재개는 각 행의 재진입 조건이 사실이 될 때만.

### 5.a 서 있는 판정

| id | verdict | reason (one clause) | re-entry condition |
|---|---|---|---|
| codegen (cranelift) | 기각 | 경계가 런의 ~38% 인데 천장이 8.9~11.3%(§5.1-be) · 실행되는 wprog 프로그램의 56~86% 가 op 하나짜리 `Load`/`Const` | leaf 로드와 2-state 산술을 생성 코드에 인라인(호출 0)하면서 의미를 두 번 안 적을 방법 |
| D2-b 저장소 2-state | 거부 | 트랩이 사다리 하강이다 | 정확성 거래 없는 방법을 먼저 찾을 것 |
| cycle-based 모드 | 거부 | picorv32 비율 10.32 vs 게이트 1.84 · 조합 블록이 사이클당 0.097 회만 평가되어 이벤트 구동이 이미 조합 작업의 90.3% 를 건너뛴다 | 블록당 평가/사이클 ≥1 인 실수요 |
| levelize (랭크 순 Active 드레인) | 폐기 | 랭크를 지어 재니 깊이 1~24 전 구간 1.00× · 뿌리는 `settle_cont_assigns` 였고 dirty-settle 이 닫았다 | 없음 |
| 프로세스 융합(E 축) | REVERTED | intra-delta 순서를 iverilog 에 핀한 시뮬레이터에서 의미보존이 아니다 — 체인 출력의 독자가 완전 전파된 값을 봐서 exit 0 · 진단 없음 · 값이 다름 = silent-wrong | 없음(반례 = `a_comb_chain_output_is_sampled_mid_propagation`) |
| 넷 개수 축소(flatten) | 보류 | `--probe`/계층 VCD/`%m`/계층 참조가 지목하는 대상을 지운다(G2 충돌) | 오너 판정 |
| S4 스케줄 소거 · S5 NBA 전용화 | 중단 | S4 표적 합 ≈6% = 1.06× 로 중단 판정(<1.3×) 아래 · S5 는 `k_schedule_nba_scalar` 3.8% | 없음 |
| settle 안 wprog 거절 가족 | 착수 안 함 | 전부 승인해도 serv 전체의 ~2~2.5% 인데 `Tern` 을 조건부 점프로 바꿔야 한다 | 상금이 기구를 넘길 때 |
| §5.1-c | 완료 | 슬라이스 2(heap) 그라운딩 — 넷으로 갈린다 | — |
| §5.1-e | 기록 | 오라클 부식 — V1 이 자기 오라클을 무디게 한다(실측) | — |
| §5.1-f | 완료 | A1 그라운딩 census — Phase A 의 두 숫자를 정정 | — |
| §5.1-n | 완료 | A3-i subset 호출 · 81.66% → 84.91% | — |
| §5.1-p | 완료 | A2-i plain OOP · 88.55% → 90.59% · 착수 전 census 재실행 규칙의 출처 | — |
| §5.1-av | 완료 | D1 벤치 확장 — 하네스가 제품 백엔드를 안 재고 있었다 | — |
| §5.1-ax | 완료 | D1.6 — 필요조건을 충분조건으로 쓴 대가 · struct 축 회복 | — |
| §5.1-az | 완료 | D4 착수 전 재census — 프로파일이 cranelift 가 다음이 아니라고 답했다 | — |
| §5.1-bb | 기각 | `drain_range_diags` early-out — 이득 0 | — |
| §5.1-be | 기각 | D4 기계어 코드젠 — 지어서, 배선해서, 재서 기각 | codegen 행과 동일 |

`§5.1-<x>` 원문 = ROADMAP_ARCHIVE_PHASE_A-D.md `#### 5.1-`.

### 5.b 열린 성능·하드닝 잔여

| id | symptom · measurement | mechanism · code site | fix shape · prerequisite | expected gain (as stated) |
|---|---|---|---|---|
| 4b-r | interp 는 풀링하는데 native 는 안 한다 | `fire_waiters` 의 `Vec<bool>` · `settle_cont_assigns` 의 md-group `vals` | 커널 스크래치 재사용 | 미측정 |
| 5c | 프레임 본문이 컴파일 백엔드에 안 들어간다 · POST 프로파일(keccak_f · 5,949 샘플) 제네릭 walk 25.1% vs `WProg::run` 2.0% | 프레임 윈도가 `Vec<Value>`(`state/mod.rs:585`)라 슬롯 읽기마다 72바이트 복제(`frame_eval.rs:281`) · `wprog` 의 `Load { vi }` 는 평탄한 u64 쌍을 요구 ⇒ `arena.frame` declines(`wprog.rs:461`) | 평탄 워드 윈도 · 선행 = arena | 6–10주 · 대체분 ~38% · 상한 2.33×(keccak_f_arr 한 행) · 재가격 aes 3.13× · arr 2.52× · keccak_f 1.65× |
| ARR-LHS | 2-D/3-D/packed element LHS 가 양 백엔드에서 ~10× 절벽 — 1-D 50.0 ns 대비 2-D 546.7/675.8 · 3-D 829.2/967.5 · packed `logic [63:0][31:0]` 410.8/441.7 ns(native/vm) | 미특정 · 비가 0.81–0.93 이라 공유 plumbing | 자체 census 선행 — 추측 금지 | 미산정 |
| INLINE-FOLD | 인라인 fold 가 지수적 — `elab_s` 0.35 ms 평탄, `sim_s` 0.16 s → 14.36 s · 로컬을 1회 읽는 6문장은 0.19 s 인라인 / 0.24 s 프레임, 3회면 14.39 s / 0.35 s | 아레나가 서브트리를 DAG 로 공유하는데 평가기가 TREE 로 재-walk | per-activation memoisation · 인라이너 확장은 반대 방향 | 미산정 |
| MEM-GUARD | 프로세스 수준 메모리 가드가 없다 — 폭주한 `vita` 하나가 33 GB × 2 를 잡아 커널 패닉까지 몰았다 | 기존 가드(`max_deltas`·`max_body_steps`·`time_limit`)는 델타도 문장도 진행하지 않는 시스템태스크 내부 루프를 못 본다(`$writemem*` 는 닫힘) | ⓑ RSS 워치독 + 기본 상한 + `--max-mem` · 선행 = macOS `mach_task_basic_info` 가 unsafe FFI · ⓐ 할당 카운팅 allocator 는 기본 ON 이면 회귀 | — |
| CI-NEXTEST | `cargo test --workspace` 450 타깃 순차 = 724 s(per-target 합 62 s) vs `cargo nextest run --workspace` 같은 5183 테스트 30 s · CI 4잡만 남았다 | 두 실행기는 빌드 트리를 공유하지 않아 전환마다 ≈470 s 재빌드 | ci.yml 4잡 교체 + nextest 0.9.100(0.9.143 은 rustc 1.91 요구) · 선행 = temp 이름 충돌(368개 파일 `vita_<tag>_<pid>_<프로세스별 카운터>` · nextest 는 테스트마다 새 프로세스라 카운터 0 부터 + PID 재사용) | 24× |
| MSRV-CEIL | "새 Rust 를 따라간다" 정책 미검증 — toolchain·`rust-version`·ci.yml 4잡 전부 1.85.0 고정 · `stable`/`beta` 잡 0개 | 천장을 아무도 안 밟는다 | 비차단 `stable` 잡 1개 | — |
| EXEC-ROWS | `native::run::executor_rows` 가 모든 `simulate` 에서 백엔드와 무관하게 전 프로세스의 전 문장을 훑는다 | census 는 실행기와 무관해야 하므로 무조건이 맞다 | 비용이 문제면 캐시 | — |
| DELAY-CLAMP | u32::MAX 를 넘는 `#delay` 는 여전히 CLAMP — 4.29e9 틱에 실제로 도달하는 런에서만 틀리다 | IR 필드가 u32(동결 타입) | 표현하려면 format bump · 알리려면 새 W-code | correct-or-loud 완성 |
| KPRED-3RD | ③층 판정에 "오늘의 커널이 돌릴 수 있는가" 층이 없다 — `$sformatf`·`$display`·transport-delay NBA·재arm 은 적격·빌드 가능인데 커널이 없다 | run.json 은 `eligible`/`buildable` 만 싣고 `kpred::rhs_routes_to_worker` 는 게이트에 안 물려 있다 | dispatch 배선 때 세 번째 층 | 오독 제거 |
| QUIESCE-NBA | ③층 quiescence 가 커널의 `delayed_nba` 를 안 본다 — 트랜스포트가 유일한 대기 작업이면 quiescent 로 보고되고 업데이트가 사라진다 | 엔진 `next` 는 `Scheduler` 의 `wheel`/`delayed_ca`/`delayed_nba` 최소값인데 네이티브 런에서 그 맵은 비어 있다 | S1d-4c-2 | 사다리 하강의 유일한 잔여 |
| BYTE-GATE-6 | S1d-4d 바이트 동일 게이트가 만날 pre-existing 오라클 차이 6건(전부 PRE==POST): ① `$finish` 틱의 pending NBA/트랜스포트 ② VCD intra-tick 입도 ③ t=0 initial 순서 ④ t0 arm 순서 ⑤ 읽기 집합이 빈 `always @(*)` 는 vita 만 t0 에 돈다 ⑥ `.velab` 이 `vcmp` 실행 간 재현 안 됨(RULEV-MTIME) | 의도된 설계 반 · LRM 미정의 반 | 여섯 개를 어느 쪽으로 고정할지 먼저 | — |
| MON-RENDER | `$monitor`/`$strobe` 의 ③층 렌더 경로 거부 | 렌더가 `sched/run_loop.rs::flush_postponed` 인데 그 경로가 리더를 안 받는다 | 배선 = S1d-4c 와 한 슬라이스 | 거부 해제 |
| FD-EOF + FEOF | `NetArena` 의 `fd_eof` X-poison 구멍(`fd_eof` 만 "heap/class/frame 없음" 논증 밖 · 지금은 `$feof` 과잉표시가 가림) · `$feof` 가 정본 stmt-effect 술어에서 과잉표시라 `e = $feof(fd);` 거부 · `while (!$feof(fd))` 통과 | `k_feof` 는 순수 읽기인데 `sysfunc_is_stmt_effect` 가 `true` · 한 소비자만 고치면 철자가 둘 | 한 슬라이스로 · 정본 수정 = tier-2 게이트도 넓힘 · byte-identity 논증 | ③층 과잉거부 해소 |
| NETSLOT-PREV | `NetSlot.prev` 를 읽는 곳이 워크스페이스 전체에서 0(선언·생성자·pass (c) 쓰기뿐) ⇒ pass (c) 의 `clone_from` 2회/변경넷/델타가 죽은 일 | 아무도 안 읽음 | 제거 · 자명함 자체를 검증하는 별도 슬라이스 | perf |
| LOW-ROI | FMT-CACHE part b(render_template pre-segment) · GEN-3X-STR part a(unroll plan 캐시 = byte-identity 위험>이득) · QUEUE-MID-ON(스펙 내재 O(n) · iverilog 동일) | — | 보류 · QUEUE-MID-ON 은 영구 비권장 monitor-only | — |

### 5.c Current state

| | |
|---|---|
| 기본 백엔드 | `native`(③층) · 코퍼스 100.00% 실행 · 발산 0 |
| 제품 형태 | `--no-default-features` = 실행기 하나 · 게이트 거부는 치명 |
| 워크로드 코퍼스 | 10/10 · 거절 0 |
| 코드젠 | 기본 OFF · 기각됨(§5.1-be) — 빌드·배선·측정·정확성은 갖춰 둔 상태 |

## 5.2 Queue (start order)

Canonical start order. LOOPROMPT.md NEXT mirrors this table; when they differ this table wins. Slot 1 = §3 item, slots 2·3 = §2 rows (single root, two oracles, outside the walls and the oracle splits, no format bump).

| # | slot | item | source | rank |
|---|---|---|---|---|
| 1 | 1 | §3 ⑤ⓕ: the unpacked-array typedef residue, cheapest first — a tf-port FORMAL and a NON-ANSI port (both 2-oracle, both already declining with the reason named), then `$bits(a_t)` of the bare type name (both oracles 32) | §3 ⑤ⓕ | ② |
| 2 | 2 | §2 🆕 O: a runtime read of an outer ARRAY shadowed by a generate-scope scalar `localparam` reads the outer array's element (`ROTA[1]` = 20 for both oracles' 1), while the WHOLE-name read of the same name in the same `$display` is right — `arrays.rs:41`'s `lookup_net_scoped` walks `symbols` only; the const lane's twin already carries the innermost-binding guard (§4.5.416) | §2 🆕 O | ① |
| 3 | 3 | §2 🆕 L ⓩ residue: a NEGATIVE declared LSB (`logic [3:-4] A; {A[3:0], A[3:0]}` = `cc`, both oracles `33`) — `param_range`'s lo is a `u32`, so widening it to `i64` touches its producer and `param_sel_range` / `norm_offset_for_range` / `const_norm_bit`. Re-measure demand at start | §2 🆕 L ⓩ | ① |
| 4 | next | mixed-caller callee (intersection rule, design decision) · `m #(8)` / `defparam u.T$w` (illegal input accepted) · VCD `$scope` `[0]` spelling · `genblk<N>` label collision (split) · 🆕 L ⓦ residue (package constants outside the i64 interpreter) · §2 🆕 N residue: a labelled CONCURRENT `assert property` action block drops the label (1-oracle; the label dies in the parser, `Stmt::ConcurrentAssert` has no field for it ⇒ frozen-type change or a parser rewrap) | §4.5.432/436/437/440/444 residues | ② |

Do not start: rows 14/16/25/26/30 and 🆕 F (declared-width provenance / §11.8.1 region sign wall), row 34 (one oracle, zero demand), row 31 (pure half correct ⇒ performance), any widening of the wide fold's accept set before §11.8.1 region sign stands.

External reports (round-N) pre-empt the queue; reproduce every item at HEAD first. Oracle-split axes (§2 "Oracle splits") are never chased.

## 6. G2 — AI-Agent 친화 OBS 트랙 (SPEC=[preview/19](preview/19-ai-agent-observability.md))

완료: OBS-0 스펙 · OBS-1a run.json+results.jsonl(§4.5.73) · OBS-1b coverage.json(§4.5.99) · OBS-2 v1 trace.jsonl(§4.5.100) · OBS-3 stage.jsonl(§4.5.101) · OBS-S0 `--hier-tree`/`--inst-paths`.

teeth = 3-way 내부 차분(JSONL ≡ VCD ≡ `$display`) + 결정성 골든. 틀린 로그 = silent-wrong 과 동급.
값 인코딩: trace `old`/`new` = full-width 4-state binary · stage `vals[]` = `%0d` decimal(doc-19 §3 pin 4).

현재 위치: 다음 = 표 첫 행(OBS-2 잔여) · 트랙 전체는 §2·§3 뒤 3순위.

| 단계 | 산출물 | 공수 |
|---|---|---|
| OBS-2 잔여 | sva.jsonl(R-L6·SVA property명+support-cone v0) · per-element array probe · class/event probe(no-oracle) | M |
| OBS-1 잔여 | staged/vrun obs(.velab source-identity) · compile-fail manifest · `--seed` | S-M |
| R-L4 | 로그 채널 분리 | M |
| OBS-4 | `vrun --control stdio` JSON-RPC(peek/poke/step/run_until)+poke 저널 replay | L |
| OBS-5 | snapshot/restore/rewind(엔진 상태 postcard 직렬화) | L-XL |
| OBS-6 | X-origin·region-annotated events·정적 backward cone | L+ |

- 비목표: FSDB/UCDB·SQLite 내장·waveform GUI·UVM 연동. VCD는 사람용 유지.

## 7. 조건부 / 장기 (재진입 트리거 충족 시에만 승격 · 정확성과 직교)

| id | 항목 | 트리거 |
|---|---|---|
| BACKEND | ① 2-state 별도 모드 기각 ② PDES BSP 병렬(Amdahl 상한 T4≈2.5x) ③ native-eval 잔여 lane(signed>64·>128bit·sysfunc·real) ④ in-process JIT(cranelift-jit) 기각 | ② 지속 W≥64+grain≥200ns ③ 저ROI 상시 defer ④ 근거·재개 조건 = §5.a codegen 행 |
| VHDL | VHDL 프론트엔드(9-value std_logic 매핑·별도 파서·GHDL 오라클·E7xxx) | SV plateau + 값도메인 결정 + GHDL 셋업 |
| VCD-EXT | `$dumpports*`(포트 strength) | 파형 툴 수요 (FST=§4.5.149·150 지원 — `$dumpfile("x.fst")`/`-o x.fst`; known-edge=소형 타임테이블 fst-writer [issue #4] loud 거부) |
| MVP-CUT | string concat-nonassign · wildcard assoc `[*]` · package internal-import/scoped-call 잔여 · cross-frame disable | 개별 수요 시 |

## 8. 비계획 (영구 비목표 · gap 아님)

- DEFPARAM(IEEE deprecated·`#(.param())`로 충분) · IMPLICIT-NET(정책=E3010 명시 에러) · OOS(synthesis·waveform GUI·UPF/SDF/DPI-C·shortreal·trireg·UVM 생태계·unique/priority 다중-match 검사).

## 9. 완료 이력 포인터

- 완료 슬라이스 상세 로그(§4.5.x) = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존).
- Phase A~D 실행 기록(§5.1-x · ③층 native 백엔드 · 슬라이스 59건) = [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md)(무삭제·§번호 보존). 코드·커밋의 `ROADMAP §5.1-<x>` 는 거기서 찾는다.
- 구 §0~§7 원문 = [ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md)(§번호 보존).
- 탄 단위 내러티브·방법론 교훈 = [DEVLOG.md](DEVLOG.md)·ARCHIVE §3.
- 외부 호환성 리포트 1·2차 전말(A1~C1·EXT2 체인) = ARCHIVE §6·§6-2 — 잔여는 §3 "외부 리포트 잔여" 3건뿐.
