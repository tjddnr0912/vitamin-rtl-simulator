# ROADMAP ARCHIVE — 완료 항목 상세 보관 (vitamin)

> [ROADMAP.md](ROADMAP.md)에서 이관한 **완료 항목의 상세 로그 보관소**. §번호는 이관 당시 번호를 그대로 보존한다 — 다른 문서·커밋·테스트 주석의 "ROADMAP §4.5.x / §0~§7" 참조는 **이 파일**에서 찾는다.
>
> - **살아있는 전방 문서**: 잔여 과제 = [ROADMAP.md](ROADMAP.md) · 상위 스냅샷 = [REMAINING_WORK.md](REMAINING_WORK.md) · 실행 큐 = `LOOPROMPT.md` NEXT(로컬 dev-meta).
> - **이력 내러티브**(탄 단위) = [DEVLOG.md](DEVLOG.md). SPEC 정본 = `docs/preview/`.
> - ⚠️ **`ROADMAP §5.1-<x>` 참조는 이 파일이 아니라 [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md)** 에 있다(2026-08-18 이관 · ③층 Phase A~D 실행 기록 3,074 줄 · 무삭제·§번호 보존). 이 파일은 **§4.5.x 슬라이스**를 담는다.
> - **운용 규칙**: 신규 완료 슬라이스 로그는 아래 "완료 슬라이스 로그(이관 이후)" 섹션에 `#### 4.5.<N> <제목> (<날짜>, branch <slug>) ✅` 양식으로 **최신이 위**로 추가한다(기존 §4.5.x 양식 유지·기존 항목 삭제 금지).

## 인덱스 — 완료 슬라이스 355건 (최신순·⚠️ = 미머지 · 번호는 1~381 중 354개가 실재 — 결번은 병합·취소분)

> 본문은 `#### 4.5.<N>` 로 검색하면 바로 찾을 수 있다. ⚠️ = 미머지/보류.


**§4.5.220–280**
- `4.5.406` **A fill in a shift COUNT is one bit; the row-30 fill lane built · 3 rounds · REVERTED onto a §11.8.1 region-sign prerequisite** (2026-09-03 · 288/880 shift cells fixed · 5 new pre-existing §2 rows)
- `4.5.405` **A size cast's constant leaves, the width the operand is evaluated at, and the type an override gives an untyped parameter (§2 row 29 + the prerequisite §4.5.318 reverted on + §2 row 25)** (2026-09-02 · 6,452 green · format 29 unchanged · adversarial review: 2 lenses × 5 rounds then 8 lenses × 1 · corpus 10/10 digests unchanged): row 29 said the cast "fails to apply its own width down into a context-determined operand", no prerequisite, ~30–50 lines. Half true. `ast_ctx_signed` answered `None` for EVERY constant leaf (parameter, localparam, genvar, enum label, `pkg::`, >64-bit param), so `N'(op)` fell to the fill-only path and ran at self width: `parameter W32 = 32'hDEADBEEF; 64'(~(W32 + 32'd1))` printed `000000002152410f` for both oracles' `ffffffff2152410f`; `8'(-EA)` on a 4-bit enum label `07` for `f7`. HEAD: 114 of 720 cells (16 leaf kinds × 15 operators × N∈{64,128,8}), 0 with a net leaf, 0 splits. ⭐ The classifier now answers a bare name through the lowering's OWN decision: new `ident_route.rs` `bare_ident_route` (with-iterator → inline formal `subst_lookup` → task out-formal → innermost key over string/wide/real/params/symbols with the hoisted-block-local shadow test → `lookup_scoped`), which `lower_expr`'s `Ident` arm now matches on and only materializes — the sign of each route is the sign of the node that route builds (`expr_self_signed` of a substituted actual, `param_const_signed` of a constant), so classifier and lowering cannot disagree. ⚠️⚠️ **The other half of row 29 was written one bullet lower in the same §2, as the reason §4.5.318 built exactly this and reverted it**: with the classifier alone, `8'(13 + (PS8 >> 2))` (`PS8 = -8'sd16`) went from the fill-only `09` (= both oracles) to `49` — 6 of 720 revert-shape cells correct→silent — because `lower_size_ctx` recursed at N while §11.8.1 evaluates the operand at **max(N, its self width)** (the 32-bit literal makes the shift run at 32 bits). ⭐⭐ The NET spelling `8'(13 + (s8 >> 2))` had been `49` all along, so the width model was the defect, not the classifier, and the queue row and the older bullet were two views of one item — only the older one carried the prerequisite. Fix = `lower_size_ctx_entry` evaluates at `max(N, W)`, `W` from a new AST walk `size_ctx_self_width` over exactly the leaf kinds the classifier resolves, each width taken from the table the lowering materializes it from (`ir_bits_of`'s Signal rule incl. a real net's 64, `param_const_width` = the width half of `const_param_expr_w`, a wide const's own, literal, fill = 0, selects with foldable bounds, concat/replicate, a nested `Cast`'s target, `$signed/$unsigned`); a multi-packed-dim index and a select OF an element decline, and a declined walk recurses at N (every decline is an unsigned opaque leaf, and an unsigned expression's low N bits do not depend on the evaluation width — the first cut probed by lowering the operand plain and re-reported every diagnostic inside it, ENGINEERING_RULES §4.5.365). ⭐ The number has to be exact, not safe (an over-estimate changes what `>>` shifts in as surely as an under-estimate), so a dev hook `VITA_SCW_CHECK=<file>` makes every routed cast also probe and log agreement on `max(n, w)`; suite-wide and over three matrices the mismatch count is 0 and a test keeps it there. ⚠️⚠️ **Round 1: four BLOCKING findings, two mechanisms, both "the classifier resolved the name in a different order than the lowering"** — an inline function/task formal or local whose bare name is also a module constant answered the constant (`8'(pa >>> 4)` `0f` for `ff`; the classifier never asked `subst_lookup`), and an outer NET beat an inner generate-scope `localparam` (`lookup_net_scoped` was asked first) ⇒ the shared route above. ⚠️⚠️ **And one that exposed a producer**: `parameter P = 5` overridden with `32'hF0F0F0F0` — the classifier read the DEFAULT literal's sign and `64'(P >> 1)` became `7ffffffff8787878` where the fill-only path had been right by accident (`0000000078787878`); PRE was already wrong at the VALUE level (`P < 0` = 1, `64'(P)` sign-extended, `parameter Q = 8'sd1` + `32'hDEADBEEF` truncated to `ef` with `$bits` 8) = §2 row 25, whose diagnosis named the site. ⭐ **The producer fix was built and it fixed 7,982 of 122,774 override cells and serv's `|WITH_CSR`** (a `width` channel on `ResolvedOverride`; `param_decl_width_opt` declining the default's literal under an override; `param_range` from the override; `override_at_declared_width` resizing to the declared type) — ⚠️⚠️ **and three consecutive review rounds each found a NEW correct→silent edge in it** (round 2: `defparam` and fills lost the default's type with nothing replacing it, and the unrestricted fallback `param_decl_width` passes `default_binds = false` too; round 3: a `defparam` with a NAME rhs, a `time` parameter with a decimal default forwarded (`const_self_width` guesses 32 for a meta-less name), `$signed(64'h…)` typed 32 and resized down before being read wide). **Three rounds of blockers on one axis = revert** (ENGINEERING_RULES): the patch and its five oracle-pinned tests are kept (`scratchpad/r29/row25/`), the three edges are written into §2 row 25, and the size-cast slice ships with the classifier DECLINING every guessed type — `param_type_guessed` marks an overridden untyped parameter and every body constant whose initializer reads one, and `bare_ident_route` reports it — so those casts stay on the pre-slice path, right or wrong as they were (probe2: 12 override tags fixed, 5 pre-existing, 0 regressions). Round 3 also surfaced a class round 1 had measured and never reported: **a signed element of a 2-D unpacked / dynamic / queue array beside a constant leaf** was classified UNSIGNED (`Some(false)`) and, once the constant resolved, extended as unsigned (`64'(PS16 / g2[0][1])` `0000000000000df4` for the oracles' `ffffffffffffffbc`, 669 cells) ⇒ a bit-select's sign is `None` unless it selects a bit of a plain packed net / a constant or an element of a 1-D static array. And the verbatim-actual rule gained its second producer: a body local `y = x` where `x` is a hierarchical or class-field actual re-wraps the recorded node into a new one (`resize_inline_assign` over an untrusted width) ⇒ recorded too. ⚠️ **Round 4 (on the decline design) found three more gaps in the decline itself**: a header-list sibling derived from a guessed parameter (`#(parameter P = 5, parameter Q = P + 1)`) was not marked — only the overridden declaration and the body binders were — so `64'(Q >> 1)` read the guessed sign (14 cells) ⇒ `bind_one_param` also marks an untyped sibling whose initializer reads a guessed name, and typed derived constants are never marked (a declared type is a fact both oracles honour); a frame-local / formal unpacked array is an md-packed slot (`array_len` 1, element sign in `frame_arr_formal_meta`) that the element rule read as a packed vector (`64'(PS16 / la[0])` `10b` for `4d`, 8 cells); and the round-4 element rule itself had sent a BIT of an element (`a1[0][3]`) to `None` — the fill-only path evaluates a 1-bit leaf at 1 bit (`8'(-a1[0][3])` `ff` → `01`, 290 cells) ⇒ one `select_chain` rule for both walks: `k` selects on a name with `d` unpacked dimensions is a BIT (`k > d`, 1-bit unsigned whatever the base), an ELEMENT (`k == d`: a static array's or a frame array's element sign and width; a dynamic / queue handle's declines) or not a value; an unresolvable base keeps the pre-slice `false`. Round 5 found the last door of the same rule: a frame array's dimension count is `ArrayFormal::dims.len()`, not 1 (a 2-D frame local's element read as a bit, `64'(PS16 / m[0][0])` `10b` for `4d`) ⇒ the frame arm uses the lowering's own count, the predicate `lower_packed_read` re-stamps the element's sign on. Round 5's differential added two rules of the same kind: a `time` constant derived from a guessed parameter is a guess (the meta producer has no `time` arm), and an operand that reads a HIERARCHICAL or class-member name keeps the pre-slice decision entirely (nets answer, constants and formals decline, `&&` short-circuits) — such a read is a placeholder with no width at classification time, and both moves away from PRE regressed cells (`16'(PS16 * u.a1[2])` `xxxx` when its constant resolved; `8'(-u.v[3])` `01` when the leaf declined); final re-grade of every review matrix (sc1 13,440 · el5 2,115 · el1 8,030 · dc1 4,531 · gm/gmi 5,616 · the 122,774-cell override set): **0 regressions**. ⚠️⚠️ **Round 6 (eight lenses) found the two spellings the guard and the chain still missed, and both are the same lesson twice**: ⓐ the `Concat`/`Replicate` sign arms answer `Some(false)` WITHOUT descending, so an OPAQUE leaf in braces walked past the hierarchical guard three ways — the callee's own NAME is a path (`u.hf(x)`: the args were walked, the name was not), an inline formal is a bare name bound to a hierarchical ACTUAL (no hierarchical spelling in the operand at all), and the WIDTH walk sized a select from its own `[msb+:w]` without looking at the base (`4'(s8 - u.v[0+:4])` measured 8 and evaluated wider than the pre-slice `n`) — each printed `x` where PRE and both oracles agree ⇒ ONE `has_opaque_leaf` gate now decides BOTH walks, and a kind the walk does not model is opaque rather than a guess ⓑ `select_chain` resolved its BASE with `lookup_net_scoped` + the i64 constant domain instead of `bare_ident_route`, so a bit of a parameter wider than i64, of its package twin, or of a substituted formal declined and the fill-only path evaluated a 1-bit leaf at 1 bit (108 cells where PRE routed and was right) ⇒ the base takes the same route as everything else. ⭐ **A tried-and-measured alternative is recorded because it FAILED**: requiring a measured width to route at all ("route only what the walk can measure") is stricter and simpler, and it regressed 458 cells across four matrices — PRE routed operands whose width this walk cannot measure, and un-routing them is a change away from PRE, not toward it. Round-4 differential re-graded lens A2's 122,774-cell set on the decline tree: 0 regressions; the round-4 fixes re-measured on its 8,030-cell element matrix: 0 regressions, 1,145 fixed. Round-3 differential re-graded lens A2's 122,774-cell set on the final tree: no-cast designs byte-identical PRE vs POST, diagnostics counts identical, every round-1/2/3 repro at oracle parity where the fix reaches and at PRE parity where it declines. Results: probe1 114 → 16 (all 16 = the function-call leaf, PRE == POST, its own slice), probe2 (override channels · genvar · generate localparam · 96-bit · `pkg::` · `NAME'(`) 23 → 0, probe3 (the §4.5.318 revert shapes) 50 → 0, round-1 lens A 27,809 cells: 832 FIXED and every regression class closed in round 2 · corpus 10/10 with every digest unchanged · new tests `size_cast_const_leaf.rs` (14), all oracle-pinned; the reverted producer's five tests are kept with its patch.
- `4.5.404` **스킵을 허락하는 것은 순수성이 아니라 의존 집합의 완전성이다 (§3 ⑧ · 코퍼스 10/10)** (2026-09-02 · **6,438 green** · format 29 불변 · 적대 2렌즈): 큐엔 §3 ⑧ 한 줄(*"함수 본문의 `$finish` 가 verilog-ethernet 을 막는다"*)이었고 그건 **절반**이었다 — `$finish` 를 손으로 지우면 **elaborate 는 1.03 s 에 통과하는데 시뮬이 ~38시간**(iverilog 7.62 s). ⭐⭐ **나머지 절반은 이 워크로드의 줄에도, 어느 줄에도 없었다**: `levelize::expr_is_pure_of_nets` 가 **모든 `Expr::Call` 을 `false`** 로 답해 `assign y = f(…)` 가 **영원히 매 패스** 재평가된다 — `lfsr.v` 의 `wire [39:0] mask = lfsr_mask(n)` **80줄**이 `--obs-procs-time` 상 **런타임의 99.99%**(각 240회). 3줄로 줄이면 `wire cc = 8'd3+8'd4` **1회** / `assign a = sm(8'd3)`(자명한 함수·상수 인자) **90회** = **vita 일반 동작**. ⭐ **고침의 기준은 그 술어의 주석이 함의하는 것이 아니다** — *"함수는 넷이 기록하지 않는 상태를 읽을 수 있다"* 는 참이고 **틀린 기준**이다. 두 오라클은 연속대입을 **감도 목록의 넷이 움직일 때 정확히** 재평가하므로 필요한 것은 순수성이 아니라 **의존 집합의 완전성** ⇒ 신규 `levelize::func_read_deps` 가 콜리가 (중첩 호출까지) 읽는 모듈 넷을 모으고 귀속 못 하는 건 전부 거절(positive allow-list). ⭐ **모양을 정한 두 측정, 방향이 반대**: ⓐ 인자 없이 모듈 넷을 읽는 함수는 **오라클 분열**(iverilog 는 t0 값으로 얼리고 verilator 는 추적 · vita 는 이미 verilator 편) ⇒ 안 모으면 **조용히 iverilog 편으로 옮겨간다** ⓑ 정적 지역을 이월하는 함수는 **위험해 보이지만 아니다**(iverilog 도 같은 상태를 같은 트리거 규칙으로 나르고 오늘 셋이 일치) = `ca_always` 의 매-패스가 이상치. ⭐ ⓑ가 일반화돼 두 절반이 만난다: **의존 없는** 본문의 `$display` 가 5사이클 **30회 → 1회**(두 오라클) · `$error` 도 30 → 1 · ⚠️ 의존이 있으면 **의존 변화당 1회**(리뷰 실측 26 → 3)이고 그 수는 **어느 오라클도 중재 못 한다**(iverilog 는 감도 목록이 비었고 verilator 는 첫 `$error` 에서 abort) ⇒ **효과의 옳은 발생 횟수 = 오라클의 평가 횟수**이고, 그래서 시스템 태스크를 인증에 넣는 것이 방종이 아니라 같은 정리다 — 그리고 그것이 **`$finish` 를 그 인증에서 뺄 수 없었던 이유**다(`lfsr_mask` 안에 `$error` 가 있어 태스크 가족을 통째로 빼면 설계가 정확하고 80배 느리다). **§3 ⑧ 절반** = 모양은 `classify_frame_body` 가 받고 **도달한** 것만 `frame_end_is_loud` 가 **위치 단 fatal**(F4004 · file:line + 인스턴스)로 끝낸다 ⇒ §4.5.372 의 ⓐ(*멈춘 본문의 반환값* · 세 툴이 다 다르다)에 **답할 필요가 없다**. 라우팅 셋 전부 가르쳤고(elaborate classify · `frame_eval`+`task_frames` 실행기 · `native/frames` 승인 walk) ⚠️ **태스크는 재라우팅 안 됐다**(전 스위트 계측 · `$finish` 는 여전히 깨끗이 종료 = correct→loud 였을 자리). ⚠️⚠️ **적대 리뷰 BLOCKING 하나, 그리고 그게 내 docstring 의 *"측정으로 처분했다"* 를 반증했다** — 정적 지역에 **카운터**를 이월하는 함수는 값이 *"몇 번 돌았나"* 의 함수이고 **몇 번 도는지가 바로 이 인증이 바꾸는 것**: 의존이 살아 있으면 PRE `3 3 3`(**verilator 정확히**) / 무게이트 POST `2 3 3`(**아무와도 불일치**) / iverilog `1 2 3`, 비포화 쌍둥이는 PRE **F4016 exit 1** → POST **조용히 `2 3 4`** ⇒ correct→silent-wrong · loud→silent-wrong. ⭐ 고침 = **definite-assignment 게이트**인데 ⚠️ **그것만으로는 헤드라인이 죽는다**(`lfsr_mask` 는 마스크 배열을 `for` 로 지우고 **0회 돌 수도 있는 루프는 definite assignment 가 아니다**) ⇒ ⭐⭐ **선언지**: *definite-assignment* **또는** *의존 집합이 비었다*(비면 settle 시드에서 **딱 한 번** 평가 = 두 오라클이 빈 감도 목록에 하는 그것). 같은 카운터에서 의존만 없애면 PRE **F4016** → POST **`1 1` = iverilog 정확히** · 잔여는 규칙이 아니라 **평가 한 번**(vita t0 추가 settle). ⚠️ 리뷰가 **거짓 판정한 내 문구 셋** 전부 수정(진단문의 *"ends the run here"* 는 본문이 **계속 실행**되는 사실과 어긋남 · *"verilator 편을 지킨다"* 는 **재귀 콜리에서 무너진다** · `Fdisplay|Fwrite => true` 는 도달 불가인데 이웃 `Fclose => false` 와 모순 ⇒ **거절로**) · differential 렌즈는 **~1,475 설계 / ~7,900 호출에 BLOCKING 0**. ⚠️⚠️ **설계가 바뀌었으므로 재리뷰했고, 라운드 2 가 BLOCKING 을 셋 더 냈다 — 셋 다 라운드 1 의 고침이 만든 것이고, 셋 다 같은 카운터를 다른 문으로 들여보낸다**: ⓐ **`release` 가 전제를 깬다** — *"의존이 없으면 딱 한 번 평가"* 가 거짓이다(`k_release` 가 `redirty_drivers_of` 를 **무조건** 부른다 · 대응하는 `force` 가 없는 `release` 도 두 오라클엔 no-op) ⇒ `1 + release 수` 번 평가 ⇒ PRE `3 3 3`(verilator) → `1 2 3`(아무도 아님) · 비포화 쌍둥이는 **F4016 → 조용한 답** · ⭐ **census 가 고침을 정밀하게 만들었다**: `ca_dirty_flag` 생산자는 **정확히 셋**(시드 · `note_change` · `redirty_drivers_of`)이므로 **force/release 타깃만 이름 부르면** 의존 변화가 아닌 평가를 **전부** 이름 부른다 ⓑ **인자 0개 함수의 반환 슬롯이 formal 로 세어졌다** — 경계 검사를 `insert` **뒤**에 둬서 `n_params==0` 에 하나가 들어갔다(라운드 1 BLOCKING 이 **자기 repro 에 토큰 하나 바꿔** 부활) ⓒ **부분 쓰기가 넷 전체의 definite assignment 를 세웠다** — 내 docstring 은 이걸 *"array-word 부정확 · 코퍼스도 프로브도 낸 적 없는 모양"* 이라 적었는데 **양쪽 다 반박**됐다(배열 없는 packed part-select 로 카운터 세탁 · 프로브 둘이 냈고 둘 다 loud→silent-wrong) ⇒ 셋 다 PRE 와 **정확히 일치**로 복귀했고 헤드라인은 그대로. **verilog-ethernet 이 무수정으로 2.24 s** 에 두 오라클 다이제스트(`ca4945d0044f74d8`) · iverilog 7.72 s = **3.45×** · **코퍼스 9/10 → 10/10 · 거절 0** · 기하평균 **1.74×**(아홉) **1.93×**(서드파티 일곱) **2.15×**(serv 뺀 여섯) · examples 4/4 바이트 동일 · 다이제스트 전부 불변 · 신규 §2 표 **31**(본문의 **시스템 함수**는 여전히 거절 — `$random`/`$time` 이 매 패스 다시 뽑힌다 · 두 오라클은 얼린다 · **씨앗은 넷이 아니다**) **32**(도달한 `$finish` 다음 문장이 실행된다 — `$fatal` 과 **같은 동작**이라 pre-existing).
- `4.5.403` **선언 폭을 fold 의 입력으로: 거절 하나를 없애면 대신 답하는 놈이 생긴다 (§2 표 21)** (2026-09-01 · **6,418 green** · format 29 불변 · 적대 2렌즈 · BLOCKING 3 · 312칸 **159 FIXED · 0 REGRESSION**): wide 상수 fold 가 모든 노드를 **제 폭**에서 접고 나중에 resize 했는데 §11.6.1 상 문맥결정 연산자는 **주변 대입에서 폭을 받는다** — `localparam logic [127:0] C = ~32'd0;` 이 `00…00ffffffffffffffff`(두 오라클 128 ones · exit 0) · `3 ** 41` 은 아예 거절. `fold_self_bits` → `fold_bits_at(e, ctx, name)` 로 쪼개 문맥결정 arm 은 `max(피연산자 폭, ctx)` 에서 계산하고 ctx 를 물려주며, LRM 이 파 놓은 **두 자리**(시프트 오른쪽 §11.4.10 · `**` 지수 Table 11-21)는 자기결정 유지 = **표 27 과 같은 규칙** · size cast 는 **자기 문맥**이라 ctx 가 거기서 멈춘다. ⭐⭐ **핀 넷이 loud→correct 로 바뀌었고 넷 다 자기 docstring 에 이 선행조건을 적어 뒀다**(*"until the walk can carry a context width"* 등) ⇒ `const_pow_exponent_domain.rs` 엔 이제 `loud` 헬퍼 자체가 없다. ⚠️⚠️ **BLOCKING 셋 전부 내 것, 둘은 두 렌즈가 독립적으로**: ⓐ **여는 괄호 하나가 슬라이스 전체를 우회** — `wide_param_const_in_scope` 가 `Paren` 을 최상위에서 매치해 ctx 0 으로 접는 `fold_init` 로 보냈고, PRE 는 backstop 이 거절해 i64 walk 가 맞게 답했는데 **backstop 에 ctx 를 넣자 거절이 안 나가** 낡은 좁은 경로가 이겼다(`(8'hFF+8'hFF)` `…01fe`→`…00fe` · 슬라이스 자신의 headline 셀이 괄호를 통하면 **더 나빠졌다**) ⇒ 괄호를 먼저 벗긴다 ⓑ `widen_to`/`bp_operands_at` 이 피연산자를 **제 부호**로 확장했는데 §11.8.2 는 **식의 부호를 먼저** 정하고 피연산자를 거기 맞춘다 — ⭐ **vita 런타임은 이미 옳게 하고 있었다** (상수 도메인이 런타임과 모순) ⓒ x/z 를 확장하니 **unknown MSB 를 복제**해 loud 거절이 120비트 조용한 `x` 가 됐다 ⇒ 거절 유지. ⭐ 리뷰가 내 docstring 의 *"분류는 여기 안 적었다"* 도 **반박** — arm 이 손으로 매치하므로 이건 canonical 리스트의 **여섯 번째 사본**이고 드리프트가 이미 있었다(`BitXnor` 누락 ⇒ `~^` 가 PRE·POST 둘 다 틀림) ⇒ 추가하고 문구를 사실로 · ⚠️ `resize_bits` 에 항등 early-return(없으면 MAX_NET_WIDTH 300단 `+` 체인이 elaborate 3.0 s) · ⚠️ **override 채널은 의도적으로 안 건드렸다**(표 16 대부분이 표 17 오라클 분열이고 verilator 가 vita 편 · 다섯 철자에서 불변 실측) · 신규 §2 표 **28**(>64비트 파라미터 계층 읽기 불가 — silent→loud 상승이지만 합법 설계에 E3010) **29**(⭐ **런타임이 size cast 를 문맥이 통과하게 둔다** — 상수 도메인이 먼저 맞아져서 드러났다) **30**(unsized fill 이 wide fold 를 거절 — 이제 ctx 가 있으니 작은 arm).
- `4.5.402` 큐 여섯 줄을 HEAD 에서 재측정했더니 여섯 줄 다 모양이 바뀌었다** (2026-09-01 · **제품 코드 변경 0** · 계측·재그라운딩만 · 6-agent census): 다음 착수를 정하려고 열린 후보 전부를 3-툴로 재현 — **여섯이 네 방향으로 틀렸다**. ⭐ **표 27**(시프트 카운트)이 1순위로 확정: *"상수 함수 코너"* 가 아니라 **함수도 이름도 없이 모듈 스코프에서 도달**한다 — `logic [(16'h0100 >> 3'sb101)-1:0] bus` 가 `$bits` **2**(오라클 8) = 조용히 틀린 버스 폭이고 `generate if` 는 **반대 가지**를 골라 본문을 지운다 · 158칸 중 **66 silent-wrong**(+ 32칸은 latent) · ⚠️ 기록된 원인 **반박**(`eval_const_assign` 가 아니라 `eval_const_env_self` 가 제 부호를 준다) · *"이미 정확"* 주장 둘도 반박(`C[1:0]`·`$unsigned(C)` 는 **loud** · **부호 있는** 리터럴 카운트는 틀린다) · ⭐⭐ **정답 구현이 트리 안에 있다**(`const_wide.rs::fold_shift_count` 가 §11.4.10 을 인용) · 고침은 공유 함수(17 콜러)가 아니라 **호출부 한 곳** · 선행조건 0 · **66칸 전부 vita 자기모순**(런타임 ✓ · wide 레인 ✓ · i64 레인 ✗). ⭐ **표 15** 는 기록된 헤드라인(narrow plane 인프라 · 40자리)이 **착수 금지**이고 그 밑의 **10칸 silent-wrong**(override 가 x/z 평면을 잃는다 · `8'b1010_010x`→`10100100` · exit 0)이 **조건문 하나·콜러 하나**, 12줄 위 형제 arm 의 branch-parity 갭, format 29 무영향(확인) ⇒ 그 절반만 착수. ⭐ **표 21** 은 loud 가 아니라 **2-오라클 silent-wrong 48/108**(`~32'd0` 이 128비트 타깃에서 조용히 틀린다) · 자리는 `fold_self_bits` 의 문맥 폭 인자 하나이고 설치 배관(`wide_disagreeing_value`)은 이미 있다 · **선행조건 = 표 27 을 한 번만 적어 두 walk 가 소비**(미러 드리프트 방지) · ⚠️ **표 16 은 대부분 표 17 의 오라클 분열로 흡수**(verilator 가 4칸에서 **vita 편**) · ⚠️ 표 14 는 **선행조건이 아니다**(리터럴만으로 재현). ⭐ **표 22** 는 3 철자가 아니라 **126칸**이고 §2 가 아니라 **§3**(틀린 답이 없다 · 12×12 매트릭스에서 두 오라클 132/132 거부 · 회귀 노출 **0** 실측) · ⚠️ 승자가 **소비자마다 다르다**(enum 라벨이 `localparam` 을 이기고 `typedef`×net 이 값 4비트 / `$bits` 8). ⭐⭐ **표 24 는 기록보다 훨씬 나쁘다** — *"쓰기가 x 로 착지"* 가 아니라 **clocking OUTPUT 으로 선언하는 것만으로 신호가 모듈 경계를 넘어 영구 파괴**된다(쓰기 없이 · exit 0 · verilator 오라클) ⇒ 24a(clobber · 착수 가능 · `clocking_commit_plan` 의 무조건 OUTPUT 단계)와 24b(한 사이클 lag · **앵커 없음**)로 분리 · 표 23 의 충돌 절반은 **합법 설계 오거부**(vita 가 제 합성 이름끼리 충돌) ⇒ §3. ⭐⭐ **§3 ⑧ 은 닫아도 코퍼스가 안 열린다** — `$finish` 하나 지우면 elaborate 는 **1.03 s 에 통째로 통과**(두 번째 갭 없음)하는데 시뮬이 **~38.5시간 대 iverilog 7.62 s(≈18,000×)** 이고 `--obs-procs-time` 이 **런타임 100.0%** 를 `wire = lfsr_mask(n)` **80줄**(상수 인자인데 각 **76회** 재평가)에 놓는다 · 3칸으로 줄이면 **vita 일반 동작**(`Expr::Call` 에 닿는 연속대입은 매 패스 재평가 · `levelize.rs::expr_is_pure_of_nets:329` → `ca_always`) ⇒ **성능 고침 뒤에 ⑧**, 그러면 10/10 · ⚠️ 거절 자체도 둘이 아니라 **일곱 철자** · ⭐ 싼 대안 기록(정적 walk 가 아니라 **도달한** `$finish` 를 런타임 loud 로 = ⓐ 에 답할 필요 없음).
- `4.5.401` 고침은 맞았고, 그것이 부르는 walk 가 아니었다 (§2 표 14: 지어서·재서·되돌렸다)** (2026-09-01 · **6,415 green** · format 29 불변 · **제품 코드 변경 0** · 적대 2렌즈 **3라운드 · BLOCKING 7**): `localparam logic signed [7:0] NM=-8'sd2; localparam logic [63:0] X = NM ^ 64'h0;` 이 vita `fffffffffffffffe` / 두 오라클 `00000000000000fe` — §11.8.2 는 unsigned 식의 signed 피연산자를 **제 폭에서** 변환하라 하고 vita 엔 그 변환(`leaf_into_ctx`)이 **이미 있다**. ⭐⭐ **vita 가 자기모순**: 같은 식을 **함수 지역변수**로 쓰면 `00…fe` — 지역변수의 선언 폭은 `envw` 에 있고 모듈 스코프 초기화식은 **폭-무제한** `const_eval_in_scope` 로 접힌다. ⭐ **지은 것** = 선언 폭이 있는 타깃을 `eval_const_assign`(상수함수 인터프리터가 이미 쓰는 폭-인식 진입점)로 라우팅 — **된다**: 44칸 census **27 → 3** · §4.5.366 의 module-scope 4연산자 잔여 해소 · 핀돼 있던 `>>>` 자기모순(상수 18446744073709551615 / 런타임 4294967295) 소멸. ⚠️⚠️ **되돌린 이유 = 3라운드 BLOCKING 7**: 라운드 1~2 다섯은 내 게이트였고 고쳤다(**모든 LEAF 의 provenance** 요구 — `param_meta` 는 무타입 파라미터에 **기본값**이고 `time` 엔 **부재** · **i64 레인 위 거절**(아니면 정답의 절단 = silent↔silent 맞바꿈) · **fill 피연산자 거절**(`'1` 오사이징을 옛 overflow decline 이 붙들고 있었다) · 스코프 walk **3치화** (`remove` 가 *"이 스코프는 이 이름을 안 묶었다"* 와 구분 불가 ⇒ 안쪽 `localparam time NM` 이 바깥 `logic [7:0] NM` 의 폭을 물려받아 `NM|64'h0` 이 **44**, 세 툴 300) · **패키지 바인더는 라우팅 금지**(표 26)). ⚠️⚠️ **라운드 3 의 둘은 공유 walk 안이고 라운드 1부터 있었다**: ⓐ §11.4.10 상 시프트의 **오른쪽 피연산자는 자기결정·unsigned** 인데 walk 가 문맥을 **모든 leaf 에** 밀어 넣어 좁은 signed 카운트가 부호확장 ⇒ `16'hFF01 << 3'b101` 이 **0**(30 correct→wrong + 36 loud→wrong) — ⭐ **라우팅 없이 상수 FUNCTION 으로 오늘 재현된다** ⇒ **표 27 신설, 선행조건** · ⓑ i64 레인 경계가 **타깃에만** 있어 `logic signed [64:0]` **leaf** 가 64비트 피연산자로 평가돼 `/`·`%` 가 틀림(83칸) · ⇒ **선행조건 = 폭-인식 walk 자체가 옳을 것**(상수함수 인터프리터와 공유 = 다른 blast radius) ⇒ 되돌리고 진단·선행조건 둘·*움직여야 할 칸과 움직이면 안 되는 칸*을 테스트로 핀 · ⭐ 곁수확 = 표 **26**(패키지 값이 canonical 해지면 `pk::X` 소비자는 여전히 무제한으로 접어 **1,233 correct→silent-wrong vs 714 fixed**) · 리뷰 differential 총 **~48,000칸** · examples 4/4 · 코퍼스 10/10 불변.
- `4.5.400` 두 번 하는 것은 두 번째가 첫 번째와 같다는 증명이 있을 때만 안전하다** (2026-09-01 · **6,415 green** · format 29 불변 · **코퍼스 8/10 → 9/10**): §2 표 **8b · 11** 을 사용자 지시 순서대로. ⭐ **11**: enum 라벨은 const 선언이므로 **decl-order** 로 바인딩한다 — 큐 줄은 `localparam Q = EA;` 하나였는데 census 가 넓혔다(param→label→param **체인** `A=3 L=4 B=5` 가 `B` 에서 loud · 나중 선언 **폭** · `generate if` · 다음 typedef 의 라벨). 전방 참조(iverilog 거부/verilator 5)는 **뒤 패스가 그대로** 접으므로 오라클 분열 축은 안 움직인다. ⚠️⚠️ **BLOCKING 셋, 전부 내 것이고 한 모양** = *두 패스 사이에서 접힌 소비자는 1번 답을, 그 뒤는 2번 답을 갖는다 ⇒ 한 라벨 두 값, exit 0*: ⓐ 못 접는 라벨을 `continue` 로 건너뛰며 **auto-increment 카운터도** 건너뛰어 `{A=LP, B}` 가 `Q=B=0` 인데 `B` 는 6 ⓑ base 폭이 아직 사실이 아니라 **마스크가 안 돌아** `enum logic [W-1:0] {A=-8'sd2}` 가 `fffe`(2번 패스는 254) ⓒ 라벨 값 안의 이름이 1번은 **wildcard import**, 2번은 body `localparam` 으로 풀린다. ⓐⓑ 는 게이트로(라벨 하나가 아니라 **typedef 통째로** 거절 — 카운터와 폭을 공유하므로), ⓒ 는 **게이트로 볼 수 없다**(fold 가 성공한다, 다른 답으로) ⇒ `enum_label_prepass` 로 **2번 패스가 검증**하고 어긋나면 loud · ⭐ **8b**: read-only 퍼널 하나로 — ⚠️ 리뷰가 census 를 **여섯이 아니라 아홉**으로 고쳤다(`$cast` 목적지 — `$sformat` 검사가 *"mirrors"* 라고 인용한 바로 그 가드 — 와 엔진이 **전진시키는** `$random`/`$dist_*` **시드**) · ⚠️⚠️ **키워드 하나가 고침 전체를 껐다**: `automatic` 태스크 안이나 계층 철자의 목적지는 이름이 존재하기 전에 lower 되어 퍼널이 `POISON_NET` 을 받고 **아무 답도 안 한다** — `$readmem*` 만 loud 였던 건 무관한 이유의 사이드맵 덕이었다 ⇒ `hier_write_args` + `deny_readonly_write_at` 로 **resolve 시점에 다시 묻는다**(15 철자 전부 loud · verilator 15/15 거부) · ⭐ 곁수확 = `verilog-axi` 가 **돌기 시작했고** 다이제스트 차이가 §2-N 이 이미 판정한 t0 이벤트 순서 29 x-사이클뿐 ⇒ corpus-runner 에 **`Expect::Split`**(양쪽 다이제스트를 핀 — vita 가 움직이면 여전히 REGRESSION, 둘이 다시 일치하면 PROMOTED) · examples 4/4 바이트 동일 · 신규 §2 표 **22**(모듈 스코프 이름 중복 검사 부재 — 두 오라클 거부, vita 실행) **23**(진단이 인스턴스 접두사를 잃는다) **24**(clocking OUTPUT 쓰기가 x).
- `4.5.399` 남은 후보 전부를 재측정했고, census 가 코드 없이 세 줄을 닫았다** (2026-09-01 · **6,394 green** · format 29 불변 · 적대 2렌즈 + 10 검증자): §2 표 **7·3b·8·10·11·15~21** + §3 ⑧ + 신규 세 줄을 순차 착수 — ⭐ **먼저 전부 HEAD 에서 3-오라클 재현**했고 그게 무엇을 실을지 정했다. **실은 것 둘**: **18**(`defparam` 이 override 식의 **부호**를 안 날라 음수가 bit 63 에서 멈췄다 — `-32'sd7` → `0000…fff9`, 두 오라클 all-ones) · **8**(clocking INPUT 이 `$readmem*` 인자로 **쓰기 가능**했다 — lvalue 퍼널은 loud 인데 인자 퍼널은 홀딩 넷을 exit 0 으로 썼다). ⭐⭐ **코드 없이 닫은 셋**: `buf` 줄은 **반박**됐다 — `~(~i)` 는 rename 을 숨긴 게 아니라 **IEEE 1364 §7.3 의 z→x 강제 그 자체**(vita x / iverilog z · 옆의 `assign` 은 둘 다 z · ⭐ iverilog 는 자기 `and` 게이트에서 같은 z 를 x 로 만든다 = 자기모순) ⇒ ruling 핀 · **FST** 줄은 진단이 틀렸다(스냅샷이 첫 `#t` 앞에 오는 건 맞지만 **시간 0 을 열어도 값이 안 들어온다** — 지어서·재서·**되돌렸다**; 진짜 자리는 fst-writer 의 per-var 초기값 API) · **표 10** 은 넷 중 **둘이 이미 닫혀** bare-import 철자 하나만 남았다. ⚠️⚠️ **BLOCKING 둘, 둘 다 내 수정** — ⓐ clocking 게이트를 **모든 인자**에 걸어 `$readmemh(cb.fn, mem)`(파일 이름 = 순수 읽기)을 **correct→loud** 로 거절(§21.4 는 `(file, mem, start, end)` · 고침은 `argi == 1` 한 토큰이고 **의도는 doc 에 두 번 적혀 있었다**) ⓑ `const_signed_env` 의 이름 arm 이 `param_meta` 를 읽는데 그 `signed` 는 **기본값이지 사실이 아니다** — §6.20.2 상 무타입 파라미터의 타입은 **최종 override** 가 정하므로 `defparam u.K = PW;` 가 **correct→silent-wrong** ⇒ **부호가 구문에서 evident 할 때만** 기록(리터럴과 그 위 연산자 트리) · ⚠️ 표 8 은 **줄이 지목한 철자를 닫았지 클래스를 닫지 않았다** — `$fgets`·`$fread`·`$fscanf`·`$sscanf`·`$value$plusargs`·`$swrite` 여섯과 `$readmem` 의 **계층 철자**가 여전히 홀딩 넷을 쓴다 ⇒ §2 표 8b 신설 · ⚠️ **표 21·16 은 표 14 와 같은 선행조건 하나**(대상 선언 폭에서 접기)라 세 줄이 아니라 **한 인프라 항목** · 표 7 은 2-오라클 silent-wrong 이지만 **코퍼스 수요 0 이고 format 30 을 요구**한다.
- `4.5.398` Four queue rows, and four of the five blocking defects in the fixes were one shape** (2026-09-01 · **6,387 green** · format 29 불변 · 적대 2렌즈 + 14 검증자): §2 표 **13 · 6 · 4 · 12** 을 큐 순서대로. ⭐ **착수 전 재측정이 둘의 모양을 바꿨다** — 표 14 는 fold 의 부호 규칙이 **맞고**(함수 지역변수면 정확) 빠진 게 **이름의 선언 폭 provenance** 라 **네 번째 벽 도착**으로 착수 금지 · 표 12 의 기록된 증상(*"한 라벨 두 답"*)은 §4.5.384 가 이미 닫아 **사다리 한 칸 위**였다. ⭐ **13**: `|| v < 0`(§4.5.154)를 **세 바인더 전부**에서 제거 + 값을 **선언 폭에서 canonical** 하게 저장 — 64칸 census 가 틀린 칸만 움직였고 리뷰가 **주장 안 한 세 개**(net 폭 4→254 · `$size` 3→255 · `generate if` 가지)를 더 쟀다 · **6**: 인라인 fold 가 LHS **철자만** 보고 "반환값이냐 지역변수냐" 를 정해 **모듈 net 쓰기가 사라졌다**(`automatic` 쌍둥이는 원래 loud) ⇒ 소유 이름 집합으로 거절(사다리 상승 · 수행은 hoist 기구라 §2 잔여) · **4**: `$itor(3.9)` 가 IEEE-754 비트(4.6e18) ⇒ **평가기 한 곳**에서 값으로 반올림 · **12**: 파서 §6.19 검사가 파라미터 base 에서 fail-open ⇒ elaborate 쌍둥이. ⚠️⚠️ **적대 리뷰 BLOCKING 다섯, 전부 내 수정의 산물이고 넷이 같은 모양** = *다른 PHASE 에서 빌려온 술어는 거울이고 거울은 닿는 순간 어긋난다*: ⓐ `1i128 << w` 가 **128 에서 랩**(합법 128비트 base 거절 · debug 는 w≥127 에서 **패닉**) ⓑ 파서 fold 를 손으로 미러링한 `is_literal` 이 즉시 드리프트해 `[(3):0]`·`[8'd7:0]` 이 **양쪽 검사 모두** 빠짐(⇒ 미러 **삭제** — 파서 에러는 파이프라인을 멈추므로 스킵이 애초에 불필요) ⓒ *"see instance.rs"* 라 적힌 **세 쌍둥이 중 하나가 거짓**(package 가 auto-increment 를 **마스크 전** 값에서) ⓓ 마스크가 package 내부 concat 을 loud→silent-wrong ⇒ 뿌리는 **패키지가 파라미터 폭만 자기 본문에 살려 두고 라벨 폭은 안 살렸다** ⓔ `$itor` 게이트가 AST 술어라 **프레임 함수 real 반환에 눈이 멀었다**(+ 2⁶³ 넘어 부호 뒤집힘 + `--obs-procs` 빌트인 census 부풀림) ⇒ **값으로 묻는 한 arm** 이 셋을 동시에 해결 · ⚠️ 옛 동작을 박제한 핀 하나 재조준(*"never over-reject on unknown width"* — 두 오라클이 그 소스를 거부한다) · examples 4/4 · 코퍼스 다이제스트 전부 불변.
- `4.5.397` A port connection invented a transition, and the fix had to be a suppression, not a mirror** (2026-09-01 · **6,376 green** · format 29 unchanged · adversarial 2-round review): ROADMAP §2-N's remaining half. `assign n = m;` between two whole nets of equal width — a port connection, a bus bit, half a fabric's wiring — gives `n` no state of its own, but vita drove it from the t0 structural settle, which runs BEFORE declaration initializers, so the settle read the source at its declared default and the run loop's first delta moved the net AGAIN. ⭐ **The queue's own proposed fix was too narrow twice**: the same defect appears with NO port (`assign w = r;` in one module), and it must not read the initializer — an UNINITIALISED source must equally not manufacture an edge. Shipped = `sim_engine::alias::copy_nets` (whole net or a constant, in-range slice on both sides · equal widths · no delay · not multi-driver) + a repair pass in each backend's t0 arming · **47 FIXED · 0 REGRESSION** over 162 three-way cells · corpus digests, `.velab` and examples byte-identical. ⚠️⚠️ **The blocking finding was mine and both verifiers found it from opposite ends**: I first handed the copy net its sources' event status (`dirty[n] := OR(srcs)`), which reads as the same rule and is not — vita gives `n` and its source their **own storage defaults** (driven `wire` = z, `logic`/`reg` = x) where iverilog collapses them into ONE net, so a source can move while the destination provably never does, and the OR arm woke a child on a port holding `x` all run (16/56 generated cells, correct→wrong). ⇒ **suppression only** — and then **round 2 broke that**: asking only the IMMEDIATE source is wrong too, because a copy can stay put because its OWN default already equals the copied value (`assign vv=2'b1z` moves · its `wire` bit-0 copy does not · the `logic` copy of that one does, and iverilog fires on it) ⇒ the movement is carried along the CHAIN, with the `x` twin as the control that stops it collapsing back into the OR arm. ⚠️ Second NIT, also found twice: the cycle DFS marked the WHOLE stack on a back edge, so whether a reader of a cycle was repaired depended on **net-id order** — swap two `wire` lines and the same cell stops being fixed. ⚠️ **verilog-axi still not promoted and the residual is an ORACLE ARTIFACT**: with identical operands and values iverilog fires on `assign w = a | b;` and not on `assign w = a & b;` (it folds `&1'b1`, `~~`, `{}`, `?:` in its elaborator and not `|1'b0`, `^1'b0`) ⇒ matching it means reproducing a functor table, not a rule. ⚠️ Three existing tests encoded the old behaviour — two `bind` cells pinned **verilator's** extra t0 line on an axis where verilator is not an oracle, and one two-instance `case` cell could no longer see the bug it existed to pin (both instances are `x` in iverilog too) ⇒ re-aimed at an `initial` write, which IS an event. 신규 §2 행 = FST loses every value in a single-time-entry dump · `buf` lowers to `~(~i)` so a gate primitive is a bit move `copy_nets` cannot see.
- `4.5.396` 순서를 뒤집으니 답의 부호가 뒤집혔다**(2026-08-30 · **6,326 green** · format 29 불변 · **제품 코드 변경 0**): §4.5.395 의 leaf 예외가 게이트를 **한 단계 위로** 옮겨(darkriscv `Ternary` 4개 · 6,425,888 요청) 나머지 **set-wide 제거**를 시도 — §5.1 이 지어서·재서(1.00×)·기각한 그 변경이고, 그 측정이 **두 설계**였으니 여덟 설계로 재질문할 만했다 · 지었고(`>>>` 가드를 `signed`→`sw.signed && signed` 로 **먼저** 고쳐야 했다 — 게이트가 그 둘을 같은 질문으로 만들던 유일한 것) **sound 하다**: 배터리 8,225→**8,260** 승인 + 넓히기 45,180→**48,660**(전부 제네릭과 값 일치) · 리뷰 **~330,000칸 PRE==POST 바이트 동일** · 코퍼스 10/10 바이트 동일 · ⭐ hot 모양 **14개 중 13개가 2.1~4.7×**(24-assign 혼합부호 설계 1.26→0.27 s) · ⚠️⚠️ **그런데 코퍼스에선 여전히 1.00× ⇒ 되돌렸다** — §5.1 판정이 **두 설계가 아니라 여덟 위에서 재확인**됐고, 큐 문구는 *"쓸모없다"* 가 아니라 **"혼합부호 트리에서 2~4×인데 이 설계들 핫루프엔 그 모양이 없다"** 여야 한다 · ⭐⭐ **기록할 가치는 여기다: 내 첫 측정이 −4.5% 로 틀렸고 검증자 둘이 독립적으로 잡았다** — 방법 결함은 두 바이너리를 **같은 경로로 복사**하며 **항상 PRE 먼저** 돌린 것. **순서를 뒤집으면 답의 부호가 뒤집힌다**(PRE 먼저 = POST 0.2% 느림 / POST 먼저 = POST 1.1% 빠름) ⇒ 진짜 델타는 **±1% 위치 편향 안** · ⚠️ 이 세션에서 **복사-후-측정 A/B 가 속인 두 번째**(다른 하나는 되돌려진 변경에 −1.5% 를 지어냈고 다시 넣으니 **+3.2%**) ⇒ **양방향 인터리브가 아니면 재는 건 first-vs-second** · ⚠️ 재현 안 되는 성능 주장은 **방어하기 전에 스스로 다시 재라** — 이번엔 렌즈가 잡았지 내가 잡은 게 아니다.
- `4.5.395` leaf 는 묻지도 않았는데, 그 게이트가 한 설계 거절의 92%였다**(2026-08-30 · **6,326 green** · format 29 불변 · 적대 2렌즈 **BLOCKING 0** · 제품 코드 **한 조건**): §4.5.394 의 *"16바이트 레인 커버리지를 넓혀라"* 착수 · 새 실행가중 decline census 가 **darkriscv 거절의 92% = 6,600,872 요청을 표현식 딱 2개**에 몰아준다 — `compile_node` 진입 게이트 `sw.signed != signed` 가 **등폭 `Signal` leaf** 에서 발화 · ⚠️ **그 게이트의 sign 절반을 모든 노드에서 없애는 것은 이미 지어서·재서(1.00×)·기각**됐는데 그 측정이 **picorv32·keccak 두 설계**였다(헤더가 그렇게 적어 뒀다) — **darkriscv 는 거기 없었고**, 이 코퍼스가 존재하는 이유가 정확히 그 함정이다 · ⭐ **leaf 예외만** 취했고 근거는 *"비트가 어차피 같다"* 가 아니라 **arm 이 아예 안 묻는다** — `Signal` arm 의 **어느 출구도** `signed` 를 읽지 않는다(마스크를 통해서도). `slot.width == w` 를 요구하고, 아레나 슬롯 불변식(쓰기 생산자들이 전부 마스크)이 로드가 자기 마스크를 생략하게 해 준다 · **picorv32 −3.4% · darkriscv −1.6% · biriscv −1.6% · serv −1.2% · sha256 −0.9%** · 다이제스트 전부 불변 · ⭐ **코퍼스 수치는 레인을 과소평가한다** — 리뷰가 hot 단일 모양에서 **혼합부호 트리 4.8× · `signed^unsigned` 2.0× · 런타임 인덱스 signed 메모리 읽기 2.3×** 를 쟀다. 그 모양이 코퍼스에서 소수인 것이지 레인이 느린 게 아니다 · 리뷰 differential **CLEAN ~46,000칸**(13폭 × 부호 4조합 전 연산자 · 넓히기 합성 · 전 net kind · x/z 9,360칸 · fuzz 24,000) · soundness NIT 하나는 **내 주석**(상수 인덱스 `Load{vi}` 만 지목했는데 **런타임 인덱스 `LoadIdx` arm 도 새로 승인**되고 그게 이 모듈에서 **유일하게 진단을 움직이는 op** 다) ⇒ 주석 수정 + 셀 추가 · ⚠️ **재-census: 게이트는 사라진 게 아니라 한 단계 올라갔다** — 같은 테스트가 이제 **`Ternary` 4개에서 6,425,888 요청**으로 실패한다(게이트가 노드마다라 부모는 승인된 자식에게서 아무것도 물려받지 않는다) ⇒ 기각됐던 set-wide 완화의 영역이고 **`AShr` 가드를 `signed` → `sw.signed && signed` 로 먼저 바꿔야** 한다 · 남은 상위 버킷 = aes `ctx-width>64 (w=128)` 387k(넓은 레인) · keccak·biriscv `Call w=64`(프레임 축) · picorv32 `Ternary` 533k + `Unary(BitNot)` sign 178k · serv `Select(PartConst) w=6` / `Ternary w=2` 각 500k · 대 iverilog 기하평균 **1.57×**(여덟) · 1.71×(서드파티 여섯) · 1.93×(serv 뺀 다섯).
- `4.5.394` 4-state 는 데이터를 2배로 할 뿐, 나머지 56바이트가 인터프리터다**(2026-08-30 · 제품 코드 변경 **0** · 계측 후 되돌림): 질문은 *"VCS 는 4-state 인데도 빠른데, 우리가 느린 게 Rust 탓이냐"* · ⭐ **아니다, 그리고 4-state 탓도 아니다** — 4-state 는 **데이터를 2배**로 만들 뿐이라 44× 를 설명 못 하고, VCS 가 4-state 로 빠르다는 사실 자체가 그 반증이다 · `Value` 72바이트 중 **4-state 데이터는 16**, 나머지 56은 폭·부호·real/str 플래그 = **인터프리터가 필요한 메타데이터**(컴파일형은 생성 코드에 상수로 박는다) · 코퍼스 8개 `eval_ctx` 반환값 전수: **definite ∧ ≤64비트 = 83.9~100%**(기하평균 **95.7%** · 중앙값 99.7% · 최저 sha256 83.9(폭) · serv 89.7(부정확성) · picorv32 100) ⇒ **그게 `wprog` 의 `W=(val,unk)` 16바이트가 이미 나르는 모양**이고 2-state 레인은 `u64` **8바이트** ⇒ **표현은 트리 안에 있고 없는 건 커버리지다**(§4.5.391 의 승인 확대가 darkriscv −6.2% 를 낸 이유) · ⭐ 곁수확 = **heap 은 비용이 아니다**(8개 중 6개 `Vec` spill **0.00%** · sha256 만 16%) ⇒ 72바이트는 **인라인 값 복사**이고 그래서 `resize_keep_sign` 등폭 제자리 답이 산술 0으로 **−17.2%** · **unk 평면 지연화는 상금이 아니다**(≤64비트에선 두 평면 다 인라인 = 할당 0) · **전역 2-state + 트립와이어는 틀린 모양**(serv 의 x 는 진짜 — 초기화 안 된 레지스터 파일 — 리셋 직후 걸려 계속 걸린다) ⇒ 입도는 **연산 단위 레인** · ⚠️ `genpath_reads` 열은 **편향 표본**(`read_net` 은 빠른 경로가 거절한 넷만) — 설계 x/z 비율로 읽으면 안 된다 · ⚠️ **VCS 는 여전히 미측정** · 상세 = [study/03](docs/study/03-workload-corpus.md).
- `4.5.393` 고침은 찾았는데 한 축의 BLOCKING 넷이 배포하지 말라고 했다**(2026-08-29 · **6,321 green** · format 29 불변 · **제품 코드 변경 0** · 리뷰 2라운드): §2 🆕 B 착수 · ⓑ `case` 집합 부호는 스크루티니를 `$unsigned` 로 **감싸서** 적용했는데 그건 **래퍼에서 멈춘다**(인자가 self-determined · **네 툴 모두 그렇다**) ⇒ 스크루티니 안의 `>>>`·`/`·`%` 가 부호 읽기를 유지해 엉뚱한 arm · ⭐ **고침은 새 기구가 필요 없었다** — §11.8.1 size cast 가 이미 쓰는 **부호 나르는 문맥 lowering**(`lower_size_ctx_entry` · 제 폭을 넘겨 **폭엔 no-op**)으로 재-lower ⇒ case/casez/casex **6칸 두 오라클 일치** + **`/`·`%` 도 같이**(`b/c` 1→3 · `b%c` 2→1) · ⚠️⚠️ **되돌림 — 한 축 BLOCKING 넷이 전부 내 수정의 산물**: ⓐ 래퍼를 **폴백으로 안 남기고 교체**(signed `localparam`·계층 참조에서 규칙이 통째로 소멸) ⓑ **signed 함수호출 라벨이 unsigned 로 투표**(catch-all 이 절단 캐스트엔 보수적, 여기선 정반대) ⓒ 그걸 막으려 만든 `expr_sign_is_known` 의 `SysFunc => true` 가 화이트리스트와 **드리프트** — ⭐⭐ **내가 그 함수 docstring 에 경고로 적어 둔 바로 그 드리프트**이고 **두 렌즈가 독립적으로** 잡았다 ⓓ 그것까지 고치니 **`$bits` 라벨 2→1**(SV 상 signed `int` 인데 vita 는 unsigned 상수로 접는다) · ⭐⭐ **뿌리 한 문장** = 집합 투표가 `expr_self_signed` 에 기대는데 그 *"unsigned"* 는 호출·비화이트리스트 시스템함수·거기서 접힌 상수에 대해 **사실이 아니라 기본값**이다 ⇒ **선행조건 = 부호 provenance 를 "규칙이냐 기본값이냐"로 구분** · ⓐ(elaborate 상수 폴더)는 **`>>>` 결함이 아니라는 게 더 날카로운 진단**: `localparam [7:0] T=(8'd200+8'd100)>>2` 이 vita **75** / iverilog **11**(평범한 `+`·`>>`) — 모듈스코프 폴딩은 **선언 폭을 못 보는 무제한 i64 도메인**이고 폭-인식 폴더는 **const 함수 본문에서만** 도달한다 = 선언폭 provenance 벽 · ⚠️ 그 밖에 잰 것: 재-lower 가 진단을 **1→3**(이미 실패하는 설계에서만 · 일반 억제 기구는 loud→silent 라 더 나쁨) · §12.5 **공통 최대폭**은 **부호 참여자 없이도** 나타나는 별개 미구현.
- `4.5.392` 시프트는 제 피연산자에게 물었는데 LRM 은 결과에게 묻는다(그리고 그게 더 큰 규칙을 드러냈다)**(2026-08-29 · **6,321 green** · format 29 불변 · 리뷰 **3라운드**): §2 🆕 A 착수 · `>>>` 가 **왼쪽 피연산자**가 signed 면 부호채움했는데 §11.4.10 은 **결과 타입**이 signed 일 때만 그러라 하고 §11.8.1 은 문맥결정 영역에 unsigned 피연산자가 하나만 있어도 전체가 unsigned 라 한다 ⇒ 303칸 census **70 wrong · 오라클 분열 0**(`(b>>>2)+8'd0` vita 236 / 두 오라클 44) · ⭐ 부호는 **시프트가 품지 않은 피연산자**에서 온다 — 그래서 이웃 철자(`>>>` 단독·signed 비교·`$signed`)가 전부 정확했고 그래서 살아남았다 · 고침 = `eff_signed` · 복사본 둘(`eval_core` + tier-2 `native_eval`) · `wprog` 는 균일-부호 게이트가 이 가족을 먼저 거절해 무관(⚠️ 내 첫 docstring 은 *"wprog 는 이미 옳은 답을 냈다"* 고 적었는데 **리뷰가 측정으로 반박** — 거절과 정답은 다른 주장이다) · ⚠️⚠️ **그 수정이 만든 correct→silent-wrong 을 적대 리뷰가 BLOCKING 으로 잡았다**: 프레임 인자 copy-in 이 **formal 의 부호**를 문맥 부호로 넘긴다(§13.5.1 bind=대입 · §11.8.3 은 **폭만** · §11.8.1 은 부호가 좌변에 의존 안 함 — 실측 iverilog 에서 formal 선언 부호는 **효과 0**). 옛 `AShr` 이 `ctx_signed` 를 **무시해서** 우연히 면역이었고 고치자 `au(b>>>2)` 가 **4294967276 → 44** · ⭐⭐ **퍼널이 셋이고 라운드마다 하나씩** — 둘 고친 뒤 soundness 가 12자리 census 로 *"둘이면 충분"* 이라 했는데 **differential 이 세 번째를 측정으로** 찾았다(`task_frames.rs` `Terminator::Call` = **프레임 본문 안에서** 부른 **비-suspendable** 태스크 · 콜리에 `$display` 하나면 이미 고친 경로로 라우팅) ⇒ **자리를 세는 census 는 콜리의 성질이 고르는 자리를 놓친다** = 2렌즈 강제의 근거 · 라운드 3 **~9,200칸 · 회귀 0 · 분열 0 · 코퍼스 바이트 동일**(automatic/static · task/function · delay/wait/fork/$display 본문 · 깊이 1~4 에 suspendable 혼합 · class/virtual/package/hier) · 곁수확 = `inline_formal_bind.rs` 의 **문서화된 갭**(`ff(8'shf7)` `000000f7`→**`0000fff7`**)과 옆의 pre-existing 셋 · ⭐ 부산물 판정: signed **default 인자**에서 iverilog **자기모순**(같은 default·formal 인데 함수 `000000b3`/태스크 `ffffffb3` · 자기 plain-assign 은 태스크 쪽) ⇒ 여기서 오라클 아님 ⇒ `oracle_split_rulings.rs` 핀 · ⚠️ 남은 복사본 둘 = **§2 🆕 B**(elaborate 상수 폴더 · `case` 영역의 바깥 `$unsigned`).
- `4.5.391` 거절의 84%가 능력 하나였다: 좁은 값을 넓은 문맥으로 확장 못 함**(2026-08-29 · **6,312 green** · format 29 불변 · 적대 2렌즈 **BLOCKING 0**): 첫 실패 노드로 census 를 좁히자 darkriscv 거절 325k 중 **273k 가 비교 피연산자 폭 불일치**(3 vs 32 등)이고 나머지 대부분이 좁은 `Signal`/`LogNot`/`Select` 의 `sw.width != w` ⇒ **`wprog` 가 확장을 못 한다** 하나 · serv·picorv32 도 상위 두 가족이 같다 · ⭐ **목록은 훨씬 작다** — wcache 가 `(eid,w,signed)` 당 한 번만 컴파일하므로 241k 요청 = **표현식 7개** · 고침 = 좁은 노드를 승인하되 **어떻게는 LRM sizing 규칙**이 정한다(`node_ctx_class`): self-determined(리프·`Select`·`Concat`/`Replicate`·모든 1비트 결과)는 **제 폭에서 계산 후 변환**, context-determined(`~`·단항 `-`·비트연산·`+`/`-`·시프트·`?:`)는 **문맥 폭에서 계산** · 비교는 `ow=max(폭)`·`os=lw.signed && rw.signed`(= `eval_binary_ctx` 그대로) · 부호확장 = `WOp::Sext` 가 **`value::resize_word`**(= `Value::resize` 의 ≤64 arm 이 쓰는 그 함수)를 호출 ⇒ **의미를 두 번 안 적는다** · zero-extension 은 **op 자체가 없다**(모든 스택 값이 제 폭으로 마스크됨 — `LogBin` arm 이 이미 기대던 불변식) · **절단은 여전히 거절** · ⚠️⚠️ **`sw.width < w` 는 *"좁게 접고 넓혀라"* 가 아니다** — 첫 버전이 전부에 적용해 `logic [7:0] s = v[8:11] + 4'd1` 이 **16 대신 0**(15+1 을 4비트에서 접음), **핀 테스트가 잡았다** ⇒ 분류는 연산자 enum 위 **`_`-free match**(새 연산자가 catch-all 로 sizing 규칙을 물려받으면 안 된다) · **darkriscv −6.2% · serv −2.7% · picorv32 −1.8%**(aes·keccak 은 비용이 frame 안이라 flat) · 다이제스트 전부 불변 · ⭐ **darkriscv 동률 → 1.08× 앞섬** · 기하평균 **1.58×**(여덟) · **1.72×**(서드파티 여섯) · **1.94×**(serv 뺀 다섯) · 배터리 7,960 → **8,225** 승인 + **넓히기 스윕 45,180개** 전부 제네릭과 값 일치 · 리뷰 differential **CLEAN**(~10만 케이스 · 비교 64×64 전수 · 4-state 전수 · fuzz 1,130설계 · 코퍼스 바이트 동일 · ⭐ 먼저 **22개 hot 설계로 승인이 실제로 발화하는지** 확인해 null 결과가 커버리지 착시가 아님을 보였다) · soundness NIT 셋 = **전부 내가 남긴 stale 주석**(모듈 헤더의 *"admitted tree 안엔 확장도 부호확장도 절단도 없다"* 가 이 슬라이스로 거짓이 됐다 · 비교 arm 주석 · `kernel.rs` 승인 요약) ⇒ 넷 다 수정 · ⭐⭐ **fuzz 가 pre-existing silent-wrong 을 냈다**: §11.8.1 상 한쪽이 unsigned 면 비교 전체가 unsigned 이고 그 부호가 **피연산자로 내려가** `>>>` 를 논리 시프트로 강등해야 하는데 vita 는 안 내려보낸다 — `reg signed [7:0] b=8'shB3; (b>>>4) > 8'd100` 이 **vita 1 / 두 오라클 0** · `> 8'sd100` 과 `$signed(b>>>4)` 는 셋 다 일치 = 축이 하나로 분리됨 · 세 백엔드 전부 같은 답 ⇒ wprog 무관, 공유 sizing 경로 ⇒ **§2 🆕 A** 로 기록 + 핀.
- `4.5.390` 아레나 추정은 한 행에서 나왔고, 다음 슬라이스는 아레나가 아니다**(2026-08-28 · 제품 코드 변경 **0** · 계측만 · `3548ba9`): 직전 커밋이 *"새 프로파일로 재가격하라"* 고 적은 것을 실행 · ⭐ **상한이 내려간 게 아니라 올라갔고 설계도 바뀌었다** — 프레임 호출 안 제네릭 evaluator/`Value` 몫이 **aes 68.0%(3.13×) · keccak-arr 60.4%(2.52×) · keccak_f 39.3%(1.65×)**(프레임 호출 자체 비중 88.8 / 82.5 / 44.8%) ⇒ 옛 **2.33× 는 keccak-arr 한 행**에서 나온 수인데 실제 최대는 **aes**(서드파티 · 이미 2.22× 로 이김) · ⚠️⚠️ **그럼에도 다음 표적은 5c 가 아니다** — `frame_bodies`=0 인 **darkriscv(동률)를 처음 재니** 제네릭 evaluator **45%** vs 컴파일된 `WProg::run` **6.8%** 인데 **body 는 13/16 able** ⇒ 갭은 body 승인이 아니라 **body 안 표현식의 wprog decline** · 임시 실행가중 계측(**되돌렸고 커밋 안 함**): compile 요청 **568k 중 325k(57%) declined · 한 노드 종류가 78%**(`Ternary` w=32 **252,975/17,487**) · 그 밖 `BitXor` w=92(20k · lane 초과) · `Eq` w=1 **19,999/0** · `LogOr` 16,645/2,504 · `LogAnd` **8,746/0** · `Ne` **2,500/0** · ⭐⭐ **목록이 작다** — wcache 가 `(eid,w,signed)` 당 한 번만 컴파일하므로 그 수는 **요청 수**이지 표현식 수가 아니다: Ternary 는 **29개 표현식**이 253k 번 요청되고, 거절 사유는 **가지가 거절**(else 17 · cond 6 · then 4)이며 arm 이 방어하려던 `LoadIdx` 규칙은 **단 2건** · w=1 비교는 이 설계에서 **한 번도 컴파일 안 된다**(31k 요청 · 0 성공) = `lw.signed != rw.signed` 게이트이고 wprog 헤더가 *"round-34 리포트의 모양에서 같은 게이트가 2.9×"* 라 이미 적어 둔 가족 · ⚠️ **그 헤더의 census 는 picorv32·keccak 두 설계였다** — §4.5.369 가 잡은 바로 그 함정이고 darkriscv·serv 는 그때 없었다 ⇒ **다음 성능 슬라이스 = `wprog` 표현식 승인**(29개 목록 · 아레나가 아무것도 안 해주는 sha256·picorv32·darkriscv 를 덮는다 · 6–10주가 아니다).
- `4.5.389` 윈도는 불변 IR 의 순함수인데 호출마다 다시 지었다 (아레나 5a·5b)**(2026-08-28 · **6,305 green** · format 29 불변 · 적대 2렌즈 **BLOCKING 0**): 세 프레임 진입점(`run_frame_call_with`·`run_task`·`enter_task_frame`)이 **같은 여섯 줄**로 콜리 로컬 윈도를 IR 에서 재구성 — `Vec` 할당 + `locals_len`개 `Value::from_packed` + pop 에서 free, **호출마다** ⇒ 함수당 **템플릿 1회**(`frame_slot_default` 단일 생산자) + **캡 64 free-list**(`clear`+`extend_from_slice`) · ⭐ **로컬이 전부 static 인 함수**(= 평범한 비-`automatic` 베릴로그 함수)는 그 윈도를 짓고 `entry(func).or_insert(fresh)` 에 넘겨 **첫 호출 빼고 전부 버렸다** — 아무도 안 읽는 값에 per-call 비용 전액 · **5b** 정적 slab 이 dense `FuncId` 키의 `BTreeMap`(근거가 *"결정성"* 이었는데 인덱스면 **구성상** 결정적이고 아무도 iterate 안 한다) ⇒ `Vec<Option<Vec<Value>>>` · keccak **−7.7%** · aes **−6.8%** · keccak-arr −3.5% · sha256 −3.3% · picorv32 −2.2% · biriscv −1.4% · serv −0.5% · darkriscv +0.6%(잡음) · 코퍼스 다이제스트 전부 불변 · ⚠️ **sha256·picorv32 는 `frame_bodies`=0 인데 움직였다** — 그쪽이 받은 건 **버려지던 윈도 arm + slab 인덱스** · ⚠️⚠️ **재활용은 pop 이 증명가능하게 DROP 하는 자리에서만** — `stash_windows_in` 도 pop 하지만 윈도를 `FrameRec` 로 **옮겼다가 되돌린다**(재활용하면 살아 있는 윈도를 다음 호출에 넘긴다 · loud 증상 없음) ⇒ 동기 `&self` 실행기의 종단 pop **둘만** retire, Case-B fork 윈도는 아레나로 가므로 제외 · ⭐ 리뷰: differential **~624 설계 × 3 백엔드 = ~1,870 쌍 차분 0**(코퍼스 10 바이트 동일 · VCD 동일 · staged 동일), soundness 는 `frame_stack` 에서 항목을 빼는 **5개 자리 전수 census** 후 `run_task` 의 docstring 주장을 **받아들이지 않고 검증**(`compute_suspendable_tasks` → `exec/process.rs:391` 라우팅) · NIT 둘 중 하나(세 번째 retire 자리)는 **검증자가 측정으로 반박**(주장한 피해 메커니즘 재현 실패 · 최대화 설계에서도 상한 1.2% = 잡음 이하) · ⭐⭐ **그래서 아레나 6–10주 추정의 자리가 좁아졌다** — 그 밑 세 슬라이스 중 **둘이 윈도 표현을 안 건드리고** 코퍼스 전역 이득을 냈다 ⇒ 추정이 실제로 사는 곳은 **5c**(평탄 워드 버퍼 + `wprog` 가 프레임 본문 컴파일)뿐이고 **새 프로파일로 재가격**해야 한다 · flat/calls 콜 레짐 **7.3× → 6.9×**(⚠️ 양끝이 같이 줄어 각 변경이 시사하는 것보다 느리게 닫힌다) · 코퍼스 대 iverilog 기하평균 **1.55×**(여덟) · **1.68×**(서드파티 여섯) · **1.89×**(serv 뺀 다섯).
- `4.5.385` **Round 34: two external reports, and the biggest finding was in neither of them** · ⭐⭐ R5 was filed as a LOUD gap and the census found **19 silent-wrong cells** under it — a NARROW override of a `parameter logic [127:0]` was DISCARDED and the default used, at exit 0, on every channel (root = `wide_disagreeing_value(&p.value…)` folding the DECLARED DEFAULT while an override is in flight) · ⚠️ extending needs the override EXPRESSION's signedness, which three expressions with the SAME i64 extend two different ways ⇒ new `signed` channel, `None` declines · **R1** the count/index channel took `val[0] as u32` **silently** — boundary is 2³² not the report's 2⁶⁴ — so a shift folded UNSHIFTED, `A[2**32]` folded to `A[0]`, and a 2³²+2 replication built TWO elements; shift → **correct** (saturating, both oracles agree), count/index → **loud** (no consensus to adopt) · **R2** the diagnostics blamed an operand for an out-of-range index; the VALUE half is blocked on the narrow parameter store having no unknown plane (§2 15) · **R3** `/ % ** $clog2` wide arms, with the limb math **MOVED** to `sim-ir` so there is no second spelling · **R30-2** ⭐⭐ **both the report's axis AND §4.5.382's were refuted** — the only family where native loses is a **SIGNED CONSTANT operand in the RHS** (−57% once `wprog` admits a `Const` leaf across sign), and dropping the sign half entirely had already been built, measured **1.00×** on picorv32/keccak, and reverted · **R4** `able` is ANTI-correlated with speed (14.39 s inlined vs 0.35 s framed, digest-identical) so *"widen the inliner"* was the wrong direction · plus R6 · V34-1/2/4/5/6 · V33-1/3 · 상세=아래
- `4.5.384` **라벨의 폭은 선언된 사실인데 어떤 맵도 기록하지 않았다** · §2 9번 · ⭐ §4.5.383 과 같은 모양, 선언 자리만 하나 옆(enum base 타입) — `enum_base_range` 를 라벨 바인딩 **세 자리**가 기록 · ⭐⭐ **직전 슬라이스의 단일 바인더가 즉시 값을 했다**(안쪽 `localparam` 이 라벨을 가리는 칸이 새 규칙 없이 그대로 정답 — §4.5.383 에선 그게 BLOCKING 이었다) · ⚠️⚠️ **비-0 LSB base 는 오라클이 갈린다**(iverilog 171 / verilator 52)·ascending 은 iverilog 가 선언 거부 ⇒ **둘 다 declines** · ⚠️⚠️ **두 렌즈가 같은 전제를 반대편에서 반증했고 그건 내 것이었다** — *"§6.19 가 값의 canonical 을 보장한다"* 는 거짓(파서 검사가 **파라미터 bound 에서 fail-open**, `-G W=8` 로 두 오라클이 **거부**하는 설계를 vita 가 exit 0 · 한 라벨이 한 실행에서 **두 답**) ⇒ 실제 근거는 **소비자가 폭으로 좁힌다**는 것이고 그래서 `1..=64` 캡을 **source 에** 달았다 · 59칸 **FIXED 39 · REGRESSION 0 · MOVED 0** + 렌즈A 45칸 **REGRESSION 0 · 추가 19 수정** · 기록 = §2 12~14번 + 11번 · 상세=아래
- `4.5.383` **선언 폭이 사실인지 답하는 그 맵에 package twin 이 없었다** · §2 ⓓ · ⭐ 큐가 적어 둔 선행조건(*"패키지 const 테이블이 선언 폭 provenance 를 나른다"*)이 정확했고 답은 **새 맵 하나**(`pkg_const_range`, 모듈 twin 과 **같은 `param_decl_range_opt`** = 한 생산자 한 규칙) · ⭐⭐ census 가 한 줄 뒤에서 **다섯 레인**을 찾았다 — ⚠️ 큐의 *"런타임은 이미 세 툴 다 맞다"* 는 **zero-LSB 에서만** 참이라 `parameter [39:8] B` 는 `pk::B[15:8]` 을 **171** 로 찍고 있었다(두 오라클 52 · exit 0) · 세 철자(`pk::W` / wildcard / explicit)가 **한 갭** · 곁수확 = 파서가 `pk::W[7:0]` 을 *"이름이 아니다"* 라고 경고 · ⚠️⚠️ **적대 BLOCKING 둘이 내 것**(import 가 패키지 선언을 **모듈 자기 키**에 바인딩하면서 enum 라벨·genvar 의 재바인딩이 남은 range 를 물려받아 190→239 · 0→8) ⇒ **3/13 사이트 clear 는 틀린 모양** · ⭐⭐ 고침 = **바인더를 하나로**(`bind_param_value` 가 지우고 `bind_param_range` 가 쓴다 · raw writer grep 0) · ⭐ **두 렌즈가 같은 잔여**(>64비트 패키지 파라미터의 range 가 4분면 중 1개만 · **PRE 도 전부 틀렸다** = 사다리 이동 없음 ⇒ §2 10번) · 132칸 **FIXED 61 · REGRESSION 0 · MOVED 0** · 상세=아래
- `4.5.382` **외부 리포트 둘, 축은 하나: 상수 도메인이 i64 로 계산하는데 선언은 폭을 말하고 있었다** · round-33(11건) + aes_top 감사(14건) · ⭐⭐ census 가 둘을 합쳤다 — **25건 중 11건이 한 축**이고 뿌리는 `fold_self_bits` 의 NAME 훅이 `wide_param_bits` **하나만** 물어 좁은 이름이 전부 declines 한 것 · ⚠️⚠️ 폭 출처가 소유권 논증 전부 = **`param_range`**(선언 provenance만) — §4.5.373 이 `param_meta` 위에 짓고 되돌린 그 자리 · 실은 것: 좁은 이름 피연산자 · wide 산술/비교/리덕션/select/삼항/`>>>`/`$countones` 계열 · **패키지 wide 파라미터** · **override 가 폭을 나른다**(§6.20.2 · `-G K=128'h…` 도) · ⭐ **replication count 이름**(§4.5.371 의 BLOCKING 넷이 *같은 resolver 로 접기* 하나로 전부 답해졌다) · 패키지 real · 상수 문맥 string 메서드 · **generate 키워드 선택(§27.3)·헤더 genvar(§27.4)** · 패키지 static function 의 형제 호출(§26.3) · **진단 6건 = 한 결함**(캐럿이 모듈 헤더 · 원인을 안 말한다) · ⚠️⚠️ 내가 만든 결함 셋(**cast 는 문맥이다** · count 가 상수함수 지역변수를 읽었다 · 폭 미상 지역변수를 바인딩했다) · **핀 21개 전량 2-오라클 재측정** · ⭐⭐ **serv PROMOTED · 코퍼스 7/10 → 8/10** · ⚠️⚠️ **R30-2 는 리포트 진단이 반박됐다**(축은 LHS 모양이 아니라 **RHS 의 계산된 인덱스** · 0.92× vs 2.05× · wprog 넓힘은 지어서 되돌렸다) · 상세=아래
- `4.5.381` **매뉴얼이 쓰라고 안내하던 그 변환도 거절당하고 있었다** · §0 T2 의 마지막 열린 항목 = **real 상수가 정수 문맥에 닿는 것** · ⭐⭐ **철회가 이 슬라이스를 지목해 뒀다**: §4.5.232 가 i64 twin 으로 닫으려다 5건 silent-wrong 으로 철회하며 `strings.rs` 주석에 *"closing it means routing those sites through the real domain, which is its own slice"* 라고 적었고, 그 다섯 자리 중 셋은 §4.5.241/242 가 이미 닫았다 — 남은 것을 **leaf 가 아니라 소비자**에서 변환한다 · ⭐ **census 가 축을 명시/암시로 갈랐고 두 오라클이 같은 선을 긋는다**: 16형태×3툴에서 vita 는 16/16 loud, 두 오라클 합의 12 · 분열 넷은 **전부 암시 변환**이고 **서로 반대 방향**이다(`logic [R-1:0]` = iverilog 3/verilator 거부 · `{R{1'b1}}` = iverilog 거부/verilator 7 · `logic v [R]` · `R<<1`) ⇒ 규칙은 *"vita 가 숫자를 낼 수 있는가"* 가 아니라 **"언어가 이 노드를 변환이라 부르는가"** · ⭐ **근인 한 줄**: `count_reads_real_param`/`count_lowers_real_param` 이 *"real 파라미터를 언급하는가"* 를 묻는데 **`Cast` 와 `SysCall` 을 그대로 통과해 내려간다** — 그 둘이 바로 변환 그 자체 · ⭐ **반올림의 참조 구현도 트리 안에 있었다**(§6.24.1 = half away from zero · `$rtoi` 는 절단 · iverilog·verilator·**vita 자기 런타임**이 12칸 전부 일치 · 상수 도메인만 안 물었다) · ⚠️⚠️ **내가 만든 결함 둘, 둘 다 loud→silent-wrong 이고 둘 다 내 프로브가 잡았다**(스위트는 첫 번째를 통과했다): ⓐ *parity* 로 `$rtoi` arm 을 상수함수 walk 에 미러링했더니 정수 formal 이 모듈 `real` 을 가리는데 **모듈 스코프 resolver** 를 타 `f(9)`=3(iverilog 9 · §4.5.371 ⓷ 와 같은 모양) ⇒ ⭐ **고침은 arm 삭제** — 그 walk 의 catch-all 이 이미 *가릴 것이 없음을 증명하는 게이트* 아래서 모듈 스코프에 위임한다 ⇒ **shadow 규칙은 `env` 를 가진 walk 가 가진다** · ⓑ 도메인 순서 — real 도메인이 정수 부분트리를 `as f64` 로 올려 `$rtoi(2^53+1)` 이 **조용히 1 작게** ⇒ **정수 먼저**(real 은 답을 더할 수만 있지 고칠 수는 없다) · ⭐⭐ **스위트가 자기 핀 여섯을 지목했고 전부 제거된 제한을 인코딩**하고 있었다(다섯은 앞 슬라이스들이 갭을 **방어하려고** 쓴 것 · 하나는 docstring 에 §0 T2 를 **자기 선행조건으로 지목**) — ⚠️ 그중 **둘은 silent 를 핀하고 있었다**(`v[int'(3.5):0]` 이 **1비트** 를 읽는 것을 `Y=1` 로 · `$clog2(8.0)` 의 **0-count replication** 을 *"declines to 0, never a wrong non-zero"* 라 변호) — 0폭 replication 은 사다리의 안전한 쪽이 아니라 **조용한 쪽**이다 ⇒ 전부 오라클 재측정 후 **값-어서션** · ⚠️ **핀 하나는 제목이 틀렸다**(`a_non_integral_real_never_folds_into_the_integer_domain` 의 세 칸 중 둘은 **경계가 있어서** 변환이 불변식의 *처방*이다) ⇒ 불변식을 재는 칸만 남기고 **긍정 칸 추가** · ⭐ **곁수확 pre-existing silent-wrong 둘**(`nonconst_bound_reason` 에 `Cast` arm 부재 ⇒ `logic [int'(NOPE)-1:0]` 가 **1비트 exit 0** 인데 bare 쌍둥이는 loud = 도메인-쌍둥이 탐지기 · 그리고 위 select) · ⭐ **census 가 놓친 레인** = real **리터럴**(`int'(3.0)`·`$clog2(8.0)`) — 스위트 핀이 알려 줬다 · ⚠️ **수요 근거는 앞 네 슬라이스보다 약하다**(서드파티 8설계에 `parameter real` **0건** — 다만 코퍼스가 합성 RTL 이라 real 파라미터가 사는 테스트벤치가 없다) ⇒ 죽이는 근거로 쓰지 않고 기록 · ⚠️ **방법 메모**: 스윕 한 칸이 verilator 거부로 읽혔는데 **20초 워치독이 느린 기동을 죽인 것**이었다 — 워치독 킬은 오라클 판정이 아니다 · examples 4/4 바이트 동일 · 5,943 → **5,963 green** · format 29 불변 · **제품 코드 ~30줄**
- `4.5.380` **외부 리포트 두 건 — 진단이 스스로 지운 규칙을 인용하고 있었다** · round-32(hash_top) + aes_top, 둘 다 `6c4be81` 기준이라 **HEAD(4 슬라이스 뒤)에서 전 항목 재현부터**([[external-report-fresh-probe-triage]]) · **해결 4 · 기록 6** · ⭐ **N32-1 = silent-wrong**: `wire [$bits(8'h00)-1:0] c;` 가 **1비트 net** 을 선언해 `8'hA5` 가 `1` 로 잘렸다(exit 0 · 3 백엔드 동일 · **포트 바운드면 모듈 경계를 넘어** 잘린다) — 같은 호출이 런타임 6 · unpacked 차원 8 · 파라미터 E3009 = **한 줄에 세 답** · 고침 = ⓐ 값 fold `bits_of_selfdet`(리터럴·concat·replication, **leaf 는 전부 `bits_of_view`** = 이미 도는 `$bits(<이름>)` 과 **같은 resolver**, `const_self_width` 는 **의도적으로 안 씀** — 그건 `param_meta` 로 이름을 풀고 §4.5.373 이 그 폭의 canonical 성을 반증했다) ⓑ **못 접는 형태는 loud**(`nonconst_bound_reason` 의 SysCall arm — §4.5.377 에서 내가 고친 **바로 그 catch-all** 의 다른 arm이고, `$bits(<미선언 이름>)` 조차 조용히 1비트를 만들고 있었다) · ⭐⭐ **N32-3 이 이 슬라이스의 교훈**: 진단이 *"direct rhs of a blocking assignment (v9)"* 라는 **§4.5.374 가 지운 규칙**을 인용해 곧이 들으면 **멀쩡한 코드를 되돌린다** — ⚠️ 그리고 **자리가 둘**이었다(`$fgetc` 계열 `(v9)` · `$value$plusargs` `(v7)`), **이유도 서로 다르다**(평가 **횟수** vs ref **쓰기 순서**) ⇒ 문장 둘 + 캐럿을 **호출**로 · ⚠️⚠️ **내 첫 교체 문구가 과대주장**이었다(*"a task argument … supported"*)인데 `$monitor` 인자는 **task 인자이면서 정당하게 거절**된다(재렌더) — **기존 핀이 잡았다** ⇒ 원리를 *"written 과 다른 **횟수**로 실행된다"* 로 다시 씀 · **aes §4** `a[7:0][3:0]` 무경고 ⇒ ⭐ iverilog 문구가 규칙을 정확히 준다(*"All but the final index … must be a single value, not a range"*)라 **파싱 시점 판정 가능** · ⚠️ *"두 번째 select 면 경고"* 는 **틀린다** — `mem[i][3:0]` 은 어디서나 합법이고 도처에 있다 ⇒ 판별자는 **직전 select 가 range 였는가**(측정 후 확정) · **aes §5** 진단 둘이 한 사건에 모순(*"is not a constant; default kept"* 인데 상수이고 companion 이 error) ⇒ **wide 리터럴만** 분리, ⚠️ 진짜로 default 가 유지되는 `#(.W(sig))` 는 **옛 문구가 참**이라 핀이 지켜 줬다(내 첫 시도는 그것도 갈아엎었다) · 📌 **기록 6**: aes §1(값 2⁶³ 두 레인 — reduction 은 §3 ⑦, 나머지는 §2 canonical 벽) · aes §2(**CHANGELOG 정정** — *"discriminator is one keyword"* 가 거짓, 자체 실측 plain 3/5 vs `automatic`·`for`·`if`·`case` **전부 1/5** · `p::f()` 2/5 ⇒ **8행 표**로 재작성) · aes §3(⚠️ **iverilog 도 통과** ⇒ 2-오라클 결함 아닌 **lint 기회**) · R30-1 · R30-2 · N32-2 · 코퍼스 7/10 zero drift · examples 4/4 · 5,935 → **5,943 green** · format 29 불변
- `4.5.379` **enum 라벨이 상수 이름을 못 읽었다 — 그리고 ⑪ 은 착수 불가였다** · ⚠️ 원래 표적은 §3 ⑪ 이었는데 **census 가 착수 자체를 막았다**: verilog-axi 의 **54 에러가 전부 §3 ② 한 뿌리의 연쇄**이고(`parameter S_THREADS = {S_COUNT{32'd2}}` = 파라미터 count replication, §4.5.371 이 되돌린 것) ⑪ 의 네 칸은 **② 가 서기 전엔 보이지도 재현되지도 않는다** · ⭐ 그럼에도 ⑪ 을 최소 재현으로 재느니 **이름이 틀렸다**: *"wide 반환"* 이 아니라 **값**이다 — `[127:0]` 선언에 값이 `0x1122` 면 **이미 정확**하고, 막히는 건 **값이 64비트 초과**(65비트 리터럴 · part-select 로 쌓는 128비트 누산기)뿐 · ⚠️ **싼 절반(넓은 리터럴)은 수요 0**이고 verilog-axi 가 쓰는 건 **비싼 절반**(`calcBaseAddrs` 의 루프 누산기) = §4.5.373 이 되돌린 그 형태 ⇒ **② 뒤로 재배치**(넓은 도메인 `fold_self_bits`/`WideBits`/`BitPacked` 는 이미 있으나 **carry-free**) · ⇒ 소유자가 **§0 T2** 로 전환 · **실은 것 = enum 라벨의 상수 이름** · ⭐ **여기서도 census 가 큐 줄을 반박**: 줄은 *"sized-literal enum label"* 이라는데 sized literal 은 **이미 돈다**(`enum bit[7:0] { A = 8'hFF }` · `.name`/`.first`/`.next`/`.num` 전부 정확 · §4.5.234 가 했다) — 실제로 막힌 건 **이름 라벨**(`A = L`·`L+1`·`L*2`)이고 그러면 enum 이 통째로 `enum_defs` 에 안 들어가 **모든 메서드**가 오도적인 *"hierarchical function call"* 로 loud 였다 · ⭐⭐ **참조 구현이 파서 안에 이미 있었다**: `const_locals`(모듈 스코프 **localparam** 의 리터럴 값)와 `try_const_index` — 라벨 폴드만 그걸 안 물었다 · ⭐⭐ **안전성 논증도 이미 코드에 적혀 있었다**(*"A `parameter` is overridable → never recorded"*) 그리고 **측정으로 확인**했다: `m #(.K(9))` 로 override 하면 iverilog 가 **10/`first=9`**(4/3 아님) ⇒ 파서 시점 fold 는 **silent-wrong** 이므로 `parameter` 라벨은 **거절이 정답** · 14칸 census **FIXED 4 · 회귀 0** · 잔여 둘 **둘 다 loud**: `parameter` 라벨(근본 해소 = enum-method desugar 를 elaborate 로 = **아키텍처**) · sized-literal `localparam`(`const_locals` 가 **decimal 만** 기록 · 그 표는 generate 인덱스와 **공유**라 생산자를 넓히면 다른 소비자가 움직인다 ⇒ 별도 항목) · ⚠️ 스위트의 핀 하나가 **옛 제한을 인코딩**해 붉어졌다(`collision_nonfoldable_enum_methods_are_loud` 가 **localparam** 라벨을 썼다) ⇒ 둘로 쪼개 **localparam 은 값-어서션**(`num=2` 가 pa 의 3 이 아니라 **pb 자기 라벨**임을 **긍정적으로** 증명 = 옛 핀보다 강하다) · **parameter 는 loud 유지**(staleness 위험이 실제로 사는 자리) · ⚠️ 내가 새로 쓴 테스트 하나가 처음에 **아무것도 어서션 안 했다**(`let _ = (...)`) — §4.5.373 ⓒ 와 같은 모양이라 즉시 고치고 **세 기대값을 iverilog 로 먼저 재고** 핀했다 · 코퍼스 7/10 zero drift · examples 4/4 바이트 동일 · 5,932 → **5,935 green** · format 29 불변 · **제품 코드 한 줄**(`.or_else(|| self.try_const_index(e))`)
- `4.5.378` **루트의 포트는 정의상 unconnected — 큐가 지목한 원인은 iverilog 와 같은 동작이었다** · 큐가 지목한 §3 ⑥ · ⭐ **census 가 진단을 반박**: 줄은 *"auto-top 이 미인스턴스화 루트를 전부 잡는다"* 를 결함으로 봤는데 그건 **IEEE 1364·iverilog 가 하는 그대로**다 — 독립 모듈 둘이면 iverilog 도 **둘 다** `initial` 을 돌리고(`VAL=A|VAL=B`), 공유 서브모듈의 오류도 **루트마다** 낸다(`in libtop.i` **그리고** `in tb.i`) · vita 는 거기에 `W-ELAB-AUTOTOP-AMBIGUOUS` 로 루트 이름까지 알려 주므로 **iverilog 보다 앞서 있다** ⇒ **루트 선택은 손대지 않았다** · ⭐⭐ **진짜 결함은 다른 것**: serv 경고 21 중 **19가 *"output port left unconnected"*** 인데 그건 **루트라면 반드시 성립하는 조건**(top 의 포트는 어디에도 안 붙는다 — 그게 top 의 정의) · ⚠️ **auto-top 과 무관**하다는 것이 결정적 — 포트를 가진 top 을 `--top` 으로 **하나만 핀해도 똑같이 경고**하고, serv 에서 `--top tb` 가 조용했던 건 그 tb 가 **마침 포트가 없어서**였다 · 두 오라클 다 침묵(iverilog 는 `-Wall` 로도) ⇒ **루트에서만** 그 경고를 끈다 · ⚠️ **포트 바인딩은 대용물이 될 수 없다** — 자식을 `dut u();` 로 써도 **같은 빈 바인딩**으로 들어오는데 거기선 그 경고가 **진짜 정보**다(측정으로 확인) ⇒ 판별자는 호출자가 아는 **`parent_inst.is_none()`** 을 `wire_ports` 로 넘긴다(§4.5.377 의 *"프록시를 쓰지 말고 사실을 넘겨라"* 와 같은 모양) · **측정**: serv auto-top **경고 21 → 2**(남은 둘 = auto-top 모호성 + timescale = 둘 다 실행 가능) · 오류 6 불변(루트 2 × 실제 3 = iverilog 와 같은 중복) · `--top tb` 3/1 불변 · 자식 경고 두 형태(`u()` · `u(.clk(c))`) **보존** · 코퍼스 7/10 zero drift · examples 4/4 바이트 동일 · 5,930 → **5,932 green** · format 29 불변 · **제품 코드 3줄 + 시그니처 한 인자**
- `4.5.377` **패키지 파라미터 fold 의 네 번째 복사본이 string/real 을 몰랐다 — 그리고 §2·§3 두 줄이 한 뿌리였다** · 큐가 지목한 §3 ⑨ · ⭐ **census 가 큐 줄을 또 넓혔다**: 줄은 *삼항*(`(S=="AUTO") ? "RED" : S`)을 갭이라 했는데 실제로는 **패키지 안의 string/real 이 전부** loud 였다 — `parameter S = "RED";` 라는 **바레 리터럴**까지(fold 가 `const_eval_in_scope` **하나만** 묻는데 정수 전용이라 리터럴이 **착지할 도메인이 없다**) · ⭐⭐ 그리고 **§2 의 package-real silent-wrong 이 같은 뿌리**였다 — `pk::PR` 는 3.000000 으로 **값이 넘어가는데 도메인이 안 넘어가서** `pk::PR / 2` 가 정수 도메인에서 나눠 **1.0**(두 오라클 1.5, exit 0) ⇒ **§2 행과 §3 행이 한 수정으로 닫혔다**(ⓔ 가 거짓으로 주장했던 그것의 진짜 사례) · 19칸 census **FIXED 13**(loud→correct 12 + silent-wrong→correct 1) · **회귀 0** · ⭐ **참조 구현 = `generate.rs`**, 그게 옳은 이유는 형태를 정하는 성질을 공유하기 때문 — generate 스코프도 패키지도 **override 채널이 없다** ⇒ 선언 기본값이 언제나 바인딩 · arm 순서 real → string → 정수, 앞 둘은 **fall-through**(§4.5.364) · ⚠️ 큐가 권한 *"`bind_one_param` 이 흡수"* 는 **안 했다**(그쪽은 override 채널을 다루는 다른 케이스) · ⚠️⚠️ **내 수정이 만든 회귀 둘을 내 soundness 렌즈가 잡았다 — 둘 다 내가 깨뜨린 한 불변식**(*"모든 패키지 파라미터는 `pkg_consts` 에 있다"*): ⓐ 이름공간 중복 검사가 i64 `consts` 를 물어 `parameter S = "RED"; int S;` 가 **exit 0 통과**(두 오라클 거절 · 정수 twin 은 loud 유지) ⇒ 검사가 묻는 건 **NAME SPACE** 이므로 인자를 이름 집합으로 **재타입** ⓑ `nonconst_bound_reason` 의 `pkg::` arm 이 **자기 주석에 예고한 구멍**(*"알 수 없는 `pkg::name` 은 조용한 unfoldable 을 유지한다"*)으로 `logic [P::S-1:0] v;` 가 **조용히 1비트**(두 오라클 5391684) ⇒ **모듈 스코프 쌍둥이가 loud** 라 branch parity 로 복구(값 도메인 escalation 을 피하는 자리 = §4.5.370 의 함정) · ⚠️ 그 수정의 NIT 도 내 것 — reason 을 **문장**으로 써서 호출자의 *"… is not allowed in a constant range bound"* 와 겹쳐 읽혔다 ⇒ 형제들처럼 **명사구**로 · ⭐ **곁수확**: `const_str.rs` 의 `PkgScoped` arm 이 `str_param_raw` 를 **`"P::S"` 철자**로 찾는데 **그 키의 생산자가 없다**(fold 는 `$pkg$P.S`) ⇒ 지원되는 것처럼 읽히는 **도달 불가 코드**(§4.5.370 의 *"소비자가 하나뿐인 참조 구현"* 과 같은 모양) — 고치니 `generate if (P::S == "AUTO")` 가 산다 · ⚠️ **열지 않은 것 = 와일드카드 import**(`import pk::*` 뒤 바레 이름): fold 는 성공하고 **import 바인딩만 없다**, `apply_import_consts` 가 §26.8 부기와 함께 `params`(i64)로 재바인딩하는 **배관**이라 별도 항목 · **loud 유지, silent 아님** · real 의 generate-if 조건도 loud 유지(`const_real.rs` 에 `PkgScoped` arm 부재) · 코퍼스 **7/10 zero drift** · examples 4/4 stdout+VCD **바이트 동일** · staged `vcmp→velab→vrun` 동일 · 5,919 → **5,930 green** · format 29 불변
- `4.5.376` **The revert's reason was the thing that needed measuring — §3 ④ re-landed** · started as §2 row 7 (the t0 process order) and the **census refuted the queue line, which was §4.5.375's whole case for a revert** · that slice built the hierarchical `$readmem*` memory argument, matched forty-odd shapes, then reverted it because *"vita runs a PARENT's `initial` before its child's while **both oracles** run the child's first"* · ⭐⭐ **re-measured on the exact design that claim cites: iverilog `aa bb cc dd`, verilator `01 02 03 04` — verilator prints VITA's answer** · confirmed not a dropped write (delete the child's competing load and verilator honours the parent's hierarchical `$readmemh`; a plain `u1.s = 8'hAA` likewise) ⇒ IEEE 1800 §4.7 makes `initial` order explicitly nondeterministic and the two oracles use that freedom in opposite directions = **ORACLE SPLIT**, same ruling as §4.5.372's cont-assign order · ⭐ **and the demand was absent too**: none of ④'s four motivating testbenches has a competing child load (serv never sets `+firmware=`, picorv32's `wb_ram` gets no `.memfile`, `axi4_memory` has no load of its own), and the serv digest the revert cited **cannot have come from ④** — serv does not elaborate with or without it, PRE and POST both stopping at the same three §3 ⑦ errors, byte-identical ⇒ **④ was never blocked** · **implementation = the one §4.5.375 described, and it builds nothing**: `expr_array_view` already resolves dotted paths, the cross-instance name simply has no net yet (child nets are pass 8, the parent body lowers in pass 7), and the deferred placeholder is ALREADY `Signal { net: POISON_NET, word: None }` = the exact shape the local arm builds by hand ⇒ the change RECORDS which eids sit in the `$readmem*` memory position and exempts **only** the whole-array arm of the read guard for them; the arm deliberately calls the SAME `lower_expr` it would have fallen through to, so the entire behavioural delta is *which eids the guard skips* · scoped to the readmem family and §21.4's arg 1 (`$dumpvars` keeps its word-0 surface; args ≥ 2 are addresses) · `deny_const_param_write` moved to RESOLVE time for `$readmem*` only, so `$writememh(f, dut.P)` still passes (the §4.5.375 soundness NIT about over-applying it) · parens peeled (its other NIT) · **18 cells**: 8 positive (depth 1/2, `$readmemb`, start/end addresses, `$writememh`, filename from `reg [1023:0]`, parenthesised, generate-scoped) all matching iverilog with `$writememh`'s file **byte-identical**; 3 edge (interface member unchanged, time-ordered overwrite `t1=aa t6=99`, **upward** ref `tb.mem` from a child); 7 negative holding (`x = dut.mem;` still loud, event, dyn handle, automatic local, `$dumpvars` unchanged) · corpus **7/10 unchanged, zero drift** · examples 4/4 stdout+VCD **byte-identical** · staged `vcmp→velab→vrun` identical ⇒ **format 29 unchanged** (no sidecar) · 5,912 → **5,919 green** · ⚠️ **what did NOT ship**: the READ direction (a parent `initial` reading a child net at t0 gives X where both oracles give the value — 10 cells, no disagreement) is a real §2 silent-wrong but is **pre-existing, independent of `$readmem`, and exercised by zero of the ten corpus workloads** ⇒ rewritten as §2 row 7 and ⓔ **demoted** · ⭐⭐ lesson to ENGINEERING_RULES: *a revert's reason is a measurement, not a finding* — it propagates into the queue line, a new §2 row, the briefing and a test docstring **within one slice**, so a wrong reason acquires three corroborating copies by the next morning; and *"both oracles agree"* is a two-part claim whose **second half is the load-bearing one**, so record that tool's actual output text rather than the conclusion
- `4.5.375` **The file name was never allowed to be a value; the hierarchical memory was built, measured, and reverted** · started on §3 ④ and the **census refuted the queue line** — every ELEMENT shape already worked (11 cells, closed back in June by N3.1 / its multi-dim follow-on / HIER-REST), and the real gap was the **whole array** as a `$readmem*`/`$writemem*` argument, three cells at one gate asking *"does this have a readable value?"* — the wrong question for a task that wants the array · demand = **four upstream testbenches** (serv, picorv32 ×2, ibex) · ⭐ the implementation was small and verified (`expr_array_view` already resolves dotted paths; the deferred placeholder is ALREADY the shape `$readmemh` builds by hand) and **40+ shapes matched both oracles**, including the real serv SoC · ⚠️⚠️ **reverted anyway**: vita runs a PARENT's `initial` before its child's while **both oracles run the child's first**, so a RAM loading its own memory overwrites the testbench's load (`01 02 03 04` vs `aa bb cc dd`, exit 0; serv digest `…523a` vs `…543a`) ⇒ the construct traded loud for **silent-wrong on its own motivating idiom** · ⭐ revert rather than fix because the ordering is **pre-existing and independent** (`u1.s = 8'hAA` already loses, no `$readmem` anywhere), `sim_ir::Process` is **frozen** so the rank must be an out-of-band sidecar, and `tie` is shared with `Comb`/`Latch` and fork children ⇒ blast radius is every design with a child `initial` · ⭐⭐ **what ships is what the revert uncovered**: `$readmem*`/`$writemem*` accepted only a string LITERAL and took a **bare `return`** otherwise — no file, no diagnostic, exit 0 — though §21.4 asks only for a string EXPRESSION and the canonical testbench uses `reg [1023:0] firmware_file`; a `string` variable was equally silent; **purely local designs were affected**, so it stands alone · ⚠️ the two sites were the same six lines ⇒ one `memfile_name` helper · adversarial: soundness **no BLOCKING** (3 NITs, 2 fixed), differential **1 BLOCKING** + 40 clean, and it **retracted four findings** measured against a binary rebuilt underneath it ⇒ re-measured from a frozen snapshot · 55 designs **zero correct→wrong** · examples 4/4 + corpus 10 **PRE == POST** · format 29 unchanged
- `4.5.374` **직접-rhs 전용 시스템함수를 식 어디에나 — 참조 구현이 또 트리 안에 있었다** · 코퍼스가 지목한 §3 ③ · **darkriscv 전체 SoC(3,115줄·9모듈)가 처음 돌고 다이제스트가 라이브 iverilog 일치**(`b4a7bb6d411fea85`) · census 42칸(4형태 × 11위치) **PRE 41 loud** — 유일 예외가 `if ($value$plusargs(…))` 로, `lower_branch_cond` 가 **한 계열·한 위치**만 desugar 했고 `!` 하나에 도로 E3009(serv 철자) ⇒ POST **31 correct · WRONG 0** · ⭐ `hoist/general.rs` 가 이미 같은 변환을 하고 `shape()` 라는 평가-순서 **단일 정본**을 갖고 있어 그대로 소비 · ⚠️⚠️ **리뷰 전에 내가 잡은 둘** — temp 를 unsigned 로 지어 `$fgetc(fd)!=-1` 이 영원히 참(EOF 는 **−1**) · rhs 가 bare call 일 때 인덱스만 hoist 해 **평가 순서를 뒤집었다**(오라클은 rhs 먼저) · ⚠️⚠️ **적대 2렌즈 BLOCKING 여섯, 전부 내 게이트의 구멍**: ⓐ `write_arg_range` 가 **뒤집혀** `$fgets`/`$fread` 의 fd 를 write 로 세 겹침 게이트가 전부 통과 ⓑ `Shape::NoHoist` 자식 읽기가 안 보이는데 `rebuild` 는 노드를 살린다(`general.rs` 의 `reads_are_opaque` 가 방어하는 그 구멍) ⓒ 별칭을 놓침(자기-계층 `m.a` = `a` · `p::v`) ⓓ 미러링하며 `task_call_inout_root_written` 누락 — 그것만 되살려도 부족해 **input 아닌 actual 전부**를 살아남는 읽기로 넣는 `_with(seq, extra)` 신설 ⓔ 새 sigil 이 VCD/FST 에 새고 **이미 동작하던** `if ($value$plusargs(…))` 의 파형까지 바꿈 ⓕ ⭐⭐ **내가 이 슬라이스에서 쓴 docstring 의 전제가 거짓** — *"fd 상태는 문장의 어떤 식도 읽을 수 없다"* 위에 조기탈출을 세웠는데 `$feof` 가 그걸 읽는다(vita 9 / ivl −1) 그리고 **값 의존적**이라 파일 중간 프로브는 초록 · ⓑⓒ 는 fail-closed `opaque`, ⓕ 는 `fd_observer` + 조기탈출 순서 반전으로 해결 · ⚠️ 마지막 NIT 도 내 수정의 부작용(`Unevaluated` 를 `Uncond` 에 묶어 `$bits` 자식이 읽기로) · differential 렌즈 **112칸 스윕 DIVERGE 0** · examples 4/4·코퍼스 10 PRE==POST · ⭐ 스위트가 **옛 제한을 인코딩한 핀 여덟**을 짚어 전부 값-어서션으로 갱신 + 거절 쌍둥이 둘 신설(그중 한 docstring 은 *"desugar 할 문장이 없다 — deliberately, not a gap to close cheaply"* 라는 **불가능 주장**이었다) · 회귀 24건 · format 29 불변
- `4.5.373` **리덕션 연산자가 상수 도메인에 없다 — 지어서·재서·되돌렸다** · 코퍼스가 지목한 §3 ⑦ · 넣으면 **serv 가 돌고 다이제스트가 오라클과 일치**(코퍼스 7/10 → **8/10**)했지만 BLOCKING 넷에 되돌렸다 · ⭐ census: generate 만의 문제가 아니라 **여섯 연산자 · 두 철자 전부** 2-오라클 loud 이고 **값 walk 하나만** 비어 있다(폭은 이미 1, 부호는 이미 unsigned — §4.5.371 의 **정반대**) · ⚠️⚠️ ⓐ 리덕션이 `const_self_width` 를 **값으로** 쓰는데 그건 `param_meta`(선언·추론 **혼합**, 미스 32)를 읽어 `generate if (&W)` 가 **반대 가지**를 exit 0 으로 골랐다 ⓑ `~`/`-` 는 **문맥 결정**인데 module-scope walk 엔 문맥이 없어 `~(\|P)` 가 raw i64(4294967294 vs 0) ⓒ 좁힘 게이트를 **한 arm 에만** 달아 `localparam` 철자에 ⓐ 가 살아 있었고, 그 칸의 테스트가 **`VAL` 을 어서션 안 해** 틀린 답 위에서 초록이었으며 docstring 의 PRE 인용도 **거짓**이었다 ⓓ ⭐⭐ **좁힘의 근거인 정리가 거짓** — 전제 *"기록된 i64 는 참값의 확장"* 이 성립 안 한다: `parameter A=4'h1; localparam W=A<<4;` 가 vita **16/32비트** vs 두 오라클 **0/4비트** ⇒ 여분 비트가 **안쪽**에 있어 폭에 둔감한 `\|` 마저 가지를 뒤집는다 ⇒ **이름 위의 리덕션은 여섯 전부 신뢰 불가** · ⭐⭐ 선행조건이 **두 단계**(폭의 declared provenance + **값이 그 폭에서 canonical**)이고 리터럴만 허용하는 반쪽은 **수요 0** 이라 전부 되돌림 · ⚠️ **같은 벽을 세 문으로**(§4.5.371 select 바운드 · concat 폭 · 리덕션) ⇒ 큐 세 줄이 아니라 **벽 하나**로 §3 에 기록 · 되돌린 뒤 6칸 PRE==POST · **제품 코드 변경 0** · format 29 불변
- `4.5.372` **`$fatal` 이 자기 출력을 못 막았다 — 그리고 `$finish` 승격은 지어서·재서·되돌렸다** · 코퍼스가 지목한 §3 ⑧ · ⭐ census 가 진단을 반박(*"system task call"* 이라는데 10칸 중 **`$finish`·`$stop` 둘만** 거절 · 나머지 여덟은 이미 돈다) · **실은 것** = §20.10 상 `$fatal` 은 종료하는데 `&self` 실행기의 래치를 **문장 경계**에서 처리하느라 `$display("VAL=%0d", f(7))` 의 출력이 나갔다(**pre-existing silent-wrong** · task 호출과 `r=f(7)` 은 원래 정확 = 자리가 좁아 고칠 수 있었다) · ⚠️ 술어는 `call_fatal` 이 아니라 **`call_fatal && !finished`** — 래치가 안 지워져 맨 셀로 물으면 `final` 블록 출력을 통째로 삼킨다 · ⚠️⚠️ **되돌린 것** = `$finish`/`$stop` 승격(넣으면 verilog-ethernet 이 elaborate 통과) — **BLOCKING 여섯 중 넷이 내 수정이 만든 것**: 라우팅이 셋이라 native 가 *"실행기가 드롭한다"* 는 **거짓 진단**으로 폴백 · `Step::Fatal` 소비부 넷이 `Error` 를 가정해 `$finish` 가 exit 1 · BB 루프 bail 이 없어 `$finish` 뒤 문장이 계속 출력하고 `$fatal;$finish;` 가 `run.json` 을 **재라벨** · ⭐⭐ 그런데 그 bail 이 **중간에 멈춘 본문의 반환값을 caller lvalue 에 커밋**시켰다(vita **x** / iverilog **55** / verilator **21** = **셋 다 다른 답**)고 `$fdisplay`/`$strobe` 값을 **8→x** 로 바꿔 **silent↔silent 맞바꿈** ⇒ 승격은 *"멈춘 본문의 반환값"* 이 미정이고 쓰기-다음-검사 순서가 E4002 를 지키려 **의도적**이라 되돌리고 선행조건 기록 · ⭐ 리뷰의 다섯 번째는 **컨트롤이 무죄 증명**(cont-assign 순서는 PRE 도 같고 **verilator 가 vita 편** = 오라클 분열) · 13설계 **바이트 동일** · 회귀 6건 · format 29 불변
- `4.5.371` **concat 의 폭을 값에서 추론했다 — 그리고 count 넓힘은 지어서·재서·되돌렸다** · 코퍼스가 지목한 §3 ② 착수 · census 가 큐를 반박(*"loud→supported"* 인데 **11칸 전부 2-오라클 합의인데 vita 는 전부 틀리고 둘은 silent-wrong**) · **실은 것 = 폭 축**: §11.4.12 상 concat 은 자기결정인데 무타입 fallthrough 가 접힌 i64 에서 `min_signed_bits(v).max(32)` 로 재 `{2{32'd2}}` 이 **35**(오라클 64) · 미override `{2{8'h1}}` 이 **32**(오라클 16) — 둘 다 **pre-existing silent-wrong** · ⚠️⚠️ 그 arm 의 게이트 둘은 **리뷰가 측정한 뒤 붙었다**: ⓐ `default_binds` — §6.20.2 는 범위를 **최종 override 값**에서 가져오는데 선언식에서 폭을 정하니 `#(.P(32'hDEADBEEF))` 가 **16비트 `beef`**, 한 칸은 PRE loud 였는데 **514비트 버스를 조용히** 지었다(⭐ 내가 *"타입-결정 계열"* 이라 적은 게 정확히 틀린 자리 — **선언 타입은 override 를 견디지만 값-결정 폭은 못 견딘다**) ⓑ **leaf 단위 provenance** — resolver 가 `param_meta`(값-추론 폭)를 읽고 없으면 `(32,false)` 추측이라 `{W}` 가 **263비트 net**(§4.5.363 회귀의 concat 문 재진입) ⇒ leaf 가 전부 sized 리터럴일 때만 declared · **되돌린 것 = count 넓힘**: verilog-axi 를 **54 → 4** 까지 내렸지만 BLOCKING 넷, **마지막 셋이 내 수정이 만든 것** — ⓵ **폭 쌍둥이를 안 넓혀** 폭-인식 walk 가 무제한 도메인으로(⭐ **오라클 없이 유죄**: 같은 파일의 리터럴 쌍둥이가 다른 답) ⓶ 상수함수 **지역변수**를 접어 170(iverilog 명시 거부) ⓷ 고쳤더니 module-scope evaluator 가 **같은 이름의 지역변수를 지나쳐 파라미터**를 잡아 **43690** ⓸ 그 evaluator 는 **콜 깊이를 0으로 재시작**해 count 안의 호출이 **스택 오버플로·abort**(PRE loud) ⇒ ⭐⭐ count 축은 상수함수 **스코프**·**깊이**·`declared_only` **provenance** 를 동시에 만족해야 하고 그건 폭 축과 다른 머신러리다 ⇒ **되돌리고 메커니즘 기록**(§4.5.361 선례) · 13설계 **바이트 동일 13/13** · 회귀 9건 · format 29 불변
- `4.5.370` **문자열 상수 도메인이 리터럴 전용이었다 — 그리고 참조 구현은 소비자가 하나뿐이었다** · 코퍼스(§4.5.369)가 지목한 §3 ① · ⭐⭐ `const_str_in_scope` 가 StrLit·Paren·Ident·PkgScoped 를 이미 풀고 있었는데 **호출자가 하나**(문자열 동등비교)라 아무도 **값**을 안 물었고 파라미터 바인딩 일곱은 리터럴 전용 쌍둥이를 쓰고 있었다 · 큐엔 *"삼항"* 한 줄인데 census 는 **바레 리터럴을 뺀 도메인 전체**(15/16칸이 2-오라클 합의인데 vita 전부 loud) · 고침 = `Ternary`(조건은 정수 도메인과 **같은 철자** · **양 arm 다 문자열** = fail-closed)와 `Concat` 두 arm + 값 소비자 라우팅(진단 가드는 *"리터럴인가"* 라는 다른 질문이라 제외) · ⚠️⚠️ **BLOCKING 넷 중 셋이 내 수정이 만든 것**: ⓐ 값이 **따옴표 포함 raw** 라 `{"RE","D"}`=`RE""D` ⓑ ⭐⭐ `str_param_raw` 가 **폭을 안 들고 다니는데** 문자열 경로가 수치 경로보다 먼저 돌아 `[95:0] X={"A","B"}` 가 96→**16비트** ⇒ **폭 게이트** ⓒ `{"ab",""}` 의 `""` 는 **NUL 한 바이트**인데 텍스트 join 은 0글자 ⇒ declines ⓓ *"문자열인가"* 가드를 **값 도메인**에 물어 `parameter W={"A","B"}` 가 합법적 `#(.W(9))` 를 거절(correct→loud) — 그런데 가드만 되돌리니 **폴드된 기본값이 override 를 삼켜** 16706(loud→silent) ⇒ `overridden` 플래그 · ⚠️⚠️ **재리뷰 BLOCKING**: 게이트 술어 `ty == Implicit` 의 틈으로 **범위 없는 `logic`/`reg`/`bit`**(폭 1비트인데 `Implicit`+`range:None`)가 빠져 `localparam bit P={"A","B"}` 가 1→**16비트** — 파서가 `var_kind` 를 **기록했다가 버려** `parameter bit P` 와 `parameter P` 가 구별 불가능했다 ⇒ 파서 **세 줄**(range 의 **마지막** fallback `[0:0]` — `forced_range` 에 넣으면 explicit range 를 덮는다) · ⭐⭐ **곁수확**: 그 틈이 문자열과 무관하게 이미 새고 있었다 — `localparam bit N=8'hFF` 가 vita **255/8** vs 두 오라클 **1/1** · 27칸 3-오라클 **FIXED 15 · ok→wrong 0** · 14설계 중 **12 바이트 동일**(나머지 둘이 표적: serv 10→3 · vether 6→2 에러) · 5,845 → **5,865 green**(회귀 20건) · `const_fn.rs` 1402줄 ⇒ **`const_str.rs`** 분리 · format 29 불변
- `4.5.369` **워크로드 코퍼스 — 남이 쓴 RTL 로 값을 매기기** · 성능 판단이 오래 설계 **둘** 위에 서 있었다(picorv32 · **우리가 재려고 쓴** keccak) — 허가적 라이선스 서드파티 **여덟**을 핀된 SHA 로 가져와 오라클로 고정 · ⭐⭐ **첫 수확이 성능이 아니라 정확성이었다**: 여덟 중 **셋이 거절**되고 셋이 **전부 같은 축**(상수 도메인 파라미터 폴딩 — 문자열 삼항[verilog-ethernet] · 문자열 Ident[serv] · 정수 replication[verilog-axi])이며, **우리 프로브에서 나온 §2 큐엔 그 축이 한 줄**로 있었다 ⇒ 우리 프로브는 우리가 의심하는 것을 찾고 남의 RTL 은 **우리가 의심하지 않는 것**을 찾는다 · ⭐ 최소 재현이 축을 삼항 하나로 좁혔다(`(S=="AUTO")` 정수 localparam · `parameter SI="RED"` · generate-if 문자열 비교는 **셋 다 이미 정확**하고 `1 ? "RED" : "BLUE"` 만 E3009 · **iverilog·verilator 둘 다 RED**) · 그 밖에 `$fgetc`/`$value$plusargs` 가 **blocking 대입 rhs 에서만** 산다(논블로킹 쌍둥이·조건식 · 4줄 재현 · [[branch-parity-before-new-traffic]]) · `$readmemh(f, dut.ram.mem)` 계층 원소 선택 · **ibex 는 오라클이 없다**(iverilog 13 이 struct 타입 localparam 에서 syntax error, positional 로 바꾸면 `net_scope.cc:449` **abort**) · **성능은 양방향으로 뒤집혔다**: 서드파티 다섯 기하평균 **1.61×**(sha256 2.89 · biriscv 1.88 · aes 1.74 · picorv32 1.48 · darkriscv **0.78**), 도는 일곱 전체는 1.30 ⇒ ⭐ **우리가 쓴 keccak 둘이 평균을 끌어내리고 있었다 = 유리한 벤치가 아니라 가장 어려운 벤치** · ⚠️ 지는 둘(keccak-arr 0.53 · darkriscv 0.78)의 **공통점은 아직 안 쟀다** — 다음 성능 계측은 darkriscv 부터 · 도구 = `crates/corpus-runner`(stub → 실물 · **의존성 0** · 매니페스트는 Rust `const` 테이블 · 거절이 **일급 상태** `Expect::Refused{diag}` 라 known-gap / DRIFTED / PROMOTED / **loud→silent-wrong REGRESSION** 을 구분) · ⚠️⚠️ **적대 soundness 렌즈 BLOCKING 넷, 넷 다 도구가 자기 목적을 배신하는 모양** — ⓐ ⭐⭐ **`Grade::Promoted` 가 도달 불가능**: `expect_exit` 를 `expect` **옆의 자유 필드**로 뒀고 거절 행 셋은 *거절하면서 내는* 코드라 `1` 이었다 ⇒ 갭이 닫히면 vita 는 0 으로 나가 동등 비교가 깨지고 **"was loud, now crashes" 빨간불**이 된다 = **코퍼스가 존재하는 이유인 그 사건이 거짓 문구로 실패 보고** ⇒ 값을 타입 안으로 = **`Expect::Runs { exit }`**(거절 행엔 적을 자리가 없다) ⓑ **여덟 행이 다른 기계에서 복원 불가**: 테스트벤치가 전부 1st-party 인데 `bench/*` 가 통째로 gitignore 였고(핀된 SHA 는 **상류 RTL** 을 복원하지 **하네스**를 복원하지 않는다) 존재 판정이 `cwd.is_dir()` 라 `fetch` 직후 **실행되고** `cannot read 'tb.v'` 가 **"newly refused" 회귀 여덟 개**로 채점된다 ⇒ gitignore 를 **allow-list** 로 뒤집어 하네스 30파일 커밋 + 존재 판정을 **소스와 런타임 데이터**로 · 곁가지로 펌웨어 둘이 실은 상류 것이라 `prepare.sh` 재생성/클론 참조로 ⓒ **파이프를 안 비웠다** — `verilog-axi` 는 2×2 크로스바 하나로 **19,238 B** 를 쏟고 macOS 파이프는 16 KiB 시작이라 넘기면 교착 ⇒ 예산 600초 후 `Timeout` = **정직하게 거절하는 설계를 "was loud, now hangs" 로 무고** ⇒ 양 파이프를 **스레드로** 드레인 + stdout/stderr 분리 ⓓ `cargo fmt --check` 실패 · ⚠️ NIT 열셋 중 ⭐⭐ **내가 ENGINEERING_RULES 에 ★★★ 로 적은 *"호출자가 순차 측정을 할 방법이 없다"* 가 거짓**(`measure(&jobs[0..1])` 두 번이면 순차) — 하필 *"규칙은 타입이어야 한다"* 를 주장하는 문단에서 타입이 아닌 것을 타입이라 불렀다 ⇒ 정정, 그리고 그게 **진짜로 가능했던** 자리는 ⓐ 였다 · 비결정성 검출기가 `Grade::Ok` 뒤에 갇혀 **죽은 코드**(다이제스트가 갈리면 `Mismatch` 로 은퇴하므로 도달 불가) ⇒ 흔들리는 툴이 **일관되게 틀린 툴로** 보고되던 것 수정 · `--reps 3` 이 샘플 **둘**을 주던 것(“median” 이라 적힌 평균) ⇒ `--reps` = **타임드 샘플 수** · **제품 코드 변경 0** · 5,821 → **5,845 green**(러너 16 + 매니페스트 위생 8) · format 29 불변  · ⚠️⚠️ **후속(같은 슬라이스, 커밋 `ee42eaf` 이후)**: 멈춰 선 differential 렌즈가 킬 시점에 남긴 **한 문장**이 최대 발견을 냈다 — ⭐⭐ **`picorv32` 워크로드가 아무것도 재고 있지 않았다**. 코어의 덧셈기를 뒤집어도(`reg_op1+reg_op2` → `+1`) 다이제스트가 **바이트 동일**(`7836648e76208dc9`) — 테스트 프로그램이 `addi`/`add`/`beq` 뿐이라 계산된 레지스터 값이 **버스에 한 번도 안 나오고**, 다이제스트가 보는 건 버스뿐이라 그건 **설계 검사의 이름을 단 PC 추적**이었다(결정성·오라클 일치·사이클 해상도는 **전부 통과**했다 — 통과 못 하는 유일한 검사가 변형이다) ⇒ 프로그램에 `sw x5,64(x0)` + 바퀴 간 `x1` 이월 추가, 새 다이제스트 `68d30f61bf9bf1d4`(iverilog 일치 · N=400000 · 7.05 s) · **계약 5번 신설**(*"설계를 건드리면 다이제스트가 움직여야 한다"*) 후 **아홉 전부 변형 확인 = 전부 움직인다** · ⚠️ **죽은 변형 주의**: verilog-ethernet 이 처음 *"안 움직인다"* 를 냈는데 결함이 아니라 **루프백이 TX·RX 에 같은 `lfsr` 을 쓴다** — CRC 를 바꾸면 TX 가 틀린 FCS 를 붙이고 RX 가 정확하다고 검증해 상쇄된다(RX 쪽만 건드리니 즉시 움직임) ⇒ **변형은 비대칭이어야 한다** · picorv32 비가 1.48 → **1.44**, 서드파티 기하평균 1.61 → **1.60** · 상세 = [study/03](study/03-workload-corpus.md)
- `4.5.368` **canonical `Value` 를 재수립하지 말고 주장하라 — 그리고 "스위트가 전부 통과했다" 는 증명이 아니다** · `Value::resize` 의 no-op 팔(`new_width == self.width`)이 `mask_top()` 을 무조건 불러 **이미 성립하는 불변식을 재수립**하고 있었다(평면이 정확히 `nwords(width)` 워드 · top 워드에 `width` 위 비트 없음) · ⭐ 자리 특정 = 콜 귀속: `mask_top` self **15.9%** 중 **40.6% 가 바로 이 호출** ⇒ ≈6.5% · ⭐⭐ 호출부 **31곳 중 30곳은 생산자**(raw 워드에서 값을 만들며 불변식을 *세운다*) — 이 하나만 **소비자**였다 ⇒ 호출을 `debug_assert!(is_canonical())` 로 바꿔 **부담을 생산자에게** 옮겼다 · ⚠️⚠️ **적대 soundness 가 그 불변식을 엔진으로 반증했다(BLOCKING)**: `eval/sysfunc.rs` 의 `$realtobits` 가 `v.width = 64` 를 찍으면서 평면은 **인자 폭** 그대로 둔다 ⇒ `$realtobits(<128비트>)` 가 non-canonical 을 만들어 **디버그 빌드를 패닉**(release 는 PRE 와 동일하게 정상 = correct→loud, 사다리의 금지 방향) — ⭐⭐ 내가 증거로 삼은 *"5,812 테스트에서 발화 0"* 은 **증명이 아니라 커버리지 진술**이었고, 제거된 방어 검사에는 **생산자 전수 census** 가 필요하다는 것이 이 슬라이스의 교훈 · 고침은 그 생산자 한 줄(`v.mask_top()`) + 곁가지로 differential 이 찾은 `arena.rs` 의 zero-width OOB arm(도달 불가지만 불변식이 **side condition 에 기대면 안 된다**) · ⚠️ differential 은 **~840 PRE/POST 쌍**(assert 살아있는 debug 로 패닉 사냥 · >128비트 Heap·dyn/queue/assoc·class·struct·string·`$readmem`·force/release·real↔int·CRV·SVA·fork·VCD/FST·계층 쓰기·OOB 배열 읽기)에서 **패닉 0 · 차분 0** — 두 렌즈의 상보성이 실증된 자리(differential 이 CLEAN 인데 soundness 가 code-path 로 잡았다) · ⚠️ NIT: 성능 6.5% 귀속과 11–13% 벽시계는 **두 측정**이고 나머지 갭은 이론(호출 제거로 `Value` 가 escape 하지 않아 레지스터에 남는다) · **순차 A-then-B 타이밍이 picorv32 에서 가짜 +12.5%** 를 냈다 ⇒ PRE/POST **인터리브** · keccak_f_arr **3.75 → 3.34 (−11.1%)** · keccak_f **1.92 → 1.66 (−13.2%)** · flat +0.4% · picorv32 −0.9%(노이즈) · anchor·VCD·FST 바이트 동일 · 5,812 → **5,821 green** · format 29 불변
- `4.5.367` **frame part-select 쓰기의 per-bit 루프 — 그리고 S0 가 arena 를 정확히 가격했다** · ⭐ 착수는 측정이었다(S0): keccak_f_arr 런타임의 **65.0%** 가 `run_frame_call` 안(콜 귀속 · `/usr/bin/sample` 5,318 작업 샘플)이고 run.json 이 `able 1/4 · frame_bodies 3`(flat 은 `2/4 · 0`) — **15.8× 차이가 frame body 3개와 정확히 상관** · ⚠️⚠️ **계측이 리뷰의 메커니즘 주장을 반박**했다: `wprog.rs:441` 의 frame-local decline 게이트는 keccak·picorv32 둘 다에서 **한 번도 발화하지 않는다**(`frame_decline=0`) — `wprog::compile` 이 **모듈 프로세스 body 에만** 호출되기 때문이고, `WProg::run` 은 값을 `arena.buf[slot]` 으로 읽는데 frame local 엔 슬롯이 없다 ⇒ **arena 가 진짜 선행조건**(6–10주 · 상한 2.33×)임이 가격됐고 이 슬라이스는 그 65% 안의 **bounded 한 조각**만 가져갔다 · 결함 = frame slot(과 dyn-array 원소)의 part-select 쓰기가 값을 **한 비트씩** 예치하는데 그 루프가 동시에 IEEE §11.5.1 의 **범위 밖 DROP** 을 구현한다 ⇒ 창이 net 안에 완전히 들어갈 때만 word-parallel `replace_bits`, **else 는 pre-slice 루프 verbatim** · ⚠️⚠️ **`copy_bits` 를 그대로 쓰면 silent-wrong**: 그 함수는 대상 범위가 **0이어야** 하고 비트를 **OR-merge** 하는데 part-select 쓰기의 대상은 슬롯의 **현재 값**이다(`8'hF0` + `8'h0F` = `8'hFF`) ⇒ `clear 후 copy_bits` = `replace_bits` · 적대 2렌즈 **BLOCKING 0** · differential 246칸 + 200칸 퍼즈에서 **PRE≡POST**, iverilog 159/159 · soundness 가 clear 창과 write 창이 **정확히 같음**을 구성으로 증명 · ⚠️ NIT 넷 반영: ⓐ **리뷰 스냅샷이 debug 바이너리였다**(성능 슬라이스인데 — soundness 가 +88% picorv32 "회귀" 를 재다가 발견) ⓑ `replace_bits` 는 `copy_bits` 가 `dst.val[dw]` 를 **직접 인덱싱**하므로 `set_vu` 와 달리 **할당을 늘리지 않는다**(문서화 + `debug_assert`) ⓒ 게이트가 두 파일에 **손으로 복사**돼 있었다 ⇒ `window_in_range` 한 철자(§4.5.359 모양, 여섯이 되기 전 둘에서 잡음) ⓓ 성능 주장의 **메커니즘**: `set_vu` 가 인라인되지 않아 self 6.6% 가 아니라 **leaf 12.8% 를 더한 ~19%** 가 표적이다 · **keccak_f_arr 4.49 → 3.79 s = −15.6%**(release · best-of-3 · 첫 런 폐기) · keccak_f/flat/picorv32 **불변** · 4-way anchor 불변 · examples+bench **8/8 바이트 동일** · 5,806 → **5,812 green** · format 29 불변
- `4.5.366` **상수 도메인이 정확히 64비트에서 unsigned 를 잃는다 — 그리고 비교를 고치자 시프트가 드러났다** · `localparam L = ((64'd1 - 64'd2) > 64'd0) ? 111 : 222;` 가 vita **222** / 두 오라클 **111** · ⭐ **63비트 쌍둥이와 런타임 철자는 이미 정확**했다 = 내부 판별자, 상수 도메인만의 결함 · 근인 = `eval_const_env_at` 이 값을 문맥 폭·부호로 **마스킹해 정규화**하는데 `masking = ctx_w > 0 && ctx_w < 64` 이라 **정확히 64에서 항등** ⇒ unsigned 값이 최상위 비트를 쥔 채 남고 **부호 민감 연산**이 음수로 읽는다 · ⭐ 규칙은 하나(`const_i64_is_unsigned_at`)이고 폭 인식 walk 의 **소비자 넷**이 그것을 묻는다: 순서비교(피연산자 쌍의 `(w, cs)`) · `/`·`%`(둘러싼 문맥) · 두 시프트 · leaf 재해석 · ⚠️⚠️ **적대 soundness 렌즈가 BLOCKING 넷** — ⓐ ⭐⭐ **`>>>` 도 부호 민감**(§11.4.10: arithmetic 은 왼쪽 피연산자가 signed 일 때뿐이고 §11.6.1 이 이미 unsigned 로 변환한다)인데 안 덮었고, **비교 리다이렉트가 그 잠복 결함을 드러내** 14칸이 correct→silent-wrong 이 됐다(두 오라클은 declared-signed 왼쪽 피연산자에서도 logical 을 답한다) ⓑ `w >= 64` 는 **추측**이다 — >64 는 i64 가 이미 절단했고 두 방향이 서로 반대로 틀린다(carry 모양은 signed 읽기가, 뺄셈 모양은 unsigned 읽기가 맞다) ⇒ **`w == 64`** 로 좁힘, 형제 `const_unsigned_selfdet` 이 이미 그 자리에 선을 그어 두었다 ⓒ ctx≥64 에서 walk 가 **`leaf_into_ctx` 를 안 돈다** ⇒ 좁은 **signed** leaf 가 부호확장된 채 도착해 u64 경로가 그 확장을 크기로 읽는다(`logic signed [7:0] P=-100` 이 156 이 아니라 0xFFFF…FF9C 로 나눠졌고, 그 위에 세운 `$bits` part-select 폭이 **loud→silent-wrong**) ⇒ 그 문맥에서도 leaf 를 정규화 ⓓ clippy `double_parens` 로 `-D warnings` 게이트 실패 · 120칸 4-way(iverilog·verilator·PRE·POST) **오라클 분열 0 · ok→wrong 0 · wrong→ok 14 · wrong↔다른-wrong 0** · 33칸 census 비교 클래스 전부 해소 · 상수함수 본문의 `%` 는 −1 → **5**, `>>` 는 **전체 REJ(loud) → 정답** · bench+examples **8설계 `.velab` 바이트 동일** · 5,796 → **5,806 green** · format 29 불변
- `4.5.365` **real actual 이 정수 formal 의 폭을 안 받는다 — 그리고 참조 구현은 이미 트리 안에 있었다** · §13.5.3 은 호출을 **formal 선언 타입 변수에의 대입**으로 정의하므로 real actual 은 반올림(§6.24.1) 후 **formal 폭으로 좁혀야** 하는데 vita 는 반올림만 했다(`function integer f(input byte k); f = k;` 를 `f(300.0)` 로 → **300**, 두 오라클 **44**, exit 0) · ⚠️⚠️ **큐 문구의 절반이 STALE 이었다** — `%0d` 반올림·등가·`integer` formal·automatic 함수/task 는 **이미 정확**했고 비트셀렉트/비트연산/`%h` 는 §2 가 아니라 **§3(E3009 loud)** · ⚠️ 그리고 **첫 프로브가 나를 속였다**: `input int` + `3.0`(폭에 들어맞는 값)만 봐서 frame 경로가 정확해 보였다 — **폭보다 큰 값**을 넣어야 보인다 · ⭐ **참조 구현이 트리 안에 있었다**: inline TASK 는 입력 actual 을 **formal-폭 지역 net 에 copy-in** 하므로 저장이 `coerce_assign` 을 탄다 ⇒ 나머지 셋을 거기 맞췄다(공유 헬퍼 `coerce_real_actual_to_formal` 하나) · 184칸(10 formal × 6 real actual × 3 라우팅) **mismatch 54 → 0** · 1,170칸 스윕 **1170/1170** iverilog 일치 · ⚠️⚠️ **적대 2렌즈 3라운드 · BLOCKING 넷** — ⓐ `cast_operand_is_real` 이 **철자로** 판정(패키지 본문의 bare `g()` 는 `P::g` 인데 술어는 모듈 레벨 `real g` 를 찾는다) ⇒ **정수** actual 이 f64 왕복으로 2⁵³ 초과를 잃었다(correct→silent-wrong) ⇒ **값 기반 `expr_is_real`** 로(⭐ 잃는 것 없음 — 유일한 차이인 real-반환 `Call` 은 `expr_is_repeatable` 이 이미 거절) ⓑ frame-task 게이트가 **net kind** 라 약해 `string` INPUT formal 을 1비트 캐스트로 파괴 ⇒ 형제 둘과 **같은 철자** ⓒ `input time signed` 가 세 경로에서 correct→wrong(뿌리 = `kind_signedness` 가 명시 한정자를 버려 **formal net** 이 unsigned · pre-existing) ⇒ **좁은 decline**(집합 차 = 정확히 `{Time}` 임을 렌즈가 증명) ⓓ ⭐⭐ **`lower_real_to_int_cast` 의 ≤32비트 가지가 `$rtoi(e±0.5)`** 라 [2⁵²,2⁵³) 홀수에서 **tie-to-even**(2⁵²+1 → 2, 두 오라클 1) — >32 가지엔 정확 구성이 이미 있었고 자기 주석이 그 함정을 적어 두었다; 내 변경이 30칸을 그 가지로 새로 보내 **wrong↔wrong 맞바꿈**이 될 뻔했다 ⇒ 정확 구성을 **hoist 해 두 가지가 공유** · ⭐ **곁수확**: pre-existing `int'()`/`byte'()`/`shortint'()` 결함 40칸이 함께 해소(`0.49999999999999994` → 0 포함) · ⚠️ 렌즈가 잡은 마지막 NIT = 내가 `range_to_dims` 를 두 번 불러 **진단이 두 번** 찍혔다(그 함수는 emit 한다) ⇒ 첫 호출의 부호를 바인딩 · bench+examples **8설계 `.velab` 바이트 동일**(정수 actual 42설계 포함 50/50) · 5,785 → **5,796 green** · format 29 불변
- `4.5.364` **구조적 지연의 값 fold 가 리터럴 전용이었다 — 그리고 리뷰가 내 수정의 수정을 세 번 잡았다** · `parameter D = 7; assign #(D) y = a;` 가 **t=1 에 전파**(두 오라클 t=8) · exit 0 · 진단 0 — `fold_ca_delay` 가 부르던 `const_delay_ticks` 의 정수 가지가 **리터럴 전용**(`IntLit`/`(…)`/unary ± 뒤 `_ => None`)이고 **호출자가 `None` 을 조용한 기본값(딜레이 없음)으로 소비**한다 · ⭐ 큐엔 *"파라미터"* 한 줄이었고 census 는 **레인 셋**: 정수(**자기결정 폭 · unsigned 읽기** — `#(4'd15+4'd1)`=0, `parameter signed [7:0] D=-8'sd1`=**255**, 둘 다 2-오라클) · real(`parameter real RD=2.5`) · **TimeLit**(`#(5ns)`) · ⭐ 정수 레인은 `$clog2` 의 헬퍼를 **추출해 공유**(`const_unsigned_selfdet`) — 같은 위치가 §20.8.1 과 §28.16 에서 같은 철자라 두 소비자가 문맥에 합의한다 · ⭐ 파서가 게이트 프리미티브·net-decl 지연을 전부 `ContinuousAssign` 으로 desugar 하므로 **한 퍼널**(assign/wire/buf/and/or/xor/bufif1 · rise-fall-turnoff 사이드카 포함) · ⚠️ 절차적 `#delay` 는 **opt-in 밖에 둔다**(그 레인은 amount 를 런타임 식으로 낮추므로 원래 제한이 없었고, region 판정만 넓히면 `#(ZERO_PARAM)` 이 Active→Inactive 로 바뀐다 = 결함 없는 스케줄링 변경) · ⚠️ **zero-rise 는 pre-slice 모양 유지**: `Some(0)` 은 CA 를 delayed 레인으로 보내고 vita 의 zero-tick write 는 **자기 타임스텝의 Postponed 리전 뒤에** 착지한다(`$strobe` 실측 — 두 오라클 1, vita 0) ⇒ `#(ZERO_PARAM)` 을 `Some(0)` 으로 만들면 맞던 답을 그 lag 와 맞바꾼다 · ⚠️⚠️ **적대 2렌즈가 4라운드에 걸쳐 BLOCKING 을 넷 냈고 셋은 내 라운드-2 수정이 만든 것이다**: ⓐ 정수 도메인을 먼저 물어 **정확히 정수인 `parameter real R = 11` 의 i64 쌍둥이**가 잡혀 `#(R/2)` 가 5(두 오라클 6 · vita 자신의 절차적 레인도 6) ⇒ `param_real_value` 가 **이미 적어 둔 순서**(real 먼저)로 재배치 ⓑ 그런데 real-먼저를 **return** 으로 쓰자 real 도메인에 없는 연산자(`%`·비트·shift·`$clog2`·call)가 통째로 조용한 기본값이 됐다(다섯 칸이 correct→silent-wrong) ⇒ **fallback 필수** ⓒ 게이트가 쓰던 `expr_mentions_real` 은 `real_param_val` **단독 walk** 라 안쪽 `localparam R=9` 가 바깥 `parameter real R=5` 를 가려도 real 이라 답한다 ⇒ **`shadow_correct` 옵트인** 신설(결합 바인딩 walk · 기존 호출자 셋은 리터럴 `false`) — soundness 렌즈가 그 술어가 **엄격한 좁힘**(true→false 방향으로만 다르다)임을 코드로 증명 ⓓ 새 TimeLit 레인이 오버플로에 **decline**(형제 레인은 saturate)해 `#(20000s)` 가 즉시 발화 ⇒ saturate · ⚠️ **적대 렌즈의 BLOCKING 하나는 반박했다**: 지연 멀티드라이버 E3001 을 *"correct→loud 회귀"* 라 했으나 **10 ns 격자가 조용히 버려진 2 ns 지연을 못 본 것**(1 ns 격자: PRE `t=11 bus=1` / iverilog `bus=z`) ⇒ silent→loud = 사다리 **상승**, 렌즈도 세 구성으로 반례를 시도한 뒤 철회 · 70칸 3-오라클 **FIXED 51 · REGRESSION 0 · SAME-OK 15 · DIVERGENT 3 · STILL-WRONG 1**(런타임 변수 지연 = 범위 밖) · 353설계 드리프트 스캔이 라운드마다 **수정 표적만** 움직였음을 확인 · bench+examples **8설계 `.velab` 바이트 동일** · 5,770 → **5,785 green** · format 29 불변
- `4.5.363` **파라미터 셀렉트가 상수 도메인에 아예 없었다 — 그리고 값도 폭도 이미 거기 있었다** · `localparam logic [31:0] W = 32'h34; logic [W[7:0]-1:0] v;` 가 **1비트** net 을 exit 0 로 선언(두 오라클 52) — ⭐ **런타임 레인은 이미 정확**했고(`$display("%0d", W[7:0])` 세 툴 다 52) 선언 폭도 `param_range` 에 이미 있었다 ⇒ 없던 건 `const_eval_in_scope` 의 **arm 하나**(그 함수의 유일한 `BitSelect` arm 은 const-**배열원소** 조회라 스칼라 param 에서 declines) · ⚠️ **캐스트 뭉치의 AST self-폭 패스에 막히지 않는다**(셀렉트의 self-width 는 `|msb-lsb|+1` = 하강 불필요) · ⭐⭐ **두 개의 decline 규칙이 슬라이스의 본체다**: ⓐ **방향/선언 LSB** — `[39:8]` 도 `[0:31]` 도 저바이트가 52 이므로 `[w-1:0]` 을 가정하면 silent-wrong 을 **다른 silent-wrong** 과 맞바꾼다 ⇒ 런타임 하강이 쓰는 **바로 그 `param_sel_range`** 를 쓴다(곁수확: `param_decl_range` 에 오름차순 arm 이 생겨 **런타임** `A[26]` 이 0→1 로 고쳐졌다 = 2-오라클 silent-wrong) ⓑ **폭 provenance** — 무타입 파라미터의 폭은 값에서 *추론*될 수 있어(`localparam W = ~8'hCB` → 32 기록, 실제 8) 그것을 믿으면 **263비트 net** 이 나온다 ⇒ `param_decl_width_opt(declared_only)` 로 **선언/타입/리터럴 유래만** 채택 · ⚠️⚠️ **적대 3렌즈가 다섯을 잡았고 그중 둘이 BLOCKING** — 첫 설계는 wrap 가드를 **공유 평가기**(`const_eval_in_scope` 의 Binary arm)에 넣었는데 그 함수는 **폭 규칙이 반대인 소비자 둘**이 공유한다: 파라미터 **값**은 자기결정이 아니라 **대입**이라 `localparam int Q = W[7:0]+8'd240` 이 36(두 오라클 292 · PRE 는 loud) = **loud→silent-wrong**, 게다가 가드가 **한 arm 에만** 닿아 `~W[3:0]` 이 6(두 오라클 12) = silent→**다른** silent, 게다가 셀렉트 **없는** 8000항 식이 **121배** 느려졌다(Θ(n²)) ⇒ 규칙을 **소비자(선언 range bound = 정의상 자기결정)** 로 옮기니 셋이 한꺼번에 사라졌다 · 그 밖에 셀렉트 **자신의 인덱스**도 자기결정(`W[4'd15+4'd1]` 은 bit 0)·`[5:5]` 동일 끝점이 하강 base 에서만 거절되던 방향표·음수 선언 bound 의 `max(0)` **거짓말**(오름차순 `[-2:3]` 이 correct→silent-wrong) 수정 · 93칸 3-오라클 **AGREE3 29 → 81 · FIXED 52 · 회귀 0 · 컨트롤 28/28 불변** · bench+examples 17 산출물 **바이트 동일** · 5,753 → **5,770 green** · format 29 불변
- `4.5.362` **real 지수는 밑수의 문맥을 회수한다 — 그리고 직전 슬라이스의 판정이 틀렸다** · §4.5.361 은 `'1 ** r` 을 *"iverilog 가 480 을 내니 vita 의 480 이 맞다"* 로 닫고, 후보 고침을 **iverilog 일치도**로 채점해 기각했다(267 → 247) — ⚠️⚠️ **그 점수가 재고 있던 것이 바로 누수였다** · ⭐⭐ 실격 질문은 한 개였고 내가 묻지 않았다: **대입 폭이 없는 곳에 같은 식을 보내라** — `real x = ('1+4'h0) ** r` 은 **세 시뮬레이터 전부 871.4213**, §11.4.9 가 연산자의 정의로 지정한 `$pow(('1+4'h0), r)` 도 **세 시뮬레이터 전부 871.4213** ⇒ 밑수는 15 이고, **밑수의 값이 결과를 나중에 담을 변수의 폭에 따라 달라질 수는 없다** ⇒ 480 은 목적지의 역류 · ⭐ 규칙은 이미 `+`·`-`·`*`·`/` 가 지키고 있었다(같은 네 밑수가 `+ r` 에서 **4·18·2·18**, `** r` 에서 **480·480·480·480**) · 게이트를 **연산자형 vs `$pow` 형** 동치로 세워 192쌍 **115 → 192**(옮겨간 77칸이 **전부 기준값에 정확히 착지**) · ⚠️ 순수-정수 `**` **384칸 바이트 동일** = Table 11-21 의 나머지 절반 보존 · ⚠️ 적대 리뷰가 **이웃의 가드를 그대로 베낀 것**을 잡았다(`!expr_contains_fill(rhs)` — fill 은 real 이 못 되지만 fill 을 품은 식은 real 이 된다) · bench·examples 8설계 stdout+stderr+VCD **바이트 동일** · 곁수확 = **static function 의 모듈 net 쓰기가 조용히 사라진다**(2-오라클) · 5,748 → **5,753 green** · format 29 불변
- `4.5.361` **분열 넷을 판정했고 셋은 no-op, 넷째는 지어서 되돌렸다** · §2-1d(셋)와 §2-2b(하나)는 *"2-오라클 합의가 없으므로 착수 불가"* 로 큐에 묶여 있었다 — 재보니 **넷 다 판정 가능했고, 판정의 형태가 "어느 답이 맞나"가 아니라 "어느 오라클이 이 자리에서 오라클이 아닌가"** 였다 · ⭐⭐ **오라클 실격은 vita 를 안 보고 증명된다**: iverilog 는 `s="ab"` 에서 `s<"ab"`·`s<"aa"`·`s<"zz"` 를 **전부 1** 이라 답하고(셋이 동시에 참일 수 없다), `$itor` 은 unsigned 와 signed `longint` 의 같은 값을 **똑같이 8** 로 읽는다(부호 해석이면 갈려야 한다) ⇒ 둘 다 **자기 답들 사이의 모순**이라 vita 와의 비교가 필요 없다 · ⚠️⚠️ **넷째(`'1 ** r`)는 고쳤다가 측정이 기각했다** — 근거는 vita 자신의 불일치(fill 쓴 밑수 480 / 안 쓴 같은 값 0)였고 그 불일치는 진짜지만, 288칸 PRE-3-way 가 iverilog 일치 **267 → 247**, 옮겨간 35칸이 **전부 멀어짐**을 냈다 ⇒ 되돌림 · ⭐ 그리고 **진단이 뒤집혔다**: iverilog 는 `(4'd15+4'd1) ** r` 을 **1024** 로 읽으니 대입 문맥이 밑수에 **닿는다** ⇒ vita 의 불일치는 fill 경로가 문맥을 더 줘서가 아니라 non-fill 경로가 **덜 줘서**다(§2-1f 신규) · ⭐ `%` on real 은 두 툴이 **서로 다른 답**을 내므로(fmod 1.5 / 0) 정의된 의미가 아니다 ⇒ loud 유지 · 판정 4건 전부 `oracle_split_rulings.rs` 로 핀 — 이 파일의 목적은 기능이 아니라 **미래의 스윕이 "한 툴과 다르네" 로 silent-wrong 을 만드는 것을 막는 것** · 곁수확 둘(`$itor(<real>)` silent-wrong · `string'(<integral>)` 파서 갭) · 5,744 → **5,748 green** · format 29 불변 · **제품 코드 변경 0**
- `4.5.360` **선행조건이 "스코프 스냅샷"이 아니라 문자열 하나였다** · §4.5.357 이 §2-1e 잔여 4칸에 붙여 둔 선행조건은 *"resolve 시점에 쓸 수 있는 lowering 스코프"* 였다 — 그건 **결과의 진술**이었고, 실제로 재보니 vita 의 이름 해석은 `cur_prefix` 에서 바깥으로 걷는 walk + **elaboration 내내 살아 있는 FQ-keyed 테이블**이다 ⇒ 평범한 이름에 필요한 "스코프" 는 **prefix 문자열 하나**뿐이고, pending 레코드에 담아 복원하면 끝난다 · ⭐ 술어를 리터럴/연산자에 **단일 세그먼트 `Ident`** 까지 넓혔다(점 있는 경로와, push/pop 되는 상태로 해석되는 것 — 인라인 formal 의 subst·frame window — 은 여전히 제외: 그것들은 resolve 시점에 없다) · ⚠️⚠️ **재-lower 전에 읽기 전용으로 검증한다**: `lower_expr_ctx` 는 미해결 이름을 **진단을 뱉어서** 알리므로 실패한 재-lower 는 되돌릴 수 없다 — 틀린 값이 **거짓 loud** 로 바뀌면 값이 틀렸어도 회귀다 ⇒ `every_name_resolves` 가 **하강이 쓰는 그 walk** 로 먼저 확인하고, 하나라도 못 찾으면 `lower_expr` 의 IR 을 그대로 둔다 · 70칸 **residue 4 → 0 · bad 0** · 520칸 계층 스윕 **440 agree · MISMATCH 0** 불변 · 새 회귀 1건 + 기존 2건 갱신 · 5,743 → **5,744 green** · format 29 불변
- `4.5.359` **음수 bound 의 남은 선언 스코프 — 여섯 복사가 아니라 한 함수** · §4.5.350 이 모듈 스코프 net 만 고치고 나머지를 **의도적으로** 남긴 자리(폭 opt-in 과 select 정규화 기록은 **함께** 켜야 하고, net 을 만드는 자리가 한 곳이 아니다) · 큐엔 **셋**(포트·서브프로그램 지역·클래스 속성)인데 census 는 **여섯**(+함수 **반환 타입** · 함수 **형식인자** · **static task 지역** — automatic/static 이 별개 collector 라 두 철자를 다 돌려야 보였다) · ⭐ 자리 특정은 `#[track_caller]` + 임시 계장으로 **clamp 하는 호출부를 직접 찍어서** 했다(28개 호출부 중 5개) · ⭐⭐ 고침은 **한 함수**: 모듈 스코프 선언에 인라인돼 있던 20줄을 `record_declared_bounds`/`record_declared_bounds_for` 로 뽑고, 각 자리는 `declared_odd_bound` 로 opt-in 을 묻고 방금 만든 net 으로 기록한다(복사했으면 **한 규칙의 여섯 철자**가 됐고 다음 선언 자리가 일곱 번째로 잊었을 것이다) · `func_return_dims` 는 호출자가 셋이라 **opt-in 변형**을 따로 두어 frame 경로만 켰다(나머지는 리터럴 `false` = 바이트 동일) · 78칸(13 스코프 × 6 range) **전부 iverilog 일치 · 차이 0** · ⚠️ **클래스 속성만 남겼다**(§2-3b): 클래스 필드는 net 이 아니고 정규화 맵은 NetId 키라 **기록할 자리가 없다** — 폭만 켜면 §4.5.350 리뷰가 잡은 하강이 된다(오라클도 하나뿐: iverilog 는 assertion 으로 죽는다) · 새 회귀 6건 · 5,737 → **5,743 green** · format 29 불변
- `4.5.358` **real 타깃은 폭을 빌려주지 않는다 — 같은 덫의 세 번째, 그리고 엔진에서 처음** · ⭐ 큐 §2 가 적어 둔 *"mixed-real 잔여 셋"* 은 **재-census 에서 이미 전부 두 오라클 일치**였다(이후 슬라이스가 닫았다) — 대신 33칸 census 가 낸 **새 칸 하나**가 60칸 스윕에서 **12칸 클래스**로 자랐다 · §6.12 상 real 은 비트 폭이 없으므로 대입 규칙 `width = max(lhs, self(rhs))` 에서 왼쪽이 줄 게 없는데, `lvalue_width` 는 real 의 **저장 폭 64** 를 답한다 ⇒ **unsigned 이면서 64보다 좁은** 피연산자가 값이 바뀐다(`byte unsigned b=8; r=-b;` 두 오라클 **248** / vita **2⁶⁴−8**; signed 는 부호확장이 값을 보존해 멀쩡했고 그래서 숨어 있었다) · ⭐ **식 자체는 늘 옳았다** — `$display("%0d", -b)` 는 PRE 에도 248 ⇒ 자리는 연산자가 아니라 **대입 경계** · ⭐⭐ **엔진이 같은 질문을 세 곳에서 한다**: 스케줄러 `eval_for_lvalue` · 네이티브 `k_eval_for_lvalue` · **네이티브 프로그램 컴파일러**(`backend.rs`) ⇒ 앞의 둘만 고쳤을 때 **`#1` 이 있으면 맞고 없으면 틀린** 상태가 됐다(딜레이 없는 프로세스가 네이티브 프로그램으로 컴파일된다) — 그래서 술어를 **이름 있는 공유 함수**(`width::lvalue_targets_real`)로 뒀고, 테스트도 **딜레이 유무 양쪽**을 돈다 · ⚠️ 내 손계산이 또 틀렸다(`b - 100` 을 164 로 적었는데 `100` 이 32비트 unsized 라 `2³²−92` 가 맞다 — 오라클에서 가져왔다) · 192칸(12타입 × 8식 × 딜레이 2) **전부 iverilog 일치 · 차이 0** · 남긴 것 = `$itor(-unsigned)` **오라클 분열**(§2-2b) · 새 회귀 5건 · 5,732 → **5,737 green** · format 29 불변
- `4.5.357` **제약은 모양이 아니라 스코프였다 — 지연 fill 을 bare 에서 scope-free 로** · §4.5.355 가 남긴 §2-1e 를 70칸 census 로 재니 **24칸**(전부 계층 전용 · 로컬 쌍둥이는 전부 정상) · ⭐ **먼저 조기 해석을 시도했고 죽은 길임을 계측으로 확인**: `resize_fill_rhs` 에 임시 계장을 넣어 lowering 시점 `hier_lookup` 을 찍었더니 **항상 `None`** — 자식 인스턴스의 net 은 그때 정말로 없다(지연은 보수적인 게 아니라 필수) · ⭐⭐ 그래서 제약을 다시 진술했다: *"bare fill"* 이 아니라 **"스코프를 안 보는 식"** ⇒ `scope_free_fill_expr` = 리터럴과 그 위 연산자만(구조적 화이트리스트 · `_` arm 없음) · ⚠️⚠️ **두 번째 아키텍처 제약이 여기서 나왔다**: 새 부분식의 루트 노드를 옛 슬롯에 복사했더니 `sim-ir/selfwidth.rs` 가 *"child 8 not yet computed for parent 3 — arena not post-order"* 로 패닉 ⇒ **arena 는 post-order** 이고 자식 없는 `Const` 하나만 그렇게 옮길 수 있다(그게 §4.5.355 가 통했던 이유) ⇒ 슬롯이 아니라 **문장의 rhs 참조**를 새 id 로 돌린다 · 70칸 **FIXED 20 · bad 0 · residue 4** · ⭐ 곁수확: §4.5.355 의 520칸 스윕 잔여 **MISMATCH 4 → 0** · 남은 4 = `c ? '1 : 1'b0`(RHS 가 net 을 읽는다 — §2-1e 선행조건 그대로) · 새 회귀 2건 · 5,730 → **5,732 green** · format 29 불변
- `4.5.356` **case 의 §12.5 공통 max — 규칙이 한 방향으로만 흐르고 있었다** · 큐엔 *"`case ('1)` 셀렉터가 1비트로 남는다"* 였고 census 가 **아홉**(`casez`/`casex`/괄호 포함), 이어서 420칸 스윕이 **나머지 절반**을 냈다 · ⭐ vita 는 §12.5 의 절반만 갖고 있었다 — `lower_case_label` 이 **셀렉터의 폭을 라벨로** 밀어 주는데, 그건 셀렉터가 폭을 가질 때만 규칙 전체다. bare fill 은 폭이 없으니 `case ('1) 8'hFF: a=1; 1'b1: a=3;` 가 1비트 arm 을 골랐다(두 오라클은 8비트 arm) · ⭐⭐ **양끝이 다 움직여야 한다**: `case ('1) 8'h01: ; '1: ;` 는 두 오라클이 `'1` arm 을 고르는데, 셀렉터만 넓히면 **default 로 떨어진다**(맞던 답을 다른 틀린 답으로 바꾸는 것 = 금지된 맞바꿈) ⇒ 넓힌 폭으로 **같은 `lower_case_label` 을 fill 라벨에 다시** 돌린다 · ⭐ **일반화는 스윕이 시켰다** — 1차 고침 뒤에도 15칸이 남았고 전부 `case (1'b1) '1: ; 8'hFF: ;` 형태(셀렉터에 fill 이 **없다**)였다: 공통 max 는 **라벨끼리도** 성립하므로 `'1` 은 8비트가 되고 답은 default 다 ⇒ 게이트를 *"셀렉터에 fill"* 에서 *"어디든 fill"* 로 · ⚠️ **real 셀렉터는 제외**(§6.12 · §4.5.354 핀 — 8비트 형제 라벨이 있어도 세 오라클이 `'1` arm 을 유지) · 420칸 3-오라클 **일치 342 → 420 · MISMATCH 78 → 0** · bench 4설계 `.vu`/`.velab` 바이트 동일 · 새 회귀 8건 · 5,722 → **5,730 green** · format 29 불변
- `4.5.355` **계층 타깃의 fill 폭 — 폭은 없던 게 아니라 늦었다** · 큐엔 **셋**, census 는 **일곱**, 적대 프로브가 **열**(+`'x`/`'z`·task 스코프) · ⚠️ **ROADMAP 의 근인 기록이 절반이었다** — 계측해 보니 레인이 **둘**이다(`0xFF000000` 전체-net · `0xFE000000` element/select) · ⭐⭐ **고침은 새 폭 규칙이 아니라 같은 질문을 나중에 하는 것**: 두 resolve 레인이 각자 결정한 chunk 를 publish 하고, `resolve_pending_fill_widths` 가 **lowering 때 1 을 답했던 바로 그 `ir_lvalue_width`** 를 다시 부른다 ⇒ 하위 케이스(전체-net·part-select·element)가 열거 없이 떨어지고, **결정적으로 `u.a[0]`(1비트 계층 타깃, 세 오라클 `...1`)이 구조적으로 보호된다** — 레인별로 짰으면 그것까지 12개 1 로 넓혔을 것이다 · ⭐ **bare fill 만 지연**하는 이유는 지름길이 아니라 **resolve 에 lowering 스코프가 없기 때문**(`(raw,kind,width)` 로 재구성 가능한 유일한 모양) · ⚠️⚠️ **자기 정정**: 중간에 *"bare 가 곧 깨진 집합 전체"* 라고 결론냈는데 그건 **12비트 프로브의 우연**이었다 — 520칸 스윕의 64비트 타깃이 `u.wide = '1+1` = 4294967296(로컬은 0, iverilog 0)을 내놓아 깨졌다 ⇒ §2-1e 로 기록(선행조건 = 스코프를 가진 재-lower) · 520칸 3-오라클 **일치 308 → 436 · MISMATCH 132 → 4** 이고 **바뀐 128칸이 전부 iverilog 일치**(변했는데 안 맞는 것 0) · 잔여 4 = §2-1e(PRE==POST) · 잔여 80 = `force`/절차적 `assign` 의 계층 part/bit-select E3009(PRE==POST · 기존 loud) · bench 4설계 `.vu`/`.velab` 바이트 동일 · 새 회귀 8건 · 5,714 → **5,722 green** · format 29 불변
- `4.5.354` **`lower_expr_ctx` 가 `lower_expr` 의 도메인 라우트를 전부 잃었다 — 고침의 대부분은 삭제였다** · 큐엔 *"`{<string>,'1}` 이 무한 루프 ⇒ **loud 화**"* 라고 적혀 있었고, 착수 census 가 **1 → 6** 으로, 적대 리뷰가 다시 **6 → 10** 으로 늘리는 동시에 **계획을 뒤집었다**(loud 가 아니라 correct-support) · 뿌리는 hang 이 아니다 — `lower_expr_ctx` 는 문맥 전파 6종에 대한 **두 번째 dispatch** 이고 `lower_expr` 이 가진 도메인 라우트를 **하나도** 제대로 안 갖췄다 ⇒ **노드 어딘가의 fill 리터럴이 그 라우트 전부를 우회하는 스위치**였다: ⓐ string concat `{s,'1}` = **비종료**(상호 tail call — 릴리즈는 RSS 고정 100% CPU, 디버그는 스택 오버플로 rc=134) ⓑ string replicate `{2{s,'1}}` = 문자열이 비트로 붕괴해 **24'd5** ⓒ string 비교 `s < {"a",'1}` = packed(비-lexicographic) **0**, 두 오라클 **1** ⓓ handle 비교 `h == '1` = **loud→silent**(쌍둥이 `h == 1'b1` 은 E3009) ⓔ `{ {0{x}}, '1 }` = **거짓 loud**(`repl_zero_ok` 미설정) ⓕ `N=-2; {N{'1}}` = 조용히 `111111111111` ⓖ `r & '1`/`r << '1` = **loud→silent**(§6.2) + `r ** '1` 이 §11.4.9 `$pow` **라우트**를 통째로 잃어 `0`(두 오라클 3) ⓗ `r + '1` = `0`(두 오라클 **4**) — `ir_bits_of` 가 real 에 **저장 크기 64** 를 답했다 = §4.5.353 이 `ir_lvalue_width` 에서 만난 덫이 **한 층 아래** · ⓘ 같은 덫의 Ternary arm(`c ? r : '1`) ⓙ 같은 덫의 `case` 셀렉터(`case (r) '1:`) — ⓘⓙ 는 *"나머지 층을 찾아가라"* 를 나 자신에게 적용한 `ir_bits_of` **전수 grep** 이 냈다 · ⭐⭐ **적대 리뷰가 설계를 바꿨다** — ⓔⓕ 를 찾고 나서야 **Concat/Replicate arm 이 피연산자에 `ctx = 0` 을 넘긴다**(=`lower_expr` 이 하는 것과 정의상 동일)는 게 보였다 ⇒ **가드 넷이 아니라 arm 둘 삭제**가 답이고, 그 삭제가 ⓔⓕ 를 부수적으로 해소 · 진짜 가드가 필요한 건 **Binary 하나**(`binary_stops_ctx` = 해당 사이트 조건의 verbatim 사본) · ⭐ 종료성이 **구조적**이 됐다(`lower_expr_ungated`: ctx 를 떠나는 모든 호출은 **진부분식**이거나 **게이트 없는 진입점**) · 검증은 **합성 differential** — 이 족속엔 외부 오라클이 없지만(iverilog `recv_string not implemented` 로 죽고 verilator 빌드 거부 · fill 없는 `{s,8'h0F}` 도 마찬가지) *"concat 피연산자는 self-det ⇒ fill 은 정확히 1비트"* 는 두 오라클이 string-free 족속에서 확인해 주고, 문장 레벨 `string_concat_special` 과 `1'b1` 쌍둥이가 vita 안의 두 철자로 값을 고정한다 · ⭐ string 타깃 `r={s,'1}` 은 네 산출(iverilog·verilator·PRE·POST)이 **바이트 동일 `ab\x01`** — 터미널로 봤으면 넷 다 `ab` 로 같아 보였을 자리 · differential 564 fill 설계 **0 차이** + string/handle 160칸 46 변화가 **전부 의도 집합** + bench 4설계 `.velab` 바이트 동일 · 새 회귀 18건 · 5,694 → **5,712 green** · format 29 불변
- `4.5.353` **fill 리터럴이 문맥 폭을 못 받는 대입 형태 셋 — 머신러리는 완성돼 있었고 세 자리가 안 불렀다** · 큐엔 **하나**로 적혀 있었고 census 가 **셋**(ⓐ net 선언 초기화 ⓑ `force` ⓒ 절차적 continuous assign) · ⭐ **같은 구문의 두 하강 중 하나가 잊었다** — `wire [7:0] a = '1;` 은 암묵적 continuous assign 인데 옆의 `assign a = '1;` 과 답이 달랐다(00000001 vs 11111111) · ⓑ·ⓒ 는 **write-twin 스윕**이 찾았다 · 고침은 이미 있던 `resize_fill_rhs` 를 **한 줄씩 세 군데** 부르는 것 · ⚠️⚠️ **적대 리뷰가 내가 만든 correct→silent-wrong 을 잡았다**: `real` 타깃은 비트 문맥이 없는데 `ir_lvalue_width` 가 64(저장 폭)를 답해 `force r='1;` 이 1.0 → **1.84467e+19** (두 오라클 1.0) ⇒ ⭐ **가드를 호출부가 아니라 헬퍼 안에** 두니 내 회귀 2건을 막으면서 **선행 호출자의 pre-existing real silent-wrong 4건**(`e='1;`·`real d='1;`·`f[0]='1;`)을 덤으로 고쳤다 · ⚠️ 게이트 술어 `expr_contains_fill` 이 **`MinTypMax` 를 안 걸어** `(1:'1:2)` 가 전 자리에서 안 고쳐지고 있었다(같은 클래스라 이번에 수정) · ⚠️ **규칙의 반대 절반도 핀**(shift 량·`**` 지수·`&&`/`||`·인덱스의 fill 은 1비트 유지 — 안 그러면 silent-wrong 맞바꾸기) · `'x`/`'z` 곁수확(verilator 의 `0` 은 2-state 한계이지 분열이 아니다) · PRE-3-way **FIXED 17 · REGRESSION 0 · unchanged-ok 24** · 비-fill 설계는 기계적 바이트 동일(picorv32·keccak 3종 실측) · 스코프 밖 pre-existing 셋을 §2 에 기록(**계층 타깃 = 네 번째 자리·뿌리 다름** · `{string,'1}` **무한 루프** · `case ('1)` 셀렉터) · **뮤테이션 10/10 KILLED**(⭐ 단축 평가 mutant 이 살 거라는 내 예측이 틀렸다 — `clog2_repl_count` 가 죽인다: 그건 값 가드다) · 새 회귀 10건 · 5,684 → **5,694 green** · format 29 불변
- `4.5.352` **settle 가 델타마다 199개를 훑어 0개를 찾고 있었다 — 나눗셈이 게이트가 아니라 탐색이었다** · 착수는 직전 슬라이스의 추천 후보(`settle_cont_assigns` 의 72바이트 `Value`)였고 그건 진짜였다(**−2.4%**) — 다만 *"짓기 전에 나눗셈"* 규칙을 지키려 그 함수를 프로파일했더니 **같은 함수 안에 6배 큰 것**이 있었다 · `schedule_delayed_cas` 가 `for ci in 0..cont_assigns.len()` 로 전부 만지고 `delay` 없는 것을 `continue` 했다 ⇒ picorv32 는 **딜레이 붙은 cont-assign 이 0개**인데 **1,400,006 settle × 199 = 278,601,194 회**를 돌아 런타임의 **7.2%** · ⭐ **단위가 두 자릿수 틀렸다** — settle 은 타임스텝당이 아니라 **델타당**(사이클당 7) · ⓐ `delayed_ca_idx` 를 생성자에서 한 번(바디의 `let Some(d) = … else { continue }` 는 **verbatim 유지** ⇒ 사전필터가 두 번째 철자가 되지 않고, 계측이 `delayskip == 0` 으로 둘의 일치를 증명) ⓑ `write_settled` = flat store 의 **세 번째 리전**(블로킹 D1.6 · NBA §4.5.351 · settle 은 **델타당 도는 리전**) · picorv32 **2.31 → 2.07 s (−10.5%)**(반 A −8.4% · 반 B 추가 −2.4%) · 프로파일 독립 확인(분모 맞춘 재측정) `settle_cont_assigns` self **8.83% → 1.62%** · differential 14설계 × 3백엔드 × {stdout,stderr,exit,VCD,FST} **42쌍 동일** + staged(`.velab` 바이트 동일) + obs 레일 동일 · 비공허성 양 arm 발화 실측 · **뮤테이션 12발 6 KILLED / 6 등가**(⭐ m2·m11 의 생존이 곧 "의미 불변"의 증명) · 새 회귀 5건 · 적대 2렌즈 **BLOCKING 0**(유일 생존 지적 = 반 A 의 최악 케이스 **+0.36%**, retired-instructions 로 측정 · 손익분기 ≈ 딜레이 50%) · 5,679 → **5,684 green** · format 29 불변
- `4.5.351` **NBA 적용이 이미 지어 둔 flat store 를 한 번도 안 불렀다 — 증명을 만들어 놓고 버렸다** · `k_write_scalar` 는 평범한 스칼라 쓰기용 flat store 이고 그 주석이 존재 이유를 적어 뒀다(*picorv32 에서 102,911 건이 `write_routed` 의 heap/assoc/frame/class 라우팅을 걸어 이미 필요 없다고 증명된 store 에 도달*) — 그런데 `apply_nba` 가 **한 번도 안 불렀다**. 같은 설계에서 NBA 는 **1,231만 건**(24배)이고 **100.0% 가 그 모양** · ⭐ 증명은 이미 있었다(`Op::ScheduleNbaScalar` 가 `plain_scalar_dest` 로 게이팅) — `NbaUpdate` 가 값과 offset 만 나르고 분류를 안 날라 apply 시점에 사라진다 ⇒ 술어를 **같은 함수로**(`plain_scalar_dest_of` 추출·두 호출자) 다시 묻는다 · picorv32 **2.40 → 2.31 s (−3.9%)** 백투백 A/B · **바이트 동일** · keccak flat(NBA 쓰기의 88.8% 가 배열/wide) · ⚠️ **측정이 가드도 고쳤다**: 첫 프로브의 `offsets==[(0,0)]` 만 보는 약한 가드는 picorv32 에선 우연히 동일했지만(weak_only 0) **keccak 에선 2,603 건이 샜다** — dead slot 에 flat write 될 수 있는 자리 · 적대 2렌즈 **BLOCKING 0**: soundness 가 `write_routed` 의 여섯 레인이 술어로 전부 막히고 꼬리가 **동일 인자의 동일 `write_chunk_word`** 로 환원됨을 증명(부작용 여섯이 전부 분기점 아래 · `plain_scalar` 가 잠시 비는 창은 fail-safe), differential 이 34쌍 8축 전 채널 동일 + **비공허성 증명**(fast arm 22/27 발화 · 모든 fast-arm apply 의 offsets 가 예외 없이 `(0,0)`) · 5,679 green · format 29 불변
- `4.5.350` **§2 음수 상수를 먹는 소비자 셋 — 소비자가 언어 규칙 대신 컨테이너의 비트 패턴을 읽었다** · 큐엔 **둘**로 적혀 있었고 3-오라클 census 가 **셋**임을 보였다(세 번째 = `logic q[-3]` 가 조용히 1워드) 동시에 **불가침 축 넷**(음수 bound 의 part-select 계열 — iverilog `0101xx` vs verilator `000010`)을 잘라냈다 · ⓐ replication count 가 `-56` 을 **4294967240** 으로 읽어 타깃 폭 절단 후 **255, exit 0**(`$bits` 가 실제로 그 수를 답했다) ⇒ LOUD · ⭐⭐ **부호는 자기결정 폭에서만 존재한다**: `{(4'd0-4'd1){1'b1}}` 는 두 오라클 **255(15 복사)** 이고 `{(4'sd0-4'sd1){1'b1}}` 는 **거부**인데 폭-무제한 fold 는 **둘 다 −1** — 무제한으로 물었으면 맞는 설계를 false-reject 했다(경계 두 칸 테스트 핀 · 새 `const_bound_signed`) · 읽기를 **둘**(lowered `Const` / AST self-det) 둔 이유 = 파라미터는 접히고 `{(2-3){…}}` 는 `Add`/`Sub` 로 남아 u32 fold 가 **saturate 해 0** ⇒ 옛 진단이 −1 에 대해 *"count of zero"* 라고 말했다 · ⓑ 오름차순 음수 range bound(`[-3:0]`·`[-56:0]`·`[-8:-1]`·**`parameter W=0; logic [W-1:0]` 관용구**)가 폭 1 클램프 + *"param value 0?"* 라는 **파라미터가 없는 리터럴에도 나오던 거짓 진단** ⇒ `|msb-lsb|+1`(두 오라클 2·57·5·8·2) · ⭐ 근인은 머신러리가 아니라 §4.5.228 이 세운 **가정**(*"msb<0·lsb≥0 은 degenerate 파라미터 언더플로"*) — 방향이 어느 bound 가 내부 비트 0 인지를 정하지 **어느 쪽이 음수인지가 정하지 않는다** · 저장은 정규화 + 새 sparse map `net_decl_asc_lsb`(`internal = l − raw`, 내림차순의 거울) · VCD `$var` 는 선언 쌍 · PART select 는 쌍둥이와 같은 이유로 loud · 곁: 폭 캡 초과의 **조용한 `.min(MAX_NET_WIDTH)`** 를 두 방향 모두 평범한 cap 에러로 · ⓒ unpacked 크기 `q[-3]`/`q[0]` 는 `.max(1)` 바닥이 먹고 있었다(경계 실측: 0 과 모든 음수는 두 오라클 거부·1 은 수용) · ⚠️ **옛 동작을 의도로 고정하던 테스트 2건 갱신**(삭제 아님) · ⚠️ 적대 리뷰 **BLOCKING**: dyn/queue 원소 net 이 넓은 폭만 받고 사이드맵 기록은 early-`continue` 로 건너뛰어 **warn → 조용한 `x`** 하강 ⇒ opt-in 을 사이드맵이 실제로 정규화하는 net 으로 좁혔다 · ⚠️ **두 렌즈 수렴**: `dim_coord` 의 `debug_assert` 불변식이 **이미 거짓**이었다(PRE 도 패닉) — release 는 그것을 컴파일아웃하고 `lo.max(0)` 로 **조용히 틀린 비트**를 읽는다 ⇒ 두 빌드 모두 LOUD · 곁: string **변수** replicate 의 음수 카운트(같은 규칙의 세 번째 철자) · 79칸 4-way **회귀 0 · FIXED 30 · 오라클 분열 7(불가침)** · differential 렌즈가 55칸 추가(staged·쓰기·부호·replication 경계) **회귀 0** · 5,654→**5,679 green** · 뮤테이션 15/16(1 방어) · format 29 불변
- `4.5.349` **§2 런타임 mixed-real — 변환 경계는 self-det, 그리고 그 규칙을 두 크레이트가 공유한다** · 뿌리 = `sim_ir::selfwidth` 가 real const 를 `{width:64}` 로 답해 mixed Binary 의 문맥이 64 가 되고 정수 형제가 거기서 평가됐다(`1.0 + (-s)` 두 오라클 −7 / vita 9) · ⭐ **선행조건을 먼저 세웠다**: `elaborate::expr_is_real` 을 **`sim_ir::realness` 로 리프트**해 `child` 콜백으로 두 크레이트가 한 철자를 공유(엔진은 `WidthTable` 옆에 메모이즈) — 앞 반복이 *"값으로 판정하면 `$random` 이 두 번 뽑히고, self-det 후 resize 는 중첩식을 깬다"* 를 금지로 적어 둔 덕에 짧은 우회를 안 골랐다 · ⚠️ 적대 리뷰 **BLOCKING**: 정적 규칙은 삼항을 real 이라 하는데 엔진의 삼항이 취해진 정수 arm 을 변환하지 않아, PRE 에서 **상쇄되던 두 오차**가 한쪽만 고쳐지며 correct→silent-wrong · 삼항이 실제로 real 을 **생산**하게 고치고 비교 연산자까지 같은 규칙으로 확장(그 전엔 `1.0 + (-s)` 와 `1.0 > (-s)` 가 한 설계 안에서 불일치, `if` 가 다른 가지를 탔다) · 68칸 3-way **회귀 0 · FIXED 11** · 4 백엔드 일치 · 뮤테이션 10/11(1 등가는 도달 불가 증명) · 5,647→**5,654 green**
- `4.5.348` **§2 `**` 지수의 부호 — i64 는 컨테이너이지 값의 타입이 아니다** · ⭐ **재센서스가 기록된 둘 중 하나를 지웠다**: 「폭-미상 wrapping 지수」는 §4.5.345 의 multi-packed 폭이 그 기록이 스스로 예측한 선행조건이었고 **이미 닫혀 있었다**(정작 그 슬라이스는 몰랐다) · 남은 뿌리 = `64'd0 - 64'd8` 은 **부호 없는** 뺄셈인데 컨테이너의 부호 비트를 읽어 IEEE 음수-지수 표를 적용, **조용히 0**(두 오라클·vita 런타임 926288481) ⇒ 지수의 부호가 값과 함께 다니고 도메인 밖 크기는 정직한 loud · 밑수 0/±1 은 **패리티**만 쓰므로 도메인 밖 지수에도 답한다 · ⚠️⚠️ **모듈러 fold 를 지어서 되돌렸다** — mod 2^64 는 문맥 ≤64비트에서만 옳은데 모듈 스코프 fold 엔 폭이 없어 `localparam [127:0]` 이 이미 잘린 값을 zero-extend(loud→silent-wrong · 96비트에선 silent-wrong→다른 silent-wrong) · **적대 2렌즈 독립 수렴** · 곁: 파서 `try_const_index` 의 unchecked 산술이 평범한 `localparam` 하나로 **런 전체를 패닉**시키던 것 해소 · 72칸 3-way **회귀 0 · wrong→LOUD 5** · 뮤테이션 8/11(3 등가) · 5,642→**5,647 green**
- `4.5.347` **§2 top-level 자기결정 위치 — 착수 스코프를 3-오라클 census 가 정했다** · ⭐ §2 항목은 *"top-level const 문맥 전반"* 이라 넓게 적혀 있었지만 손댈 수 있는 것은 **두 오라클이 합치하는 자리**뿐이라, 18칸 × iverilog/verilator/vita 로 먼저 갈랐다 — **9칸 합치·vita 만 틀림**(비교 5·논리 3·조건 2) vs **갈리는 축 둘**(untyped localparam 16/0 · repeat 2/18)은 불가침 · 비교/등가/논리는 결과가 1비트이고 피연산자가 **서로에 대해** sizing 되므로 노드 전체가 자기결정인데 `const_eval_in_scope` 이 무제한으로 접고 있었다 · 분할은 **와일드카드 없는 exhaustive match** 한 자리(`binop_result_is_context_determined`) · 리뷰가 낸 **이웃 불일치 둘**(시프트 카운트가 비교 아래에서만 정답 · 와일드카드 비교의 LHS 가 여전히 무제한 walk) + **`const_self_width` 의 Replicate arm 부재**(replication 피연산자가 폭을 미상으로 만들어 규칙 무력화) 해소 · 193칸 3-way **회귀 0 · FIXED 15** · ⚡ 비교 fold 가 ~3× 느려졌으나 **현실 설계에서는 미측정**(picorv32 elaborate 0.030→0.030 s) — 수치·처방 §2 · 5,633→**5,642 green**
- `4.5.346` **§2 크기 캐스트의 상수 도메인 — 피연산자를 먼저 좁힌다**(옛 착수순서 1번) · 옛 arm 은 피연산자를 **무제한 도메인에서 접고 절단**했고, 절단이 근사라서 *"부호 두 해석이 일치할 때만"* 이라는 자기제약을 달고 있었다 — 그 제약이 `8'(255)`·`4'(9)`·`8'(P)`·`64'(-1)`·`1'(3)` 같은 **평범한 좁힘 캐스트 20종**을 거절 · 수정은 §4.5.345 가 상수함수 본문 arm 에 깐 **같은 라우팅**(`max(self,N)` + 피연산자 부호)이고 이제 **`const_size_cast` 한 함수를 두 arm 이 공유** · ⚠️ 적대 리뷰가 **BLOCKING 둘**(N>64 에서 carry 유실 · whole-node 문자열/와일드카드 비교가 폭-인식 walk 에 가려짐) → 도메인 경계 두 겹 복원 + `const_compare_special` 공용화 · ⚠️⚠️ **게이트가 두 렌즈와 147칸이 못 본 것을 잡았다** — 헬퍼 추출이 공유 계산을 복제해 좌편향 200단 체인이 **2^n**(240초 타임아웃) · 147칸 3-way **회귀 0 · FIXED 4 · LOUD→CORR 23** · 뮤테이션 9/10(1 등가) · 5,627→**5,633 green**
- `4.5.345` **§2 상수함수 해석기의 선언 바인딩 — 모르는 것을 값으로 만들지 않는다**(옛 착수순서 1번 + 2번 앞머리) · ⚠️ **재센서스가 항목을 셋으로 갈랐다**: init 조용한 0 은 재현 · 기록된 `int t = f();` 크래시는 **소멸**(진짜는 *자기참조* `int t = g();`) · 폭-0 항목은 **기록된 메커니즘이 오진** · 미fold 초기화는 **미바인딩**(읽을 때만 loud ⇒ 죽은 초기화는 정답 유지 = correct→loud 하강 없음) + 선언 이름의 파라미터 폴백 금지(없으면 동명 param 이 조용히 답한다) + `body_decls` 를 본문 깊이에서(스택 오버플로→E3009) + multi-packed 폭 = **차원의 곱** · ⚠️⚠️ **기록대로 고친 첫 판이 correct→silent-wrong 회귀**(16→10000)였고 **PRE-3-way 가 잡았다** — `max(self,0)`=자기 폭은 옳은 degrade, 결함은 계산 가능한 폭을 declined 한 것 · ⭐ 적대 리뷰의 major(참값 0 인 미fold 초기화는 옛 기본값이 우연히 정답)를 **위로** 닫았다: concat·replication·SIZE 캐스트를 **carry-free wide folder**(>64비트 파라미터를 짓는 그 코드)로 접고 **모듈 스코프도 같은 헬퍼로** ⇒ 두 스코프 한 철자 · 61칸 3-way **회귀 0 · FIXED 19 · LOUD→CORR 4 · wrong→LOUD 8(크래시 2)** · 뮤테이션 14 설계(생존 7 을 킬러로 전환) · 5,616→**5,627 green**
- `4.5.344` **§2 bound/count/index lane 이 자기 폭에서 접힌다**(옛 착수순서 1번 — 실체는 `**` 미fold 가 아니라 한 결함의 두 철자: `const_bound_u32` 전면 decline + lowered-tree 얕은 fold 폭-무시) · tier-3 자기결정 걷기(폭-기지 술어로 degrade 차단) + `lower_index_expr` 한 자리 합의-보존 치환(read/write/index/offset/width 동시 정합) · §11.4.12.1 0-repl 합법성 loud(verilator 동판정) · ⚠️ fill-literal 잠복 결함(32-bit all-ones)이 노출돼 엔진 패닉 → 퍼널 자체로 특례 승격 · 옛 decline-핀 6개 강화(0→4·30→14·1→ef·0→11·0→14·0→4 — 전부 오라클 값) · 뮤테이션 7/7 · 5,608→**5,616 green**
- `4.5.343` **§2 정확성 큐 1~3 — 자기결정 위치 셋을 한 뿌리로 닫다**($clog2 인자 · 캐스트 SIZE 식 · real 변환 경계 — 무제한 i64 fold 가 셋 다 넓혔다) · vita 자기 불일치 해소(module-scope 4 vs 상수함수 0 → 세 평가기 0 합치) · §20.8.1 unsigned = **자기 폭 비트 패턴**(verilator+런타임 핀 · iverilog 32 는 결함 ⑤) · real 클래스는 **`**` 만이 아니라 모든 연산자**(8형태) — 수정은 게이트 하나 · 뮤테이션 **10/10**(생존 1 을 라우팅 증명으로 재판정) · PRE-3-way 회귀 0 · 5,596→**5,608 green**
- `4.5.342` **3판 잔여 클리어 라운드(순서 1~10 + P1)** — §3.1(a)(b) W2004 · §3.3 wide fold · §3.5-①③ repeat · §3.6-①② default clocking/disable iff · §3.7 INPUT string formal · P1 inner-NET shadow · **#9 staged file:line(v28)** · **#10 런타임 severity file:line+인스턴스(v29)** · §3.11 은 지어서 재고 되돌림(전제 반증) · 5,533→**5,596 green** · format 27→**29** · MsgCode 67→**68**
- `4.5.341` **외부 aes_top 3판 — 로그를 데이터로 쓸 수 있게, 그리고 그 과정에서 찾은 escape 다섯**(리포트에 값 결함 또 0 · 11 항목이 전부 **로그가 무엇을 말하는가**): ⭐⭐ **§3.2 요청은 `"\r"` 하나에 "고쳐 달라가 아니라 경고해 달라"였는데, escape 축 전체를 iverilog+verilator 로 재자 IEEE Table 5-1 이 **정의하는** `\ddd`·`\xhh`·`\v`·`\f`·`\a` **다섯이 조용히 틀려 있었다**(값도 폭도 — 백슬래시가 남았다) · ⚠️⚠️ **리포터가 제안한 우회 `"\015"`·`"\x0D"` 가 그 다섯 중 둘**이라 요청대로만 했으면 **동작하지 않는 해결책을 권했을 것** · `\r` 은 **값을 안 바꾸고**(IEEE 미정의 ∧ **오라클이 갈린다** — verilator 는 vita 와 같은 0x0D) **W3059 신설**로 silent→loud · ⭐⭐ **119 형태 sweep 이 정책의 증명**: 40 일치(전부 Table 5-1) · 79 발산(전부 미정의) · **조용한 발산 0 · 과잉경고 0 = 발산 집합 ≡ 경고 집합** · ⭐ **§3.4** `W3056 … default kept` 가 **적용된** string 오버라이드에 발화(오버라이드는 **세 채널**인데 경고가 `value` 하나만 물었다 · escalation 은 **이미 셋을 다 묻고 있었다**) → 레코드를 먼저 짓고 경고가 그것을 읽는다 · ⭐⭐ **§3.8** 화면의 유일한 문자열(`VITA-W3056`)로 `-Wno-`/`-Werror=` 가 안 됐다(`explain` 은 됐다 = **한 질문의 두 철자**) → `MsgCode::resolve` 하나를 **세 소비자가 공유** · `-Werror=all` 신설 · **렌더러가 mnemonic 을 함께 찍는다**(doc-15 worked example **55 중 42 가 그 형태**인데 제품은 0) · ⭐⭐ **§3.10** picorv32 진단 **71 건 중 위치를 가진 것이 0** — 모델은 `location`·`context`·`sim_time` 셋을 이미 들고 있었고(35 생성 사이트 중 33 이 `location: None` · `context` 는 **채우는 곳도 읽는 곳도 0** = 죽은 계약) ⇒ 인스턴스 경로 + 인스턴스화 지점 ambient span + **런타임 진단이 배열을 이름으로** → elaborate **58/58 이 위치를 얻었다** · ⚠️ `WOp::LoadIdx` 에 넷 필드를 **안 붙였다**(payload 16B → 모든 op 24→32B · 그 레인이 런의 ~20%) → **cold path 의 `off→net` 역인덱스**(offset 이 엄격 증가 = 유일, 코퍼스가 단언) · ⭐ **§3.9** `--obs-dir`/`--probe`/`--hier-tree`/`--inst-paths` 가 `--help` 에 없어 리포터가 6일 만에 **무관한 에러 메시지 안에서** 찾았다 · ⭐⭐ **§3.11** CHANGELOG 의 *"function 은 공짜"* 를 실측이 갈랐다 — **`function` 은 인라인되고 `function automatic` 은 안 된다**(그것이 스타일 가이드가 요구하는 철자다) · `run.json` 에 **`elab_s`/`sim_s`**(`wall_s` 하나로는 귀속 불가) · ⚠️⚠️ **절차 사고: 배터리 앞의 패턴 검증 루프에 `git checkout -- .` 를 써서 커밋 안 된 22 파일이 HEAD 로 되돌아갔다**(스냅샷의 10 개만 생존) → 복구 후 규칙을 두 방향으로 강화 · 전 스위트 **5,533 green** · MsgCode **66 → 67**
- `4.5.340` **외부 round-29 — correct-or-loud 의 loud 쪽을 읽을 수 있게**(값 결함 0 인 리포트 · 리포터는 코퍼스 전량을 새 기본 백엔드로 돌려 **틀린 값 0**·3-way 219 시나리오 바이트 동일을 확인했고, 찾은 것은 전부 **진단 품질**이다): **R29-1** 모든 구문 오류가 `{found:?}` 로 **vita 의 렉서 enum 을 유출**(`found Word(Keyword(End))`) — 철자는 span 에서 복구 가능했는데 **에러에 이미 있던 span 으로는 안 된다**(`error_at` 은 **더 이른 노드**에 보고하고 `found` 는 커서 토큰이라 둘을 **별개 필드**로 갈랐다) · **R29-2** clocking `default input/output` skew 아이템(실제 clocking 블록의 첫 줄)이 **문장 하나를 `expected {X}` 슬롯에 넣은** 메시지로 거부되고 그 메시지가 **없는 길**(`default skew only`)을 가리켰다 → IEEE §14.3 `default_skew` 를 **파싱하고 skew 없는 아이템에 스탬프**(두 번째 메커니즘이 아니라 = 판정은 여전히 elaborate 한 술자리) · 거부는 **skew 를 쓴 대로**(`#0`, `0` 아님)와 **신호 이름**을 부른다 · **R29-4** 런타임 진단에 위치가 **전혀** 없었다(같은 실행의 parse 진단은 `file:line:col`) — ⭐ **시각은 빠진 적이 없다**: `sim_time` 은 **모든 런타임 emitter 가 이미 찍고 있었고**(34 중 29 · 나머지 5 는 `RtlOutput` 둘 + 실행 밖 진단 둘) **렌더러가 버렸다** ⇒ ` [at time N]` · 5,496 테스트 중 **정확히 둘**이 움직였고 둘 다 **강화**됐다 · **R29-3** `unique`/`priority` 위반이 RTL `$warning` 과 **한 코드**였다(파서가 위반 arm 을 **문자 그대로 `$warning` 문장으로 desugar**) ⇒ benign 하나를 끄면 **`$warning` 전부**가 꺼지고 doc-15 가 CI 게이트로 명시한 `-Werror=W-RUN-USER-WARNING` 이 **`$warning` 없는 설계를 깨뜨렸다**(실측 errors=2 → 0) → **W4031 `W-RUN-UNIQUE-VIOLATION`** 신설 · `SeverityKind` 가 **클래스**가 되고(`diag_class` 한 철자를 두 emitter 가 공유 — 그 전엔 **두 번째 철자**였다) · **format_version 27** · ⭐ 곁가지로 **`$__vita_` 예약 네임스페이스** — desugar 의 채널이 **이름뿐**이라 소스가 그 이름을 쓰면 **위반하지 않은 violation 을 파일할 수 있었다**(규칙이지 목록이 아니라 다음 desugar 도 덮인다) · **§5 성능**: 리포트의 *"Keccak 지배에서 native 가 vm 보다 ~5% 느리다"* 는 **재현 안 됐지만 지적한 벤치 갭은 진짜였다** — 8 형태가 **전부 절차 바디 지배**(한 레짐의 여덟 표본)라 두 형태를 더했고(`cont-assign-heavy` 0.76 · `heap-heavy` 0.86) **둘 다 native 가 이긴다** · ⭐⭐ **이기는 이유가 리포트의 메커니즘을 반증한다** — 두 형태 다 **vm/interp 0.97**(컴파일할 바디가 없다)인데 native 는 **15~24% 빠르다** ⇒ tier-3 의 이득은 **아레나에서도** 온다 · 실물 `bench/keccak` 도 0.85/0.93/0.63 · **8/8 → 10/10** · ⚠️ **stale staged bin 함정 재발**(`vcmp`/`velab`/`vrun` 은 `separate-bins` required-features 라 `--workspace` 빌드에 안 들어간다 — 12일 된 바이너리가 *"staged 가 옛 코드를 쓴다"* 는 **거짓 신호**를 냈다) · 전 스위트 **5,517 green** · MsgCode **66**
- `4.5.339` **elaborate 상수접기의 `**` 지수 자기결정 — 그리고 거부가 답이 아니라는 실측**: `localparam signed [15:0] P = 4'sd3 ** -4'sd8` 이 vita **6561** / iverilog·**vita 런타임** **0**(= 같은 소스가 param 과 런타임에서 다른 값) · 수정은 새 평가기가 아니라 **§4.5.186 폭 모델의 재사용**(헬퍼 하나를 세 fold 사이트가 공유) + `Concat`·Named-cast 폭/부호 arm + module-scope 위임 · 97칸 3-way **FIXED 10 · LOUD→CORR 2 · WRONG→LOUD 2 · 회귀 0** · ⚠️⚠️ **첫 설계(폭-미상 거부)를 적대 라운드가 반증했다 — 거부는 caller 만큼만 loud 다**(range bound 의 decline 경로가 **조용한 1비트 기본값**이라 correct→**silent-wrong**) ⇒ 거부를 걷고 **폭을 알 수 있게** 만들자 loud 였던 칸이 **정답 fold**(19683)로 올라갔다 · ⚠️ **깊이 청구도 과했다** — 인자는 유한 AST 하강이라 사이클이 없는데 청구하면 **65단 인자 중첩이 correct→LOUD** ⇒ **사이클이 있는 default 만** 청구하고 위임 게이트는 `depth==0` **∨ call-free` · ⚠️⚠️ **뮤테이션이 "등가" 를 반증** — Named 캐스트의 **부호** arm 이 스윕 밖 두 칸을 silent-wrong 에서 건져낸다(`RPS'(2)-4'sd9` 가 signed 면 −7→0 · unsigned 면 9→19683) · ⚠️ genvar 는 `param_meta` 가 없어 무부호로 읽혀 `2 ** (g-1)` 이 correct→LOUD 였고 **(32,true) 시딩으로 함께 닫았다**(IEEE §27.4) · 뮤테이션 **10/10 사망** · 발굴 11건 §2 등재
- `4.5.337` ③층 **V1 슬라이스 1 — SVA: 거부가 순전히 보수적이었다**(커널 코드 0줄 추가 · 지운 것은 `design_eligibility` 의 `sva` 행 하나): 네이티브 커버리지 **54.7% → 66.8%**(+760 호출) · ⭐ **SVA 는 런타임 기계장치가 아니다** — elaborate 가 `assert property(@(clk) a |-> b)` 를 `always @(clk) if (a && !b) $error(…)` 로 desugar 하므로 도착하는 것은 평범한 IR + StmtId 테이블 둘이고, 그 둘은 **공유 `builtins::dispatch` 안**에서 읽힌다(tier-3 은 §4.5.293 이래 배선돼 있다) · ⭐ **진짜 기계장치가 필요한 형태는 이미 자기 이름으로 거부 중** — `cover property`/liveness 는 `final_procs` → `final` 행, deferred assertion 은 `mature_deferred` 훅이 없어 별도 `deferred_assert` 행(census: **`sva` 혼자 760 · `deferred_assert` 혼자 14 · 겹침 0**) · ⭐⭐ **게이트가 세 번 자기를 반증했다** — ⓐ **하네스가 `assert_fire`/`assert_ctl`/`defer_*` 를 설치 안 해서** `$assertoff` 가 플래그 대신 출력되는 설계로 두 백엔드를 비교하고 있었다(발견은 거부 이웃 핀이 `Ok(())` 를 낸 것) · ⓑ **차분은 공유 코드에 구조적으로 눈멀었다(실측)** — 공유 dispatch 뮤테이션 둘이 엔진 게이트를 전부 통과하고 `cli::sva_rest` 절대 핀에서만 죽어서, **절대 앵커**를 짓자 엔진에서도 죽고 하네스 뮤테이션 둘도 함께 죽었다(그 전엔 생존) · ⓒ 앵커를 짓다 **내 기대가 틀린 것**을 쟀다(`$asserton` 시점에 위반 조건이 남아 있어 다음 posedge 에서도 발화) · **뮤테이션 6/6 사망** · ⚠️ 절차 사고: 뮤테이션 복원에 `git checkout --` 을 써서 **커밋 안 된 편집 둘이 HEAD 로 되돌아가 지워졌다**(가짜 kill 2건 유발) → 복원은 **바이트 스냅샷**
- `4.5.336` ③층 **V0 — 커버리지를 처음 쟀다**(구현 아님·트리 변경 0): 기본 백엔드를 `Native` 로 flip → `simulate()` **6,251 중 3,417(54.7%)이 네이티브** · **발산 0**(5377 중 5374 통과, 실패 3건은 전부 *"기본 백엔드가 vm 이다"* 를 단언하는 테스트) = **오늘의 ③층은 자기가 받는 절반에 대해 이미 바이트 정확하고 남은 것은 전부 커버리지다** · ⚠️ 계기가 두 번 틀렸고 두 번 다 측정 자신이 잡았다 — **`SimOpts::default()` 가 enum default 를 안 쓰고 `Bytecode` 를 하드코딩**(flip 이 CLI 절반만 움직였다)하고, **`writeln!` 이 unbuffered `File` 에 조각마다 `write(2)`** 를 내 프로세스별 nextest 에서 행이 찢어져 **1.85× 부풀었다**(11,563 vs 6,251 · 판별자는 레이아웃을 아는 파서) · ⭐⭐ **첫 census 는 V1 을 정렬하지 못했다 — 게이트가 단락하기 때문**(design 에서 걸린 설계의 storage/executor 는 미측정) → 세 층을 **독립으로** 물으니 구조가 나왔다: **design 게이트와 storage 게이트가 같은 기능을 두 번 이름 부른다**(`D:string` 369회 발화·단독 원인 **0회** = `S:heap-slot` 과 짝 · `D:real`/`S:real-slot` 도) ⇒ **슬라이스의 단위는 행이 아니라 한 기능이 걸린 게이트 전부의 집합** · V1 순서(greedy 누적) = **SVA +774 → 67.0% · heap 저장+네 종류 +560 → 76.0% · 서브루틴 프레임 +712 → 87.4%** · `stmt_effect` 91.2 · class 93.9 · fork 95.5 · 시스템태스크 96.8 · `real` 98.0 · coverage 99.0 · 나머지 다섯 100% — **상위 셋이 87.4% 를 산다** · ⚠️ `fork`·`file_directed` 는 **단독 이득 0** · ⚠️ fallback 2,834 중 **2,656(93.7%)이 CLI 서브프로세스**라 테스트 단위 귀속은 못 쓴다(단위 = `simulate()` 호출)
- `4.5.335` ③층 **S4 — 중단 판정 발동**: `settle_cont_assigns` 가 **연속 대입 평가마다 `Lvalue` clone**(= 힙 할당)하던 것을 borrow 로(강제한 것 없음 — `ir` 은 `k` 와 독립 파라미터) + md 그룹 멤버 Vec clone 제거 → picorv32 0.517→**0.504**(+2.5%) · ⭐⭐ **나머지는 짓기 전에 프로파일이 결론냈다** — S4 가 겨냥하는 것(`propagate` 2.1+`wake` 1.4+`pass` 구성 ~2)은 **≈6% = 1.06×** 로 자기 중단 판정(<1.3%) 아래이고 `settle` 의 9.1% 는 대부분 **스케줄이 아니라 실제 평가**다 → **정적 깨우기 마스크는 짓지 않는다** · ⭐⭐ 고정 비용 실측 **19 ms · 3.7% → 상한 26.8×**(doc-21 §6.3 의 *"~85 ms · 14% · 7×"* 정정) · **성능 축 수확 체감 도달 → 다음은 커버리지**(ROADMAP §5.1: interp=오라클 + native=제품, VM 은퇴)
- `4.5.334` ③층 **S3 코드젠 본체 착수 — census 가 S3 의 전제를 반증했다**: picorv32 0.540→**0.517**(native/vm **1.71×**) · ⭐⭐ 실행 census 가 셋을 반증 — **인라인할 산술이 1.5% 뿐**이고 · **leaf load 는 이미 `buf[vi]` 직접 접근**(②층 JIT 이 진 이유는 S1 이 이미 없앴다) · **인라인 불가한 30.8%(`LogBin`+`Unary1`+`Cmp`)가 하필 §4.5.315/330/331 이 한 철자로 통합한 함수들**(cranelift IR 재작성 = 의미 재진술 = §4.5.279 클래스, 콜백 = op 당 경계 = ②층이 진 모양) · ⭐⭐ **win 은 프로그램이 아니라 그 주위 기계장치에 있다**(실행의 **47.2% 가 op 하나짜리 프로그램**) → 대입 전체를 한 seam 으로(`k_eval_write_scalar`/`k_eval_nba_scalar`, 기본 구현 = 기존 두 호출이라 ②층 불변) · ⭐ 뮤테이션 7 중 6 생존이 **옳다**(전부 동치 증명 · 중복 가드 삭제 · 캐시 키 술어 공유)
- `4.5.333` ③층 **S3 착수** — **컴파일된 바디**: picorv32 워크 대비 **+3.9%**(1.57→**1.64×**) · ⭐⭐ **컴파일된 표현 자체는 win 이 아니다**(융합 전 완전 wash — `k_resolve_lvalue_offsets` 3.1% 가 사라진 만큼 op 루프와 `Value` 레지스터 왕복이 먹었다) → 평가+쓰기 융합으로 op **2000→1068**·대입 93.7% 융합 · ⭐⭐ 남은 비용은 커널 호출을 *고르는* 데가 아니라 그 *안*에 있다 = **S3 의 값은 전적으로 코드젠 단계에 있다** · ⭐⭐ `vm_exec` 에 **문장 경계가 없었다**(②층에도 있던 `call_fatal` 발산) · ⭐ 뮤테이션이 내 구멍 둘(`k_enter_body`·lvalue 인덱스 호출)을 찾았다 · 7/9 사망(2 동치)
- `4.5.332` ③층 — **쓰기 퍼널의 한 워드 진입점**: picorv32 1.44→**1.53×**(이 슬라이스만 6.2%) · 퍼널+`resize` 20.6→**12.5%** · ⭐⭐ 표적을 정한 것은 census 이고 그것이 내 계획의 절반을 버렸다(비트 직렬은 **1.4%**·통짜 저장이 98.6%) · ⭐⭐ 내 스윕을 리뷰해 구멍 둘(엣지 감도 0개 · `is_real`/`is_str` 미설정)을 스스로 찾았다 · 뮤테이션 12/12 사망
- `4.5.331` ③층 — 한 비트 단항과 `&&`/`||` 도 평면으로: **W 평가기에서 `Value` 소멸** · picorv32 1.27→**1.43×**(이 슬라이스만 11.1%) · keccak 은 0 = **형태가 다르면 같은 수정이 다른 값을 낸다** …
- `4.5.330` ③층 — 비교 3종을 평면 레벨로(정본이 한 워드면 그리로 위임): picorv32 1.16→**1.28×**(이 슬라이스만 9.2%) · 공유의 대가로 차분이 눈멀어져 **오라클 핀 넷으로 확인** …
- `4.5.329` ③층 S2 종료 — 프로파일이 **admission 축의 사망을 확인**(`eval_ctx` 50%→2.2%) · 정본 `truthiness` 를 평면 레벨로(≤64비트 O(1)) native 3.1% · **다음 표적은 `Value` 마샬링 15.7% + 쓰기 퍼널 8.7%** …
- `4.5.328` ③층 S2 슬라이스 7 — `Ternary` admission(게으름은 **진단**의 문제다): picorv32 1.03→**1.12×** · 같은 앵커가 **기본 백엔드(vm)의 exit-code 결함**을 찾아 함께 고쳤다 …
- `4.5.327` ③층 S2 슬라이스 6 — `Select` admission(상수·범위내 창 = 시프트+마스크): **네이티브가 picorv32 에서 처음 VM 을 앞섰다 0.90→1.03×** · 두 후보(Select/Ternary)의 **천장을 먼저 재고** 순서를 정했다 …
- `4.5.326` ③층 S2 슬라이스 5 — `Concat`/`Replicate` admission: **인덱스 봉인 하나가 모든 런타임 배열 읽기를 제네릭으로 떨어뜨리고 있었다**(배열 읽기 0.94→1.26×·메모리 0.92→1.85×) · 개정 6 의 "S2 는 레버가 아니다" 는 **틀린 축의 ablation** 이었다 …
- `4.5.325` **인라인 인자 바인드가 formal 의 선언 타입을 적용한다 — 셋을 함께, 그리고 두 렌즈가 낸 두 결함이 같은 한 문장에서 나왔다.** 인라인(SSA-fold)은 actual 의 ExprId 를 이름에 치환할 뿐이라 §13.4.3 이 말하는 *"formal 은 선언 타입의 변수"* 가 본문에 도달하지 않는다 — 폭(§11.6.2)·부호·2-state(§6.11.1) 셋 다. 프레임 쌍둥이는 formal 이 넷이라 셋을 엔진에게서 공짜로 받는다. §4.5.323 이 세 번 부분 적용해 세 번 회귀했으므로 이번엔 **하나의 게이트**(신뢰 가능한 actual 폭)로 셋을 함께 걸고, 없으면 pre-slice 를 그대로 둔다. 1,920칸 **FIXED 636·REG 0·still-wrong 8**(그 8은 프레임 퍼널). ⭐⭐ **적대 두 렌즈가 낸 BLOCKING 둘이 같은 문장의 두 얼굴이었다 — "2-state 단계가 `resize_inline_assign` 밖에 있다"**: 밖이라 real/string 가드를 못 받아 `real` actual 이 IEEE-754 페이로드로 조각났고(correct→silent-wrong 40칸), 동시에 그 프리미티브가 **선언 비트마다 피연산자를 언급**한다는 사실이 내 논거에 없어 런타임 15.7×(`int`)·42.7×(`longint`)·중첩은 미완주였다. ⭐⭐ **내 바이트-동일성 게이트가 그 축에 구조적으로 눈멀어 있었다** — `examples/`·`bench/` 전체에 2-state formal 이 **0개**다. ⭐⭐ **내가 스스로 잡은 회귀 하나**: 부호 스탬프가 하류 `extend_to` 의 sign-fill(= 피연산자 두 번째 언급)을 불러 `$random`·프레임 호출 actual 이 **두 번 뽑혔다**(§4.5.320 S1 이 바인드가 아니라 **반환 리사이즈**에서 재발). ⭐ 뮤테이션 26 중 생존 11, 그중 8이 **내 테스트 actual 축의 구멍**(real·계층참조·unsigned 2-state formal·좁은 signed→넓은 2-state·2-state 피연산자·ternary·다인자 정렬)이라 전부 행을 지어 닫았다. ⭐ 완화 둘(메모·노드 예산)을 지어서 **개선 0 으로 반증**하고 그 사실을 §2 에 적었다. 신규 §2 여섯(프레임 바인드 CLASS·배열 원소 반복가능성·계층/클래스필드 폭·`p::f()` 철자·O(폭) 강제·body-local 2-state).
- `4.5.324` **상수 배열 인덱스가 부호를 가진다 — 여섯 줄, 그리고 §4.5.323 의 선행조건.** §4.5.310 은 상수 인덱스를 일부러 봉인 전 경로에 두어 "정적으로 범위 밖이면 진단된다" 를 지키려 했는데, 엔진이 인덱스를 `to_u64` 로 읽어 **32비트보다 좁은 signed 상수가 양수로 도착**했다 — `arr[-8'sd1]` 이 원소 255(`a5`)를 조용히 읽고 같은 비트의 `arr[8'sd255]` 도 마찬가지. ⭐ **숨은 이유가 판별자다**: 무부호 읽기가 **우연히** 범위 밖일 때만 맞았고(`sm[-4'sd1]`=15), **넷 인덱스는 §4.5.309 이래 봉인**돼 늘 정확했다 — 틀린 건 상수 carve-out 하나뿐. ⭐ 평범한 철자 `arr[3]` 은 unsized 십진이 32비트라 **바이트 동일**. 2,790칸 FIXED 159·REG 0·still-wrong 0 + differential 213칸 FIXED 53·REG 0 · 24런 바이트 동일. ⭐ 뮤테이션 생존 9 중 하나는 등가가 아니라 **패닉**(폭 가드를 지우면 `32−64` 언더플로) → `-64'sd1` 행으로 막았고, **폭 31 은 원리적으로 판별 불가**(무부호 읽기가 20억)라 공허한 행 대신 그 사실을 적었다. ⭐ 리뷰어의 거짓-주석 지적은 실측으로 기각(그 문장은 기각된 대안에 대한 것). 신규 §2 넷 = queue/dyn 봉인 부재(**더 넓다**)·함수 호출 인덱스·`$bits`·평범한 벡터 비트선택.
- `4.5.323` ⚠️ **인라인 인자 바인드의 formal 타입 — 지어서 세 라운드 재고 되돌렸다(코드 미머지).** 인라인은 actual 의 ExprId 를 이름에 치환할 뿐이라 formal 의 **폭·부호·2-state** 를 하나도 적용하지 않는다(프레임은 formal 이 넷이라 셋을 공짜로 받는다) — `fi(8'hf7)` 이 inline `82` / frame·iverilog `-3`. ⭐⭐ **셋은 서로의 전제라 부분 적용이 미적용보다 나쁘다**: 부호만 넣으면 45칸 회귀, 폭까지 넣으면 절단이 상수를 접어 §4.5.310 의 상수-인덱스 carve-out(무부호로 읽는다)에 착지해 loud `xx` → 조용한 `a5`. 세 라운드가 매번 새 회귀를 냈고(마지막은 **§4.5.310 이 이미 기록한 '상수 인식이 철자 기반' 함정의 재발**), ⭐⭐ **후퇴안조차 측정이 반증했다**(FIXED 303→215·REG 0→**80**). ⇒ **선행조건은 carve-out 의 부호**이고 그것이 다음 표적이다. 되돌림.
- `4.5.321` **인라인 함수의 선언 반환 폭도 자기결정 경계다.** §4.5.320 이 캐스트에서 고친 결함이 인라인(SSA-fold) 경로에 그대로 있었다 — `resize_inline_assign` 의 `$signed`/`$unsigned` 스탬프가 **조건부**였고 **붙지 않는 경우가 곧 봉인이 없는 경우**다(`function [7:0] fm(input [7:0] x); fm = x*x;` 를 64비트로 읽으면 `fe01`, iverilog `0001`). ⭐ 1,440칸 중 **48칸이 전부 한 구멍** — 선언 폭 == rhs 자기 폭 ∧ **무부호 반환**(부호 반환은 무부호 rhs 덕에 스탬프가 생겨 **우연히** 봉인돼 있었다). 수정 = 스탬프를 무조건으로 + §4.5.320 의 신뢰-가능-폭 가드. ⭐⭐ **그 가드에 이빨이 0개**였고(뮤테이션 셋 생존) 킬러는 클래스 필드였다(핸들넷의 지어진 `Some(32)` 가 선언 폭과 우연히 같다). ⚠️ 앵커를 지으려다 **새 결함**을 쟀다 — 인라인 경로가 **선언 폭을 본문 안으로 안 내린다**(넷 버전 static `00000001` / `automatic` `0000fe01`=iverilog, 같은 본문이 lifetime 철자로 갈린다). ⚠️ 내가 적은 *"부호 채택은 얻는 게 없다"* 도 거짓 — signed 클래스 필드는 **순수·반복 가능**한데 그 arm 에 도달한다. differential 6,680칸 REG 0·FIXED 235.
- `4.5.320` **캐스트는 자기결정 경계다 — 봉인, 그리고 상속 부호를 정본에서 읽기.** `lower_size_cast` 의 동일-폭 가지가 피연산자 노드를 **그대로 반환**해서, 엔진에게 문맥결정인 `Binary`/`Unary`/`Ternary` 가 바깥 폭으로 **연산 전체를 다시 돌았다** — `logic [15:0] r = 8'(a*a)` 가 `fe01`(오라클 `0001`) · 4,032칸 중 **634칸 조용히 오답, 전부 대상폭 > N**(대입·NBA·연속대입·포트·`case`·비교·함수인자가 전부 샜고 concat·`$display` 는 자기결정이라 정상). 수정 = 꼬리를 `$signed`/`$unsigned` 로 **획일화**(이 저장소가 이미 봉인이라 부르는 프리미티브). 같은 두 줄의 둘째 결함 = 상속 부호를 `expr_self_signed`(자체 미러)에서 읽던 것 → 정본 `sim_ir::selfwidth`. ⭐⭐ **이 슬라이스의 진짜 내용은 봉인이 아니라, 봉인이 드러낸 것들이다** — 적대 4라운드가 낸 회귀 다섯이 **전부 "elaborate 가 피연산자의 모양을 틀리게 안다"** 한 뿌리였다(placeholder·string 은 폭이 `None` → 32 를 지어냄 · **클래스 필드는 핸들넷의 틀린 `Some(32)`** · unpacked 원소는 `ast_ctx_signed` 가 packed 규칙을 적용 · 부호확장 fill 이 피연산자를 **두 번** 부른다). ⭐⭐ **그리고 내 첫 R-B 수정("모르면 옛 경로")이 170칸을 깼다** — 원소의 답은 "모른다"가 아니라 **자기 부호**다. 최종 = 폭을 **믿을 수 있을 때만** 봉인 · 불순한 피연산자에는 정본 부호를 채택하지 않음(비용 명시) · 원소는 자기 부호. 원소 1,560칸 FIXED 456·REG 0 · 형태 82칸 FIXED 37·REG 0 · 계층 1,080칸 REG 0 · 4,032칸 0 diff.
- `4.5.319` **`**` 의 지수는 자기결정이다(Table 11-21) — 하나의 불리언이 세 질문을 대신하고 있었다.** 공유 `arith` 커널이 두 피연산자를 **집합 부호 하나**(§11.8.1)로 읽는데 `**` 만 그 계약을 깨고, 커널은 **지수의 부호를 밑수의 것으로 찍어** 무마하고 있었다 — `4'sd3 ** 4'd11` 이 11 을 −5 로 읽어 0(오라클 11), 3,584칸 중 **336칸 조용히 오답**. ⭐⭐ **첫 수정이 하나를 둘과 맞바꿨다** — 피연산자 쌍을 등가 all-signed 로 다시 쓰는 트릭이 `WIDE_ARITH_CAP` 을 한 비트 넘겨 correct→**조용한 x**(loud 게이트는 **선언 폭**을 봐서 구조적으로 발화 불가)를 만들고, byte-identity 논증이 선 `base.width == w` 가 **`is_str`/`is_real` 에서 거짓**이었다(`resize_keep_sign` short-circuit · 스위트 진입 114회 중 0회라 아무도 못 잡았다). ⭐⭐ **교훈 = 공유 커널의 계약이 안 맞으면 피연산자를 계약에 맞게 다시 쓰지 말고 계약을 고쳐라** → `ArithSigns{lhs,rhs,res}` 3분할(비-`**` 는 셋이 같아 **기계적 항등** · 800칸 PRE↔POST 바이트 동일로 실측). ⭐⭐ **iverilog 는 폭 ≥ 33 의 음수 지수 `**` 에서 오라클이 아니다**(32/33 경계에서 뒤집힌다 · 안 박으면 폭 스윕이 거짓 REGRESSION 36칸을 낸다). ⭐⭐ **철자 여섯 중 셋만 고쳤다** — CA 하강·bytecode·상수접기는 CLASS 라 §2 로(백엔드 WORSE 0).
- `4.5.318` **크기 캐스트: fill 크기와 §11.8.1 전-피연산자 강제 — 그리고 지어서 잰 뒤 되돌린 leaf 해석.** 두 수정은 남았다: unsized fill 이 `n` 에서 지어지고(`4'(a+'1)` 1110→1100), 좁힘 가지가 **모든 피연산자**를 식의 부호로 재귀한다(`4'(a+((i13|s4)>>2))` 12→0, 1620칸 207 fixed·0 regressed). ⭐⭐ **본체였던 leaf 해석은 빼서 기록했다** — `ast_ctx_signed` 가 param/pkg/call 을 못 풀어 63칸 중 35칸이 틀리지만(`8'(P*Q)`=13), 붙이면 **세 하위 차단**이 동시에 열린다: 안쪽 self 폭이 `n` 을 넘는 트리에서 42칸 맞바꾸기 · `extend_to` 이중 참조로 함수 leaf 가 **두 번 호출**되고 `$random` 두 draw 가 섞임 · formal/local 이 결합 집합에 없어 동명 파라미터의 부호를 가져옴. ⭐⭐ **내 스윕이 세 번 연속 "0 regressed" 를 냈고 세 번 다 형태가 빠져 있었다.**
- `4.5.317` **크기 캐스트의 real 거부를 퍼널로 — 그리고 한 자리를 막는 것이 왜 절반인가.** `4'(r*2)` 가 `r=7.5` 에서 `0000` 을 찍고 있었다(참값 15). leaf 한 자리에 가드를 넣자 **세 부류가 그대로 조용**했다: 시프트 양(`4'(u << r)` — ⭐⭐ **캐스트 밖의 같은 식은 이미 loud 였다**, 크기-캐스트 하강이 있던 게이트를 침묵시키고 있었다) · `/ %` 좁힘 가지(`8'(u / r)` → `00110011`) · `$signed`/`$unsigned` 투명성(`4'($signed(r)*2)` 는 `0000`, 무캐스트 같은 식은 real 도메인 15 — **한 식에 두 답**). 수정 = 하강이 만드는 **모든 피연산자 자리**를 한 함수로 + `expr_is_real` 에 `$signed`/`$unsigned` 통과 팔(래퍼가 연산 **아래**에 있어 캐스트-로컬로는 못 잡는다) + 캐스트당 **진단 1건**(잎마다 내면 `MAX_ELAB_ERRORS` 캡을 먹어 뒤쪽 진단이 사라진다·실측). ⭐⭐ **완전성 주장을 스스로 반증당했다** — 차분이 288칸 중 **84칸**을 남겼고(fill ∧ 비-넷 real 원천 ∧ real 비전파 연산자) 그건 CLASS 라 §2 로 내보냈다 …
- `4.5.316` **사이즈 캐스트의 문맥 규칙 = `max(self, N)` — 그리고 "회귀 0" 을 두 번 반증당했다.** §4.5.212 가 캐스트 폭을 피연산자 문맥으로 그대로 내려보내 **좁힐 때** 조용히 틀렸다(`2'(k%4)` → `xx`, 두 오라클 `11`). 1차 시도는 문맥을 통째로 버려 886 고치고 **856 깼고**(맞바꾸기·되돌림), 2차는 제대로 구현해 내 1675칸 스윕에서 **0 regressed** 를 쟀는데 리뷰가 ~74,400칸으로 **1,586 회귀**를 냈다 — 전부 안쪽 `>>>` 하나(좁힘 가지가 부호 강제를 빠뜨렸고 `>>>` 에선 그것이 **연산을 바꾼다**). ⭐⭐ **스윕이 넓다고 판별력이 있는 것이 아니다** — 차이는 크기가 아니라 형태였다 …
- `4.5.315` **③층 S2 — 1비트 결과 연산자 가족(`&&`·`||`·`!`·6 reduction·`===`) admit.** 표적은 셈이 정했다(거절 ~300 중 **276**이 이 여섯). 의미는 재진술하지 않고 **자유 함수 7개 추출 + 메서드 위임**. 논거는 측정된 사실 하나 — 제네릭이 `&&`/`||` 를 **단락하지 않으므로** 즉시 평가가 같은 순서다(값이 아니라 **진단**이 걸린 문제 · `?:` 로는 전이 안 됨). picorv32 admission 60.8→66.3% · native 1.085→0.984 s. ⭐⭐ **뮤테이션 둘이 살아남았고 둘 다 내 테스트 설계 실수** — `lw≠rw` 를 노린 행이 **무해한 방향**이었고(직전 반복 교훈의 재발, 이번엔 값이 아니라 **방향**), 슬라이스의 correctness 논거 자체엔 테스트가 없었다(단락 뮤테이션이 loud→SILENT 로 만들며 전 스위트 통과) …
- `4.5.314` **IEEE 1364-2005 §12.2 암시적 파라미터 포트 리스트 — 그리고 "어디까지 빼는지" 를 정한 것은 셈이었다.** ANSI 헤더 없는 모듈의 본문 파라미터에 네 override 채널이 전부 도달하게 하고(PRE 는 넷 다 loud), `-G` 를 staged 에 배선하고, `'x`/`'z` override 의 조용한 0/all-ones 를 닫았다. 적대 **4라운드가 결함 24건**을 냈고 **다섯은 내 앞 라운드 수정이 만든 하강**이며 거의 전부 한 모양이었다 — *i64 채널에 i64 아닌 것을 넣으면 이름은 풀리고 값은 틀린다*. 계층 real 읽기는 세 표현을 다 지어 재고 골랐다: real Const 패치는 모든 정수 소비자를 깨고, i64 twin 은 **72칸 정답/6칸 오답**(오답은 전부 분수 몫 나눗셈), loud 는 그 72칸을 되돌린다 → **인터페이스는 twin 유지·모듈/ANSI 는 loud**(둘 다 PRE 동일). ⭐⭐ 판별 못 하는 테스트 값(`4/2==2`)이 하강 하나를 라운드 3까지 숨겼고, 그 뒤 "판별자는 값" 이라는 내 일반화를 4라운드가 매트릭스로 반증했다 — **판별자는 연산자였다** …
- `4.5.291` **③층 S1d-4a — 트레이트가 강제한 결정: 퍼널을 안 거치는 효과를 게이트가 답한다.** 먼저 계획을 고쳤다 — ③층 실행기는 두 번째 실행기가 아니라 **`Kernel` 의 두 번째 구현자**이고(이음매는 **가운데만** 제네릭: `run_process`·`builtins::dispatch` 는 `Scheduler` 고정), 52 메서드 중 **~27 이 그 선결 과제**라 *"나중에 배선"* 이 선택지에서 사라진다. → **`stmt_effect` 거부 행**을 tier-2 와 **같은 술어**로 추가. ⭐ **적격률이 처음 움직였다 79/79 → 77/79**(keccak 둘, 이유는 TB 의 `$value$plusargs` 하나) — 그리고 **②→③ 76× 와 성공 기준이 keccak 위에 서 있어서** 그 배선이 재측정의 선행조건이 됐다. 리뷰: **`$cast` TASK 형 누락**(내 SysTask 쪽만 `matches!` 암묵 catch-all — 주석은 여섯 줄 위에서 exhaustive 라고 주장 중이었다) → sim-ir 정본 `systask_net_write` **41 변종 `_`-free** 신설 + **`NetWrite{Flat,Heap}`** 로 double-booking 제거 …
- `4.5.290` **③층 S1d-3 — wake 결정, 그리고 게이트가 수정을 거부하도록 굳어 있던 일.** 변경 집합 → **어느 프로세스가 ready 이고 어떤 순서인가**(값이 아니라 **결정**을 비교). 적격성이 사준 단순화 = fork 거부 ⇒ activity ≡ process·`Ready` 가 proc id 로 붕괴. 게이트가 갭을 두 번 잡았다: `always @(a)` = static Level(첫 실행에 발산) → 그리고 적대 리뷰가 **`Level|Comb|Latch` 셋 다 같은 웨이터**임을 짚어 **조합 프로세스 전체가 잠들어 있었음**을 발견. ⭐⭐ **등록만 고치면 게이트가 깨진다** — `arm_processes` 가 Comb/Latch 를 t0 에 arm 하지 않고 **큐잉**하므로 `level_armed` 초기값이 `kind == Level` 이어야 한다(두 줄이 짝). 관측 granularity 는 **축**이고, 리뷰가 dedup 리셋 주기·저자 분포·granularity 커플링 셋을 더 잡아 **8규칙 전부 teeth** …
- `4.5.289` **③층 S1d-2 — dirty/edge 채널, 그리고 내 게이트의 teeth 를 세 번 재확인.** 스케줄링의 절반은 루프가 아니라 **쓰기**에 달린다 — `dirty`(멤버십 = 변경 집합이라 A→B→A 왕복도 관측)·`last_blocking_writer`·**`slot_edge`**(끝점이 잃은 엣지 **종류** 복원 · 값 비교로는 절대 안 보임). edge-target 스캔은 **한 철자로 추출**(두 번째 스캔이 func/task arena 를 빠뜨리면 증상은 틀린 값이 아니라 **안 뜨는 posedge**). ⭐ 게이트 첫 판은 7개 행동 중 **2개가 공허**했다(쓰기마다 take → dirty 길이 1 → 정렬 계약 무의미·같은 슬롯 재기록 없음 → glitch 누적 없음) → **배치**로 4/6, writer tag 구동 + edge-target `0→1→0→1` 퍼널 구동으로 **7/7 teeth**. 교훈 = *게이트를 짰다 ≠ 게이트가 검사한다*(공허함의 원인은 대개 **관측 시점**) …
- `4.5.288` **③층 S1d-1 — 백엔드 선택과 런타임 게이트: 판정을 두 층으로.** S1d 도 넷으로 분해(배선+게이트 → 스케줄러 코어 → settle/wired → 바이트 게이트). `Backend::Native`+`--backend native` 는 실행기가 없어 **항상 VM 폴백**하되 run.json 이 **요청이 아니라 결과**를 적는다(요청을 적으면 돌지 않은 실행기를 보고하게 된다). ⭐ `native{eligible, buildable, refused}` = **범위 상한 / 오늘의 저장소 / AND 의 이유** — 한 플래그로 접으면 **상한이 능력으로 읽힌다**(서브루틴 설계는 eligible ∧ ¬buildable). `buildable` 은 `build` 의 **무할당 쌍둥이**(build 가 먼저 호출 = 드리프트 불가). S1c 가 남긴 필수 4건 중 **2건 종료** …
- `4.5.287` **③층 S1c — 쓰기 퍼널, 그리고 그 게이트가 내 퍼널의 silent-wrong 을 잡았다.** 엔진 write 사슬의 Value-수준 미러(기하는 살리고 적격성이 답한 레인은 지운다·아레나 원소는 구성상 워드 정렬이라 정렬 테스트가 접힌다). ⭐ 문장 단위 차분(한 문장 실행 후 `changed`+전 스토어 비교)이 **방금 쓴 퍼널에서 결함 1건**을 냈다 — **real 값 rhs + real 넷 없음**(적격!)에서 엔진은 §6.2 반올림, 아레나는 IEEE 비트 그대로. 원인은 코드가 아니라 **주석의 논증**: "Real 은 아레나를 못 짓는다"는 **목적지** 쪽 답인데 강제 변환 조건은 **값** 쪽이다. 오프셋 해석기는 `eval::resolve_offsets` 로 **한 철자**(X/Z 인덱스를 한쪽만 버리는 발산 차단) · 게이트에 `force_release`/`disable` **문장 스캔** 2행(사이드카는 이들을 보고하지 않는다·적격률 79/79 불변) · **S1d 착수 前 필수 3건**(프레임 로컬 쓰기·dirty/edge 채널·`warn_run_range`) 기록 …
- `4.5.286` **③층 S1a+S1b — R1 아레나가 서고, 기존 eval 이 그 위에서 돈다.** S1 을 4 내부 단계로 분해(스케줄러를 한 번에 짓지 않는다 — 검증이 구현을 못 따라간다)하고 첫 둘을 완결: `NetArena`(원소당 `(val,unk)` 인접·워드 정렬·슬롯 desc 가 per-access 질문 전부를 대체) + `impl NetReader for NetArena`(기존 `EvalCtx` 가 제네릭이라 **평가 의미 공유 = parity by construction**·차분 표면이 read-path 로 좁아짐). **R1 중단 판정 통과**(corpus 72 폭별 슬롯 빌드 전수 성공). 게이트 = init parity 297넷 + 워드 경계 라운드트립 + **미러 차분 17,940건 발산 0**(정확 핀). 평가기 교체는 R2=S2 의 일 …
- `4.5.285` **③층 T0+S0 — 계기 두 개를 달고 적격률을 쟀다: 79/79, S1 go.** run.json 에 `codegen`(②층 VM claim + 거부 사유 히스토그램 — REAL gate 와 **한 walk 공유**라 드리프트가 구조적으로 불가)·`native`(③층 설계 수준 판정 — SimOpts **`..` 없는 전수 destructure** + **NetKind 스캔**[plain `int q[$]` 는 사이드카가 없다]). `func_table` 은 §4.3 초판의 거부 목록에서 **코어로**(개정 4 가 T1/T2 를 S3 에 흡수). 함정 = `opts.fork_modes` 가 스케줄러로 move — 런 끝에서 읽었으면 fork 설계가 빈 테이블 위에서 eligible 로 조작될 뻔(컴파일러가 잡음). **측정 = 실사용 7종 + corpus 72 전부 적격 → S0 중단 판정 통과, S1 go**. keccak 호출형 = `able 1/4 · frame_bodies 3` 이 JSON 한 줄(round-26 맹점의 계기화). ≥30× 기준 vs v1 string 거부의 모순은 **열어 둔 채 기록**(preview/21 §7.3.1) …
- `4.5.284` **외부 round-28 — IEEE 1364-2005 §3.5 암시적 net 선언을 구현했다.** 리포터가 상용 ASIC 트리의 `E3010` 97 + `E3009` 3 건을 사이트별로 "표준에서 합법인가"로 분류했고 **고유 원인 7개 중 6개가 vita 갭**이었다. 핵심 = vita 가 항상 `BTdefault_nettype none` 처럼 동작했다(doc-15 가 **의도된 정책**으로 명문화). 보수적이지만 **비준수**이고, 결정적으로 **사용자가 고칠 수 없다** — 7개 중 2개가 파운드리 납품 셀 라이브러리/IP 모델 안이다. 안전은 refusal 대신 **`W2003`**(doc-15 가 이미 예약해 둔 코드)이 산다. **경계는 iverilog 로 핀**(rhs·procedural lvalue·`none` 은 전부 error). 8-D(12→1비트 절단)는 값은 iverilog 와 같게 두고 **폭을 말하는 W3056** 을 낸다 — 모든 시뮬레이터가 조용히 하는 일을 vita 만 말한다. 부수: `BTdefault_nettype` 디렉티브 · `specparam` · 9-A 진단(EVENT CONTROL 을 lvalue 라 부르던 것) …
- `4.5.283` ★★ **외부 round-27 — `@(*)` 가 attribute instance 로 렉싱되어 소스가 통째로 삼켜졌다.** 이 저장소가 받은 **최고 심각도** 리포트: `//` 주석 안의 `*)` 가 감도 리스트의 `(*` 를 닫아 **주석이 실행 코드로 승격**되고 `errors=0` 으로 **틀린 값**이 나왔다(correct-or-loud 위반). 뿌리 = attribute 스킵이 **원문(raw text) 정규식**이라 주석·문자열을 뚫고 지나가고, 짝을 못 찾으면 **조용히** 폴백했다 — 그래서 발현 여부가 **컴파일 단위 전체의 `(*`/`*)` 개수**에 달려 `@(*)` 하나면 통과·둘이면 파괴, 진단은 **원인이 아닌 두 번째 블록**에 찍혔고 **파일 경계를 넘었다**. 수정 = attribute 를 **토큰 스트림**에서 인식(주석은 이미 사라졌고 문자열은 한 토큰) + `@` 직후는 event control + 안 닫힌 opener 는 loud. 3-way(PRE/POST/iverilog) 16형 전수 **회귀 0 · 수정 8** …
- `4.5.282` **③층 격차를 처음으로 쟀다(76×) — 그리고 ②층이 고갈되지 않았음을 알았다** · 외부 round-26 §3(진단 꼬리 절이 "동작한다"에 붙어 **동작하는 형태가 subset 밖**인 것처럼 읽히던 것)을 구조적으로 고치고(`(what, detail)` 분리), verilator 5.050 을 들여 **같은 설계·같은 기계로 3개 층**을 나란히 쟀다. 새 1st-party 벤치 `bench/keccak`(Keccak-f[1600], 오라클 4중 일치). 최대 발견은 격차가 아니라 **커버리지**였다 — 사용자 함수를 부르는 프로세스를 `is_codegen_able` 이 통째로 거부해 **VM 기여가 정확히 0%**, 거기 **10.7×** 가 미청구로 남아 있다 …
- `4.5.281` 외부 round-25 4건 — ★ `string` 원소 읽기가 서브루틴 안에서 **O(len)** 이라 문자당 루프가 O(len²)였다(N=32000 에서 172×) · `automatic <unpacked struct>` 서브루틴 지역변수 파서 구멍 · body-step 가드가 `max_deltas` 를 빌려 써서 정상 `for` 루프를 "combinational oscillation" 으로 오진하던 것(→ `F4027` 신설) · E3009 문구 …
- `4.5.280` 백엔드 **default 를 VM 으로 뒤집었다** — 근거는 72-디자인 differential 이 아니라 **전 스위트가 그 default 로 통과**한다는 것 · 뒤집자 기존 differential 셋이 **VM 대 VM** 이 되어 공허해지는 것을 같이 고쳤다(양변 명시 + default 값 별도 핀) · 그리고 **iverilog 대조 수치를 정정**했다(cold 첫 실행을 쟀다 — vita 가 1.28x **느리다**) …
- `4.5.279` **백엔드 default 를 뒤집어 전 스위트를 돌렸더니 18 타깃 39건이 터졌다** — P5 게이트(72 디자인)는 그 전부를 초록으로 통과하고 있었다. 뿌리 넷: 네이티브 레지스터가 `(val,unk)` 한 쌍이라 **`is_real`/`is_str` 를 나를 수 없는데 `try_compile` 에 타입 게이트가 아예 없었다**(`assign w = real` 이 IEEE 비트를 그대로 뱉었다 — **내가 §4.5.278 다음 커밋에서 만든 회귀**) · 바디 prologue 가 **복사되어 표류**(3개 중 1개만 · `%m` 과 시간 정밀도) · `c = new` 는 **StmtId 사이드테이블에 의미가 있는데 분류기는 IR 만 봤다** · 그리고 intercept 목록이 정본(`sysfunc_is_stmt_effect`, `_` arm 없는 exhaustive)의 **손복사본**이라 seeded `$dist_*` 7개 중 1개만 담고 있었다 …
- `4.5.278` 외부 round-23 — 호출의 **copy-out 목적지**를 분류기가 아예 안 보고 있었다(그건 `Stmt` lvalue 가 아니라 call-site 사이드 테이블에 있다) · 옆 문장이 답을 정하던 마지막 자리 · E3009 문구가 **패닉하는 형태를 "동작한다"고 명시**하던 것 · 그리고 loud 를 걷자 드러난 `StrPutC` 프레임-로컬 silent-wrong · §3.2 성능은 **iverilog 가 같은 깊이 스케일링**임을 실측(리포트의 뿌리 가설 반증) …
- `4.5.277` 외부 round-22 — 실행기 선택이 **무관한 `$display` 한 줄**에 달려 있었다(분류기가 문장을 목적지로만 봤고 효과는 rhs 에 있었다) · 함수도 같은 뿌리 · static task 의 `string` 로컬은 **세 번째 수집기** · fatal 이 안 멈추던 것 · 그리고 "이름 없던" 패닉 조건에 이름을 붙였다(목적지가 프레임 창 밖인가) …
- `4.5.276` 외부 round-20 — **내가 만든 회귀**(모듈 전역 키의 함수 전체 stand-down 이 무관한 dyn arm 을 껐다) · `inout` copy-in 이 죽었음의 증명 · 루프 trip-count · 그리고 그것을 고치다 만든 silent-wrong 5건(fold 도메인·resolver·스코프 교차·decl-init·재선언) …
- `4.5.275` 값을 반환하는 output-formal 호출을 **아무 표현식 위치에서나** — 한 shape 서술을 네 워커가 공유 · 조건부 자리는 guard 블록 · 왼쪽 읽기는 pre-call 스냅샷 …
- `4.5.274` 외부 round-19 — 값을 반환하는 호출의 output actual(33/34) · `void'(f(out))` 문장 · named arg 매핑 · 그리고 그 밑의 silent-wrong 2건(default 인자 스코프 · frame body 안의 파일 읽기) …
- `4.5.273` 외부 round-18 — suspend 하는 callee(11/12) · struct 멤버 비트 커버리지 · `automatic` unpacked struct 의 lifetime 이 파서에서 사라지던 것 · 그리고 그 밑의 silent-wrong …
- `4.5.272` `-v` 유효 invocation echo — 그리고 그것이 드러낸 filelist 플래그-값 결함(`--top` false-loud · `--hier-tree` silent 위치) …
- `4.5.271` 오라클을 만들다 나온 silent-wrong 2건 — 리시버를 못 보는 참조 워커 · `atoi` 계열이 `strtol` 이었다 …
- `4.5.270` 안 쓴 로컬은 per-entry 저장과 바이트 동일 — 그리고 그걸 열자 §23.9 구멍이 드러났다 …
- `4.5.269` 외부 round-17 §3.1/§3.1b/§3.3 — arm 하나가 없었고, catch-all 하나가 이미 쓴 걸 잊고 있었다 …
- `4.5.268` 외부 round-16 §3.4~§3.7+§4 — 두 단계가 스코프를 다르게 중첩(hoist 는 평평, lowering 은 중첩) …
- `4.5.267` 고정 크기 `automatic` unpacked 배열 — per-entry 리셋은 측정으로 기각(저장은 activation 단위) …
- `4.5.266` definite-assignment 이 제어 흐름(break/continue)과 callee 본문을 본다 — 리포트 84건 중 53건 …
- `4.5.265` 초기화자 소유권을 랭크 경로로 — bool 은 중첩된 두 generate 를 못 가른다 …
- `4.5.264` gen-item 리스트의 맨몸 `begin…end` 도 문법 — 라벨 유무가 경계 …
- `4.5.263` generate REGION 은 스코프가 아니라 문법 — 함수 하나가 두 역할을 하고 있었다 …
- `4.5.262` bind band 를 인스턴스 경계에서 리셋 — 두 라운드 전 자기 규칙을 새 플래그에 적용 …
- `4.5.261` 인스턴스 랭크 성분 3개(band·key·sub) — root 순서·`bind` 위치·배열 원소가 각각 깨져 있었다 …
- `4.5.260` 인스턴스 랭크를 선언 오프셋으로 — 인터페이스(Nets)와 모듈 자식(Instances)은 카운터로 못 섞는다 …
- `4.5.259` 초기화 phase 적대 리뷰 — 하강 4(dirty 통째 clear·슬롯 공유 카운터·랭크 없는 package flush·인터페이스 슬롯) + false-loud 1 …
- `4.5.258` generate 안 블록 로컬 — 분류기가 generate 프로세스를 아예 안 보고 있었다 …
- `4.5.257` 초기화는 프로세스가 아니라 **arm 이전 PHASE**(이벤트 0) · 상수 fold 제거로 자기모순 해소 …
- `4.5.256` t0 초기화 순서를 랭크 경로 데이터로 — 소유권 축과 초기화 축 분리 …
- `4.5.255` 같은-이름 `string` 배열 correct-support + 리뷰 2연(generate 본문엔 prefix 가 없다 · 소유권은 플래그) …
- `4.5.254` t0 정적 초기화자 순서 = 모듈 전부 → 블록 로컬 전부(실측) · 리스트 3개를 하나로 …
- `4.5.253` §4.5.251 적대 리뷰 — 하강 4건(kind-only 술어·선언 순서 2·gather 확장이 뺏은 스코핑) …
- `4.5.252` `$sformatf` — 근인은 포맷을 무시하는 degenerate `eval` arm, hoist 는 그 우회였다 …
- `4.5.251` `$blk$` 경로 decl-init 수집 — 제외 3개(초기화자·스칼라 string·multi-name)가 함께 사라짐 …
- `4.5.250` §4.5.248/249 적대 리뷰 — 사다리 하강 6건(평가 이동 5 + 게이트 극성 1) …
- `4.5.249` 외부 round-20 §6+§4.11 — elaborate 진단 file:line · 같은 이름 동적 로컬 분리 …
- `4.5.248` 외부 round-20 8 가족 — fork-arm 블록 로컬 · queue 관용구 · named arg · `$sformatf` …
- `4.5.247` §4.5.246 회귀 수정 — flatten 블록 로컬이 generate 스코프를 shadow(적대 리뷰 발굴) …
- `4.5.246` inner NET 이 outer PARAM 을 shadow — 마지막 ①-급 해소(퍼널은 이미 있었고 fall-through 가 무시했다) …
- `4.5.245` inner-NET shadow 메커니즘 확정(트리거=param 이름충돌·범위는 task/블록까지·근인=해석 순서) …
- `4.5.244` 남은 대형 3건 착수 판단표(C>A>B) — 사이드카 우회 불가·B payoff 작음을 실측 …
- `4.5.243` generate case 의 real scrutinee = 비목표 확정(iverilog 도 거부) — real 가족 완결 …
- `4.5.242` generate 제어식 real 라우팅(`const_truth_in_scope`) — "real→정수 문맥" 완결 …
- `4.5.241` generate 스코프 `localparam real` loud→correct-support(퍼널은 이미 있었고 한 호출부만 안 쓰고 있었다) …
- `4.5.240` `$value$plusargs` 크기 추정 정정(관용 배치 2종 이미 동작·남은 loud 는 패밀리 불변식) + if-cond 핀 …
- `4.5.239` 스캔 4차 완료 — `$typename` 핀(iverilog 미구현)·`%u`/`%z`/`%l` 정리 …
- `4.5.238` 스캔 3차 honest-loud 확인 + LOOPROMPT §8 압축(20142→18934B, 규칙 무삭제) …
- `4.5.237` 테스트 0건 스펙 스캔 2차 — `$sformat`/`$swrite`/plusargs 검증 후 핀(결함 없음) …
- `4.5.236` `%p` 가 real 을 정수 반올림하던 silent-wrong(`2.5`→`3`) — 무오라클 스펙, 테스트 0건이었다 …
- `4.5.235` fresh-area 스윕 CLEAN + 무오라클 능력 2건(modport 포트·함수결과 part-select) 핀 …
- `4.5.234` sized-literal enum label — enum 메서드 전부 loud→correct-support(두 술어를 "합의 가능한 부분집합"으로 봉인) …
- `4.5.232` `real` const-fold — 실수 산술 loud→correct-support(§11.8.1 순서가 핵심; i64 twin 확장은 5건 silent-wrong 을 열어 철회) …
- `4.5.231` 모듈 스코프 상수식 = **비목표 판정**(iverilog 3갈래 자기모순 실측) + vita 자기일관성 teeth …
- `4.5.230` 상수함수 인터프리터 폭 인식 — 좁은 대입 대상이 안 잘려 `localparam W=f()` 가 조용히 틀렸다(내부 차분: 인터프리터 vs 런타임) …
- `4.5.229` 상수식 BOUND/COUNT 단일 퍼널 — part-select 폭·replication count·indexed part 폭 silent-wrong 8가족 (`const_bound_u32` + `Cast` const arm; 가드는 리프가 아니라 값) …
- `4.5.228` round-20 8항목: fork-arm 재개 🔴 · 음수 하한 unpacked 🔴 / packed(multi-packed 는 silent) · 동시 활성화 dyn 배열 · generate/interface 스코프 · VCD 선언범위 · `$fmonitor`/`$fstrobe` (format 23→25) …
- `4.5.226` §0 T1-6: 계층 dynamic-container element READ loud→supported (`u.s[0]`·`u.d[0]`·`u.q[0]`) …
- `4.5.225` §0 T1-7: task/function body-local string ARRAY loud→supported (frame-entry pre-size) …
- `4.5.224` §0 T1-5: multi-dim fixed string array loud→supported (row-major flat 컨테이너) …
- `4.5.223` §0 T1-4: `string q[$]` loud→supported + dynamic-container element 단일 퍼널 …
- `4.5.222` §0 T1 부분: FIXED string array의 RUNTIME 인덱스 + `foreach` loud→supported (zero-based ascending만 라우팅) …
- `4.5.221` real-valued parameters (`parameter real`) loud→supported — 정수-상수 문맥은 loud (적대 리뷰 6라운드·머지됨) …
- `4.5.220` SILENT-WRONG 수정: DYN string-array element의 byte select가 0 → supported + write-…

**§4.5.210–219**
- `4.5.219` FIXED string-array decl-init `string s[N] = '{…}` loud→supported (t0 pre-sweep…
- `4.5.218` SILENT-WRONG 수정: inner-scope local이 OUTER string-array side-map을 shadow 못 하던 문…
- `4.5.217` SILENT-WRONG 수정: string-ARRAY ELEMENT가 concat/replicate/relational에서 packed로 새…
- `4.5.216` round-19 follow-on: F-record-out short-circuit output-formal call — if-cond + …
- `4.5.215` round-19 리포트 4-가족(BL·Q·F-struct·F-record-out) loud→correct-support (리포트 2-오진 정…
- `4.5.214` fork…join[_any|_none] inside a suspendable task body loud→supported (Case A/B …
- `4.5.213` round-18 리포트 대응: 8-가족 loud→supported + C1 const-repeat
- `4.5.212` SILENT-WRONG 수정: size-cast `N'(expr)` context-width 미전파 → supported (size-cast…
- `4.5.211` NON-ZERO-BASE DESCENDING unpacked-array formal loud→supported (over-conservati…
- `4.5.210` forwarding a frame task/function's OWN unpacked-array FORMAL into a nested hie…

**§4.5.200–209**
- `4.5.209` hierarchical TASK enable OUTPUT/INOUT unpacked-array formal loud→supported (de…
- `4.5.208` hierarchical TASK enable NESTED in a frame-task body loud→supported (format_ve…
- `4.5.207` hierarchical TASK enable with an INPUT unpacked-array formal loud→supported (d…
- `4.5.206` NON-ZERO-BASE ASCENDING array formal loud→supported (per-dim base threaded int…
- `4.5.205` DESCENDING / mixed-direction multi-dim array formal loud→supported (over-conse…
- `4.5.204` OUTPUT/INOUT multi-dim array formal loud→supported (multi-index copy-out unpac…
- `4.5.203` STATIC (non-`automatic`) task with a FIXED unpacked-array formal loud→supporte…
- `4.5.202` multi-dimensional array FORMAL on framed function / `task automatic` loud→supp…
- `4.5.201` hierarchical task OUTPUT/INOUT scalar formal loud→supported (cross-boundary co…
- `4.5.200` STATIC-task hierarchical call `u1.tk()` loud→supported (frame↔inline parity, s…

**§4.5.190–199**
- `4.5.199` multi-dim frame-LOCAL array loud→supported (frame↔inline parity, step 2 — 마지막 …
- `4.5.198` frame task MODULE array-element write loud→supported (frame↔inline parity, ste…
- `4.5.197` hierarchical TASK call `u1.tk(x)` loud→supported (+ frame-aware suspend-classi…
- `4.5.196` round-17 D-가족(minor 잔여) tractable 4항목 loud→supported
- `4.5.195` round-17 리포트 3-가족(A/B/C) + CLI 이원-top 전부 loud→supported
- `4.5.194` round-16 executor-bound gaps 전부 loud→supported (dyn_heap RefCell interior-muta…
- `4.5.193` output unpacked-fixed array formal on TASK loud→supported (md-packed slot → pa…
- `4.5.192` packable unpacked-struct scalar body-local in task/function loud→supported (V8…
- `4.5.191` FIXED 1-D array of packable unpacked struct loud→supported (V6)
- `4.5.190` arr[i].field on packed-struct 1-D array loud→supported

**§4.5.180–189**
- `4.5.189` SILENT-WRONG 수정: loop-body block-local initializer ran once, not per-entry
- `4.5.188` input unpacked-fixed array formal on TASK loud→supported
- `4.5.187` $fopen runtime filename loud→supported (string literal → variable/concat/packe…
- `4.5.186` constant-function evaluation in const contexts loud→supported (elaborate-time …
- `4.5.185` `$bits(TYPE)` loud→supported (parser type-size fold)
- `4.5.184` multi-dimensional packed array struct/union member loud→supported (parser flat…
- `4.5.183` SILENT-WRONG 수정: block-local `string s[] = '{…}` dyn-array init이 조용히 drop → su…
- `4.5.182` queue / dynamic-array `{…}` (unpacked-array concat) decl-init loud→supported (…
- `4.5.181` enum `.next(N)` / `.prev(N)` CONSTANT-step loud→supported (parser N-step terna…
- `4.5.180` SILENT-WRONG 수정: same-named STATIC block-locals in DISJOINT procedural blocks …

**§4.5.170–179**
- `4.5.179` FRAMED function dyn-formal call BURIED in an expression loud→supported (R5-B h…
- `4.5.178` SILENT-WRONG 수정: writing a dynamic-array `input` formal in a synchronous frame…
- `4.5.177` FRAMED function `input` dynamic-array formal loud→supported (DynArray reserve …
- `4.5.176` `foreach` over dynamic array/queue/assoc inside a FUNCTION / SUBSET-task body …
- `4.5.175` SILENT-WRONG 수정 — `foreach` over dynamic array/queue/assoc inside a FUNCTION /…
- `4.5.174` inline static-task `foreach`-on-dyn-formal loud→supported (dyn_handle → dyn_ha…
- `4.5.173` V2A-frame — AUTOMATIC task `input` dynamic-array formal loud→supported (per-ac…
- `4.5.172` frame-body validator over-scan false-REJECT 수정 (linear scan → reachable-block …
- `4.5.171` V5 — frame-local (task-body) DYNAMIC array loud→supported (per-net heap + reen…
- `4.5.170` V2A — TASK `input` dynamic-array formal loud→supported (static task·R2 dyn_sub…

**§4.5.160–169**
- `4.5.169` frame-local unpacked ARRAY loud→supported (md-packed frame slot, array-FORMAL …
- `4.5.168` V3/V4 suspendable tasks — `@`/`#`/wait/NBA/$systask/재귀/nested in a task body r…
- `4.5.167` round-14 loud→supported: 64-bit MSB-set 리터럴 fold(V9) + frame-local string(V1) …
- `4.5.166` comb sensitivity 읽기집합 완전성 — LHS-index + 계층 ref silent→correct (SV §9.2.2.2.1)
- `4.5.165` enum label 범위검증 — out-of-range label을 silent-truncate→loud (SV §6.19)
- `4.5.164` enum `.name()`/`.name` — SV §6.19.5 label-string method (loud→supported)
- `4.5.163` untyped param — PACKAGE-scoped alias 값-결정 타입 (§4.5.162 후속)
- `4.5.162` untyped param value-determined type — IDENT/EXPRESSION initializer (§4.5.161 완…
- `4.5.161` untyped param value-determined signedness+width (IEEE §6.20.2)
- `4.5.160` body param/localparam comma-list — `localparam A=1, B=2;` (loud→supported)

**§4.5.150–159**
- `4.5.159` ANSI `#(…)` param-port comma-list — continuation이 type prefix 상속
- `4.5.158` enum label operand signedness — 선언 sign 상속   [.vu re-pin]
- `4.5.157` atom+packed-dims loud-reject — 全 decl-site 완결 (§4.5.156 follow-through)   [§3 …
- `4.5.156` loud-reject a packed range/dimension on a non-vector type (byte/int/…/real/str…
- `4.5.155` `$bits` of a fixed-width atom array element (byte/shortint/int/longint)
- `4.5.154` enum built-in base kind preservation — width + 2-state-ness (byte/shortint/int…
- `4.5.153` enum base-type signedness — vector-base `signed` + atom-base `unsigned` whole-…
- `4.5.152` signed packed struct/union whole-value signedness + case collective-signedness
- `4.5.151` 전면 감사(spec↔코드 정합 + 적대 버그헌트 + 문서 최신화) — silent-wrong 3·panic 2·게이트 갭 1 수정, form…
- `4.5.150` FST: fst-writer 0.2.6→0.3.1 + MSRV 1.82→1.85 (Surfer 상호운용 수정)

**§4.5.140–149**
- `4.5.149` FST waveform output (`$dumpfile("x.fst")` / `-o x.fst`) — G2 breadth
- `4.5.148` explicit single-symbol TYPE import (`import p::t;`) loud→supported
- `4.5.147` width-63 param `coerce_i64_to_width` overflow-panic fix (robustness)
- `4.5.146` compound-const `==?`/`!=?` wildcard-literal fold (loud→supported)
- `4.5.145` md-packed nested part-select WRITE `x[j][m:l]`/`arr[i][j][m:l]` (loud→supporte…
- `4.5.144` `%s` 출력 문자열 fidelity: 숫자-const NUL→space + bare string-var template
- `4.5.143` runtime `$clog2(real)` round-then-count (silent-63 → correct)
- `4.5.142` `%d`-of-real 기본 필드폭 제거 + fresh-area sweep
- `4.5.141` real `**` 지원(loud→supported·$pow desugar)
- `4.5.140` 지연 게이트/cont-assign 출력=초기 X (Z 아님)

**§4.5.130–139**
- `4.5.139` VCD VALUE 검증(정확)+회귀 가드·fresh-area sweep
- `4.5.138` VCD `$var` `[msb:lsb]` bit-range reference
- `4.5.137` `$fflush` = 인식된 no-op (오진단 warning 제거)
- `4.5.136` neg-range-bound 진단 정직화 + grounding sweep
- `4.5.135` `$time`/`$stime` 반올림 + `$stime`-in-`$monitor` 제외
- `4.5.134` round-11 N3 — heterogeneous heap: mixed 2-/4-state + string/real record arrays
- `4.5.133` round-11 N3 — packable-record dynamic array: read + element/member write
- `4.5.132` round-11 R2 — read-only input dynamic-array function formal via caller-net ali…
- `4.5.131` round-11 R5 — unpacked-struct tf-port + function output/inout formal copy-out
- `4.5.130` external round-11 report — 9/12 string/subroutine gaps loud→supported (string …

**§4.5.120–129**
- `4.5.129` external round-10 report — twelve tb/*.sv gaps (string method/return, dyn/sign…
- `4.5.128` `$sformatf(...)` as a `$sformat`/`$swrite*` (string-dest) VALUE argument loud→…
- `4.5.127` `$sformatf(...)` as an immediate-format-task VALUE argument loud→supported (ho…
- `4.5.126` bare-statement `$dist_uniform(seed,…);` silently drops the seed writeback → ad…
- `4.5.125` bare-statement `$random(seed);` silently drops the seed writeback → advances i…
- `4.5.124` `$clog2(const)` SINGLE-Const replication-count / part-select-width silent-0 → …
- `4.5.123` bare-statement `$value$plusargs`/`$fgetc`/`$ungetc` silently drops side-effect…
- `4.5.122` bare-statement SYS-READ (`$sscanf`/`$fscanf`/`$fgets`/`$fread`) silently no-wr…
- `4.5.121` non-constant replication count silent-`000` → loud
- `4.5.120` `+` (force-sign) format flag loud→supported + real zero-pad (`%08.2f`)

**§4.5.110–119**
- `4.5.119` `-` (left-justify) format flag loud→supported + `%c/%v/%m` field width
- `4.5.118` localparam/parameter non-zero-LSB part-select silent-wrong → correct
- `4.5.117` multi-dim packed ELEMENT sub-select of a non-zero-LSB inner dim silent-wrong →…
- `4.5.116` interface-member non-zero-LSB READ sub-select silent-wrong → correct
- `4.5.115` hierarchical part-select READ of a non-zero-LSB net silent-wrong → correct
- `4.5.114` packed-struct member NEGATIVE-LSB sub-select silent-wrong → loud
- `4.5.113` packed-struct member NON-zero-LSB sub-select — READ silent-wrong→correct · WRI…
- `4.5.112` external round-9 report — PKG2 (same-pkg const body refs + `pkg::T'()` cast) ·…
- `4.5.111` external round-7 report — PKG: package-scoped subroutine call `pkg::f()` + UAR…
- `4.5.110` external round-6 report — UARR: unpacked-array subroutine formals (IEEE §13.3)…

**§4.5.100–109**
- `4.5.109` external round-4 RESIDUAL — concat pkg-operand width(Issue1)·replication const…
- `4.5.108` external round-5 report — per-block scope for colliding block-local `automatic…
- `4.5.107` external round-4 report — package-array generate fold(G)·guard-aware block-loc…
- `4.5.106` external round-3 report — generate-scope localparam(G)·block-local `automatic`…
- `4.5.105` `@(pkg::sig)` scoped-package event control (② loud→supported·§5 소형잔여 NEXT #5)
- `4.5.104` package/module typedef bare-NAME leak → per-unit type scoping (① oracle-backed…
- `4.5.103` LOCAL array-element part/indexed-select 非-제로-LSB silent-wrong → correct (① ora…
- `4.5.102` package-scoped ELEMENT/DIRECT-select read 지원 + array-element sub-select loud-g…
- `4.5.101` OBS-3 — `$vita_stage` → stage.jsonl (R-S3) — vendor stage-trace task (G2 관찰 ra…
- `4.5.100` OBS-2 (v1) — `trace.jsonl` (R-L3): `--probe` net change-stream (G2 관찰 rail)

**§4.5.90–99**
- `4.5.99` OBS-1b — `coverage.json` (R-L5): N5 functional coverage export (G2 관찰 rail)
- `4.5.98` MODULE sibling-block string coalesce READ silent-wrong → read-gated loud-rejec…
- `4.5.97` EXT2-H — FRAME 함수/태스크 body의 bit/part-select 대입 `r[7:0]=x` (외부 리포트 2차·§4b H·lou…
- `4.5.96` `integer unsigned` PARAM/VAR parse-loud → supported (§4.5.91 companion·loud→su…
- `4.5.95` INLINE 함수 sibling-block same-name string/packed 충돌 silent-wrong → loud-reject …
- `4.5.94` FRAME 함수 string FORMAL element-select `s[i]` silent 0/X → 실제 byte (§C priority…
- `4.5.93` INLINE(4-state-return) 함수 string LOCAL/FORMAL element-select `s[i]` silent bit…
- `4.5.92` INLINE 함수 multi-dim PACKED local silent-wrong (whole-truncate + element-select…
- `4.5.91` `int/byte/shortint/longint unsigned` PARAMETER 부호 drop silent-wrong (§C 소형 잔여)
- `4.5.90` class METHOD·CONSTRUCTOR omitted DEFAULT-ARG drop silent-wrong (§4.5.89 적대리뷰 발…

**§4.5.80–89**
- `4.5.89` CLASS-METHOD `string` formal 2종 silent-wrong (relational packed-compare + LITE…
- `4.5.88` TASK `string` formal 2종 silent-wrong (frame relational packed-compare + LITERA…
- `4.5.87` FRAME 함수 `string` LITERAL 인자 truncate silent-wrong (§4.5.81 발굴 잔여·1-bit Wire 슬…
- `4.5.86` 2-state(`bit`) class FIELD X→0 coercion 부재 (§4.5.85 sibling·object-heap 경로)
- `4.5.85` class-method 2-state(`bit`) formal/return/local X→0 coercion 부재 (§4.5.83 발굴·fr…
- `4.5.84` string LOCAL relational-compare silent-wrong (inline subst-bound local resize …
- `4.5.83` class-method multi-dim packed local element-select silent-wrong (§4.5.82 4번째 n…
- `4.5.82` ㊂ frame-local multi-dim packed array element-select silent-wrong (full width +…
- `4.5.81` string-formal relational compare silent-wrong (declared-type detection)
- `4.5.80` ㊁ INLINE static-function widening context-width silent-wrong (frame-routing·§1…

**§4.5.70–79**
- `4.5.79` EXT2-F — generate-for step `g++`/`g--`/`g op= e` (외부 리포트 2차·§4b F)
- `4.5.78` ㊀ narrow-actual INLINE function arg not width-extended (silent-wrong 수정·EXT2-C…
- `4.5.77` EXT2-C — packed struct/union typedef tf-port `function f(input cfg_t c)` (외부 리…
- `4.5.76` EXT2-E — scoped type `pkg::t` in type positions (외부 리포트 2차·port 8곳+그라운딩 확대)
- `4.5.75` EXT2-A — 컨테이너 end-label `endmodule : m` 계열 전수 (외부 리포트 2차·14/18 파일 차단)
- `4.5.74` EXT2-0 — `$fgets(string, fd)` silent no-op 수정 (외부 리포트 2차 최상위·유일 silent-wrong)
- `4.5.72` generate/interface-scope 배열 `'{…}` decl-init silent-drop 수정 + array-param 게이트 …
- `4.5.71` A2b — package array parameter + package §6.8 pre-sweep
- `4.5.70` A2b-prereq — package-level 변수 저장 (IEEE §26, iverilog 라이브 차분)

**§4.5.60–69**
- `4.5.69` A2a — module-body array parameter `localparam <ty> X [dims] = '{…}` (§6.20.2, …
- `4.5.68` assoc typed-key 스펠링 `[int]`/`[longint]`/`[shortint]`/`[byte]` (§7.8, hand-IEEE…
- `4.5.67` sigpipe 스레드 EPIPE panic race 수정 (robustness)
- `4.5.66` `unique0` / `priority0` 수식어 (§12.4.2, hand-IEEE)
- `4.5.65` queue slice read `dst = src[a:b]` (§7.10.1, hand-IEEE)
- `4.5.64` D docs sync — 사용자 매뉴얼 리얼리티 스윕
- `4.5.63` whole-handle copy `dst = src` (queue/dyn/assoc §7.5.1/§7.9/§7.10)
- `4.5.62` wildcard equality `==?`/`!=?` §11.4.6 + 코어 `==`/`!=` definite-mismatch 수정
- `4.5.61` pre-opened file descriptors STDIN/STDOUT/STDERR §21.3.4
- `4.5.60` `%t`/`$timeformat` full §21.3.2 semantics + proc_multipliers u64

**§4.5.50–59**
- `4.5.59` SIGPIPE: no panic on `vita design.sv | head`
- `4.5.58` numeric cast to a typedef name `mode_e'(raw)`
- `4.5.57` `$value$plusargs` in an `if`-condition `if ($value$plusargs("L=%d", n)) …`
- `4.5.56` user-defined type name as a MODULE port type `module m (input mode_e mode)`
- `4.5.55` ANSI module-header package import `module m import p::*; (ports);` + undefined…
- `4.5.54` string element indexing `s[i]` read/write — byte-index not bit-select
- `4.5.53` enum BASE type given as a typedef name `typedef enum b_t {R,G,B}`
- `4.5.52` block-local struct VAR shadow scoping — `var_struct` flat-map clobber fix
- `4.5.51` procedural body-local typedef DEFINITION `function f; typedef logic[3:0] t; t …
- `4.5.50` tf-port type given as a typedef name `function f(byte_t a)`

**§4.5.40–49**
- `4.5.49` chained typedef alias `typedef base_t alias_t;`
- `4.5.48` packed struct/union MEMBER with a user-defined type name `typedef struct packe…
- `4.5.47` function/task default argument values `function f(int a, int b = 10)`
- `4.5.46` multi-dimensional unpacked array assignment pattern `int a[2][3]='{'{1,2,3},'{…
- `4.5.45` queue / dynamic-array `'{…}` declaration initializer
- `4.5.44` multi-dimension `foreach (a[i,j,…])`
- `4.5.43` typed for-init with a user-defined type name `for (my_t i=0; …)`
- `4.5.42` inline function return value + body locals resized to declared width/sign
- `4.5.41` function RETURN type via user-defined type name `function b_t f;`
- `4.5.40` block-local / tf-body declaration with a user-defined type name `initial begin…

**§4.5.30–39**
- `4.5.39` block-local unpacked-array decl-init `initial begin int a[4]='{1,2,3,4}; end`
- `4.5.38` ANSI tf-port comma-shared type stickiness `task t(input logic [7:0] x, y)`
- `4.5.37` packed-struct `'{…}` pattern on a 1-D struct-array element `arr[i]`
- `4.5.36` packed-struct `'{…}` pattern in continuous/force/for assign contexts
- `4.5.35` packed-struct positional assignment pattern `s = '{e0,…,eN}`
- `4.5.34` cast of a fill literal takes the cast width (`8'('1)`=`ff`)
- `4.5.33` positional assignment pattern `'{…}` — 1-D unpacked array
- `4.5.32` block-local `string s = expr;` 선언 초기화자
- `4.5.31` `string s = expr;` 선언 초기화자 (module scope)
- `4.5.30` block-local scope leak honest-loud detection

**§4.5.20–29**
- `4.5.29` `.*` 암시적 wildcard 포트 연결
- `4.5.28` `.name` 암시적-named 포트 연결 shorthand
- `4.5.27` `defparam inst.param = value` 직접-자식 override
- `4.5.26` zero-count replication `{0{x}}` width
- `4.5.25` `%v`/`%V` multi-bit strength
- `4.5.24` uppercase format spec letters `%S/%C/%M/%E/%G/%T`
- `4.5.23` explicit `%Ns` field-width for `%s`
- `4.5.22` `%0s` of packed reg leading-null strip + `%s`/`%0s` x/z byte→space
- `4.5.21` signed `%d` default 필드폭 (sign column)
- `4.5.20` `$swrite`/`$swriteb`/`$swriteo`/`$swriteh` + Sformat dest 가드

**§4.5.10–19**
- `4.5.19` bare event control `@e` (no parens)
- `4.5.18` (개발 후보·deferred) frame/task 바디 inner-block local이 module var collision (pre-ex…
- `4.5.17` function/task가 module-level 변수 READ (frame path)
- `4.5.16` function이 module-level 변수 참조 (READ) — §4.5.17서  해결
- `4.5.15` positional assignment pattern `'{e0,…,eN}` (SV §10.9) — UNPACKED array (§4.5.3…
- `4.5.14` enum value methods (`.first/.last/.num/.next/.prev`)
- `4.5.13` break / continue loop control
- `4.5.12` increment/decrement + compound assignment (`i++` `i += e`)
- `4.5.11` net-declaration delays (`wire #3 w = a;`)
- `4.5.10` wand/wor wired-logic net resolution

**§4.5.0–9**
- `4.5.9` multi-driver continuous-assign wire resolution
- `4.5.8` wide 4-state posedge/negedge
- `4.5.7` gated/derived clock per-cluster edge collapse
- `4.5.6` 블로킹 self-write self-retrigger 수정
- `4.5.5` 블로킹/NBA 글리치 wake-collapse 수정
- `4.5.4` YELLOW 구현 배치
- `4.5.3` Honest-loud 배치 + 잔여 항목 트리아지
- `4.5.2` 추후 진행 — 개발 예정 플랜
- `4.5.1` Medium 묶음 게이트 플랜

## 완료 슬라이스 로그 (이관 이후 — 최신이 위)

#### 4.5.408 — A procedural read of a whole-net copy sees the writer's own write (§2 row 33) (2026-09-03 · format 29 unchanged · adversarial review: 2 lenses (differential 14 designs CLEAN · soundness 12 designs + read-site census, one FAIL fixed by narrowing the alias set) + delta re-verification · corpus 10/10)

**The cell.** `wire [7:0] c; assign c = v;` then, in one process, `v = 8'hA5; cap = c;`
latched `cap = 00` in vita on all three backends where iverilog and verilator both latch
`a5`. The copy is written by the continuous-assign settle, which runs between process
batches, so a read one statement after the writer's own write saw the value from before
it — and it persisted, since nothing re-ran the copy until the next delta. iverilog
COLLAPSES a whole-net copy into its source (its VCD gives both one identifier), which is
why `assign c = v[3:0]` is stale there too: the collapse, not eager evaluation, is the
oracle-side property. 91-cell census (4 widths × 3 source kinds × 5 destination kinds +
27 shapes: declaration-initializer form, parens, 2/3-deep chains, signed either side,
slice write, condition / index / `$display` reads, task write, function read, port
copies read from the parent, cross-process order, force, `$monitor`): **59 silent→correct
· 6 ok→ok · 20 oracle-split · 5 wrong→wrong · 0 regressed**.

**Built first and REFUSED — the store-side forward the row proposed.** Forwarding `v`'s
new words into every whole-net copy inside the write funnel (`store_words`, both stores)
fixed the cell and moved three things the oracle pins: picorv32's digest (`REGRESSION`,
the digest is cycle-resolution bus activity), a UDP shift-register chain
(`udp_seq_instance_shift_register_output_first`: the second DFF sampled its FRESH input
on the same clock edge, iverilog samples the old one) and native-vs-VM parity on keccak.
Three of the four example VCDs also changed (t0 values toward iverilog, plus in-slot
record order). The settle's consumers — UDPs, gates, downstream assigns — are
evaluated in an order that a same-slot store forward changes; iverilog's collapse is
coupled to its own functor order and cannot be emulated by pushing values earlier.

**Shipped — READ-THROUGH, static and procedural.** ① `alias::copy_alias`: net → root
source for the strict members of `copy_nets` (one driver, one source, both sides the
whole net, rhs a bare `Signal`), excluding 2-state destinations and any net that is a
`force`/`release` target. ② `levelize::proc_read_alias`: per-ExprId, for every process
whose `blocking_writes` include the root, every `Signal` node in that process's
statements and terminators reading a copy of it → the root. ③ `WidthTable::read_alias`:
the one sidecar every evaluator already carries, so `eval_core` (interp and every
`k_eval` fallback), `native_eval::lower` (VM bytecode) and `wprog::compile_node` (native
compiled) resolve the same read to the same net at compile/eval time. Nothing in the
settle, the store, the dirty/wake channel or the VCD changes: the examples' stdout AND
VCD are byte-identical, 6,465 tests green before the pins were added, corpus 10/10.

⚠️ **Scope, and why it is the whole defined case.** A read that follows the writer's
OWN blocking write in program order has one answer in every tool. A read in ANOTHER
process in the same delta is a §5.4.1 race whose outcome is process order — both
oracles print `a5` when the writer is declared first and `00` when the reader is —
and vita keeps the settle's value there (`xp_writer_first` is the one 2-oracle cell
left wrong by design). Residue → §2 🆕 I: that race cell, multi-driver with a z driver
(`copy_nets` excludes the group), a constant-index array-word source (`flat` refuses
the base), a blocking write inside a `fork` arm / non-inlined frame body (the writer
predicate walks the process's own blocks), the 2-state destination (a dead guard today —
vita refuses `bit` copy destinations at elaborate), a SIGN-MISMATCHED copy and an
`automatic` task's actual.

**Review.** Differential lens: 14 designs outside the census (128-bit arithmetic, a
function formal, hierarchical / interface / generate-scoped copies, runtime part-select,
`always_comb`, fork arm, NBA source, gates + `$dumpvars`) — every change lands on the
defined cell, three backends byte-identical, gates unchanged: CLEAN. Soundness lens:
read-site census (three consulting arms, every other `Signal` match a string/queue
handle or a write destination), table lifetime (installed after the settle programs
compile; no expression interning), and one **FAIL**: `wire signed [7:0] c; assign c =
v;` with an unsigned `v` then `r = c;` — the substituted net lent its sign to the
extension, so the interpreter and VM printed 255 (slot sign) while the native compiled
path printed the oracles' 4294967295 (node sign). Fixed by requiring the copy's declared
sign to equal its root's, beside the width equality `copied_source` already demands;
the cell reverts to PRE's stale value with backend parity, pinned as parity only.

Tests: `copy_net_read_through.rs` (5, every cell on all three backends, values
2-oracle). 6,465 → 6,470.

#### 4.5.407 — A reduction operator inside a declaration bound sizes the declaration; an untyped parameter takes a reduction's type (§3 ⑦) (2026-09-03 · format 29 unchanged · adversarial review: 2 lenses (differential 12 designs · soundness 10 designs + a 13-site census), PASS/PASS, one wording finding fixed, delta re-census after each re-freeze · corpus 10/10)

**The queue row said 12 silent cells; the census said 171.** The six §11.4.14 reduction
operators folded in the value domain since §4.5.382, but only through
`param_i64_at_declared`, which a declaration WITH a width reaches. Every position that
asks the module-scope i64 walk (`const_eval_in_scope`) — a packed range, an unpacked
dimension, a port range, a replication count, an untyped parameter's initializer — got
`None`, and the bound consumers read `None` as ONE BIT at exit 0. `wire
[((|4'b1010)+1):0] x;` was 1 bit where both oracles declare 3; `input [((|P)+7):0] p`
sized 1 bit and truncated a 12-bit actual to `0` across the module boundary; `logic x
[(|P)+2:0]` lost its dimension; `localparam R = |4'b1010;` was E3009. **Census 1**: 4
provenances (literal · sized-literal param · typed localparam · package constant) × 9
positions × 6 operators + 60 edge shapes = 320 cells → with the first arm alone **117
silent→correct · 43 loud→correct · 35 wrong→wrong**; with all four pieces **148
silent→correct · 46 loud→correct · 4 loud→(oracle-split, vita = verilator) · 0
loud→wrong · 4 wrong→wrong** (all x-plane). **Census 2** (102 cells: constant-function
formals and shadowing, `#()` and `-G` overrides, decimal and value-inferred provenance,
`!`/`?:`/`**`/select under the reduction, untyped-parameter type): **48 silent→correct ·
18 loud→correct · 6 loud→(split, vita = verilator) · 6 silent→loud · 0 loud→wrong · 4
wrong→wrong** (x-plane).

**Mechanism — four pieces.** ① `const_eval_in_scope` gets an arm for the six operators →
`selfdet_bits_i64(e)`: the wide bit domain, whose names resolve through `wide_name_bits`
= DECLARED provenance only (`param_range`, sized literal, type, package meta). That is
the whole soundness argument §4.5.373 missed and §4.5.382 established: the operand's
width never comes from `param_meta`. ② The width-aware twin `eval_const_env_at` had no
reduction arm either, and it is what a bound holding a parameter SELECT reaches
(`const_range_bound_fold`) and what `!`, a ternary condition and a `**` exponent reach
for their operand — so `wire [(|P[3:1])+2:0]` and `wire [(!(|4'b1010))+2:0]` stayed 1
bit after ①. The arm takes the wide domain when the operand names no constant-function
local (one text, one answer at module scope) and the local's `envw` width otherwise
(`fnshadow`: a 4-bit formal `P` shadowing an 8-bit module `P` reduces the formal). ⭐ It
is also what closed the 24 replication-count cells the first census left at a silent
`0000` — `const_bound_u32`'s third tier is this walk. ③ `param_decl_width_opt`: an
untyped parameter whose initializer TOP is a reduction or a `!` records `(1, unsigned)`
— a type fact (§11.8.1 / §11.4.7), so it answers under `declared_only`; without it
`$bits(R)` was 32 for a value both oracles hold at 1 bit, and the delta re-census caught
`localparam R = !(|4'b1010);` going loud → `0 32` before `!` was added. ④
`nonconst_bound_reason` / `unfoldable_reason`: a reduction that declined because
`wide_selfdet_width(operand)` is None says *"a reduction of an operand whose width is
not declared"* — `localparam E = 4'hF | 4'h0; wire [(&E)+2:0]` was a silent 1 bit (both
oracles 4) and is loud now.

⚠️⚠️ **The loud→silent trade this slice refused.** In an UNTYPED parameter, the
value-inferred tail sizes the initializer at ≥32 bits and the unlimited fold computes a
context-determined operator at i64, so once the reduction folds, `localparam R =
~(|4'b1010);` would print 4294967294 where both oracles print 0 at one bit — and
`(|4'b1010) << 2` → 4 (oracles 0), `-(|4'b1010)` → 4294967295 (oracles 1). Three cells
from loud to silent-wrong. The class is pre-existing (`localparam R = ~4'b1010;` is
4294967285/32 at HEAD — §2 row 14's declared-vs-inferred wall) and this slice does not
touch it; instead `param_init_kept_loud(p)` — implicit type, no range, a
context-determined top over a reduction, self-determined width < 32 or unknown —
declines at the four untyped value sites (instance · header · generate · package) and
in the width tail, so those cells stay exactly as loud as they were. ⚠️ The "contains a
reduction" conjunct is a delta-limiter, not a semantic claim; `~(!4'b0)` sits in the
same class unguarded because it already folds today. The typed twins fold (`localparam
logic [1:0] R = ~(|4'b1010)` is 2 on both oracles) and a top whose self width is already
32 folds untyped (`(|x) * 3` is `3 32`, verilator; iverilog says 33 for its own reasons).

**Review**: differential lens (12 designs outside the tables: generate-for/case,
`repeat`, indexed part-select width, `$clog2`, size cast, typedef / function-return /
interface ranges, 64- and 65-bit and signed operands, nested and concatenated operands,
`~`/`<<`/`-` over a reduction in a BOUND — all correct) PASS; soundness lens (13-site
census of every untyped-parameter fold, every one guarded; `defparam`, `-G`, `#()`,
interface and class scopes measured) PASS with **F1**: the new loud sentence told the
author of `parameter [0:3] P` to *declare a range* — `narrow_param_bits` refuses an
ascending or non-zero-LSB declaration because the bit domain indexes positionally, so
the text now names that case. **Residue → §2 🆕 H**: x-plane partial-known reductions
(`&4'b110x` is a known 0) still decline (8 cells, 2-oracle) · an ascending / non-zero-LSB
operand is loud where PRE was a silent 1 (oracles 4) · a value-inferred operand is loud,
not correct · the kept-loud untyped shapes above · a constant-function BODY local
assigned then reduced is loud in PRE and POST.

Tests: `reduction_in_declaration_bound.rs` (8, all values 2-oracle unless the docstring
says otherwise) + `wide_const_domain::an_inferred_width_never_supplies_a_reduction` now
covers the untyped and the bound spelling its name claims. 6,457 → 6,465.

#### 4.5.406 — A fill in a shift COUNT is one bit, and the row-30 fill lane is built, measured and reverted onto a prerequisite (§2 row 30) (2026-09-03 · format 29 unchanged · adversarial review: 2 lenses × 3 rounds · corpus 10/10)

**SHIPPED — the part that separates.** §11.4.10 makes a shift amount self-determined and
§5.7.1 gives an unsized fill in a self-determined position a width of ONE, so `'1` is a
shift by 1. `fold_shift_count` had no fill arm: it asked `const_eval_u32`, which sizes a
fill at a hard 32, got `0xFFFFFFFF`, and its saturation turned every such shift into zero.
`localparam logic [39:0] R = 40'hFF << '1;` was `0000000000` at exit 0 where iverilog and
verilator both give `00000001fe`. **880-cell census** (5 left operands × 4 shift operators
× {`'0`,`'1`} × 11 widths × {unsigned, signed}): **288 FIXED · 0 regressed · 0 oracle
splits**. ⭐ It separates because it is a value→value correction — the fill sits in the
COUNT, the left operand folds exactly as it always did, and nothing that was loud becomes
a value. `'0` is a shift by zero at any width and was accidentally right already; an x/z
amount still declines. **Residue, pinned in `param_fill_shift_count.rs`**: 104 cells where
the width-UNLIMITED i64 walk folds a right shift by `0xFFFFFFFF` to 0 and answers first —
the same hard 32 in the OTHER constant lane, now ROADMAP §2 row 🆕 C.

**BUILT, MEASURED, REVERTED — the row-30 fill lane itself.** An unsized fill in a
parameter initializer is context-determined and vita's constant domain gave it 32:
`localparam logic [127:0] C = '1 ^ 1'b0;` printed `…00000000ffffffff` where both oracles —
and vita's OWN runtime, continuous assign and port connection for the same text — give 128
ones. 165 of a 264-cell census. The four-piece patch works: **778 FIXED / 0 new-silent /
0 new-loud** over the review lenses' 1,622-cell sweep, all four binder copies, x/z loud
baseline byte-identical, corpus 10/10, staged `vcmp → velab → vrun` byte-identical.

⚠️⚠️ **It was reverted because each review round's fix produced the next BLOCKING defect,
and the third round's landed on a different axis in code this slice only ROUTES into** —
the repo's own stop rule. The three, in order, all found independently by both lenses:
① freezing the fill at `ctx` at the LEAF is wrong (§11.6.1 evaluates at `max(ctx, EVERY
self-determined operand's width)`, so a wider sibling makes the operator ZERO-extend the
frozen fill — 150 cells, 36 of them **loud→silent**); ② the guard that fixed ① reused the
ROUTING predicate, which reports a fill in a self-determined position too, so it declined
folds PRE performed correctly — 104 **NEW-LOUD**, and the *"declining is always safe,
every caller is one link of an `or_else` chain"* argument was measurably false because
`param_bits_at_declared` sits at the LAST link of every binder chain; ③ with the left
operand folding, `'1 << '1` exposed the shift count above — and then the soundness lens
found the wall: of the 504 PRE-loud→value cells the slice opens, **215 are silently wrong
on the SIGN axis**. `localparam logic [7:0] B = ($signed(4'hF) + 1) | 8'h00;` is `00`
against both oracles' `10` **with no fill anywhere, in PRE and POST alike** — `fold_bits_at`
decides an expression's sign NODE-LOCALLY (`sg = ls && rs`) where §11.8.1 makes the whole
region unsigned if ANY operand is. The slice does not create that defect; it removes the
loud standing over it, which is the one move the accuracy ladder forbids.
⭐⭐ **Prerequisite = §11.8.1 region sign in the wide fold** — row 14's declared-sign
provenance wall seen from the constant side, and the same root as the new row 🆕 F.

**What the rounds cost and bought**: ~2.6M subagent tokens over 3 census agents, 1 critic
and 6 lens agents; 3 BLOCKING defects, every one of them in MY OWN fix rather than in the
code under review; **five new pre-existing rows** (🆕 C class · 🆕 D `CastTarget::Named` ·
🆕 E a self-determined fill declines the whole tree · 🆕 F signed operand zero-extended ·
🆕 G a false `E3010` on a wide-param hierarchical read); one ORACLE-SPLIT recorded (a fill
in a parameter OVERRIDE is the formal's declared width in iverilog and 1 bit in verilator —
and the mirror image is REVERSED on port connections, so neither tool is an oracle there);
and **six of my own doc claims measured false and rewritten** before the revert.
⚠️ One of them was a Θ(n²): the routing predicate opened by delegating to
`expr_contains_fill` and then recursing, re-walking the whole subtree at every node —
a **fill-free** 8,000-term initializer went 0.31 s → 1.26 s. A predicate that promises to
cost nothing has to be measured saying so.

#### 4.5.385 — Round 34: two external reports, and the biggest finding was in neither of them (2026-08-26 · format 29 unchanged)

**Input**: two forwarded reports on the same day — hash_top round-34 (10 items) and an
aes_top audit (7 items) — filed against `9471a8a`. Every item was reproduced and
re-triaged at HEAD before anything was built, in a 10-agent census.

⭐⭐ **The census's largest finding is in neither report.** `R5` was filed as a LOUD gap
(*"`-G/--param` cannot carry a value wider than 64 bits"*). Widening the census over the
DECLARATION rather than the override found **19 silent-wrong cells** underneath it: on
`parameter logic [127:0] K = 128'hAAAA…`, a NARROW override — `#(.K(5))`,
`#(.K(32'hDEADBEEF))`, `#(.K(-1))` — was DISCARDED and the declared default used, at
`errors=0`. Every channel (`#()` named and positional, `defparam`, `-G`, generate scope,
forwarding through an intermediate module), every storage class. The flip is at declared
width **65**, not at the override's width.

The root is one call folding the wrong expression: `wide_disagreeing_value(&p.value, …)`
asks *"do the two domains agree?"* about `p.value`, which is the DECLARED DEFAULT's
expression. With an override in flight the two are different expressions, so they
disagree by construction and the arm installed the default and returned.

⚠️ **Extending an override past the i64 lane needs the EXPRESSION's signedness, and that
is not derivable from the i64.** `64'hFFFF_FFFF_FFFF_FFFF + 64'd0`, `-(64'sd1)` and
`32'd0 - 32'd1` all fold to the same i64; the oracles extend the first with zeros and the
second with ones. `ResolvedOverride` gains a `signed` channel recorded at each producer;
`None` (a `defparam`, whose collector folds before the record exists) declines rather than
guesses. The value goes into `wide_param_bits` **only when it must** — when there are bits
the i64 lane cannot carry — which also repairs a pre-existing correct→loud regression
(`logic [K-1:0]` broke the moment any wide-literal override appeared).

⚠️ And `bits` became a FOURTH applying channel in both `keeps_default_of` and
`has_applied_override`. They are one rule and had drifted, which is why a wide literal
warned *"not a constant; default kept"* and then errored, about an override that is a
constant and is now applied.

**R1 — the count/index channel truncated to 32 bits, silently.** `const_eval_u32` returned
`BitPacked.val[0] as u32`, and every consumer of that helper is a COUNT or an INDEX, so
the discarded bits were the ones that decide the answer:

| expression | vita const lane | vita runtime | both oracles |
|---|---|---|---|
| `64'hDEADBEEF12345678 >> 64'h1_0000_0000` | the operand UNSHIFTED | 0 | 0 |
| `A[2**32]` | `A[0]` | x | x / refuse |
| `$bits({(2**32+2){8'hA5}})` | 16 (a TWO-element replication) | — | warn / refuse |

⚠️ **The report put the boundary at 2⁶⁴** because it wrote 128-bit literals. It is **2³²**:
`>> 64'h1_0000_0000` is a 64-bit literal the i64 domain carries fine.

The two halves land on different rungs deliberately. A SHIFT is now CORRECT — §11.4.10
vacates with zeros (with the sign bit for `>>>`), so every amount at or above the width
gives the same answer and `fold_shift_count` saturates; both oracles agree and vita's own
runtime lane already did. A COUNT or a SELECT INDEX goes LOUD, because there is no
consensus to adopt: iverilog truncates a replication count with a warning and verilator
refuses it; for an out-of-range select iverilog gives `x` and verilator, a 2-state tool,
gives 0.

⭐ Side effect: a NAMED wide shift amount (`A >> SH`) folds now where it was E3009,
because the amount takes the same route its literal twin does.

**R2 — the diagnostics named the wrong thing.** `A[128]` over a 128-bit `A` said *"`A` is
wider than 64 bits"*, which is equally true of `A[127]`, and that folds. `B[9]` over an
8-bit `B` said *"the select `B[…]` has no constant-fold arm"*, and the arm exists.
`unfoldable_reason` asked only the i64 domain, so every operand of a wide expression
looked like a cause. It now steps over a sub-expression the WIDE domain answers, and an
out-of-range select or count is named for that before any operand is blamed.

⚠️ The VALUE half (§11.5.1 makes an out-of-range select `x`) is blocked one level down and
not on the select: the ≤64-bit parameter store is an `i64` with no unknown plane, which is
why `localparam logic X = 'x;` is loud too, with no select anywhere ⇒ §2 row 15, the same
prerequisite as the existing `'x`-valued-parameter row.

**R3 — `/`, `%`, `**` and `$clog2` had no wide arm** while the runtime lane computed all
four correctly. ⭐ The limb math is MOVED, not re-spelled: `mw_divmod`/`mw_pow` were pure
`&[u64]` functions in sim-engine and now live in `sim-ir::mw`, called by both. That
answers the decline comment already in `const_wide.rs` — *"implementing one here would be
a second spelling of the engine's arithmetic, and a subtly wrong one produces a silent
wrong PARAMETER"* — by removing the second spelling. `<<<` rides the logical-shift arm
(at a fixed width an arithmetic LEFT shift has `<<`'s zero fill), so `A <<< 4` stops being
E3009 where `A << 4` folds. `**` folds its base at the context width and its exponent at
its own (§11.6.1), which `128'd2 ** 100` is the cell for. `x / 0` stays loud. Verified
against live iverilog AND an independent Python golden.

**R30-2 — the report's axis was refuted, and so was §4.5.382's.** See ROADMAP §5. One
line: the only family where `native` loses to `vm` is a **SIGNED CONSTANT operand in the
RHS** (118.1 ns/eval vs 73.2), because `wprog`'s entry gate declined a `Const` leaf on
sign alone and took the whole assign off the compiled path. Relaxed for `Const` only ⇒
**−57%**. ⚠️⚠️ Dropping the sign half ENTIRELY was already built, measured sound, measured
**1.00×** on picorv32 and keccak, and reverted on 2026-08-20 — that measurement was right
and its conclusion was scoped to two designs without this shape.

**R4 — the CHANGELOG's discriminator table named 4 of the 12 shapes that frame a
function**, got the qualified-call row wrong (the refusal is per CALL SITE, so 1/5 not
2/5), and filed *"widen the inliner to control-flow bodies"* as the improvement.
⭐⭐ Measured, that is the WRONG DIRECTION: the inline fold is kⁿ in (references per
statement)^(chained statements) where the frame path is linear — the same design is
0.19 s inlined / 0.24 s framed at one reference and **14.39 s / 0.35 s** at three,
digest-identical. `able` is a coverage number, not a speed proxy. ⇒ §2 row 20.

**Also shipped, one commit each**: `R6` (a `string` output/inout formal in a static task —
narrowed at the ACTUAL rather than deleted, and `ref`/`const ref` stop being spelled
"inout"); `V34-4` (the §7.12 array manipulation methods reach fixed-size unpacked arrays,
with `item.index` as the DECLARED index — the hazard the census flagged); `V34-5` (`%p`
on an aggregate, matching verilator byte for byte); `V34-1` (the streaming operator on the
RHS, matching verilator on every cell; the LHS unpack stays loud with the working rewrite
in the message); `V34-2` (a body-scope `bind`); `V34-6` (an interface instance passed as a
port stops drawing a fake implicit-net warning); `V33-1` (all ten generate rejections
carry a real span and name the offending sub-expression); `V33-3` (an enum method on a
label stops being described as a chained method call — it stays LOUD, because iverilog
aborts on it and verilator reports no such function).

⚠️ **Three pins encoded limitations this round removed** and were rewritten as VALUE
assertions: the wide-literal-override wording, and two streaming-refusal wordings. One of
those also had to change its input — `8'hA5` is `1010_0101`, whose bit reversal is itself,
so the cell would have certified itself whatever the code did.

⚠️ **Two defects were introduced and caught inside the round**: reading the wide fold back
into the i64 lane (`#(.K({64'h0, 64'h5}))` folds to a 128-bit constant of value 5, and the
i64 walk declines a concatenation, so it fell through to the declared default — loud
traded for silent); and `wide_self_folds` treating an x-carrying fold as an answer, which
made `128'hx / 128'd3` blame the `/`.

⚠️ **One pinned value in the new sign battery was a prediction, not a measurement**:
`-100 >>> -7` is 0, not −1 — a negative SIGNED right operand makes the whole expression
unsigned (§11.8.1) so the fill is zero. All three tools said 0.

**The last three items, each its own commit.**

`V33-8` — the runtime warnings named the array and the file but never the line. ⭐ The
first job was to find out whether a span is even AVAILABLE there, and the honest answer is
NO at the emitting primitive: `warn_run_index` is called from `NetReader::read_net`, a
`&self` hot primitive whose whole signature is `(net, word)`. But a span IS available one
level up, and the mechanism already existed — the `severity_locs` sidecar (StmtId →
SourceLoc, resolved at elaborate, riding the `.velab` trailer) is exactly the *"statement-id
→ SourceLoc sidecar outside the golden IR"* the queue line asked someone to build. It is
renamed `stmt_locs`, its key set widened to the statements a runtime diagnostic can point
at, and the engine publishes `cur_stmt` from all THREE process-body executors. postcard
encodes struct fields positionally, so the rename and the widened key set are wire-neutral:
**no `format_version` bump**, and old and new artifacts still interoperate.
⚠️ **A side effect had to be fixed to ship it**: `SourceMap::resolve` scanned from byte 0,
which was fine for a handful of spans and made `velab` 0.01 s → 0.10 s once every
array-indexing statement resolved one. Memoised per-file line starts, fuzzed against the
linear walk. ⚠️ **And one residue was BUILT and then removed**: publishing the callee's
StmtId from the frame executor made `--backend interp` anchor at the subscript and
`--backend native` anchor nowhere — one design, one flag apart, two answers. Backend
agreement won and the weaker anchor is pinned.

`V34-3` — keyed assignment patterns (§10.9.1/§10.9.2). The parser hook is the shared
`lvalue = rhs` funnel, so `'{name: value}` arrived at every assignment form at once, and
`'{default: v}` covers ascending, descending, offset and multi-dimensional bounds. ⚠️ The
oracle is verilator: iverilog 13 does not parse a keyed pattern at all. New AST variant
(a SIBLING of `AssignPattern`, not a widening of it — 46 match sites across 19 files), so
the hdl-ast schema hash re-pins and every `.vu` is stale; `format_version` stays 29.

`R7` (the tractable half) — `--obs-procs` / `--obs-procs-time` put a `processes` object in
`run.json`: one row per process AND per continuous assign, with a scope the reader can act
on. ⭐ The identity is a new elaborate sidecar rather than a field on the frozen
`sim_ir::Process`, so a module instantiated 40 times gives 40 distinguishable rows with no
`format_version` bump. Synthesized bodies are labelled apart (`var_init`/`sva`/`covergroup`
/`clocking`/`port`/`net_init`) because reporting `always` at a line whose first keyword is
`covergroup` is the exact misdirection the feature exists to prevent. ⚠️ **The headline ask
is NOT covered and the queue line now says so**: the reporter needs ~440 cycle/s and
measures 20.4, which is a 21× scheduler question tracked in §5.1.

#### 4.5.384 — A label's width was a declared fact that no map recorded (2026-08-25 · 6,020 green · format 29 unchanged)

**Input**: §2 row 9, which the previous slice's census had produced: `typedef enum logic [31:0] { EA = 32'hAB34 } e_t;` then `logic [EA[7:0]-1:0] v;` declared a **one-bit** net at exit 0 where both oracles declare 52 — at MODULE scope as much as in a package.

⭐ **Same shape as §4.5.383, one declaration site over.** The value was never in doubt: the RUNTIME read of the same `EA[7:0]` was already 52 in all three tools. What was missing is that `param_range` — the map answering *"is this constant's width a DECLARED fact?"* — is written at parameter declaration sites, and a label is not one. But a label's width **is** declared: it comes from the enum's base type. `enum_base_range` reports it and the three sites that bind labels (module body, package, function/task body) record it beside the `param_meta` they already wrote.

⭐⭐ **The single binder built one slice earlier paid for itself immediately.** An inner `localparam [39:8] EA` shadowing a module label answers 34 in POST as it did in PRE, with no new rule: `bind_param_value` clears the range, `bind_param_range` sets it, so a rebind cannot inherit. The shadow case was a BLOCKING regression in §4.5.383 precisely because that funnel did not exist yet.

⚠️⚠️ **The oracles split, and the axis is untouched.** With `enum logic [39:8] { EA = 32'hAB34 }`, `EA[15:8]` is **171** in iverilog — which reads the label as a plain value of the base's WIDTH, indexed from 0 — and **52** in verilator, which honours the declared LSB as both tools do for a NET of that type. An ascending base is worse: iverilog rejects the declaration outright. `enum_base_range` declines both, so those cells stay exactly where they were rather than installing one tool's reading as truth.

**Measurement**: 59 cells over base kind {`logic [31:0]`, `logic [3:0]`, `int`, `byte`, base-less, signed, `[39:8]`, `[0:31]`} × scope {module, `pk::`, wildcard import} × consumer {net width, unpacked dim, replication count, localparam, generate cond, runtime read} × form {part, bit, `+:`, `-:`, `[i:i]`, nested} plus hazards — **FIXED 39 · REGRESSION 0 · MOVED 0**. Lens A adds 45 further cells: **REGRESSION 0**, 19 more silent-wrong → correct, including a `program` block, a `-G`-overridden base, a genvar loop bound, a `$dumpvars` width (the VCD `$var` follows), an instance override feeding a port width, and an enum whose base range is *another* enum's label select.

⚠️⚠️ **BOTH LENSES REFUTED THE SAME PREMISE, from opposite ends, and it was mine.** The first draft argued the second §4.5.373 prerequisite — *the value must be canonical at the recorded width* — was already supplied by §6.19, since `enum_label_range.rs` makes an out-of-range label a loud `E2002`. That gate lives in the PARSER and is fail-open twice: it needs a bare-literal bound, and it abandons every later label in an enum after the first it cannot fold itself. Lens B built `typedef enum logic [P-1:0] { A = 300 }` from the code; Lens A built the `-G W=8` twin from the oracles and found that **iverilog and verilator both REJECT** designs vita accepts at exit 0 — where the bare name then reads the raw `0xAB34` and a select of it reads the truncated 8-bit view, two answers to one label in one run.

⭐ **What actually makes the slice sound is the consumer, not the gate**: `select_base_at_declared` masks and `narrow_param_bits` resizes 64 → w, so no consumer acts on bits above the recorded width. The docstring now says that, names both fail-open holes, and warns that the one operation which would AMPLIFY instead of mask is a width above 64 — which is why `enum_base_range` caps at `1 ..= 64` rather than leaving it to the callers. The old draft also credited a negative-bound guard that could never fire (`lo != 0` subsumes it); that is gone.

⚠️ The premise is false in a second way Lens A measured: `enum logic [7:0] { EA = -8'sd2 }` — an unsigned base, exactly the shape this function records — stores `-2` where both oracles read 254. Width provenance right, sign provenance wrong. Pre-existing, PRE == POST, nothing acts on it yet ⇒ **ROADMAP §2 row 13**.

**Recorded, not fixed** (all pre-existing, all PRE == POST): the §6.19 fail-open (§2 row 12) · the unsigned-base negative label (row 13) · a mixed-sign zero-extension in the wide fold, which is the constant domain's sign rule and not enum-specific — `pk::EA ^ 64'h0` gives `fffffffffffffffe` against both oracles' `00000000000000fe`, and the plain package-parameter twin is wrong the same way (row 14) · and a module-scope label being invisible to a body `localparam` at all, which is phase order (3b binds params, 3c binds labels) and has nothing to do with selects — the package spelling of the same text folds (row 11, where one spelling also blames a missing replication fold-arm that exists).

⭐ **The census's own trap, twice.** `EA[5]` is 1 and a clamped bound is also 1, so the bit-select and equal-endpoint cells certified themselves until the result was scaled by 52. Same family as §4.5.365's *"a probe whose value fits the width cannot see a width defect"* — here it was the probe's ANSWER that collided with its failure mode.

#### 4.5.383 — The table that says "this width is a declared fact" had no package twin (2026-08-25 · 6,006 green · format 29 unchanged)

**Input**: the §2 queue's next item, ⓓ *"package-scope parameter select"* — the residue §4.5.363 left behind, with its prerequisite already written down: *"the package const table has to carry declared width provenance."*

⭐ **The prerequisite was right, and it was one map.** `param_range` is the map that answers *"is this parameter's width a DECLARED fact?"*; §4.5.382 had just used it to settle a whole axis. The package scope had no twin of it. `pkg_const_meta` records widths, but it mixes declared ones with widths INFERRED from a folded value, so no consumer could ask it the only question that matters before extracting bits — and §4.5.382 had written that refusal down explicitly, as a deliberate decline in `wide_name_bits`.

⭐⭐ **The census found five lanes behind the one queue line.**

| lane | module scope | `pk::W` · bare-imported |
|---|---|---|
| constant width / bound (net, unpacked dim, replication count, localparam, generate cond, submodule param, port list) | correct | **silent 1** — and `$size` gave `x` |
| runtime read of a zero-LSB declaration | correct | correct |
| runtime read of `[39:8]` | correct | **silent 171** — the raw internal bits |
| runtime read of `[0:31]` | correct | **loud** |
| intra-package sibling (`parameter Q = W[7:0];`) | — | **loud** |

⚠️ **The queue line said the runtime lane was already right in all three tools.** It was measured on a ZERO-LSB declaration, where the normalization is a no-op. `parameter [39:8] B = 32'hAB34;` makes `pk::B[15:8]` print **171** where both oracles print 52, at exit 0 — a silent-wrong the queue did not know about, found by widening the census rather than by re-reading the line.

⭐ **And all three SPELLINGS behaved identically** — `pk::W[m:l]`, the bare name after `import pk::*`, the bare name after `import pk::W`. One gap, not three: the two bare ones arrive as an `Ident` whose key IS in `params` (the import binds it) but never in `param_range`, because the import bound the value and the meta and not the range.

**What shipped**: `pkg_const_range`, filled by the SAME `param_decl_range_opt` the module twin reads back — so the provenance rule stays one rule and a value-inferred package width still declines. It is bound alongside the value on `import` (wildcard and explicit), made live during the package's own fold so an intra-package sibling resolves and then restored, and read by `param_sel_range` (which drives BOTH the constant fold and the runtime offset normalization) and by `const_select_base`'s new package arm.

⚠️ **`bind_param_range` is set-or-CLEAR, and the clear is the point.** Imports made a rebind reachable that never was: a wildcard `import pk::*` binds `pk`'s `parameter [31:0] W` at `top.W`, and a module-body `localparam W = ~8'hCB;` rebinds the VALUE two phases later (3a.5 imports, 3b body params). Leaving the package's range behind would extract bits of a 32-bit declaration out of an 8-bit value — the §4.5.363 "263-bit net" regression, arriving through the import door.

⭐ **Side finding on the same axis, in the parser.** `W-PARSE-SELECT-BASE` fired on `pk::W[7:0]`, saying the select "applies to an expression, not to a net or variable". A package-scoped reference IS a name (§26.3) and both oracles accept the select; `from_name` simply did not list `PkgScoped`. So the same select drew a portability warning or nothing at all depending on which of its three spellings you wrote.

**Measurement**: a 3-oracle census over spelling × consumer × declared range × select form × hazard — the transition matrix is at the bottom, after the review. Corpus **8/10, every pinned digest matched**; examples 4/4 stdout + VCD byte-identical.

**Residue recorded, not fixed**: a select of an ENUM LABEL in a constant context is silent-1 at MODULE scope too (`typedef enum logic [31:0] { EA = 32'hAB34 }`; `EA[7:0]` in a width) — a different declaration site, not this axis. A package STRING parameter's select stays loud (the string constant domain's gap). A package const ARRAY element in a width stays loud, as at module scope.

## What the adversarial round cost, and what it bought

⚠️⚠️ **Two BLOCKING findings were regressions this slice created, and the soundness lens
measured both.** `param_range` is keyed exactly like `params`, and until this slice **no
binder could rebind a key another binder had ranged** — the FQ key made every scope
disjoint, which is why maintaining the side map at 3 of ~13 binders had been safe. A
wildcard import breaks that property: it binds a PACKAGE declaration at the MODULE's own
key, and a local enum label (phase 3c), a genvar, a real parameter's integer twin or a
body parameter rebinds that same key one phase later.

| | PRE | this slice, round 1 | all three tools |
|---|---|---|---|
| `import pk::*` over `parameter [39:8] W` + local `enum { W = 32'hDEADBEEF }`, `W[15:8]` | 190 | **239** | 190 |
| the same with `genvar W` bound to 8, `W[11:8]` | 0 | **8** | 0 |

⭐⭐ **The fix is that there is only one binder.** `bind_param_value` clears the range and
`bind_param_range` (with a `debug_assert` on the order) sets it, so `grep` for a raw
`self.params.insert` / `.remove` outside those two functions returns nothing. The author
had added the clear at three sites and believed it complete; the lens censused thirteen.
Patching the writers you can think of is the wrong shape — *"a writer that forgot"* has to
be unrepresentable.

⭐ **Both lenses independently found the same remaining hole**, which makes it high
confidence: a >64-bit package parameter's declared range reaches **one of four**
consumer × spelling quadrants. `pk::K[31:24]` normalizes, the bare-imported `K[31:24]`
does not, and neither spelling sizes a net correctly. ⚠️ **PRE was wrong on all eight
cells too**, so no ladder move — and the two halves need different machinery (the bare
spelling needs the range on a key whose value lives in `wide_param_bits`, a second map
with its own ~10 binders; the width consumer needs the WIDE BIT DOMAIN to carry a declared
direction at all, which is a capability). ⇒ **ROADMAP §2 row 10**, with both prerequisites
written out — and the two docstrings that overclaimed the reach are corrected in place,
because a false claim in a comment is how this repo's last two slices got their premises
wrong.

⭐ Round 2 also closed the last asymmetry the census had left: a NESTED select
(`pk::W[15:0][7:0]`) folded through the bare-imported spelling and declared **one bit**
through the `pkg::` one, because only the wide bit domain folds a select of a select and
its name resolver declined every narrow PACKAGE constant. That decline was written one
slice earlier with a reason — *"its declared width lives in `pkg_const_meta`, which is not
provenance-filtered"* — that `pkg_const_range` had just made obsolete.

⚠️ And a parenthesis had started to change the answer: `pk::B[15:8]` normalized to `34`
while `(pk::B)[15:8]` still read the raw `ab`, in one file. Both oracles reject the
parenthesised spelling, so there is no oracle here — the target is vita's own answer to
the same select two characters away.

**Final measurement**: 132 cells, PRE vs POST, three tools — **FIXED 61 · REGRESSION 0 ·
MOVED 0**. Lens A adds a 64-cell non-package sweep at **PRE == POST, 0 diffs** (the direct
probe of the new set-or-CLEAR) and four instances of one module with a ranged parameter,
two of them overridden, unchanged.

#### 4.5.382 — Two external reports, one axis: the constant domain was computing in i64 while the declaration stated a width (2026-08-25 · 5,987 green · format 29 unchanged · corpus **7/10 → 8/10**)

**Input**: round-33 (`hash_top` v0.15, 11 items) and an AES IP audit (14 items), delivered together with the instruction to finish them in one operation rather than file them.

⭐⭐ **The census merged them.** Eleven of the twenty-five items were one axis seen from eleven sites: every crypto / AXI constant idiom that mixes a NAME with a wide value was rejected, and **which operator you wrote decided whether the parameter existed**. `A ^ 128'h1` folded; `A ^ B` — the same value, named — did not. `+`, `-`, `*` and the comparisons had no wide arm at all. A select of a >64-bit parameter had none. A reduction (`^A`) or `$countones` had none at ANY width. A package parameter could not be wider than 64 bits *in a package* while the identical declaration in a module, a `#()` header, a one-element array and a packed struct all worked.

⭐ **One resolver was the root of most of it.** `fold_self_bits`'s NAME hook consulted `wide_param_bits` alone — a table that holds nothing under 65 bits — so every narrow name declined and a wide operand beside it had nothing to combine with.

⚠️⚠️ **The width source is the entire soundness argument, and §4.5.373 is why.** That slice built the reduction operators on `param_meta`, where widths INFERRED from a folded value sit beside declared ones, and measured `localparam W = 4'hF | 4'h0;` reducing 32 bits where both oracles hold 4 — picking the opposite generate branch at exit 0. It reverted and recorded the prerequisite as *"declared provenance, and under it, the value canonical at that width"*. Both halves are answered by `param_range`, which `param_decl_range` populates only from a declared range / type / sized literal, and beside which the stored value is coerced at binding. **Measured on that exact counterexample**: with the range declared, `localparam logic [3:0] W = A<<4;` is 0 in vita and 0 in iverilog.

**Shipped, by cause:**

- **Narrow named parameter as a wide operand** (`narrow_param_bits`) — declines a non-zero LSB and an ascending declaration, because the bit domain indexes positionally and carries no direction (§4.5.363's rule at this domain's boundary).
- **Wide arithmetic / comparison / reduction / select / ternary / `>>>` / `$countones`·`$onehot`·`$onehot0`·`$signed`·`$unsigned`.** The old banner said carry-free ONLY, "rather than grow a 128-bit adder inside the elaborator" — a statement about RISK, and the risk is payable: one limb kernel, the operand-extension rule (§11.6.1) written out once at the single place both operands reach a common width, and a DECLINE on any unknown bit. Division and modulus stay out.
- **Package `wide_param_bits`** — a fourth domain for the package scalar path, published through wildcard and explicit imports, `pkg::K`, and the package-function scope push.
- **`ResolvedOverride::bits`, a fifth override channel** carrying the override's WIDTH. §6.20.2 gives an untyped parameter the range of its FINAL override value and the i64 channel carried none, so `#(.M_ISSUE(M_ISSUE))` forwarding `{2{32'd4}}` arrived 35 bits wide (the value's minimal signed width) where every tool has 64 — and `M_ISSUE[32 +: 32]` then read past the end. It also makes `-G K=128'h…` apply instead of being refused.
- ⭐ **Replication COUNT over a NAME.** §4.5.371 built this, measured it and reverted over four blocking defects; **all four are answered by folding the count through the SAME name resolver the surrounding fold already uses** rather than through a second evaluator. ⓐ *"widen the width twin at the same instant"* — there is no twin: the width consumer and the value consumers call one function. ⓑ/ⓒ *"the resolver catches a const-function LOCAL / skips past a shadowing local to the module parameter"* — an `is_count` flag on the hook declines at the local rather than reading it or passing it (measured: `int n = 2; {n{4'hA}}` is E3009 here and *"a reference to a net or variable is not allowed in a constant expression"* in iverilog). ⓓ *"the evaluator restarts the call depth at 0 and overflows the stack"* — this walk calls no evaluator.
- **Package `real` in module-scope constants** — the predicate that CHOOSES the real domain (`expr_mentions_real`) had no `pkg::` arm, so a value that crossed the boundary fine everywhere else was invisible exactly where a constant needed it.
- **String methods in a constant context** (`localparam int W = S.len();`), and a **method on a `pkg::`-qualified value** (`pk::S.len()`).
- **`generate`/`endgenerate` optional (§27.3)** and a **`genvar` declared in the `for` header (§27.4)**.
- **A package's STATIC `function` sees its own sibling** under a selective import (IEEE §26.3) — only the frame path pushed `cur_rtn_pkg`. Pushed around the BODY only: a caller's own same-named function must still win in an ACTUAL argument, and that is pinned.

**Diagnostics — six report items, one defect**: the message named the DECLARED thing and not the REJECTED one. The caret came from `cur_span`, which during module elaboration is the MODULE HEADER, so N rejections in one module printed one `file:line:col` at a line with no parameter on it; the text stopped at *"is not a foldable constant expression"*, so a package `real`, a string method and a replication count were indistinguishable and none of `real` / `string` / `replication` appeared in any of them; and when the cause was an UNDEFINED NAME the name was in hand and thrown away. Now `error_at` + `unfoldable_reason`. Also: a missing package file is **one** error naming the package instead of **seven** (the first at the `::`, the rest at correct declarations that only failed because the tf-port list never closed); an enum-method rejection describes enum methods instead of hierarchical function calls, with a location; a non-constant bound prints once, at the bound; an unconnected child **INPUT** warns (the asymmetry ran opposite to the consequence — a dangling output discards a value, a dangling input MANUFACTURES one); and a variable driven by both a declaration initializer and `always_comb` warns (vita ran it at exit 0 while xcelium stops elaboration and verilator errors).

⚠️⚠️ **Three defects I created, all caught by measurement before the suite saw two of them:**
- ⓐ **A size cast is a CONTEXT for its operand**, not a truncation after it (§11.6.1). Folding at the operand's own width made `65'(64'd18446744073709551615 + 64'd1) >> 64` answer 0 where both oracles answer 1 — the 65-bit sum carries into bit 64. Fail-closed: a context-determined top whose fold is narrower than the cast declines.
- ⓑ **A COUNT read a const-function LOCAL** — §4.5.371's blocking defect ⓶, reproduced exactly by routing counts through the ordinary resolver. `int n = 2; {n{4'hA}}` folded 170 where iverilog rejects the function.
- ⓒ **A local whose declared WIDTH does not fold was bound anyway.** The suite pinned "the loud must not be conditional on the width being known" — and that cell had been passing because the INITIALIZERS reaching such a declaration did not fold either. Once they did, one unknown-width local bound 34 and its neighbour bound 7, with nothing to truncate them. **Neither oracle has a value there** (iverilog aborts with an internal assertion, verilator fails to build), so the safe side of the ladder is loud, and both cells are pinned that way.

⚠️ **21 suite pins failed and every one was re-measured against both oracles before being touched.** Four were the new message text. Two were pins on a SILENT answer (`VAL=1/16` for a select bound, with a docstring reading *"iverilog says 9/16; if the bound is no longer 1, promote this row"*). Several used `{4'd2, (4'd1+4'd1)}` as a stand-in for "unfoldable" — it folds now (34, both oracles), so the proxy became `{4'd2, 4'bxx01}`, which a parameter value can never hold. One (`generate_scope_decl_init_is_loud`) had been passing **for the wrong reason entirely**: the desugar it claimed was missing had been opened by §4.5.228, and the cell survived only because the keyword-less `if (1) begin … end` did not parse.

⭐⭐ **serv PROMOTED — corpus 7/10 → 8/10.** Its digest is `f3f45af36093b2b1`, the value §4.5.373 measured against iverilog before reverting. **verilog-axi advanced through two blockers** (`S_THREADS` → `$clog2(M_ISSUE[n*32 +: 32]+1)` → `calcBaseAddrs(…)`, a constant FUNCTION with an accumulator wider than i64 = §4.5.379's *"expensive half"*); its pin was re-cut to the new refusal.

⚠️⚠️ **R30-2 (perf): the report's diagnosis is REFUTED by the controlled twin, and the fix is not landed.** It measured `native` losing 1.88× to `vm` on continuous assigns whose LHS is an unpacked-array ELEMENT and concluded "the axis is the LHS shape". Varying one thing at a time over 25 assigns and one digest: element LHS with `a[gy][gx]` is **0.92×** (native wins); with `a[gy][(gx+2*gy)%5]` it is **2.05×**; with the indices written as literals it is **1.03×**. The LHS is incidental — it is what makes the index exist. The axis is a **computed array index in the RHS**: `(gx+2*gy)%5` over genvars is a constant that survives generate unrolling as an unfolded `Binary` tree, so the element read is `LoadIdx` (index re-evaluated per access) instead of a slot `Load` — and that is Keccak's π step, `B[y][(2*x+3*y)%5]`, which is what the report measured. ⚠️ **Widening `wprog` to `*`/`/`/`%` was built and reverted**: byte-identical, and it moved the row by nothing (2.09× → 2.09×). The prerequisite is a constant-fold of the IR expression tree. The shape is now `CONT_ASSIGN_ELEM` in `perf_baseline.rs` with the three measurements written beside it.

**Not addressed, and why** (each recorded in ROADMAP): **V33-8** runtime-warning locations and **R14** per-process timing both need a span/instrumentation sidecar from elaborate to the engine — no runtime diagnostic in vita carries a `SourceLoc` today, so it is infrastructure, not a message fix. **R7(a)** widening the inline discriminator to control-flow bodies is ROADMAP §3 ⓑ; the CHANGELOG table the report asked to correct was already corrected in round 32. **R10** static-task `string` output formals — the report itself ranks it low and says the diagnostic is already good; measured unchanged and it is.

**Gates**: 5,987 tests green · clippy `-D warnings` clean · fmt clean · corpus **8/10** rc=0 · examples PRE == POST byte-identical · staged `vcmp→velab→vrun` == one-shot.

#### 4.5.381 — The conversion the manual told users to write was refused too (2026-08-25 · 5,963 green · format 29 unchanged)

**One line**: §0 T2's last open item — a `real` constant reaching an INTEGER context — closed at
the CONTEXT BOUNDARY the withdrawn §4.5.232 twin could not reach, and the census split the axis
cleanly into what both oracles answer and what they disagree about.

**The withdrawal named this slice as its own remedy.** §4.5.232 tried to close this by giving an
exactly-integral real parameter an i64 TWIN, and withdrew over five silent-wrongs. Its comment in
`strings.rs` is still the best statement of why: a twin lets the INTEGER domain succeed on an
expression that mentions a real, and `param_real_value` is the only site that applies §11.8.1's
*"any real operand ⇒ evaluate in the real domain"*, so every other const-evaluation context
answered from a truncated integer — `generate if (R/2 > 2)` with `R = 5.0` took the ELSE branch.
That comment ends: *"closing it means routing those sites through the real domain, which is its
own slice"*. This is that slice. Three of the five sites it lists were already closed
(§4.5.241/242); the remaining ones are what this one does, and it does them at the CONSUMER, so
no new i64 ever enters the shared integer walk.

**⭐ The census split the axis on explicit-vs-implicit, and both oracles drew the same line.**
16 shapes × 3 tools: vita was loud on all 16, the two oracles agreed on 12. The four splits are
not scattered — every one is an IMPLICIT conversion, and the tools break in OPPOSITE directions:

| shape | iverilog | verilator |
|---|---|---|
| `logic [R-1:0]` | 3 | **rejects** |
| `logic v [R]` | 3 | **rejects** |
| `{R{1'b1}}` | **rejects** | 7 |
| `R << 1` | **rejects** | 6 |

Where the source SAYS it converts (`int'(R)`, `$clog2(R)`, `$rtoi(R)`) or the destination has a
declared integral type, the two agree without exception. So the implemented rule is not "vita can
compute a number here" but "the language calls this node a conversion" — and the four splits stay
loud with that measurement written into the code.

**⭐ Root cause, one sentence.** Two gates — `count_reads_real_param` and
`count_lowers_real_param` — ask *"does this expression MENTION a real parameter?"* and both descend
straight THROUGH `Cast` and `SysCall`, which are precisely the two nodes that ARE the conversion.

**⭐ Rounding: the reference implementation was already in the tree.** §6.24.1 rounds half AWAY
from zero (`2.5`→3, `−2.5`→−3, `−0.5`→−1) and `$rtoi` truncates instead. iverilog, verilator and
**vita's own runtime** (`lower_real_to_int_cast`, whose tie-to-even bug §4.5.365 fixed) agree on
all twelve cells; only the constant domain never asked. `real_round_to_i64` uses Rust's
`f64::round`, which IS that rule — deliberately not the lowered form's `e + (e >= 0 ? 0.5 : −0.5)`,
which is tie-to-EVEN in [2^52, 2^53) and is what cost §4.5.365 a silent-wrong.

**Shipped**: `const_eval_cast`'s Prim arm falls back to the real domain; `const_clog2_selfdet`
converts a real argument; a `$rtoi` const arm; and `param_value_via_real` at all three declaration
sites (module body, generate, package — three spellings of one fold, so without all three the same
text answered at module scope and went loud one `generate` deeper). ~30 lines of code.

**⚠️⚠️ Two defects I created, both loud → silent-wrong, both caught by probing my own change**
(the suite was green through the first):

- **A shadow, from a "parity" edit.** Mirroring the `$rtoi` arm into the const-FUNCTION walk made
  `function int f(int N); f = $rtoi(N);` with `localparam real N = 3.9;` fold `f(9)` to **3** —
  the module parameter, read through the integer formal — where iverilog gives 9 and PRE was loud.
  `const_rtoi_via_real` resolves at module scope. Same shape as §4.5.371 ⓷. ⭐ The fix was to
  DELETE the arm: that walk's catch-all already delegates to the module-scope fold under a guard
  proving nothing local can be shadowed, so parity comes for free and **the shadow rule stays with
  the walk that owns `env`**. The same `env.is_empty() && envw.is_empty()` guard then went on the
  `$clog2` fallback, which had the identical latent hazard and escaped only because its env-aware
  resolver happens to answer first.
- **Precision, from a domain order.** `const_eval_real_in_scope` promotes a real-free subtree with
  `as f64`, so `$rtoi(64'd9007199254740993)` — exactly 2^53+1 — came back one LESS, silently,
  where PRE was loud. Fixed by asking the integer domain FIRST: the real domain can only ADD
  answers there, never correct one. (vita now returns it exactly; both oracles truncate `$rtoi` to
  a 32-bit container — iverilog −2, verilator 2147483647 — the recorded `$itor` divergence family,
  so it is pinned hand-IEEE.)

**⭐⭐ The suite pointed at six of its own pins, and every one encoded the removed limitation.**
Five were written by earlier slices to DEFEND the gap; one named §0 T2 as its prerequisite in its
own docstring. Two were pinning SILENT behaviour: `int_cast_of_real_param_bound_is_a_tracked_-
preexisting_gap` asserted `v[int'(3.5):0]` reads ONE bit at exit 0 (iverilog: `v[4:0]` = `0d`), and
`clog2_real_literal_arg_declines` asserted a **0-count replication** for `$clog2(8.0)` — defended in
its docstring as *"declines to 0, never a wrong non-zero"* — where both oracles print 3. A 0-width
replication is not the safe side of the ladder, it is the quiet side. All six re-measured against
iverilog (plus verilator on the two that stay loud) and rewritten as VALUE assertions.

**⚠️ And one pin was mis-titled, which matters more than the six.**
`a_non_integral_real_never_folds_into_the_integer_domain` guards a real invariant — never convert
at the LEAF — but two of its three cells were not testing it: `localparam int A = (R == 2)` and
`logic [$clog2(R)-1:0]` both HAVE a context boundary, so converting there is what the invariant
PRESCRIBES. Kept the one cell that tests it, and added a positive one (`R > 2` with `R = 2.4` must
still be TRUE, decided in the real domain rather than on a truncated 2).

**⭐ Two pre-existing silent-wrongs closed as a side effect**, both the *"a bound that does not fold
degrades quietly"* shape §4.5.380 hit one node over: `nonconst_bound_reason` had no `Cast` arm, so
`logic [int'(NOPE)-1:0] v;` declared a 1-bit net at exit 0 while the bare `[NOPE-1:0]` twin was
loud about that very name (the domain-twin detector again); and the `v[int'(R):0]` select above.

**⭐ A lane the census missed.** Every grid cell named a `parameter real`; the LITERAL spelling is
the same axis and the suite's own pin is what surfaced it — `int'(3.0)`, `$clog2(8.0)` and
`int'(2.5)` in bounds all work now and all match iverilog.

**⚠️ Demand is weaker evidence here than in the last four slices**: ZERO `parameter real` across
the eight third-party corpus designs. That is partly a corpus limitation — it is synthesizable RTL,
where real parameters are rare by nature; they live in testbenches, which the corpus does not
contain — so it is recorded rather than used to kill the item. The item was owner-directed §0.

**⚠️ Method note**: one sweep cell first read as a verilator REJECTION and was not — a 20 s
watchdog killed a slow binary start. A watchdog kill is not an oracle verdict; re-run before
scoring one.

**Measurements**: grid A 16 cells (10 fixed, 4 splits held loud, 2 out of scope) · scope-parity
sweep 12 cells (module/generate/package × 4 forms, all matching both oracles) · risk sweep 13 cells
· adversarial batch 12 cells (5 newly correct, 7 residues **every one PRE-loud → POST-loud**, zero
regressions) · the four shapes the withdrawn twin broke unchanged · examples 4/4 stdout+VCD
byte-identical · corpus unchanged · 5,943 → **5,963 green** · format 29 unchanged.

**Residues, all loud** (ROADMAP §0 T2 item 8): the four oracle splits · an untyped localparam with
a real value (§6.20.2 makes it a REAL parameter — rounding it is the withdrawn silent-wrong) ·
real override `#(.R(2.5))` · `1.0/0.0` · `localparam time T = R*2.0` (`param_decl_width_opt` sends
`time` down its untyped branch, and that function is shared by eight sites) · `$rtoi` inside a
const-function body (needs the env-aware walk, per the shadow above) · `int'(real'(R))`.


#### 4.5.375 — The file name was never allowed to be a value; the hierarchical memory was built, measured, and reverted (2026-08-24 · 5,912 green · format 29 unchanged)

**One line**: started on §3 ④, found the queue line half wrong, built the real gap, and reverted
it — but not before it uncovered a pre-existing silent-wrong underneath, which is what ships.

**The census refuted the queue.** §3 ④ claimed `dut.ram.mem[i] = …` and
`$readmemh(f, dut.ram.mem)` were both `E3009`. A 13-cell census across three tools found every
ELEMENT shape already working — element read and write, variable index, part-select,
bit-select, multi-dim, non-blocking, inside a task — 11 cells, no failure. The line had been
stale for about two months; **N3.1**, its multi-dim follow-on and **HIER-REST track 9** closed
it back in June. What is actually refused is the WHOLE array, in exactly three cells, all at
one gate whose message says a whole array "has no plain readable value" — true for
`x = dut.mem;` and the wrong question for `$readmemh`, which wants the array, not a value.

**Demand is four upstream testbenches**, all the same firmware-loading line: serv
(`servant_sim.v:20`), picorv32 (`testbench.v:253` and `testbench_wb.v:146`), ibex.

⭐ **The implementation was small, and verified.** `$readmemh` already calls
`expr_array_view`, which already resolves dotted paths; a cross-instance name simply has no
net yet, so it defers — and the deferred placeholder is ALREADY
`Signal { net: POISON_NET, word: None }`, the exact shape the local path builds by hand. A
consumer-scoped exemption (the call registers its own memory argument, so the guard keeps
refusing whole-array reads everywhere else) made **40+ shapes match both oracles**, including
the real serv SoC end to end.

**⚠️⚠️ Reverted anyway.** vita runs a PARENT's `initial` before its child's; both oracles run
the child's first. A RAM that loads its own memory therefore overwrites the testbench's
hierarchical load — `01 02 03 04` where both oracles give `aa bb cc dd`, at exit 0. Measured
on the real serv SoC too: `DIGEST=…543a` (iverilog) vs `…523a` (vita). Opening the construct
traded a loud reject for a **silent wrong answer on its own motivating idiom**.

⭐ Three facts made that a revert rather than a fix-in-place. The ordering is **pre-existing
and independent** — `u1.s = 8'hAA` already loses to the child's `initial`, with no `$readmem`
anywhere, through a write supported since June, so this is an already-open class and the
construct merely adds a door. `sim_ir::Process` is **frozen** and has no instance field, so
the order cannot live in the IR; it needs an out-of-band per-process rank sidecar. And `tie`
is shared with `Comb`/`Latch` seeding and with `compose_child_tie`, so the blast radius is
every design with a child `initial` — its own census, its own review. Recorded with the repro
and the fix shape in §3 ④, and as a new §2 row.

**⭐⭐ What ships is what the revert uncovered.** `$readmem*`/`$writemem*` accepted only a
string LITERAL and took a **bare `return`** on anything else: no file read, no file written,
no diagnostic, exit 0. IEEE 1800 §21.4 asks only for a string EXPRESSION, and the canonical
testbench keeps the name in a packed reg (`reg [1023:0] firmware_file`). A SystemVerilog
`string` variable was equally silent. Purely local designs were affected — no hierarchy
needed — so this is independent of the reverted half and stands on its own.

⚠️ The two sites were **byte-for-byte the same six lines**, and only the read side is the one
anybody probes, so they now share one `memfile_name` helper rather than being fixed one at a
time. An empty name warns instead of vanishing.

**Adversarial review.** Soundness: **no BLOCKING**, three NITs — the array-parameter check
was run for the whole family though the local twin restricts it to the tasks that WRITE the
memory (so `$writememh` on a hierarchical array parameter was refused where the local form
worked, under a message naming the wrong task), and `is_dotted_ident` did not peel
parentheses though the helper it complements does. Both fixed before the revert; both went
out with it. Differential: **1 BLOCKING** (the ordering, above) and 40+ clean shapes. ⚠️ The
differential lens also had to **retract four findings** measured against a binary this session
rebuilt underneath it — it re-measured against a frozen snapshot, which is the right method
and worth copying.

**Measured.** 13-cell census 3-oracle · differential 55 designs, **zero correct→wrong** ·
suite green · `$dumpvars` stdout and VCD **byte-identical** PRE vs POST · examples 4/4 and all
ten corpus workloads **PRE == POST** · `$writememh` output files byte-identical to iverilog's,
including the `// 0x00000000` header · 8 pinned tests, every value from a live iverilog run ·
format 29 unchanged.

#### 4.5.374 — 직접-rhs 전용 시스템함수를 식 어디에나: 참조 구현이 또 트리 안에 있었다 (2026-08-24 · 5,904 green · format 29 불변)

**한 줄**: 코퍼스가 지목한 §3 ③. `$fgetc`·`$value$plusargs` 계열은 **blocking 대입의 직접 rhs**
에서만 살았다 — 다른 모든 위치가 E3009 인데 두 오라클은 전부 받는다. 호출을 temp 로 끌어내는
**hoist 를 지어** 여덟 위치를 열었고, **darkriscv 전체 SoC(3,115줄·9모듈)가 처음으로 돌면서
다이제스트가 라이브 iverilog 와 일치**한다(`b4a7bb6d411fea85`).

**census.** 4 형태 × 11 위치 = 42칸. **PRE 41칸 loud**, 유일한 예외가 `if ($value$plusargs(…))`
— `lower_branch_cond` 가 **한 계열·한 위치**만 desugar 하고 있었고 `!` 하나만 붙여도 E3009 였다
(serv 테스트벤치의 철자가 정확히 그것). POST **31칸 correct · WRONG 0**, 남은 11은 설계상 거절.

⭐ **참조 구현이 이미 트리 안에 있었다** — `hoist/general.rs`(3,237줄)가 output-formal 호출에
대해 똑같은 변환을 하고, `Shape`/`shape()` 라는 *"어떤 자식이 · 무조건 · 어떤 순서로 평가되는가"*
의 **단일 정본**과, 짝이 되는 statement arm 들(`repeat` 은 count 를 한 번 읽지만 `while` 조건은
아니다 · `$monitor` 는 인자를 나중에 **다시 렌더**한다)을 갖고 있다. 새 모듈은 그 `shape()` 를
**그대로 소비**하고 arm 구성을 미러링한다.

**⚠️⚠️ 내가 측정으로 잡은 둘(리뷰 전).**
- **temp 의 부호.** `$fgetc` 는 EOF 에 **−1** 을 낸다. 기존 `fresh_ia_tmp` 은 unsigned `Reg` 라
  `$fgetc(fd) != -1` 이 `4294967295 != -1` = 영원히 참이 된다. 직접-rhs 형태는 사용자의
  `integer` 에 바로 대입해서 이걸 한 번도 안 드러냈다.
- **평가 순서.** rhs 가 bare call 일 때 lvalue 인덱스만 hoist 했더니 iverilog 가 **rhs 를 먼저**
  평가하는 `m[$fgetc(f)] = $fgetc(f)` 가 뒤집혔다(65↔66). 종료 가드를 *"whole rhs 가 bare call"*
  에서 *"bare call 이면서 문장에 hoist 할 게 더 없다"* 로 좁혀 해결 — 그 좁은 형태가 정확히
  이 모듈이 합성하는 문장이다.

**⚠️⚠️ 적대 2렌즈 · BLOCKING 여섯 · 전부 내 게이트의 구멍.**
- ⓐ **`write_arg_range` 가 뒤집혀 있었다.** `$fgets(str,fd)`·`$fread(mem,fd,…)` 의 목적지는
  **arg 0** 인데(IEEE §21.3.4, 그리고 vita 자신의 `sys_special.rs` 가 `args[0]` 에
  `deny_const_param_write` 를 건다) `$value$plusargs` 와 한 팔에 묶어 **fd** 를 write 로 셌다.
  fd 는 살아남는 식이 절대 들지 않으므로 겹침 게이트가 **전부 통과**시켰다.
- ⓑ **`Shape::NoHoist` 자식의 읽기가 게이트에 안 보이는데 노드는 살아남는다.**
  `shape_children` 이 그 노드에 `[]` 를 주는데 `rebuild` 는 `clone()` 한다 — `general.rs` 가
  `reads_are_opaque` 로 방어하는 바로 그 구멍이고 새 모듈엔 대응물이 없었다.
- ⓒ **별칭.** 루트를 `HierPath` 첫 세그먼트로만 봐서 자기-계층(`m.a` 는 v1 flatten 상 `a` 와
  **같은 net**)과 `p::v`(`PkgScoped` 는 `Ident` arm 에 안 걸린다)를 놓쳤다.
- ⓓ **미러링하면서 조건을 하나 빠뜨렸다.** `UserTaskCall` arm 의 `task_call_inout_root_written`.
  ⭐ 그런데 그것만 되살려도 안 됐다 — 그 술어는 inout **함수 호출**용이라 syscall 을 못 본다 ⇒
  `rhs_only_hoist_ok_seq_with(seq, extra)` 를 만들어 **input 이 아닌 actual 전부**를 살아남는
  읽기로 넣었다.
- ⓔ **temp 가 파형에 샜다.** 새 sigil `$rhs_only_tmp$` 를 만들었는데 VCD/FST 필터는 `$ia_tmp$`
  **하나만** 안다. 그리고 이 훅이 `lower_branch_cond` 를 가로채므로 **이미 동작하던**
  `if ($value$plusargs(…))` 의 파형까지 바뀌었다 ⇒ sigil 통일(kind 는 signed 유지).
- ⓕ ⭐⭐ **내가 이 슬라이스에서 쓴 docstring 의 전제가 거짓이었다.** *"they mutate only fd
  state, **which no expression in the statement can read** (`$feof` … is PURE and so never
  hoisted — it would be re-evaluated where it stands either way)"* 라고 적고 그 위에
  *"write 없음 ⇒ hazard 없음"* 조기탈출을 세웠다. `$feof` 는 **인자에 대해서만** pure 하고
  파일 위치를 읽으며, *"제자리에서 재평가"* 가 바로 결함의 정의다 — **변이가 그 앞으로 이동**한다.
  측정: 파일 소진 후 `x = $feof(fd)*10 + $fgetc(fd)` 가 vita **9** / iverilog **−1** / PRE loud.
  ⚠️ **값 의존적**이라 더 나쁘다 — 파일 중간에서는 두 도구가 **일치**하므로 끝까지 읽지 않는
  프로브는 틀린 답 위에서 초록이다.

ⓑⓒ 는 별칭 형태를 열거하는 대신 **fail-closed `opaque` 플래그**로 답했고, ⓕ 는 `fd_observer`
플래그와 **조기탈출 순서 반전**으로 답했다(root 비교만 건너뛰고 walk 자체는 항상 돈다).

⚠️ 마지막 NIT 도 **내 수정의 부작용**이었다 — 이중 `shape()` 호출을 접으면서 `Unevaluated` 를
`Uncond` 에 묶어 `$bits(a)` 의 자식이 읽기로 집계됐다(§20.5 상 런타임에 아무것도 안 읽는다).
한 줄 복원.

**측정.** 42칸 census(**FIXED 31 · WRONG 0**) · differential 렌즈의 **112칸 생성 스윕 DIVERGE 0**
· darkriscv 전체 SoC **라이브 iverilog 다이제스트 일치** · examples 4/4 및 코퍼스 10 워크로드
**PRE == POST** · `if ($value$plusargs(…))` VCD **바이트 동일** · 회귀 24건(값은 전부 iverilog 대조)
· ⭐ 스위트가 **옛 제한을 인코딩한 핀 여덟**을 짚었다(`$fopen(…)+0` · `$value$plusargs(…)+1` ·
`$display(…, $value$plusargs(…))` · `$fgetc+1` · `$fgets+1` · `$sscanf+1` · 식 중첩 ·
`(call && x)` 조건) — 전부 오라클 재측정 후 **값을 어서션**하도록 고치고 거절 쌍둥이 둘(`?:` arm ·
`&&` 우항)을 신설. 그중 하나의 docstring 은 *"임의의 식 위치엔 desugar 할 문장이 없다 —
**deliberately, not a gap to close cheaply**"* 라는 **불가능 주장**이었고, 이 슬라이스가 정확히 그
문장을 지었다 · elaborate 세금은 실제 워크로드에서 **30–40 ms 로 잡음
안**(리뷰가 잰 +15% 는 60k 문장 합성 설계) · format 29 불변.

#### 4.5.373 — 리덕션 연산자가 상수 도메인에 없다: 지어서·재서·되돌렸다 (2026-08-24 · format 29 불변 · **제품 코드 변경 0**)

**한 줄**: 코퍼스가 지목한 §3 ⑦ 착수. 넣으면 **serv 가 돌고 다이제스트가 오라클과 일치**했지만
(코퍼스 7/10 → **8/10**), 적대 리뷰 2라운드가 **BLOCKING 넷**을 냈고 **마지막이 좁힘의 근거인
정리 자체를 반증**해서 되돌렸다.

**census.** 진단은 `generate if (|WITH_CSR)` 가 **E3010** 이라고만 말한다. 12칸을 재보니
generate 만의 문제가 아니라 `localparam Q = |P;` 도 loud 였고, **여섯 연산자 · 두 철자 전부**
2-오라클 loud 였다. ⭐ 그리고 **값 walk 하나만** 비어 있었다 — `const_self_width` 는 리덕션에 **1**
을, `const_signed_env` 는 **unsigned** 를 이미 답한다(§4.5.371 이 나를 물었던 *"값만 넓히고 폭
쌍둥이를 안 넓혔다"* 의 **정반대**). 폭-인식 walk 의 `!` arm 주석은 *"`!x` **(and the
reductions)** yield a 1-BIT result"* 라고 적어 두고 구현만 없었다.

**⚠️⚠️ BLOCKING 넷.**

- ⓐ **폭의 provenance**(1라운드). 리덕션이 `const_self_width` 를 **값으로** 소비하는데, 그 함수는
  이름을 `param_meta` 에서 읽고 거긴 **선언 폭과 값-추론 폭이 섞여** 있으며(§4.5.363 이 이미 기록)
  미스는 **32** 로 때운다 ⇒ `localparam W = 4'hF | 4'h0`(참값 4비트, `4'hF`)이 32로 기록돼
  `generate if (&W)` 가 **두 오라클과 반대 가지**를 exit 0 으로 골랐다 = **loud→silent-wrong**.
- ⓑ **`~`/`-` 의 문맥**(1라운드 differential). 그 둘은 **문맥 결정**(Table 11-21)이고 문맥이 없으면
  폭은 피연산자의 **self 폭** — 리덕션이면 1 — 인데 module-scope walk 엔 문맥이 없어 raw i64 를
  돌려줬다: `localparam A = ~(|P);` 가 **4294967294**(두 오라클 **0**).
- ⓒ **좁힘을 한 arm 에만 적용**(2라운드). 정리 게이트를 폭-인식 walk 에만 달고 module-scope walk 은
  **무방비로 남겼다** ⇒ ⓐ 가 `localparam` 철자에서 그대로 살아 있었다(`&W` 가 0, 오라클 1).
  ⚠️ 게다가 그 칸을 덮는다고 쓴 테스트가 **`VAL` 을 어서션하지 않고** `exit 0`·`bits=32` 만 봐서
  **틀린 답 위에서 초록**이었고, docstring 은 *"PRE 도 0 을 찍는다"* 라고 적었는데 **PRE 는
  E3009** 였다 — 회귀를 잔여로 박제할 뻔했다.
- ⓓ ⭐⭐ **정리가 거짓이다**(2라운드, 결정적). 좁힘은 이 정리에 기댔다 —
  *"창을 넓히면 피연산자 **바깥** 비트만 더해지고, 그건 0 이거나(음 아님) 1인데(부호확장된 음수)
  음수는 이미 set 된 비트가 있다 ⇒ «어떤 비트든 set 인가»는 불변"* ⇒ `|`·`~|` 는 이름 위에서도
  허용. **전제가 성립하지 않는다**: vita 는 파라미터를 **주장한 폭으로 감싸지 않고** 기록한다 —
  `parameter A = 4'h1; localparam W = A << 4;` 가 **W=16 · $bits=32** 인데 두 오라클은 **W=0 ·
  4비트**다. 여분 비트가 참 피연산자 **바깥이 아니라 안쪽**에 있어 `|W` 가 1(오라클 0)이고
  `generate if (|W)` 가 가지를 뒤집는다 ⇒ **이름 위의 리덕션은 여섯 전부 신뢰 불가**.

⭐⭐ **판단**: 선행조건이 **두 단계**다 — ⓐ 폭이 declared provenance 여야 하고(그것만도
`param_meta` 를 쓰는 **여덟 자리**의 합의가 필요 = §4.5.371 이 두 번 물린 4-copy 가족), 그 아래
ⓑ **값이 기록된 폭에서 canonical** 해야 한다. 리터럴 피연산자만 허용하는 반쪽은 **수요가 0**
(SERV 는 이름을 쓴다)이라 싣지 않고 전부 되돌렸다.

⚠️ **같은 벽을 세 문으로 쳤다**: §4.5.371 의 select 바운드 · 같은 슬라이스의 concat 폭 · 여기의
리덕션. 셋 다 *"폭을 계산할 수는 있는데 보증할 수 없다"* 에서 멈췄고, 이번에 그 아래 한 겹이 더
드러났다. ⇒ 큐 세 줄이 아니라 **벽 하나**로 §3 에 적었다.

**측정.** 12칸 census 2-오라클 · 구현 후 14칸 전부 일치 · **serv 다이제스트 `f3f45af36093b2b1`
오라클 일치**(8.7 s vs iverilog 7.4 s) · 되돌린 뒤 6칸 **PRE == POST 전부 loud** · 작업 트리
clean · 코퍼스 **7/10 복귀** · **제품 코드 변경 0** · format 29 불변.

#### 4.5.372 — `$fatal` 이 자기 출력을 못 막았다; `$finish` 승격은 지어서·재서·되돌렸다 (2026-08-23 · format 29 불변)

**한 줄**: 코퍼스가 지목한 §3 ⑧ 착수. 실은 것 = **pre-existing silent-wrong 하나**(`$fatal` 이
`$display`/`$write` 의 인자에서 걸려도 그 출력이 나갔다). 되돌린 것 = `$finish`/`$stop` 의 프레임
서브셋 승격 — 적대 리뷰 **4라운드에 BLOCKING 여섯**, 그중 넷이 내 수정이 만든 것이다.

**census 가 진단을 반박했다.** 진단은 *"함수 본문이 **system task call** 을 써서 frame-call 서브셋
밖"* 이라 말한다. 10칸을 재보니 `$display`·`$write`·`$error`·`$warning`·`$fatal`·`$fflush` 와
시스템 **함수**(`$time`·`$random`·`$fopen`)가 **전부 이미 돌고**, **`$finish`·`$stop` 딱 둘**만
거절된다. 두 오라클은 열 개 다 돈다. 문구가 실제 규칙보다 다섯 배 넓었다.

**실은 것.** §20.10 상 `$fatal` 은 종료한다. 그런데 `&self` 프레임 실행기는 `Step` 을 반환할 수
없어 `call_fatal` 을 래치하고 스케줄러가 **문장 경계**에서 처리하는데, `$display("VAL=%0d", f(7))`
에서 그 경계는 **출력 뒤**다 ⇒ iverilog 가 `BEFORE,STOPPING` 에서 멈출 때 vita 는 `VAL=8` 을 한 줄
더 찍었다. ⭐ 자리가 좁아서 고칠 수 있었다 — task 호출과 `r = f(7)` 대입은 **원래 정확**했고, **자기
인자를 평가한 뒤 출력하는 문장**만 틀렸다. 고침 = `render_task_args` **다음에** 래치를 묻고 쓰기를
건너뛴다.

⚠️ 그리고 그 술어는 `call_fatal` 이 아니라 **`call_fatal && !finished`** 여야 했다(리뷰 발견):
래치는 **지워지지 않으므로** 맨 셀로 물으면 `final` 블록의 출력까지 전부 삼킨다 — `final` 은
스케줄러가 래치를 소비한 **뒤에** 돈다. iverilog 는 그걸 찍고 PRE 의 vita 도 찍었다 ⇒
**loud→조용히 삼킨 사용자 출력**이 될 뻔했다.

**되돌린 것 — `$finish`/`$stop` 승격.** 넣으면 **verilog-ethernet 이 elaborate 를 통과한다**.
BLOCKING 여섯 중 넷이 내 수정이 만든 것이다:

1. **라우팅이 셋**(elaborate `frames_classify` · `native/frames` · `frame_eval`) — 하나만 가르치니
   native 가 vm 으로 폴백하며 *"a subroutine statement the frame executor drops"* 라는 **거짓
   진단**을 냈다(실행기는 그걸 실행한다).
2. `Step::Fatal` 소비부 **넷이 전부 `Error` 를 가정** ⇒ `$finish` 가 **exit 1**. 고침 = `call_fatal`
   은 두고 **이유만** `call_end` 로 실어 `latched_end()` 로 읽는다(두 번째 래치를 인터프리터·VM·
   JIT·native 넷에 각각 가르치지 않기 위해).
3. ⭐⭐ **프레임 BB 루프에 bail 이 없어** `$finish` 뒤 문장이 계속 돌며 **출력**했고(`loud→silent`),
   `$fatal; $finish;` 가 `run.json` 을 **`"finish"` 로 재라벨**했다(에이전트가 읽는 그 필드).
4. ⚠️⚠️ 그런데 그 bail 이 **다음 두 개를 만들었다**: ⓐ 중간에 멈춘 본문의 **반환값이 caller 의
   lvalue 에 커밋된다** — `r = f(7)` 이 vita **x**, iverilog **55**(대입 안 함), verilator **21**
   (끝까지 실행) = **셋 다 다른 답**이고 vita 는 어느 쪽도 아니다 · ⓑ `$fdisplay`/`$strobe`/
   `$monitor` 는 억제 검사가 없어 값이 **8 → x** 로 바뀌었다 = **silent↔silent 맞바꿈**(사다리가
   명시적으로 금지).

⭐⭐ **판단**: 승격은 *"본문이 중간에 멈춘다"* 를 요구하고, 멈춘 본문은 **반환값이 정의되지
않는다**. 쓰기-다음-검사 순서는 `mem[f(i)] = 1` 의 E4002 를 지키려고 **의도적으로** 그렇게 돼 있고
(`backend.rs:505-524`), `$finish` 레인을 그 레인과 분리해야 한다. 네 라운드 동안 고칠 때마다 새
BLOCKING 이 나왔으므로 **되돌리고 선행조건을 §3 ⑧ 에 기록**한다(§4.5.361/§4.5.371 선례).

⭐ **리뷰의 다섯 번째는 컨트롤이 무죄를 증명했다** — cont-assign 에서 `BEFORE` 가 빠지는 건
`$finish` 없는 같은 설계도 **PRE 부터** 그렇고, **verilator 도 vita 와 같은 순서**다(iverilog 만
다르다) ⇒ 결함이 아니라 **오라클 분열 축**.

**측정(실은 범위).** `$display`/`$write` 인자 fatal 억제 = **iverilog 와 정확히 일치**(PRE 는 한 줄
더) · `final` 출력 **보존** · `$fdisplay` 값 **PRE 와 동일**(맞바꿈 없음) · task/대입 위치 불변 ·
프레임 없는 설계 불변 · 13설계 PRE/POST **바이트 동일** · 회귀 테스트 **6건** · clippy 0 · fmt 0 ·
format 29 불변.

#### 4.5.371 — concat 의 폭을 값에서 추론했다; count 넓힘은 지어서·재서·되돌렸다 (2026-08-23 · format 29 불변)

**한 줄**: 코퍼스가 지목한 §3 ② 를 착수했는데, **결함이 둘이고 그중 하나만 안전하게 실을 수
있었다**. 실은 것 = concat/replication 의 **폭**(pre-existing silent-wrong 둘). 되돌린 것 =
replication **count** 의 넓힘 — 적대 리뷰 3라운드가 BLOCKING **여섯**을 냈고 **셋이 내 수정이
만든 것**이었다.

**census 가 큐를 반박했다.** 큐엔 *"loud→supported"* 로 적혀 있었는데 11칸 3-오라클은 **전부
2-오라클 합의인데 vita 는 11칸 전부 틀리고, 그중 둘은 loud 가 아니라 silent-wrong** 이라 답했다.

**실은 것 — concat 의 self-폭.** §11.4.12 상 concat 은 자기결정(피연산자 폭의 합)인데 무타입
파라미터 fallthrough 가 접힌 i64 에서 `min_signed_bits(v).max(32)` 로 재고 있었다 ⇒
`{2{32'd2}}` 이 **35비트**(두 오라클 64). 그리고 override 가 없을 때의 `{2{8'h1}}` 이 **32**(두
오라클 16). 둘 다 이미 접히던 형태라 **pre-existing silent-wrong** 이다. 고침 = 타입-결정 계열에
arm 하나(+`const_placement_env` 를 쪼개 폭을 노출하는 `const_placement_wide`).

⚠️⚠️ **그 arm 은 게이트가 둘이고, 둘 다 리뷰가 측정한 뒤에 붙었다.**

- ⓐ **`default_binds`** — §6.20.2 는 무타입 파라미터의 범위를 **최종 override 값**에서 가져오는데,
  선언식에서 폭을 정하니 `#(parameter P = {2{8'h1}})` + `#(.P(32'hDEADBEEF))` 가 **16비트 `beef`**
  (두 오라클 32/`deadbeef`) = correct→silent-wrong. 한 칸 더 나쁜 것도 있었다: `wire [P-1:0] bus`
  를 단 형태는 **PRE 가 loud** 인데 **514비트 버스를 조용히** 지었다(verilator 33,686,018).
  ⭐ 내가 주석에 *"타입-결정 계열에 속한다"* 고 적은 것이 정확히 틀린 자리다 — **선언 타입은
  override 를 견디지만 값-결정 폭은 못 견딘다**. 게이트는 **네 override 채널 전부** 를 본다.
- ⓑ **leaf 단위 provenance** — 그 arm 을 `declared_only` 아래서도 답하게 두고 *"폭이 피연산자의
  선언 폭에서 온다"* 고 정당화했는데, ⚠️⚠️ **그것을 계산하는 resolver 가 반증한다**: 이름 폭을
  `param_meta`(값-추론 폭이 기록되는 자리)에서 읽고, 없으면 `unwrap_or((32,false))` 로 **추측**한다
  ⇒ `localparam W = ~8'hCB; localparam Q = {W}; logic [(Q[15:8])+8-1:0] v;` 가 **263비트** net
  (iverilog 1) = §4.5.363 이 닫은 회귀의 **concat 문 재진입**. 고침 = leaf 가 **전부 sized 리터럴**
  일 때만 declared 라 답한다(이름 leaf 는 declines · **count 는 안 본다** — 폭을 배수할 뿐 자기
  비트를 안 낸다).

**되돌린 것 — count 넓힘.** `fold_self_bits` 의 `Replicate` arm 이 count 를 **자유함수**
`const_eval_u32` 로 접어 리터럴 전용이었고, `{S_COUNT{32'd2}}`(모든 파라미터화 AXI/Ethernet 코어의
per-port 관용구)가 통째로 loud 다. 넓혀 봤고 **verilog-axi 의 elaborate 에러가 54 → 4** 까지
내려갔다. 그런데 그 길에서 BLOCKING 이 넷 나왔고 **마지막 셋은 전부 내 수정이 만든 것**이다:

1. **폭 쌍둥이를 안 넓혔다**(`const_fn_width::const_self_width`) ⇒ `{N{4'd15}}` 이 **값은 있고
   self 폭은 없는** 노드가 되고 폭-인식 walk 가 **무제한 도메인으로 degrade** 해
   `({N{4'd15}} + 4'd1) > 4'd0` 이 **1**(두 오라클 0). ⭐ **오라클 없이 유죄**였다 — 같은 파일의
   리터럴 쌍둥이 `{2{4'd15}}` 는 0 을 답한다.
2. count 를 이름 resolver 로 접으니 상수함수 **지역변수**가 잡혀 **170**(§11.4.12.1 상 count 는
   상수식 · iverilog 가 그대로 말한다: *"a reference to a net or variable is not allowed in a
   constant expression"*). ⇒ 표현식 콜백 `CountFoldFn` + `const_eval_in_scope` 로 바꿨더니
3. 그 evaluator 는 **모듈 스코프**라 같은 이름의 지역변수가 모듈 파라미터를 **가릴 때 파라미터가
   이긴다** ⇒ **43690**(= `{4{4'hA}}`, 모듈 param 의 4). 170 을 지우고 43690 을 설치한 셈이다.
4. 그리고 그 evaluator 는 **콜 깊이를 0으로 재시작**한다(같은 파일이 400줄 위에서 그 이유로
   `ast_contains_call` 가드를 두고 있다) ⇒ count 안의 호출이 **스택 오버플로 · abort**.
   PRE 는 loud 였다. loud→crash 는 사다리 하강이다.

⭐⭐ **판단**: count 축은 (a) 상수함수 **스코프 경계**, (b) 상수함수 **콜 깊이**, (c) `declared_only`
**provenance** 세 가지를 동시에 만족해야 하고, 그 셋은 이 슬라이스의 폭 축과 **다른 머신러리**다.
세 라운드 동안 고칠 때마다 새 BLOCKING 이 나왔으므로 **되돌리고 메커니즘을 기록**한다(§4.5.361
선례). 폭 축만 실었고 그 결과 **verilog-axi 는 PRE 그대로**다.

**측정(실은 범위).** `{2{32'd2}}` 폭 **35 → 64** · 미override `{2{8'h1}}` 폭 **32 → 16**(둘 다
두 오라클 일치) · 되돌린 셋(b1 지역변수 · b2 재귀 · b3 select 바운드) **PRE == POST 전부 loud** ·
override `#(.P(32'hDEADBEEF))` **PRE == POST == iverilog** · 13설계 PRE/POST **바이트 동일 13/13** ·
회귀 테스트 **9건** · clippy 0 · fmt 0 · format 29 불변.

**잔여(전부 테스트로 핀).** ⓐ **count 는 여전히 리터럴 전용**(§3 ② · 위 메커니즘) · ⓑ concat
파라미터에 대한 **select 바운드가 조용히 1비트**(`localparam W={2{8'h09}}; logic [W[7:0]-1:0] v;`
가 1, iverilog 9 — **PRE 도 1** 이라 이 슬라이스가 만든 것이 아니고, 폭이 아니라 **기록된 range**
(`param_sel_range`)가 필요하다) · ⓒ override 채널이 32비트라 64비트 concat 을 조용히 자른다.

#### 4.5.370 — 문자열 상수 도메인이 리터럴 전용이었다; 참조 구현은 소비자가 하나뿐이었다 (2026-08-23 · format 29 불변)

**한 줄**: 워크로드 코퍼스(§4.5.369)가 *"서드파티 여덟 중 셋을 막는 한 축"* 으로 지목한 §3 ①.
⭐⭐ 그리고 **참조 구현이 이미 트리 안에 있었다** — `const_fn.rs::const_str_in_scope` 가
StrLit·Paren·Ident·PkgScoped 를 이미 풀고 있었는데 **호출자가 하나**(문자열 **동등비교**)라
아무도 **값**을 묻지 않았고, 파라미터 바인딩 일곱 자리는 전부 리터럴 전용 쌍둥이
`param_str_literal` 을 쓰고 있었다.

**census 가 큐를 넓혔다.** 큐엔 *"문자열 삼항"* 한 줄이었는데 16칸 3-오라클은 **15칸이 2-오라클
합의인데 vita 는 전부 loud** 라고 답했다 — 바레 리터럴을 뺀 **문자열 상수 도메인 전체**
(삼항[리터럴 조건·문자열비교 조건·정수 조건·중첩·한 arm 이 Ident] · Ident 전달 ·
인스턴스 override · concat · generate-if 소비 · 패키지).

**고침.** 그 도메인에 두 arm — `Ternary`(§11.4.11 · 조건은 정수 도메인과 **같은 철자**
`const_int_selfdet` · **양 arm 이 다 문자열이어야** 한다 = fail-closed, 안 그러면 같은 원문이
override 값에 따라 도메인을 바꾼다)와 `Concat`(§11.4.12) — 을 더하고, 파라미터 **값** 소비자를
넓힌 리졸버로 라우팅. ⚠️ `systask.rs` 의 진단 가드는 *"리터럴인가"* 라는 **다른 질문**이라 그대로.

**⚠️⚠️ 적대 2렌즈 + 재리뷰 3라운드 · BLOCKING 넷 — 그중 셋이 내 수정이 만든 것이다.**

- ⓐ **따옴표 밀수(1라운드 자가 발견)**: 이 도메인의 값은 **따옴표를 포함한 raw** 라 raw 를 그냥
  이어 붙이니 `{"RE","D"}` 가 `RE""D`(정수로 1092756034)가 됐다 = **loud→silent-wrong** ⇒ 내용만
  잇고 다시 따옴표.
- ⓑ ⭐⭐ **폭 축(soundness)**: `str_param_raw` 는 **폭을 안 들고 다니고** 문자열 경로는 폭을 들고
  다니는 수치 경로보다 **먼저** 돈다 ⇒ `localparam [95:0] X = {"A","B"}` 가 96비트를 **16비트**로,
  `[95:0] Z = 1 ? "AB" : "CD"` 는 iverilog 0 을 **16706** 으로 접었다(PRE 는 둘 다 loud) ⇒
  **폭 게이트**: 넓힌 fallback 은 선언이 폭·정수타입을 안 들고 있을 때만.
- ⓒ **빈 피연산자(differential)**: `{"ab",""}` 의 `""` 는 **NUL 한 바이트**(두 오라클 0x616200)인데
  텍스트 join 은 0글자로 표현해 0x6162 ⇒ **declines**(표현이 못 담으면 loud 가 정직하다).
- ⓓ **가드가 값 도메인에 물었다(두 렌즈가 독립적으로)**: *"이 파라미터는 문자열인가"* escalation 을
  넓힌 리졸버에 물으니 기본값이 우연히 문자열 식인 평범한 수치 파라미터
  `parameter W = {"A","B"}` 가 합법적 `#(.W(9))` 를 거절 = **correct→loud**. ⚠️⚠️ 그런데 가드만
  되돌리니 이번엔 **폴드된 기본값이 override 를 삼켜** 9 대신 16706 이 exit 0 로 나왔다 —
  loud 고치려다 **silent-wrong** 을 만든 것 ⇒ 셋째 버전 = *"숫자 override 가 겨누면 폴드
  fallback 은 물러선다"*(`overridden` 플래그) + override 채널엔 `str_is_literal` 로 같은 폭 게이트.

**⚠️⚠️ 재리뷰(설계가 바뀌면 재리뷰) BLOCKING 하나 — 게이트의 술어가 틀렸다.**
`p.range.is_none() && p.ty == Implicit` 로 *"선언이 폭을 말하지 않는다"* 를 표현했는데,
**범위 없는 `logic`/`reg`/`bit`** 가 그 틈으로 빠진다: 폭은 **1비트**인데 `ty` 는 `Implicit` 이고
`range` 는 `None` 이다. 파서는 그걸 `var_kind` 에 기록했다가 **버렸고** `ParamDecl` 엔 그 필드가
없어 `parameter bit P` 와 `parameter P` 가 하류에서 **구별 불가능**했다 ⇒
`localparam bit P = {"A","B"}` 가 두 오라클 1비트를 **16비트 16706** 으로 = 또 loud→silent-wrong.
⭐ 고침은 파서 **세 줄** — `var_kind` 가 `finish_param_assignment` 까지 살아 있으므로 range 의
**마지막 fallback** 으로 `[0:0]`(⚠️ `forced_range` 에 넣으면 explicit range 를 덮어
`parameter logic [7:0] P` 가 1비트가 된다).
⭐⭐ **곁수확 = pre-existing silent-wrong 해소**: 그 틈은 문자열과 무관하게 이미 새고 있었다 —
`localparam bit N = 8'hFF` 를 vita 는 **255/8** 로 읽었고 두 오라클은 **1/1** 이다. 이제 일치.

**측정.** 27칸 3-오라클 PRE/POST **FIXED 15 · ok→wrong 0 · loud→loud 7 · 불변 5** ·
14설계 PRE/POST 바이트 동일 **12/14**(다른 둘은 이 슬라이스의 표적: serv 에러 **10→3**,
verilog-ethernet **6→2**, 둘 다 순수 감소) · 회귀 테스트 **20건** 신설(5,845 → **5,865 green**) · clippy 0 · fmt 0 ·
format_version **29 불변**.

**부수.** `const_fn.rs` 가 1402줄(정책 ≤1000)이라 문자열 도메인을 **`const_str.rs`** 로 분리
(1188 + 242) · Ident 술어에 `wide_param_bits` 누락 보완(불변식이 *다른 맵의 우연*으로 성립하고
있었다) · `str_raw_content` 를 fail-closed 로 · depth 가드 신설(`const_compare_special` 이 모든
binary 노드의 양 피연산자에 이걸 물어 Θ(n²) 위험).

**잔여.** ⓐ 패키지 스코프는 여전히 loud — 파라미터 선언 fold 의 **네 번째 복사본**(`package.rs`)이
string/real 을 아예 라우팅하지 않는다(§3 ⑨ · 회귀 테스트가 loud 로 핀). ⓑ `localparam integer Q =
1?"A":"B"` 류는 **의도적으로 loud**(폭을 잃느니 거절). ⓒ 폭 손실 자체는 pre-existing 클래스 —
리터럴 override 는 PRE 부터 이미 폭을 잃고 있었고 이 슬라이스는 **그 클래스를 넓히지 않는 것**만
책임진다.

#### 4.5.369 — 워크로드 코퍼스: 남이 쓴 RTL 로 값을 매기기 (2026-08-23 · 5,845 green · format 29 불변 · **제품 코드 변경 0**)

**한 줄**: 성능 판단이 설계 **두 개** 위에 서 있었다 — `picorv32`(서드파티)와 `bench/keccak`(**우리가
재려고 쓴 것**). 허가적 라이선스 서드파티 **여덟**을 오라클로 고정해 훑으니 그림이 **양방향으로**
뒤집혔고, 첫 수확은 성능이 아니라 **정확성**이었다.

**왜.** §4.5.367 이 *"아레나 = 6–10주 · 상한 2.33×"* 라는 값을 매겼을 때 그 65.0% 도 2.33 도
`keccak_f_arr` **하나**에서 나온 수다. 설계 하나에서 나온 천장은 **그 설계의 성질**이지 시뮬레이터의
성질이 아니다.

**방법 — 오라클 먼저, vita 나중.** 여덟 후보를 각각 독립 스카우트가 맡아 같은 순서를 밟았다:
clone → 라이선스 → 자립 실행 구성 → **iverilog** → *그 다음에야* vita. vita 를 먼저 보면 거절당한
구문을 피해 테스트벤치를 다시 쓰게 되고, 그렇게 만든 워크로드는 **vita 가 이미 할 줄 아는 것만**
잰다. 지침에 그 문장을 그대로 넣었다 — *"vita 가 거절하거나 틀리는 설계는 네가 가져올 수 있는
가장 값진 결과다. RTL 을 단순화·재작성·축소하지 마라 — 그건 측정을 파괴한다."*

**정확성 수확 — 거절 셋이 전부 한 축.**

| 설계 | 구문 | 도메인 |
|---|---|---|
| verilog-ethernet | `parameter SI = (S=="AUTO") ? "RED" : S;`(`lfsr.v`) | 문자열 삼항 |
| serv | `.RESET_STRATEGY(reset_strategy)` | 문자열 Ident |
| verilog-axi | `parameter S_THREADS = {S_COUNT{32'd2}};` | 정수 replication |

앞의 둘은 **같은 함수**다 — `param_str_literal` 이 `StrLit`·`Paren` 두 팔뿐, 즉 문자열 상수
도메인이 **리터럴 전용**이다(§4.5.364 의 *"구조적 지연의 값 fold 가 리터럴 전용"* 과 같은 모양).
⭐ 최소 재현이 축을 **삼항 하나**로 좁혔다: `(S=="AUTO")` 정수 localparam · `parameter SI="RED"` ·
generate-if 의 문자열 비교는 **셋 다 이미 정확**하고, `1 ? "RED" : "BLUE"` 처럼 조건이 리터럴이어도
거절한다. iverilog·verilator **둘 다** `RED` 를 접는다 ⇒ 축 자체에 2-오라클.

그 밖에 — `$fgetc`/`$value$plusargs` 류가 **blocking 대입 rhs 에서만** 산다(darkriscv SoC 의
`UART_RFIFO <= $fgetc(fd)` · serv tb 의 `if (!$value$plusargs(…))` · 4줄 재현: `c = $fgetc(fd)` 는
양쪽 65, `c <= $fgetc(fd)` 는 iverilog 66 / vita E3009) · `$readmemh(f, dut.ram.mem)` 계층 원소
선택(SoC 벤치의 정석 관용구) · `$display("…",)` 널 인자(§17.1.1.2 가 허용) · **ibex 는 오라클이
없다**(iverilog 13 이 `ibex_pkg.sv` 의 struct 타입 localparam 에서 syntax error, positional 로
바꾸면 `net_scope.cc:449` assertion 으로 **abort**) ⇒ 코퍼스에서 뺐지만
[[no-oracle-not-a-defer-reason]] 대로 hand-IEEE 대상으로 §3 ⑤ 에 남겼다.

**성능 수확 — 양방향으로 뒤집혔다.**

| | vita | iverilog | 비 |
|---|---|---|---|
| sha256 | 1.441 | 4.171 | **2.89×** |
| biriscv | 4.967 | 9.315 | **1.88×** |
| aes | 3.541 | 6.154 | **1.74×** |
| picorv32 | 4.512 | 6.682 | **1.48×** |
| keccak *(1st-party)* | 8.426 | 9.295 | 1.10× |
| darkriscv | 9.187 | 7.194 | **0.78×** |
| keccak-arr *(1st-party)* | 17.237 | 9.169 | **0.53×** |

기하평균 **1.30×**(도는 일곱) · **1.61×**(서드파티 다섯). ⭐ **우리가 쓴 둘을 빼면 올라간다** —
keccak 은 유리한 벤치가 아니라 **가장 어려운 벤치**였다. ⚠️ 지는 둘의 **공통점은 아직 안 쟀다**:
`keccak_f_arr` 는 호출마다 25원소 배열을 짓고 `darkriscv` 는 그런 게 없다 ⇒ 다음 성능 계측의
첫 표적은 **darkriscv**.

**도구.** `crates/corpus-runner` 가 1줄 stub 에서 실물로. **의존성 0**(std 만 — 워크스페이스가
`--locked` 로 3-OS 재현성을 지키는데 일 년에 몇 번 바뀌는 파일을 읽자고 파서를 들이는 건 값이 안
맞는다) · 매니페스트는 Rust `const` 테이블 · **거절이 일급 상태**(`Expect::Refused { diag }`)라
`known-gap`(초록 — 사다리가 작동 중) / `DRIFTED`(핀이 더는 설계를 설명 못 함) / `PROMOTED` /
**`REGRESSION`(loud→silent-wrong)** 이 갈린다.

**⚠️⚠️ 적대 soundness 렌즈 BLOCKING 넷 — 넷 다 도구가 자기 목적을 배신하는 모양이다.**

- ⓐ ⭐⭐ **`Grade::Promoted` 가 도달 불가능했다.** `expect_exit` 를 `expect` **옆의 자유 필드**로
  뒀고, 거절 행 셋은 vita 가 *거절하면서* 내는 코드라 `1` 이었다. 갭이 닫히는 날 vita 는 **0** 으로
  나가 동등 비교가 깨지고 등급은 **`"was loud, now crashes"`** — **코퍼스가 존재하는 이유인 그
  사건이, 거짓 문구로 실패 보고되게 배선돼 있었다.** `Grade::Promoted` 와 그 메시지가 통째로 죽은
  코드. 고침 = 값을 타입 안으로, **`Expect::Runs { exit }`** ⇒ 거절 행엔 exit 코드를 **적을 자리가
  없다**. (`aes` 도 같은 모양의 잠복 — 오늘 정답을 찍으며 exit 1 이라, 그 과잉-loud 를 고치면 행이
  회귀로 뜬다.)
- ⓑ **여덟 행이 다른 기계에서 복원되지 않았고, 그 실패를 vita 회귀로 채점했다.** 테스트벤치는 전부
  1st-party 인데 `bench/*` 가 통째로 gitignore 였다 — 핀된 SHA 는 **상류 RTL** 을 복원하지
  **하네스**를 복원하지 않고, 다이제스트를 만든 건 하네스다. 게다가 존재 판정이 `cwd.is_dir()` 라
  `fetch` 가 `bench/<name>/src` 를 만드는 순간 `absent` 가 아니라 **실행**되고
  `cannot read 'tb.v'` 가 `Refused` 로 읽혀 **"newly refused" 회귀 여덟 개**가 뜬다. 고침 둘 =
  gitignore 를 **allow-list** 로 뒤집어(상류 클론·빌드 산출물만 제외) 하네스 30파일 커밋 + 존재
  판정을 **소스와 런타임 데이터 파일**로. ⚠️ 곁가지로 펌웨어 둘이 실은 상류 것임이 드러났다
  (serv `blinky.hex` 는 상류 `sw/` 와 바이트 동일 · biriscv `prog.hex` 는 상류 `test.elf` 추출)
  ⇒ 커밋하지 않고 클론 참조 또는 `prepare.sh` 재생성.
- ⓒ **파이프를 안 비웠다.** `try_wait` 폴링 후 `wait_with_output` 하는 흔한 모양은 워크로드가 파이프
  버퍼를 넘기는 순간 **교착**한다(자식이 `write` 에서 막히니 안 끝나고, 부모는 안 끝났으니 안
  읽는다) ⇒ 예산 600초를 다 쓰고 `Timeout`, `Expect::Refused` 행에서는 **`"was loud, now hangs"`**
  — **정직하고 즉각적으로 거절하는 설계를 행업으로 무고한다.** 가설이 아니다: `verilog-axi` 는 2×2
  크로스바 하나로 **19,238 바이트**를 stderr 로 쏟고 macOS 파이프는 16 KiB 로 시작한다(내 주석은
  *"한 줄이라 버퍼 안쪽에 한참 못 미친다"* 고 적고 있었다). 고침 = 양 파이프를 **각자의 스레드로**
  드레인 · 곁가지로 stdout/stderr 를 **분리**(다이제스트는 stdout, 진단은 stderr).
- ⓓ `cargo fmt --all -- --check` 실패 — CI 게이트.

**NIT 열셋 중 남는 셋.**

- ⚠️⚠️ ⭐⭐ **내가 ENGINEERING_RULES 에 ★★★ 교훈으로 적은 *"호출자가 순차 측정을 할 방법이
  없다"* 가 거짓이었다** — `measure(&jobs[0..1])` 를 두 번 부르면 그게 순차이고, 그게 바로 가짜
  +12.5% 를 낸 모양이다. 인터리브는 **타입의 성질이 아니라 호출부의 성질**이었다. 하필
  *"규칙은 타입이어야 한다"* 를 주장하는 문단에서 **타입이 아닌 것을 타입이라 불렀다**. 정정했고,
  그게 **진짜로 가능했던** 자리는 ⓐ 다.
- **비결정성 검출기가 죽은 코드였다** — `Grade::Ok` 뒤에 가둬 뒀는데 다이제스트가 갈리면
  `Mismatch` 로 은퇴하므로 `Ok` 에 도달할 수 없다 ⇒ **흔들리는 툴이 일관되게 틀린 툴로** 보고된다.
  이제 등급보다 먼저 본다.
- **`--reps 3` 이 샘플 둘을 줬다**(라운드 셋 중 하나 폐기) — "median" 이라 적힌 평균이다 ⇒
  `--reps` 를 **타임드 샘플 수**로 재정의(라운드 N+1).

그 밖에 반영: `serv` 의 두 툴이 서로 다른 루트를 elaborate 하던 것(`-s tb` 추가) · 핀된 진단을
**stderr 전체**에서 찾는다(verilog-ethernet 은 경고 24줄 뒤에 있어 emission order 에 의존하고
있었다) · `.vvp` 가 핀된 상류 체크아웃 안에 떨어지던 것 · `--reps`/`--filter` 의 조용한 폴백 ·
20 ms 폴링 → 2 ms.

**⚠️⚠️ 후속 — 멈춰 선 렌즈의 한 문장이 최대 발견을 냈다.** differential 렌즈가 5시간 stall 후
킬됐는데, 그 시점의 부분 출력이 *"picorv32 의 다이제스트가 코어의 덧셈기를 변형했는데 안
움직인다"* 였다. 사실이었다:

```
picorv32.v:1240   reg_op1 + reg_op2  →  reg_op1 + reg_op2 + 32'd1
CONTROL : DIGEST=7836648e76208dc9
MUTATED : DIGEST=7836648e76208dc9      ← 바이트 동일
```

⭐ 근인은 테스트벤치가 아니라 **프로그램**이었다 — `addi`/`add`/`beq` 루프뿐이라 계산된
레지스터 값이 **버스에 한 번도 안 나온다**. 다이제스트가 볼 수 있는 건 버스뿐이므로 그건
**설계 검사의 이름을 단 프로그램 카운터 추적**이었다. ⚠️⚠️ **정상과 구별되지 않는다**:
결정성·오라클 일치·사이클 해상도·재실행 바이트 동일이 **전부 통과**한다. 통과 못 하는 유일한
검사가 **변형**이다.

고침 = 프로그램이 매 바퀴 `sw x5, 64(x0)` 로 저장하고 `x1` 을 바퀴 사이로 나른다 ⇒ 같은 변형이
이제 움직인다. 새 다이제스트 `68d30f61bf9bf1d4`(iverilog 일치 · N=400000 · 7.05 s), picorv32
비 1.48 → **1.44**, 서드파티 기하평균 1.61 → **1.60**.

이건 한 워크로드의 사고가 아니라 **계약의 구멍**이었으므로 계약에 **5번**을 넣고(*"설계를
건드리면 다이제스트가 움직여야 한다"*) 나머지 여덟도 상류 RTL 한 줄씩 변형해 확인했다 —
**아홉 전부 움직인다**(sha256 라운드 상수 · aes `new_block` · biriscv `ALU_ADD` · darkriscv
`RMDATA` · serv `result_add`[X 로 발산] · keccak theta rotl · axi_ram read 경로 ·
axis_gmii_rx 데이터패스). ⚠️ **죽은 변형 주의**: verilog-ethernet 이 처음 *"안 움직인다"* 를
냈는데 결함이 아니었다 — **루프백이라 TX·RX 가 같은 `lfsr` 인스턴스**를 쓰고, CRC 다항식을
바꾸면 TX 가 틀린 FCS 를 붙이고 RX 가 그걸 **정확하다고 검증**해 상쇄된다. RX 쪽만 건드리니
즉시 움직였다 ⇒ **변형은 비대칭이어야 한다**.

**게이트.** 5,821 → **5,845 green**(러너 16 + 매니페스트 위생 8) · clippy 0 · fmt 0 ·
format_version **29 불변** · **제품 코드 변경 0**.

#### 4.5.368 — canonical `Value` 를 재수립하지 말고 주장하라 (2026-08-23 · 5,821 green · format 29 불변)

**결함.** `Value::resize` 의 `new_width == self.width` 팔이 `self.mask_top()` 을 무조건 부른다. 그런데
`Value` 는 **구성상 canonical** 이다 — 두 평면이 정확히 `nwords(width)` 워드이고 top 워드에 `width` 위
비트가 없다. 즉 이미 성립하는 것을 재수립한다.

**자리 특정 = 콜 귀속.** `mask_top` 은 self **15.9%**(keccak_f_arr, release+symbols)이고 그중 **40.6%**
가 바로 이 호출이다(≈6.5%). ⭐⭐ 그리고 `mask_top()` 호출부 **31곳 중 30곳은 생산자** — raw 워드에서
값을 만들며 불변식을 *세운다*. **이 하나만 소비자**였다. ⇒ 호출을 `debug_assert!(is_canonical())` 로
바꿔 부담을 생산자에게 옮겼다.

**⚠️⚠️ 적대 soundness 가 그 불변식을 엔진으로 반증했다 — BLOCKING.**
`eval/sysfunc.rs` 의 `$realtobits` 는 `v.width = 64` 를 찍으면서 평면은 **인자 폭** 그대로 둔다.
`$realtobits(<128비트>)` 는 따라서 width 64 · 2워드짜리 non-canonical 을 만들고, 그 값이 이 팔에 도달해
**디버그 빌드를 패닉**시킨다. release 는 PRE 와 바이트 동일하게 정상이므로 이것은
**correct → loud**, 사다리의 금지 방향이고, 하필 CLAUDE.md 가 표준 빌드로 적어 둔 구성에서 난다.

⭐⭐ **그리고 이게 이 슬라이스의 진짜 교훈이다.** 나는 *"`debug_assert` 를 박고 5,812 테스트를 돌렸는데
한 번도 발화하지 않았다"* 를 증명으로 제시했다. 그건 **커버리지 진술**이다 — 어떤 테스트도
`$realtobits` 를 64비트 아닌 인자로 부르지 않았을 뿐이다. **제거된 방어 검사는 초록 스위트가 아니라
생산자 전수 census 로 정당화해야 한다.**

고침은 그 생산자 한 줄(`v.mask_top()`). 곁가지로 differential 이 `native/arena.rs` 의 zero-width OOB
arm 에서 같은 결함 클래스를 찾았다 — `Value::xs(width.max(1))` 뒤에 `v.width = s.width` 를 찍으므로
폭 0이면 `unk` 비트 하나가 위에 남는다. 오늘 도달 불가(폭 0 배열 net 이 만들어지지 않는다)지만
**불변식이 side condition 에 기대면 안 되므로** 함께 고쳤다.

**⚠️ 두 렌즈의 상보성이 실증된 자리.** differential 은 **~840 PRE/POST 쌍**(assert 가 살아 있는 debug
빌드로 패닉 사냥 — >128비트 `Words::Heap` 산술·dyn/queue/assoc(문자열 키 포함)·class·packed
struct/union·string↔bits·`$readmem`·force/release·real↔int·`$random`/CRV·SVA·covergroup·fork·계층
읽기/쓰기·interface·program·>64비트 파라미터·VCD/FST·OOB 배열 읽기)에서 **패닉 0 · 차분 0** 으로
CLEAN 이었다. 결함은 **soundness 가 코드 경로 census 로** 잡았다.

**측정.** keccak_f_arr **3.75 → 3.34 s (−11.1%)** · keccak_f **1.92 → 1.66 s (−13.2%)** ·
keccak_f_flat +0.4% · picorv32 −0.9%(노이즈) · 4-way anchor·VCD·FST **바이트 동일** ·
release 에서 `debug_assert` 와 `is_canonical` 이 **문자열·심볼 부재로 컴파일아웃 확인** · 새 회귀 9건
(불변식 유닛 6 + `$realtobits` 커버리지 3).

**⚠️ NIT 둘.** ⓐ 6.5% 귀속과 11–13% 벽시계는 **두 측정**이고 나머지 갭은 이론이다(호출을 없애면
`Value` 가 escape 하지 않아 호출부가 두 평면을 스택에 흘리지 않는다) — 주석이 그렇게 말하도록 고쳤다.
ⓑ **순차 A-then-B 타이밍이 picorv32 에서 가짜 +12.5%** 를 냈다(0.45 s 워크로드에서도) ⇒ PRE/POST 를
**런 단위로 인터리브**. 직전 슬라이스의 *"debug 바이너리로 +88%"* 와 같은 계열이다.

**잔여**(§3): `$realtobits`/`$bitstoreal` 이 64비트 아닌 인자를 애초에 받는 것(iverilog 는 거부).

#### 4.5.367 — frame part-select 쓰기의 per-bit 루프 (2026-08-23 · 5,812 green · format 29 불변)

**착수는 구현이 아니라 측정이었다(S0).** 오너가 VCS급 목표를 유지하기로 하면서 성능 축이 열렸고,
전 라운드의 분석이 지목한 frame 축을 **가격**하는 것이 첫 일이었다.

- `/usr/bin/sample`, keccak_f_arr, 작업 스레드 5,318 샘플: `run_frame_call` **65.0%**,
  그 안 `frame_write_lvalue` inclusive 36.9%.
- `run.json` 의 codegen 리포트: keccak_f/_arr = `able 1/4 · frame_bodies 3` · keccak_f_flat =
  `able 2/4 · frame_bodies 0`. **15.8× 성능차가 frame body 3개와 정확히 상관.**
- ⚠️⚠️ **계측이 앞선 리뷰의 메커니즘 주장을 반박했다.** `wprog.rs:441` 의 frame-local decline 게이트는
  keccak 과 picorv32 **둘 다에서 한 번도 발화하지 않는다**(`frame_decline=0`, 임시 카운터 실측).
  이유는 게이트가 관대해서가 아니라 **아무도 묻지 않기 때문**이다 — `wprog::compile` 은
  `ir.processes[proc].body` 에만 호출된다. 그리고 `WProg::run` 은 값을 `arena.buf[slot]` 으로 읽는데
  frame local 에는 슬롯이 없다.
  ⇒ **65% 를 컴파일 레인으로 보내려면 frame local 에 arena 슬롯을 주는 것이 선행조건**이고, 그건
  6–10주짜리 별건이다(상한 = `Value` 47.4% + frame/callstack 9.7% ⇒ **2.33×**). 이 슬라이스는 그 65%
  안에서 **bounded 한 조각**만 가져간다.

**결함.** frame slot 과 dyn-array 원소의 part-select **쓰기**가 값을 **한 비트씩** 예치한다. 그 루프는
동시에 IEEE §11.5.1 의 **범위 밖 DROP** 을 구현하므로 지울 수 없다 ⇒ 창이 net 안에 완전히 들어갈 때만
word-parallel 로 가고, **else 는 pre-slice 루프 verbatim**.

**⚠️⚠️ `copy_bits` 를 그대로 부르면 silent-wrong 을 배포한다.** 그 함수의 doc 이 못박는다 —
*"The destination range must be ZERO on entry … bits are OR-merged in"*. part-select 쓰기의 대상은 슬롯의
**현재 값**이라 `8'hF0` 에 `8'h0F` 를 쓰면 `8'hFF` 가 된다. ⇒ 형제 `replace_bits` = **창을 지우고 나서**
같은 `copy_bits` 를 부른다(비트 이동의 철자는 하나로 유지).

**적대 2렌즈 — BLOCKING 0.**
differential 이 246칸 + **200칸 랜덤 퍼즈**(net 폭 17종 × lsb ∈ [−w−2, net_w+2] × `[h:l]`/`+:`/`-:` ×
01xz 패턴)를 돌려 **PRE≡POST 전 칸**, iverilog 일치 159/159. soundness 가 clear 창과 write 창이
**정확히 같음**(시작·길이·두 평면·모든 정렬)을 구성으로 증명하고, 게이트가 *"per-bit 루프가 모든 비트를
썼을 조건"* 과 **동치**임을 보였다(루프 자신의 가드를 밖으로 끌어낸 것).

**⚠️ NIT 넷 — 하나는 내 절차 실수다.**

1. **리뷰 스냅샷이 debug 바이너리였다.** 성능 슬라이스인데 `target/debug/vita` 를 넘겼다. soundness 가
   +88% picorv32 *"회귀"* 를 재다가 알아채고 스스로 release 를 다시 빌드했다. **성능 슬라이스의 스냅샷은
   release 여야 한다** — 루프 규칙으로 승격.
2. `replace_bits` 는 `copy_bits` 가 `dst.val[dw]` 를 **직접 인덱싱**하므로 `set_vu`(늘린다)와 달리
   **할당을 늘리지 않는다**. 오늘 도달 불가지만 문서화 + 각 호출부에 `debug_assert`.
3. 게이트가 두 파일에 **손으로 복사**돼 있었고 bound 철자도 둘이었다(`net_w`/`w`) ⇒ `window_in_range`
   한 철자로. §4.5.359 의 *"복사했으면 한 규칙의 여섯 철자"* 를 **둘일 때** 잡았다.
4. 성능 주장의 **메커니즘**: `Value::set_vu` 는 인라인되지 않아 자기 스택 프레임으로 나타난다 ⇒ 표적은
   self 6.6% 가 아니라 **그 leaf 12.8% 를 더한 ~19%** 다. 숫자만 있고 메커니즘이 없으면 결과가 아니다.

**측정.** keccak_f_arr **4.49 → 3.79 s = −15.6%**(release · best-of-3 · 첫 런 폐기) · keccak_f 1.929→1.928 ·
keccak_f_flat 0.124→0.123 · picorv32 0.450→0.450(전부 노이즈 내) · 4-way anchor 불변 ·
examples+bench **8/8** stdout·stderr·exit 바이트 동일 · 새 회귀 6 테스트.
vs iverilog: keccak_f_arr **0.41× → 0.50×**(vita 의 유일한 패배가 좁혀졌다).

**잔여**(§2): queue·assoc 원소의 part-select 쓰기가 **조용히 사라진다**(dyn 철자만 구현됨 = write-twin 갭) ·
폭 0 인덱스 part-select 를 조용히 받는다(iverilog 는 거부) · gather 모양 세 자리(`task_frames`·`netread`·
`init_diag`)는 소스가 `Value` 가 아니라 raw store 라 **다른 기계** — 후속으로 분리.

#### 4.5.366 — 상수 도메인이 정확히 64비트에서 unsigned 를 잃는다 (2026-08-23 · 5,806 green · format 29 불변)

**결함.** `localparam L = ((64'd1 - 64'd2) > 64'd0) ? 111 : 222;` 가 vita **222**, 두 오라클 **111**.
⭐ **63비트 쌍둥이도 런타임 철자도 이미 정확**했다 — 내부 판별자가 확보된, 상수 도메인만의 결함.

**근인.** `eval_const_env_at` 은 값을 **문맥 폭·부호로 마스킹**해 정규화하는데 그 게이트가
`masking = ctx_w > 0 && ctx_w < 64` 다. 64비트 미만에선 unsigned 가 zero-extend 되어 signed i64
연산이 **우연히** 맞고, 정확히 64에선 마스킹이 **항등**이라 값이 최상위 비트를 쥔 채 남는다 ⇒
**부호 민감 연산**이 그것을 음수로 읽는다.

**규칙은 하나, 소비자는 넷.** `const_i64_is_unsigned_at(w, signed) = !signed && w == 64` 를
폭 인식 walk 의 네 자리가 묻는다 — 순서비교(피연산자 **쌍**의 `(w, cs)`) · `/`·`%`(**둘러싼** 문맥) ·
논리/산술 시프트 · `_` leaf arm 의 재해석. `==`/`!=`·`+`·`-`·`*`·`<<`·비트연산은 비트패턴 연산이라
**의도적으로 제외**했다.

**⚠️⚠️ 적대 soundness 렌즈 — BLOCKING 넷, 그리고 셋은 하나의 모양이다.**
렌즈의 요약이 정확했다: *"술어가 주장하는 성질(이 i64 는 폭 w 의 unsigned 비트패턴이다)을 주변 walk 가
정확히 `w == 64` 이고 leaf 도 w 폭일 때만 보장한다."*

1. ⭐⭐ **`>>>` 도 부호 민감인데 안 덮었다.** §11.4.10 상 `>>>` 는 **왼쪽 피연산자가 signed 일 때만**
   arithmetic 이고, §11.6.1 이 그 피연산자를 이미 식의 타입(여기선 unsigned)으로 변환한다. 두 오라클은
   **declared-signed 왼쪽 피연산자에서도** logical 을 답한다. 그리고 이건 놓친 수정이 아니라 **내가 만든
   회귀**다 — 비교 리다이렉트가 잠복 결함을 **드러내** 14칸이 correct→silent-wrong 이 됐다.
   ⇒ `matches!(op, B::Shr | B::AShr)`.
2. **`w >= 64` 는 규칙이 아니라 추측이었다.** >64 에선 i64 가 이미 절단했고 두 방향이 **서로 반대로**
   틀린다 — `(64'hFFFF…FFFF + 65'd1) > 64'hFFFF…FFFF` 는 signed 읽기가 맞고(오라클 1),
   `((65'd1-65'd2) > 65'd0)` 은 unsigned 읽기가 맞다(오라클 1). **어느 쪽도 지배하지 않는 것이
   추측의 정의다** ⇒ `w == 64`. ⭐ 형제 `const_unsigned_selfdet` 이 이미 같은 자리에 선을 그어 두었다.
3. **walk 가 ctx≥64 에서 `leaf_into_ctx` 를 안 돈다** ⇒ 좁은 **signed** leaf 가 부호확장된 채 도착하고
   u64 경로가 그 확장을 **크기로** 읽는다(`logic signed [7:0] P = -100` 이 156 이 아니라 0xFFFF…FF9C 로
   나눠졌다). 그 위에 세운 `$bits` indexed part-select 폭은 **loud → silent-wrong** 이었다.
   ⇒ 그 문맥에서도 leaf 를 정규화(= §11.6.1 재해석 그대로).
4. clippy `double_parens` 로 `-D warnings` 게이트 실패.

**측정.** 120칸 4-way(iverilog · verilator · PRE · POST) — **오라클 분열 0 · ok→wrong 0 · wrong→ok 14 ·
ok→ok 80 · wrong↔다른-wrong 0**. 33칸 census 의 비교 클래스 전부 해소. 폭 인식 walk 가 소유하는 자리에서
상수함수 본문의 `x % 64'd10` 은 **−1 → 5**, `x >> 32` 는 **전체 REJ(loud) → 정답**.
bench+examples **8설계 `.velab` 바이트 동일**. 새 회귀 10 테스트.

**적대 differential = CLEAN**(BLOCKING 0). 48칸 4-way + 47칸 byte-identity + 25칸 경계. ⭐ decline 을
조용한 기본값으로 먹는 소비자 일곱(범위 바운드·replication·part-select 폭·unpacked 크기·`$clog2`·
genfor 바운드·case 라벨)에서 **loud→정답 6칸 · silent-wrong→정답 17칸**, 그 반대는 **0**. byte-identity
47칸 **차이 0**(진단 문구·exit 코드 포함). 경계 25칸에서 **정확히 한 칸만 이동**(w=64 비교) ⇒ 규칙이
문서대로 정확히 64다. ⚠️ 곁수확 = **iverilog 자기모순** 확정(`64'hFF…FF % 64'd10`=5 인데 같은 값의
`(64'd0-64'd1) % 64'd10`=1) ⇒ divergence 목록에 추가.

**⚠️ 잔여**(§2): module-scope `localparam` 의 `/`·`%`·`>>`·`>>>` — **선언 폭이 있어도** 그렇다(적대 렌즈
실측: `localparam [63:0] P = …%…` PRE==POST) — 그 경로는 `const_eval_in_scope` 가
**문맥-결정 연산자를 자기가 폭-무제한 signed 로** 접는다(비교만 폭 인식 walk 로 redirect 된다) ⇒
선행조건은 §2 가 말하는 **AST self-폭 패스** · >64비트 decline · `*` 의 64비트 unsigned 오버플로 loud.

#### 4.5.365 — real actual 이 정수 formal 의 폭을 안 받는다 (2026-08-22 · 5,796 green · format 29 불변)

**결함.** IEEE 1800 §13.5.3 은 호출을 **formal 의 선언 타입 변수에 대한 대입**으로 정의한다 ⇒ real actual 은
반올림(§6.24.1, 0에서 멀어지는 쪽)한 뒤 **formal 폭으로 좁혀야** 한다. vita 는 반올림만 했다:
`function integer f(input byte k); f = k;` 를 `f(300.0)` 로 부르면 **300**(두 오라클 **44**), exit 0, 진단 0.

**⚠️⚠️ 착수 census 가 큐 문구의 절반을 반박했다.** 큐엔 *"real→정수 formal 이 IEEE-754 페이로드 · 56칸 ·
`%0d` 가 반올림으로 숨긴다 · 엔진 절반 ~40칸"* 이라고 적혀 있었는데, HEAD 에서 재보니 `%0d` 반올림
(`f(3.0)`→3 · `f(3.7)`→4)·**등가**(`(k==3)`→111)·`integer` formal·**automatic 함수/task** 는 전부 이미 정확했다
(이전 슬라이스들이 닫았다). 비트/파트 셀렉트·비트연산·`%h` 는 **조용히 틀리지 않고 E3009 로 거부**하므로
§2 가 아니라 **§3** 다. 남은 살아 있는 축은 하나 — **real→정수 경계**.

**⚠️ 그리고 첫 프로브가 나를 속였다.** `input int` formal 에 `3.0` 을 넣어 보고 *"frame 경로는 이미 맞다"* 고
적었다(그 값은 32비트에 **들어맞는다**). **폭보다 큰 값**을 넣자 frame 함수도 frame task 도 똑같이 틀렸다.
직전 슬라이스의 *"프로브의 해상도가 판정을 뒤집는다"* 와 같은 교훈의 **폭 축 버전**이다.

**⭐ 참조 구현이 이미 트리 안에 있었다.** inline TASK 는 입력 actual 을 **formal-폭 지역 net 에 copy-in** 하고
그 저장이 `value::coerce_assign` 을 탄다 — 그래서 60칸 전부 맞았다. 나머지 셋(inline 함수는 actual 의 ExprId 를
formal **이름**에 치환하고, frame 함수/ task 는 raw 값을 슬롯 in-bind 로 넘긴다)을 공유 헬퍼
`coerce_real_actual_to_formal` 하나로 거기 맞췄다. 184칸(10 formal 타입 × 6 real actual × 3 라우팅)
**mismatch 54 → 0**, 적대 렌즈의 1,170칸 스윕도 **1170/1170** iverilog 일치.

**⚠️⚠️ 적대 2렌즈 3라운드 — BLOCKING 넷.**

1. **`cast_operand_is_real` 이 철자로 판정한다.** 그 술어의 AST 반쪽은 **bare 단일 세그먼트** 이름을
   `func_table` 에서 찾는데, 패키지 본문의 bare `g()` 는 **`P::g`** 로 해석된다 — 그래서 모듈 레벨의
   real-반환 `g` 가 이름을 가리면 **정수** actual 이 real 이라고 판정되고, f64 왕복이 2⁵³ 초과를 잃었다
   (`…993` → `…992`, correct→silent-wrong). ⇒ 술어를 **값 기반 `expr_is_real`** 로 바꿨다.
   ⭐ 잃는 것이 없다는 논증이 구조적이다: `cast_operand_is_real` 만 보는 유일한 모양은 real-반환
   `Expr::Call` 이고 그건 `expr_is_repeatable` 이 이미 거절한다 ⇒ 캐스트에 도달한 적이 없다.
   (`classifier-must-match-its-lowering-resolver` 의 세 번째 사례.)
2. **frame-task 의 게이트가 형제보다 약했다.** net kind 로 물으면 `String`→`Wire`·`Event`→`Reg`·
   `ClassHandle`→`Integer` 가 통과한다 ⇒ `string` INPUT formal 이 **1비트** real→int 캐스트로 파괴됐다
   (`range_to_dims(String, None)` = 폭 1). ⇒ 게이트를 다른 두 자리와 **같은 철자**(`ast_kind_is_bit_vector`)로.
3. **`input time signed` 가 세 경로에서 correct→wrong.** 뿌리는 pre-existing — `kind_signedness` 가
   `time` 을 unsigned 로 하드코딩하고 명시 한정자를 버려 **formal net** 이 unsigned 로 만들어진다(그래서
   net 을 쓰는 static task 는 PRE 에서도 틀렸다). 좁히기 시작하자 나머지 셋도 그 net 을 따라갔다. ⇒
   **좁은 decline**(`time` + 명시 `signed` 만): 그 칸들은 PRE 와 완전히 동일해지고, 평범한 `time` 은
   loud(E3009) → **44** 로 올라간다. ⭐ 렌즈가 decline 집합이 **정확히 `{Time}`** 임을 집합 차로 증명했다
   (`kind_signedness` 가 덮는 다섯 중 `ast_kind_is_bit_vector` 가 막지 않는 유일한 것).
4. ⭐⭐ **`lower_real_to_int_cast` 의 ≤32비트 가지가 틀려 있었다.** `$rtoi(e ± 0.5)` 는 |e| 가
   [2⁵², 2⁵³) 인 **홀수 정수**에서 f64 ulp 가 1.0 이라 **tie** 이고 IEEE-754 는 그것을 **짝수로** 깬다 ⇒
   2⁵²+1 이 **2**(두 오라클 1). >32비트 가지는 이미 정확한 `te`/`frac`/±1 구성을 갖고 있었고 **자기 주석에
   그 함정을 적어 두었는데** 좁은 가지엔 없었다. `int'()`/`byte'()` 만 그 가지에 닿을 땐 눈에 안 띄었지만
   이 슬라이스가 **≤32비트 formal 전부**를 거기로 보내면서 **wrong↔wrong 맞바꿈**이 될 뻔했다 ⇒ 정확
   구성을 `real_round_half_away` 로 **hoist 해 두 가지가 공유**한다. ⭐ **곁수확**: pre-existing 캐스트 결함
   40칸이 함께 해소됐다(2⁵²+1→1 · 2⁵²+3→3 · −(2⁵²+1)→−1, 그리고 `int'(0.49999999999999994)` **1→0**).
   ⚠️ 반대 절반은 불변(3.5→4 · −3.5→−4 · 2.4→2) 이고 >32비트 가지는 **933바이트 문자 동일**(렌즈 검증).

**⚠️ 마지막 NIT 도 내 것이었다.** 부호를 얻으려 `range_to_dims` 를 **두 번** 불렀는데 그 함수는 **진단을
emit 한다** ⇒ `input logic [3:-2]` 에서 W3056 이 **두 번** 찍혔다. 첫 호출의 부호를 바인딩해 해소.

**측정.** 184칸 3-오라클 **FIXED 54 · REGRESSION 0** · 1,170칸 스윕 **0 회귀** · 진단/exit 스윕 **0 차이** ·
bench+examples **8설계 `.velab` 바이트 동일**(정수 actual 42설계 포함 50/50) · 새 회귀 11 테스트.
⚠️ **비용**: `int'(real)` 이 지배하는 루프가 **~2.5배** 느려진다(정확 반올림의 대가 · 64비트 경로는 불변) —
이 슬라이스의 기능과 무관한 자리라 기록해 둔다.

**잔여**(§2): 바인딩 자리 **아홉 중 다섯**이 남았다(output formal 을 가진 frame 함수 · 계층 task/fn ·
class 메서드/생성자) · `expr_is_repeatable` decline 이 남기는 조용한 기본값 · `time` 의 버려진 부호 한정자 ·
범위 밖 real 의 클램프 · `int'($random*1.0)` 의 draw 횟수 변화.

#### 4.5.364 — 구조적 지연의 값 fold 가 리터럴 전용이었다 (2026-08-22 · 5,785 green · format 29 불변)

**결함.** `parameter D = 7; assign #(D) y = a;` 가 t=1 에 전파한다 — 두 오라클은 t=8. exit 0, 진단 0.
`#(2+3)` 도 같으므로 *"파라미터 미지원"* 이 아니라 **리터럴 전용 fold** 다: `fold_ca_delay` → `const_delay_ticks`
→ `const_delay_u64` 의 `_` arm → `const_eval_u32`(IntLit/Paren/unary ± 뒤 `_ => None`), 그리고 **호출자가
`None` 을 조용한 기본값(딜레이 없음)으로 소비**한다. 지연 자체와 inertial 취소는 이미 정확했다 — 깨진 건 **값**.

**census 가 하나를 셋으로 갈랐다.** 큐엔 *"파라미터"* 한 줄이었는데 70칸 3-오라클 census 는 레인이 셋임을 보였다.

- **정수 — 자기결정 폭 · unsigned 읽기.** 오라클이 규칙을 정했다: `#(4'd15 + 4'd1)` → **0**(4비트 합이 wrap),
  `parameter signed [7:0] D = -8'sd1` → **255**(8비트 패턴을 무부호로), `parameter [3:0] W=15; #(W+1)` → **16**
  (unsized 형제가 32비트). 폭-무제한으로 접으면 각각 16 · 4294967295 가 된다. 이 자리는 §20.8.1 의 `$clog2`
  인자와 **같은 철자**이므로 그 헬퍼를 `const_unsigned_selfdet` 로 **순수 추출해 공유**했다(clog2 동작 불변).
- **real** — `parameter real RD = 2.5`·`#(RD/2.0)`·`#(2.5+1.0)`. 두 단계 반올림도 리터럴 가지에서
  `real_delay_ticks` 로 추출해 **한 철자**로 공유(`#2.5` 와 `#(RD)` 가 `10ns/1ns` 에서 갈리지 않게).
- **TimeLit** — `#(5ns)`. `const_eval_in_scope` 의 arm 은 **모듈 단위**로 답하고 단위의 정수배가 아니면
  declines 하므로(`10ns/1ns` 안의 `#(5ns)` = 반 단위) 그 decline 이 다시 조용한 no-delay 였다 ⇒ 이 레인은
  **전역 tick 으로 직접** 스케일한다. 정수배 칸은 값이 그대로다(그 답 × `cur_time_mult` 가 곧 이 곱).

**한 퍼널.** 파서가 게이트 프리미티브와 net-decl 지연을 전부 `ContinuousAssign` 으로 desugar 하므로
assign / `wire #(D) w = e` / buf·and·or·xor·bufif1 / rise-fall-turnoff 사이드카가 같은 fold 를 지난다.

**opt-in 경계.** 절차적 `#delay`(`lower_delay`)는 **넓히지 않았다** — 그 레인은 amount 를 런타임 식으로
낮추므로 애초에 리터럴 제한이 없었고, region 판정만 넓히면 `#(ZERO_PARAM)` 이 Active→Inactive 로 바뀐다
(결함 없는 스케줄링 변경). `expr_mentions_real` 의 `shadow_correct` 도 같은 형태의 옵트인이다.

**⚠️ zero-rise 는 pre-slice 모양을 유지한다(거래이지 누락이 아니다).** `Some(0)` 은 CA 를 delayed 레인으로
보내고 vita 의 zero-tick write 는 **자기 타임스텝의 Postponed 리전 뒤에** 착지한다(`$strobe` 실측: 두 오라클
`y=1`, vita `y=0`). 그래서 스코프-fold 된 rise 0 은 `None` 으로 억제한다 — 안 그러면 맞던 답을 그 lag 와
맞바꾼다. 대가는 `#(ZERO_PARAM, F)` 의 fall 이 버려지는 것이고(리터럴 `#(0,F)` 는 맞다), **양쪽 다 2-오라클
합치**라 어느 쪽으로 가든 silent↔silent 맞바꿈이다 ⇒ 뿌리(zero-tick lag)를 §2 에 적고 둘 다 핀으로 고정.

**⚠️⚠️ 적대 2렌즈 4라운드 — BLOCKING 넷 중 셋은 내 라운드-2 수정이 만들었다.**

1. **정수 도메인을 먼저 물었다.** 정확히 정수인 `parameter real R = 11` 은 `real_param_val` 과 `params`
   **양쪽**에 등록되므로 정수 walk 가 그 i64 쌍둥이를 찾아 `#(R/2)` 를 **정수 나눗셈 5** 로 접었다(두 오라클
   6 · vita 자신의 절차적 `#(R/2)` 도 6). ⭐ **코드베이스가 이미 규칙을 적어 두었다** — `param_real_value` 의
   주석이 *"초기화자가 real 을 언급하는 순간 real fold 가 정수보다 **먼저** 돌아야 한다"* 이고 그 이유로 바로
   이 절단을 든다. 내가 근거로 삼은 `const_truth_in_scope` 의 순서는 **진리값 소비자**의 것이라 절단이
   보이지 않는다. **지연은 크기(magnitude)** 다 ⇒ 순서를 바꿨다.
2. **그런데 real-먼저를 `return` 으로 썼다.** real 도메인엔 `%`·비트·shift·`$clog2`·call arm 이 없어서
   `#(RD % 4)`·`#($clog2(RD))`·`#(half(RD))`·`#(RD & 7)`·`#(RD << 1)` 다섯 칸이 **correct→silent-wrong**
   이 됐다(라운드-1 빌드에선 맞던 칸들) ⇒ **fallback 필수**. ⭐ 거래가 아니라는 증명이 구조적이다: i64
   쌍둥이는 real 이 **정확히 정수**일 때만 존재하므로 두 레인은 **값**에서 갈릴 수 없고 **연산자**에서만
   갈리는데, real 이 없는 연산자는 전부 정수 전용이다.
3. **게이트의 resolver 가 틀렸다.** `expr_mentions_real` 의 이름 arm 은 `real_param_val` **단독 walk** 라
   안쪽 `localparam R = 9;` 가 바깥 `parameter real R = 5;` 를 가려도 real 이라 답하고, real 레인이 **바깥
   5** 를 접었다(두 오라클 9). 이 함정도 저장소가 이미 적어 두었다 — `real_param_is_non_integral` 의 주석이
   *"`real_param_val` 단독 walk 는 안쪽 net/숫자 param 이 가려도 바깥 real 을 잡아 한 이름을 두 가지로
   해석한다"*. ⇒ `expr_mentions_real_opt(e, shadow_correct)` **옵트인** 신설(결합 바인딩 walk =
   `real_param_lowers_real`), 기존 호출자 셋은 **리터럴 `false`**. soundness 렌즈가 새 resolver 가 **엄격한
   좁힘**임을 코드로 증명했다(결합 술어가 상위집합이라 walk 는 같거나 더 안쪽에서 멈춘다 ⇒ 차이는
   `true→false` 방향뿐 ⇒ real 레인으로 **새로 들어가는 것은 없다**). 곁수확으로 POST2 의 shadow 결함 둘이
   더 닫혔다(genvar 루프 · 중첩 generate).
4. **새 TimeLit 레인이 오버플로에 decline 했다.** 형제 레인들은 전부 saturate 하는데 이것만 `?` 로 빠져
   `#(20000s)`(`1ns/1fs` 에서 `10^15 × 20000` > u64)가 **즉시 발화**했다 — 버려진 지연은 clamp 된 지연보다
   **더 이르다** ⇒ saturate. **두 렌즈가 독립적으로 같은 발견**을 냈다.

**⚠️ 렌즈의 BLOCKING 하나는 반박했다 — 프로브의 격자 해상도가 판정을 뒤집는다.** soundness 렌즈가 지연
멀티드라이버의 `E3001` 을 *"PRE 는 두 오라클과 바이트 동일했다 = correct→loud 회귀"* 로 냈다. 같은 설계를
**1 ns 격자**로 다시 재니 PRE 는 `t=11 bus=1`, iverilog 는 `bus=z` — PRE 는 **지연을 조용히 버리고 있었고**
10 ns 격자가 그것을 못 본 것이다 ⇒ silent-wrong → honest-loud = **사다리 상승**. 렌즈도 세 가지 구성으로
반례를 시도한 뒤 *"지연 드라이버는 `[0,D)` 동안 x 를 쥐는데 PRE 엔 그 구간이 아예 없다"* 는 구조적 이유로
철회하고 §3 행으로 재분류했다.

**측정.** 70칸 3-오라클(iverilog 13.0 · verilator 5.050 · vita) PRE/POST 전이 = **FIXED 51 · REGRESSION 0 ·
SAME-OK 15 · DIVERGENT-ORACLE 3 · STILL-WRONG 1**(런타임 변수 지연 = 범위 밖). 라운드마다 353설계 드리프트
스캔으로 **수정 표적만** 움직였음을 확인(라운드 3 = 8칸, 전부 라운드-2 결함의 repro). bench+examples
**8설계 `.velab` 바이트 동일**. 진단·exit 코드 스윕 전 라운드 **0 차이**. 새 회귀 테스트 15건.

**잔여**(전부 PRE==POST · 각각 다른 경로 ⇒ CLASS 분리 규칙으로 §2 에 기록): 런타임 변수 지연 ·
사이즈드 음수 **리터럴**이 새 규칙에 도달 못 함(`#(-4'd1)` 은 never / 파라미터 쌍둥이는 15 = 두 오라클) ·
zero-rise 사이드카 거래 · TimeLit 이 식의 루트가 아닐 때. 곁가지 = 지연 CA 의 **인덱스 안 함수 호출** 패닉.

#### 4.5.363 — 파라미터 셀렉트가 상수 도메인에 아예 없었다 (2026-08-22 · 5,770 green · format 29 불변)

**증상.** `localparam logic [31:0] W = 32'h0000_0034; logic [W[7:0]-1:0] v;` 가 `$bits(v)=1` · exit 0 · 진단
없음. iverilog 13.0 과 verilator 5.050 은 둘 다 **52**.

**⭐ 값도 폭도 이미 있었다.** 같은 텍스트의 런타임 레인(`$display("%0d", W[7:0])`)은 세 툴 다 52 이고, 선언
범위는 `param_range` 에 이미 기록돼 있었다(런타임 하강의 `norm_offset_if_net` 이 그것으로 오프셋을 정규화한다).
없던 것은 `const_eval_in_scope` 의 **arm 하나** — 그 함수의 유일한 `BitSelect` arm 은 const-**배열원소** 조회
(`ROT[i]`)라 스칼라 param 에서 declines 하고, `PartSelect`/`IndexedPart` arm 은 catch-all 앞에 아예 없다.
그래서 폭·차원·반복수·`$bits`·루프 bound 가 전부 `clamp_bound_u32(None)` = 0 = **폭 1** 로 떨어졌다.

**⚠️ 캐스트 뭉치의 선행조건에 막히지 않는다.** §2 가 *"트리 전역 AST self-폭 패스가 먼저"* 라고 잠가 둔 네
항목과 달리, 셀렉트의 self-width 는 `|msb-lsb|+1` 로 **AST 상수에서** 나온다 — 하강이 필요 없다.

**⭐⭐ 슬라이스의 본체는 두 개의 decline 규칙이다.**

ⓐ **방향과 선언 LSB.** 비트를 뽑으려면 폭이 아니라 **선언 범위**가 필요하다. `[39:8]` 의 `B[15:8]` 도
`[0:31]` 의 `A[24:31]` 도 두 오라클이 **52** 인데, `[w-1:0]` 을 가정하면 둘 다 0 을 읽는다 — silent-wrong 을
**다른** silent-wrong 과 맞바꾸는 금지된 거래다. 그래서 런타임 하강이 쓰는 **바로 그 `param_sel_range`** 를
쓴다(두 도메인이 갈릴 수 없다). ⭐ **곁수확**: 이를 위해 `param_decl_range` 에 오름차순 arm 이 생겼고
(`lo == 0` 이 `[0:31]` 을 `[31:0]` 과 함께 삼키고 있었다) 그것이 **런타임** 셀렉트까지 고쳤다 —
`localparam logic [0:31] A = 32'h34; A[26]` 이 0 → **1**(두 오라클 1). §2 에 불릿조차 없던 silent-wrong 이다.

ⓑ **폭 provenance.** 무타입 파라미터의 폭은 **값에서 추론**될 수 있다(`param_decl_width` 의 마지막 arm =
`min_signed_bits(v).max(32)`). `localparam W = ~8'hCB;` 는 실폭 8 인데 32 로 기록되고, 그 폭을 믿고 비트를
뽑으면 **없는 비트를 지어낸다** — `logic [(W[15:8])+8-1:0] v;` 가 **263비트** net(iverilog 1, 그리고 **PRE 도
1** = correct→silent-wrong). ⇒ `param_decl_width_opt(p, declared_only)` opt-in 을 만들어 **선언 범위 · 타입 ·
리터럴** 유래만 채택하고, `param_decl_range` 가 그 provenance 의 유일한 기록처가 된다(기존 호출부는 전부
리터럴 `false` = 바이트 동일).

**⚠️⚠️ 적대 3렌즈(differential · soundness · ladder/residue)가 다섯을 잡았고 둘이 BLOCKING 이었다.**
첫 설계는 wrap 가드(*"셀렉트는 좁은 leaf 라 그 위 산술이 폭-무제한 도메인에서 안 감긴다"*)를 **공유 평가기**
`const_eval_in_scope` 의 Binary arm 에 넣었다. 그 함수는 **폭 규칙이 정반대인 소비자 둘**이 공유한다:

1. **파라미터 값은 자기결정이 아니라 대입이다**(§11.6) — `localparam int Q = W[7:0] + 8'd240;` 가 **36**
   (두 오라클 292 · PRE 는 honest-loud) = **loud → silent-wrong**.
2. **가드가 한 arm 에만 닿았다** — `~W[3:0]` 은 Unary arm 으로 빠져 **6**(두 오라클 12 · PRE 1) = silent-wrong
   을 **다른** silent-wrong 과 맞바꿈. 정확히 그 가드가 막으려던 것.
3. **셀렉트가 없는 식이 121배 느려졌다** — 8000항 상수식에서 Θ(n²)(모든 Binary 노드마다 부분트리를 통째로 걷고
   `Vec` 를 새로 할당).

⇒ **규칙을 소비자로 옮겼다**: 선언 range bound 는 **정의상 자기결정**(§11.6.1)이므로 `array_geom` 의 bound
fold 한 자리(`const_range_bound_fold`)에 두면 셋이 한꺼번에 사라진다 — 정확하고, bound 당 O(1) 이고, 셀렉트
없는 bound 는 **같은 호출 그대로**다. 나머지 상수-bound 소비자는 이미 계층화된 `const_bound_u32` 퍼널을 지나고
그 퍼널의 `ast_const_leaves_min32` 가 셀렉트에 대해 **fail-closed** 다(같은 이유).

**그 밖에 리뷰가 잡은 셋.** ⓒ 셀렉트 **자신의** 인덱스도 자기결정이다(Table 11-21) — `W[4'd15+4'd1]` 은 4비트
합이 감겨 **bit 0** 인데 무제한 fold 가 bit 16 을 읽어 bound 가 10비트(두 오라클 9)였고, **vita 자신의 런타임이
같은 실행에서 0 을 답하고 있었다**. ⓓ `[5:5]` 동일 끝점 — 방향표가 하강 base 에 `a <= b` 를 물어 `a == b` 를
거절했다(오름차순 쌍둥이는 접혔다 = 한 표에서 갈린 두 답). ⓔ 음수 선언 bound 의 `max(0)` — `param_range` 가
음수 `lo` 를 담을 수 없어 **거짓말**을 적고 있었는데, `lo == 0` 조기 반환이 그걸 걸러 주는 동안엔 무해했다.
오름차순 arm 이 그 반환을 없애자 `[-2:3]` 이 `(0,6,true)` 로 기록돼 `A[0]`/`A[3]` 을 correct(0,0) 에서
wrong(1,1) 로 바꿨다 ⇒ **음수 bound 는 decline**.

**측정.** 93칸 3-오라클 그리드(PRE 바이너리는 `git archive HEAD` 로 별도 트리 빌드):
**AGREE3 29 → 81 · VITA_WRONG_2ORACLE 46 → 5 · VITA_LOUD 17 → 6 · FIXED 52 · 회귀 0 · 컨트롤 28/28 불변.**
bench(keccak 3 · picorv32) + examples 4 의 `.vu`/`.velab` **17개 전부 바이트 동일**. 8000항 상수식 PRE 0.058s /
POST 0.057s. 전 스위트 5,753 → **5,770 green** · clippy 0 · fmt 0 · format_version **29 불변**.

**곁수확 둘.** ⭐ `wide_param_name_in` 이 셀렉트-맹인 `collect_bare_idents` 를 쓰고 있어 >64비트 파라미터
셀렉트가 *"undefined name `K`"* 라는 **사실과 다른** 진단을 냈다(같은 화면의 다른 진단은 *"필요한 비트를
선택하라"* 고 안내한다) — const 도메인 자신의 walk 로 바꿔 진실을 말하게 했다. ⚠️ `collect_bare_idents` 자체는
**건드리지 않았다**: implicit-net 패스가 공유하는데 거기선 arm 하나를 더하면 **조용한 implicit net** 이 생길 수
있어, 진단 하나 고치자고 무관한 경로의 사다리를 거래하는 셈이 된다. ⭐ `count_reads_real_param` 도 같은 모양
이어서 `logic [R[7:0]-1:0]`(R 은 real)이 *"undefined name"* 을 냈다 — 셀렉트 arm 셋을 더해 쌍둥이
`logic [R-1:0]` 과 같은 진실을 말하게 했다(loud→loud, 소비자 하나뿐이라 공유 위험 없음).

**기존 테스트 하나를 바꿨다.** `round4_report_gaps::gap_g_local_scalar_shadowing_import_*` 은 **LOUD 를 단언**
하고 있었는데, 그 loud 는 능력 갭을 의도로 고정한 것이었다(주석이 그렇게 적고 있다 — *"const 도메인이 스칼라
비트 셀렉트를 접지 못하므로 fold 와 런타임이 갈리지 않게 둘 다 거절"*). 이제 둘이 **일치**하고(99 의 bit 1 = 1)
세 툴이 그 값에 동의하므로, 테스트를 **더 강한 성질**(imported 원소 20 이 아니라 지역 99 의 bit 를 읽는다)로
다시 썼다.

**남긴 잔여**(전부 사다리 이동 없음 · ROADMAP §2): package 스코프 base **넷이 아니라 하나**(같은 파일에서
bare 는 52, `pk::` 는 1 — 가장 급하다) · 셀렉트들의 concat(`fold_self_bits` 의 leaf resolver 가 `HierPath`
만 받는다) · 중첩 셀렉트 · 파라미터에서 파생된 파라미터 · >64비트 base(loud).

#### 4.5.362 — real 지수는 밑수의 문맥을 회수한다 (2026-08-22 · 5,753 green · format 29 불변)

**착수 지점, 그리고 그것이 틀렸다는 것.** §2-1f 는 한 시간 전 §4.5.361 이 직접 적어 넣은 행이고,
거기 적힌 진단은 *"iverilog 가 `(4'd15+4'd1) ** r` 을 1024 로 읽으니 대입 문맥이 `**` 의 밑수에
닿는다 ⇒ vita 의 결함은 **non-fill 경로가 덜 주는 것**"* 이었다. 착수 census 가 **첫 여덟 칸에서**
그것을 반박했다.

**⭐ 이웃 연산자가 규칙을 이미 갖고 있었다.** 같은 네 밑수를 `+ r` 에 물리면:

| 밑수 | `+ r` (iv=vl=vita) | `** r` (iv=vita) |
|---|---|---|
| `'1` | **4** (밑수 = 1) | 480 |
| `('1+4'h0)` | **18** (밑수 = 15, 4비트 형제) | 480 |
| `('1\|16'd0)` | **2** (밑수 = 65535, 16비트 형제) | 480 |
| `(c?'1:4'h2)` | **18** (밑수 = 15, 삼항 형제) | 480 |

`+`·`-`·`*`·`/` 는 **96/96 칸에서 iverilog 와 vita 가 일치**하고 형제 폭을 정확히 반영한다. 그런데
`**` 만 네 밑수를 **한 값(480)** 으로 뭉갠다 — 즉 형제를 무시하고 **대입 폭 16** 을 쓴다. 한 밑수
표현식의 값이 그것을 소비하는 연산자에 따라 달라지는 것이다.

**⭐⭐ 그리고 실격 질문은 한 개였다 — 대입 폭이 없는 곳에 같은 식을 보내라.**

| 같은 식 | iverilog | verilator | vita |
|---|---|---|---|
| `logic [15:0] a = ('1+4'h0) ** r` | 480 | 1 | 480 |
| **`real x = ('1+4'h0) ** r`** | **871.4213** | 1 | **871.4213** |
| **`$pow(('1+4'h0), r)`** | **871.4213** | **871.4213** | **871.4213** |

§11.4.9 는 real 지수의 `**` 를 **`$pow` 로 정의한다**. 그 철자에서 세 시뮬레이터가 전부 871.4213 을
내고, `real` 목적지에서도 iverilog 자신이 871.4213 을 낸다 ⇒ **iverilog 는 그 밑수를 15 라고 답한다.**
16비트 목적지에서만 65535 가 되는 것은 **목적지가 밑수로 역류**한 것이고, 밑수의 값이 결과를 나중에
담을 변수의 폭에 의존할 수는 없다.

⚠️⚠️ **그래서 §4.5.361 의 채점이 틀렸다.** 그 슬라이스는 후보 고침을 *iverilog 일치도* 로 재서
267 → 247 을 보고 기각했는데, **그 점수가 재고 있던 것이 바로 누수였다**. 규칙 파일에 *"어느 툴이
여기서 오라클이 아닌가를 물어라"* 를 적어 넣고, 같은 세션에서 그 규칙을 적용하지 않은 채 옆
연산자를 확인하지 않았다.

**게이트를 다시 세웠다 — 연산자형 vs `$pow` 형 동치.** 오라클을 툴이 아니라 **같은 툴의 다른 철자**로
잡으면 채점이 누수에 오염되지 않는다. 192쌍(12 밑수 × 4 real 지수 × 4 타깃 폭):

| | 연산자형 == `$pow` 형 |
|---|---|
| PRE | **115 / 192** |
| POST | **192 / 192** |

옮겨간 **77칸이 전부 기준값에 정확히 착지**했다(근처가 아니라 동일).

**⚠️ 반대 절반은 핀으로 지켰다.** Table 11-21 은 `**` 의 밑수를 **문맥-결정**으로 둔다 — 그건
정수 지수에서 여전히 참이고, 거기서는 fill 이 4비트 형제가 아니라 **대입 폭**을 가져간다
(`('1+4'h0) ** 3` = 65535, 세 툴 중 둘 일치). 그래서 고침은 **지수가 real 일 때만** 발동하고,
**순수-정수 `**` 384칸이 바이트 동일**이다. 두 읽기가 한 테스트 파일에 나란히 들어가 있다.

**⚠️ 적대 리뷰: 이웃의 가드를 그대로 베꼈다.** 첫 판에서 조건을 `Pow && fill(lhs) && !fill(rhs)` 로
썼다 — 바로 아래 arith 팔의 `lf && !rf` 를 복사한 것이다. 그 팔의 `!rf` 는 *"어느 쪽이 폭을
공급하는가"* 를 묻기 때문에 존재하고, 여기서 묻는 것은 *"지수의 도메인"* 이라 **조건이 이월되지
않는다**: fill 자체는 real 이 될 수 없어도 **fill 을 품은 식은 real 이 된다**(`r + '1`). 그래서
`('1+4'h0) ** (r + '1)` 이 64976 으로 남아 있었고(`$pow` 는 13071), 가드를 `Pow && fill(lhs)` 로
좁히자 사라졌다. *— 인접한 가드를 복사할 때는 그 가드가 답하는 질문까지 같은지 확인한다.*

**⚠️ 재배치는 관찰 불가능하다(soundness).** 밑수를 지수 뒤에 낮추므로 피연산자에 부작용이 있으면
순서가 뒤집힌다. 실측: `automatic` 함수가 모듈 net 에 쓰면 vita 는 **E3009 로 거부**하고, `static`
철자는 그 쓰기를 **떨어뜨린다**(PRE·POST 동일) ⇒ 이 슬라이스가 만드는 차이가 없다. 게다가 arith
팔이 이미 같은 재배치를 하고 있다.

**곁수확 = static function 의 모듈 net 쓰기가 조용히 사라진다**(§2-6 신규 · 2-오라클):
`function logic [3:0] f(); seq = seq + 7; f = 4'h3; endfunction` 에서 `a = f();` 는 반환값 `3` 은
맞는데 `seq` 가 **0** 이다(iverilog·verilator 둘 다 7). ⭐ `automatic` 철자는 같은 본문을 **정직하게
거부**하므로 사다리상 **static 철자만 한 칸 아래**다. 발견 경로 자체가 기록할 만하다 — 이 슬라이스의
재배치가 관찰 가능한지 묻는 프로브를 짜다가 나왔고, PRE·POST 동일이라 이 슬라이스 것이 아니다.

**뮤테이션 7발 = 3 KILLED / 4 등가.** 죽은 셋이 정확히 의미를 바꾸는 셋이다 — `base_ctx = ctx`
(고침 취소) · `base_ctx = 0`(정수 지수 절반 파괴) · 조건 반전. ⚠️ **m3 은 예측이 틀렸다**: 게이트를
`Pow | Shl` 로 넓히면 죽을 줄 알았는데 살았다. 사후 설명을 믿지 않고 **재서 확인했다** — real
피연산자 shift 는 네 철자 전부 E3009 로 거부되므로 그 팔은 *받아들여지는 설계에서 도달 불가*이고,
정수 shift 량이면 `base_ctx = ctx` 라 폭이 같다. ⭐ 그 생존이 곧 **`Pow` 조건이 정확성이 아니라
arena 순서의 바이트 동일성만 지킨다**는 증명이다.

**게이트**: 5,753 green · 192쌍 `$pow` 동치 100% · 순수-정수 384칸 바이트 동일 · bench·examples
8설계 stdout+stderr+VCD 바이트 동일 · clippy 0 · fmt 0 · format 29 불변.

#### 4.5.361 — 분열 넷을 판정했고 셋은 no-op, 넷째는 지어서 되돌렸다 (2026-08-22 · 5,748 green · format 29 불변 · 제품 코드 변경 0)

**착수 지점.** §2 큐의 네 행(§2-1d ⓐⓑⓒ · §2-2b)이 전부 같은 문장으로 막혀 있었다 — *"2-오라클
합의가 없으므로 착수 불가, 먼저 필요한 건 구현이 아니라 hand-IEEE 판정"*. 그 문장은 **판정을 미룬 게
아니라 판정의 형태를 잘못 잡고 있었다**: 넷 다 *"어느 답이 맞나"* 로 물으면 답이 안 나오고,
*"어느 오라클이 이 자리에서 오라클이 아닌가"* 로 물으면 **셋이 즉시 닫힌다**.

**⭐⭐ 오라클 실격은 vita 를 보지 않고 증명된다.** 이게 이 슬라이스의 방법론적 수확이다. 분열을
푼다고 하면 보통 세 번째 오라클을 찾거나 LRM 을 읽는데, 두 자리는 **한 툴의 답들끼리 모순**이라
그럴 필요가 없었다.

| 자리 | 그 툴이 스스로 낸 모순 | 결론 |
|---|---|---|
| string 관계연산 | `s="ab"` 에서 `s<"ab"`·`s<"aa"`·`s<"zz"` 를 **전부 1** — 폭·부호·변환이 하나도 없는 string↔string 비교인데 셋이 동시에 참일 수 없다 | iverilog 의 `1` 은 **정보가 없다**. vita=verilator, 전 칸 일치 ⇒ no-op |
| `$itor` | `$itor(64'h1_0000_0008)` 이 `longint unsigned` 에서도 signed `longint` 에서도 **똑같이 8** — 부호 해석이라면 갈려야 한다 | 부호가 아니라 **32비트 컨테이너 절단**. 채택하면 vita 가 맞는 **2³¹ 이상 전 구간**이 correct-support → silent-wrong ⇒ 사다리 금지 ⇒ no-op |

**ⓒ `%` on real — 관대함의 서명.** 두 툴이 실행한다는 사실만 보면 vita 의 E3009 가 갭처럼 보인다.
그런데 **두 툴의 답이 서로 다르다**(iverilog 는 fmod 로 1.5, verilator 는 피연산자를 반올림해 0).
정의된 의미가 있다면 두 구현이 같은 답을 낸다 — 다른 답은 **미정의 구석의 서명**이다. 게다가
iverilog 는 한 연산자 옆(`r << 1`)에서는 같은 Table 을 **집행한다**. 불법 코드에 대한 loud reject 는
사다리의 최상단이므로 이건 §3 갭이 아니다.

**⚠️⚠️ ⓑ `'1 ** r` — 지어서, 재서, 되돌렸다.** 이 항목만 판정이 "고칠 것 있음" 으로 나왔고,
근거는 튼튼해 보였다: vita 자신이 **같은 값을 두 철자에서 다르게** 읽는다.

```
logic [15:0] a; real r = 2.5;
a = '1 ** r;              // vita 480   (iverilog 480, verilator 1)
a = (4'd15+4'd1) ** r;    // vita 0     ← 같은 all-ones 밑수인데 다른 답
```

내부 불일치는 **진짜**다. 그래서 `expr_ctx.rs` 의 `Pow` 팔에서 지수가 real 이면 밑수의 ctx 를 0 으로
떨어뜨리도록 고쳤다(빌드 통과). 그리고 착수 전에 세운 규칙대로 288칸(12 밑수 × 6 지수 × 4 타깃)
PRE-3-way 를 돌렸다.

| | iverilog 일치 | 불일치 |
|---|---|---|
| PRE | **267** | 21 |
| POST | **247** | 41 |

**옮겨간 35칸이 전부 iverilog 와 일치하던 것에서 불일치로 갔다.** 이 프로젝트의 규칙이
*"soundness 와 differential 이 충돌하면 differential 이 이긴다"* 이고, 여기서 내부-일관성 논증은
soundness 축이다. 되돌렸다(되돌린 뒤 267 로 복귀 확인).

**⭐ 그리고 측정이 진단을 뒤집었다.** 되돌린 김에 반대편을 물었다:

```
(4'd15+4'd1) ** r  →  iverilog 1024   vita 0
```

1024 = 16^2.5 다. 즉 iverilog 는 밑수를 **대입 문맥 16비트**로 재서 16 을 얻는다 — `**` 의 밑수에
문맥이 **닿는다**. vita 는 자기 폭 4비트로 재서 0 을 얻는다. 그러면 vita 의 불일치는 **fill 경로가
문맥을 더 줘서가 아니라 non-fill 경로가 덜 줘서**이고, 480 은 고칠 대상이 아니라 **이미 맞는 쪽**이다.
§4.5.351/352/353 과 **같은 모양**(머신러리는 있고 한 경로가 안 부른다)이라 §2-1f 로 신규 등록했다 —
다만 선행조건이 무겁다: plain `lower_expr` 에는 ctx 인자가 아예 없다.

**남긴 것 = 테스트 파일 하나, 제품 코드 변경 0.** `crates/cli/tests/oracle_split_rulings.rs`(4 테스트).
이 파일의 목적은 기능이 아니라 **방어**다 — 이 저장소의 다른 모든 differential 테스트는 *두 툴이
합의했고 vita 가 달랐다* 라서 존재하는데, 이 넷은 정반대라서 **미래의 스윕이 "한 툴과 다르네" 로
'고쳐서' silent-wrong 을 만드는** 경로가 열려 있다. 특히:

- `a_power_with_a_real_exponent_keeps_the_assignment_context` 는 **되돌린 그 값(480)** 을 핀하고,
  같은 테스트가 `(4'd15+4'd1) ** r` 의 **0** 도 핀한다 ⇒ §2-1f 를 닫는 사람에게 두 칸이 한 쌍임을 알린다.
- `itor_keeps_the_value_instead_of_truncating_to_thirty_two_bits` 의 **anti-절단 핀**(unary minus 가
  없고 signed 인 두 칸)은 "iverilog 에 맞추자" 는 편집이 **가장 먼저** 걸리는 자리다.

**곁수확 둘**(둘 다 §2 큐 신규):

- **`$itor(<real>)`** — `$itor(3.9)` 가 vita 4.61596e+18(= 3.9 의 double 비트패턴) vs iverilog 4.
  ⭐ **같은 값의 `real'(3.9)` 은 vita 도 3.9 로 맞다** ⇒ 캐스트 경로는 맞고 `$itor` arm 만 인자를
  integral 로 가정한다. ⚠️ 착수 시 §2-2b 가 기록한 봉인 확인 필수 — `real'(x)` 가 **같은
  `SysFuncId::Itor`** 로 내려가므로 arm 을 나누면 두 경로가 갈린다.
- **`string'(<integral>)`** — vita 가 **파싱조차 못 한다**(E2002). ⚠️ 원래 보고는 *"NUL 스트리핑
  차이"* 였는데 **그 repro 가 vita 에서 성립하지 않는다** ⇒ NUL 축은 다른 repro 로 재측정해야 열린다.
  (보고된 증상을 확인하려다 그 아래의 다른 갭을 찾은 형태.)

**게이트**: 5,748 green · clippy 3 구성 0 · fmt 0 · format 29 불변 · MsgCode 68 불변.

#### 4.5.360 — 선행조건이 "스코프 스냅샷"이 아니라 문자열 하나였다 (2026-08-22 · 5,744 green · format 29 불변)

**착수.** §4.5.357 은 §2-1e 의 잔여 4칸(`u.a = c ? '1 : 1'b0`)을 남기며 선행조건을 *"resolve 시점에 쓸 수 있는 lowering 스코프 스냅샷"* 이라고 적었다. 그건 §4.5.357 자신이 배운 교훈 — *"제약을 긍정형으로 다시 진술하면 대개 넓어진다"* — 을 **한 번 더 적용하지 않은** 자리였다.

**⭐ 재보니 스코프가 문자열이었다.** vita 는 이름을 `walk_scopes_key` 로 해석한다 — `cur_prefix` 에서 바깥으로 걸으며 **FQ-keyed `BTreeMap`**(`symbols`/`params`)을 본다. 그 테이블은 elaboration 내내 살아 있고 resolve 패스에서도 그대로 있다. ⇒ 평범한 이름 하나에 필요한 "스코프" 는 **`cur_prefix` 문자열 하나**다. pending 레코드에 담아 두었다가 재-lower 전후로 갈아끼우면 된다.

술어(`scope_free`)에 **단일 세그먼트 `Ident`** 를 더했다. 여전히 배제하는 것:
- **점 있는 경로** — 평범한 바깥 walk 가 아니다;
- **push/pop 되는 상태로 해석되는 이름** — 인라인 함수 formal 의 `subst`, frame window. 그것들은 resolve 시점에 **없고**, 없는 채로 재-lower 하면 이름이 다른 곳으로 해석된다.

**⚠️⚠️ 그리고 재-lower 는 읽기 전용 검증 뒤에만 한다.** `lower_expr_ctx` 는 미해결 이름을 **진단을 emit 해서** 알린다 — 즉 실패한 재-lower 는 **되돌릴 수 없다**(오류는 이미 sink 로 나갔다). 값이 틀렸던 설계가 **거짓 loud** 가 되는 건, 값이 틀렸다는 사실과 무관하게 회귀다. 그래서 `every_name_resolves` 가 먼저 확인하고, 하나라도 못 찾으면 `lower_expr` 의 IR 을 그대로 둔다. ⭐ 그 확인은 **하강이 쓰는 바로 그 walk**(`walk_scopes_key` over `symbols`/`params`)를 부른다 — §4.5.276 이 세 번 물린 *"분류기는 하강이 쓰는 리졸버를 써야 한다"* 그대로다.

**결과.** 70칸 census **residue 4 → 0 · bad-change 0**. 520칸 계층 스윕은 **440 agree · MISMATCH 0** 으로 불변. ⭐ 곁수확: task 본문 안에서의 계층 쓰기(`task automatic g(logic q); u.a = q ? '1 : 1'b0;`)도 맞는다 — task formal 은 모듈 net 으로 hoist 되므로 같은 walk 가 찾는다.

**⚠️ 가드를 값이 아니라 성질로 핀했다.** 검증이 무엇을 결정하든 **설계는 여전히 돌아야 한다**는 것이 가드가 지키는 성질이고, 테스트는 그것을 단언한다. 가드를 실제로 발화시키면서 컴파일도 되는 설계를 만들지 못했고, 만든 척하지 않았다(함수 본문의 계층 쓰기는 fill 과 무관하게 pre-existing E3009 라 이 경로에 닿지 않는다).

**게이트.** 5,743 → **5,744 green** · clippy 0 · fmt 0 · format 29 불변.

#### 4.5.359 — 음수 bound 의 남은 선언 스코프: 여섯 복사가 아니라 한 함수 (2026-08-22 · 5,743 green · format 29 불변)

**착수.** §4.5.350 은 `logic [-3:0] v` 를 모듈 스코프 net 에서 고치고 나머지 선언 자리를 **의도적으로** 남겼다 — 폭 opt-in(`range_to_dims_opt`)과 select 정규화 기록(`net_decl_asc_lsb`/`net_decl_neg_lsb`)은 **함께** 켜야 하고, 켜지 않으면 넓은 net 의 select 가 엉뚱한 비트를 짚는다. 그 적대 리뷰가 어디가 남았는지 재 두었고, 이 슬라이스가 그것을 붙인다.

**census 가 셋을 여섯으로.** 큐는 *포트 · 서브프로그램 지역 · 클래스 속성* 셋이었다. 10칸 census 는 **여섯**을 냈다 — 추가로 **함수 반환 타입**, **함수 형식인자**, 그리고 **static task 지역**. ⭐ 마지막 것은 `automatic`/static 이 **별개의 collector** 라서, 두 철자를 다 돌리는 census 가 아니었으면 안 보였다(§4.5.358 이 딜레이 유무로 배운 것과 같은 모양).

**⭐ 자리 특정은 추측이 아니라 계측으로.** `range_to_dims` 에 `#[track_caller]` 와 임시 `eprintln!(Location::caller())` 를 달아 **clamp 하는 호출부를 직접 찍었다** — 28개 호출부 중 정확히 5개였다(`ports.rs` · `frames_reserve.rs` ×2 · `frames_call/mod.rs` · `classes.rs` · `inline_task.rs`).

**⭐⭐ 고침은 한 함수다.** 모듈 스코프 선언 안에 인라인돼 있던 20줄(방향으로 맵을 고르고, VCD 라벨용 declared pair 도 남긴다)을 `record_declared_bounds` / `record_declared_bounds_for` 로 뽑았다. 각 자리는 `declared_odd_bound` 에 opt-in 을 묻고, 방금 만든 net 으로 기록한다. 서브프로그램 자리들은 `add_net` 직전에 `nets.len()` 을 이미 계산하므로 NetId 를 받는 쪽을 쓴다. **복사했으면 한 규칙의 여섯 철자가 됐고, 다음 선언 자리가 일곱 번째로 잊었을 것이다.**

`func_return_dims` 는 호출자가 셋(frame · inline · class)이라 전역으로 켜면 규칙을 어긴다 ⇒ **opt-in 변형**을 따로 두고 frame 경로만 켰다. 나머지 둘은 리터럴 `false` 라 바이트 동일이다.

**⚠️ 클래스 속성만 남겼다(§2-3b) — 구조가 다르다.** 클래스 필드는 net 이 아니라 `ClassField` 이고 heap slot 으로 물질화된다. 정규화 맵은 **NetId 로 키잉**되므로(`net_decl_asc_lsb.get(&net)`) **기록할 자리가 아예 없다.** 폭만 켜는 것은 §4.5.350 의 적대 리뷰가 정확히 잡은 하강(넓은 저장 + 정규화 안 된 select)이므로 하지 않았다. 선행조건은 **필드 키 정규화 맵**이다. ⚠️ 그 모양은 오라클도 하나뿐이다 — iverilog 는 `ivl_type_packed_msb(ptype,0) >= 0` assertion 으로 **죽고**, verilator 만 답한다(`w=4`, vita 의 목표값과 일치).

**differential.** 78칸(13 선언 스코프 × 6 range — `[-3:0]`·`[-1:0]`·`[-8:-1]`·`[3:-2]`·`[7:0]`·`[0:3]`) **전부 iverilog 일치 · 차이 0**. ⭐ 두 방향(오름차순 음수 msb / 내림차순 음수 lsb)이 **같은 헬퍼**를 타므로 자리를 하나 붙이면 두 철자가 동시에 켜진다 — 회귀 테스트가 그것도 핀한다.

**⚠️ 옛 제한을 의도로 고정하던 테스트 2건을 갱신했다**(삭제 아님 — §4.5.350 이 같은 자리에서 한 것과 같은 처리). `a_port_with_a_negative_bound_stays_clamped_and_loud` 는 **제한 자체를 단언**하고 있었고, `v3_12_zero_width_param_does_not_explode` 는 `parameter W=0; [W-1:0]` 관용구의 폭을 **1** 로 박아 두었다 — 두 오라클은 **2** 다(§4.5.350 이 이미 잰 값이고, 여기서 다시 확인했다). 두 테스트가 지키던 것(포트가 동작할 것 · W=0 이 IR 을 버리지 않을 것)은 그대로 두고 **숫자만 오라클에서 다시 가져왔다**.

**게이트.** 5,737 → **5,743 green**(새 회귀 6건 · 갱신 2건) · clippy 0 · fmt 0 · format 29 불변.

#### 4.5.358 — real 타깃은 폭을 빌려주지 않는다 (2026-08-22 · 5,737 green · format 29 불변)

**⭐ 큐가 가리킨 곳엔 아무것도 없었다.** §2 항목 2 는 §4.5.349 가 남긴 *"mixed-real 잔여 셋"* — framed `automatic` real 함수를 직접 피연산자로 · package/class 함수 · 단순 대입 경계(`real r; r = (-s);`) — 였다. 셋 다 재보니 **이미 두 오라클과 일치**한다. 이후 슬라이스들이 닫았고 큐가 그걸 몰랐던 것이다(§4.5.348 이 겪은 것과 같은 모양).

**대신 census 가 새 칸을 냈다.** 33칸 mixed-real 코퍼스에서 **1칸**만 달랐다 — `int unsigned us = 8; real r = (-us);` 가 두 오라클 `4.29497e+09`(2³²−8) / vita `1.84467e+19`(2⁶⁴−8). 그 가족을 60칸으로 넓히니 **12칸**이었다.

**근인 — 같은 덫의 세 번째.** §6.12 상 real 은 비트 폭이 없으므로 대입 규칙 `width = max(lhs, self(rhs))` 에서 **왼쪽이 줄 것이 없고**, RHS 는 self-determined 로 평가된 뒤 값이 변환된다(§11.8.1). 그런데 `lvalue_width` 에 물으면 real 의 **저장 폭 64** 가 나온다. 그래서 RHS 가 64비트 문맥에서 평가되고, **unsigned 이면서 64보다 좁은** 피연산자는 값이 달라진다. ⚠️ **signed 는 부호확장이 값을 보존해서 멀쩡했고, 그래서 이 버그가 숨어 있었다.**

⭐ **식 자체는 늘 옳았다.** `$display("%0d", -b)` 는 PRE 에도 248 이었고 `w32 = -b` 도 옳았다 — 그것이 자리를 연산자가 아니라 **대입 경계**로 좁혔다. §4.5.353 은 `ir_lvalue_width`, §4.5.354 는 `ir_bits_of` 에서 같은 64 를 만났다(둘 다 elaborate). 이번이 **엔진의 사본**이다.

**⭐⭐ 그리고 엔진은 같은 질문을 세 곳에서 한다.** 스케줄러 `eval_for_lvalue`(interp/VM) · 네이티브 `k_eval_for_lvalue` · **네이티브 프로그램 컴파일러**(`backend.rs`). 앞의 둘만 고쳤을 때 증상이 이렇게 됐다:

> `real r; r = -b;` 가 **`#1` 이 있는 프로세스에선 맞고 없는 프로세스에선 틀리다.**

딜레이 없는 프로세스가 네이티브 프로그램으로 컴파일되기 때문이다. ⇒ 술어를 **이름 있는 공유 함수**(`width::lvalue_targets_real`)로 두어 세 자리가 같은 철자를 쓰게 했고, ⚠️ **회귀 테스트도 딜레이 유무 양쪽을 돈다** — 이 저장소의 다른 테스트가 습관적으로 쓰는 `#1 $display(…)` 만 썼다면 **반쯤 고쳐진 엔진을 통과했을 것이다.**

**⚠️ 내 손계산이 또 틀렸다.** `byte unsigned b = 8; r = b - 100;` 을 164 로 단언했는데 두 오라클은 `2³²−92`다 — `100` 이 unsized 십진이라 32비트이고 식의 self-width 가 32 이기 때문이다. 오라클에서 값을 가져왔고, 폭이 살아남는 쪽(`b - 8'd100` = 164)을 대조로 함께 넣었다.

**differential.** 192칸(12 타입 × 8 식 × 딜레이 2종) 3-오라클 **전부 일치 · 차이 0**. 33칸 mixed-real 코퍼스도 0. real 이 없는 설계는 술어가 false 라 `lw` 가 그대로이므로 **기계적으로 불변**이다.

**⚠️ 남긴 것(§2-2b).** `$itor(-<unsigned>)` 는 **오라클이 갈린다** — iverilog 는 −8(부호 있는 해석), verilator 는 unsigned 값. vita 는 verilator 편이고 PRE==POST 다. 2-오라클 합의가 없으므로 값을 단정하지 않았다.

**게이트.** 5,732 → **5,737 green**(새 회귀 5건) · clippy 0 · fmt 0 · format 29 불변(엔진 전용 · IR 무변경).

#### 4.5.357 — 제약은 모양이 아니라 스코프였다 (2026-08-21 · 5,732 green · format 29 불변)

**착수.** §4.5.355 는 지연된 계층 타깃의 fill 을 고치면서 **bare fill 만** 지연했고, 그 이유를 *"resolve 엔 스코프가 없으니 `(raw,kind,width)` 로 재구성 가능한 유일한 모양"* 이라고 적었다. §2-1e 를 70칸으로 재니 **24칸**이 남아 있었다 — 전부 계층 전용이고 **로컬 쌍둥이는 전부 정상**이다.

**⭐ 먼저 더 나은 길이 있는지 계측했다.** 지연을 아예 없앨 수 있다면(=lowering 시점에 `hier_lookup` 이 성공한다면) 모양 제약 자체가 사라진다. `resize_fill_rhs` 에 임시 계장을 넣어 찍어 보니 **모든 케이스에서 `None`** — 자식 인스턴스의 net 은 그 시점에 **정말로 없다**. 지연은 보수적인 선택이 아니라 필수였고, 그 길은 죽었다.

**⭐⭐ 그래서 제약을 다시 진술했다.** 막고 있던 것은 *"bare fill"* 이라는 **모양**이 아니라 **스코프**다. 그러면 조건은 긍정형으로 쓸 수 있다 — *"lowering 이 스코프에 아무것도 안 묻는 식"*. `scope_free_fill_expr` = `expr_contains_fill` ∧ 구조적 화이트리스트(IntLit·Paren·MinTypMax·Unary·Binary·Ternary·Concat·Replicate). ⚠️ `_` arm 을 두지 않았다 — 다음에 추가되는 노드 종류를 **조용히 허용**하지 않기 위해서다.

**⚠️⚠️ 두 번째 아키텍처 제약을 여기서 만났다.** 새로 lower 한 부분식의 **루트 노드를 옛 슬롯에 복사**했더니 48칸이 출력을 잃었다:

> `thread 'vita-main' panicked at sim-ir/src/selfwidth.rs:60: width pass: child 8 not yet computed for parent 3 (arena not post-order)`

expr arena 는 **post-order** 다 — 부모의 id 는 모든 자식보다 커야 한다. 나중에 지은 부분식을 앞선 슬롯에 넣으면 그게 깨진다. ⭐ **자식 없는 노드 하나만 그렇게 옮길 수 있고, 그게 정확히 §4.5.355 가 통했던 이유다**(bare fill 은 `Const` 하나). ⇒ arena 슬롯이 아니라 **문장의 rhs 참조**를 새 id 로 돌린다(새 루트는 모든 것보다 큰 id 라 불변식이 유지된다).

**결과.** 70칸 census **FIXED 20 · bad-change 0 · residue 4**. ⭐ 곁수확으로 §4.5.355 의 520칸 스윕 잔여가 **MISMATCH 4 → 0** 이 됐다(그 4칸이 바로 `u.wide = '1+1`).

**⚠️ 남은 4칸과 그 이유.** `u.a = c ? '1 : 1'b0` — RHS 가 **net 을 읽는다.** 재-lower 하려면 `c` 가 resolve 되는 스코프가 필요하고, 그것이 §2-1e 가 처음부터 적어 둔 선행조건이다. 술어에서 일부러 배제했고, 그 배제가 술어를 정직하게 만든다 — **무엇이 재구성 가능한지에 대한 진술이지 무엇이 흔한지에 대한 추측이 아니다.** 테스트는 그 4칸의 **현재 답을 핀**해서, 스코프 스냅샷이 생기는 날 그 테스트가 스스로 실패하며 자기를 가리키게 했다.

**게이트.** 5,730 → **5,732 green**(새 회귀 2건) · clippy 0 · fmt 0 · format 29 불변.

#### 4.5.356 — case 의 §12.5 공통 max: 규칙이 한 방향으로만 흐르고 있었다 (2026-08-21 · 5,730 green · format 29 불변)

**착수.** 큐 §2-1c 는 *"`case ('1)` 셀렉터가 1비트로 남는다 — 라벨 쪽은 이미 문맥을 받는다"* 였다. 그 진단은 맞았지만 **절반이었다.**

**census 가 아홉.** `case ('1) 8'hFF: a=1; 1'b1: a=3;` 가 vita 3 / 두 오라클 1 — 폭 4·8·16, `casez`/`casex`, 괄호 셀렉터 전부 같은 모양. ⭐ 그리고 `'0` 셀렉터·`'1+1`·`{2{'1}}`·라벨 쪽 fill 은 **이미 전부 맞다** — 앞 두 슬라이스와 같은 이유(형제가 폭을 주거나 피연산자가 self-det)라, 깨진 집합은 또 **bare fill** 이다.

**⭐⭐ 양끝이 다 움직인다.** 1차 설계는 *"셀렉터를 라벨들의 max 로 다시 낮춘다"* 였는데, 그것만으로는 `case ('1) 8'h01: a=1; '1: a=3;` 가 **default** 로 떨어진다(두 오라클은 `'1` arm = 3). 라벨 `'1` 도 옛 1비트 셀렉터에 맞춰 낮춰졌기 때문이다. ⇒ 넓힌 폭으로 **같은 `lower_case_label`** 을 fill 라벨에 다시 돌린다. 맞던 답을 **다른 틀린 답**으로 바꾸는 것은 이 저장소가 금지하는 맞바꿈이고, 그 칸이 그것을 잡았다.

**⭐ 일반화는 내가 아니라 스윕이 시켰다.** 1차 고침 뒤 420칸 스윕에 **15칸이 남았다**. 전부 `case (1'b1) '1: a=1; 8'hFF: a=3;` 꼴 — **셀렉터에 fill 이 없다.** 그래도 §12.5 의 공통 max 는 셀렉터와 **모든 아이템**에 걸리므로 max 는 8 이고, `'1` 은 `8'hFF` 가 되어 1비트 셀렉터와도 `8'hFF` 값과도 안 맞아 **default** 다(두 오라클 일치). vita 는 `'1` 에 **셀렉터의** 폭 1 을 줘서 매치시켰다 ⇒ 게이트를 *"셀렉터에 fill"* 에서 **"셀렉터든 라벨이든 fill"** 로 넓혔고, 그러자 420/420 이 됐다.

**⚠️ real 셀렉터는 제외.** §6.12 이고 §4.5.354 가 박아 둔 핀이다 — `case (r) '1: a=1; 8'hFF: a=3;` 는 8비트 형제 라벨이 있는데도 **세 오라클 모두 `'1` arm**. real 은 max 에 들어갈 비트 폭이 없으므로 공통 패스 자체를 건너뛴다(라벨별로는 `sibling_ctx` 가 이미 같은 이유로 폭을 withhold 한다 — 규칙 하나, 두 자리).

**differential.** 420 설계(14 셀렉터 × 10 라벨셋 × 3 case 종류) 3-오라클: **일치 342 → 420 · MISMATCH 78 → 0**, PRE 대비 78칸이 바뀌었고 **전부 iverilog 로 수렴**. bench 4설계(keccak ×3 · picorv32) `.vu`/`.velab` **바이트 동일** — fill 없는 `case` 는 두 번째 패스에 들어가지도 않는다.

**게이트.** 5,722 → **5,730 green**(새 회귀 8건 = `crates/cli/tests/case_selector_fill_width.rs`) · clippy 0 · fmt 0 · format_version 29 불변(elaborate 전용).

#### 4.5.355 — 계층 타깃의 fill 폭: 폭은 없던 게 아니라 늦었다 (2026-08-21 · 5,722 green · format 29 불변)

**착수.** §4.5.353 이 `resize_fill_rhs` 로 *"fill 은 대입의 폭을 받는다"* 를 전 대입 형태에 깔았는데, 타깃이 `a` 가 아니라 `u.a` 면 안 먹었다. 교차-인스턴스 쓰기는 lvalue 를 **센티널 net** 으로 낮추고(자식 인스턴스의 net 이 아직 없다), `ir_lvalue_width` 가 `nets.get(0xFF00_0000)` → `None` → `.max(1)` → **1** 을 답한다.

**census 가 3 을 7 로, 적대 프로브가 10 으로.**

| 모양 | PRE | 두 오라클 |
|---|---|---|
| `u.a = '1` · `u.a <= '1` · `force u.a = '1` | `000000000001` | `111111111111` |
| `assign u.a = '1`(절차적) | `000000000001` | `111111111111` |
| `u.v.q = '1`(2단) | `000000000001` | `111111111111` |
| `u.arr[0] = '1` | `000000000001` | `111111111111` |
| `u.a[7:0] = '1` | `xxxx00000001` | `xxxx11111111` |
| `u.a = 'x` · `'z` | `00000000000x` · `…z` | `xxxxxxxxxxxx` · `zzz…` |
| task 스코프 안의 `u.a = '1` | `000000000001` | `111111111111` |

⚠️ **ROADMAP 이 적어 둔 근인은 절반이었다.** *"센티널 = `HIER_WRITE_SENTINEL_BASE`"* 만 적혀 있었는데, 계측(`resize_fill_rhs` 임시 계장)이 레인이 **둘**임을 보였다 — `u.a` 는 `0xFF000000`(write 레인), `u.a[7:0]`·`u.arr[0]`·`u.a[0]` 는 `0xFE000000`(**sel 레인**). ⭐ 그리고 **`u.a[0]` 도 sel 레인이고 width=1 인데 그건 옳다**(1비트 타깃 · 세 오라클 일치) ⇒ **레인 단위로 고쳤으면 맞는 것을 깨뜨렸다.**

**⭐⭐ 설계 = 새 규칙이 아니라 같은 질문을 나중에.** sel 레인의 resolve 는 net 의 shape 로부터 `LvalChunk` 를 **통째로 재구축**한다 — 즉 그 시점엔 폭이 이미 정확하다. 그래서 두 레인이 각자 결정한 chunk 를 `hier_resolved_chunk` 로 publish 하고, `resolve_pending_fill_widths` 가 **lowering 때 1 을 답했던 바로 그 `ir_lvalue_width`** 를 그 chunk 에 다시 묻는다. 하위 케이스가 열거 없이 떨어진다:

- 전체-net → net 폭 · part-select → 재구축 chunk 의 part 폭 · element → 원소 폭 ·
- **bit-select → 1, 즉 변화 없음** — 불가침 핀이 **구조적으로** 지켜진다.

이 대칭이 곧 정당화다: 아키텍처는 이미 *"지금은 센티널로 낮추고 알게 되면 패치한다"* 이고, **폭은 그때 모르는 두 번째 값**일 뿐이다.

**⭐ bare fill 만 지연하는 이유.** 지름길이 아니라 제약이다 — resolve 패스엔 **lowering 스코프가 없고**, `(raw, kind, width)` 만으로 재구성 가능한 모양은 bare fill(+ `(…)`/`MinTypMax` 투명 래퍼)뿐이다. `push_expr` 은 append 라 각 리터럴이 자기 arena 슬롯을 독점하므로, 그 슬롯의 `Const` 를 덮어써도 다른 식을 건드리지 않는다.

**⚠️⚠️ 자기 정정 — 중간 결론이 우연이었다.** 12비트 타깃으로 중첩 fill 을 프로브하고 *"`'1+1`·`{2{'1}}`·`c?'1:12'h0`·`~'0` 이 전부 이미 맞으니 **bare 가 곧 깨진 집합 전체**"* 라고 적었다. **520칸 스윕이 그것을 깼다**: `u.wide = '1+1`(64비트) = **4294967296** 인데 로컬 `W = '1+1` 은 **0**(iverilog 일치). 규칙은 `max(ctx, sibling)` 이고 형제(`1` = 32비트)가 타깃보다 좁으면 ctx 가 여전히 유일한 폭 출처다 — 12비트에선 두 경로가 우연히 만나 보이지 않았을 뿐이다. ⇒ §2-1e 로 기록했고, 테스트의 해당 주장도 **형제의 폭**으로 다시 썼다(`u.a = '1 & 12'hF0F` 가 규칙을 정확히 진술하는 칸).

**적대 프로브.** real 계층 타깃(`u.r = '1` = 1.0 · §6.12 가드가 지연 레인을 통과해도 살아있다) · 미해결 계층 타깃(E3010 유지) · 같은 타깃 두 번 · 한 문장 안 두 센티널 · task 스코프 — 전부 PRE 대비 회귀 0.

**differential.** 520 설계(13 타깃 × 10 RHS × 4 대입 형태) 3-오라클: **일치 308 → 436 · MISMATCH 132 → 4** 이고 ⭐ **바뀐 128칸이 전부 iverilog 와 일치**(변했는데 안 맞는 것 **0**). 남은 4 = §2-1e(PRE==POST), 남은 80 = `force`/절차적 `assign` 의 계층 part/bit-select E3009(PRE==POST · 기존 loud follow-on). bench 4설계(keccak ×3 · picorv32) `.vu`/`.velab` **바이트 동일**.

**게이트.** 5,714 → **5,722 green**(새 회귀 8건 = `crates/cli/tests/fill_hierarchical_target_width.rs`) · clippy 0 · fmt 0 · format_version 29 불변(elaborate 전용 · 직렬화 형상 무변경).

#### 4.5.354 — `lower_expr_ctx` 가 `lower_expr` 의 도메인 라우트를 전부 잃었다 (2026-08-21 · 5,714 green · format 29 불변)

**착수와 반전.** 큐 §2-1b 는 *"`{<string>, '1}` 이 무한 루프 · 최소 조치는 **loud 화**"* 였고, 직전 세션이 *"오라클이 없으니 loud 가 정답"* 이라고 못까지 박아 두었다. 착수 첫 행동을 구현이 아니라 **census** 로 잡은 것이 그 계획을 뒤집었다.

**census 가 1 을 6 으로, 리뷰가 다시 10 으로 만들었다.** 33칸 3-오라클 census + 코드 읽기가 같은 뿌리에서 여섯 모양을 냈고, 적대 리뷰가 real 축에서 넷을 더했다(아래 ⓖ~ⓙ).

| 모양 | PRE | fill 없는 쌍둥이 | 판정 |
|---|---|---|---|
| `wire[11:0] a = {s, '1}` | **비종료** | `{s,1'b1}` 정상 | 10개 설계 |
| `wire[23:0] a = {2{s,'1}}` | `…0101`(문자열→비트) | `{2{s,1'b1}}` 정상 | silent-wrong |
| `s < {"a",'1}` | `0` | `s < {"a",1'b1}` = `1` | **두 오라클 1** |
| `h == '1` | `0` at exit 0 | `h == 1'b1` = E3009 | **loud→silent** |
| `{ {0{x}}, '1 }` | E3009 | `{ {0{x}}, 1'b1 }` 정상 | **거짓 loud**(iverilog 정상) |
| `N=-2; {N{'1}}` | `111111111111` | `{N{1'b1}}` = E3009 | silent-wrong |
| `r & '1` · `r << '1` | `0` at exit 0 | `r & 1'b1` = E3009 | **loud→silent**(§6.2) |
| `r ** '1` | `0` | `r ** 1'b1` = `3` | **두 오라클 3**(§11.4.9 `$pow` **라우트**를 통째로 잃었다) |
| `r + '1` | `0` | `r + 1'b1` = `4` | **두 오라클 4**(`ir_bits_of` 가 real 에 **64** 를 답한다) |
| `c ? r : '1` (c=0) | `0` | `c ? r : 1'b1` = `1` | **두 오라클 1**(같은 덫·Ternary arm) |
| `case (r) '1:` | `default` 로 흘렀다 | `case (r) 1'b1:` 은 match | **두 오라클 match**(같은 덫·`lower_case_label`) |

⭐ **모든 행에 "fill 없는 쌍둥이" 열이 있다.** 그게 곧 뿌리의 진술이다 — 이건 열 개의 버그가 아니라 **fill 리터럴이 라우팅 스위치가 되어 있던 것 하나**다. `lower_expr` 의 front gate 가 fill 을 보고 `lower_expr_ctx` 로 보내는데, `lower_expr_ctx` 는 문맥 전파 6종을 **다시 구현한 두 번째 dispatch** 이고 원본의 도메인 라우트(string concat·string replicate·StrCmp·N7 handle gate·§6.2 real)를 하나도 안 갖췄다.

**비종료의 정체.** `lower_expr_ctx` 의 Concat arm 이 string part 를 보면 `return self.lower_expr(e)` 로 **같은 노드**를 돌려주고, front gate 의 두 조건(`is_ctx_node` ∧ `expr_contains_fill`)이 그 노드에 대해 여전히 참이라 곧바로 되돌아온다. ⚠️ **둘 다 tail call 이라 릴리즈는 스택이 안 자란다** — 100% CPU 에 RSS 고정이라 프로세스만 봐선 무한 재귀로 안 보인다. 디버그만 스택 오버플로로 죽는다(rc=134).

**⭐⭐ 적대 리뷰가 설계를 바꿨다.** 1차 설계는 *"도메인 라우트 넷을 흉내내는 가드 하나"* 였고 census 4칸을 전부 고쳤다. soundness 렌즈가 그 다음에 ⓔ(`repl_zero_ok` 미설정 ⇒ 거짓 loud)와 ⓕ(음수 replication count 무검사)를 찾았고 — **둘 다 PRE==POST, 즉 내가 만든 게 아니라 같은 뿌리의 나머지** — 그것을 보고서야 진짜 구조가 보였다:

> `Concat`/`Replicate` 피연산자는 **self-determined** 다(§11.4.12). 그래서 그 두 ctx arm 은 모든 피연산자에 `ctx = 0` 을 넘기고 있었는데, **그건 정의상 `lower_expr` 이 하는 것과 같다**(`lower_ctx_or_plain(p,0) ≡ lower_expr(p)` — 전 노드 종류에 대해 확인). 즉 두 arm 은 **순수 중복이면서 원본보다 뒤처진 중복**이었다.

⇒ 2차 설계 = **가드 넷이 아니라 arm 둘 삭제.** ⓔⓕ 가 부수적으로 해소되고(다시 어긋날 수도 없다), 진짜로 가드가 필요한 건 **Binary 비교 하나**만 남는다 — 거긴 정수 경우에 ctx arm 이 실제 일을 하기 때문이다. `binary_stops_ctx` 는 `lower_expr` 사이트의 handle/StrCmp 조건을 **그 사이트의 순서대로 verbatim 복사**해서, 추론이 아니라 diff 로 검사되게 했다.

**⭐ 종료성이 구조적이 됐다.** `lower_expr` 을 게이트와 `lower_expr_ungated` 로 쪼갰다. 이제 불변식은 한 문장이다 — *`lower_expr_ctx` 를 떠나는 모든 호출은 **진부분식**에 대한 것이거나 **게이트 없는 진입점**으로 간다.* `is_ctx_node` 에 종류를 추가하거나 `lower_expr_ctx` 에서 arm 을 지워도 사이클이 부활할 수 없다. (`_` arm 도 ungated 로 적었다.)

⚠️⚠️ **그리고 그 `_` arm 에 대한 내 주석이 틀렸다는 것을 뮤테이션이 잡았다.** 처음엔 *"`is_ctx_node` 가 받는 종류는 전부 명시 arm 이 있으니 두 철자는 기계적으로 바이트 동일"* 이라고 적었고, 그래서 *"게이트 있는 `lower_expr(e)` 로 되돌리는 뮤턴트는 **SURVIVE** 할 것"* 이라고 예측했다. **KILLED, 8개 테스트.** 이유는 같은 슬라이스가 그 뒤에 한 일이다 — `Concat`/`Replicate` arm 을 **삭제**했으므로 그 두 종류가 `_` 로 떨어지고, 둘 다 `is_ctx_node` 라 게이트가 다시 발화해 **원래의 비종료가 그대로 복원된다**. 주석은 쓸 때 참이었고 삭제 뒤 거짓이 됐다. ⇒ `lower_expr_ungated` 는 스타일이 아니라 **삭제를 안전하게 만드는 부품**이고, 주석을 그렇게 고쳤다. ⭐ **뮤테이션이 소스가 아니라 내 추론을 검사한 사례** — *"뮤턴트의 예상 결과를 먼저 적어라, 틀리면 그게 발견이다"* 가 그대로 작동했다.

**검증 — 외부 오라클이 없는 족속을 어떻게 고정했나.** string 이 든 비트-concat 은 오라클이 **없다**: iverilog 13.0 은 `internal error: 14vvp_fun_concat: recv_string not implemented` + assertion 으로 죽고, verilator 5.050 은 `V3Number.cpp: Number operation called with non-logic argument` 로 빌드를 거부한다. ⚠️ **그건 fill 없는 `{s, 8'h0F}` 도 마찬가지** — 즉 *"오라클이 없다"* 는 이 슬라이스가 만든 조건이 아니라 이미 그랬던 조건이다. 그래서 값은 **합성**으로 고정했다:

- *"concat 피연산자는 self-det ⇒ fill 은 정확히 1비트"* 는 **두 오라클이 string-free 족속**(`{4'hA,'1}`·`{'1,4'hA}`·`{'1,'1}`·`{'1}`·`{2{'1}}`)에서 확인해 준다;
- 문자열 부분의 처리는 이 슬라이스가 **안 건드린** 기존 경로이고, 문장 레벨 `string_concat_special`(게이트를 안 타서 PRE 에도 옳았다)과 `1'b1` 쌍둥이라는 **vita 안의 두 철자**가 값을 고정한다.
- ⭐ 그리고 한 칸은 진짜 외부 오라클을 갖는다 — string 타깃 `string r; r = {s,'1};` 은 iverilog·verilator·PRE·POST 네 산출이 **바이트 동일 `ab\x01`**. 나머지가 믿고 가는 단계(1비트 fill 이 `%s` 로 0x01 한 바이트가 된다)가 거기서 확인된다.
- ⚠️ **그 칸이 방법론 교훈이기도 하다**: 네 산출을 터미널로 비교했으면 마지막 바이트가 무엇이든 넷 다 `ab` 로 보인다. 실제로 첫 테스트 기대값이 그래서 틀렸다.

**⚠️ 리뷰가 두 번 더 늘렸다 — 그리고 두 번째는 `real` 이었다.** diff 를 적대적으로 다시 읽다 *"`lower_expr` 의 Binary arm 엔 §6.2 real 게이트가 있는데 ctx 트윈엔 없다"* 를 봤고, 실측이 **loud→silent** 를 확인했다(`r & 1'b1` = E3009 인데 `r & '1` 은 조용히 `0`). ⭐ 그런데 그 안에 **진단이 아닌 것 하나**가 섞여 있었다 — §11.4.9 의 `**` → `$pow` 는 **라우트**라, 트윈은 그것을 통째로 잃고 정수 `Binary` 를 지었다(`r ** '1` = `0`, **두 오라클 3**). ⇒ §6.2 블록 전체를 `binary_real_operand_route` 로 뽑아 **두 arm 이 한 철자를 공유**하게 했다(반환 `Option<u32>` = 연산자가 **대체되는** 경우와 진단만 하는 경우의 구분).

그 수정을 재는 중에 **여덟 번째**가 나왔다: `r + '1` 이 `0`(두 오라클 **4**), 쌍둥이 `r + 1'b1` 은 `4`. ⚠️ **`ir_bits_of` 가 real 넷에 64 를 답한다** — 저장 크기이지 언어가 노출하는 폭이 아니다. §4.5.353 이 `ir_lvalue_width` 에서 만난 **바로 그 덫이 한 층 아래**(대입이 아니라 **연산자**)에 있었던 것. §11.8.1 은 혼합식의 정수 피연산자를 real 로 변환하므로 fill 은 자기결정 1비트로 남아야 하고, 오라클의 `2.5 + 1 = 3.5 → 4` 가 그것이 1비트임을 증명한다 ⇒ `sibling_ctx`(real 형제는 폭을 빌려주지 않는다).

⭐⭐ **그리고 여기서 규칙을 나 자신에게 적용했다.** *"같은 덫이 두 층에서 나왔으면 나머지 층을 찾아가라"* 는 이 슬라이스가 방금 ENGINEERING_RULES 에 적은 문장이다. `ir_bits_of` **전수 grep** 이 fill 문맥을 먹이는 자리를 네 개 더 냈고, 그중 **둘이 진짜였다** — Ternary arm(`c ? r : '1` = `0`, 두 오라클 **1**)과 `lower_case_label`(`case (r) '1:` 이 `default` 로 흘렀다, 두 오라클은 **match**). 나머지 둘은 이미 막혀 있었다(`lower_wildcard_eq` 는 real 을 loud 로 거부 · `resize_inline_assign` 은 첫 줄에서 real 을 반환). ⇒ `sibling_ctx` 는 **호출자 셋**을 갖는다 — *"내 형제는 얼마나 넓은가"* 를 묻는 자리가 binary 피연산자·ternary 분기·`case` 셀렉터 셋이기 때문이다. ⚠️ Ternary 는 **프로브 방향이 중요**했다: `c ? '1 : r` 은 같은 식으로 잘못 sizing 되는데도 그 분기가 선택되지 않아 증상이 안 보인다.

**real 축 3-오라클 census(240칸).** 불일치 **80 → 16**(FIXED 65). 남은 16 은 전부 **쌍둥이도 같은** pre-existing 가족이다 — 12칸은 `%`-on-real(IEEE §11.4.3 이 금지하는데 iverilog·verilator 는 **둘 다 실행**한다 ⇒ vita 의 E3009 가 LRM 편이고, no-fill 쌍둥이 6칸은 PRE 에도 이미 E3009 였다), 4칸은 `r / '0`(0 나눗셈 · 쌍둥이 `r / 1'b0` 동일). ⚠️ 유일하게 "이동" 한 `'0 % r` 은 PRE 가 **우연히** 맞는 숫자를 낸 자리다(같은 경로가 `'1 % r` 엔 틀린 답을 냈고 쌍둥이는 이미 E3009 였다) — 우연한 일치는 correct-support 가 아니다.

**⭐ 삭제의 근거는 뮤테이션이 경험적으로도 확인해 줬다.** arm 둘을 지울 수 있었던 이유는 `lower_ctx_or_plain(p, 0) ≡ lower_expr(p)` 라는 등가성이다. front gate 에서 `is_ctx_node` 필터를 **빼는** 뮤턴트(= fill 을 가진 **비-ctx** 노드까지 전부 `lower_expr_ctx(e,0)` 로 보내는 것)를 *"SURVIVE 할 것"* 이라 예측했고 **SURVIVED** — 그 필터는 의미가 아니라 O(1) 성능 필터임이 확인됐고, 그것이 곧 위 등가성이다.

**differential.** ⓐ 생성 fill 코퍼스 **564 설계**(6종 노드 × 4종 fill × 2종 대입 철자) PRE↔POST **0 차이** ⓑ string/handle 코퍼스 160칸 중 **46 변화 = 전부 의도 집합**(비종료→값 20 · replicate 4 · handle→E3009 10 · 비교 12) ⓑ' real 코퍼스 **240 설계** 3-오라클 = 일치 160 → **224**(불일치 80 → 16) ⓑ'' ternary/case 코퍼스 **60 설계** = iverilog 일치 48 → **52 · 그 외 변화 0** ⓒ bench 4설계(keccak ×3 · picorv32) `.vu`/`.velab` **바이트 동일** ⓓ 구조적 근거: `lower_expr_ctx` 의 **다섯 진입점이 전부 fill-게이트** 라 fill 없는 설계는 새 코드에 닿지도 않는다.

**⚠️ 남긴 것 — 오라클 분열 둘 + 관대함 하나(§2-1d).** ⓐ `s <= 1'b1` 이 iverilog **1** / verilator **0**(vita 는 0) · ⓑ `'1 ** r` 이 iverilog **480** / verilator **1**(vita 는 480 — PRE 는 **둘 다 아닌 0** 이었으니 악화는 아니다): `**` 의 밑수는 Table 11-21 상 문맥-결정인데 §11.8.1 이 식 전체를 real 로 만들 때 그 문맥이 살아남는지가 갈린다 · ⓒ `%` 는 IEEE §11.4.3 이 real 피연산자를 금지하는데 **iverilog·verilator 둘 다 실행한다**(vita = E3009 = LRM 편 · PRE 부터·쌍둥이도 그랬다) ⇒ 고칠 것 없음.

셋 다 **fill 이 있든 없든 같다** — 이 슬라이스가 만든 게 아니라 **드러낸** 것이고, ⓐⓑ 는 2-오라클 합의가 없으므로 값을 단정하지 않았다. 그래서 비교 테스트는 **값이 아니라 "fill 철자와 `1'b1` 쌍둥이가 일치한다"는 성질**을 고정하고, `'1 ** r` 은 단언 대신 주석으로 남겼다 — 이 슬라이스가 책임지는 건 그 일치이지 StrCmp/`**` 의 의미가 아니다. ⭐ 값을 단언했으면 **분열을 조용히 판결**한 것이 됐을 것이다.

**뮤테이션 배터리(16발 설계 · 12발 실행 · **10 KILLED / 1 SURVIVED**).** m1~m8·m10·m11 **KILLED** · m9 **SURVIVED**(예측대로 — 위의 등가성 확인) · 보충 m12 **KILLED**(real 라우트) · ⚠️ **m8 은 예측이 틀렸다**(위 참조 — 등가를 예상했는데 8개 테스트를 죽였고, 그 오류가 곧 발견이었다). 보충 4발(m13~m16)은 3-way CPU 경합으로 중단했다 — m12 가 같은 두 헬퍼를 이미 죽였고 m9 가 m16 이 하려던 등가성 확인을 대신했으므로, **중단이 가린 것은 없다**(그래도 실행하지 **않은** 것이니 여기 적는다).

**게이트.** 5,694 → **5,714 green**(새 회귀 20건 = `crates/cli/tests/ctx_lowering_domain_routes.rs`) · clippy 0 · fmt 0 · format_version 29 불변(elaborate 전용 · 직렬화 형상 무변경). 새 테스트의 `run_bounded` 는 60초 타임아웃을 갖는다 — 재발이 CI 를 영원히 멈추는 대신 *"상호 tail call 이 돌아왔다"* 는 문장과 함께 실패하도록. ⚠️ **정확히는 보험이다**: 테스트가 도는 디버그 빌드에서는 사이클이 스택 오버플로로 abort 하므로 재발은 1초 안에 `assert!(success)` 로 잡힌다(사이클을 복원하는 뮤턴트가 실제로 그렇게 KILLED 됐다). 타임아웃이 필요한 쪽은 **릴리즈 모양** — 거긴 진짜 tail call 이라 죽지 않고 영원히 돈다.

#### 4.5.353 — fill 리터럴이 문맥 폭을 못 받는 대입 형태 셋 (2026-08-21 · 5,694 green · format 29 불변)

**착수 = §2 큐 머리**(*"net 선언 초기화의 fill 리터럴이 1비트로 sizing 된다"*). 큐엔 **하나**로 적혀 있었고
3-오라클 census 가 **셋**임을 보였다. 그리고 **머신러리는 이미 완성돼 있었다** — 세 자리가 그걸 안 불렀다.

**뿌리 = 같은 구문의 두 하강 중 하나가 잊었다.** IEEE 1800 §5.7.1 은 unsized unbased 리터럴
(`'0`/`'1`/`'x`/`'z`)이 **문맥 폭을 받아 전 비트를 채운다**고 하고, §11.6 은 대입 RHS 를 lvalue 폭의
문맥-결정 위치로 만든다. `Elaborator::resize_fill_rhs` 가 정확히 그것이고 이미 `assign`·블로킹·논블로킹
하강에서 **불리고 있었다**. 안 부른 셋:

- ⓐ **net 선언 초기화** `wire [7:0] a = '1;` — 이건 **암묵적 continuous assign** 이라 바로 옆에 적은
  `assign a = '1;` 과 답이 달랐다(00000001 vs 11111111). ⭐ **그 비대칭이 자리를 찍어 줬다.**
- ⓑ **`force`** `force a = '1;`
- ⓒ **절차적 continuous assign** `assign a = '1;`(프로세스 안) — `Stmt::Force` 로 하강하는데,
  **어느 IR 노드로 내리느냐는 구현 선택이지 폭 규칙이 아니다.**

⭐ **ⓑ·ⓒ 는 write-twin 스윕이 찾았다**(memory: widen-read-sweep-write-twin). 큐가 준 ⓐ 만 고쳤으면
`force` 는 조용히 틀린 채 남았다. 대입 모양 census 표에서 port 연결·함수 인자·파라미터 오버라이드·
assignment pattern·case label·함수 반환·task output·NBA·part-select·concat-lvalue·배열 원소·
struct 필드는 **이미 옳았다**.

**⚠️⚠️ 적대 리뷰가 내가 만든 correct→silent-wrong 을 잡았다 — 이 슬라이스의 가장 중요한 사건.**
ⓑ·ⓒ 에 호출을 **무조건** 달았더니 `real` 타깃을 같이 끌고 들어갔다. `ir_lvalue_width` 는 real net 에
**64**(저장 폭)를 답하는데 real 대입엔 **비트 문맥이 없다**(§6.12/§11.6) ⇒ `'1` 이 2^64−1 이 되고
엔진의 정수→real 변환이 **1.84467e+19** 를 만든다. 두 오라클은 **1.0**.
PRE 는 그 arm 에서 helper 를 아예 안 불렀으니 **PRE 가 맞고 POST 가 틀렸다** = 사다리 하강.

⭐ **가드를 호출부가 아니라 `resize_fill_rhs` 안에 뒀다**, 그리고 그게 정답이었다 — 규칙 하나를
다섯 호출자가 한 철자로 공유하고, **선행 호출자들이 이미 같은 방식으로 틀려 있었기** 때문이다:
`real e; e = '1;` · `real d = '1;` · `real f[0] = '1;` 셋 다 PRE 에서 이미 1.84467e+19 였다.
⇒ **내 회귀 2건을 막으면서 pre-existing silent-wrong 4건을 덤으로 고쳤다.** 호출부에 달았으면
철자가 셋에서 다섯으로 늘기만 했다.

**⚠️ 두 번째 리뷰 발견 — 단축 평가가 스스로 under-detect 하고 있었다(같은 클래스라 이번에 고침).**
`expr_contains_fill` 이 `MinTypMax` 를 안 걸었다. `lower_expr` 는 `(1:'1:2)` 를 **투명하게** `typ` 로
내리는데(문맥은 도달함) **게이트가 못 봐서** 전 대입 자리에서 1비트로 남았다(iverilog 11111111 /
PRE·초안 POST 00000001). 두 walk 에 arm 하나씩 — `expr_contains_fill` 과 `lower_expr_ctx` — 을 추가.

⚠️ **규칙의 나머지 절반도 같이 핀했다.** 자기결정 위치의 fill 은 **1비트로 남아야** 한다 — shift 량,
`**` 지수, `&&`/`||` 피연산자, 비트 인덱스. 여기를 같이 넓혔으면 `8'd1 << '1` 이 `1 << 255` = 0 이 되어
**silent-wrong 을 다른 silent-wrong 과 맞바꾸는** 짓이 된다. 8칸 실측 + 테스트 고정.

**곁수확**: `'x`/`'z` 도 같이 고쳐졌다(이제 `xxxxxxxx`/`zzzzzzzz` = iverilog). ⚠️ verilator 의
`00000000` 은 **2-state 한계**이지 오라클 분열이 아니다 — §5.7.1 이 명시적이라 iverilog + LRM 이 정본.

**측정(PRE-3-way)**: `git archive 7b9b708` PRE 바이너리 · 41칸 파일 코퍼스에서
**FIXED 17 · REGRESSION 0 · unchanged-ok 24** + 인라인 25칸 전부 ok.
**비-fill 설계는 기계적으로 바이트 동일** — `!expr_contains_fill(rhs)` 리터럴 조기 반환이라 IR 이
한 비트도 안 바뀐다. picorv32(20k) + keccak 3종 stdout·stderr·exit 동일로 실측.

**뮤테이션 10발 — 10 KILLED(생존 0).** 세 사이트 각각 제거·fill 폭을 1 로 고정·`resize_fill_rhs` 를
no-op 화·real 가드 제거·real 술어를 항상 false·`MinTypMax` arm 을 두 walk 에서 각각 제거 — 전부 죽었다.
⭐ **내 예측이 하나 틀렸고 배터리가 고쳤다**: 단축 평가(`if !expr_contains_fill { return }`)를 없앤
mutant 을 *"IR/성능 가드일 뿐 값 가드는 아니니 생존"* 이라 적었는데 **KILLED** 였다 —
`cli::clog2_repl_count` 4건이 죽는다. `lower_expr_ctx` 의 Replicate arm 은 count 를
`lower_index_expr` 로 내리는데 `lower_expr` 의 arm 은 `$clog2` 폴드 경로를 쓴다 ⇒ **fill 없는 replication 을
문맥 경로로 보내면 값이 바뀐다**. 단축 평가는 바이트 동일성 장치가 아니라 **값 장치**이고,
그래서 real 가드는 반드시 **그 뒤**에 온다.

**적대 3렌즈**(soundness / differential / completeness-census): BLOCKING **1건 = 위의 real 회귀**
(두 렌즈가 독립으로 잡았고 refuter 도 confirm) · 나머지는 전부 **pre-existing 이고 PRE==POST** 로 확인.
soundness 가 증명한 것 중 값진 둘: ⓐ **dead IR 은 런타임 부작용을 못 만든다**(`lower_expr`/
`lower_expr_ctx` 는 `ProcessBuilder` 를 안 받으므로 문장을 emit 할 수 없다 ⇒ 버려진 첫 하강은 어떤
문장 루트에서도 도달 불가) ⓑ ⓐ 사이트의 **폭은 이미 확정**이다(net 은 pass 4, `elaborate_net_init_drivers`
는 pass 7).

**스코프 밖 — 전부 pre-existing·PRE==POST 로 실측하고 §2/§3 에 기록**:
- 🔴 **계층 타깃**(`u.a = '1` · `u.b <= '1` · `force u.c = '1`) — **네 번째 자리이고 뿌리가 다르다**:
  deferred-hier 센티널 net id 때문에 `ir_lvalue_width` 가 **1** 을 답해 다섯 호출자 전부에서 no-op.
  두 오라클 111111111111 / vita 000000000001.
- 🔴 **`{string, '1}` 이 무한 루프**(진단도 종료도 없음) — `lower_expr` 의 front gate 와
  `lower_expr_ctx` 의 Concat-string arm 이 서로를 tail-call 한다. correct-or-loud 의 최악 결과.
- 🔴 **`case ('1)` 셀렉터가 1비트로 남는다**(§12.5 는 case 식과 item 을 공통 max 로 sizing).
- ⚠️ NIT: 이중 하강이 **elaborate 진단을 두 번** 낸다(pre-existing · loud→louder · `MAX_ELAB_ERRORS`
  200 을 두 배로 소모). `resize_fill_rhs` 의 doc 이 **없는 `!= 32` 가드**를 설명하고 있던 것도 정정.
- honest-loud 확인: `tri1 [7:0] a = '1;`(E3009) · `force a[11:4] = '1;`(E3009 — LRM 은 불법이라지만
  두 오라클 다 0ff0) · 함수 지역 `logic [7:0] v = '1;`(E3010). struct assignment-pattern 멤버 fill 은
  **진짜 오라클 분열**(iverilog 거부/verilator 수용)이라 불가침.

#### 4.5.352 — settle 가 델타마다 199개를 훑어 0개를 찾고 있었다 (2026-08-21 · picorv32 −10.5%)

**착수 = 직전 슬라이스가 남긴 추천 후보** — *"`settle_cont_assigns` 쪽 `run_wprog` 는 아직 평면 워드에서
72바이트 `Value` 를 다시 짓는다"*. 그 후보는 **진짜였고 −2.4% 였다.** 그런데 §4.5.351 이 세운 규칙
(*"짓기 전에 나눗셈부터"*)을 지키려고 그 함수를 프로파일했더니, **같은 함수 안에 6배 큰 것**이 있었다.

**뿌리 = `continue` 만 하는 루프 바디.** `Scheduler::schedule_delayed_cas` 는 이렇게 시작했다:

```rust
for ci in 0..self.st.ir.cont_assigns.len() {
    let Some(d) = self.st.ir.cont_assigns[ci].delay else { continue };
```

소스로 읽으면 *"딜레이 붙은 assign 들을 처리한다"* 이다. 실제로는 **199개를 전부 만지고, settle 마다,
1,400,006 번**이다. 그리고 picorv32 에는 **딜레이 붙은 cont-assign 이 하나도 없다**.

⭐ **단위가 두 자릿수 틀려 있었다.** settle 은 **타임스텝당이 아니라 델타당**이다 — 200k 사이클에
settle 1,400,006 회(사이클당 7). O(설계 크기) 스캔을 델타당 함수 안에 두면 O(설계 × 델타)다.
계측: **1,400,006 × 199 = 278,601,194 회 반복, 그중 일한 것 0회.** 프로파일과 산술이 독립으로 일치했다
(`/usr/bin/sample` 542/7482 = 7.24% ↔ 278.6M × ~2 사이클 = 0.17 s / 2.31 s = 7.3%).
⚠️ **첫 재프로파일은 분모가 안 맞았다**(10 s 창 · POST 런이 창 안에서 끝나 7482 vs 6879). 창을 6 s 로
줄여 **분모를 맞춘 뒤**(4474 vs 4499) 다시 재니 `settle_cont_assigns` self **8.83% → 1.62%**
(절대 시간 ≈ **204 ms → 34 ms**, 200k 사이클 한 런 기준). 같은 재측정에서 `exec_vm::run` 은
12.96% → 14.60% 로 **올랐는데 그게 정상**이다 — 전체가 10.4% 줄면 안 바뀐 함수의 share 는
1/0.896 = 1.116 배가 되고, 측정된 비는 1.127 이다.

**ⓐ 반 A — 딜레이 집합을 한 번만 정한다.** `Scheduler` 에 `delayed_ca_idx: Vec<u32>`(오름차순, 생성자에서
`ca.delay.is_some()` 로 한 번). `schedule_delayed_cas` 는 그걸 순회한다. ⭐ **바디의
`let Some(d) = … else { continue }` 는 그대로 뒀다** — `unwrap` 으로 바꾸면 사전필터가 규칙의 **두 번째
철자**가 된다. 그대로 두면 필터가 틀렸을 때 옛 경로로 안전하게 떨어지고, 실제로 **계측이
`delayskip == 0`** 을 보여 필터와 바디의 판단이 런타임에 정확히 일치함을 증명했다(공허하지 않게:
같은 계측이 `delaybody = delaycall × delaylen` 도 보였다). `st.ir` 은 `&SimIr` 이고 `cont_assigns` 를
런타임에 바꾸는 곳은 없다(프로덕션 mutation 은 전부 elaborate) ⇒ 집합은 상수.
**엔진·tier-3 양쪽 호출자가 같이 받는다.**

**ⓑ 반 B — 추천 후보 그 자체.** settle 의 평범한 arm 이 `k_eval_for_lvalue → k_resolve_lvalue_offsets →
write_routed` 였다. 새 `NativeKernel::write_settled` 가 목적지가 이미 증명된 plain scalar 면
`store_plain_word`(= `write_chunk_word`) 로 곧장 간다. **세 번째 리전**이다 — 블로킹은 D1.6
(`k_write_scalar`), NBA 는 §4.5.351, 남은 하나가 cont-assign settle 이었고 **그게 델타당 도는 리전**이다.
picorv32/200k 에서 settle 쓰기 **3,343,081 건 중 100.0%** 가 그 모양. 술어는 다른 두 리전과 **같은 함수**
(`plain_scalar_dest_of`) — 재유도하면 heap/frame/class 를 맞혀야 하고 그 dead slot 에 flat write 하면 조용하다.
`store_plain_word` 가 이제 `changed` 를 **반환**한다(settle fixpoint 의 계속 조건) · `k_write_scalar` 는
`let _ =` 로 버린다(그쪽엔 소비자가 없다).

**측정**: picorv32 200k **2.31 → 2.07 s**, 백투백 A/B 로 **세 배치가 −10.4% / −10.5% / −10.8%**
(각 배치 콜드 1쌍 폐기) ⇒ **≈ −10.5%, 배치간 산포 ±0.2%p**. 반 A 단독 **−8.4%** · 반 B 를 그 위에 얹어
**추가 −2.4%**. keccak 회귀 없음. 마지막 배치는 **커밋되는 바로 그 소스**로 다시 쟀다.
⭐ **독립 재측정이 일치**(리뷰 3렌즈): 2.31 → 2.07 = **−10.4%** · 반 A **−7.8%** · 반 B **−2.8%**.
그 렌즈는 PRE 를 **자기 손으로 다시 빌드**해 **동일 소스 두 빌드의 노이즈 바닥 0.0%** 를 보였고,
**무해 레이아웃 대조군**(`Apad` = PRE 소스 + 필드만 추가, 루프는 옛것)까지 지어 **레이아웃만으로
picorv32 가 +0.9%, cont_assign_heavy 가 −5% 움직인다**는 것을 보였다 — 즉 **형태별 ±1~2% 는 코드 배치**다.

⚠️⚠️ **반 A 의 최악 케이스는 0 이 아니다 — 적대 리뷰에서 유일하게 살아남은 지적**(NIT · 정직하게 기록).
cont-assign 이 **전부** 딜레이인 설계에서는 사전필터가 아무것도 걸러내지 않고 `delayed_ca_idx[i]` 의
로드 하나만 남는다. ⭐ **검증자가 나보다 좋은 계기를 썼다** — wall 로는 부호가 안 정해져서(같은 설계
25라운드에 +0.24% 와 +0.59% 가 둘 다 나옴) `/usr/bin/time -l` 의 **retired instructions**(±0.02%)로 쟀다:

| 바이너리 | 200 딜레이/0 평문 (min of 7) | |
|---|---:|---|
| PRE | 25.7738 G | 기준 |
| PRE 재빌드 | 25.7774 G | +0.014% (같은 소스 두 빌드의 바닥) |
| **Apad**(필드만 추가·루프는 옛것) | 25.7755 G | **+0.007% ⇒ 레이아웃 비용은 0** |
| 반 A | 25.8666 G | **+0.360%** |
| 반 A+B | 25.9049 G | **+0.509%** |

그리고 **선형**이다 — 딜레이 iteration 당 **+3 명령**, 건너뛴 iteration 당 **−3.3 명령**
(0/200 딜레이 −2.29% · 100/200 −0.16% · 200/200 +0.36% · 400/400 +0.36%). ⇒ **손익분기 ≈ 딜레이 비율 50%**.
실 RTL 은 딜레이 cont-assign 이 희소하므로(picorv32 0/199) 방향은 옳지만 **"항상 공짜"는 아니다**.

**differential**: 리뷰 렌즈가 **649 설계 · vita 호출 ~4,900 회**(3 바이너리 × 3 백엔드)를 돌려
**PRE-vs-POST 차이 0**. 내 코퍼스는 14 설계 × 전 채널 — 원샷 stdout·stderr·exit·VCD 동일 · **3 백엔드**
(interp/vm/native) × FST **42 쌍 동일** · **staged**(vcmp→velab→vrun) 로그·VCD·`.velab` **바이트 동일** ·
**obs 레일**(run.json·results.jsonl) 동일(`wall_s`/`elab_s`/`sim_s` 만 다름 = 빨라진 증거).
**비공허성**: 계측 바이너리가 반 B 의 fast arm(c02 9/9 · c11 11/11 클록 · picorv32 33,661/33,661)과
**`else` arm 둘 다**(c06 concat 3 · c12 100비트 3) 발화를 보였고, 반 A 는 `delaylen` 1~2 인 설계에서
바디가 돌고(c13: 15 호출 × 2 = 30) 0 인 설계에서 **호출은 8~12 회인데 반복이 0** 임을 보였다.

**뮤테이션 12발 — 6 KILLED · 6 SURVIVED, 그리고 살아남은 여섯이 전부 등가임을 따로 증명했다.**
⭐ 그중 **둘은 살아남아야 정상**이고 그 생존이 곧 이 슬라이스의 주장이다: **m2**(반 A 의 필터를 "전부"로)
와 **m11**(반 B 의 fast arm 을 끔 = PRE 경로)이 전 스위트 green 이라는 것이 **"두 반 다 의미를 안 바꾼다"**
의 증명이다. 나머지 넷(`value.width <= 64` · `s.words == 1` · `!s.is_real` · `s.width > 0`)은 각각
**앞선 가드에 의해 함의**된다 — 폭 둘은 `value.width >= net width`(whole-net lvalue) 때문에, 뒤 둘은
`build_plain_scalar` 가 이미 `!is_real && width > 0` 을 요구하기 때문에. 읽기 증명만으로 끝내지 않고
**128/100/65비트 목적지 설계를 지어 m5·m6 바이너리로 실측**해 값이 같음을 확인했다.
죽은 여섯: 필터 반전 · 마지막 인덱스 누락 · `!value.is_real` 삭제 · `changed` 를 상수 true/false 로 ·
술어를 `chunks.first().map(net)` 로 재유도.

**새 회귀 테스트 5건**(`crates/cli/tests/settled_cont_assign_store.rs`) — 등가 뮤턴트는 **못 죽인다**
(그것도 실측했다). 대신 **함의 자체를 핀**한다: fast arm 이 도는 1~64비트 스팬 + 체인, `else` 로 가야
하는 65/100/128비트 whole-net(⚠️ `plain_scalar_dest_of` 는 **폭 상한이 없어서** 이들도 admit 한다 —
막는 건 `value.width <= 64` 하나뿐이다), part-select/concat/배열원소/2-state 목적지, 딜레이 assign 이
첫·중간·끝 인덱스에 흩어진 순서 케이스, 그리고 딜레이가 0개인 설계. 전 셀 iverilog 핀(딜레이 순서는
verilator 까지 2오라클). ⚠️ 그 순서 케이스의 **내 손 예측이 틀렸고 오라클이 고쳤다**.

**적대 2렌즈**: BLOCKING 0. soundness 는 ⓐ `delayed_ca_idx` 가 옛 순회 집합과 **같은 집합·같은 순서**
이고 `cont_assigns` 를 런타임에 바꾸는 곳이 없음(프로덕션 mutation 은 전부 elaborate) ⓑ `write_routed`
의 레인들이 술어로 전부 막히고 꼬리가 **동일 인자의 동일 `write_chunk_word`** 로 환원됨 ⓒ 반환하는
`changed` 가 한 청크에서 `write_lvalue_inner` 의 OR 과 같음을 증명. 낸 지적 셋은 **전부 하드닝 요청**
이었고 적대 검증에서 refute 됐지만, 그중 **드리프트 tripwire 는 채택**했다 — §4.5.351 이 `apply_nba` 에
넣은 것과 같은 `debug_assert`(플랫 store 세 리전이 같은 가드를 들어야 한다).

**곁 발굴(스코프 밖·기록만)**: POST 프로파일의 다음 자리 — `k_schedule_nba_scalar` **4.3%**(NBA 스케줄마다
`chunks[0].clone()`) · `propagate` **2.5%**(델타마다 `Vec` 셋 — 옛 D6 표적, 아직 미측정) · 할당자 잔여.
`drain_range_diags` 1.8% 는 **이미 재고 기각**된 자리다(§5.1-bb — 비용은 문장당 호출 자체).

⚠️ **format_version 불변(29)** — `Scheduler` 는 런타임 상태이고 직렬화 타입이 아니다.

#### 4.5.351 — NBA 적용이 이미 지어 둔 flat store 를 한 번도 안 불렀다 (2026-08-21 · picorv32 −3.9%)

**착수 = 성능.** 다만 그라운딩은 *"무엇이 느린가"* 가 아니라 **"무엇이 거절되는가"**(LOOPROMPT §2)였고,
그 census 가 세 축을 죽인 뒤 남긴 하나다.

**뿌리 = 증명을 만들어 놓고 버렸다.** `k_write_scalar` 는 평범한 스칼라 쓰기를 위한 flat store 이고,
그 함수의 주석이 존재 이유를 적어 뒀다 — *"picorv32 에서 **102,911** 건이 `write_routed` 의
heap/assoc/frame/class 라우팅을 걸어 이미 필요 없다고 증명된 store 에 도달하고 있었다"*.
**그런데 `apply_nba` 는 그것을 한 번도 안 불렀다.** 같은 설계에서 NBA 는 **1,231만 건**(그 24배)이고
**100.0% 가 그 모양**이다. 그리고 증명은 이미 있었다 — `Op::ScheduleNbaScalar` 가
`plain_scalar_dest` 로 게이팅되므로 **스케줄 시점엔 안다**. 다만 `NbaUpdate` 가 값과 offset 리스트만
나르고 분류를 안 날라서 apply 시점에 사라진다. 그래서 술어를 apply 쪽에서 **다시 묻되 같은 함수로**
묻는다(`plain_scalar_dest_of` 추출 · 두 호출자).

**측정**: picorv32 200k 사이클 **2.40 → 2.31 s (−3.9%, 백투백 A/B)** best-of-5 · **stdout·stderr 바이트 동일** ·
keccak 동일·flat(그 설계 NBA 쓰기의 88.8% 가 배열/wide 라 원래 경로).

⚠️ **측정이 가드도 고쳤다.** 첫 프로브는 `offsets == [(0,0)]` 만 봤고 picorv32 에선 강한 가드와
우연히 동일했다(weak_only = 0). **keccak 에선 2,603 건이 샜다** — heap/frame/handle/2-state/array net 이
dead slot 에 flat write 될 수 있는 자리다. 답은 우연히 안 바뀌었지만(내부 테스트가 잡았다)
*"두 설계에서 안전했다"* 는 증명이 아니다. 강한 술어는 picorv32 에서 비용이 0 이다.

**적대 2렌즈.** soundness = **SOUND**: `write_routed` 의 여섯 레인이 술어에 의해 전부 막히고(넷은 청크
모양 · 둘은 **같은 테이블**(frame_local)과 **같은 match**(kind_is_heap ↔ dyn_is_handle)), 살아남는 꼬리가
**동일 인자의 동일 `write_chunk_word`** 로 환원된다. 버려지는 `u.offsets` 는 이 모양에서 `(0,0)` 임이
증명되고 **동시에 하위에서 무시됨**이 증명된다(`chunk_elem`/`chunk_lsb` 가 raw 를 보기 전에 반환).
여섯 부작용(dirty·edge·last_blocking_writer·ca_dirty·VCD·probe)이 **전부 분기점 아래**. `plain_scalar` 가
잠시 비는 창(`compiled_type` 의 take→restore)은 **fail-safe** — 빈 테이블은 `else` 를 고르는데 그것이
변경 전 동작이다. 리뷰의 MINOR 반영 = heap 판정이 **두 match 로 갈라진** 자리에 drift tripwire.

**곁에서 발굴(스코프 밖)**: `crates/cli/tests/` 의 세 파일이 **하드코딩 `/tmp` 경로 12곳**을 쓴다.
전체 스위트 두 개가 겹치면 같은 파일을 동시에 쓰고 `bare_fscanf_writes_dest` 가 깨진다(이번에 실측 ·
격리 재실행은 통과). 제품 결함은 아니지만 flake 클래스다.

**differential 렌즈**: 34 PRE/POST 쌍 · 8축 전부 · **stdout·stderr·exit·VCD·FST·obs 전 채널 동일**
(keccak VCD 1,800,440 바이트 동일). ⭐ **비공허성까지 증명했다** — 계측 바이너리로 재니 fast arm 이
27 설계 중 **22** 에서 발화하고, **기록된 모든 fast-arm apply 에서 offsets 가 예외 없이 `[(0,0)]`**
인 반면 `else` 는 `[(0,17)]` 류를 본다(즉 "버려지는 offsets 는 항상 (0,0)" 이 공허하게 참인 게 아니다).
곁 NIT 반영: `plain_scalar_dest_of` 의 "scalar" 는 **whole-net** 이지 1비트가 아니다(128비트 reg 30건이
fast arm 을 탔고 출력 동일) — 소비자가 이름만 보고 한 워드를 가정하면 틀린다.

⚠️ **전체 스위트 실패 1건은 제 하네스 문제였다** — 게이트 런이 도는 중에 두 번째 스위트를 띄웠고
`bare_fscanf_writes_dest` 가 **하드코딩 `/tmp` 경로**를 써서 두 프로세스가 같은 파일을 동시에 썼다.
격리 재실행·단독 재실행 모두 통과(5,679 green).

#### 4.5.350 — §2 음수 상수를 먹는 소비자 셋 (2026-08-20 · 5,679 green · format 29 불변)

**착수 = 큐 2번**(*"음수 상수가 소비자에서 조용히 먹힌다"*). 3-오라클 census 로 스코프를 먼저 갈랐고,
그 census 가 **기록에 없던 세 번째 소비자**(unpacked 배열 크기)를 찾아냈고 **불가침 축 넷**(part-select
계열 — iverilog vs verilator 불일치)을 잘라냈다.

**ⓐ replication count `{W{1'b1}}` (W<0) — 조용한 255 → LOUD.**
카운트가 2의 보수 비트 패턴으로 엔진에 갔다: `W=-56` → 4294967240 복사 → 타깃 폭으로 절단 → **255,
exit 0**. `$bits({W{1'b1}})` 는 실제로 **4294967240** 을 답했다. 두 오라클 모두 거부한다.
읽기를 **둘** 둔다 — `const_index_value`(lowered 노드, 부호 인식)와 `const_bound_signed`(AST,
자기결정 walk). 한쪽만으로는 가족이 안 덮인다: 파라미터는 `Const` 로 접히고, `{(2-3){…}}` ·
`{(W-4){…}}` 는 `Add`/`Sub` 로 남아 u32 fold 가 **saturate 해 0** 이 되므로 옛 zero-체크가
*"count of zero"* 라고 **-1 에 대해** 말하고 있었다.
⚠️ **부호는 자기결정 폭에서 읽어야 한다** — `{(4'd0-4'd1){1'b1}}` 는 두 오라클에서 **15 복사(255)**
이고 `{(4'sd0-4'sd1){1'b1}}` 는 **거부**다. 폭-무제한 fold 는 **둘 다 -1** 이라고 답한다. 무제한으로
읽었으면 맞는 설계를 false-reject 했을 것이다(경계 두 칸 전부 테스트에 핀).

**ⓑ 오름차순 음수 range bound `logic [-3:0]` — W3056 클램프+거짓 문구 → correct-support.**
`[-3:0]`·`[-56:0]`·`[-3:1]`·`[-8:-1]`, 그리고 **`parameter W=0; logic [W-1:0]` 관용구**가 전부
폭 1 로 클램프되고 있었고 그 경고 문구(*"parameterized range underflowed (param value 0?)"*)는
파라미터가 없는 리터럴 철자에도 나왔다 — **사실이 아닌 진단**. IEEE §7.4.2 는 부호와 무관하게
`|msb-lsb|+1` 이고 두 오라클이 그렇게 한다(2·57·5·8·2).
근인은 `declared_neg_lsb` 의 *"`msb<0`, `lsb≥0` 은 degenerate 파라미터 언더플로"* 라는 **가정**이었다.
그 가정은 §4.5.228 이 내림차순만 지원할 때 세운 것이고, 실측하면 그 칸도 오라클은 2비트를 낸다.
방향이 어느 bound 가 내부 비트 0 인지를 정하는 것이지 **어느 쪽이 음수인지가 정하는 게 아니다**.
저장은 정규화(`[0:w-1]` 오름차순 · `[w-1:0]` 내림차순 — 동결 `u32` 필드), 선언 bound 는 새 sparse
map `net_decl_asc_lsb` 에 싣고 `norm_offset_for_net` 이 `internal = l - raw` 로 읽는다(내림차순
map 의 `raw - l` 의 거울). VCD `$var` 는 선언 쌍 그대로. PART select 는 내림차순 쌍둥이와 **같은
이유로 loud** 유지.
곁: 폭 캡 초과가 **조용한 `.min(MAX_NET_WIDTH)`** 였다(내림차순도 마찬가지) — 이제 두 방향 모두
평범한 `exceeds the v1 cap` 에러다(silent-wrong → loud).

**ⓒ unpacked 배열 크기 `logic q[-3]` / `q[0]` — 조용한 1워드 → LOUD.** (census 발굴)
`.max(1)` 바닥이 먹고 있었다. 경계 양쪽 실측: **0 과 모든 음수는 두 오라클 거부 · 1 은 수용**.

**옛 동작을 의도로 고정하던 테스트 2건을 갱신**(삭제 아님·사유 주석): `[W-1:0]` W=0 이
*"underflow 경고 + 1비트"* 를 유지한다고 적혀 있었다. 두 오라클은 2비트다.

**곁에서 발굴(스코프 밖·§2 기록)**: `wire [7:0] a = '1;` 이 `00000001`(두 오라클 `11111111`) —
**net 선언 초기화의 fill 리터럴이 1비트로 sizing** 된다. `assign a = '1;` 과 `reg [7:0] b = '1;` 은
정상이라 자리가 하나로 좁혀진다. 음수와 무관한 pre-existing.

**적대 2렌즈 · 라운드 2로 종료.** differential = **회귀 0**(replication 카운트 경계 14칸 — const 함수·
캐스트·삼항·genvar·`#(-3)` 오버라이드·`defparam` · 쓰기 전 형태 · staged `vcmp→velab→vrun` · 부호 있는
net · 총 55칸 추가). soundness = **BLOCKING 1**(dyn/queue 원소가 넓은 폭만 받고 사이드맵 기록은
early-`continue` 로 건너뛰어 **warn→조용한 `x`** 하강) + 두 렌즈 수렴 1(`dim_coord` 의 `debug_assert`).
라운드 2 수정 셋은 79칸 census 를 **한 칸도 움직이지 않았다**.

**뮤테이션 15/16 사망.** 생존 하나는 `norm_k_sub` 의 unsealed 음수-k arm — 그 arm 은 도달 시
`k.max(0)` 이 **틀린 값**이 되는 방어 가드라 유지하고 근거를 코드에 적었다. ⚠️ 배터리가 **테스트 공백
넷**을 찾아 메웠다(오름차순 indexed part-select · dyn 저장 clamp · packed 음수 dim 비트선택 · string
**변수** replicate 두 arm). ⚠️ 그리고 *"읽기를 둘 두었다"* 는 **주석의 주장을 반증**해 한 쪽을 지웠다 —
lowered 읽기가 답하는데 AST self-det 읽기가 못 답하는 셀을 배터리도 프로브도 찾지 못했고(인라인
치환은 AST 를 이미 리터럴로 바꾼다), 근거가 무너진 코드는 남기지 않는다.

전 스위트 **5,654 → 5,679 green** · clippy 양축 0 · fmt 0 · 제품 구성 0 · format_version 29 불변.

#### 4.5.349 §2 정확성 큐 — 런타임 mixed-real 변환 경계 (2026-08-20) ✅

**한 줄**: IEEE §11.8.1 은 정수→real **변환 경계를 self-det** 로 규정하는데, 엔진은 mixed Binary 의
두 피연산자에게 노드의 문맥 폭을 주고 있었다. `sim_ir::selfwidth` 가 real const 를 `{width:64}` 로
답하므로 그 문맥이 64 가 되고 정수 형제가 거기서 평가됐다 — `logic signed [3:0] s = -8` 에서
`1.0 + (-s)` 가 두 오라클 **−7** 인데 **9**, `1.0 + (s+s)` 가 **1** 인데 **−15**.

**선행조건을 먼저 세운 것이 이 슬라이스의 형태를 정했다.** 앞 반복(그라운딩)이 *"값으로 real 을
판정해 재평가하면 mixed 케이스에서 `$random` 이 두 번 뽑혀 RNG 시퀀스가 바뀐다"* 와 *"self-det 평가 후
resize 로 바꾸면 `eval_ctx` 가 문맥을 **하위로** 미는 성질이 사라져 중첩 정수식이 광역 회귀한다"* 를
**금지 우회로 적어 두었다**. 둘 다 코드가 훨씬 짧으므로, 그 기록이 없었다면 골랐을 것이다. 남는 답은
**정적 술어**뿐이고 엔진엔 그것이 없었다 ⇒ `elaborate::expr_is_real`(성숙한 IR 걷기 — `$signed`
투명성·dyn-array real 원소 arm 포함)을 **`sim_ir::realness` 로 리프트**했다. 공유 방식은 발명하지
않았다: `selfwidth` 가 이미 `call_ret` 콜백으로 같은 문제를 풀고 있어 **`child` 콜백**으로 그 패턴을
따랐다 — elaborate 는 재귀(아레나가 lowering 중에 자란다), 엔진은 `WidthTable` 옆의 메모 테이블.
`real_math_arity` 도 함께 옮겼다(`SysFuncId` 의 성질이고 호출자가 하나였다).

**⚠️ 적대 리뷰가 BLOCKING 을 냈다 — 그리고 그 모양이 교훈이다.** 공유 규칙은 삼항을 *어느 한 arm 이
real 이면 real* 이라 답하는데, **엔진의 삼항은 취해진 정수 arm 을 변환하지 않았다**. PRE 에서는 양쪽
피연산자가 똑같이 64비트 문맥을 받아 **두 오차가 상쇄**되어 `(sel ? 1.0 : (s+s)) - (s+s)` 가 우연히
정답 0 이었는데, 내 변경이 형제 쪽 문맥만 없애자 상쇄가 깨져 **−16**(correct→silent-wrong)이 됐다.
⇒ 정적 주장이 **값에 대해 참이 되도록** 삼항이 실제로 real 을 생산하게 고쳤다(이것이 `sel ? (-s) : 2.0`
= 두 오라클 −8.0 이라는 pre-existing 셀도 함께 닫았다). 곁으로 리뷰가 **비교 연산자**를 짚었다 —
그 자리도 §11.8.1 대상인데 안 고쳐서 `1.0 + (-s)` 와 `1.0 > (-s)` 가 **한 설계 안에서 불일치**했고
`if` 가 다른 가지를 탔다. 같은 규칙으로 확장했다.

**뮤테이션 배터리** 11 종 · **10 사망 · 1 등가(논거를 코드에)**. 생존한 것은 *"`Pow` 를 real 전파
목록에서 뺀다"* 인데, 이는 **구성상 도달 불가**임이 증명됐다 — elaborate 가 `**` 를
`expr_is_real(lhs) || expr_is_real(rhs)` 조건으로 `SysFunc::Pow` 로 desugar 하고, 그것이 이 arm 이
발화할 조건과 **정확히 같으므로** real 피연산자를 가진 `Binary{Pow}` 노드는 IR 에 존재하지 않는다.
목록은 *현재의 lowering* 이 아니라 *언어 규칙* 을 적은 것이므로 남긴다. ⚠️ 배터리 설계가 킬러 없는
변이 셋(부호 반전된 real 피연산자 · `real d[]` 원소 · 사이드카 스레딩)을 찾아 전부 메웠고, 사이드카
건은 **라이브러리 레벨로는 못 잡는다**(그쪽은 모든 사이드카가 비어 있어 이름과 다른 것을 잰다)는
지적대로 CLI 경유로 지었다.

**측정** — 68칸 3-way(iverilog / PRE=`2075449` / POST) + verilator 교차 + **4 백엔드**(기본·interp·
vm·native) 일치: **회귀 0 · WRONG→FIXED 11 · 동일 55**. S-섹션(순수 정수 Binary 는 여전히 문맥을
받는다 — 이 변경이 깨뜨리면 안 되는 절반) 7칸 전부 불변. 성능: picorv32 +0.1%, 정수 산술 루프 +0.5%.
전 스위트 **5,654 green** · clippy 양축 0 · fmt 0 · 제품 구성 0.

**§2 로 보낸 것**(전부 pre-existing · PRE==POST · 2-오라클 합치): `automatic`(framed) real 함수를
**직접 피연산자**로 쓰면 여전히 넓힌다(`fa(1) + (-s)` = 9 / −7 · 곁: `{fa(1),1'b0}` 이 조용히 통과) ·
package/class 함수도 같은 구멍 · **단순 대입 경계는 아직 문맥-결정**(`real r; r = (-s);` 가 8.0 / −8.0).
즉 이 슬라이스가 닫은 것은 **Binary(산술·비교)와 Ternary** 이고, 공유 규칙의 `Call` arm 은 그 형태에
도달하지 않는다는 것이 실측됐으므로 주석을 **전달 범위대로** 다시 썼다.

---

#### 4.5.348 §2 정확성 큐 — `**` 지수의 부호 (2026-08-20) ✅

**한 줄**: 상수 도메인이 지수를 나르는 `i64` 는 **컨테이너**이지 값의 타입이 아닌데, 그 컨테이너의
부호 비트를 값의 부호로 읽고 있었다.

**재센서스가 기록된 두 항목 중 하나를 지웠다.** 「폭-미상 wrapping 지수」는 §4.5.339 가 *"해소는 거부가
아니라 폭 모델 완성 — `const_decl_wsign` 이 다중 packed 를 답하게"* 라고 적어 둔 항목인데, **§4.5.345 가
정확히 그것을 했다**. 재현하니 `3 ** (m - 8'd250)` 이 두 오라클과 같은 59049 였다. 그 슬라이스는 자기가
무엇을 닫았는지 몰랐고, 재센서스가 아니었으면 이미 고친 것을 다시 고쳤을 것이다.

**남은 뿌리** — `64'd0 - 64'd8` 은 **부호 없는** 뺄셈이라 18446744073709551608 인데, 상수 도메인이 i64
컨테이너의 부호 비트를 보고 −8 이라 판단해 IEEE 음수-지수 표를 적용, **조용히 0** 을 냈다(iverilog ·
verilator · **vita 자신의 런타임** 전부 926288481). 수정은 지수의 **부호가 값과 함께 다니게** 하는
것(`const_pow_exponent_selfdet` 이 `(값, 부호)` 를 반환)이고, 크기가 i64 도메인 밖이면 **정직한 loud** 다.
곁으로 밑수 0/±1 은 지수의 **크기가 아니라 패리티**만 쓰므로 도메인 밖 지수에서도 답해야 한다.

**⚠️⚠️ 모듈러 fold 를 지어서 되돌렸다 — 적대 2렌즈가 독립적으로 같은 BLOCKING 을 냈다.**
지수 부호를 고치면 거대 지수의 답은 필연적으로 모듈러이고, 엔진이 이미 그 알고리즘(square-and-multiply)을
쓰므로 상수 도메인에도 놓았다. mod 2^64 는 **문맥이 64비트 이하일 때는 정확**하고 두 오라클로 6칸을
확인했다. 그러나 **모듈 스코프 fold 에는 문맥 폭이 없다** — `localparam [127:0] P = 3 ** 41` 이 **이미
잘린** 64비트 값을 `resize_bits` 로 zero-extend 하므로 잘려 나간 상위 비트를 복구할 수 없다:
**loud→silent-wrong**(96비트 타깃에선 silent-wrong→**다른** silent-wrong = 명시적 금지). 되돌리는 과정에서
내 수정이 만든 **두 번째 회귀**도 잡았다 — 밑수 ±1 을 도메인 검사 뒤로 밀어 두 오라클의 1 을 decline 으로
떨어뜨렸다.

**곁: 파서 패닉**(pre-existing · 이 슬라이스의 스윕이 발굴 · 별건). `hdl-parser` 의 `try_const_index` 는
generate-array 인덱스용 헬퍼인데 파서가 보는 **모든 이항식**에 돌면서 unchecked 산술을 했다 — 평범한
`localparam integer L = 3037000500 * 3037000500;` 하나가 *"attempt to multiply with overflow"* 로 **런
전체를 패닉**시켰다. 크래시는 loud 아래다. `checked_*` 로 바꿔 decline(=인덱스가 아님)하게 했고, generate
인덱스는 그대로 접힌다.

**뮤테이션 배터리** 11 종 · **8 사망 · 3 등가(논거를 전부 코드에)**. 생존 셋이 각각 무언가를 증명한다:
`const_binop` 의 Pow 팔에 옛 부호-읽기 경로를 되돌리는 변이가 **전 스위트를 통과한다** — 그것이 곧
*"세 호출부가 전수다"* 의 증명이고, decline 은 네 번째 자리가 생겼을 때를 위한 fail-closed 가드로 남는다.
`u32::try_from(b as u64)` ↔ `try_from(b)` 는 u32 에 들어가는 값은 i64 로도 음수가 아니므로 진짜 등가.
파서의 `checked_neg` 는 `i64::MIN` 이 십진 리터럴로 도달 불가(`9223372036854775808` 이 i64 로 파싱되지
않아 피연산자가 먼저 decline)라 등가. ⚠️ 배터리가 **파서의 두 번째 fold 자리**(`const_lit`)도 드러냈다 —
그쪽은 **이미 checked 였고 그 이유까지 주석에 적혀 있었다**. 같은 규칙의 두 철자 중 하나만 지켜지고
있었던 셈이고, 주석이 이제 서로를 가리킨다.

**측정** — 72칸 3-way(iverilog / PRE=`590084e` / POST) + 3-오라클 grounding: **회귀 0 · wrong→LOUD 5**
(조용한 0 넷 + 파서 패닉 하나) · 동일 58 · 정직한 decline 9. 전 스위트 **5,647 green** · clippy 양축 0 ·
fmt 0 · 제품 구성 0.

**§2 로 보낸 것**: ⓐ **i64 도메인의 오버플로 decline 은 `+`·`*`·`**` 에 걸친 CLASS** 이고, ⚠️ **문맥 폭
없이 모듈러로 접으면 안 된다**는 실측 근거가 이제 붙어 있다(선행조건 = 폭-인식 모듈 스코프 fold ·
vita 런타임이 전부 정확하므로 오라클은 자기 안에 있다) ⓑ 크기 0 인 unpacked 차원을 조용히 받는다
(두 오라클 거부) ⓒ untyped `localparam` 의 거대 `**` 는 **iverilog 가 행(hang)** 하므로 그 철자엔 오라클이
없다(10분 100% CPU 확인 후 종료 · verilator 단독 판정만 가능).

---

#### 4.5.347 §2 정확성 큐 — top-level 자기결정 위치 (2026-08-20) ✅

**한 줄**: `const_eval_in_scope`(폭-무제한 모듈 스코프 fold)가 IEEE §11.6.1 Table 11-21 이 **자기
자신으로 sizing 한다**고 규정한 자리를 답하고 있었다.

**스코프를 정한 것은 3-오라클 census 다.** §2 항목은 *"top-level const 문맥 전반"* 이라고 넓게 적혀
있었는데, 손댈 수 있는 것은 **iverilog 와 verilator 가 합치하는 자리**뿐이다(LOOPROMPT 0a). 18칸을
셋으로 재니 경계가 선명했다:

| | iverilog | verilator | vita(PRE) | |
|---|---|---|---|---|
| 비교 5종 · 논리 3종 · 삼항/generate-if 조건 | 같음 | 같음 | **다름** | **9칸 = 착수 대상** |
| untyped `localparam L = 4'd15+4'd1` | 16 | 0 | 16 | **갈림 — 불가침** |
| `repeat` 카운트 | 2 | 18 | 2 | **갈림 — 불가침** |
| 리덕션 `\|e` | 0 | 0 | **loud** | 정직한 loud = ②급 갭 |
| bound·count·delay·case·width 소비자 | 같음 | 같음 | 같음 | §4.5.344 가 이미 닫았다 |

**수정** — 넷 중 셋은 §4.5.343 이 만든 `const_int_selfdet` 한 줄 치환이다(삼항 조건 §11.4.11 ·
`!e` 피연산자 · `const_truth_in_scope`). 넷째(비교·등가·논리)는 **결과가 문맥을 받는가**로 갈라야
해서, 그 분할을 `binop_result_is_context_determined` 라는 **와일드카드 없는 exhaustive match** 하나에
적었다 — 새 `BinOp` 가 생기면 컴파일이 깨지므로 조용히 한쪽에 붙을 수 없다. 26 변종이 9+5+12 로
정확히 덮이고, 그 전수성이 곧 **종료 증명**이다(비교 노드가 `eval_const_env_at` 의 fallthrough 로
새어 `const_eval_in_scope` 을 같은 노드로 다시 부르는 경로가 존재하지 않는다).

**적대 리뷰가 낸 것**(BLOCKING 0):

- **이웃 불일치 둘 — 둘 다 이 슬라이스가 만들었다.** ⓐ 시프트 카운트는 폭-인식 twin 이 이미 다섯을
  전부 자기결정으로 돌리는데 모듈 스코프 arm 은 `**` 만 돌렸다. 비교가 twin 으로 리다이렉트되기
  시작하자 **같은 부분식이 비교 아래에서만 정답**이 됐다(`8'd1 << (4'd15+4'd1)` 이 홀로 65536, `>`
  아래에서 1 · 두 오라클 1). ⓑ `==?`/`!=?` 의 whole-node 헬퍼가 LHS 를 **무제한 walk 로** 읽고 있어,
  자기 가족 12 중 혼자 옛 평가기를 쓰고 있었다(`===` 는 같은 편집에서 고쳐졌다).
- **`const_self_width` 에 Replicate arm 이 없다.** replication 피연산자가 폭을 미상으로 만들어
  폭-인식 walk 가 무제한 도메인으로 **degrade** 하므로, 이 슬라이스가 선언한 규칙이 네 자리 전부에서
  무력화됐다(`({2{4'd15}} + 4'd1) > 4'd0` 이 두 오라클 0 / vita 1). Concat 쌍둥이는 이미 있었고, 그
  대비가 구멍을 드러냈다.
- **평범한 env twin(`eval_const_env`)이 마지막으로 술어와 모순**이었다(도달 불가·잠재) — 같은 규칙으로
  정렬.
- 트리 전역에 같은 12-op 목록의 **손수 짠 사본 넷**이 더 있다(`expr_ctx.rs` ×2 · `hoist/arms.rs` ·
  `sim-ir::selfwidth`). 전부 읽었고 **일치**하며, const↔런타임 발산은 열린 게 아니라 **닫혔다**
  (`((4'd15+4'd1) > 4'd0)` 을 파라미터와 런타임 양쪽으로 쓴 설계가 PRE 1/0 갈림 → POST 0/0 합치).
  주석의 *"ONE spelling"* 주장은 const 도메인으로 범위를 좁혀 다시 썼다.

**⚡ 성능** — 비교/`!`/삼항조건 fold 가 **~3× 느려졌다**(값은 전부 정답). `const_int_selfdet` 이
평가 전에 폭·부호로 트리를 두 번 더 걷고 비교 arm 이 lhs·rhs 를 각각 다시 걷는다 = **6 walk vs 옛 2**.
병리적 실측 3.7×(이중 generate-for · 60항 체인이 bound), **컨트롤(같은 크기에 `+`) 1.00×** 라 비용은
정확히 리다이렉트다. ⚠️ **현실 설계에서는 측정되지 않는다** — picorv32 elaborate 0.030 → 0.030 s ·
평범한 3,000회 generate-for 0.116 → 0.162 s. 처방 후보 둘과 수치는 §2 에 등재.

**뮤테이션 배터리** 13 종 · **11 사망 · 1 도달불가 생존 · 1 진짜 무변화 · 1 BUILD-FAIL(설계대로)**.
BUILD-FAIL 이 이 슬라이스의 설계 증명이다 — 연산자를 한쪽에서 빼면 exhaustive match 가 **컴파일을
깨뜨리므로**, *"조용히 반대편에 붙는"* 변이는 아예 만들 수 없다. 생존 둘은 각각 논거가 있다:
`eval_const_env` 가드는 리뷰가 이미 **도달 불가**로 판정했고(주석에 기록), Pow 를 양쪽 arm 에 두는
변이는 첫 arm 이 이기므로 **진짜 무변화**다.

⚠️⚠️ **절차 사고 — 배터리가 오염됐고, 그것을 잡은 방식이 소득이다.** 첫 런에서 *"무변화여야 할
변이가 죽었다"* 는 모순이 나왔다. 해명하는 대신 `git diff` 로 트리를 보니 **배터리의 스냅샷 집합이 두
파일로 하드코딩**돼 있었고, 세 번째 파일(`const_real.rs`)을 건드리는 변이가 **런의 나머지 내내 적용된
채로 남아** 이후 판정이 전부 거짓 KILL 이었다. 복원 집합을 **뮤턴트 목록에서 유도**하도록 고치고
여섯 건을 재판정했다(그중 하나가 생존으로 뒤집혔다). 규칙은 ENGINEERING_RULES 에 병합.

**측정** — 193칸 3-way(iverilog / PRE=`f0c3cec` / POST) + 18칸 3-오라클 census: **회귀 0 ·
WRONG→FIXED 15 · 동일 154 · 정직한 decline 24**. 직전 슬라이스가 *"캐스트 안쪽은 맞는데 bare 는
틀리다"* 로 기록해 둔 셀(M12)이 이번에 FIXED 로 넘어와 그 자기 불일치가 닫혔다.
전 스위트 **5,642 green** · clippy 양축 0 · fmt 0 · 제품 구성 0.

---

#### 4.5.346 §2 정확성 큐 — 크기 캐스트의 상수 도메인 (2026-08-20) ✅

**한 줄**: 크기 캐스트가 피연산자를 **무제한 도메인에서 접고 나서 절단**하고 있었다. 절단이
근사이므로 그 arm 은 *"부호의 두 해석이 일치할 때만 접는다"* 는 자기제약을 달고 있었고, 그 제약이
평범한 좁힘 캐스트 20종을 거절했다. 피연산자를 **먼저** `max(self, N)` 으로 좁히면 절단이 근사가
아니게 되어 제약이 통째로 필요 없어진다.

**재현(HEAD, 24칸)** — 8 ok / 16 DIFF. DIFF 의 실체는 **silent-wrong 1 + loud 15** 였다:
`4'((4'd8+4'd8)/4'd3)` 이 4비트 합 0 을 16 으로 읽어 3 으로 나눈 **5**(iverilog 0), 그리고
`8'(255)`(iv −1) `4'(9)`(−7) `1'(3)`(−1) `8'(P)`(200) `64'(-1)`(−1) `W'(511)`(−1) 등이 전부 loud.

**수정** — `const_eval_cast` 의 Size/Named 팔을
`eval_const_assign(operand, …, Some((N, const_signed_env(operand))))` 로 라우팅. §4.5.316 이 핀한
`max(self, N)` 규칙 그대로이고, **§4.5.345 가 `eval_const_env` 의 Cast 팔에 이미 깐 것과 같은 철자**라
두 스펠링이 갈릴 수 없다. 라운드 2 에서 **`const_size_cast` 한 함수로 합쳐** 두 arm 이 공유한다.

**적대 리뷰가 BLOCKING 둘을 냈다**(두 렌즈가 각각 하나씩, 서로 다른 진입점):

- **N > 64 에서 값을 답한다.** 피연산자는 여전히 64비트에서 돌므로(`ctx = …min(64)`) bit 63 위의
  carry 가 사라진다 — `65'(64'hFFFF_FFFF_FFFF_FFFF + 1) >> 64` 가 **두 오라클 1 / POST 0**. PRE 의
  `!(1..=64).contains(&n)` 이 바로 그 가드였고 내가 지웠다. 복원하고, `N == 64` 에는 §4.5.345 의
  `const_placement_env` 와 **같은 적합성 규칙**(unsigned + 최상위 비트 = decline)을 붙였다.
- **캐스트 안의 whole-node 비교가 사라졌다.** `const_eval_in_scope` 의 Binary 팔은 문자열 동등
  (§6.16 은 텍스트를 비교한다 — 어느 피연산자도 i64 값이 없다)과 `==?` x/z 패턴(§11.4.6)이라는
  **노드 전체 특례 둘**을 들고 있는데, 폭-인식 walk 는 비교의 **피연산자로 재귀**하므로 그것을 가린다.
  `8'(MODE == "Y")` 가 iverilog 1 에서 decline 이 되고, **폭 위치에서는 조용한 1비트 넷**이 된다.
  `const_compare_special` 로 빼내 폭-인식 비교 분기가 **로컬 바인딩이 없을 때만** 참조하게 했다.
  ⚠️ **노드를 통째로 위임하면 안 된다** — `8'((4'd15+4'd1) > 4'd0)` 이 폭-정직 0 에서 무제한 1 로
  되돌아간다(그 셀이 지금 iverilog 와 맞는 것 자체가 폭-인식 경로가 캐스트 안에서 쓰인다는 증명이자
  추가 FIXED 2칸이다).

**⚠️⚠️ 그리고 게이트가 두 렌즈와 147칸 스윕이 전부 못 본 것을 잡았다.** 특례를 헬퍼로 빼내면서
원본에서 **특례와 일반 경로가 공유하던 한 번의 계산**(`let a = const_eval_in_scope(lhs)?`)을 헬퍼가
복제했고, 그래서 **모든 이항 노드가 좌변을 두 번** 접게 됐다 = 좌편향 200단 인덱스에서 **2^n**.
밀리초짜리 테스트가 **240초 타임아웃**이 됐다. 값은 어디서도 틀리지 않으므로 값 차분에는 보이지
않는다. 그리고 nextest 요약은 *"5,631 passed, 1 timed out"* 이라 `FAIL` 만 grep 하면 초록으로 읽힌다 —
교훈 둘 다 ENGINEERING_RULES 에 병합.

**측정** — 147칸 3-way(iverilog 13.0 / PRE=`7289ce8` / POST): **회귀 0 · WRONG→FIXED 4 ·
LOUD→CORRECT 23 · 동일 97 · 정직한 decline 23**. 전 스위트 **5,633 green**(5,627 +5) · clippy 양축 0 ·
fmt 0 · 제품 구성 0. soundness 렌즈가 **const↔런타임 발산이 하나 사라졌다**는 것도 측정했다(같은
텍스트를 파라미터로도 런타임으로도 쓰는 설계에서 PRE 5/0 → POST 0/0).

**뮤테이션 배터리** 10 종 · **9 사망 · 1 등가(논거를 코드에)**. 생존 둘을 각각 증명했다:
*"특례 위임의 `env.is_empty()` 가드를 없앤다"* 는 **등가가 아니었다** — 동명 모듈 파라미터를 가리는
body-local 위의 와일드카드 비교에서 변이가 **파라미터의 답 0** 을 내고(지역변수의 답은 iverilog 1,
가드가 있으면 정직한 loud) 변이 빌드로 증명해 킬러를 넣었다. *"Prim 팔도 같은 퍼널로"* 는 **진짜
등가**다 — 좁히는 `byte'`/`shortint'` 아래의 나눗셈·나머지·시프트, 64비트 피연산자, wrapping 합까지
13 판별자를 지었는데 하나도 안 갈렸다(고정 폭으로의 최종 coercion 이 이미 넓은 평가를 줄인다).

**§2 로 보낸 것**(전부 리뷰 실측 · pre-existing · PRE==POST): 음수 replication count 가 진단 없이 255
(iverilog 는 거부) · 음수 range bound 가 W3056 로 1비트 클램프(iverilog 57)이고 경고 문구가 사실과
다름 · `cast_size_bits` 가 깊이를 0 으로 고정해 size 식 안의 자기참조 default 가 스택 오버플로 ·
그리고 **bare lane 의 무제한 fold**(캐스트 안쪽은 이제 폭-정직인데 `((4'd15+4'd1) > 4'd0)` 은 여전히
1) — 이것이 새 착수 순서 1번이 됐다.

---

#### 4.5.345 §2 정확성 큐 — 상수함수 해석기의 선언 바인딩 (2026-08-20) ✅

**한 줄**: 한 뿌리 = **해석기가 *모르는 것*(미fold 초기화 · 미상 폭)을 값으로 만들었다**. 고친 자리는
넷이고, 적대 리뷰가 그 중 하나를 *더 위로* 밀었다.

**재센서스가 큐 항목을 셋으로 갈랐다** (착수 전 HEAD 재현 — LOOPROMPT §1):

| 기록 | HEAD 실측 |
|---|---|
| body-local init 조용한 0 | ✅ 재현 (`int x = 8'(5); g = x;` → vita 0 / iverilog 5) |
| body-decl `int t = f();` 크래시 | ❌ **소멸** — 지금은 7 = iverilog. 진짜 크래시는 **자기참조** `int t = g();` (SIGABRT 134) |
| 폭-0 타깃 마스킹 | ✅ 재현하나 **기록된 메커니즘이 오진**(아래 H4) |

**수정**

- **H1 읽기 arm** — `envw` 에 선언돼 있으나 `env` 에 없는 단일 세그먼트 이름을 읽으면 `None`(loud).
  이 가드가 없으면 미바인딩 지역변수가 **동명 모듈 파라미터로 폴백**한다(실측: `int X = …` 가 99 —
  참조가 가리키는 것과 다른 객체가 답한다).
- **H2 `bind_const_decl`** — 중복돼 있던 선언 루프 둘(함수-레벨 `body_decls` · 블록 `decls`)을 한 자리로.
  *초기화 없음*→0 · *fold 됨*→값 · **미fold→미바인딩**. 옛 `unwrap_or(0)` 은 앞의 둘을 합쳤다.
  미바인딩은 **읽을 때만** loud 이므로 초기화가 죽은 지역변수는 그대로 접힌다(측정 42/9/8).
- **H3 본문 깊이** — `body_decls` 를 `depth + 1`(본문이 받는 그 깊이)에서 평가. 자기참조 초기화가
  같은 AST 노드를 **고정 깊이로** 재진입하던 것이 깊이 캡에 묶인다 = 크래시 → E3009.
- **H4 multi-packed 폭** — `const_decl_wsign` 이 추가 packed 차원을 **곱한다**(`bit [1:0][3:0]` = 8비트).
  ⚠️⚠️ **기록된 진단(*"`w.max(tw)` 가 `tw=0` 을 실폭처럼 쓴다`"*)대로 고친 첫 판이 회귀였다** —
  폭-0 을 무제한 도메인으로 degrade 시키면 `bit [1:0][3:0] tt = 8'd100 * 8'd100` 이 iverilog 16 에서
  **10000** 이 된다(**correct→silent-wrong**). `max(self, 0)` = RHS 자기 폭은 **옳은** degrade 였고
  (참 타깃이 RHS 보다 좁거나 같은 모든 칸에서 정답), 진짜 결함은 *계산 가능한 곱-폭을 declined* 한
  것이었다. PRE-3-way 가 이것을 잡았다 — iverilog 만 보는 차분으로는 둘 다 "vita 가 틀렸다" 로 읽힌다.
- **H5(리뷰 라운드 2) `const_placement_env`** — differential 렌즈의 major: *참값이 0 인 미fold 초기화는
  옛 `unwrap_or(0)` 이 우연히 정답이므로 loud 화가 correct→loud 다*(`{2{2'b00}}` iverilog 7 / PRE 7 /
  POST loud). 되돌리는 대신 **위로** 닫았다 — concat·replication·SIZE 캐스트는 자기결정 **비트 배치**라
  이미 있는 **carry-free wide folder**(`fold_self_bits` — >64비트 파라미터를 짓는 그 코드)에 이름
  해석기를 넘겨 접는다. 산술은 두 번 쓰지 않는다. 모듈 스코프의 손수 짠 Concat 루프도 **같은 헬퍼**로
  교체 ⇒ 두 스코프가 한 철자(그 전엔 본문이 `{2{4'hA}}` 를 접고 모듈 스코프는 loud). SIZE 캐스트는
  §4.5.316 이 핀한 `max(self, N)` 규칙 그대로 **대입 퍼널**로 라우팅했다.

**측정** — 89칸 PRE-3-way(iverilog 13.0 / PRE=`3d46115` / POST):
**회귀 0 · WRONG→FIXED 22 · LOUD→CORRECT 6 · wrong→LOUD 8(그 중 2 는 CRASH→loud) · 동일 44 ·
pre-existing 미변 9**(마지막 9 는 전부 PRE 와 같은 정직한 loud — 아래 라운드 3 참조). 전 스위트 **5,627 green**(5,616 +11) · clippy 양축 0 · fmt 0 · 제품 구성 0.

**적대 리뷰 2렌즈가 수렴했다** — differential 과 soundness 가 서로 다른 진입점에서 **같은 두 발견**을
냈다(고신뢰): ⓐ 위 H5 ⓑ **decl-init 호출 사슬의 깊이 캡 64**. ⓑ 는 유지 판정 — 같은 설계의 *문장* 철자
(`f<k> = f<k+1>() + 1;`)가 **PRE 에서도 이미 loud** 였으므로 64 는 기존 합의 상수이고, H3 은 decl 자리를
문장 자리에 맞춘 것이며 그 과금이 곧 크래시를 없앤다. 무손실 대안(재진입 시에만 과금)은 ROADMAP §2.

**소득 셋 더** (전부 리뷰 실측): 옛 decline 핀 `cast_size_selfdet_const_declines_loud` 가
**LOUD→CORRECT 로 상승**해 값 핀으로 재작성됐다(`4'd3 ** ((4'd9+4'd8)'(2))` = iverilog 1) ·
델리게이션 arm 의 *"두 맵은 lockstep 쌍"* 주석이 **양방향으로 거짓**임이 증명돼 재작성 ·
`remove` 의 근거 주석이 **도달 불가**(블록-로컬 shadow 게이트가 먼저 잡는다)임이 측정돼 방어적 조치로
재기술. 남은 잔여 넷(깊이 캡 · 4-state 무초기화 · packed 곱 u32 오버플로 패닉 · 선언 폭 모델 셋)과
**다음 슬라이스의 정답**(`const_eval_cast` 의 Size arm 을 같은 폭-인식 퍼널로 — 실측 3칸)은 ROADMAP §2.

**라운드 3 재리뷰가 내가 만든 BLOCKING 하나를 잡았다** — 두 렌즈가 또 독립적으로 수렴했다.
H5c 가 삭제한 손수 짠 concat 루프에는 `total > 63` 캡이 있었는데 그것을 *능력 제한*으로 읽고
`w > 64` 로 완화한 것이 결함이다. 실제로는 **도메인 가드**였다 — i64 상수 도메인은 부호 없는
**63비트**만 담고 `coerce_int_width(v, 64, false)` 는 항등이라, 최상위 비트가 선 64비트 배치가
**음수 i64** 로 새어 나간다. 실측: `({32'hFFFFFFFF, 32'h0} > 0)` 이 두 오라클 111 인데 vita 222 ·
`generate if ({1'b1, 63'd0} > 0)` 이 **다른 계층을 elaborate** · case item miss · 상수함수 본문 철자도
같다 — **전부 PRE 는 loud 였으므로 loud→silent-wrong**. 89칸으로 늘린 스윕이 이것을 못 잡은 이유도
기록해 둔다: **≥64비트 배치를 만드는 셀이 하나도 없었다**. 수정은 폭이 아니라 **적합성**으로 거절
(`w == 64 && !sg && v >> 63 != 0`) — 최상위 비트가 빈 64비트 배치는 계속 접히므로 옛 캡이 버리던
이득은 유지된다. ⚠️ 같은 결함이 **64비트 리터럴·파라미터에는 pre-existing** 임을 함께 측정했고
(`(64'hFFFFFFFF00000000 > 0)` 이 PRE 도 222), 그것이 새 문법으로 넓힐 라이선스는 아니라고 판정해
§2 에 등재했다. 곁 셋: `const_placement_env` 의 doc 이 하지 않는 decline 을 주장하고 있었고,
Prim 캐스트 거절의 *이유* 가 오라클로 반증됐으며(`int'(4'd15+4'd1)` = iverilog 16 = **Size 와 같은**
`max(self,N)` 규칙 · self-det 인 것은 Signing 뿐), 손으로 지은 `BitPacked` 의 두 plane 길이 불변이
`debug_assert` 로 핀됐다.

**뮤테이션 배터리** 18 종(14 + 라운드 3 의 4) · **16 사망 · 1 등가(논거를 코드에) · 1 은 판별자를
지어 사망**. 설계 단계에서 **생존 7 + 크래시 유발 1**이 나왔고 전부 킬러로 전환했다 —
가장 값진 셋: **3차원 packed**(기존 핀이 전부 2차원이라 *"첫 추가 차원만 곱하기"* 와 *"둘 이상은 decline"*
둘 다 통과했다) · **폭-미상 지역변수 + 동명 파라미터**(H1 가드의 진짜 킬러 — 폭이 알려진 `int X` 로는
변이가 byte-identical) · **블록-로컬 초기화 안의 재귀**(두 선언 자리의 깊이 과금이 다르다는 것의 핀).
⚠️ 라운드 3 의 *"캐스트 부호를 항상 unsigned 로"* 는 **전 워크스페이스에서도 살아남았고, 등가가 아니었다** —
64비트 미만 타깃에서는 감싸는 문맥(`leaf_into_ctx`)이 부호를 **다시 유도**하므로 캐스트 자신의 부호가
관측되는 자리는 **≥64비트 타깃뿐**이다(`longint x = 8'(255)` — 변이 255 / 정본·iverilog −1, 변이 빌드로 증명).
*"적합성 가드의 `!sg`"* 는 **진짜 등가**다(두 호출자가 전부 Concat/Replicate 이고 §11.4.12 가 그것을 unsigned 로
만든다) — 논거를 코드에 적었다.

---

#### 4.5.344 §2 정확성 큐 — bound/count/index lane 이 자기 폭에서 접힌다 (2026-08-19) ✅

**한 줄**: `**` 미fold 로 기록됐던 항목의 실체는 **한 결함의 두 철자 + 그 수정이 노출한 합법성 게이트
하나**였다 — ① `const_bound_u32` 가 width-inexact 형태 전부를 decline 하고 소비자의
`unwrap_or(1)/(0)` 가 조용히 삼키던 것(repl count `{(8'd2**8'd3){1'b1}}` → empty · `v[0 +: (8'd2**8'd3)]`
→ 1비트) ② lowered tree 의 **얕은 fold 가 폭-무시**(`Const 4'd9 + Const 4'd8` → 17)라
`v[(4'd9+4'd8):0]` 이 18비트 select ③ 폭-정직 fold 가 `{(4'd15+4'd1){1'b1}}` 을 **0-count** 로 만들자
드러난 §11.4.12.1(0-repl 은 concat 직속 피연산자에서만 합법). 신규 테스트 8(+기존 핀 6 강화) ·
5,608 → **5,614 green** · 뮤테이션 N1~N7 · format bump 없음.

- **tier-3**: `const_bound_u32` 에 자기결정 걷기(§4.5.343 기계)를 additive 로 — 단 ⓐ 치환-이름 가드는
  두 강한 티어 공통 ⓑ **전 노드 self-폭 기지(`ast_selfwidths_all_known`) 술어**로 degrade-무제한 값이
  wrap-민감 bound 로 새는 것을 차단(안 걸면 CW1 셀에서 silent 1 → silent 300 맞교환).
- **한 철자 자리 = `lower_index_expr`**: 얕은-fold 불일치 교정을 index·bound·offset·width·**write funnel**
  이 공유하는 퍼널 끝단 한 곳에 두자 read/write 쌍둥이가 함께 정합(42 소비자 · 합의 시 노드 보존 =
  기존 IR 바이트 동일). `width_from_msb_lsb_dir`·`lower_const_width_expr` 는 **무변경**으로 정합.
- ⚠️⚠️ **회귀 1 — 엔진 패닉으로 잡음**: `vec['1 +: 4]` 가 shift-overflow panic. 근인은 새 코드가 아니라
  **`const_bound_u32` tier-1 의 잠복 결함**(fill literal 을 32-bit all-ones 로 — `repeat` 의 fill 특례
  주석이 이미 그 결함을 알고 있었다) — 치환이 그것을 인덱스 lane 에 노출. 수정 = fill 특례를
  `const_bound_u32` **자체**에 승격('1→1 · '0→0 · 'x/'z→decline) ⇒ 모든 bound/count/index 소비자가
  한 철자를 공유.
- **0-repl 합법성**: `repl_zero_ok` 플래그 — Concat arm 이 **Replicate-kind 파트에만** 세팅, Replicate arm
  이 **자식 lowering 전에 `mem::take`**(중첩/인덱스 위치로 누수 불가 · 실측 L1/N4 셀). bare `{(0){x}}` =
  E3009 · concat-drop/string-empty 경로 불변. ⭐ **중첩 판별 셀에서 오라클이 갈렸다**: concat 파트의
  **인덱스 안** 0-repl 을 iverilog 는 조용히 돌리고(0017) **verilator 5.050 은 같은 조항을 인용하며
  거부**("only legal under a concatenation, IEEE 11.4.12.1") — vita = verilator = hand-IEEE.
- ⭐⭐ **옛 decline-핀 6개가 전부 자기 주석이 선언한 정답으로 강화됐다**(fail-fast 뒤에 4개 더 숨어
  있었다 — `--no-fail-fast` 전수): `$clog2(4'd15+4'd15)` count 0→**4** · bare narrow add 30→**14**(=엔진
  런타임과 합치) · `(1<<33)>>30` lsb 셀 1→**ef** · **byte 반환 const fn** 0→**11**(width-aware 인터프리터는
  원래 폭-정직 — 옛 게이트의 우려는 무제한 도메인에만 적용) · named-block 중첩 decl 0→**14** · signed
  `$clog2(4'shF)` 0→**4**(verilator+런타임 · iverilog 32 = 발산 ⑤). 전부 iverilog(또는 verilator) 값.

**잔여(§2 등재)**: 폭-미상 leaf(const-array 원소 — `const_self_width` 에 arm 없음) 위의 wrap bound 는
의도적 decline 유지(pre-slice silent 1비트 · verilator 오라클 = 44) — 해소는 폭 모델 확장이고, 마커 핀
`width_unknown_wrap_bound_keeps_preslice_decline` 이 지킨다.

#### 4.5.343 §2 정확성 큐 1~3 — 자기결정 위치 셋을 한 뿌리로 닫다 (2026-08-19) ✅

**한 줄**: 무제한 i64 fold 가 IEEE 가 **스스로 크기를 정하는** 위치 셋($clog2 인자 · 캐스트 SIZE 식 ·
real 로 변환되는 정수 서브트리)을 넓혀 접던 것을, §4.5.339 의 자기결정 걷기(`eval_const_env_self`)
하나로 전부 라우팅했다. 신규 헬퍼 둘(`const_int_selfdet` · `const_clog2_selfdet`) · 신규 테스트 12 ·
5,596 → **5,608 green** · 뮤테이션 10/10 사망 · format bump 없음(순수 elaborate 로직).

**①-급 근거(§2 표의 #1)**: `localparam L = $clog2(4'd15 + 4'd1)` 이 vita **4** / iverilog **0** 인데
**같은 소스가 상수함수 안에서는 vita 도 0** — 스펙 논쟁이 아니라 vita 자기 불일치였다. 수정 후
module-scope · 상수함수 · 런타임 엔진 **세 평가기가 0 으로 합치**(`clog2_selfdet_arg_three_evaluators_agree`).

- **#1 $clog2 인자**: 세 arm(모듈 스코프 `const_eval_in_scope` · plain env twin `eval_const_env` ·
  width twin `eval_const_env_at`)이 **한 헬퍼** `const_clog2_selfdet` 를 공유한다(산술 세 벌 → 한 벌).
  ⭐⭐ **§20.8.1 "treated as an unsigned value" 는 인자 자기 폭의 비트 패턴 읽기다** —
  `$clog2(4'sd7+4'sd1)` 은 4비트 wrap `1000` = unsigned 8 ⇒ **3**. 초판(부호 그대로 + `n<0` decline)은
  이 셀을 correct→loud 로 **강등**시켰다(적대 famA 스윕이 착지 전에 잡음). 판정 오라클 3중:
  **verilator 5.050 = 3 · vita 런타임 = 3 · hand-IEEE = 3** / iverilog 13.0 은 **32**(인자를 32비트
  정수로 승격 후 unsigned — §0 iverilog 결함 목록 ⑤ 신설). 같은 규칙이 `$clog2(-1)` 을
  **loud→32**(두 오라클 합치)로 올린다. 곁: `$clog2(2**33)` 33→0.
- **#2 캐스트 SIZE 식**: `cast_size_bits` 가 self-det 으로 접고(Size·Named 두 철자 한 걷기),
  ⭐ **lowering(`expr_cast.rs`)의 사적 `const_eval_in_scope` 가 두 번째 철자**였다 → `cast_size_bits`
  하나로 퍼널. 런타임 `4'd3 ** ((4'd9+4'd8)'(2))` **9→1**(=iverilog · 사이즈 17→1). const lane 은
  **silent 9 → loud E3009**(decline) — `const_eval_cast` 의 절단 fold 는 무제한 operand fold 위에서
  unsound 라 열지 않았다(반례 실측: `4'((4'd8+4'd8)/4'd3)` SV 0 vs 절단-fold 5 — wrap 후 나눗셈은
  저비트 폐쇄가 아니다). 잔여로 §2 등재(선행조건 = 기존 AST self-폭 패스).
- **#3 real 도메인**: ⭐⭐ **측정이 큐의 서술을 넓혔다** — `**` 지수만이 아니라 **모든 real 연산자의
  정수 피연산자**가 같은 넓힘(`1.0 + -4'sd8` → 9.0 / iverilog −7.0 · `1.0+3/2` → 2.5 / 2.0 · 8형태).
  수정은 서술보다 **작다**: `const_eval_real_in_scope` 상단에 **integral 게이트 하나**(real 을 안
  언급하는 서브트리 = §11.8.1 변환 경계 → `const_int_selfdet` 로 접어 승격) + `param_real_value` 의
  integral fallback 한 줄 + **삼항 cond**(§11.4.11 자기결정 — `(4'd15+4'd1) ? 1.5 : 2.5` 가 1.5→**2.5**).
  famC 18셀 + tern 3셀 = iverilog **바이트 동일** · PRE 는 shift/concat 셀(C10/C17)에서 **loud** 였다
  (게이트가 loud→correct 로 함께 올림). i64 twin 은 같은 self-det 값을 등록(두 읽기 불일치 불가).
  decline 시 f64 재걷기로 떨어지지 **않는다**(그 fall-through 가 닫으려는 넓힘의 재철자 — 그리고
  real-free 에서 f64-foldable ⊂ i64-foldable 실증이라 관측 불가).

**PRE-3-way**(iverilog / PRE=1071353 / POST · 34+ 셀): **WRONG→FIXED 12+ · WRONG→LOUD 1 ·
LOUD→CORRECT 3($clog2(-1)·C10·C17) · 회귀 0**. staged parity 바이트 동일(신선 separate-bins).

**뮤테이션 10/10 사망 — 단, 첫 배터리는 9/10 이었다**: 게이트 제거·arm 별 되돌림·unsigned 마스크
제거·`_self` 탈락·lowering 재철자. ⭐ 킬러 설계 실측 둘 — ① **최종 마스킹이 내부 넓힘을 구조한다**
(`$clog2(4'd15+4'd1)` 는 plain-walk 뮤턴트에서도 0) ⇒ 킬러는 **인자보다 좁은 내부 자기결정 영역**
(shift count: `$clog2(12'd100 >> (4'd15+4'd1))` = 7 vs 0). ② ⚠️ **M9(plain-twin arm)가 첫 배터리에서
생존했다** — multi-packed body-local 로 지은 킬러가 그 arm 에 **안 갔다**: Blocking 대입의 타깃은 envw 에
**항상 항목이 있고**(미상 형상도 `(0,false)` 로 기록) `Some((0,_))` 은 width-twin 으로 간다. plain twin 의
진짜 문은 **폭이 안 접히는 formal range 를 가진 call 인자**(`input logic [w4():0] x` — `const_decl_wsign`
이 call-in-bound 를 decline → target None)였고, 뮤턴트 빌드에서 G2 7→0(exit 0)으로 **라우팅을 증명**한 뒤
재판정 KILLED. 생존 뮤턴트는 등가 선언이 아니라 도달 경로 오판의 신호였다. ⚠️ 곁: 첫 M9 재판정 시도는
타임아웃이 복원 전에 명령을 죽여 **트리가 뮤턴트 상태로 남았다** — 즉시 복원했고, 재시도는 `trap restore
EXIT` 를 스크립트 안에 넣어 타임아웃이 트리를 강탈할 수 없게 했다(배터리 규칙에 병합할 것).

**곁 측정(§2 신규/갱신 등재)**: ⓐ **런타임 mixed-real Binary 가 정수 피연산자의 wrap 을 넓힌다**
(`1.0+(-s)` 9.0 / −7.0 · `1.0+(s+s)` −15.0 / 1.0 · leaf ✓ · SysFunc Pow lane ✓ — 퍼널은 lowering/엔진의
real 경계 width 문맥, elaborate const 가 정답이 된 지금 같은 소스가 localparam ✓/runtime ✗ 로 갈린다)
ⓑ **generate-if cond 무제한 fold**(2-오라클 합치: `if (4'd15+4'd1)` vita taken / iverilog+verilator else —
광역 폭 클래스, 전용 슬라이스+PRE-3-way 필요) ⓒ untyped localparam 은 **오라클 분열**(iverilog 16 =
vita / verilator 0 — §6.20.2 해석차, 기록만) ⓓ real-반환 const fn 본문 폭(PRE==POST 소형).

#### 4.5.342 3판 잔여 클리어 라운드 — 순서 1~10 + P1, 그리고 #8 은 지어서 재고 되돌렸다 (2026-08-18 ~ 08-19, main 직커밋) ✅

> aes_top 3판 리포트(§4.5.341)가 로드맵으로 넘긴 5건(§3.1·§3.3·§3.5·§3.6·§3.7)에 선행조건까지
> 붙여 10 항목 + P1 로 편성한 라운드의 전문 보관. 커밋 = `b472b67`(1) · `6e44870`(2) ·
> `6033e26`(3) · `461ddb8`(4) · `7218d3d`(5) · `395e491`(P1) · `db9f31b`(6) · `df74917`(7) ·
> `c388c98`(8 revert) · `c9a6acd`(9) · `b868ff3`(10). 게이트 5,533 → **5,596 green** ·
> format_version 27 → **29** · MsgCode 67 → **68**. 아래 표와 불릿은 ROADMAP §3 에서 이관 당시
> 원문 그대로다.

| # | 슬라이스 | 여는 것 | 사이트(실측) |
|---|---|---|---|
| 1 | ✅ §3.5-① (08-18) | `4*16`·`LP`·`LP/2`·`$clog2(LP)`·`'1` — FIXED 5 · 회귀 0 | `repeat_unroll_count` 한 철자 · 뮤테이션 8/8 |
| 2 | ✅ §3.6-① (08-18) | verilator 6/6 · bare-이름 = 클럭된 불리언 | 센티넬은 이미 있었다 · 드레인 한 자리 · 7/8 |
| 3 | ✅ §3.3 (08-18) | 9 형태 iverilog 바이트 일치 | admission = 캐리 없음 · `walk_scopes_key` 한 철자 · 9/10+도달불가 1 |
| 4 | ✅ §3.1(a)(b) (08-18) | 4형태 `W2004` · 값 불변 | 판별자는 provenance · MsgCode 68 · 3/4+도달불가 1 |
| 5 | ✅ §3.7 INPUT (08-18) | 6형태 iverilog 일치 + `StrCmp` 라우팅 | `frame_local_net_kind` 하나 · 5/5 |
| P1 | ✅ (08-18) | inner-NET shadow · generate 삭제 재발 없음 | 선언 블록의 SPAN · 6/6 |
| 6 | ✅ §3.5-③ (08-19) | `repeat` 가족 전부 프레임에서 | 카운터를 예약 패스에 · span 키 · §2 self-width 동반 수정 |
| 7 | ✅ §3.6-② (08-19) | verilator 3/3 + 중복 loud | `.vu` 재핀 · multi-clock 게이트보다 먼저 · 4/5+등가 1 |
| 8 | ⚠️ §3.11 (08-19) | — 지어서 재고 **되돌렸다** | 15 실패 · `$random` 두 번 = 인라인 경로의 핀된 결함 상속 |
| 9 | ✅ staged file:line (08-19 · v28) | velab 진단 = one-shot 바이트 동일 | `.vu` source-map tail · 같은 `MapResolver` · 6/6+절단 loud |
| 10 | ✅ 런타임 file:line (08-19 · v29) | severity 가족 전부 위치+인스턴스+시각 | `severity_locs` 사이드카 · emitter 셋이 한 헬퍼 · 7/7 |

- **✅ §3.1 (a)(b) RESOLVED — 이식성 경고 `W2004` (2026-08-18 · 클리어 순서 4번) · ⚠️ (c) 는 잔여.**
  ⭐⭐ **재측정이 리포트를 넓히고 갈랐다** — select 축은 **네 형태**이고 사이트는 **하나**다
  (`hdl-parser/expr.rs` 의 postfix 루프가 base 의 종류를 안 본다 · IEEE 1800 §11.5.1 은 select 를
  **variable reference** 에만 허용).
  | 형태 | vita | iverilog 13 | verilator 5.050 | 처분 |
  |---|---|---|---|---|
  | `((a^b)>>8)[7:0]` · `16'hABCD[7:0]` | 동작 | 거부 | **거부** | **W2004** |
  | `f(a)[7:0]` · `{a,b}[7:0]` | 동작 | 거부 | **동작(같은 값)** | **W2004** |
  ⚠️⚠️ **넷을 같은 처분으로 다루면 안 된다** — 뒤의 둘은 verilator 가 **같은 값으로 받으므로** 거부하면
  오늘 이식 가능한 코드에 **사다리 하강**이다 ⇒ **값은 그대로 두고 경고**(§3.2 의 `\r` 과 같은 정책 ·
  리포터 요청도 error 가 아니라 warning 이었다). ⭐⭐ **판별자가 AST 가 아니라 provenance 인 이유를
  실측이 강제했다** — packed struct 멤버 `p.hi` 를 **바로 그 파서 루프가 part-select 로 desugar** 하므로
  AST 가 생기고 나면 `p.hi[3:0]`(모든 툴이 받음)과 `a[7:0][3:0]` 이 **같은 모양**이다. *"체인이 이름에서
  시작했는가"* 는 파서 안에서만 알 수 있다. ⚠️ 파서에 경고 채널이 없어 **`ParseWarn` 신설**(에러와 별개
  채널 — 경고가 파스를 중단시키면 안 되고, 에러 타입에 severity 를 얹으면 *"파스를 멈췄나"* 와 *"얼마나
  나쁜가"* 가 한 자리에 섞인다) · **에러 게이트보다 먼저 발행**(다른 곳의 구문 오류가 이식성 경고를 삼키면
  안 된다) · ⚠️ **`parse()` 의 arity 는 안 바꿨다** — 넓혔더니 **테스트 44곳**이 걸렸다(한 소비자를 위해
  44곳을 `_` 로 고치는 것은 잘못된 거래) ⇒ **`parse_with_warnings()`** 를 별도 진입점으로.
  **MsgCode 67 → 68** · doc-15 항목 · 앵커 = `cli/tests/select_base_portability.rs` **4건**(억제·승격
  **3 철자** 확인 포함) · **뮤테이션 4 중 3 사망 · 1 도달 불가(프로브 0 히트)**. ⚠️ **아직 경고하지
  않는 것 1건**: `a[7:0][3:0]`(slice-of-slice)은 이름에서 시작해 통과하는데 **iverilog 는 거부한다** —
  별개 판별자가 필요해 남겼다(doc-15 에 기록).
  ⚠️⚠️ **잔여 = (c) `always_comb` 구동 변수의 선언 초기화자**(verilator `MULTIDRIVEN` error · xrun
  `*E,MULAXX` · iverilog 는 실행 = **2:1**). elaborate 층이고 감도 리스트 합성이 그 집합을 이미 안다.

- **✅ §3.3 RESOLVED (2026-08-18 · 클리어 순서 3번)** — 넓은(>64bit) `localparam` 이 **캐리 없는**
  연산으로 접힌다. ⭐ **원인은 도메인이 아니라 arm 이었다** — `>64bit` 도메인(`wide_param_bits`·
  `ir::BitPacked`·`resize_bits`)은 **선언 경로에 이미 서 있었고**(`params.rs:799`) `fold_init` 에 arm 이
  셋뿐이었다(리터럴·fill·`Paren`). **9 형태가 iverilog 와 바이트 일치**: `{8'he1,120'h0}` · `128'(8'he1)<<120`
  (둘 다 리포트가 든 것) · `{16{8'hAB}}` · `~A` · `A>>8` · `A<<4` · `A|lit` · `A&lit` · `A^{2{…}}`.
  ⚠️⚠️ **admission 이 "캐리 없음" 이고 그것이 규칙이다** — concat·replicate·size cast·상수 **논리** 시프트·
  `&`/`|`/`^`·`~` 는 각 결과 비트를 **알려진 위치의 피연산자 비트**로 정한다. `+`/`-`/`*` 는 128비트 캐리
  체인이 필요하고 `>>>` 는 부호비트를 읽는다 ⇒ 여기서 구현하면 **엔진 산술의 두 번째 철자**이고 미묘하게
  틀리면 **조용히 틀린 파라미터**(P0-5) ⇒ **거부 유지**(테스트로 핀). ⚠️ x/z 도 같은 원칙으로 갈랐다 —
  **배치 arm**(concat/replicate/shift/cast 는 비트를 **옮기지 읽지 않는다**)은 통과, **값을 읽는 arm** 은 거부.
  ⭐ 이름 해석은 **`walk_scopes_key` 를 그대로 쓴다**(`lower_expr` 과 한 철자 · 두 번째 스코프 워크가
  §4.5.218 의 모양이다) · `wide_param_bits` **만** 본다(비트·폭·부호를 함께 들고 있는 유일한 테이블 —
  좁은 파라미터는 fail-closed 로 거부). **뮤테이션 10 중 9 사망 · 1 도달 불가(실측)** — ⚠️ **넷이 눈먼
  축이었다**(`&|^` 의 x 절반 · **좁은 lhs** 가 있어야 갈리는 비트별 폭 규칙 · **두 번째 wide param** 이
  있어야 드러나는 이름 해석 · concat 안 fill) · ⚠️ 한 치환은 **의미 없는 no-op** 이었다(`get` 성공 시
  `or_else` 미실행 → SURVIVED 로 세지 않고 다시 걸었다) · ⚠️ 계층 이름 arm 은 **`panic!` 프로브 0 히트**
  (vita·iverilog 둘 다 계층 참조를 localparam 초기화자에서 먼저 거부한다) ⇒ fail-closed 유지.
  앵커 = `cli/tests/wide_param_fold.rs` **10건**. ⚠️ **잔여 = part-select**(`{A[127:64], 64'h0}` 은 아직
  거부 · iverilog 는 접는다) — 인덱스 fold 가 필요한 별개 arm 이라 이 슬라이스에 안 넣었다.
  ⚠️ **2판 잔여 "wide 파라미터 OVERRIDE 는 loud"** 는 **여전히 열려 있다**(그쪽은 `ResolvedOverride` 채널).

- **✅ §3.5-① RESOLVED (2026-08-18 · 클리어 순서 1번)** — `repeat` 의 count 가 **`const_bound_u32`**
  (모든 select bound·replication count 가 쓰는 그 퍼널)로 접힌다. ⭐⭐ **수정의 본체는 arm 이 아니라
  "한 철자"** 다 — 언롤 판정을 `repeat_unroll_count` 하나로 만들어 **lowering(`lower_repeat`)과
  분류기(`ast_has_repeat_with_timing`)가 같은 답**을 내게 했다(분류기는 `&self` 가 됐고 그것만 읽는다).
  **3-way(PRE/POST/iverilog) — FIXED 5 · 회귀 0**: `4*16`·`LP`·`LP/2`·`$clog2(LP)` 가 전부 iverilog
  값과 일치. ⭐ **다섯 번째는 리포트에 없던 pre-existing false-loud** — **`repeat ('1)`** 이 E3009 였다
  (iverilog `c=1`): `lower_repeat` 는 fill 리터럴 arm 을 늘 갖고 있었는데 **분류기의 사본에는 없어서**
  *"언롤러가 안 먹는 count"* 로 읽고 거부했다 = **두 철자 문제의 실증**. ⚠️ **일부러 loud 로 남긴 둘**:
  `repeat (4'd15+4'd1)` 은 SV 가 4비트에서 **0** 으로 자르는데(iverilog `c=0`) 무제한 i64 도메인은 16 을
  내므로 `const_bound_u32` 의 **width-exact 가드가 거절하는 것이 옳다**(loud 는 wrong 이 아니다) ·
  **언롤 상한(1024) 초과**도 런타임 카운터로 떨어져 loud. ⚠️ 잔여 = **`repeat (m_n)`(모듈 넷) = 클리어
  순서 6번**(프레임 subset 의 런타임 카운터) · **`repeat (LP)` 는 이미 되므로 P1 선행조건에서 빠졌다**
  (`const_bound_u32` 가 `const_eval_in_scope` 를 이미 쓴다 — 옛 기록이 §2 DEEP 과 묶어 둔 것은 정정).
  ⭐ **문구도 원인을 이름으로 부른다** — 세 거부 술어(`!leaf` / `unsafe_repeat` / `unsafe_ctor`)를 결정
  **전에** 이름으로 평가하고, `frame_task_has_unsafe_construct` 는 `bool` 이 아니라 **어느 구문인지**를
  돌려준다(다섯 사이트). ⚠️ **`!leaf` 분기는 도달 가능하다**(`panic!` 프로브 실측 3건 · 전부
  `cli::fork_in_frame` 의 fork-arm `return`) — 뮤테이션이 그 문구를 뭉갤 때 처음엔 생존했고 판별자를 지어
  사살했다. **뮤테이션 8/8 사망** · 앵커 = `cli/tests/repeat_count_fold.rs`(8건 · iverilog 핀).
  ⚠️ **곁가지 발굴**: 태스크 지역 `localparam`(`task automatic t; localparam int K = 3;`)은 **파서가
  거부**한다(E2002 · PRE==POST = pre-existing · iverilog 는 받는다). 별건으로 아래 §3 소형 큐에 등재.

- **✅ §3.6-① RESOLVED (2026-08-18 · 클리어 순서 2번)** — `default clocking` 이 이 스코프의 무클럭
  concurrent assertion에 **실제로 클럭을 준다**(IEEE 1800 §14.12). ⭐⭐ **센티넬은 이미 있었다** —
  `assert property(NAME)` 이 예전부터 **빈 sensitivity 리스트**로 파싱되고 코드가 그것을 *"the empty
  sentinel"* 이라 부르고 있었으므로, 파서는 **거부를 걷어내고 같은 센티넬을 남기고**(`sva_prop.rs`)
  **`sva_check.rs` 의 드레인 한 자리**가 해석한다(모듈 단위라 형제 모듈로 샐 수 없다 — `lower_clocking_blocks`
  가 `clocking_events` 와 함께 지우고, 유일한 소비자가 **같은 `elaborate_instance` 안**에서 늦게 돈다).
  ⚠️⚠️ **오라클은 verilator 5.050 이다** — **iverilog 13 은 `default clocking` 을 파싱조차 못 한다**.
  **6형태 전부 verilator 와 일치**: 무클럭 `a |-> b`(FAIL t=1,3,5) · 명시 클럭 우선(t=3,9) · **bare
  `assert property (a)` = 클럭된 불리언**(§16.12 · 명시 철자 `@(clk) a` 는 원래 돌고 있었다) · named
  property 무클럭 · 그리고 **default 가 없으면 여전히 거부**(verilator 도 거부 — 파서 에러를 지우는 것이
  거부를 *없애는* 게 아니라 *옮기는* 것임을 그 사실이 고정한다). ⚠️ bare-이름 재해석은 **`default
  clocking` 이 있을 때만**(없으면 *"unknown property"* 문구 그대로 = 진짜 오타를 안 삼킨다 · 대조군 핀).
  ⚠️ **지원 안 되는 클럭 이벤트는 보이지 않는 default 가 되지 않는다**(edge-list 검사 **뒤**에 기록 —
  ⭐ 뮤테이션이 그 순서를 **오늘은 등가**라고 실측했다: 그 거부가 Error 라 런 자체가 없다. 그래도
  fail-closed 로 남기고 논거를 코드에 적었다 — 그 거부가 경고로 내려가는 날 정확히 그것이 silent-wrong 이 된다).
  **뮤테이션 7/7**(생존 1 = 위 등가) · 앵커 = `cli/tests/default_clocking.rs` **8건**(⚠️ 그중 **모듈 간
  누수 판별자**는 배터리가 요구해서 지었다 — 나머지 일곱이 전부 단일 모듈이라 눈먼 축이었다).
  ✅ **§3.6-② 도 닫혔다(2026-08-19 · 클리어 순서 7번)** — `ModuleItem::DefaultDisableIff` + 모듈 단위
  기록 + 드레인이 빈 `disable_iff` 를 채운다(⚠️ **multi-clock 게이트보다 먼저** — early `continue` 가
  기본 리셋을 조용히 떨어뜨리지 않도록). **verilator 3/3 일치**(억제 · 명시 `disable iff` 가 이김 ·
  대조군). ⭐ 곁가지로 **한 스코프에 둘이면 loud**(IEEE §16.15 · verilator 도 명시 거부) — 그리고 그
  거부가 *"첫 것이냐 마지막이냐"* 를 **증명 가능하게 무의미**하게 만든다(⭐ 뮤테이션이 그 결론을 거꾸로
  냈다: 생존 → 판별 설계를 찾다 그것이 불법임을 알았다). ⚠️ AST 변종 하나라 **`.vu` SchemaHash 재핀**
  (sim-ir·format_version 불변 · 순수 IR-0).

- **✅ §3.7 RESOLVED — INPUT 방향 (2026-08-18 · 클리어 순서 5번) · ⚠️ output/inout 은 좁혀서 잔여.**
  ⭐ **수정은 넷 종류 하나다** — 인라인(static-lifetime) 경로가 formal-local 을 `map_net_kind_or_wire`
  로 잡아 `string` → **1비트 `Wire`** 였고 copy-in 이 잘렸다. **프레임 경로는 1비트로 둬도 되는데**
  `FuncMeta` 가 있고 엔진이 **`str_params` 마스크**로 *"이 슬롯은 힙 핸들"* 을 알기 때문이다 —
  인라인된 태스크엔 `FuncMeta` 가 없어 그 사실을 나를 것이 없다 ⇒ **`frame_local_net_kind`**(프레임 지역
  `string` 이 이미 쓰는 그 함수). 6 형태가 iverilog 와 일치(리터럴 · string 변수 · 정수 formal 병행 ·
  본문 `s.len()` · 길이 다른 연속 호출 · `automatic` 불변). ⚠️⚠️ **행을 지웠다가 되돌려 좁혔다** —
  통째로 지우니 output/inout 이 *"output/inout arg must be a simple net (v1)"* + **허위 E3010 캐스케이드**
  로 떨어졌는데(formal 이 안 묶여 본문 읽기가 미해결), **실측상 `automatic` 은 output/inout 도 고친다**
  ⇒ 옛 문구의 조언이 그 방향엔 **옳았다**. 그래서 output/inout 전용 한 메시지를 남기고 *"INPUT 은 이제
  지원된다"* 까지 말하게 했다(독자가 input 까지 다시 쓰지 않도록) · 캐스케이드 **0**.
  ⚠️⚠️ **기존 핀이 이 슬라이스의 가장 중요한 검증이었다** — `static_task_string_formal_loud_rejects` 의
  주석이 관심사를 **둘**로 적어 뒀다(*"자르고, **`StrCmp` 라우팅도 건너뛴다** — 둘 다 조용하다"*).
  내 수정은 첫째만 겨냥했으므로 둘째를 즉시 쟀고 **함께 열렸다**(`r=1`/`r=1`/`r=0` 전부 iverilog 일치)
  ⇒ 핀을 positive 로 뒤집되 ⚠️ **불일치 행을 넣었다**(일치 케이스만이면 *"항상 1을 반환하는 비교"* 도
  통과하는데 그게 바로 그 테스트가 잡으려던 결함이다). **뮤테이션 5/5** · 앵커 =
  `cli/tests/static_task_string_formal.rs` **4건**(⚠️ 방향 이름 판별자는 배터리가 요구해서 더했다 —
  `inout` 에 "output" 이라고 쓰면 독자를 틀린 포트로 보낸다 = §3.4 가 이미 값을 치른 결함).
  ⚠️ **static FUNCTION 의 `string` formal 은 원래 됐다**(PRE 실측) — §3.7 은 task 전용이었다.

- **⚠️⚠️ §3.11 `function automatic` 인라인 — 지어서 재고 되돌렸다(2026-08-19 · 클리어 순서 8번).**
  **이 저장소가 이 계획에 대해 적어 둔 전제가 측정으로 반증됐다.**
  리포터 실측 codegen `able/total` = **12.3%(core) / 16.0%(top)**, 거부 사유의 **87%가
  `user_call_in_expr`**. 이쪽 census 도 같다(`always @(posedge clk) r <= fa(a)` 한 설계에서
  `able 1/4` · `frame_bodies 1` · `user_call_in_expr 1`). 판별자는 키워드 하나다 — `function f`
  는 인라인되고 `function automatic f` 는 프레임으로 내려가 호출 프로세스가 codegen 에서 거부된다.
  ⭐ **지은 것**: `build_frame_set` 의 `f.automatic` 이 **단독으로** 프레임을 강제하지 않게 하고,
  나머지 절(2-state 반환 · 비직선 바디 · unpacked formal · string 반환 · 재귀 폐포)은 그대로 두었다.
  ⚠️ **첫 판정을 실측이 좁혔다** — `automatic` 함수가 **지역을 대입 전에 읽으면** 두 경로가 갈린다
  (프레임 = 매 호출 fresh `x` / 인라인 = 호출 사이트당 저장이라 두 번째 호출이 이전 값). iverilog 는
  `x x` 인데 인라인으로 보내면 vita 가 **E3010** = correct→loud 하강 ⇒ **지역을 선언하지 않는**
  automatic 함수만 열었다(리포트가 말한 S-box·`xtime`·GF 곱이 정확히 그 모양).
  ⚠️⚠️ **그래도 안 된다 — 전 스위트가 15건 실패했고 기제 교체가 아니라 진짜 동작 변화였다.**
  가장 선명한 둘: `inline_formal_bind::a_frame_call_actual_is_evaluated_once` 가 `$random` 인자를
  **두 번 뽑았고**(`val=01/03` → `04/09`), `size_cast_seal::a_widening_cast_never_evaluates_an_impure_operand_twice`
  가 걸렸다 — 그 테스트의 doc 이 이유를 이미 적어 뒀다: **인라인 확장이 피연산자를 두 번째로 이름 부른다**
  (`Select{Bit, base: e}`), 그래서 불순한 actual 이 두 번 돈다. 나머지 열셋도 대부분
  *"…is_a_documented_gap"* / *"…matches_bare"* = **인라인 경로의 핀된 결함들**이다.
  ⇒ ⭐⭐ **결론: `automatic` 은 "프레임 경로" 를 뜻하고, 그것이 이 저장소가 인라인 경로의 알려진
  결함으로부터 코드를 지키는 방법이다.** *"비-재귀 automatic 은 인라인이 의미상 동일하다"* 는 **거짓**이다.
  ⇒ **진짜 선행조건 둘 중 하나**(둘 다 §2/§5 에 이미 있다):
  ⓐ **인라인 확장이 피연산자를 한 번만 이름 부르게** 한다(또는 callee-purity 술어) — 그러면 그 열다섯 중
  다수가 사라지고 그때 `automatic` 을 보낼 수 있다 · ⓑ ⭐ **더 곧은 길: 인라인이 아니라 codegen 을 고친다**
  — `is_codegen_able` 이 `Terminator::Call` 을 가진 프로세스를 통째로 거부하는 것이 원인이고(§5 T1/T2),
  그쪽을 열면 **프레임 경로를 유지한 채** 리포터의 지표가 오른다. ⓑ 가 사다리를 안 건드린다.
  ⚠️ picorv32 는 어느 형태도 안 써서 65/68 이다 — **이 축은 벤치가 대표하지 않는다.**

- ✅ **staged `file:line:col` 해결(2026-08-19 · #9 · format v28).** `.vu` 가 **source-map tail**
  (파일별 `(name, ORIGINAL text)` + 세그먼트 5-튜플 — `resolve` 가 읽는 정확히 그 부분집합)을 싣고,
  `velab` 이 그것으로 `SourceMap` 을 재구성해 **one-shot 과 같은 `MapResolver`** 를 설치한다(해석
  규칙 재철자 0). `canon`/`dir` 은 **일부러 안 싣는다**(머신 로컬 절대경로 — 3-OS 바이트 동일성을
  깨고, include reader 전용이라 staged 에선 아무도 안 읽는다). 판별 축 5개 전부 one-shot 과 바이트
  동일: 평범한 E3010 · **include 파일이 자기 이름+로컬 줄**(`inc4.svh:4:8`) · **매크로 사용
  지점**(collapsed 세그먼트) · 다중 파일 G12 · **멀티바이트 char 컬럼**(17, 바이트면 21). corrupt
  tail 은 loud(`undecodable .vu source-map trailer`). 유닛 스윕은 **모든 확장 바이트**에서 roundtrip
  맵의 `resolve_span` 동일성을 단언한다. ⚠️ **잔여: `velab -L`(worklib 병합) 경로는 여전히
  위치가 없다** — 병합이 **여러 CU** 의 아이템을 한 unit 으로 합치는데 각 CU 의 스팬은 자기 확장
  버퍼(0부터)를 인덱싱해 **좌표공간이 겹친다**. 틀린 CU 의 맵으로 풀면 **틀린 file:line**(없는 것보다
  나쁘다)이라 `None` 유지 — 닫으려면 병합 시 스팬 오프셋 재작성(AST 전체 walk)이 필요하다.

- ✅ **§3.10 런타임 severity `file:line` + 인스턴스 경로 해결(2026-08-19 · #10 · format v29).**
  엔진은 span-free IR 위에서 돌므로 **스스로는 영원히 못 푼다** — 답은 elaborate 가 **한 번**
  푸는 것이다: `lower_severity_task` 가 `severities` 를 쓰는 **같은 자리**에서 `cur_location()` +
  `cur_prefix` 를 `severity_locs` 사이드카(StmtId → file·line·col·byte범위·인스턴스)로 굳히고,
  런타임 emitter 셋(statement 경로 `emit_severity_message` · frame 경로 `frame_emit_severity` ·
  deferred 성숙 `mature_deferred`)이 **한 헬퍼**(`severity_diag_meta`)로 읽는다. 출력:
  `d.sv:5:5: error[VITA-E4003] …: child says 42 [in top.u1] [at time 5]` — **한 소스 줄을 두
  인스턴스가 각자 자기 경로로 보고**한다(§4.5.249 의 논거가 런타임에 도달). 커버: `$fatal`/`$error`/
  `$warning`/`$info` · **W4031 unique/priority 위반**(case 문을 가리킨다) · **deferred assert**(성숙
  시각에 **액션 사이트** 위치 + reach-시각 값) · **frame-path severity**. one-shot ↔ staged **바이트
  동일**(velab 이 v28 소스맵으로 해석해 `.velab` extra-sidecars 로 나른다 — #9 가 선행조건이었던
  이유). iverilog 가 같은 4요소를 확인(`ERROR: d.sv:5: … Time: 5 Scope: top.u1`). resolver 없는
  경로(엔진 유닛 하네스)는 **엔트리 자체가 없어** 바이트 동일. ⚠️ **잔여(측정)**: sid 가 없는
  런타임 진단군 — RunRange/W4020/W4022/W4028 등 — 은 접근 **문장**을 아는 키가 엔진에 없다(넷
  선언 지점은 있지만 표준 위치 슬롯에 찍으면 접근 지점으로 오독된다 = 없는 것보다 나쁘다).
  §4.5.341 이 이미 배열 **이름**을 붙였으므로 남은 값은 작다.

### ⭐⭐ 3판 잔여 5건의 클리어 순서 (2026-08-18 · **전부 코드 사이트까지 실측**)

> 순서의 근거는 **선행조건 그래프**이지 리포트의 번호가 아니다. ⚠️ **§2 가 §3 보다 위**라는 원칙(§1)은
> 유지되는데, 아래 **P1 이 마침 §2 항목**이라 사다리와 이 순서가 충돌하지 않는다.

| # | 슬라이스 | 여는 것 | 선행조건 | 사이트(실측) |
|---|---|---|---|---|
| ~~1~~ | ✅ **§3.5-① 완료(2026-08-18)** | `4*16`·`LP`·`LP/2`·`$clog2(LP)`·⭐`'1` — **FIXED 5 · 회귀 0** | — | 답은 arm 이 아니라 **한 철자**(`repeat_unroll_count` 를 lowering·분류기가 공유) · 뮤테이션 **8/8** |
| ~~2~~ | ✅ **§3.6-① 완료(2026-08-18)** | verilator 6/6 일치 · bare-이름 = 클럭된 불리언까지 | — | 센티넬은 이미 있었다(`assert property(NAME)`) · 드레인 한 자리 · 뮤테이션 7/7 |
| ~~3~~ | ✅ **§3.3 완료(2026-08-18)** | 9 형태가 iverilog 바이트 일치 | — | admission = **캐리 없음** · 이름은 `walk_scopes_key` 한 철자 · 뮤테이션 9/10 + 도달불가 1 |
| ~~4~~ | ✅ **§3.1 (a)(b) 완료(2026-08-18)** | 4형태 경고 · 값 불변 · 정당한 3형태 경고 0 | — | 판별자는 AST 가 아니라 **provenance**(멤버가 part-select 로 desugar 된다) · `W2004` · 뮤테이션 3/4 + 도달불가 1 · ⚠️ **(c) 는 잔여** |
| ~~5~~ | ✅ **§3.7 INPUT 완료(2026-08-18)** | 6형태 iverilog 일치 + `StrCmp` 라우팅 | — | 수정은 **넷 종류 하나**(`frame_local_net_kind`) · 뮤테이션 5/5 · ⚠️ **output/inout 은 좁혀서 잔여** |
| ~~P1~~ | ✅ **완료(2026-08-18)** — ⚠️ **census 가 범위를 셋에서 하나로 줄였다**(나머지 둘은 이미 열려 있었다) | inner-NET shadow · 6형태 iverilog 일치 · **generate 바디 삭제 재발 없음** | — | ⭐ 필요했던 것은 이름 집합이 아니라 **선언 블록의 SPAN**(`hoist_block_local_nets` 가 이미 들고 있었다 · span 은 order-independent AST 사실) · 뮤테이션 6/6 |
| ~~6~~ | ✅ **§3.5-③ 완료(2026-08-19)** | `repeat` 가족 **전부** 프레임에서 돈다 + 동시 활성화 판별자 | — | 카운터를 **예약 패스**에서 프레임 창 안에 · 핸드오프는 **span 키** · ⭐ 곁가지로 **§2 self-width 절단 silent-wrong** 동반 수정 |
| ~~7~~ | ✅ **§3.6-② 완료(2026-08-19)** | verilator 3/3 일치 + ⭐ **중복 선언 loud** 신설 | — | AST 변종 1(**`.vu` 재핀** · sim-ir·format_version 불변) · `default clocking` 과 **같은 자리·같은 리셋** · 적용은 **multi-clock 게이트보다 먼저** · 뮤테이션 4/5 + **증명된 등가 1** |
| ⚠️ ~~8~~ | **§3.11 — 지어서 재고 되돌렸다(2026-08-19)** | — | ⇒ 선행조건이 **바뀌었다**: ⓐ 인라인이 피연산자를 **한 번만** 이름 부르게 하거나 ⓑ **codegen 쪽**(`is_codegen_able` 의 `Terminator::Call` 거부)을 연다 | 전 스위트 **15 실패** · `$random` 이 **두 번** 뽑혔다 = 인라인 경로의 **핀된 사다리 위반**을 상속한다 |
| ~~9~~ | ✅ **staged `file:line` 완료(2026-08-19 · format v28)** | `velab` 진단이 one-shot 과 **바이트 동일**한 위치를 갖는다 | — | `.vu` 에 source-map tail(`(name,text)`+세그먼트 · `dir`/`canon` 은 3-OS 동일성 때문에 제외) · `velab` 이 재구성해 **같은 `MapResolver`** 설치 · ⚠️ `-L` 병합 경로는 좌표공간 겹침으로 잔여 |
| ~~10~~ | ✅ **§3.10 런타임 `file:line` 완료(2026-08-19 · format v29)** | severity 가족 전부가 `file:line:col [in 인스턴스] [at time N]` 을 갖는다 · staged 바이트 동일 | — | elaborate 가 **한 번** 풀어 `severity_locs` 사이드카로 · emitter 셋이 **한 헬퍼** 공유 · sid 없는 진단군(RunRange 등)은 측정된 잔여 |

⭐ **1~5 는 서로 독립이고 선행조건이 없다** — 순서는 *"리포터가 실제로 막힌 정도 × 크기"* 로 정했다
(1 은 리포터가 **문구 때문에 두 번 헛수정**했고, 2 는 **한 플래그 배선**, 4 는 **sign-off 에서만 터지는
유일한 부류**). ⚠️ **1·3 은 같은 결함 클래스**(폴더에 arm 이 없다)지만 **도메인이 달라**(u32 vs
`BitPacked`) 한 슬라이스에 묶지 마라 — 묶으면 한쪽 오라클이 다른 쪽을 못 지킨다.

#### 4.5.341 외부 aes_top 3판 — 로그를 데이터로 쓸 수 있게, 그리고 그 과정에서 찾은 escape 다섯 (2026-08-18 · MsgCode 67)

**리포트에 값 결함은 또 0이었다**(14 타깃 회귀 전량 PASS · GCM/XTS/CMAC/CCM · P7 통합 · 세
백엔드 byte-identical). 11 항목이 전부 **로그가 무엇을 말하는가**에 관한 것이고, 그중 하나를
그라운딩하다 **리포터가 볼 수 없었던 값 결함 다섯**이 나왔다.

⭐⭐ **§3.2 — 요청은 "경고해 달라"였고, 재니 표준 escape 다섯이 조용히 틀려 있었다.**
리포터는 `"\r"`이 vita에서 CR·xrun에서 문자 `r`이라 `.vec` 파서의 줄끝 정리가 깨져 **CMAC
위조 807건이 "MAC 불일치"로 찍혔고 sign-off를 2회 왕복**했다고 적었다(증상이 DUT 버그로 보였다).
그들은 자기 코드를 고쳤고 요청은 경고뿐이었다. **iverilog + verilator로 그라운딩하자 더 큰 것이
나왔다** — IEEE 1800-2017 **Table 5-1이 정의하는** `\ddd`·`\xhh`·`\v`·`\f`·`\a` **다섯이
전부 문자 그대로 읽히고 백슬래시까지 남아** 값도 폭도 틀렸다(`"\v"` = 118·16bit / 오라클 11·8bit).
⚠️⚠️ **리포터가 제안한 우회 `"\015"`·`"\x0D"`가 바로 그 다섯 중 둘이다** — 그대로 썼으면
NUL+"15"(24bit)가 됐다. 정본 unescape 한 함수에 Table 5-1을 채웠다(octal 1~3자리·hex 1~2자리
경계 포함, 전부 iverilog 핀). ⭐ **`\r`은 값을 안 바꾼다** — IEEE가 정의하지 않고 **오라클이
갈린다**(verilator는 vita와 같은 0x0D · iverilog/xrun은 문자 `r`)이므로 따를 다수가 없고,
바꾸면 지금 그것에 기대는 설계를 조용히 깨뜨린다 ⇒ **W3059 신설**로 silent→loud만 올렸다.
⭐⭐ **119 형태 sweep의 결과가 그 정책의 증명이다** — **40 일치(전부 Table 5-1) · 79 발산(전부
미정의 escape) · 조용한 발산 0 · 과잉경고 0.** 즉 **발산 집합 = 경고 집합**이다.
⚠️ 그 sweep을 처음 돌렸을 때 "0 발산"이 나왔는데 **BSD `join` 문법 오류로 0행을 비교**하고 있었고,
두 번째 판에서는 `64'(...)` 캐스트를 써서 **escape가 아니라 캐스트를 재고** 있었다(iverilog가
그 캐스트에 0을 답한다) — 둘 다 잡아서 다시 쟀다. ⚠️ 세 번째로 "`\``가 안 warned"가 나온 것은
**내 정규식이 백틱 안의 백틱을 못 잡은 계측 오류**였다.

⭐ **§3.4 — 경고가 일어난 일의 반대를 말하고 있었다.** `#(.MODE("Y"))`가 `parameter string`에
**적용되는데도** `W3056 … default kept`를 냈다. 원인은 오버라이드가 **세 채널**(i64 `value` ·
`fill` · `str`)로 가는데 경고가 **`value` 하나만** 물은 것 — 그리고 `params.rs`의 escalation은
**이미 셋을 다 묻고 있었다**(`has_applied_override`). ⇒ 레코드를 **먼저 짓고 경고가 그 레코드를
읽게** 바꾸고, 접합을 `ResolvedOverride::keeps_default_of` **한 철자**로 공유했다(네 사이트).
⚠️ **틀린 진단은 없는 진단보다 나쁘다** — 사용자는 불필요한 워크어라운드를 짜거나 **맞는 결과를
의심**한다.

⭐⭐ **§3.8 — 화면에 찍힌 유일한 문자열로 아무것도 못 했다.** 진단은 `VITA-W3056`만 찍는데
`-Wno-`/`-Werror=`는 **mnemonic만** 받았다(`explain`은 둘 다 받았다 = **한 질문의 두 철자**).
⇒ `MsgCode::resolve` **하나**를 만들어 mnemonic·`VITA-W3056`·`W3056`을 대소문자 무시로 받고
**세 소비자가 전부 그것을 부른다**. `-Werror=all`도 신설(`--help`가 늘 "all, or one code"라고
적어 뒀는데 그 절반이 없었다). ⭐⭐ **그리고 렌더러가 mnemonic을 함께 찍는다** — doc-15의
**worked example 55개 중 42개가 이 형태**인데 제품은 하나도 안 찍고 있었다. 그것이 필요한 이유는
편의가 아니다: `VITA-W3056`은 **범용 simplification 채널**이라 그 번호로 끄면 무관한 진단까지
꺼지는데, mnemonic이 보여야 독자가 그 사실을 안다.

⭐⭐ **§3.10 — picorv32 진단 71건 중 `file:line:col`을 가진 것이 0이었다.** 모델은 이미
`location`·`context`(계층 프레임)·`sim_time` 셋을 들고 있었고 렌더러도 `location`을 찍는데,
**emitter가 안 채웠고 `context`는 아무도 채우지도 찍지도 않았다**(35 생성 사이트 중 33이
`location: None`, `context` 채우는 곳 **0**). 셋을 지었다 — ⓐ **인스턴스 경로**(`cur_prefix`가
곧 그것이다 · 렌더러가 `[in tb.u_m1.u_a]`) ⓑ **인스턴스화 지점을 subtree의 ambient span으로**
(포트 배선·파라미터 바인딩은 어떤 문장보다 먼저 도므로 `cur_span`이 비어 있었다 · 루트는
자기 선언으로 폴백) ⓒ **런타임 진단이 배열을 이름으로 부른다**. **결과: 그 71건 중 elaborate
58건이 전부 위치를 얻었다**(0 → 58). ⭐ 리포터의 최소 재현 4줄이 이제 **pairwise distinct**이고,
그중 둘은 **같은 소스 줄**이라 `file:line:col`만으로는 원리적으로 못 가른다 = 인스턴스 경로가
장식이 아닌 이유. ⚠️⚠️ **배열 이름은 채널이 bool 하나였다** — 지연 보고 큐 `pending_range:
Vec<bool>`이 넷을 잃는다. ⭐ **`WOp::LoadIdx`에 넷 필드를 붙이지 않았다**: `WOp`의 최대 payload가
16바이트라 네 번째 워드는 **모든 op을 24→32바이트**로 키우고 그 레인은 §5.1-ay가 런의 ~20%로
쟀다 ⇒ **cold path에서 `off → net` 역인덱스**(offset은 `words*2*elems ≥ 2`씩 증가하므로 **엄격
증가 = 유일**하고, 그 성질을 코퍼스 테스트가 `net_at_off`가 항등 역함수임과 함께 단언한다).

⭐ **§3.9 — `--help`에 없는 플래그는 없는 것이다.** `--obs-dir`/`--probe`/`--hier-tree`/
`--inst-paths`가 전부 빠져 있었고, 리포터는 `--obs-dir`을 **6일 만에 무관한 에러 메시지 안의
한 문장에서** 찾았다. observability 절을 신설했다(⚠️ 앵커가 **각 플래그가 실제로 동작하는지도**
확인한다 — 없는 플래그를 적는 것이 안 적는 것보다 나쁘다).

⭐⭐ **§3.11 — CHANGELOG가 틀렸고, 판별자는 키워드 하나였다.** *"Writing transforms as
`function` costs nothing: vitamin inlines them during elaborate"* 에 대해 리포터는 codegen
**12–16%**·거부의 **87%가 `user_call_in_expr`**를 실측했다. 재측정: **`function`은 인라인되고
`function automatic`은 안 된다**(모듈/패키지 무관). `automatic`이 비용이고 그것이 스타일 가이드가
요구하는 철자다 ⇒ CHANGELOG를 정정하고 확장은 ROADMAP §3으로. ⚠️ **picorv32는 어느 형태도 안
써서 65/68이다 — 이 축은 벤치가 대표하지 않는다.** 그리고 보고 절반을 지었다: **`run.json`이
`elab_s`/`sim_s`로 앞단과 시뮬을 가른다**(`wall_s` 하나로는 귀속이 불가능하고, `--backend` 비교는
앞단이 동일한 런들의 비교라 **앞단 지배 설계는 무조건 "backend가 안 듣는다"로 읽힌다** — 리포터가
정확히 그 결론을 냈다). ⚠️ **determinism golden이 내 실수를 잡았다** — 새 필드가 비결정적인데
isolated 목록에 없어 `two_runs_byte_identical_bar_wallclock`이 빨갛게 됐다.

⚠️ **기존 핀 4개가 걸렸고 하나는 갱신이 아니라 재조준이었다** — `count(&err, "R")`가 **단일 문자
"R"**을 세고 있었다(우연히 3이었고 mnemonic이 붙자 18). 나머지 셋은 mnemonic/`[in …]` 추가분 반영.

**게이트**: 전 스위트 **5532 green**(신규 앵커 15 + 코퍼스 성질 2) · clippy 3구성 0 · fmt ·
no-oracle lib green · MsgCode **66 → 67**(bijection이 doc-15 항목을 강제) · escape 119형
3-오라클 sweep 발산=경고 · `examples/` stdout+VCD byte-identical · keccak·picorv32 3-way
byte-identical · 뮤테이션 17건.

#### 4.5.340 외부 round-29 — correct-or-loud 의 loud 쪽을 읽을 수 있게 (2026-08-18, branch feat-round29-diagnostic-quality, **format 27**) ✅

**입력.** 값 결함 **0** 인 리포트. 리포터는 코퍼스 전량을 새 기본 백엔드(`native`)로 돌려
4 TB · 가드 105 · KAT ×3 · 무제한 스윕 **793 PASS/0 FAIL** · `+CAVP_PARTIAL_RUN` 652 ·
**3-way(native/vm/interp) 219 시나리오 stdout 바이트 동일**을 확인했고, Phase A 가 tier-3 로 옮긴
표면 중 자기 TB 가 안 쓰는 것 **14 종**을 따로 probe 로 지어 돌렸다(12 PASS · 2 는 loud).
⇒ **찾은 것은 전부 진단 품질이다**: `correct-or-loud` 는 loud 쪽이 **쓸 만한지**에 대해 아무 말도
하지 않는다.

**R29-1 — 모든 구문 오류가 Rust 토큰을 유출했다.** 한 사이트가 `{found:?}` 로 렌더해서
사용자 메시지가 vita 의 렉서 enum 을 실었다(`expected '(' after 'if', found Word(Keyword(End))`).
앞 절은 사용자의 언어이고 뒤 절은 우리 것이다 — 독자는 소스로 역번역해야 하고 로그 소비자(G2 OBS)는
vita 의 토큰 타입을 알아야 한다.

⭐ **철자는 span 에서 복구 가능했다 — 다만 에러가 이미 들고 있던 span 으로는 안 된다.**
`error_at` 은 **더 이른 노드**에 보고하는데 `found` 는 커서 토큰이라(`g[w].u.q` 에서 앵커는 `w`,
커서는 `.`) 보고 앵커를 슬라이스하면 **`found` 에 든 것이 아닌 토큰**을 인용하게 된다. `ParseError`
가 둘을 **별개 필드**로 들고 `found_span` 만 슬라이스한다. 키워드와 식별자는 **이름을 붙인다**
(그 둘은 맨 철자가 *왜* 틀렸는지를 말하지 않는다 — `end` 만 보면 이름처럼 읽힌다), 나머지는 그냥 인용.
텍스트를 복구 못 하면 **`found` 절을 버린다**(실토큰에는 도달 불가지만, 틀린 `found` 는 없는 것보다 나쁘다).

**R29-2 — clocking `default` 아이템: 깨진 템플릿이 없는 길을 가리켰다.**
`default input #1step output #0;` 는 실제 clocking 블록 대부분의 첫 줄인데, 리포터가 시도한 **5/5** 가
`expected {X}` 슬롯에 **문장 하나가 통째로** 들어간 메시지로 거부됐고, 그 메시지가 `default skew only`
= *"수식어 없는 `default #1step;` 은 된다"* 로 읽히는데 **그것도 같은 메시지로 거부**된다. 반대쪽
진단(per-signal `#0`)의 *"`#1step` is accepted as the explicit default"* 는 첫 메시지 옆에서 읽으면
**방금 거부된 것을 쓰라는 지시**다.

이제 IEEE 1800-2017 §14.3 `default_skew` 를 **파싱하고**, skew 를 자기가 선언하지 않은 아이템에
**스탬프**한다. 그것이 설계의 전부다 — **두 번째 skew 메커니즘이 아니므로** 어느 skew 를 이 서브셋이
받는지는 **여전히 elaborate 의 술어 하나**가 정하고, 그 거부가 이제 **쓴 대로의 skew**(`#0`, `0` 아님)와
**착지한 신호**를 부른다(블록 전역 default 는 한 번 쓴 skew 를 여러 아이템에 놓으므로 "어느 신호" 는
소스를 읽어서 복구되지 않는다). 맨몸 `default #1step;` 은 **언어의 문법 오류**이므로(방향 키워드가
optional 이 아니다) 그대로 loud 이고 메시지가 **그 사실**을 말한다. 두 번째 `default` 는 자기 키워드에서
loud(둘에는 정의된 읽기가 없다).

**R29-4 — 런타임 진단이 언제인지 말하지 않았다.** 한 실행이 `file:line:col` 을 붙이는 진단(parse)과
아무것도 안 붙이는 진단(runtime)을 섞었다. 리포터의 설계는 **11 파일 22 곳**의 `unique case` 와 여러
인덱스 배열에 대해 `W4029` 8건 + `W4007` 1건을 내는데 출력만으로는 하나도 안 좁혀진다.

엔진은 span 이 아니라 IR 위에서 도니 위치는 없다 — ⭐ **그런데 시각은 빠진 적이 없다**: `sim_time` 은
Diagnostic 에 이미 있고 **모든 런타임 emitter 가 찍는다**(34 사이트 중 29 · 나머지 5 는 `RtlOutput`
둘과 **실행 밖에서 나는 진단 둘**[백엔드 폴백·wide-arith 요약]이라 `None` 이 맞다). **렌더러가 버리고
있었다.** 이제 ` [at time N]` 을 붙인다 — `simulation ended (…) at time N` 과 **같은 시계·같은 문구**.
컴파일 시점 진단은 `None` 이라 그대로다(거기에 0 을 찍으면 **아무것도 아닌 것에 대한 사실**이 된다).
⭐ 5,496 테스트 중 **정확히 둘**이 움직였고 **둘 다 강화됐다**(이제 두 deferred 리포트가 **같은 슬롯**에서
성숙한다는 것을 핀한다 — 그 테스트들의 주제가 바로 그것이다).

**R29-3 — 위반 보고가 RTL `$warning` 과 한 코드였다.** 파서가 위반 arm 을 **문자 그대로 `$warning`
문장으로 desugar** 하므로 `W-RUN-USER-WARNING` 하나에 둘이 앉았고, 리포터가 잰 두 방향이 **전부 참**이다 —
ⓐ 알려진 benign unique-case 하나를 죽이려고 `-Wno-` 를 켜면 **트리의 `$warning` 이 전부** 사라지고
ⓑ 이 저장소의 doc-15 가 CI 게이트로 명시한 `-Werror=W-RUN-USER-WARNING` 이 **`$warning` 이 한 줄도
없는 설계를 깨뜨렸다**(실측 `errors=2`). IEEE 는 둘을 다른 절에 둔다 — §12.5.3 은 **시뮬레이터가
만드는 보고**, §20.10 은 **설계가 부른 태스크**.

지은 것: **`W4031 · W-RUN-UNIQUE-VIOLATION`** · `SeverityKind` 가 **severity 가 아니라 클래스**가 되고
(`UniqueViolation` 은 Warning 으로 렌더되지만 코드가 다르다) 그 매핑이 **`diag_class` 한 철자**가 된다 —
⭐ 엔진에는 emitter 가 **둘**(문장 경로·프레임 경로)이고 **그 전엔 두 번째 철자였다**(둘 다 `_`-free 라
새 variant 가 둘 다를 세우기는 했지만, 한 철자면 애초에 갈릴 자리가 없다). 파서 desugar 는 이제
`$__vita_unique_violation` 을 낸다.

⭐ **곁가지로 `$__vita_` 를 예약 네임스페이스로 만들었다.** desugar 가 elaborate 에게 말하는 채널은
**이름뿐**이고(마커 필드는 frozen AST 형상을 바꾼다) 그 이름을 소스가 쓸 수 있으면 **위반하지도 않은
§12.5.3 violation 을 소스가 파일할 수 있다**. 규칙이지 목록이 아니라서 **다음 desugar 도 자동으로 덮인다**.
진입점 **둘 다**(문장·표현식) 거부해야 채널이 반만 사적이지 않다.

⚠️ **format_version 26 → 27, 그리고 그 이유가 하위호환인데도 bump 다.** `UniqueViolation` 은 기존
`SeverityTable` 트레일러에 **마지막으로** 붙으므로 postcard discriminant 상 **v26 값은 전부 그대로
디코드된다** — 구 아티팩트는 mis-decode 하지 않는다. 깨지는 것은 **새 아티팩트 × 구 바이너리** 하나이고
그것도 loud 이긴 하다(`undecodable .velab severity trailer`). **bump 하는 이유는 그 loud 가 고치는 법을
말하지 않기 때문**이다 — 헤더 게이트의 `E-ART-FORMAT-MISMATCH` 는 말한다. **더 정확한 loud 를 사는 것**이
이 bump 의 내용이고, ENGINEERING_RULES 의 bump 사유가 2종 → **3종**이 됐다.

**§5 성능 — 재현 실패, 그러나 지적한 벤치 갭은 진짜였다.** 리포터는 자기 트리에서 native 가
Keccak 지배 워크로드에서 **~5% 느리고** SHA-2 지배에서는 1.09~1.76× 빠르다고 쟀고, *"비용이 body 실행기
밖에 있어서 tier-3 오버헤드만 남는다"* 로 진단했다. ⭐ **구조적 지적은 옳고 고칠 값이 있었다** —
`perf_baseline.rs` 의 여덟 형태가 **전부 절차 `always @(posedge clk)` 바디**에 일을 둔다 = 바디 사이드
백엔드가 가속하는 바로 그 절반 = **한 레짐의 여덟 표본**. 그래서 두 형태를 더했다:
`cont-assign-heavy`(Keccak 스타일 조합 라운드를 **연속대입**으로 · 바디는 NBA 다섯)와
`heap-heavy`(string/queue 처닝 = 벡터 파싱 TB 의 레짐).

⚠️⚠️ **둘 다 반대로 갔다** — native/vm **0.76** · **0.86**. ⭐⭐ **그리고 왜 그런지가 메커니즘을
반증한다**: 두 형태 모두 **vm/interp 0.97** = 바이트코드 VM 이 사실상 기여 0 인데(컴파일할 바디가 없다)
native 는 **15~24% 빠르다** ⇒ tier-3 의 이득이 바디 컴파일에서만 오는 게 아니라 **넷/settle 경로
(아레나)에서도** 온다. 이 저장소의 실물 Keccak-f[1600] 도 같은 답이다(`keccak_f` 0.85 ·
`keccak_f_arr` 0.93 · `keccak_f_flat` 0.63). ⇒ **그들의 숫자는 그들의 설계에 대한 측정으로 남기고
메커니즘은 미확인으로 기록한다** · 다음 프록시는 ROADMAP §5.2 에 이름을 적었다(그 워크로드는 199 초 동안
**0.87 MB 의 CAVP 레코드**를 처리한다 ⇒ Keccak 산술이 아니라 **레코드 파싱**이 지배일 수 있다) ·
**8/8 → 10/10** 이고 이제 그 문장은 **두 레짐**에 대한 진술이다.

⚠️ **절차 사고 — stale staged bin 이 거짓 신호를 냈다.** `vcmp`/`velab`/`vrun` 은
`required-features = ["separate-bins"]` 라 `cargo build --workspace` 에 **안 들어간다**. 12일 된
바이너리가 *"staged 경로가 옛 코드를 쓴다"*(W4007 + 타임스탬프 없음)로 보였고, 제대로 빌드하니
staged 도 W4031 + `[at time 0]` 이다. ⚠️ 그리고 **`-p cli` 를 no-oracle lib 게이트에 더하지 마라** —
그 조합은 **pre-existing 로 E0004** 다(lib **테스트** 타깃이 dev-dep 으로 sim-engine 의 `oracle` 만
되살려 `Backend` 는 세 variant 인데 cli 의 `#[cfg]` arm 둘이 잘린다). CI 는 `-p sim-engine` 만
테스트하므로 그 축은 초록이고 **135 green** 이 맞다 → ROADMAP §3 등재.

**게이트.** 전 스위트 **5,517 green**(신규 앵커 26) · clippy **3 구성** 0(workspace/no-oracle/separate-bins) ·
fmt 0 · no-oracle lib **135 green** · MsgCode **65 → 66**(bijection 게이트가 doc-15 항목을 강제) ·
`examples/` 4종 stdout·VCD **바이트 동일** · keccak·picorv32 **3-way 바이트 동일** · staged
`vcmp→velab→vrun` 이 새 클래스를 나른다.

#### 4.5.339 elaborate 상수접기의 `**` 지수 자기결정 (2026-08-18, branch feat-constfold-pow-exp-selfdet, format 26 불변) ✅

**표적.** ROADMAP §2 의 *"elaborate 상수접기가 `**` 지수를 자기결정하지 않는다"* — §4.5.319 가
다섯 철자(엔진 `arith`·`lower_expr_ctx`·`expr_size_ctx`·연속대입 하강·bytecode
`const_pow_exponent`)를 닫으며 **마지막 하나**로 남겨 둔 것. IEEE 1800-2017 §11.6.1 Table 11-21
은 `**` 의 지수를 **자기결정 위치**로 정하는데 상수 도메인의 i64 walk 는 폭이 없어
`-4'sd8`(4비트 패턴 `1000` = −8)을 **+8 로 넓혔다**:
`localparam signed [15:0] P = 4'sd3 ** -4'sd8` 이 **6561** / iverilog·**vita 자신의 런타임** **0**
(Table 11-6: 음수 지수 ∧ |base| > 1 → 0) ⇒ **같은 소스가 param 과 런타임에서 다른 값**.

**수정 = 새 평가기가 아니라 재사용.** §4.5.186 이 이미 폭-인지 상수 평가기를 갖고 있다
(`eval_const_env_self` = 자기결정 위치 전용 진입점 · 시프트 카운트·삼항 조건·시스템함수 인자가
쓰던 것). 지은 것은 **헬퍼 하나** `const_pow_exponent_selfdet` 이고 **세 fold 사이트가 그것을
공유**한다(`const_eval_in_scope` Binary arm · `eval_const_env` Binary arm · `eval_const_env_at`
Pow arm) — 지수 규칙이 module-scope fold 와 상수함수 인터프리터 사이에서 갈리지 않게. 곁들여
필요한 것 셋:

- `const_self_width` 의 **`Concat` arm**(파트 자기폭의 **합** · §11.8.1).
- `const_self_width`/`const_signed_env` 의 **Named 캐스트 arm** — `RPS'(e)` 는 `4'(e)` 와
  **한 구문의 두 철자**이고 `cast_size_bits` 가 둘 다 답한다(부호는 정본 `const_expr_signed` 규칙).
- `eval_const_env` 의 **module-scope 위임** — 그 env 쌍둥이는 `pkg::X`·concat·`$bits`·캐스트를
  모델하지 않아 leaf 지수가 통째로 loud 였다.

**⚠️⚠️ 첫 설계를 적대 라운드가 반증했다 — 거부는 caller 만큼만 loud 다.**
초판 헬퍼는 *"wrap 가능한 모양인데 자기폭을 모르면 **거부**"* 였다(무제한 도메인으로 degrade
하는 것이 지수에 대해서는 곧 결함이므로). differential 렌즈가 **두 방향으로** 반증했다:

- **range-bound 위치에서 그 거부는 조용하다** — `logic [f3():0] v;` 의 fold 가 decline 하면
  바운드 소비자가 **기본값을 넣는다**(pre-existing) ⇒ PRE·iverilog 가 10비트인 넷이
  **1비트 · exit 0 · 진단 0** 으로 지어졌다 = **correct→silent-wrong**(사다리 하강).
- **값이 정확한 셀을 강등했다** — `3 ** (m + 0)`(다중 packed 로컬)은 폭이 미상이어도 랩이 없어
  값이 정확한데 localparam 위치에서 correct→**loud**.

⇒ **거부를 걷고 폭 모델을 완성**했다. 그러자 거부되던 칸이 **정답으로 접힌다** —
`4'sd3 ** (RPS'(2) - 4'd9)` = **19683** = iverilog(PRE 는 **조용한 0**). ⭐ **loud 로 사는 대신
접어서 사는 쪽이 사다리 위**이고, 그 선택지는 폭을 **알 수 있게** 만들었을 때만 생긴다.

**⚠️ 깊이 청구도 과했고 자체 측정이 잡았다.** 위임 대상 `const_eval_in_scope` 는
`eval_const_call` 을 **depth 0 으로 재시작**하므로, 인자·default 를 `depth` 그대로 접던 기존
코드와 합쳐지면 self-referential default 가 **상수 깊이로 재귀**한다(`input int k = 8'(f())` 가
PRE 의 E3009 → **스택 오버플로 exit 134**). 첫 처방은 *"인자·default 둘 다 `depth + 1`"* +
*"위임은 `depth == 0` 에서만"* 이었고 **둘 다 과했다**:

- **인자는 유한 AST 하강이라 사이클을 못 만든다**(`g(g(g(0)))` 은 서로 다른 세 노드)인데
  청구하면 예산이 반이 된다: **65단 인자 중첩이 PRE·iverilog 는 65 로 접히고 청구 판은 E3009**
  = correct→loud ⇒ **사이클은 default 위치에만 있다**(default 식은 **callee 자신의 선언**에
  속해 같은 노드로 재진입한다) ⇒ **default 만 청구한다.**
- **`depth == 0` 만으로 걸면 접힐 default 를 못 접는다** — `input int k = 8'(3)` 은 call 이 없어
  깊이를 되돌릴 것이 없는데 loud 였다 ⇒ 게이트는 **`depth == 0` ∨ call-free 서브트리**(서로소인
  두 안전 이유). ⭐ **call-free 단독으로 걸어도 안 된다** — module-scope 정상 셀
  (`2 ** (8'(cf(2)) + 1)` = 64)이 correct→loud 가 된다(적용 전 자체 재검토가 잡았다).

부수 이득: **pre-existing 이던 bare 철자**(`input int k = f()`)도 이제 loud 다(크래시→loud).
⚠️⚠️ 그리고 soundness 가 **내가 그 게이트에 쓴 주석이 거짓**임을 실측으로 잡았다 —
*"depth 0 이면 call 이 안 떠 있다"* 는 **틀렸다**(body-local init 과 `eval_const_env` 의 `Call`
arm 이 평범한 caller depth 로 접는다 · 프로브가 **기존 스위트 세 테스트에서 발화**). 실제로
종료를 보장하는 것은 **`envw` 가 init fold 보다 먼저 채워진다**는 것과 **default 만 청구**라는
두 사실이고, 주석을 그렇게 다시 썼다(`env.is_empty()` 는 lockstep 쌍둥이 · 뮤테이션이 등가로
측정됐고 그 이유를 doc 에 남겼다).

**⚠️⚠️ 뮤테이션이 "등가" 를 반증했다 — Named 캐스트의 부호는 값을 바꾼다.**
`const_signed_env` 의 Named arm 을 `false` 로 되돌리는 뮤테이션이 **28 + 이웃 157 테스트를
통과**했고, 판별자를 지어 재니 등가가 아니었다: `RPS'(2) - 4'sd9` 는 4비트에서 9(`1001`)인데
문맥이 **signed 면 −7**(Table 11-6 → **0**) · **unsigned 면 9**(3^9 = **19683**). iverilog·정본
= `0 0 27` / 뮤턴트·**PRE** = `19683 21459 27` ⇒ 그 arm 은 폭만이 아니라 **스윕 밖 두 칸을
silent-wrong 에서 건져낸다**. soundness 가 준 두 번째 판별자는 **비교 arm**을 탄다
(`3 ** ((RPS'(4'sd2) > (4'sd7 + 4'sd7)) ? 2 : 5)` → 정본 **9** = iverilog · PRE **243**) —
Pow 문맥 핀이 못 닿는 축이라 둘 다 앵커로 남겼다.

**⚠️ genvar 부호를 함께 닫았다.** `2 ** (g - 1)`(g=0)이 PRE·iverilog **0** / 초판 **E3009**
= 이 슬라이스가 만든 correct→loud 였다(마스킹을 시작한 것이 원인). genvar 는 `params` 에만
들어가고 `param_meta` 가 없어 부호 모델이 **무부호**로 읽어 −1 이 4294967295 가 되고
`checked_pow` 가 넘쳤다. **IEEE 1800 §27.4 = genvar 는 signed 32비트 정수**이므로 값과
**lockstep** 으로 `(32, true)` 를 시딩한다(예측 아닌 실측: 비-음수 32비트 값은 두 부호 읽기가
같은 비트라 **중간값이 음수로 가는 식만** 움직인다 · 전 스위트 파급 0).

**측정(97칸 3-way · iverilog/PRE/POST).** FIXED **10** · LOUD→CORRECT **2** ·
WRONG→LOUD **2**(`0 ** 음수` = Table 11-6 의 `x` 인데 i64 도메인이 x 를 못 실으므로 **정직한
거부**) · **회귀 0**(초판의 genvar CORR→LOUD 도 닫혔다). 보너스 FIXED: **range bound**(PRE 는
`[3 ** -4'sd8 : 0]` 로 **6562비트 넷을 조용히 생성**) · `#()` override · **generate-if arm
선택**(틀린 지수가 **틀린 가지를 elaborate** 했다) · `$clog2` 지수(81→1) · `~4'd0` 패턴 지수
(−1 이 아니라 15) · Named-cast 폭·부호 칸 셋. `examples/` 4종 + keccak + picorv32
**PRE↔POST 바이트 동일**.

**게이트.** 전 스위트 **5496 green**(신규 앵커 19 = 파일 **33 tests**) · clippy 3 구성 0 ·
fmt 0 · no-oracle lib **135 green** · **format_version 26 불변**. 공유 코드 수정이라 백엔드
차분은 구조적으로 눈멀고(§5.1-e) **절대 앵커가 유일한 이빨**이다. **뮤테이션 10/10 사망** —
축: 인자 깊이 청구 · default 깊이 미청구 · 게이트 `depth==0` 단독 · 게이트 call-free 단독 ·
게이트 삭제 · Named 폭 arm → None · Named 부호 arm → false · Concat 합→max · 두 Pow arm 각각
→ 평문 walk. (`eval_const_env_at` 의 Pow arm 을 헬퍼 대신 `eval_const_env_self` 로 직접 부르는
것은 헬퍼가 **얇은 위임**이 된 지금 **구성상 등가** — 그래도 한 철자로 남긴 이유는 지수 규칙이
시프트 카운트와 갈리는 날 **한 자리만 고치면 되게** 하기 위해서다.) ⚠️ 절차 둘: M9 는 치환
패턴이 **두 사이트에 맞아** 미적용됐다(줄 번호로 재지정) · M7 복원 후 **바이너리 재빌드를
빠뜨려** 정본이 뮤턴트 값을 내는 것을 한 번 봤다(§4ⓕ 재발).

**발굴(전부 pre-existing·PRE==POST → §2 등재).** real 도메인 Pow 지수 · module-scope `$clog2`
인자 무제한 fold(인터프리터와 **같은 소스 두 답**) · replication/part-select 폭 lane 이 `**`
미fold · 캐스트 SIZE 식 무제한 fold · u64 패턴 지수 · 인터프리터의 폭-0 타깃 마스킹 ·
body-local init 조용한 0 · body-decl `int t = f();` 스택 오버플로 · self-referential 반환 range
크래시 · `coverpoint_domain` 미러 어긋남 · 그리고 **§2 의 fill-context 항목(ⓒ)은 HEAD 에서
재현 불가**(72칸 0 diff — 중간 슬라이스가 닫았다).

**잔여(일부러).** 폭-미상 leaf 위의 **wrapping** 지수는 무제한 fold 유지(거부가 더 나쁘다 —
위 실측) · `0 ** 음수` 는 loud(x 를 이 도메인이 못 실음).

#### 4.5.337 ③층 V1 슬라이스 1 — SVA (2026-08-11, format 26 불변) ✅

**한 일.** V0 이 정한 순서의 1번. **커널 코드는 한 줄도 안 늘었다** — `design_eligibility` 의
`flag(&mut out, "sva", …)` 행을 지우고 두 사이드카를 CORE 버킷으로 옮겼을 뿐이다.
네이티브 커버리지 **54.7% → 66.8%**(+760 `simulate()` 호출).

**왜 지을 게 없었나.** **SVA 는 런타임 기계장치가 아니다.** elaborate 가
`assert property(@(clk) a |-> b)` 를 `always @(clk) if (a && !b) $error(…)` 로 **desugar** 하므로
엔진에 도착하는 것은 평범한 IR 이고, SVA 고유의 런타임 상태는 StmtId 테이블 **둘**
(`assert_fire`·`assert_ctl`)뿐이다. 그리고 그 둘은 **공유 `builtins::dispatch` 안**에서 읽히는데,
tier-3 은 §4.5.293 이래 이미 거기로 배선돼 있다.

⭐ **진짜 기계장치가 필요한 SVA 형태는 이미 자기 이름으로 거부되고 있었다** — `cover property`
와 SVA liveness 는 `final_procs` 엔트리를 만들므로 `executor_rows` 가 **`final` 블록**으로
거부하고(실측: `refused: "\`final\` blocks …"`), §16.4 deferred assertion 은 `mature_deferred`
훅이 tier-3 리전 캐스케이드에 **없어서** 별도 `deferred_assert` 행이다. census 가 몫을 갈랐다 —
**`sva` 혼자 760 · `deferred_assert` 혼자 14 · 겹침 0** ⇒ 슬라이스는 `sva` 행 **하나**.

**게이트.** 전 스위트를 `Backend::Native` 기본으로 돌려 **출력 발산 0**(새 실패는 지운 행을
핀하던 게이트 테스트 하나뿐). 영구 게이트 = 차분 코퍼스에 SVA **5행** 추가(`|->` 발화 · `|=>`
pending reg · assert control · sampled-value 5종 · NBA 카운터 위 property) + **거부 이웃 핀**
(`cover property` → `final`, deferred → `deferred_assert`) + **절대 앵커**.

**⭐⭐ 이 슬라이스의 값은 게이트가 세 번 자기 자신을 반증한 데 있다.**

**① 하네스가 사이드카를 설치하지 않았다.** `build_with_opts` 는 손으로 고른 부분집합만 넣는데
거기 `assert_fire`/`assert_ctl`/`defer_marks`/`defer_acts` 가 없었다 → `$assertoff` 가 **플래그를
안 뒤집고 그냥 출력되는** 설계로 두 백엔드를 비교하고 있었다(= 설계가 말하는 것과 다른 것을
재고 있었다). 발견 방법은 **거부 이웃 핀이 `Ok(())` 를 낸 것**이다 — CLI 는 같은 설계를
`deferred_assert` 로 거부하는데 하네스는 통과시켰다. 넷을 설치했고, 전 스위트는 그대로 초록이다.

**② ⭐⭐ 네이티브-vs-VM 차분은 공유 코드에 구조적으로 눈멀었다 — 주장이 아니라 실측.**
뮤테이션 E(`assert_fire` 억제 early-return 삭제)와 F(`assert_ctl` 이 절대 disable 안 함)는
**엔진 게이트를 전부 통과**하고 `cli::sva_rest` 의 절대 핀에서만 죽었다. 두 백엔드가 한 자리에서
읽는 규칙은 **둘이 함께 움직이므로** 값 일치로는 원리적으로 볼 수 없다. → **절대 앵커**
(`sva_assert_control_actually_suppresses_exactly_one_violation`: 위반이 정확히 1건, stdout 0줄,
exit class `HadErrors`)를 지었고, 그러자 E·F 가 **엔진에서도** 죽는다. 그리고 같은 앵커가 하네스
뮤테이션 B·C 도 죽인다 — **그 전엔 둘 다 생존했다**(코퍼스 행만으로는 하네스 갭이 안 보인다).

**③ 앵커를 짓다 내 기대가 틀린 것을 쟀다.** 처음 쓴 설계는 `$asserton` 시점에 `a=1,b=0` 이
**그대로 남아** 다음 posedge 에서도 위반해서 2건이 나왔다. 재활성화 전에 창을 비워야 "정확히
하나" 가 참이 된다. 앵커가 자기 설계를 정정했다.

**뮤테이션 6/6 사망**(A 지운 행 복원 · B/C 하네스 · D 두 defer 테이블 · E/F 공유 dispatch).

⚠️ **절차 사고 — 뮤테이션 복원이 커밋 안 된 작업을 지웠다.** 스크립트가 `git checkout -- <file>`
로 복원했는데, 그것은 **뮤테이션 이전이 아니라 HEAD** 로 되돌린다. 대상 파일 둘이 이 슬라이스의
dirty 편집을 담고 있어서 행 제거와 하네스 수정이 통째로 날아갔고, 그 결과 **앵커 실패 2건과 가짜
kill 2건**이 나왔다. 복원은 **바이트 스냅샷**(`cp`)이어야 한다. ⭐ 그리고 D 의 첫 생존은 게이트가
약해서가 아니라 **뮤테이션이 부족해서**였다(거부 행이 `defer_marks.len() + defer_acts.len()` 인데
하나만 지웠다).

**게이트 결과.** 5380 tests green(신규 3) · clippy 0 · fmt 0. `examples/`·`bench/` 바이트
동일성은 **기계적** — 그 설계들은 assert 사이드카가 비어 있어 `design_eligibility` 가 같은 맵을
낸다.

#### 4.5.336 ③층 V0 — 커버리지 격차를 처음으로 쟀다 (2026-08-10, format 26 불변, 트리 변경 0) ✅

**한 일.** ROADMAP §5.1 의 V0: **기본 백엔드를 `Backend::Native` 로 flip 하고 전 스위트를
돌렸다.** 구현이 아니라 **계기**다 — 측정이 끝나고 스캐폴드는 통째로 되돌렸다.

기법은 이 저장소가 이미 기록해 둔 것이다. `Backend::Bytecode` 의 doc comment 가 직접
말한다 — *"the one that actually found the four defects that were hiding behind a green P5 —
running the entire workspace suite with this default in place … a corpus differential is far
weaker than 5000 real tests."*

**결과.**

| | 값 |
|---|---|
| `simulate()` 호출 | **6,251** |
| **네이티브 실행** | **3,417 (54.7%)** |
| fallback | 2,834 (45.3%) — design 2,230 · storage 496 · executor 108 |
| 테스트 | 5377 중 **5374 통과** |
| **출력 발산** | **0** |

⭐⭐ **실패 3건은 전부 "기본 백엔드가 vm 이다" 를 단언하는 테스트다**
(`the_default_backend_is_the_vm` · `run_json_codegen_is_backend_invariant_and_backend_is_recorded` ·
`run_json_codegen_pins_the_vm_claim_and_reasons`). **오늘의 ③층은 자기가 받아들이는 절반에
대해 이미 바이트 정확하다** — 남은 것은 전부 커버리지이고 정확성 부채가 아니다. 이것이 V1 을
"기능을 옮기는 일" 로 만든다(디버깅이 아니라).

**⚠️ 계기가 두 번 틀렸고, 두 번 다 측정 자신이 잡았다.**

**① `SimOpts::default()` 가 enum default 를 안 쓴다.** `backend: Backend::Bytecode` 를
**하드코딩**하고 있다. `#[default]` 만 옮기면 움직이는 것은 `opts.backend.unwrap_or_default()`
를 부르는 **CLI 절반뿐**이고, sim-engine 자체 테스트는 전부 `SimOpts::default()` 를 지어 예전
백엔드로 돈다. **"기본값을 뒤집었다" 가 참이 되려면 두 자리를 뒤집어야 한다** — 그 사실을
발견한 방법은 `grep "backend:"` 였고, 발견 못 했다면 커버리지를 **CLI 만으로** 재고 그것을
전 스위트라 불렀을 것이다.

**② 병렬 census 의 찢어진 쓰기.** `writeln!` 은 **unbuffered `File`** 에 대해 포맷 조각마다
`write(2)` 를 낸다. nextest 는 테스트마다 프로세스이므로 그 조각들이 서로 끼어들어
`okok`·`designrow`·바이너리 경로가 붙은 이유 문자열이 나왔고, **행 수가 1.85× 부풀었다**
(11,563 vs 실제 6,251). 수정 = 미리 포맷한 한 줄을 `write_all` 로 **한 번**(O_APPEND 는 작은
쓰기가 원자적). ⭐ 판별자는 **레이아웃을 아는 파서**였다 — census 스크립트가 알려진 층 이름
집합에 없는 값을 만나면 오염을 **선언**한다. 눈으로 보고 알아챈 게 아니라, 스크립트가 말했다.

**⭐⭐ 그리고 첫 census 는 V1 을 정렬하지 못했다 — 게이트가 단락하기 때문이다.**

프로덕션은 design → storage → executor 순으로 **먼저 걸리면 멈춘다**(그래야 빠르고, 그것이
옳다). 그 결과 design 에서 걸린 설계의 storage/executor 상태는 **측정되지 않고**,
*"행 X 를 닫으면 몇 개가 네이티브가 되는가"* 는 원리적으로 답할 수 없다. 계기를 고쳐 **세 층을
독립으로** 물었다(층마다 `.err()` 를 따로 계산해 벡터로 기록).

그러자 구조가 나왔다:

⭐⭐ **design 게이트와 storage 게이트가 같은 기능을 두 번 이름 부른다.** `string` 넷은
`design_eligibility` 의 `D:string` 행이면서 동시에 `NetArena::buildable` 의
`"heap kind: outside R1 storage"` 다. 그래서 **`D:string` 은 369회 발화하는데 단독 원인이
0회**이고, 한쪽만 닫으면 이득이 **정확히 0**이다. `D:real`(74회) / `S:real-slot`(69회)도 같은
짝이고, `dyn_array`·`queue`·`assoc` 은 셋 다 같은 `S:heap-slot` 으로 모인다.

**⇒ 슬라이스의 단위는 행이 아니라, 한 기능이 걸린 게이트 전부의 집합이다.**

거부 설계의 blocker 개수 분포가 같은 얘기를 한다 — 1개 1,338 · 2개 1,038 · 3개 328 · 4개 이상
130. **절반 이상이 다중 blocker** 라서 행 단위 히스토그램은 순서를 거짓말한다.

**V1 슬라이스 순서 (greedy · 각 줄은 위가 전부 닫힌 전제의 한계이득).**

| # | 슬라이스 | +호출 | 누적 |
|---|---|---|---|
| 1 | **SVA** (`sva`+`deferred_assert`) | +774 | **67.0%** |
| 2 | **heap 저장 + 네 종류** | +560 | **76.0%** |
| 3 | **서브루틴 프레임** | +712 | **87.4%** |
| 4 | `stmt_effect` | +241 | 91.2% |
| 5 | class / OOP / CRV | +163 | 93.9% |
| 6 | fork | +104 | 95.5% |
| 7 | 거부 시스템태스크 | +83 | 96.8% |
| 8 | `real` | +72 | 98.0% |
| 9 | functional coverage | +64 | 99.0% |
| 10–14 | force/release · clocking · probe/stage · `final` · file-directed | +61 | **100%** |

⭐ **상위 셋이 87.4% 를 산다**, 그리고 그 셋은 vita 의 Phase-2/3+ 자산 그 자체다.
⚠️ 단독 이득은 다르다 — 3번을 **혼자** 닫으면 +545 인데 위 둘 뒤에 닫으면 +712 이고,
`fork`·`file_directed` 는 **단독 이득이 0**이다(항상 다른 것과 함께 발화).

**⚠️ 테스트 단위 귀속은 이 측정에서 쓸 수 없다.** fallback 2,834 중 **2,656(93.7%)이 CLI
서브프로세스**라 `cli::*` 통합 테스트 전체가 바이너리 경로 하나로 뭉친다(계기는 nextest 의
`--exact <name>` 을 읽는데 CLI 는 별도 프로세스를 띄운다). 단위는 **`simulate()` 호출**이다.
그 무게가 한 설계의 반복이 아님은 따로 확인했다 — 예컨대 SVA 는 CLI 테스트 **26 파일·400+
테스트 함수**에 퍼져 있다.

**게이트.** flip 상태 5377 중 5374 통과(실패 3 = 백엔드 이름 핀) · 스캐폴드 revert 후
`git status` clean.

#### 4.5.335 ③층 S4 — 스케줄 소거의 중단 판정이 발동한다 (2026-08-10, format 26 불변) ✅

**한 일.** `settle_cont_assigns` — 이 백엔드의 가장 뜨거운 루프 — 가 **연속 대입 평가마다
`Lvalue` 를 clone** 하고 있었다. `Lvalue` 는 `Vec<LvalChunk>` 를 소유하므로 **대입당·패스당 힙
할당**이다. ⭐ **강제한 것은 아무것도 없었다**: `ir` 은 이 함수의 파라미터라 `k` 와 **독립적으로**
빌려지고, 그래서 `&mut k` 쓰기를 가로질러 살아남는다. 멀티드라이버 루프의 그룹 멤버 `Vec` clone
(그룹당·패스당)도 같은 자리에 있었고 재사용 버퍼로 바꿨다.
**picorv32 0.517 → 0.504 (+2.5%)** · keccak_f_flat 1.340 → 1.329 · **native/vm 1.73×**.

**⭐⭐ 그리고 프로파일이 S4 의 나머지를 짓기 전에 결론냈다.** S4 가 실제로 겨냥하는 것(런타임
깨우기 탐색)의 몫은 `propagate` 2.1% + `WakeTable::wake` 1.4% + `pass` 구성(take+extend+sort+dedup,
패스마다) ~2% = **≈6%**. ⚠️ `settle_cont_assigns` 의 9.1% 는 대부분 **스케줄이 아니라 실제 평가**라
정적 깨우기 마스크로 바꿔도 사라지지 않는다. **남은 상한 ≈1.06× 이고 §7.3 의 중단 판정은
"이득 <1.3× 면 기록 후 유지"** → **발동. 스케줄러는 남기고 정적 마스크는 짓지 않는다.**

**⭐⭐ 고정 비용 실측이 doc-21 §6.3 을 정정했다.** picorv32 의 parse+elaborate+setup =
**19 ms · 런의 3.7%** → **시뮬레이션이 공짜여도 상한 26.8×**. §6.3 은 *"~85 ms(14%) 이므로 상한 약
7×"* 라고 적고 있었고 그것은 낡았다. **여지는 계획이 적어 둔 것보다 훨씬 크다 — 그런데 S3·S4·S5 가
그 여지를 못 가져온다**(각각 census·1.06%·3.8%).

**⇒ ③층 v1 의 성능 축은 수확 체감에 도달했다.** 1.73× 와 26.8× 사이는 vita 가 **일부러 안 하는 것**
(2-state 좁히기 · levelize)과 **없는 축**(멀티코어)이 메운다 — VCS·Xcelium·verilator 가 하는 일이
그것이고, 셋 다 정확성 계약을 거래하거나 아키텍처를 바꾼다. **다음 표적은 속도가 아니라 커버리지**
(ROADMAP §5.1 — 오너 지시: interp=오라클 + native=제품, VM 은퇴. V0 커버리지 격차 측정부터).

**게이트.** 5377 tests · clippy 0 · fmt 0 · 예제 4 + bench 4 바이트 동일(stdout·stderr·exit·VCD).

---

#### 4.5.334 ③층 S3 코드젠 본체 착수 — census 가 전제를 반증했다 (2026-08-10, format 26 불변) ✅

S3 §5 의 논거는 한 문장이다 — *"넷은 슬롯 주소로 직접 store/load — **커널 호출 없음**(여기가 ②층 JIT 이
진 지점이다)"*. **코드를 쓰기 전에 그 전제를 쟀다.**

**실행 census (picorv32, `--backend native`)**: `WProg` 실행 **6,287,743** · op **24,862,723**(평균
**3.95**) · 활성화 **662,913**(컴파일 542,907 + 워크 120,006) → **9.49 프로그램/활성화** · 활성화당 815 ns
중 `WProg::run` **166 ns**.

| WOp | % | 인라인? | | 모양 | % |
|---|---|---|---|---|---|
| `Load` | **39.3** | ✅ **이미 직접 접근** | | `L` | **28.3** |
| `Const` | 17.6 | ✅ | | `C` | **18.9** |
| **`LogBin`** | **17.4** | ❌ 공유 자유 함수 | | `LCK` | 5.5 |
| **`Unary1`** | **7.9** | ❌ | | `LU` | 2.9 |
| **`Cmp`** | **5.5** | ❌ | | `LLB` | 2.7 |
| `Splice`+`Slice` | 9.3 | ✅ | | 상위 14 | 75.3 |
| `Tern` | 1.4 | ~ | | 전체 모양 | 84 종 |
| **산술 전부** | **1.5** | ✅ | | | |

**⭐⭐ 전제가 셋 다 틀렸다.** ⓐ 인라인할 산술이 **1.5%** 뿐이다. ⓑ `WOp::Load` 는 이미 `buf[vi]` 두 줄이라
**②층 JIT 이 진 이유(*"every leaf load is a CALL back into Rust"*)를 S1 이 이미 없앴다** — 코드젠이 더
없앨 것이 없다. ⓒ 인라인 불가한 **30.8%**(`LogBin`+`Unary1`+`Cmp`)가 하필 **§4.5.315·330·331 이 "한 철자"로
통합한 바로 그 함수들**이다: cranelift IR 로 다시 쓰면 **의미 재진술**(= §4.5.279 클래스), 콜백으로 부르면
**op 마다 경계**(= ②층 JIT 이 진 모양).

**⭐⭐ 그래서 win 은 프로그램을 컴파일하는 데가 아니라 프로그램 *주위의 기계장치*에 있다.** 실행의
**47.2% 가 op 하나짜리 프로그램**(`L` 28.3 + `C` 18.9)인데, 그 두 번의 메모리 읽기 주위에서 갈라진 경로는
`wcache` borrow + `Rc` clone · `lvalue_width` 의 IR 워크 · 72바이트 `Value` 생성 · `resize` ·
이미 알려준 청크 폭의 **쓰기 퍼널 재유도**를 전부 낸다. 그 전부가 **op 당 컴파일 시점 상수**다.

**수정.** `Kernel` 에 **대입 전체**를 보는 seam 둘 — `k_eval_write_scalar` / `k_eval_nba_scalar`. **기본
구현이 지금의 두 호출 그대로**라 `K = Scheduler` 는 기계적으로 불변이고, `NativeKernel` 만 평면 경로로
오버라이드한다: 증명된 plain scalar 목적지가 한 워드면 `run_cached_wprog`(캐시 borrow **안**에서 실행 →
`Rc` clone 0) → `resize_word` → `write_chunk_word`(§4.5.332 의 평면 진입점, 두 번째 store 가 아니다).
**`Value` 가 한 번도 안 생긴다.** 목적지 폭은 워크가 아니라 슬롯에서 읽고, 그 등가성은 논증이 아니라
`debug_assert_eq!` 로 **전 스위트가 매 실행 확인**한다(`k_schedule_nba_scalar` 가 오프셋을 그렇게 묻는다).

**picorv32 0.540 → 0.517** — 슬라이스 1 의 워크 기준선(0.562) 대비 누적 **+8.0%**, native/vm **1.71×**.

**⭐ 뮤테이션 7 중 6 생존, 그리고 그것이 옳다.** 이 경로는 **정본이 강제하는 지름길**이라 값에 독립적인
내용이 거의 없다. 도달은 `panic!` 프로브로 확인(5 테스트 전부 실패)하고 생존 각각을 증명했다 —
문맥 폭 `max` 제거·저장 폭·NBA `Value` 폭은 **하류 마스킹/클램프로 값이 같고**, `resize_word` 의 `signed` 는
**`w >= s.width` 라 확장 팔에 도달 불가**(죽은 인자임을 실측으로 증명), `width > 64` 가드는 **`words == 1` 이
이미 그 뜻**이라 중복 → **삭제하고 이유를 적었다**. 캐시 키의 `signed` 만이 원리적 실재 차이인데
(`wprog` 가 비교 op 에 부호를 굽는다) **ExprId 는 소스 한 자리라 한 폭·한 부호로만 요청되어** 어떤 설계도
판별 못 한다 → 술어를 `WCacheSlot::hits` 로 **공유**해 두 독자가 어긋날 여지를 없앴다.

**게이트.** 5377 tests · clippy 0 · fmt 0 · 예제 4 + bench 4 바이트 동일 · s3 차분(코퍼스 72 × 컴파일
ON/OFF + 판별 설계 8, 각각 `acts > 0`). ⚠️ 새 행 `fused_assignment_context_widths` 의 초안은 `#1` 하나 때문에
**바디에 `Delay` 가 생겨 프로세스 전체가 워크로 떨어졌고**, 행별 `acts > 0` 단언이 그것을 잡았다.

---

#### 4.5.333 ③층 S3 슬라이스 1 — 컴파일된 바디 (2026-08-10, format 26 불변) ✅

**계획(doc-21 §5)의 S3 · 바디 코드 생성, 1단계.** S2 는 §4.5.329 에서 닫혔고 §4.5.330~332 는 그 뒤의
프로파일 주도 정리였다 — 이 슬라이스부터 다시 계획의 단계다.

**무엇을 했나.** ③층이 `run_body`(SimIr 트리 워크) 대신 `backend::vm_exec`(`CompiledBody`)로 프로세스를
실행한다. 재진술 0: `vm_exec` 는 `impl Kernel` 로 제네릭이라 `compute_effect`/`apply_effect` 와 **같은
커널 메서드를 같은 순서로** 부른다. 선택은 `NativeKernel::compiled_for`(템플릿당 1회, `SimState::vm_compiled`
와 같은 `VmSlot` 프로토콜)가 하고, 두 호출부(t0 초기화·리전 루프)는 `dispatch_body` 하나를 지난다.

**왜 이것이 S3 의 1단계인가.** §5 S3 는 *"`CompiledBody` → cranelift 함수"* 이고, 그 `CompiledBody` 는
②층 VM 의 컴파일 형태로 **이미 존재하며** `jit.rs::compile_body(&CompiledBody)` 가 이미 그것을 cranelift 로
내린다. ③층이 그것을 **갖는 것**이 코드젠의 선행조건이다.

**`CompileCtx`.** `compile_body` 의 4-튜플을 이름 붙인 구조체로 바꿨다. 두 특수화가 독립이고 ③층은 정확히
하나만 원하기 때문이다 — `plain`(넷 **저장 형태** 사실 ⇒ 어느 커널이든 참)은 받고, `natives`(②층 표현식 VM)는
**거부**한다. `NativeKernel::k_eval_native` 는 호출마다 `NativeScratch` 를 새로 만들고 ②층 프로그램을 돌리는데,
`k_eval_for_lvalue` 는 S2 가 지은 `wprog` 로 간다 — 둘 다 받는 RHS 마다 **빠른 경로를 느린 경로와 맞바꾸는**
일이 된다. `try_compile` 이 쓰는 표 `nonint` 는 옵션 **안**에 둬서 "natives 없음"이 낡은 표를 들 수 없게 했다.

**⭐⭐ 측정: 컴파일된 표현 자체는 win 이 아니다.** 같은 바이너리에서 실행기만 바꾼 A/B(best-of-6):

| 설계 | 워크 | 컴파일(융합 전) | 컴파일(융합 후) |
|---|---|---|---|
| picorv32 | 0.562 | 0.559 | **0.540 (+3.9%)** |
| keccak_f_flat | 1.365 | 1.392 | 1.382 |
| keccak_f | 1.196 | 1.185 | 1.194 |
| keccak_f_arr | 1.111 | 1.121 | 1.123 |

융합 전은 **완전한 wash** 였고 프로파일이 이유를 말했다: `k_resolve_lvalue_offsets` **3.1% 가 통째로
사라지고** malloc 7.6→5.4% 인데, 그만큼을 op 루프(`dispatch_body` 7.2→8.8%)와 `k_schedule_nba_scalar`
(3.2→4.3%)가 먹었다. **한 문장이 두 op 이 되고 그 사이에서 `Value`(72B)가 레지스터 파일을 경유한다** —
워크의 `compute_effect`→`apply_effect` 에는 그 왕복이 없다.

→ **평가+쓰기 융합**(`Op::EvalWriteScalar`/`EvalNbaScalar`): 목적지가 컴파일 시점에 증명된 plain scalar 이고
평가가 커널 호출일 때 한 op. picorv32 op **2000 → 1068(−46.6%)**, 대입 995 중 **932(93.7%) 융합**.
그 뒤 +3.9%. native/vm **1.57× → 1.64×**.

**⭐⭐ 그래서 이 슬라이스의 진짜 산출은 숫자가 아니라 판정이다.** ③층의 남은 비용은 커널 호출을 *고르는*
데가 아니라 그 *안*에 있다(`WProg::run` 20.4% · `write_lvalue` 9.8% · `settle` 8.3%). 같은 호출을 계속
하는 컴파일된 표현은 그것을 줄일 수 없다 — **S3 의 값은 전적으로 코드젠 단계(호출 자체를 없애는 것)에 있다.**

**⭐⭐ 착수 전 그라운딩이 잡은 결함 — `vm_exec` 에 문장 경계가 없었다.** `run_body` 가 문장마다 하는
`k_call_fatal`/`k_drain_diags` 가 둘 다 없다. op 열엔 경계 표시가 없지만 정확히 복원된다: 모든 문장은
다섯(+융합 둘) op 중 하나로 끝난다 → `Op::ends_statement`(`_`-free). 둘의 위상은 다르다:

- **`k_call_fatal` 은 관측된다.** `is_codegen_able` 은 주석에 *"any expr position that can REACH a frame
  Call excludes the body"* 라 적고 **lvalue 인덱스 식을 안 본다** — `mem[f(i)] = 1` 의 `f` 안 `$fatal` 이
  컴파일된 바디에 도달한다. **②층 vs 인터프리터의 실제 발산이었고 이 슬라이스가 닫는다**(사다리 상승).
- **`k_drain_diags` 는 백스톱이고 관측 불가다** — `format_args_str_with` 가 출력 줄마다, 런 루프가 바디
  뒤에 이미 드레인한다. ⚠️ **`body.rs` 가 자기 사본에 대해 이미 그렇게 측정해 적어 뒀는데** 내 첫 주석은
  그것을 잊고 과장했고 **뮤테이션이 반증했다**. 주석을 실측대로 정정.

⚠️ 경계가 **op 마다가 아니라 문장마다**인 것이 load-bearing: `ResolveOff` 뒤에서 fatal 을 소비하면
`WriteLval` 을 건너뛰어 **그 쓰기가 빚진 E4002 를 잃는다**(뮤테이션 C 실측).

**게이트.** 코퍼스 72 × (컴파일 ON/OFF) 인터리브 스트림+finish+time+exit+VCD 바이트 · 판별 설계 7(각각
`acts > 0`) · 예제 4 + bench 4 바이트 동일 · **5377 tests · clippy 0 · fmt 0**.

⚠️ **코퍼스 72 중 30 만 비공허하다** — 나머지는 바디에 `Delay` 가 있어 codegen-able 이 아니고, 그 42 는
워크를 워크와 비교한다. 개수를 따로 핀해서 숨기지 않는다.

**⭐ 뮤테이션 9 중 7 사망, 그리고 그것이 내 테스트의 구멍 둘을 찾았다.** `k_enter_body` 삭제가 전 스위트를
통과했고(`%m`/timescale 이 다른 두 프로세스가 번갈아 도는 설계가 **0개**), `k_call_fatal` 삭제도 통과했다
(lvalue 인덱스 호출 설계가 **0개**). 둘 다 행을 지어 닫았다. 생존 둘은 **동치**이고 근거를 코드에 적었다 —
경계 드레인(위)과 `EvalNbaScalar`→`k_schedule_nba`(`NbaLhs::of([c])` = `One(c.clone())` 이고 plain scalar
resolve 가 바로 `(0,0)` ⇒ **큐에 들어가는 것이 같다**; 다른 건 비용뿐이라 핀은 값 차분이 아니라 **op-mix
census**).

⚠️ **`cfg(not(test))` 의 `use_compiled_default() = true` 는 어떤 테스트도 볼 수 없다** — 프로덕션이 컴파일
경로를 타는 증거는 CLI 바이트 동일성과 성능 측정뿐이다.

---

#### 4.5.332 ③층 — 쓰기 퍼널의 한 워드 진입점 (2026-08-10, format 26 불변) ✅

**한 줄** — 정본 `write_lvalue` 가 자기 정규화를 마친 뒤 **목적지가 아레나 워드 하나에 들어가면** 평면 진입점 `write_chunk_word` 로 위임한다. picorv32 **0.633 → 0.594 = 6.2%**, native/vm **1.44× → 1.53×**. 프로파일: `write_chunk` 가 상위 22 에서 **사라졌고** 퍼널+`resize` 블록이 **20.6% → 12.5%**.

**⭐⭐ 표적을 정한 것은 census 이고, 그 census 가 내 계획의 절반을 버렸다.** 착수 전 계측(picorv32 한 런): `write_chunk` 4,058,223 건 중 **통짜 원소 저장 98.6%**(4,001,093) · 부분(비트 직렬) **1.4%**(57,130) · `write_lvalue` 의 99.7% 가 단일 청크 · 값 폭 == 목적지 폭이 77.5% · `k_eval_for_lvalue` 의 **92.7% 가 wprog admit**. 내가 먼저 고치려던 것은 **비트 직렬 루프**였는데 그것은 1.4% 였다 — 진짜 표적은 **통짜 저장까지 가는 길 전체**(퍼널 전문 + `resize` + `mask_top`)였다. **셈이 없었으면 1.4% 를 최적화하고 있었을 것이다.**

**추출한 규칙 셋, 전부 정본이 위임한다**:

- `value::resize_word(v, u, from_w, to_w, signed)` — ≤64비트 리사이즈 규칙을 평면으로. `Value::resize` 의 한 워드 빠른 경로가 이걸 부른다. `from_w == to_w` 를 특례로 두지 **않는** 것이 요점이다(어떤 가지도 안 타고 결과가 `& top_mask(w)` = 등폭 early-return 의 `mask_top` 과 같다) — 그래서 퍼널이 **모든 폭 관계**에 대해 한 함수를 부른다.
- `chunk_lsb(c, raw_off, cw)` — 청크가 **어느 비트**를 지칭하는가. 하강 `-:` 의 `- cw + 1` 은 한 글자라 사본 둘이면 조용히 다른 창을 고른다. `cw` 를 다시 접지 않고 **받는다**(호출자가 이미 소스를 그 폭으로 리사이즈했으므로, 재폴딩이야말로 두 답이 갈라질 자리다).
- `chunk_elem(c, raw_word, elems)` — **어느 원소**인가. 범위 밖은 클램프가 아니라 **드롭**이고, 그 드롭이 E4002/W4029 를 빚진다는 **부작용까지 한 자리**에 있다.

**⭐ 차분이 공허해지지 않게 하는 방법** = 진입점을 **리터럴 `bool` 파라미터**로 켜고 끈다(`write_lvalue_general_for_test`). 위임하는 퍼널을 자기 자신과 비교하면 실패할 수 없는 테스트가 된다. 스윕 = 14 사이트 × 오프셋 8(음수·범위밖 포함) × 워드 4(범위내·범위밖·미지 센티넬) × 평면 5 × 소스 9 = **20,160행**, 비교 대상은 `changed` **와 아레나 스냅샷 전체**(buf·dirty·dirty_flag·slot_edge·last_blocking_writer·vcd_pending·pending_range) — 진입점이 **store point** 이고 store point 는 그 넷을 전부 소유하기 때문이다.

**⭐⭐ 그리고 내 스윕을 리뷰해서 구멍 둘을 스스로 찾았다.**

1. **설계에 엣지 감도가 0개**라 `is_edge_target` 이 전부 false → **글리치 캡처(`scalar_bit0` 사전 포착)와 `accumulate_edge` 가 통째로 미도달**이었다. `always @(posedge a)` 한 줄로 복구하고, "엣지 타깃이 0개면 안 된다"를 **단언으로** 박았다.
2. `Value::zeros` 는 `is_real`/`is_str` 을 절대 안 세우는데 **진입점 admission 의 논거가 바로 그 두 플래그**였다("real 은 위의 강제가 이미 지웠다" · "`is_str` 은 `resize` 가 읽지 않고 버린다"). real 행이 그 주장을 **주장이 아니라 시험**으로 만든다.

구멍을 막기 **전에는** 뮤테이션 G(`note_change` 제거)·H(엣지 누적 제거)·I(unk 평면을 val 로 별칭)가 **다른 테스트의 무한루프**로만 죽었다(스위트가 내 테스트에 도달조차 못 한다). 막은 **뒤에는 셋 다 내 테스트가 직접** 죽인다 — 확인했다.

**뮤테이션 12/12 사망**(창 비었음 판정 off-by-one · 2-state 강제 제거 · 창 하단 미클리어 · 언더플로 소스 비트 미드롭 · **소스 부호 무시** · OOB 원소를 0번에 씀 · `note_change` 제거 · 엣지 누적 제거 · unk 평면 별칭 · 하강 `-:` lsb 규칙 · `resize_word` 폭-0 가드 · `resize_word` 를 소스 폭으로 마스킹).

- ⭐ **E(소스 부호 무시)는 오직 새 차분만 잡는다** — 기존 S1c 엔진 차분은 "한 워드 목적지에 더 좁은 **signed** 소스" 형태에 도달하지 않는다.
- ⭐⭐ **K(`resize_word` 의 폭-0 가드)는 처음에 생존했다.** 판별자는 **일반 경로**(>64비트)다: 폭 0 소스는 `nwords(min(from,to)) == 0` 워드를 복사하므로 결과가 0이고, 가드는 그것을 재현한다. **프로덕션은 둘을 구분할 수 없다**(모든 생성자가 `top_mask(0) == 0` 으로 마스킹하므로 폭 0 값에는 샐 비트가 없다) — 그래서 그 행이 **유일한** 판별자이고, 공허한 행이 아니라 필요한 행이다.
- ⭐ **L 은 공유의 대가를 다시 보여준다** — `resize_word` 는 `Value::resize` 가 위임하는 정본이라 wprog 차분이 원리적으로 못 보고, 킬러는 `native_eval::tests::arith_bits::shifts_match_oracle` 등 **오라클 핀**이었다.

**게이트** — 5374 tests green · clippy 0 · fmt 0 · `examples/` 4개 3백엔드 **stdout+VCD 바이트 동일** · `bench/` 5개(picorv32·keccak 3종·혼합폭) 3백엔드 **stdout+exit 동일**.

⚠️ **형태가 다르면 이번에도 값이 다르다** — keccak_arr 과 혼합폭 설계는 **0%** 다(비용이 배열 읽기·평가에 있고 퍼널 비중이 작다). keccak_flat 은 1.37→1.44×.

**남은 표적**(POST 프로파일) = `wprog_for` **3.74%**(평가마다 `RefCell` borrow + `Rc` clone) · `k_resolve_lvalue_offsets` 2.62 + `k_eval_for_lvalue` 2.96 = 대입당 디스패치 · **`settle_cont_assigns` 8.50% + `run_body` 6.51%** 는 S3 본체.

#### 4.5.331 ③층 — 한 비트 단항과 `&&`/`||` 의 평면 진입점: W 평가기에서 `Value` 가 사라졌다 (2026-08-10, format 26 불변) ✅

**한 줄** — §4.5.330 이 남긴 마지막 두 `Value` 소비자(`!`+6 리덕션, `&&`/`||`)도 평면으로 내렸다. 이제 `WProg::run` 은 **어떤 팔에서도 `Value` 를 만들지 않는다**. picorv32 **0.712 → 0.633 = 11.1%**, native/vm **1.27× → 1.43×**.

**추출한 것 넷, 그리고 넷 다 정본이 위임한다.** `RedFacts::absorb`(한 워드가 리덕션에 기여하는 사실 — `!av` 가 폭 위 비트를 전부 세우므로 **`known0` 의 재마스크**가 사본이 흘리기 딱 좋은 줄이다) · `reduce_verdict`(리덕션 진리표) · `unary1_word`(한 비트 단항 가족의 평면 진입점 — `unary_self_of` 가 **비-real ∧ 1..=64비트**면 위임하고, real 가드가 있는 이유는 `LogNot` 만 real 에 도달하며 `truthiness` 가 그것을 f64 로 읽기 때문) · `log_bin_tri`(`&&`/`||` 를 `Value` 가 아니라 **`Tri` 두 개**로 받는다).

**⭐ 형태가 다르면 같은 수정이 다른 값을 낸다.** picorv32 는 11.1% 인데 keccak(균일 64비트 산술)은 **0** 이다 — 그 설계는 한 비트 단항과 `&&`/`||` 팔에 거의 안 들어간다. 두 숫자를 같이 적는 것이 정직한 형태다.

⚠️ **§4.5.330 과 같은 대가**: 공유하는 순간 배터리(wprog vs 제네릭)가 그 함수들에 **원리적으로** 눈멀어진다. 뮤테이션 넷 — `And`+unknown 을 X 대신 1 · `known0` 의 마스크 제거 · `&&` 의 흡수 0 제거 · `!` 반전 — 이 전부 **`native_eval::tests::arith_bits` 의 오라클 핀**에서 사망하는 것을 확인했다(둘은 CLI/e2e 앵커도 함께 잡았다).

**게이트.** 5,372 tests green · clippy 0 · fmt 0 · examples 4 + bench 4 **3-백엔드 바이트 동일** · 소진 배터리 256×256×86 불변.

**⚠️ 남은 `Value`** = **결과 경로**(`run_wprog` 이 `W` 를 감싸고 `write_lvalue` 가 다시 `resize`)와 **쓰기 퍼널**(`write_chunk` 6.7% + `write_lvalue` 3.6%). 둘은 같은 자리이고 다음 슬라이스다.

#### 4.5.330 ③층 — 비교 3종의 평면 진입점: `Value` 를 만들지 않고 같은 규칙을 부른다 (2026-08-10, format 26 불변) ✅

**한 줄** — 개정 10 이 지목한 `Value` 마샬링(15.7%)의 큰 조각. `Cmp` 가 이미 두 u64 평면을 들고 있으면서 공유 규칙을 부르려고 **72바이트 `Value` 두 개**를 만들고 그 안에서 `resize_keep_sign` 이 **또 클론**했다. picorv32 **0.781 → 0.709 = 9.2%**, native/vm **1.16× → 1.28×**.

**⭐ 평면 형태가 두 번째 철자가 되지 않게 하는 방법.** `relational_word`/`log_eq_word`/`case_eq_word` 를 자유 함수로 내고, **정본 함수가 자기 정규화를 마친 뒤 한 워드면 그리로 위임한다**. 판정 로직의 철자가 하나가 되고, ③층은 그것을 한 층 아래에서 부를 뿐이다 — 그래도 되는 이유는 admission 이 이미 *"두 피연산자가 폭과 부호를 공유하고 각자 자기 폭으로 마스크돼 있다"* 를 보장해서 **정규화가 no-op** 이기 때문이다(그 문장이 곧 논증이다). 순서 비교의 진리표도 `relational_verdict` 로 따로 뽑았다 — 두 형태가 *"`Le` 가 `Equal` 을 받는가"* 를 두고 갈릴 수 있는 유일한 자리다.

**⚠️ 공유의 대가는 차분이 눈멀어지는 것이다.** wprog 와 제네릭이 **같은 함수**를 부르므로 소진 배터리(wprog vs 제네릭)는 그 함수 안의 결함을 **원리적으로** 못 본다. 그래서 뮤테이션 넷을 **절대 앵커**에 대고 확인했다 — 부호 순서 뒤집기 · `definite` 에서 `& !u` 제거 · `case_eq` 가 unk 평면 무시 · 진리표에서 `(Le, Equal)` 제거 — **전부 `native_eval::tests::arith_bits` 의 오라클 핀에서 사망**(전 워크스페이스 실행 1건은 CLI 앵커도 함께 잡았다).

**게이트.** 5,372 tests green · clippy 0 · fmt 0 · examples 4 + bench 4 **3-백엔드 바이트 동일** · 소진 배터리 256×256×86 불변.

**⚠️ 남은 것.** `Value` 마샬링의 나머지는 **결과 경로**다 — `run_wprog` 이 결과를 `Value` 로 감싸고 `write_lvalue` 가 그것을 다시 `resize` 한다(프로파일에서 `resize` 의 최대 호출자가 `write_lvalue` 였다). 그 축이 곧 **쓰기 퍼널 8.7%** 와 같은 자리이므로 다음 슬라이스에서 함께 본다.

#### 4.5.329 ③층 S2 종료 — 다음 표적을 프로파일이 정했고, 그것은 admission 이 아니다 (2026-08-10, format 26 불변) ✅

**한 줄** — 남은 admission 축(폭·부호 불일치)을 **구현하기 전에** 프로파일을 다시 재서 그 축이 죽었음을 확인했고(`eval_ctx` **50% → 2.2%**), 대신 프로파일이 지목한 것 하나(정본 `truthiness` 의 비트별 루프)를 고쳤다 — picorv32 네이티브 **0.769 → 0.745(3.1%)**, native/vm **1.16×**.

**⭐⭐ 이 슬라이스의 산출물은 코드가 아니라 판정이다.** §4.5.326 의 교훈("가족을 닫으려면 ablation 이 아니라 census 를 재라")을 한 단계 더 밀었다 — census 는 *무엇이* 거절되는지를 말하지만 *그것이 값을 내는지*는 말하지 않는다. 유일 루트 admission 이 **93.8%** 이고 잔여가 대부분 폭·부호 불일치인데, 프로파일이 **제네릭 트리워커가 이미 2.2%** 라고 말한다 — 그 축을 전부 열어도 상한이 2.2%다. **S2 는 닫힌다**(이번엔 ablation 이 아니라 프로파일이 근거다).

**남은 비용의 실제 분포**(picorv32 네이티브, 4회 집계 2,280 샘플): `WProg::run` 15.4%(실제 작업) · **`Value` 마샬링 15.7%**(`one_word_value` 7.3 + `resize` 4.7 + `mask_top` 3.7) · **쓰기 퍼널 8.7%** · `settle_cont_assigns`+`run_body`+스케줄러 ~13%. ⭐ **doc-21 §2 의 R2 목표(*"`Value` 를 이 경로에서 완전히 제거"*)가 절반만 끝나 있었다** — 특수화 평가기가 값을 계산해 놓고 **공유 규칙을 부르려고 다시 `Value` 로 감싼다**.

**수정 = 정본 함수 안의 빠른 경로 + 평면 진입점.** `truthiness` 가 폭마다 `get_vu` 를 부르는 **비트별 루프**였다(32비트 조건 = 32회). ≤64비트를 마스크 세 번으로 답하는 팔을 **정본 함수 안에** 넣고, 평면을 이미 들고 있는 호출자(③층 W 평가기)를 위해 `truthiness_word(val, unk, m)` 진입점을 냈다 — **두 번째 규칙이 아니라 같은 규칙의 평면 철자**이고, `?:` 와 `k_truthy` 가 `Value` 를 만들지 않고 그것을 부른다.

**⭐ 등가성을 재진술 없이 증명했다.** 같은 비트 패턴을 **두 번** 묻는다 — 4비트 값(빠른 경로)과 **상위가 0인 128비트 값**(루프 경로). 즉 **루프 자신이 오라클**이고 테스트 안에 루프의 사본이 없다. 4비트 4-state 공간 256칸 소진 + 세 판정이 전부 발생함을 단언(True 175 · False 1 · Unknown 80 — **기록이 아니라 유도**: 비트마다 0/1/x/z 이므로 True = 256−3⁴). 뮤테이션(z 를 확정 1로 세기) 사망.

**⚠️ 기대했던 층간 파급은 실측되지 않았다.** 정본이 O(1) 이 됐으니 interp/vm 도 빨라진다고 적었으나 둘 다 노이즈였다(interp 1.355→1.351 · vm 0.860→0.863). ②층은 `native_eval` 안에 **자체 인라인 truthiness** 를 갖고 있고, 인터프리터는 조건 비중이 작다. **"공유 함수를 고쳤으니 모두가 이득"은 측정 전에는 주장이 아니다.**

**⇒ 다음은 admission 이 아니라 `Value` 제거의 나머지**(비교 3종의 평면 진입점 + 결과를 평면으로 쓰는 경로), 그다음이 S3.

#### 4.5.328 ③층 S2 슬라이스 7 — 삼항의 게으름은 값이 아니라 진단의 문제다 (2026-08-10, format 26 불변) ✅

**한 줄** — `wprog` 가 `?:` 를 admit 한다(**두 가지 어느 쪽도 보고할 수 없을 때**). picorv32 **1.03× → 1.12×**, 4중 중첩 삼항 **0.89× → 1.21×**.

**⭐⭐ admission 이 폭이 아니라 평가 순서에 관한 첫 사례.** `eval_ctx` 의 `Ternary` 는 조건을 평가한 뒤 **취한 가지만** 돌린다(둘 다 도는 것은 조건이 미지일 때뿐). 이 모듈은 제어흐름이 없어 늘 둘 다 도는데, 그것은 **값은 같고**(admitted op 은 전부 순수) **진단은 다르다** — `LoadIdx` 만이 범위 밖 원소 읽기를 **센다**. **진단이 생기는 것도 발산이다.** 그래서 검사는 문법이 아니라 **컴파일된 op 열**에 대고 한다(정확하고 국소적 · 보수적 근사가 아니다).

**⭐⭐ 그 앵커가 ②층 — 기본 백엔드 — 에서 같은 결함을 찾았다.** `native_eval` 의 삼항도 똑같이 즉시평가인데 가드가 없어서 `r = c ? 8'hAA : mem[9];`(원소 4개) 가 **값은 맞고 `E4002` 를 낸다**. `E4002` 는 `Severity::Error` 라 CLI 가 종료코드로 세므로 **`--backend vm` 이 exit 1, interp·native·iverilog 는 exit 0** 이었다. 같은 슬라이스에서 같은 형태의 가드(`LoadIndexed`/`WLoadIndexed` 가 가지 안에 있으면 decline)로 고쳤다. ⭐ **찾은 이유는 앵커를 두 백엔드에 대고 돌렸기 때문이다** — 내가 바꾼 백엔드만 핀했으면 못 봤다.

**⭐ 내 우려를 실측이 반증했다.** 즉시평가는 중첩에서 팔 개수만큼 낭비하므로 제네릭보다 느릴 수 있다고 적고 시작했는데, 4중 중첩(팔 5개)에서 **0.483 → 0.353** 이다. `terA` 는 **삼항이 아예 없는 기준선(0.332)에 착지**했다.

**수정 = op 하나 + 공유 함수 하나.** `WOp::Tern { cw, cs, m }` 이 else·then·cond 를 팝해 `truthiness`(이미 자유 함수)로 고르고, 미지일 때 `merge_x_word` 로 합친다 — 후자는 이 슬라이스가 `Value` 판 `merge_x` 에서 **워드 규칙만 추출**한 것이라 두 층이 한 철자를 쓴다.

**게이트.** 소진 배터리 256×256×**86** · 코퍼스·오프셋 핀 불변 · **삼항 차분 140설계 3-백엔드 + iverilog, 진단 포함 140/140 일치·발산 0** · examples 4 + bench 4 3-백엔드 바이트 동일.

**⭐ 뮤테이션 넷 전부 사망, 그리고 각자 다른 테스트로만 죽는다.** ① ③층 가드 제거 → **게으름 앵커만** 잡는다(값 테스트는 전부 통과) · ② ②층 가드 제거 → 같은 앵커 · ③ merge 를 항상 X 로 → 배터리 · ④ then/else 교환 → 배터리.

**⚠️ 그리고 행 하나를 지었다가 지웠다.** 미지-조건 X 병합 전용 행을 **두 번**(같은 두 팔 · 절반만 일치하는 두 팔) 지었는데 **둘 다 실측 잉여**였다 — 65,536 상태 스윕이 위쪽 행들만으로 모든 일치 패턴에 도달해서, whole-value 병합 뮤턴트도 always-X 뮤턴트도 그 행 없이 죽는다. **아무것도 결정하지 못하는 행은 없는 것보다 나쁘다**(커버리지처럼 읽힌다) → 지우고 그 사실을 소스에 적었다.

**남은 거절(picorv32 전수 census).** 유일 루트 기준 **845 admit / 56 decline = 93.8%**. `Ternary` arm 은 **41 → 0**; 잔여는 대부분 **폭·부호 불일치**(개정 6 의 ablation 이 0으로 잰 그 축)와 `SysFunc` 3.

#### 4.5.327 ③층 S2 슬라이스 6 — 상수·범위내 부분선택은 시프트 하나와 마스크 하나다 (2026-08-10, format 26 불변) ✅

**한 줄** — `wprog` 가 `Select` 를 admit 한다(상수 오프셋 ∧ 창이 base 안에 **증명 가능하게** 들어갈 때). `--backend native` 가 **실사용 설계(picorv32)에서 처음으로 `--backend vm` 을 앞섰다 — 0.90× → 1.03×**.

**⭐⭐ 착수 순서를 측정이 정했다.** §4.5.326 이 남긴 표적은 둘(`Select` 73 · `Ternary` 39)이었고, **둘의 천장을 판별 설계로 먼저 쟀다** — 부분선택이 산술 안에 있으면 native 가 **0.83×**(같은 산술에서 선택만 빼면 1.16×), 삼항이 산술 위에 있으면 **0.98×**(빼면 1.17×), 4중 중첩 삼항은 **0.89×**. `Select` 가 손해가 크고 기계가 단순하다. 그리고 `Ternary` 는 제네릭이 **취한 가지만 평가**하므로 즉시평가로 바꾸면 안 취한 가지의 `LoadIdx` 가 **없던 E4002 를 낸다** — 게으름(=W 프로그램의 제어흐름)이 필요하고 중첩에서 오히려 느려질 수 있어 **별도 슬라이스**로 미뤘다.

**수정 = op 하나 + 공유 함수 하나.** `WOp::Slice { k, m }` = 양 평면을 `k` 만큼 오른쪽으로 밀고 `m` 으로 마스크. 그것이 곧 `copy_bits(out, 0, src, lsb, w)` 를 0으로 채워진 `w` 비트 목적지에 하는 일이다. **admission 이 정확성 논거다**: `eval_select` 은 결과가 셋(미지/과대 오프셋 → 전부 X · 완전 범위내 → `copy_bits` 한 번 · 걸침 → 비트별 X 채움)인데, 범위를 **컴파일 시점에 증명**하면 가운데 하나만 도달 가능하다. `(lsb, 폭)` 규칙은 **`eval::binops::select_lsb_width` 로 추출해 공유**한다 — 세 형태의 차이가 정확히 `- width + 1` 하나라, 두 번째 사본이 그걸 빠뜨리면 **다른 네 비트를 조용히** 고른다.

**⭐ 오프셋은 제네릭과 같은 두 호출로 읽는다** — `to_u64().and_then(|o| i64::try_from(o).ok())`. `to_u64` 는 부호를 **일부러 무시**하므로 음수 상수는 큰 양수로 읽히고 범위 검사에서 걸러진다(제네릭과 같은 경로).

**게이트.** 소진 배터리 256×256×**81**(68→81, 신규 14행·거절 **3**행) · 코퍼스 admitted 트리 **7,890→7,960** · lvalue 오프셋 등가 **(2200,24)→(2212,12)** · **Select 차분 180설계 4-way**(interp/vm/native/PRE + iverilog) **180/180 iverilog 일치 · 백엔드 발산 0 · 회귀 0**(걸침·완전 범위밖·음수·런타임 오프셋·x/z·부호 base·concat/replicate/배열원소 위의 선택 포함) · examples 4 + bench 4 바이트 동일.

**⭐ 뮤테이션 넷.** ① 공유 `-:` 규칙에서 `- width + 1` 제거 → **CLI 절대 앵커**가 kill(차분은 원리적으로 못 본다 — 양쪽이 같이 움직인다) · ② `Slice` 의 마스크 제거 → 코퍼스 스윕이 kill(`{a,b}[5:2]` 처럼 창 **위에** 비트가 있는 행만 판별한다) · ③ 범위 검사를 `lsb >= 0` 으로 약화 → 배터리가 kill(`a[4:1]` 이 admit 돼 개수와 값이 둘 다 어긋난다) · ④ 배터리에 **거절 행이 없으면** ①③이 전부 공허하므로 `a[4:1]` 을 행으로 넣었다.

**남은 거절 census(picorv32, 실측 60건)** — `Select` **73→0** · `Concat` **32→0** · 잔여 **`Ternary` 41**(68%) · `SysFunc` 3 · 폭·부호 불일치 16(대부분 `Const` 10). 다음 표적은 `Ternary` 이고 그 값은 이미 쟀다.

**⚠️ admit 하지 않은 것.** 런타임 오프셋 인덱스 선택(`x[i +: 4]`) — 런타임 범위 검사와 X 채움 arm 이 필요하다. `+:`/`-:` 는 **오프셋이 상수면** admit 된다(측정: 배터리 3행 전부 admit).

#### 4.5.326 ③층 S2 슬라이스 5 — 인덱스 봉인의 `Concat` 이 배열 읽기를 통째로 제네릭에 두고 있었다 (2026-08-10, format 26 불변) ✅

**한 줄** — `wprog` 에 `Concat` arm 이 없어서, §4.5.308 이 **모든 런타임 배열 워드 인덱스**를 감싸는 봉인(`{22'b0, idx}`)이 트리 전체를 거절시키고 있었다. arm 하나로 배열 읽기 **0.94→1.26×**, 메모리 **0.92→1.85×**.

**⭐⭐ 착수가 곧 자기반증이다 — 직전 슬라이스의 판정이 틀린 축을 쟀다.** doc-21 개정 6(§0.1)은 `compile_node` 의 **폭·부호 균일 게이트**를 env 로 무력화해 8설계에서 **0** 을 재고 *"S2 admission 은 레버가 아니다 → S2 를 닫는다"* 로 **가족 전체**를 닫았다. 그런데 최악 형태(배열 읽기)는 균일성이 아니라 **arm 이 아예 없는 노드**에서 거절되고, **arm 없는 노드는 균일 게이트를 꺼도 그대로 거절된다** — 그 실험이 원리적으로 못 보는 축이었다. ⭐ 그리고 이번에도 먼저 지어서 반증했다: *"문장당 디스패치가 표적"* 이라는 개정 6 의 후속 주장도 **상한 실험**(`compute_effect` 의 분류 술어 13개를 통째로 건너뛰기)으로 **0~4%·picorv32 −0.3%** 를 재고 버렸다.

**⭐⭐ 답을 준 것은 ablation 이 아니라 census 다.** `compile_node` 의 fallthrough 에 노드 종류를 찍는 10분짜리 계측이 즉시 갈랐다 — `a <= mem[ad2] + 1` 한 줄 설계의 거절이 **정확히 1건, `kind=Concat`**(파트는 `Replicate(22×1'b0)` + `Signal(ad2)`), picorv32 는 `Select` **73**·`Ternary` **39**·`Concat` **32**·`SysFunc` 3.

**⭐⭐ 그리고 그 답은 이미 저장소 안에, 초록인 핀 안에 적혀 있었다.** `s2_specialized_offsets_match_the_canonical_resolver` 의 실패 메시지가 §4.5.308 이래 *"the declared-range fix seals index expressions in a `Concat`, which the W compiler does not admit … **a `Concat` arm in `wprog` would win them back**"* 를 문자 그대로 담고 있었다. 이 슬라이스가 그 문장이고, 44칸 중 **40칸을 회수**했다(2160/64 → **2200/24**).

**수정 = 한 op.** `WOp::Splice { off, stride, count, m }` — 누산기(정의 0인 `Const`) 위로 팝된 값의 비트를 `count` 번 `stride` 간격으로 **평면별 OR** 로 합친다. 4-state `Or` 가 아니라 평면 OR 인 이유가 곧 정확성 논거다: 파트들의 범위가 **겹침 없이 타일링**하고(Σ pw == w 를 검사) 목적지가 definite 0 이라, 이것은 `eval_core::copy_bits` 가 자기 문서에 적어 둔 **같은 계약**이다. `Concat` 은 `count:1` · `Replicate` 는 `count:n, stride:피연산자 폭` 으로 **한 op** 가 둘을 다 맡는다.

**세 가지 결정.** ⓐ 파트는 IEEE §11.8.1 대로 **자기결정** — 각자 자기 `(폭, 부호)` 로 컴파일하며, 확장·부호확장·절단이 **어디에도 없다**(fill 이 들어갈 자리가 없다). ⓑ **소스 순서로 emit** — `eval_concat` 이 `parts[0]`(MSB) 부터 평가하고 admitted 서브트리는 범위 밖 접근을 **센다**. LSB 부터 냈다면 비트는 같고 **진단 둘이 뒤집혔을 것**이다. ⓒ **폭 0 파트는 건너뛰지 않고 거절** — `{0{x}}` 는 합법이고 제네릭은 그것도 **평가**하므로, 건너뛰면 E4002 를 잃는다. `Replicate` 는 피연산자를 **한 번만** 컴파일한다(제네릭도 한 번 평가한다 — `count` 번 컴파일하면 진단이 `count` 배가 된다).

**모듈 문서의 거짓이 된 문장을 고쳤다.** *"admitted 트리 안에서 값이 폭 사이를 이동하는 일은 없다"* 가 이 슬라이스로 거짓이 됐다(파트 비트는 **살아남아** 더 넓은 결과로 간다). 남은 참인 주장은 더 좁다 — **확장·부호확장·절단이 없다**.

**게이트.** 소진 배터리 256×256×**68**(55→68·거절 2행) · 코퍼스 admitted 트리 **7,715→7,890** 0 발산 · lvalue 오프셋 등가 스윕(값 + **보고 수**) 2,200칸 · concat/replicate 차분 **215설계 4-way**(interp/vm/native/PRE + iverilog) **백엔드 발산 0·회귀 0**(잔여 17은 `%h` 의 x/z 대소문자, PRE==POST·무관) · 중간위치 부호/1비트 파트 48설계 0 발산 · **OOB/unknown 인덱스 concat 퍼즈 120설계에서 stdout+진단 스트림이 3백엔드 바이트 동일**(진단 144건·43설계가 2건 이상) · examples 4 + bench 4 바이트 동일.

**⭐⭐ 뮤테이션 셋, 그리고 그중 하나가 내 테스트 축의 구멍을 드러냈다.** ① 파트 역순 emit → 순서 테스트가 kill(**값 테스트는 전부 통과** = 값으로는 원리적으로 안 보인다) · ② replicate 피연산자를 `count` 번 컴파일 → 단일평가 테스트가 kill · ③ **signed 파트를 splice 전에 부호확장** → 처음엔 **생존**했다. 이유는 내 행 `{sa, b}` 가 signed 파트를 **맨 위**에 두어 fill 이 결과 폭 위로 나가 마스크에 지워지기 때문 — **구조적으로 면역인 행**이었다. `{a, sa, b}`(중간 위치)를 지어 kill 했고, 그 행을 빼면 뮤턴트가 다시 산다는 것도 실측해 주석에 적었다.

**⚠️ 하네스가 내 새 테스트를 공허하게 만들 뻔했다.** `both_backends_print` 는 `out|t=` 만 남기므로 **진단에 구조적으로 눈멀어 있다** — 진단 순서를 재려던 두 테스트가 **빈 스트림과 비교해 처음에 통과**했다. 필터 없는 `both_backends_stream` 을 지어 다시 세웠다.

**⚠️ 잔여.** picorv32 는 0.88→**0.91×** 로 아직 vm 에 진다. 남은 거절은 `Select` 73·`Ternary` 39 이고 **그 둘의 값은 아직 안 쟀다** — 개정 6 의 실수를 반복하지 않기 위해 그렇게 적는다.

#### 4.5.325 인라인 인자 바인드가 formal 의 선언 타입을 적용한다 — 셋을 함께 (2026-08-10, branch feat-inline-formal-bind, format 26 불변) ✅

**한 줄** — 인라인(SSA-fold) 호출의 인자 바인드는 §13.5.3 이 말하는 **대입**이고, 그 대입은 폭·부호·2-state 를 **동시에** 나른다. §4.5.323 이 부분 적용으로 세 번 회귀한 뒤 이름 붙인 선행조건(§4.5.324)이 서서 다시 열었다.

**착수·재현** — HEAD 에서 5형태 전부 재현하고 프레임 쌍둥이가 5/5 정답임을 확인했다(= 내부 오라클). 1,920칸 매트릭스(12 formal 타입 × 8 본문 × 10 actual × 2 반환형)로 **644칸 오답**을 쟀고, 축이 셋으로 깨끗이 갈렸다: 절단(`[3:0]`)·부호(`byte`·`signed`)·2-state(`bit [7:0]` + x actual).

**수정 = 하나의 게이트, 하나의 프리미티브.** `bind_formal_actual` 이 `resize_inline_assign(eid, w, formal_signed)` 로 폭·부호를 **한 프리미티브**에 맡기고(§4.5.323 라운드 2 는 방향마다 다른 프리미티브를 써서 real actual 의 f64 비트를 잘랐다), 그 뒤 2-state 를 얹는다. 셋 전부가 **신뢰 가능한 actual 폭** 하나에 걸리고, 없으면 pre-slice 꼬리가 verbatim 이다. `trusted_self_width` 는 §4.5.320/321 의 가드를 **한 철자로 추출**한 것이고 `resize_inline_assign` 도 그것을 쓴다.

⭐⭐ **내가 스스로 잡은 회귀 — 부호 스탬프가 하류에서 피연산자를 복제한다.** signed 스탬프 자체는 언급 1회지만, 그것이 **반환 리사이즈**의 `extend_to` 에게 sign-fill 을 시키고 그 fill 은 `Select{Bit, base:e}` = 두 번째 언급이다. `$random` 이 두 번 뽑히고(값이 두 번째 뽑기에서 나오고 스트림이 한 칸 밀린다) 프레임 호출도 두 번 실행됐다. §4.5.320 S1 이 기록한 함정이 **바인드가 아니라 다른 자리에서** 재발한 것이라, 게이트는 "불순 actual 은 signed/2-state 로 만들지 않는다" 가 됐다.

**⭐⭐ 적대 라운드 1 — 두 렌즈의 BLOCKING 둘이 같은 한 문장에서 나왔다: "2-state 단계가 `resize_inline_assign` 밖에 있다".**

- **soundness B1(correct→silent-wrong 40칸)** — 밖이라 그 함수의 real/string 가드를 상속하지 못했다. `resize_inline_assign` 은 real actual 을 올바르게 그대로 돌려주고, 그 다음 `coerce_two_state` 가 IEEE-754 페이로드를 비트로 읽었다: `fint(9.0)` 이 0, `longint` 이 `4615514078110652826`, `byte unsigned` 가 **154** — §4.5.323 라운드 2 의 바로 그 증상이 **다른 문으로** 돌아온 것.
- **differential B1(제품 성능)** — 같은 프리미티브가 **선언 비트마다 피연산자를 언급**하고 엔진은 DAG 를 트리로 걷는다: `int` 15.7× · `longint` 42.7× · `.velab` 23× · 중첩 4단이 0.06 s → 19.67 s, 5단은 미완주. ⭐⭐ **내 20런 바이트-동일성 게이트가 그 축에 구조적으로 눈멀어 있었다** — `examples/`·`bench/` 전체에 2-state formal 이 **0개**이고, 단위 테스트 704건은 값만 본다.

**수정 둘.** ⓐ real/string 가드를 함수 **꼭대기**로 올려 두 단계가 함께 받게 했다. IR 의 `expr_is_real` 은 프레임 `Expr::Call` 을 못 본다(real 반환의 반환 넷은 `NetKind::Real` 이 아니라 64비트 `Reg` — 처음엔 그 넷 kind 를 읽으려다 실측으로 기각했다) → 형제 자리가 이미 쓰던 `cast_operand_is_real` 을 쓰기 위해 AST actual 을 `reduce_function_body` 로 내려보냈다. ⓑ 새 술어 `expr_may_be_unknown` 이 **알려진 값이면 강제를 짓지 않는다**. 기본이 "강제"(안전 방향)이고, `CaseEq` 계열이 **구성상 known** 이라는 팔이 중첩 폭발을 끊는 자리다. 값 차이 **0**(1,920칸), `longint` 8.06 s → 0.16 s, 중첩 미완주 → 0.02 s, 상수 1.97 → 0.21 s.

**⭐⭐ 라운드 2 — differential CLEAN(회귀 0), soundness BLOCKING 0.** 리뷰어가 **라운드-1 바이너리를 *"항상 강제·가드 없음"* 기준선**으로 삼아 새 술어의 거짓-known 과 가드의 거짓-real 을 각각 직접 판별했고 2,382칸 + 퍼즈 1,920칸에서 0. AST actual 정렬은 13행(positional·named·역순 named·default·dyn-array formal 이 앞/중간/뒤 = `continue` 형태)으로 측정 검증됐다.

**⭐ 뮤테이션 26 중 생존 11 — 그중 8이 내 테스트 actual 축의 구멍이었다.** real actual · 계층 참조 · **unsigned** 2-state formal(기존 행이 `byte`=signed 라 2-state 항의 이빨이 **0개**였다) · 좁은 signed → 넓은 2-state(순서를 뒤집는 뮤턴트가 구조적으로 안 보였다) · 2-state 피연산자(내 술어 행이 `logic` 이라 세 팔이 **결정에 참여조차 안 했다**) · ternary(**커버리지 0**) · 다인자 정렬(off-by-one 이 1인자 축약 케이스로만 죽었다). 전부 행을 지어 닫았고 `-:` 행은 **iverilog 가 자기모순**(리터럴 x 는 강제하는데 범위 밖 셀렉트의 x 는 남긴다)이라 hand-IEEE 로 핀했다. 생존 3(`word.is_some()`·string 절반·always-true)은 **도달 불가 또는 값 등가**로 리뷰어가 코드 수준 논증을 냈다.

**게이트** — 1,920칸 FIXED 636·REG 0 · differential 약 14,350 오라클 칸 회귀 0 + 라운드 2 2,382칸 + 퍼즈 1,920칸 · 실사용 20런(examples ×3백엔드 + keccak 3변종 + picorv32 ×2) **stdout·stderr·exit·VCD 바이트 동일** · picorv32/keccak `.velab` **바이트 동일** · 5,347 → **5,365 tests**(신규 20) · clippy/fmt clean · format 26 불변 · staged(`vcmp→velab→vrun`) == one-shot.

⚠️ **완화를 지어서 반증한 기록** — 남은 O(폭) 비용에 per-query 메모와 노드 예산을 각각 지어 **둘 다 개선 0** 이었다. 비용은 깊은 워크 하나가 아니라 **바인드 개수**에 있고(인라인 fold 가 호출마다 별도 서브트리를 만들어 아레나가 진짜로 2^n 개 **서로 다른** 노드다), 영속 캐시는 in-place 패치와 충돌한다(placeholder 는 패치 전에 "알려진 1비트 `Const`" 로 읽힌다). §2 에 그대로 적었다.

⚠️ **절차** — 라운드 1 에서 리뷰가 도는 동안 트리를 건드리지 않았고(§4 규칙 준수), 수정은 두 렌즈가 **모두 반환한 뒤** 적용했다. 대신 그 대기 시간에 **복사본**에서 완화를 개발·측정해 라운드 2 브리핑에 실측치를 실었다.

#### 4.5.324 상수 배열 인덱스가 부호를 가진다 — 여섯 줄, 그리고 §4.5.323 의 선행조건 (2026-08-10, branch feat-const-index-sign, format 26 불변) ✅

**한 줄** — §4.5.310 은 상수 인덱스를 일부러 봉인 전 경로에 두어 *"정적으로 범위 밖인 인덱스는 범위 밖으로 남아 **진단**된다"* 를 지키려 했는데, 엔진이 인덱스를 `to_u64` 로 읽으므로 **32비트보다 좁은 signed 상수가 양수로 도착**했다.

**착수** — §4.5.323 이 세 라운드 회귀 끝에 **이름 붙인 선행조건**. 재현이 기록보다 넓었다: `logic [7:0] arr[0:255]` 에서 `arr[-8'sd1]` 이 `a5`(원소 255)를 **조용히** 읽고, **같은 비트의 다른 철자** `arr[8'sd255]`(= −1)도, 8원소 배열의 `sm[-3'sd1]` 도 같다.

**⭐ 결함이 숨은 이유가 곧 판별자다** — **무부호로 읽은 값이 우연히 범위 밖일 때만** 맞았다. `sm[-4'sd1]` 은 15라 이미 밖이어서 "정답" 이었고, **넷 인덱스는 §4.5.309 이래 부호확장 봉인**이라 늘 정확했다. 그래서 틀린 것은 상수 carve-out 하나뿐이었고, 수정은 그 가지에서 signed 상수를 인덱스 도메인으로 **부호확장**하는 것 — carve-out 자신의 약속을 지킨다(음수 상수는 어떤 배열도 담을 수 없는 인덱스가 되어 **loud** 로 남는다).

**⭐ 평범한 철자는 애초에 안 걸린다** — `arr[3]` 의 unsized 십진은 **32비트** signed 라 `sw.width < 32` 가 거짓이고 `.velab` 이 **바이트 동일**이다. 걸리는 것은 명시적으로 좁은 signed 상수뿐이고 그것은 대개 버그다.

**게이트/측정** — 2,790칸 스윕(배열 6종[음수 base 포함] × 폭 6 × 부호 2 × 값 11 × 철자 5) **FIXED 159 · REGRESSED 0 · still-wrong 0** · differential 213칸 추가 **FIXED 53 · REG 0**(2차원 음수 base 96칸 · 원소타입 packed struct/enum/real/string · 모듈·패키지·generate·계층·함수·태스크 스코프 · 인덱스 식 22형 · 폭 경계 31/32/33/64 · x/z 상수) · **24런 바이트 동일**(examples 4 + keccak 3변종 + picorv32 × 3백엔드) · 5,336 → **5,347 tests** · clippy/fmt clean.

**⭐ soundness 가 아무도 청구하지 않은 수정 둘을 더 쟀다** — 계층 퍼널(`u.mem[-8'sd1]` `a5`→`xx`+E4002)과 `string s[0:255]`(`"LAST"`→빈 문자열+W4020). 그리고 **1비트 signed 상수**(`arr[1'sb1]` = −1 이 원소 1 을 읽었다)와 **다차원 음수 base**(`g2[-2'sd1][2'sd2]` `47`→`xx`)가 판별 행으로 들어갔다.

**⭐ 뮤테이션 11 중 9 생존이었고, 그 중 하나는 등가가 아니라 패닉이다** — 폭 가드를 지우면 `extend_to` 가 `32 − 64` 를 계산해 언더플로한다. `arr[-64'sd1]` 행으로 막았다. ⚠️ **폭 31 은 원리적으로 판별 불가**(좁은 signed 상수의 무부호 읽기는 `2^w − |v|` 라 폭 31 을 가르려면 20억 원소 배열이 필요하다) — 그 경계를 옮기는 뮤테이션은 미검증이 아니라 **실질 등가**이고, 공허한 행을 넣는 대신 그 사실을 주석에 적었다.

**⭐ 리뷰어의 "거짓 주석" 지적은 실측으로 기각했다** — `packed.rs` 의 *"그 가드가 `Call` 을 거절해 `ma[fw(0)]` 이 loud 로 남는다"* 는 **기각된 대안**에 대한 문장이고, 실제 코드는 그 접근을 PRE·POST·iverilog 모두 `aa` 로 정확히 착지시킨다.

**비용** — bare 리터럴은 **한 노드**로 접어(`const_index_sign_extended`) 상수 인덱스의 `Const` 모양을 유지한다(tier-3 워드 인덱스 fast path 와 `index_all_const` 가 그 모양을 본다) · x/z 는 접지 않는다(W4029 와 E4002 는 다른 판정이다) · 접히지 않는 상수 식은 노드 넷(4,000개 합성 설계에서 `.velab` +137%, 실사용 설계는 바이트 동일).

**신규 §2 넷**(전부 pre-existing·오라클 ✓·다른 퍼널) — queue/dyn 인덱스에 **봉인이 아예 없다**(상수도 넷도·read/write 비대칭·이 carve-out 보다 넓다) · 함수 호출 인덱스는 어느 봉인에도 못 온다 · `$bits` 를 상수 인덱스 식에 쓰면 `x` · 평범한 벡터의 비트/부분선택.

#### 4.5.323 인라인 인자 바인드의 formal 타입 — 지어서 세 라운드 재고 되돌렸다 (2026-08-10, branch feat-inline-formal-sign, **코드 미머지**) ⚠️

**한 줄** — 인라인 함수의 인자 바인드가 formal 의 **선언 타입 셋**(폭·부호·2-state)을 하나도 적용하지 않는다. 셋은 서로의 전제라 부분 적용이 미적용보다 나쁘고, 셋째(폭)는 **다른 코드 경로의 선행조건**에 걸린다 — 그래서 이번 반복은 **코드를 되돌리고 측정을 산출물로** 남긴다.

**결함(전부 오라클 ✓·프레임 쌍둥이가 내부 오라클)** — 프레임 경로는 formal 이 넷이라 엔진에게서 셋을 공짜로 받고, 인라인은 actual 의 ExprId 를 이름에 **치환**할 뿐이다:
- **부호** — 본문이 actual 의 부호를 읽어 **연산이 바뀐다**: `function signed [7:0] fi(input signed [7:0] x); fi = x/3;` 에서 `fi(8'hf7)` 이 inline `82` / frame·iverilog `-3` · `x>>>1` `123`/`-5` · `x>0` `1`/`0`. `byte`/`int` 처럼 **kind 가 부호를 나르는** 형태도 같다(`65533` vs `82`).
- **2-state** — `byte` formal 에 x actual 이 `X`(프레임·iverilog `0`).
- **폭** — 넓은 actual 이 절단되지 않아 더 좁은 내부 formal 로 재전달되면 남은 비트를 읽는다(`1431655762` / `-3`).

**⭐⭐ 부분 적용이 미적용보다 나쁘다 — 세 라운드가 그것을 세 번 다른 형태로 증명했다.**
- **라운드 1**: 부호만 적용 → 3,150칸 FIXED 258·REG 0 이었는데 적대 리뷰가 **회귀 2건**(중첩 재바인드에서 안 잘린 비트를 새 부호로 읽음 · 2-state x/z 가 `==` 를 X 로).
- **라운드 2**: 폭·2-state 를 더해 FIXED 584·REG 0 · split 605→36(남은 36은 **프레임이 틀린 쪽**) — 그런데 **회귀 2건 더**: 두 폭 팔이 **다른 프리미티브**를 써서 real actual 이 f64 비트로 잘렸고(`f(3.7)` = 154), 절단된 **상수**가 §4.5.310 의 상수-인덱스 carve-out 에 접혀 착지해 loud `xx` 가 조용한 `a5` 가 됐다.
- **라운드 3**: 두 팔을 한 프리미티브로 묶고 상수를 skip → FIXED 303·REG 0 · 5,348 green — 그런데 **또 2건**: 폭 결정이 `ir_bits_of` 의 **지어진 32**(클래스 필드 핸들넷)를 믿어 `i16(c.bu)` 가 `xxc3` 이 됐고, skip 을 **노드 종류**로 키잉해서 같은 상수의 여섯 철자(`~16'h0`·`{8'hFF,8'hFF}`·`1?…:…`·…)가 빠져나갔다 — **§4.5.310 이 이미 기록한 "상수 인식이 철자 기반" 함정의 재발**이다. 그리고 그 skip 은 반대 방향으로 **정답 수정도 막고 있었다**(무부호 formal).

**⭐⭐ 후퇴안도 측정이 반증했다.** "한 번도 회귀 안 한 교집합 = 동일 폭만" 으로 좁히자 **FIXED 303 → 215 · REGRESSED 0 → 80** 이고 자기 테스트 2개도 깨졌다(중첩 바인드가 서로를 먹인다). **스코프 축소도 편집이고 전 매트릭스를 다시 돌려야 한다.**

**⇒ 판정.** 셋을 함께 적용하는 것이 유일한 정답이고, 그 전제는 **§4.5.310 의 상수-인덱스 carve-out 이 인덱스를 부호로 읽는 것**이다(함수 없이도 틀린다 — `arr[-8'sd1]` 이 vita `a5` / iverilog `xx`). 그 한 줄이 서면 절단이 안전해지고 이 슬라이스 전체가 열린다. 그래서 **다음 반복의 표적은 carve-out 이고, 이 항목은 그 뒤에 다시 온다.** 되돌린 코드·스윕·리포트 전부 재현 가능(§2 에 repro 등재).

⚠️ **절차 사고 하나** — 라운드 3 soundness 가 도는 중에 내가 트리를 고쳤고, 리뷰어가 mtime 으로 그것을 잡아 자기 측정 두 개를 무효 처리했다. LOOPROMPT §4 가 금지하는 바로 그 행동이다.

#### 4.5.321 인라인 함수의 선언 반환 폭도 자기결정 경계다 (2026-08-09, branch feat-inline-return-seal, format 26 불변) ✅

**한 줄** — §4.5.320 이 캐스트에서 고친 것과 같은 결함이 인라인(SSA-fold) 경로에 그대로 있었고, 그 자리에서는 **스탬프가 조건부였다는 사실 자체가 봉인의 부재**였다.

**착수** — §4.5.320 이 §2 에 "봉인 안 된 쌍둥이" 로 적어 둔 항목. 인라인 fold 는 넷을 안 쓰므로 엔진 리사이즈를 못 받고, `resize_inline_assign` 이 §10.7 을 대신 적용한다. 그런데 `$signed`/`$unsigned` 스탬프가 **목표 부호 ≠ rhs 부호일 때만** 붙었고, **붙지 않는 경우가 곧 봉인이 없는 경우**다 — 맨 `Binary`/`Unary`/`Ternary` 는 엔진에게 문맥결정이라 더 넓은 호출부가 본문식을 다시 돌린다. `function [7:0] fm(input [7:0] x); fm = x*x;` 를 64비트로 읽으면 `fe01`(iverilog `0001`).

⭐ **1,440칸 스윕의 48칸이 전부 한 구멍**이었다 — 선언 폭 == rhs 자기 폭 ∧ **무부호 반환**. 부호 반환은 무부호 rhs 때문에 스탬프가 생겨 **우연히 봉인돼 있었다**(리뷰어의 독립 648칸 스윕도 32/32 가 같은 특성). 그래서 수정은 "스탬프를 무조건으로" 한 줄이고, 가드는 §4.5.320 의 **신뢰 가능한 폭** 판정을 그대로 들여왔다.

**⭐⭐ 그 가드에 이빨이 0개였고 리뷰어가 판별 설계를 줬다.** `trusted_w` 를 `true` 로/`known_rw.is_some()` 로/정본 폭 우선으로 바꾸는 뮤테이션 셋이 전부 생존했다 — 신규 9 테스트 중 어느 것도 **지어진 폭**을 쓰는 rhs 를 갖고 있지 않았기 때문이다. 킬러는 클래스 필드다(핸들넷의 `Some(32)` 가 선언 폭 32 와 **우연히 같아** 같은-폭 팔에 들어간다): `function [31:0] fh(input [7:0] x); fh = c.fld * x;` 는 대상 폭과 무관하게 `fe01` 이어야 하는데 세 뮤턴트는 `0001` 을 낸다. 한 행이 셋을 죽인다.

**⚠️ 그리고 앵커를 지으려다 새 결함을 쟀다 — 클래스 필드가 아니라 평범한 넷 쪽이 틀린다.** 같은 본문의 넷 버전 `fh = fld * x` 는 static 에서 `00000001`, **`automatic` 에서 `0000fe01` = iverilog**. 인라인 경로가 **선언 폭을 본문 안으로 내리지 않는다**(`lower_ctx_or_plain` 은 fill 만 크기를 준다) — prim cast 와 **같은 클래스**이고, 같은 본문이 lifetime 철자 하나로 갈리는 inline↔frame split 이다. PRE==POST 라 §2 로.

**⚠️ 부호 반쪽은 일부러 남겼고, 그 이유를 리뷰가 정정했다.** 내가 적은 근거는 *"넓히기 arm 에 도달하는 유일한 리프가 불순한 프레임 `Expr::Call` 이라 정본 부호를 채택해도 얻는 게 없다"* 였는데, **signed 클래스 필드도 그 arm 에 도달하고 그것은 순수·반복 가능**하다(`fw = c.sf` 가 `00…ab` / hand-IEEE `ff…ab`). 즉 정본 부호 채택은 **반복 가능성 술어로 게이트한 진짜 수정**이다 — 무가치가 아니라 별도 슬라이스. 주석을 실측대로 고쳤다.

**⚠️ "판별자는 `automatic`" 도 거짓이었다**(리뷰 실측). 프레임 라우팅은 `frames_classify.rs` 의 논리합(`automatic || ret_two_state || body_needs_frame || unpacked formal || (body_needs_context_width && body_reads_only_locals)` + 재귀)이고 **static 함수도 프레임으로 간다**(`g = x*x` 는 프레임, 형제 `f = x` 는 인라인 — 같은 파일·같은 lifetime). `automatic` 은 상관물이지 판별자가 아니다.

**게이트** — 1,440칸 48→**0** · 형태 32종 FIXED 16/REG 0/still-wrong 0 · differential 6,680 오라클 칸 + 65 손-설계 **REGRESSION 0 · FIXED 235** · soundness 독립 648칸 PRE 32 → POST 0 · 3백엔드 바이트 동일 · `examples/` 넷 + keccak 3변종 + picorv32 **바이트 동일** · 5,326 → **5,336 tests** · clippy/fmt clean · format 26 불변 · 성능/아티팩트 무변화(200함수×7 SSA = 노드 1,400 추가에 0.20 s → 0.20 s).

**⚠️ 기존 스위트 이빨 0 (재확인)** — `inline_return_seal.rs` 를 지우고 664 cli 테스트를 돌리면 **봉인 되돌리기도, `trusted_w` 제거도 0 fail**. 이 파일이 유일한 게이트다.

#### 4.5.320 캐스트는 자기결정 경계다 — 봉인이 만든 것이 아니라 봉인이 드러낸 것 (2026-08-09, branch feat-size-cast-seal, format 26 불변) ✅

**한 줄** — `N'(expr)` 는 §11.8.1 의 자기결정 경계인데 IR 이 그 경계를 **실체화하지 않아서**, 바깥 문맥이 캐스트를 뚫고 들어가 연산 전체를 다시 돌렸다.

**착수·그라운딩** — ROADMAP §2 의 🔴 "좁히는 캐스트가 더 넓은 대입 문맥에서 안 잘린다" 를 재현하고 메커니즘까지 몰았다. `lower_size_cast` 의 세 가지 중 **동일-폭 가지만** 피연산자를 그대로 반환하는데(`extend_to` 는 `Concat`, `select_low` 는 `Select` 를 만들어 이미 자기결정), 엔진에게 `Binary`/`Unary`/`Ternary` 는 **문맥결정**이다. 4,032칸 스윕(이항/단항/삼항 × 피연산자쌍 × 캐스트폭 {2,4,8,16} × 대상 8): **634칸 조용히 오답이고 전부 `대상폭 > 캐스트폭`** — 판별이 이렇게 깨끗한 것은 근인이 하나라는 뜻이었다. 누출 문맥 = 대입·NBA·연속대입·포트연결·`case` 판별식·비교·더 넓은 이항 피연산자·함수인자. concat 부분·`$display` 인자·배열 인덱스는 **자기결정이라 이미 정상**이었고, 그래서 5,305 테스트 중 **한 개도 이 결함을 못 잡고 있었다**.

**수정 = 꼬리를 획일화한다.** `$signed`/`$unsigned` 는 이 저장소가 이미 **봉인**이라 부르는 프리미티브다(`packed.rs` 의 인덱스 봉인이 같은 이유로 쓴다 — `eval_sysfunc_ctx` 가 피연산자를 **자기결정으로** 평가한 뒤에야 문맥으로 resize 한다). 세 가지가 같은 꼬리로 끝나므로 불변식이 한 문장이 된다: *이 함수는 자기결정 N비트 노드를 돌려준다.*

**같은 두 줄의 둘째 결함** — 캐스트가 **상속하는 부호**를 `expr_self_signed`(elaborate 자체 미러)에서 읽고 있었다. 그것은 부호가 **사이드카로 사는** 리프에서 틀린다: 사용자 함수의 선언 반환형(`16'(f()+0)` = `000d`/오라클 `fffd`), 클래스 필드, 배열 리덕션, 그리고 미러가 아예 열거하지 않는 int-반환 시스템함수 가족(`4'($clog2(300))` = `0009`/**두 오라클** `fff9`). 정본 `sim_ir::selfwidth` 로 옮겼다(`index_self_width` → **`canonical_self_width`** 로 개명 · 인덱스 봉인이 첫 소비자였을 뿐이다).

**⭐⭐ 그리고 이 슬라이스의 내용은 봉인이 아니라 봉인이 드러낸 것이다.** 적대 4라운드가 낸 회귀 **다섯이 전부 한 뿌리** — *elaborate 가 피연산자의 모양을 틀리게 안다*:

- **R1 계층 참조**(라운드 1·differential) `32'(u1.s)` PRE `fffffffd`(=두 오라클) → POST `0000fffd`. `ir_bits_of` 가 placeholder 에서 `None` 이라 **폭 32를 지어내고**, 그래서 `N=32` 에서만 동일-폭 가지에 들어간다 — 거기서 PRE 의 `return e` 가 **load-bearing** 이었다(엔진의 해석 후 폭 테이블이 부호확장했다). 792칸 중 28칸.
- **R2 string**(라운드 1·differential) `40'("abcd")` PRE `61626364` → POST `164`. string 넷의 테이블 폭이 0이라 넓힘 가지의 `Concat` 이 **정본 자기폭 N−31** 을 갖고, `$unsigned` 가 정확히 그 폭으로 평가한다. 대역 `33 ≤ N ≤ 61`.
- **S1 이중 평가**(라운드 1·soundness) `extend_to` 의 부호 fill 은 `Select{Bit, base:e}` = 피연산자의 **두 번째 언급**(0 fill 은 상수라 아니다). 정본이 새로 signed 라 부른 것 중 `Call`·`$random` 은 **불순**하다 → `16'(fp(0))` 이 `fp` 를 **두 번** 부르고, `$urandom` 값이 **두 draw 로 조립**되며 스트림이 밀린다. PRE 는 미러가 `Call` 을 무부호라 불러서 도달 불가였다.
- **R-A 클래스 필드**(라운드 2·양 렌즈 수렴) `32'(c.u8 + ua)` PRE `0100` → POST `0000`, 부호 행은 `00c8` → `ffc8`. 클래스 필드는 `Signal{net: 32비트 핸들넷}` 으로 낮춰지고 진짜 폭은 사이드카에만 산다 — R1 과 같은 뿌리인데 `None` 이 아니라 **틀린 `Some`** 이라 첫 가드가 못 봤다.
- **R-B unpacked 원소**(라운드 2·differential) `40'(sm[1]*1)` PRE `xxxx…`(=iverilog) → POST `000000xxxxxxxxxx`. `ast_ctx_signed` 가 `BitSelect` 에 §5.4.1 packed 규칙을 적용해 무조건 무부호라 답한다 — **같은 AST 철자가 unpacked 원소 읽기**이기도 하다. PRE 는 봉인이 없어 바깥 문맥이 다시 넓히며 **우연히** 오라클과 맞았다.

**⭐⭐ 내 R-B 첫 수정이 170칸을 깼다.** 원소에 `None`("못 풀면 옛 경로" 계약)을 답했더니 §4.5.212 문맥 하강을 통째로 버려 `7'(am[0]+4'h3)` 이 오라클 `10` → `00`. 원소 피연산자는 대부분 무부호 형제 옆에 있고 §11.8.1 이 식을 무부호로 만들어 **옛 blanket `false` 가 우연히 맞고 있었다** — 갈리는 것은 전-signed 식뿐이다. 답은 "모른다"가 아니라 **"원소의 부호"** 였고, 그렇게 고치니 1,560칸 **FIXED 456 · REGRESSED 0 · still-wrong 0**(pre-existing `40'(sm[0]*1)` = `00fd` → `fffd` 까지 닫혔다).

**최종 형태 세 문장** — ⓐ 폭이 **믿을 수 있을 때만** 봉인한다(`ir_bits_of` 가 답했고 **정본 폭이 그것을 부정하지 않을 때**; `is_none_or` 라 정본만 declines 하는 `8'(ua ** u1.k)` 는 살아남는다 — 그 칸은 과잉 가드에 한 번 잃었다) ⓑ 넓히면서 **불순한** 피연산자에는 정본의 signed 를 채택하지 않는다(인덱스 봉인이 이미 가진 `expr_is_repeatable` · **폴백은 `false` 가 아니라 미러**여야 한다 — `16'($signed(f()))` 는 미러도 signed 라 `false` 로 두면 PRE 아래로 내려간다) ⓒ unpacked 원소는 자기 부호를 답한다.

**⚠️ 값은 정직하게 적었다** — 넓히기 + 불순 피연산자(`16'(f())`·`int'(f())`)는 부호 수정을 **못 받는다**(PRE 값 유지·오라클 `fffd`/`fffffffd`). 닫으려면 피연산자를 한 번만 부르는 확장 형태나 callee 순수성 술어가 필요하다 → §2.

**⭐ 의도된 오라클 발산 하나** — `16'($stime)` 이 PRE `ffff8000`(=iverilog) → POST `00008000`. 회귀가 아니라 **PRE 의 자기모순**이었다: vita 는 캐스트 밖에서 이미 `$stime` 을 무부호로 읽고(`q = $stime` at t=2^31 → `0000000080000000`, PRE·POST 동일), verilator 5.050 과 IEEE 1364-2005 §17.7.2("returns an **unsigned** integer")도 무부호다. iverilog 만 signed `integer` 를 돌려준다. 두 오라클이 갈리면 규격이 정한다(§4.5.319 선례).

**게이트** — 5,305 → **5,325 tests**(신규 20) · clippy/fmt clean · format 26 불변 · `examples/` 넷 + keccak + picorv32 **stdout 바이트 동일** · 3백엔드 4,032칸 바이트 동일 · 계층 1,080칸 REG 0 · 형태 82칸 FIXED 37 REG 0 · 클래스필드 780칸 중 63 delta(35 정답화 · **정답→오답 0** · 28 은 양쪽 다 x).

**⚠️ 절차 교훈 둘.** ① **기존 스위트에 이 결함의 이빨이 0개였다** — 캐스트를 쓰는 테스트가 10파일 287군데 있는데도, 리뷰어가 만든 캐스트-쪽 뮤테이션 14개 중 **하나도** 기존 스위트를 건드리지 못했다(신규 파일이 유일한 게이트). ② **라운드 4까지 간 것은 리뷰가 아니라 스코프 신호였다** — 매 라운드가 "모양을 틀리게 아는 새 자리" 를 하나씩 냈고, 그건 인스턴스가 아니라 CLASS 였다. 그 CLASS 의 나머지(prim cast 의 문맥폭 · `ir_bits_of` 의 클래스필드 폭 · `resize_inline_assign` 쌍둥이 · 중첩 캐스트 · formal 바인딩)는 전부 §2 로 내보냈다.

#### 4.5.319 `**` 의 지수는 자기결정이다 — 하나의 불리언이 세 질문을 대신하고 있었다 (2026-08-09, branch feat-pow-self-det-rhs, format 26 불변) ✅

**한 줄** — 엔진의 공유 산술 커널이 두 피연산자를 **하나의 집합 부호**(§11.8.1)로 읽는데 `**` 만 그 계약을 깨고(Table 11-21 = 지수는 자기결정), 커널은 그 불일치를 **지수의 부호를 밑수의 것으로 찍어** 무마하고 있었다 — 비트 해석이 바뀐다.

**착수** — LOOPROMPT NEXT 5번의 곁가지로 등재된 항목. **그라운딩이 기록을 정정했다**: ROADMAP §2 ⓔ 는 근인을 `elaborate/expr_ctx.rs::lower_expr_ctx` 의 자기결정-우변 목록으로 지목했는데, 재현해 보니 상수접기는 이미 정상이고(`localparam R = LS3 ** LU11` = `1011`) 세 백엔드가 같은 답을 내며 근인은 **엔진**이었다 — `eval_core.rs` Pow 팔의 `exp.signed = base.signed`. `4'sd3 ** 4'd11` 이 지수 11 을 −5 로 읽어 Table 11-6 이 0 을 냈다(두 오라클 11). 지수에 여유 비트가 있을 때만 무해했으므로 **폭이 같으면 언제나** 틀렸다. 3,584칸 매트릭스 **336칸 오답, 전부 exit 0**.

**⭐⭐ 첫 수정이 하나의 silent-wrong 을 둘과 맞바꿨다.** 나는 `arith` 의 계약을 건드리지 않으려고 **피연산자 쌍을 등가인 all-signed 쌍으로 다시 표현**했다(부호가 다를 때만 1비트 확장 후 둘 다 signed). 값 축은 336→0 이었고 형태·캐스트 축도 회귀 0 이었는데, 적대 2렌즈가 **같은 뿌리로 수렴**해 둘을 냈다:

- **`WIDE_ARITH_CAP` 경계**(differential): 선언 최대폭(`1<<20`) 피연산자가 그 `+1` 하나 때문에 캡을 넘어 `arith_wide` 가 X-poison 하는데, loud 경고 `W-RUN-WIDE-ARITH` 의 정적 스캔은 **선언 폭**을 보므로 **구조적으로 발화할 수 없다**. `logic [1048575:0] big=3; big ** 0` 이 PRE `0001`·두 오라클 `0001` → POST **조용한 `x`**. correct → silent.
- **`base.width == w` 는 불변식이 아니다**(soundness): 내 byte-identity 논증이 그 등식 위에 서 있었는데, `resize_keep_sign` 은 `is_str`/`is_real` 에서 **short-circuit** 해 자기 폭을 그대로 돌려준다. `"ab" ** 3` 이 64비트 문맥 대신 32비트에서 계산됐다(PRE 15494117157000 → POST 2170120328). ⚠️ **스위트 커버리지 0** — 리뷰어가 Pow 팔에 프로브를 심어 전 5298 테스트를 돌린 결과 **진입 114회 중 `base.width != w` 가 0회**였다. 주장은 스위트가 닿는 곳에서 전부 참이고 그 바깥에서만 거짓이라 아무도 못 잡았다.
- 곁들여 **`x ** 3` 이 1.5× 느려졌다** — unsized 십진 리터럴은 **signed** 라 unsigned 밑수의 가장 자연스러운 철자가 **언제나 mixed** 이고, `+1` 이 그것을 i128 레인에서 멀티워드 커널로 밀어냈다.

**⭐⭐ 교훈, 그리고 최종 설계.** 공유 커널의 계약이 안 맞으면 **피연산자를 계약에 맞게 다시 쓰지 말고 계약을 고쳐라.** 확장 트릭을 통째로 버리고, `arith` 의 불리언 하나를 **그것이 대신하고 있던 세 질문**으로 쪼갰다 — `ArithSigns { lhs, rhs, res }` = 왼쪽을 어떻게 읽는가 · 오른쪽을 어떻게 읽는가 · 결과의 부호는 무엇인가. `**` 아닌 모든 연산자에서 셋이 같은 불리언이므로 **기계적으로 항등** — 주장이라 실측했다(`+ − * / %` × 폭 8 × 값세트 5 × 부호조합 4 = **800칸 PRE↔POST 바이트 동일**, 리뷰어가 다른 값축 1,600칸으로 재확인). 레인 선택은 `l.signed || exp_is_neg` — 비음수 지수는 두 레인이 같은 비트를 내므로(2의 보수 곱셈은 부호 무관 mod 2^w) **실제로 갈리는 형태만** 레인을 옮긴다(라운드 2 리뷰어가 비음수 지수 2,592칸에서 PRE==POST 를 실측). `arith_wide` 도 같은 구조를 받고, `a == minus_one` 행은 `sg.lhs` 로 게이트했다(**무부호 all-ones 는 −1 이 아니다**).

**곁가지 둘, 둘 다 기록이 지목한 자리.** ① `lower_expr_ctx` 의 자기결정-우변 목록에 `Pow` 추가 — 기록의 지목은 근인은 아니었지만 **live defect 는 맞았다**(`logic [7:0] r = a ** '1` 이 fill 을 8비트로 키워 `2**255`, vita 0 / 두 오라클 2). ② `expr_size_ctx` 의 `Pow` 를 `Div|Mod|Shr|AShr` 폭-프로브 팔로 이동 — §4.5.318 이 **138 fixed / 20 broken** 으로 되돌린 그 이동이고, 되돌린 이유가 바로 이번 결함이었다. 뿌리를 고치니 **회귀 0 · FIXED 72**.

**⭐⭐ iverilog 는 폭 ≥ 33 의 음수 지수 `**` 에서 오라클이 아니다**(리뷰어 발견·직접 재확인). 32/33 경계에서 정확히 뒤집힌다 — `(-1) ** -2` 가 폭 32 에서 1, 폭 33·64 에서 **0**; `0 ** -2` 가 폭 32 에서 `x`, 폭 33 에서 **0**. wide 경로가 Table 11-6 의 ±1 행과 0-베이스 행을 통째로 잃는다. verilator 는 ±1 행을 전 폭에서 지키지만 2-state 라 `x` 행이 없고, **상수폴딩 `x` / 런타임 `0` 으로 자기모순**이다. PRE 도 자기모순이었다(`szero ** -2` = x · `uzero ** -2` = 0). **POST 가 처음으로 전 폭에서 균일하게 규격대로**다. 이 사실을 문서에 안 박으면 다음 폭 스윕이 iverilog 의 결함을 vita 회귀로 오독한다 — 리뷰어의 첫 스윕이 정확히 그렇게 **36칸 거짓 REGRESSION** 을 냈다.

**⭐ 오라클이 갈리는 84칸, 두 가족 — 양쪽 다 IEEE 로 착지했다.** (a) 밑수 0 + 음수 지수 → `x`(폭 ≤ 32 의 iverilog 와 일치) (b) **무부호** all-ones + 음수 지수 → 0(verilator 와 일치·iverilog 는 −1 로 읽는다). Table 11-6 은 op1 의 **값**을 보므로 무부호 15 는 −1 이 아니다. 다수결이 아니라 규격이 정했다.

**⭐⭐ 철자는 여섯이고 다섯을 닫았다 — 그리고 "CLASS 라 안 쫓는다" 는 내 판정이 틀렸다.** 지수 자기결정의 철자: 엔진 `arith` ✅ · `lower_expr_ctx` ✅ · `expr_size_ctx` ✅ · **연속대입 하강 ✅**(`lower_expr_ctx` 수정이 덮었다) · **bytecode `const_pow_exponent` ✅**(아래) · elaborate 상수접기 ❌(마지막 하나·§2). 백엔드 census 120칸이 판정을 뒤집었다 — 라운드 2 시점에는 PRE 6 split / POST **12 split** 이라 나는 그것을 *"서로 다른 백엔드가 서로 다른 만큼 고쳐진 결과"* 로 기록하고 CLASS 로 미뤘는데, 마지막 결함을 고치자 **POST 120/120 정답 · split 0**(PRE 는 오답 30 + split 6)이 됐다. **split 이 남는다는 것 자체가 "아직 안 고친 자리가 있다" 는 신호였고, 나는 그것을 스코프 판정으로 읽었다.**

**🔴 그리고 마지막 하나는 내가 이번에 편집한 주석이 거짓말하던 자리였다.** 라운드 2 soundness 가 냈다 — **기본 백엔드(bytecode)** 는 `native_eval` 의 두 번째 철자를 쓴다(작은 상수 지수를 Mul 체인으로 낮춘다). `const_pow_exponent` 가 그 상수를 `to_u128()` 로 읽는데 그것은 **상수 자기 폭으로 마스킹**하므로 `4'sb1110`(−2)이 **14** 로 돌아와 14단 체인이 컴파일됐다. 옆 줄 주석은 *"a negative signed const reads huge here → rejected below"* 라고 적혀 있었고 **이번 슬라이스가 바로 그 주석 블록을 편집했다**(다른 문장을 정정하면서 이 거짓 문장은 못 봤다). PRE 는 세 백엔드가 똑같이 틀려 스플릿이 없었는데, 내가 제네릭 평가기만 고쳐 **기본 백엔드가 참조 인터프리터와 갈렸다**(`u13 ** 4'sb1110` = `1001` / 두 오라클 `0000` · exit 0). ⚠️ **`initial` 블록으로는 재현되지 않는다** — 코드젠이 안 붙어 Mul 체인 레인에 들어가지 않는다. 클럭된 프로세스여야 한다. 수정 = 상수를 **자기 부호로** 읽는 한 줄. 132칸 상수-지수 census: PRE 기본 백엔드 오답 27·split 9 → **POST 132/132 정답·split 0**. ⭐ 라운드 3 이 **값 말고 진단에서도** 같은 split 이 닫힌 것을 쟀다 — Mul 체인은 밑수를 n 번 재-lower 하므로 `m[idx] ** 4'sb1111`(범위 밖 인덱스)이 PRE 에서 15단으로 펼쳐져 기본 백엔드만 E4002 를 **9건**(8-cap 소진) 냈고, POST 는 **1건 = interp/native 와 동일**이다.

**⭐ 뮤테이션 넷이 전 스위트를 생존했고, 넷 다 "관측 채널이 없는 설계" 였다.** 결과 부호 규칙(무부호 밑수는 `l.signed` 와 `both_signed` 를 **구별하지 못한다** — 구별자는 **부호 밑수 + 무부호 지수**이고 `%b` 로는 부호가 안 보인다) · wide 레인의 부호 배선 셋(파일의 wide 행에 **음수 지수가 하나도 없었다**). 넷 다 리뷰어가 준 킬러를 오라클로 확정해 심었고 **넷 다 죽는 것을 실측**했다. 곁: `assert!(out.contains("2"))` 가 8비트 결과의 **256값 중 94개**에서 통과하는 준-공허 단언이라 `assert_eq!` 로 바꿨다. ⭐ **그리고 끝까지 살아남은 넷은 전부 등가였고, 등가인 이유가 곧 설계의 성질이다** — `lhs`/`rhs`/`res` 를 분리해 두면 커널 선택이나 확장 플래그를 흔드는 뮤테이션이 **값에 닿지 못한다**(예: 우변의 `resize_keep_sign` 부호를 바꿔도 진입 시 `r.width == w` 라 플래그만 다시 쓰이고 아무도 `re.signed` 를 읽지 않는다). **생존이 항상 이빨 구멍인 것은 아니다 — 어느 쪽인지는 코드 논증 + 코퍼스 전수 차분으로 갈라야 한다.**

**⭐ 살아남은 뮤테이션 셋, 전부 "판별 못 하는 설계".** 결과 부호 규칙을 지우는 뮤테이션이 살았다 — 신규 파일의 mixed 행이 전부 **signed 밑수**(찍기가 no-op)이거나 결과가 `0000`/`xxxx`(`%b` 로 부호가 안 보인다)였다. 빠른 경로 가드를 지우는 뮤테이션은 **전 5298 테스트를 통과**했다 — 그 가드의 관측 가능한 축이 정확히 위 BLOCKING 둘뿐이었기 때문이다(**생존이 갭이 아니라 결함의 증거**였다). 넷을 지어 메웠다.

**게이트** — 5291 → **5305 tests**(신규 14) · clippy/fmt clean · 코퍼스 10런 PRE 대비 바이트 동일 · 성능 1.38 → 1.35 s. 라운드 2 differential **6,660칸 CLEAN**(신규 silent-wrong 0 · 백엔드 split 0 · 패닉 0).

⚠️ **절차 사고**: 라운드 1 의 soundness 리뷰어가 뮤테이션(`if false`)을 **공유 트리에 남긴 채** 다른 측정을 진행했고, 내가 그것을 출하된 결함으로 읽을 뻔했다. 라운드 2 브리핑부터 **복사한 소스 트리에서 뮤테이션할 것**을 요구했다.

#### 4.5.318 크기 캐스트: fill 크기와 §11.8.1 전-피연산자 강제 — 그리고 지어서 잰 뒤 되돌린 leaf 해석 (2026-08-09, branch feat-size-ctx-leaf-resolve, format 26 불변) ✅

**한 줄** — 두 개를 고쳤고, 셋째(원래의 본체)는 **지어서 재고 되돌려** ROADMAP §2 에 실행 가능한 형태로 남겼다.

**착수** — 직전 반복이 §2 에 적어 둔 항목을 mechanism-level 로 재그라운딩했더니 증상이 기록보다 훨씬 넓었다. `ast_ctx_signed` 가 leaf 하나라도 `None` 이면 캐스트가 문맥 하강을 **통째로** 건너뛰므로, 그 leaf 종류에서는 §4.5.212 의 carry 결함이 살아 있다 — 63칸 매트릭스 **35칸 발산**, 판별자는 완벽히 규칙적이었다(넷·`localparam int` 는 0/9, param/localparam/함수호출/패키지파라미터는 각 7/9, 발산 연산자는 정확히 **넓히는 일곱**).

**남긴 것 둘.**

1. **unsized fill 은 `n` 에서 지어진다.** fill 은 **값이 폭에 의존하는 유일한 leaf** 인데 `lower_size_leaf` 가 `lower_expr` 로 1비트에 짓고 확장했다(`4'(a+'1)` = `1110`, 오라클 `1100`). `lower_ctx_or_plain` 은 비-fill 에는 **문자 그대로 같은 함수**(`expr_contains_fill` 이 false) → byte-identity 가 구조적. §2 ⓒ/ⓓ 종결.
2. **§11.8.1 은 결과가 아니라 모든 피연산자를 강제한다.** §4.5.316 의 좁힘 가지가 피연산자를 자기결정으로 낮추고 부호는 **결과에만** 찍어, unsigned 식 아래의 signed 부분식이 부호확장된 채 남았다(`4'(a+((i13|s4)>>2))` = 12, 두 오라클 0). 재귀만이 안쪽에 닿는다 → 1,620칸 **FIXED 207 · REGRESSION 0**.
   곁: `range_to_dims` 의 **부호 절반을 `kind_signedness` 로 추출**(부호는 range 에 의존하지 않는다 — 리뷰어가 팔마다 읽어 확인)하고, `param_const_signed` 를 lowering(`const_param_expr_w`)이 **실제로 호출**하게 했다.

**⭐⭐ 뺀 것 — 그리고 왜 빼는 것이 옳은가.** leaf 해석은 35칸을 고치지만 세 하위 차단이 동시에 열린다(전부 실측):

| 차단 | 증거 |
|---|---|
| 안쪽 self 폭 > `n` 이면 폭 모델이 틀림 | 6,720칸 중 **42칸 correct→silent-wrong**(`4'(13+(PS>>2))` PRE −4 = 두 오라클 / POST 0). net 쌍둥이는 PRE 도 틀림 = **맞바꾸기** |
| `extend_to` 가 피연산자를 두 번 참조 | `40'(rnd(0)+0)` 이 함수를 **두 번** 부르고 `$random` **두 draw 를 한 값에 섞는다**(exit 0) — §4.5.310 의 부류 |
| formal/local 이 결합 바인딩 집합에 없음 | lowering 을 흉내 내면 그것들을 지나쳐 동명 파라미터의 **부호**를 가져온다(52칸 중 11 회귀). 넷 우선이면 generate localparam 이 가린다 — **어느 고정 순서도 규칙이 아니다** |

⭐⭐ **`**` 이동도 되돌렸다** — 138칸을 고치고 **20칸을 조용히 틀리게** 한다. 리뷰어가 뿌리를 캐스트 **밖**에서 찾았다: `lower_expr_ctx` 의 "우변 자기결정" 목록에 `Pow` 가 없어 지수가 밑수의 부호를 따른다(`s3 ** 4'd11` = 0, 두 오라클 11 · **폭이 같을 때만**). 그것을 고치면 이동은 공짜다.

**⭐⭐ 이번 반복의 교훈은 스윕이다.** 내 스윕이 **세 번 연속 "0 regressed"** 를 냈고 세 번 다 리뷰가 다른 형태로 뒤집었다 — 1차는 fill(60칸), 2차는 unsigned 바깥 아래 signed 부분식 + 음수 지수(20칸), 3차는 **중첩된 폭-민감 부분식**(42칸). 크기는 1,764 → 1,620 → 6,720 으로 늘었지만 매번 **곱하지 않은 축**이 있었다. 축 목록을 스크립트 상단에 적는 습관을 들였는데도 그랬다 — 빠진 축은 **자기가 없다는 것을 알려주지 않는다**.

**게이트** — 5,291 tests green · 두 스윕 **REGRESSION 0** · 코퍼스 12런 main 대비 바이트 동일 · clippy/fmt clean.

#### 4.5.317 크기 캐스트의 real 거부를 퍼널로 — 한 자리를 막는 것이 왜 절반인가 (2026-08-09, branch feat-real-cast-consistent-loud, format 26 불변) ✅

**한 줄** — 크기 캐스트 `N'(…)` 의 피연산자에 real 이 오면 **하강이 만드는 모든 자리에서** 거부한다. 시작점은 `4'(r*2)` 가 `r = 7.5` 에서 `0000` 을 찍는 것이었다(참값 15 = `1111`) — 리사이즈가 f64 를 비트 벡터로 읽는다.

**첫 수정은 절반이었다.** `lower_size_leaf` 한 자리에 가드를 넣었더니 세 부류가 그대로 조용했다.

| 누수 | PRE | 자리 |
|---|---|---|
| 시프트 양 `4'(u << r)` | `0000` exit 0 | `lower_expr(rhs)` — 가드를 안 지난다 |
| `/ %` 좁힘 가지 `8'(u / r)` | `00110011` exit 0 | `lower_ctx_or_plain`(§4.5.316 이 만든 가지) |
| `4'($signed(r)*2)` | `0000` exit 0 | `$signed` 래퍼가 도메인을 가린다 |

⭐⭐ **첫 번째가 이 슬라이스의 교훈이다** — **캐스트 밖의 똑같은 `u << r` 은 이미 loud 였다**(`E3009 bitwise/shift/reduction not defined on real operand`). 크기-캐스트 하강이 시프트를 자기 철자로 다시 짓느라 **이미 있던 게이트를 침묵시키고 있었다**. "피연산자" 의 두 번째 철자가 곧 게이트가 그것들을 잃은 방법이다.

**수정.**

1. `refuse_real_size_operand` 한 함수를 하강이 만드는 **여섯 자리 전부**에 놓았다(넓힘 가지 시프트 양 · 좁힘 가지 lhs/rhs/시프트 양 · `Pow|Shl|AShl` rhs · leaf). 삼항 **조건**만 의도적으로 제외한다 — 비영 판정일 뿐 리사이즈되지 않으며 `-0.0`·`0.0`·`1e-300` 셋 다 iverilog 와 일치한다.
2. `expr_is_real` 에 **`$signed`/`$unsigned` 통과 팔**. 캐스트-로컬 래퍼로는 안 된다 — 래퍼가 **연산 아래**에 있어서(`Binary{Mul, $signed(r), 2}`) **재귀만** 볼 수 있다. 전역이라 소비자별 40설계 배터리로 쟀다: **correct→loud 0**, 조용→loud 4(`%h` 포맷·concat·비트선택·배열 인덱스 — 넷 다 PRE 가 f64 비트를 읽고 있었고 iverilog 도 넷 다 거부), 조용한-오답→**correct 1**(`int'($signed(r))` `0`→`8`). 전부 사다리 상승.
3. **캐스트당 진단 1건**. iverilog 가 그렇고, 잎마다 내면 `MAX_ELAB_ERRORS` 캡을 N배 빨리 먹어 **무관한 뒤쪽 진단이 사라진다**(250잎 + 미선언 넷 → `E3010` 소실을 실측). `lower_size_ctx_entry` 가 플래그를 저장·복원하므로 **중첩 캐스트는 자기 것을 낸다**.

**측정.** 20행 결함 배터리 전부 loud(iverilog 전부 거부) · 합법 22형 PRE=POST=iverilog · 정수 피연산자 **840설계 ~10,500행 PRE≠POST 0칸** · 코퍼스 12런(examples 8 + keccak 3 + picorv32) **main 대비 바이트 동일** · 백엔드 9런 동일 · 5289 tests green.

**⭐⭐ 적대 2라운드 — 제품 결함 0, 그런데 내 주장은 셋이 거짓이었다.**

- *"`coerce_sign` **앞**이라 필요하다"* — **이 커밋이 스스로 전제를 지웠다.** 같은 슬라이스의 `$signed` 통과 팔이 `refuse(coerce_sign(x)) ≡ refuse(x)` 로 만들어, 리뷰어의 swap 뮤테이션이 전 게이트를 통과한다. 첫 cut 에서는 참이었던 문장이 그대로 남아 있었다.
- *"캐스트 팔의 `cast_operand_is_real` 은 못 잡는다"* — 과장. `Div`/`Pow` 는 `expr_is_real` 의 `Binary` 팔에 있어 `8'(r/r2)`·맨 `4'(r)` 은 **이미** 같은 메시지로 loud 였다. 못 보는 것은 leaf 가 이미 `Select`/`Concat` 으로 감싼 경우와 `Mod`/시프트/비트연산 뿌리뿐이다.
- 메시지 사이트 수 *"셋 → 다섯"* → 실제 **둘 → 셋**.

**⭐⭐ 그리고 완전성 주장을 반증당했다.** 차분이 288칸 매트릭스로 **84칸이 여전히 조용**함을 쟀다(PRE==POST). 판별자는 셋의 곱이다 — 반대편이 **fill(`'0`/`'1`)** ∧ real 원천이 **평범한 real 넷이 아님**(`parameter real`·리터럴·함수 반환·`$signed(r)`·`$realtime`·`$sqrt(r)`) ∧ 연산자가 **real 비전파**(`& | ^ << >> >>> %`). 두 자물쇠가 같이 열려야 샌다: `ast_ctx_signed` 가 그 leaf 에서 `None` 이라 하강에 **안 들어가고**, `expr_is_real` 의 `Binary` 팔에 그 연산자들이 **없다**. 하나만 고치면 다른 하나가 남으므로 **인스턴스가 아니라 CLASS** → §4 분리 규칙대로 ROADMAP §2 로 내보내고, 테스트 이름과 doc 에서 "every operand position" 이라는 주장을 **`every operand the size-context lowering builds`** 로 좁히고 탈출구를 명시했다.

**⭐ 뮤테이션 20 중 생존 4, 셋이 진짜 이빨 구멍이었다.**

- **중첩 캐스트가 바깥 캐스트의 보고를 훔치는 방향에 테스트가 0개.** 내 테스트는 **형제** 캐스트를 썼는데, entry 가 들어갈 때 플래그를 리셋하므로 형제는 **구조적으로 면역**이다 — 플래그가 실제로 지키는 방향을 시험하지 않았다. 킬러 `8'({4'(r2*2)} + r)` 에서 concat 이 load-bearing 이다(`ast_ctx_signed` 가 `Cast` 에 `None` 을 주므로 맨 중첩은 하강에 안 들어간다).
- **좁힘 가지의 시프트 양을 아무 행도 증명하지 않았다** — `>>` 행이 전부 `logic [7:0]` 이라 `w == n` 이 되어 **전부 넓힘 가지**를 탔다. 32비트 피연산자가 유일한 도달 경로다.
- **`$signed(r, u)` 는 `$signed(u, r)` 를 판별하지 못한다** — real 이 0번 슬롯이면 `args.first()` 도 잡는다. ⭐ 킬러를 **틀린 슬롯으로 지었다가** 뮤테이션이 생존해서 알았고, 술어를 `args.iter().any()` 로 바꿨다(vita 는 불법 2인자 `$signed` 를 아직 조용히 받는다 → §2).
- 넷째(placeholder 반환 삭제)는 **등가**다: 보고 시점에 이미 exit≠0 이라 시뮬레이션이 돌지 않는다.

⚠️ **절차 사고 둘.** 뮤테이션 스크립트가 **소스만 복원하고 바이너리를 안 지어서** 잠시 뮤턴트 바이너리로 측정했다(리뷰어도 같은 함정으로 staged BLOCKING 을 냈다가 철회 — `vcmp`/`velab`/`vrun` 은 `required-features` 라 평소 빌드에 없다). 그리고 게이트를 `cmd | tail` 로 묶어 **`velab` 의 exit code 대신 `grep` 의 것을 읽었다**(§5 가 경고하는 바로 그 함정).

**곁가지.** `expr_cast.rs` 가 1007줄이 되어 1000줄 정책을 넘겼다 → 크기-문맥 하강 일가를 `expr_size_ctx.rs` 로 기계적 분리(631 + 386). `lib.rs` 1154줄은 pre-existing(예외 목록에 없음).

#### 4.5.316 사이즈 캐스트의 문맥 규칙 — `max(self, N)`, 그리고 "0 regressed" 를 두 번 반증당한 것 (2026-08-08, branch feat-size-cast-maxwidth, format 26 불변) ✅

**한 줄** — `N'(expr)` 가 IEEE 1800 §11.8.1 대로 **`max(self, N)`**(문맥은 넓히기만) + **부호 무조건 전파** 로 평가된다. 직전 반복이 규칙을 핀하고 구현을 되돌린 자리를 이었고, **내가 두 번 주장한 "회귀 0" 을 리뷰가 두 번 다 반증했다**.

**결함** — §4.5.212 가 캐스트 폭 N 을 피연산자 문맥으로 그대로 내려보냈다. 넓힐 때는 옳다(`8'(a*b)` 가 캐리를 지킨다). **좁힐 때도 같은 코드가 돈다는 것을 아무도 안 쟀다**: `integer k=7` 에서 `2'(k%4)` 가 **`xx`**(2비트 문맥이 제수 `4` 를 `0` 으로) · `3'(k%4)` `111` · `2'(k/2)` `00` · `2'(k>>1)` `01`, 중첩도 뚫린다. iverilog 13 과 verilator 5.050 이 일치하고 vita 만 다르다.

**구현** — `N` 도 self 폭도 단독으로는 틀리므로 `lower_size_ctx` 의 `Div|Mod|Shr|AShr` 팔이 **두 방식으로 낮추고 §11.8.1 이 고르는 쪽만 참조**한다(`n >= w` 면 기존 문맥 경로, 아니면 자기 폭). 참조되지 않는 eid 는 죽어서 값이 두 번 평가되지 않는다 — 리뷰가 `$random` 스트림·`$display` 부작용 호출 수·E4002 개수와 순서로 실측 확인했다.

**⭐⭐ "회귀 0" 을 두 번 반증당했다**

- **1차(직전 반복)**: 그 넷을 `lower_size_leaf` 로 보냈다. 그 함수는 문맥을 **통째로** 버려서 좁힘 886칸을 고치며 넓힘·부호 **856칸을 깼다** — 맞바꾸기. 되돌리고 규칙만 기록했다.
- **2차(이번)**: `max(self,N)` 을 제대로 구현하고 **1675칸 스윕에서 FIXED 88 · REGRESSED 0** 을 쟀다. 리뷰가 **~74,400칸**을 자기 형태로 다시 재서 **1,586칸 회귀**를 냈다 — 전부 안쪽 `>>>` 하나였다. ⭐⭐ 좁힘 가지가 **부호 강제를 빠뜨렸고**, `>>>` 에서 그것은 **연산 자체를 바꾼다**(무부호 좌변 = 논리 시프트, §11.4.10). `/ % >>` 는 plain 경로가 이미 문맥 무부호를 적용해 안 보였다 — **한 연산자만 갈린 것이 신호였다.** 수정 = 좁힘 가지에서 피연산자를 `ext` 로 강제(`coerce_sign`)하고 `!ext` 인 `AShr` 을 `Shr` 로. ⭐ 그 수정은 복구가 아니라 **추가 수정**이었다: `>>>` 8형태 중 5개는 **PRE 에서도 틀렸다**.

**교훈** — ⭐⭐ **내 스윕이 넓다고 판별력이 있는 것이 아니다.** 1675칸이 0 regressed 를 냈고 74,400칸이 1,586 을 냈다 — 차이는 크기가 아니라 **형태**(내 피연산자 쌍에 "무부호 형제 + 안쪽 `>>>` + 좁힘" 조합이 없었다). ⭐ **한 연산자만 갈리면 그것이 신호다** — 넷 중 셋이 정상이고 하나가 아니면 공통 규칙이 아니라 그 하나의 성질을 놓친 것이다.

**검증** — 1675칸 스윕 두 오라클 일치 1645칸에서 FIXED 88 · REGRESSED 0 · both-wrong 0 · 리뷰의 ~74,400칸 스윕(3-백엔드 split 0) · dead-eid 주장 실측 확인 · 신규 6 테스트(전부 **좁힘 칸과 넓힘 칸을 둘 다** 갖는다 — 한 방향만 넣은 것이 1차 맞바꾸기를 통과시켰다) · 되돌렸던 구현이 가드 둘에 죽는 것 확인 · 5286 tests green · clippy/fmt clean · `examples`·keccak·picorv32 PRE↔POST 바이트 동일.

**남긴 것** — ⓐ `w = ir_bits_of(plain)` 은 **노드**의 폭이지 캐스트 피연산자 **전체**의 self 폭이 아니다(더 넓은 형제가 올린다 — `2'((s8>>u3)*s16)` 11칸, 트리 전역 self 폭 패스가 선행조건) ⓑ `N'(real …)` 좁힘이 이제 loud(iverilog 도 거부라 사다리 상승이지만 `4'(r*2)` 는 여전히 조용해 비일관) ⓒ `**` 는 음수 지수에서 저비트 폐쇄가 아니다 ⓓ 4-state 에서 `+ - *` 좁힘이 x 를 떨어뜨린다(전부 pre-existing).

#### 4.5.315 ③층 S2 — 1비트 결과 연산자 가족을 admit 한다 (2026-08-08, branch feat-s2-onebit-ops, format 26 불변) ✅

**한 줄** — `wprog` 가 `&&`·`||`·`!`·6 reduction·`===`·`!==` 를 받는다 → picorv32 admission **60.8% → 66.3%**, native **1.085 → 0.984 s(1.10×)**, native/vm **0.81× → 0.88×**. 의미는 **한 줄도 재진술하지 않았다**(자유 함수 7개 추출 + 기존 메서드 위임).

**표적은 셈이 정했다.** 직전 반복이 "admission 이 표적" 까지 좁혔고, 이번엔 **무엇이 거절되는가**를 스크래치 사본의 카운터로 서로 다른 식 단위까지 분해했다 — `&&` 100 · `===` 91 · `?:` 34 · `||` 20 · `|x` 18 · `!` 13 이 ~300 중 **276**. 여섯이 전부 **1비트 결과 연산자**라 한 가족이고, 한 슬라이스다.

**구현 = 추출.** `binops::{case_eq, log_and, log_or}` · `sysfunc::truthiness` · `eval_core::{reduce_word, reduce_bit}` 를 자유 함수로 꺼내고 기존 `EvalCtx` 메서드는 위임한다. 신규 `unary_self_of` 는 `eval_unary_self` 의 본문 그대로(=`!` + 6 reduction 매핑 전부)라 W 실행기와 제네릭이 **같은 함수**를 부른다. `===`/`!==` 는 별도 arm 이 아니라 **기존 비교 arm 에 합류**했다(피연산자 admission 이 이미 같다).

**⭐⭐ 정확성 논거는 측정된 사실 하나에 선다** — 제네릭 평가기는 `&&`/`||` 를 **단락하지 않는다**(`let l = self.eval(lhs); let r = self.eval(rhs);`). 그래서 W 프로그램의 즉시 평가는 선택이 아니라 **같은 순서**다. 이것이 값이 아니라 **진단** 문제이기 때문에 중요하다: admit 된 서브트리도 `LoadIdx` 로 범위 밖 읽기를 **셀 수 있다**. ⚠️ 같은 논거는 `?:` 로 **전이되지 않는다**(제네릭은 취한 가지만 평가한다) — 그래서 `?:` 는 여전히 거절이고, 34건은 남는다.

**⭐⭐ 적대 2렌즈: differential CLEAN · soundness 신규 silent-wrong 0 — 그러나 뮤테이션 둘이 살아남았고 둘 다 내 테스트 설계 실수였다**

- **`lw ≠ rw` 커버리지가 저장소 전체에 0** 이었다. 내가 그 목적으로 넣은 행 `a && (a<b)` 는 `lw=4, rw=1` = **무해한 방향**(작은 값을 넓게 읽으면 0비트만 붙는다). 위험한 방향은 `lw < rw` 이고 행이 없었다 → `LogBin` 이 두 피연산자에 **같은 폭을 쓰는** 뮤테이션이 5280 테스트를 통과했다. 그리고 **picorv32 는 그 형태를 실제로 만든다**(`(1,1) (1,2) (1,5) (1,32) (32,1)`). 판별 행 `(a<b) && b`·`!a || b` 추가로 kill. ⭐ 직전 슬라이스의 교훈(*판별 못 하는 값으로 쓴 테스트는 기능을 증명하지 않는다*)이 **한 반복 만에 같은 모양으로 재발**했다 — 이번엔 값이 아니라 **방향**이었다.
- **이 슬라이스의 correctness 논거 자체에 테스트가 없었다.** 상수-lhs 단락을 넣는 뮤테이션이 전 스위트를 통과하고 native 를 **exit 1 → exit 0, E4002 2건 → 0건**(loud→SILENT)으로 만든다 → `agree()` 하네스에 판별 설계 추가(`m[0] && m[oob]` = 확정 false 인 lhs 로도 rhs 가 읽혀야 한다). 모듈은 `Shl` arm 에 **같은 hazard 를 이미 문단으로 적어 두고** 있었다.
- 거짓/stale 주석 셋: 모듈 헤더의 admit 목록이 슬라이스 前 상태 · *"TWO 예외"* 가 이제 **넷**이고 한 프로그램에 **네 폭**이 실제로 나온다(`(sa<sb) && m[idx]` = 1/8/8/4) · *"재진술 안 한다"* 목록 누락. 그리고 새 variant 가 **`Cmp` 의 doc 주석을 가로챘다**. 폭 가드 둘은 **구조상 도달 불가**(뮤테이션으로 0 히트 실측) — 지우지 않고 *"보험이지 검사가 아니다"* 를 적었다.

**검증** — differential 이 무작위 445 설계 × 3백엔드 + PRE/POST + VCD, **iverilog 핀 116,684 스칼라 비교**(폭 1~66·혼합 폭·signed) **0 diff** · 전수 배터리 55 형태 × 4-state 전조합 = **3,604,480 비교** 제네릭 일치 · soundness 가 마스킹 불변식을 어서션으로 심어 12 설계에서 GREEN(그리고 `Shl` 의 `& m` 을 지우면 발화 = teeth 증명) · `examples`·keccak 3변종·picorv32·staged 전부 native↔vm 바이트 동일 · 5280 tests green · clippy/fmt clean · format 26 불변.

**⭐ 곁가지 발굴 2건(전부 pre-existing·PRE==POST)** — ⓐ **`N'(expr)` 사이즈 캐스트가 `%`·`/` 위에서 조용히 갈린다**(`integer k=7` 에서 `2'(k%4)` vs iverilog `11`, `2'(k/2)` = `00` vs `11`, exit 0 · `2'(k)`·`2'(k&3)` 는 정상) — **오라클 있는 silent-wrong** 이라 ROADMAP §2 최상단. ⓑ `inside` 가 IEEE §11.4.13 의 `==?` 와일드카드를 안 한다(`8'h0f inside {8'hxx}` → `x`) — iverilog 가 `inside` 자체를 거부해 무오라클.

**잔여** — `?:`(34) 는 단락 때문에 별개 슬라이스 · `Select`(12) · `Concat`(4) · `SysFunc`(3) · 폭 불일치(14).

#### 4.5.314 IEEE 1364-2005 §12.2 암시적 파라미터 포트 리스트 — 그리고 답이 "고치기" 가 아니라 "빼기" 였던 축 (2026-08-08, branch feat-implicit-param-ports, format 26 불변) ✅

**한 줄** — ANSI 헤더가 없는 모듈(`module m; parameter W = 8;`)의 본문 파라미터를 **네 override 채널 전부**가 도달하게 하고(PRE 는 넷 다 loud 거부 = 코어 Verilog-2005 의 거부), `-G` 를 staged 경로에 배선했으며, `'x`/`'z` override 의 조용한 0/all-ones 설치를 닫았다. **적대 4라운드가 결함 24건**을 냈고 그중 **넷은 내 앞 라운드 수정이 만든 하강**이었다 — 마지막 하나는 고치는 대신 **기능을 슬라이스에서 빼는 것**이 답이었다.

**PRE 가 거부하던 것** — `sub #(.W(11), .D(12))` / `sub #(21, 22)` / `defparam u.W = 44` / `-G W=8` 이 각각 `override of unknown parameter` · `more positional parameter overrides than module parameters` · `names no parameter of any top module`. 즉 비-ANSI 모듈에는 **어떤 방법으로도 파라미터를 넘길 수 없었다.**

**구조 = 한 철자로 모으기.** `param_ports(module)` = 헤더가 있으면 헤더, 없으면 **본문 top-level `parameter` 선언**(generate 안은 다른 스코프라 도달 불가 — iverilog 도 거부). 네 채널이 전부 이 목록으로 이름을 푼다. `localparam` 은 목록에 **있고**(그래야 "unknown parameter" 가 아니라 정확한 `cannot override localparam` 이 나온다) **positional 슬롯은 안 먹는다**(`positional_param_ports` — iverilog 핀: `parameter A; localparam L; parameter B;` + `#(10,30)` = A=10 B=30). `bind_params` 를 `resolve_param_overrides`(이름/위치 해석) + **`bind_one_param`**(선언 1개 바인딩)으로 갈라 호출자 셋이 공유 — ANSI 헤더 · 모듈 BODY · **인터페이스 BODY**. 마지막은 PRE 에서 **축소 복사본**이라 인터페이스 본문 파라미터가 `param_meta`/`param_range` 를 기록하지 않았고 string/real/>64비트를 아예 라우팅하지 않았다(`parameter S = "abc"` 가 자기 기본값만으로 E3009).

⭐⭐ **첫 컷은 override 된 선언만 공유 경로로 보냈다** — 그러자 **파라미터의 등록 폭이 "override 됐는가" 에 달리게 됐다**: `ifc #(.P(8'hA5)) a(); ifc b();` 가 같은 값인데 `a.P[15:12]=a`, `b.P[15:12]=0`(exit 0). 선언 전부를 한 바인더로.

**staged `-G`(doc-14 RULE B)** — `velab -G` 는 플래그를 파싱하고 `errors=0` 을 보고하면서 **선언 기본값으로 elaborate** 했다(silent wrong-design). `-L` compose 경로도 같았다. `vcmp`/`vrun` 은 받아서 버렸다 → 이제 wrong-stage loud. ⚠️ **`-G` 를 RULE-V 다이제스트에 섞으면 안 된다**(시도했다): 그 필드는 **업스트림** `.vu` 의 다이제스트라, 섞으면 모든 `-G` 빌드가 바뀌지도 않은 파일에 대해 `E9003 digest changed` 를 낸다. RULE B 는 **자기 헤더 필드**(format bump)가 필요 → ROADMAP §0-14 잔여.

**⭐⭐ 라운드 1(soundness) — 결함 8, 하나는 이 슬라이스가 만든 하강**

- **fill 은 자기 폭이 없고 타깃의 폭을 받는다.** `#()` 만 원문을 나르고 `defparam`·`-G` 는 **부모 쪽에서 32비트로 먼저 접었다** → 64비트 파라미터에서 `defparam u.K='1`·`-G K='1` 이 `0000_0000_ffff_ffff`, 같은 설계의 `#(.K('1))` 은 `ffff…ffff`. **한 설계 안에서 두 철자가 갈리고 exit 0.** 본문 파라미터가 override 가능해지면서 PRE 의 loud 가 이 자리로 내려왔다 = **내 하강**. 수정은 소비자 보정이 아니라 **퍼널** — `DefparamOverride` 3번째 성분과 `cli_overrides_for` 가 fill 을 원문으로 나르고, 선언 폭을 아는 유일한 자리가 다시 접는다.
- **localparam 판정을 맨 위 한 자리로.** 아래 진단들은 전부 *override 가 무엇을 하는가*(안 접혔다·문자열이다·x/z 평면이 없다)를 말하는데 **localparam 이 거절하는 이유는 그중 어느 것도 아니다** — `#(.L(sig))` 가 "parameter `L` 은 상수가 아니다"(틀린 명사·틀린 이유)라 했고 fill-only override 는 **진단 없이** 도달했다. iverilog 는 `Cannot override localparam`.
- **거짓 주석 둘** — `-G` 의 *"`'0`/`'1` 은 value 경로가 이미 타깃 폭으로 접는다"*(반증) · 인터페이스 주석이 **방향이 반대**였고 남은 복사본을 2개로 셌으나 실제 **3개**(`package.rs` 누락).
- 위생 — `params.rs` 1025줄(→ 857 + `param_query.rs` 226) · `.f` 의 **분리형** `-G W=8` 이 `W=8` 을 경로로 해석(붙인 `-GW=8` 은 정상 = 한 플래그 두 철자 중 하나만 깨져 스윕을 통과했다) · `-v` echo 에 `-G` 행 없음.

**⭐⭐ 라운드 2 — 결함 10, 둘이 내 라운드-1 수정이 만든 하강**

- **`hier_params` 는 i64 테이블이다.** real 파라미터를 거기 등록했더니 **이름은 풀리고 값은 실수성을 잃었다** — `parameter real P = 4.5;` + `#(.P(9))` 가 안에서 `P/2 = 4.5`, 밖에서 `a.P/2 = 4.0`, exit 0.
- **`-G` 의 fill 을 `value: None` 으로 보낸 것이 두 번째 하강.** *fill 이 cannot-apply 인지 결정하는 모든 가드가 `by_name` 을 읽는다* — `-G S='1`(string) · `-G R='1`(real) · `-G T='1`(폭 없는 `time`)이 **플래그를 안 준 것과 바이트 동일한 출력에 exit 0**(PRE 는 string 에 대해 loud). 수정 = `#()` 와 **같은 모양**을 내게 한다.
- **`#()` 의 fill 이 나중 `defparam` 을 이겼다**(pre-existing·퍼널에 붙어 있다): 채널마다 독립 insert 라 나중 override 가 **다른 채널**의 stale 엔트리를 못 지웠다 → `clear_target`(채널 간 last-write-wins · `.W()` 는 override 가 아니므로 `had_value` 게이트). iverilog 6 / vita 0 이었다.
- **좁은 `'x` 선언 기본값의 진단이 틀린 이유를 댔다** — `'x` 는 **접히는 상수다**(iverilog `xx`). 없는 것은 표현이다 → 세 폴드가 공유하는 `param_value_unfoldable` 이 도메인을 말한다.

**⭐⭐ 라운드 3~4 — 답은 "빼기" 였고, 어디까지 빼는지는 셈이 정했다**

- differential 이 **~9000 설계**를 재고 `correct→loud 0 · correct→wrong 0` 을 확인하면서, 라운드-2 의 real 수정이 **캐스트를 조용히 깨뜨린다**를 잡았다: `int'(a.P)` = **0**, `longint'(a.P)` = **4616752568008179712**(4.5 의 IEEE-754 워드), `integer'` 만 우연히 5. ⭐⭐ 뿌리 = **leaf 를 패치하는데 문맥은 이미 결정돼 있다** — 지연 계층 읽기는 lowering 시점에 `Signal{POISON_NET}` 이라 `lower_cast` 의 `expr_is_real` 이 false 를 보고 정수 경로를 굽고, 해소는 그 뒤에 leaf 만 바꾼다. 같은 브로큰이 real **변수**(`a.rv`)에 대해 **PRE 에도 있다**.
- ⭐⭐ **그래서 어떤 부분 지원도 silent-wrong 을 다른 silent-wrong 과 맞바꾼다**(둘 다 지어서 측정했다): i64 로 실으면 `a.P/2` 가 정수 나눗셈(P=9 → 4, iverilog 4.5), real Const 로 패치하면 캐스트·concat 이 비트를 읽는다. **축 전체를 슬라이스에서 빼고** ROADMAP §2 에 기록했다 — 계층 real 읽기는 정직한 loud.
- ⭐⭐ **그 다음이 이 반복에서 가장 값진 정정이다.** 되돌리면서 인터페이스의 exact-int real twin 까지 뺐고, 근거로 *"PRE 의 그 답은 조용히 틀린 값이고 `P = 4` 가 그것을 가린다"* 를 적었다. **4라운드가 그 문장을 셈으로 반증했다** — 13소비자 × 6값 매트릭스에서 PRE 는 **72칸 정답 · 6칸 오답**이고 오답은 전부 `/` 의 몫이 분수인 칸이었다. 즉 내가 한 것은 silent-wrong 6 을 없애려고 **정답 72칸을 loud 로 되돌린 회귀**였다. ⭐⭐ 그리고 **판별자는 값이 아니라 연산자**였다(`P = 8` 도 `/2` 에서 정답이다 — 내가 "유일한 값" 이라 부른 것은 애초에 값의 성질이 아니었다). twin 복원 · 남은 비대칭(bare 2.5 / hier 2.0)은 ROADMAP §2 에 실측치와 함께 기록.
- ⭐ **되돌리기가 테스트 하나를 같이 지웠고 그 손실이 안 보였다** — `an_x_fill_as_a_declared_default_is_not_folded_to_zero` 가 사라진 동안 그 문구를 되돌리는 뮤테이션이 **전 스위트를 통과**했다(리뷰어가 생존자로 보고해 발각). 복원 + `-G` fill 커버리지 신설.
- ⭐ 거짓 주석 4건(삭제된 함수를 가리키는 주석 · 삭제된 테스트를 인용하는 doc · 존재하지 않는 리더를 주장하는 주석 둘)과 ROADMAP 의 모순 중복 1건도 그 되돌리기가 만든 것.

**교훈** — ⭐⭐ 네 하강이 전부 **한 모양**이었다: *i64 채널에 i64 아닌 것을 넣으면 이름은 풀리고 값은 틀린다*(fill 의 폭 · real 의 실수성 · fill 의 `value: None` · 인터페이스 republish). ⭐⭐ **판별하지 못하는 값으로 쓴 테스트는 기능을 증명하지 않는다** — `P = 4` 는 `4/2 == 2` 라 두 도메인을 못 가르고, 그 한 글자가 하강 하나를 라운드 3까지 숨겼다. ⚠️ **절차 실수 둘**: 리뷰가 도는 중에 트리를 두 번 고쳐(§4 가 §4.5.310 실측으로 금지한 것) 리뷰어의 뮤테이션 배치가 두 번 무효화됐다.

**남긴 것(전부 pre-existing·PRE==POST·측정됨 → ROADMAP §2/§3)** — 계층 read 가 REAL 을 못 나른다(축 전체·real 변수 포함) · 계층 경로가 **스코프 아닌 자식**을 지나 바깥의 동명 인스턴스에 커밋한다 · fill 이 32비트로 접히는 네 형태(>64비트·`time`·untyped·`real`) · 파라미터 선언 fold **네 벌**(정본 + instance/generate/package) · `defparam` 이 인터페이스 인스턴스에 안 닿는다 · run.json 이 `-G` 를 안 싣는다.

**게이트** — 5245 → **5280 tests green**(신규 35, 전부 오라클에 핀) · clippy/fmt clean · **format 26·MsgCode 64 불변** · `examples/` 4종 stdout+VCD · keccak 3변종 · picorv32 전부 **PRE↔POST 바이트 동일**.

#### 4.5.313 외부 리포트 aes_top — 16항목 + 자체 리뷰 17결함 (2026-08-07, branch feat-aes-report, format 26 불변 / **AST 스키마 re-pin**) ✅

**한 줄** — AES IP 개발자의 2판 리포트(§3 구현요청 11 · §4 진단 5)를 **전부 해결**하고, 그 수정에 대한 적대 2렌즈가 낸 **결함 17건**(그중 다수가 이번 수정이 만든 것)까지 닫았다. 리포트가 못 본 silent-wrong 을 **7건 더** 찾았다.

**리포트 진단 5건을 실측이 정정했다** — 착수 전 재현·그라운딩의 값:

| 리포트 진단 | 실측 |
|---|---|
| §3.8 "256비트 localparam" | **폭 제한이 아니다** — `[255:0] B = 256'h1` 은 접힌다. 조건은 **접힌 값이 64비트 초과** |
| §3.7 "override 가 안 접힌다" | **override 무관** — default `"Y"` 만으로도 E3009. 뿌리는 **string 파라미터 자체** |
| §3.6 "generate-scope" | **generate 무관** — 모듈 스코프에서도 실패, 런타임에선 동작. 뿌리는 **크기가 이름인 캐스트** |
| §3.5 "포트 연결 함수 호출" | `4'($clog2(8))` 는 **된다** — 안 되는 건 **사용자 함수** |
| **§4.3 "$display ternary → 숫자"** | **결함이 아니다.** iverilog 가 **같은 숫자**를 낸다(IEEE §5.9: 문자열 리터럴 = packed integral). 값은 그대로 두고 **W3058 경고**만 신설 |

**해결(값·오라클 일치)** — §3.2 unpacked 배열 포트(ANSI·비ANSI·다차원, **원소별 연결**) · §3.3 선택적 import(본문이 자기 패키지에서 해석) · §3.3b `pkg::f()` 제약 완화(제어흐름·중첩 호출 허용, **자유이름 폐쇄만** 유지) · §3.4 콤마 import · §3.5 포트 연결(부모 테이블 스왑) · §3.6 `RPS'()` 폴딩(세 술어 동기) · §3.7 string 파라미터+`==` 폴딩 · §3.8 64비트 초과 파라미터 · **§3.11 `-G`/`--param` 오버라이드**(4구성 스윕이 래퍼 없이) · §4.1 **E4002/W4029 분리** · §4.2 **`$sscanf` scanset 구현**(C 의미·7형태) · §4.4 선언 전 사용 loud · §4.5 **W1018** 부분 timescale(누락 모듈명 표시).

**⭐⭐ 리포트에 없던 silent-wrong 7건** — ⓐ 모듈이 같은 이름 함수를 가지면 패키지 함수 본문이 그걸 부름(1002 vs 4) ⓑ 자식이 같은 이름 함수를 가지면 부모 포트 연결식이 **진단 없이 자식 함수**에 바인딩 ⓒ generate localparam 이 선언 폭을 잃음(`[1:0]` 이 32비트) ⓓ `collect_callee_stmt` 에 **`Return` 팔 없음**(재귀 탐지에도 쓰인다) ⓔ 인덱스 규칙 **네 번째 철자**(`native_eval::exec_vm::word_index` 가 알려진 `32'hFFFFFFFF` 를 unknown 으로) ⓕ 패키지 **태스크** 경로 전체 미배선(1001 vs 43, 999 vs 7) ⓖ `import p::*` + 모듈 동명 helper.

**⭐⭐ 적대 2렌즈가 결함 17건 — 다수가 이번 수정이 만든 것**

- ⭐⭐ **한 뿌리 셋**(차분 F1/F4/F5 + soundness 2/3): *side map 으로 보내는 모든 `continue` 가 같은 슬라이스가 넣은 검사를 건너뛴다.* `real` 경로만 escalation 을 `continue` **앞에** 둬서 유일하게 정상이었다. 증상은 correct→loud(96비트 값 3이 정수 정체성 상실·**내 필드 주석이 정반대를 주장**), loud→silent(string override 가 조용히 default), 값 회귀(`K=0` vs 7).
- ⭐⭐ **admission 과 collection 이 다른 노드 집합**(차분 F3): `pkg_expr_pure_inner` 는 `Cast` 를 받는데 `collect_callee_expr` 엔 `Cast` 팔이 없어, 캐스트 안의 호출이 주입 안 되고 **호출자 모듈의 동명 함수가 조용히** 불렸다(1001 vs 2). 기록된 "accept-gate walker completeness" 규칙 그대로.
- ⭐⭐ **순수성은 루트만, 주입은 전이적**(soundness 1): 자유 이름을 가진 형제가 주입돼 그 이름이 **호출자 넷**에 바인딩(101 vs iverilog 거부).
- ⭐⭐ **선언 전 사용이 plain-static 블록 로컬을 오탐**(차분 F2 + soundness 4): flatten 모델이 **모듈 키에 publish** 해서 내 `own_key` 가드가 그 클래스엔 공허 — 평범한 TB 가 5번 거부됐다. 판별자를 AST 에서 직접 모아 해결.
- ⭐⭐ **센티넬 충돌**(soundness 5): `OFF_UNKNOWN = (1<<30)+1` 이 i32 도메인 안이라 **알려진 1073741825** 가 unknown 으로 분류(exit 1 → 0). `word_index_of` 는 같은 remap 을 갖고 있었고 write 쪽 쌍둥이만 없었다.
- ⭐⭐ **진단 순서**(soundness 7): 두 카운터는 **순서를 못 담는다** — native 만 E4002→W4029 로 역전. 순서 있는 채널로 교체.
- ⭐ 배열 포트가 스칼라 경로의 검사 셋 누락 + ⭐⭐ **내가 만든 silent-wrong**: 자식 `[0:3]` ↔ 부모 `[3:0]` 이 vita 4 / iverilog 1 — IEEE §7.6 은 **위치 대응**이라 flat-index 연결이 원소를 뒤집는다 → 방향 불일치를 loud 로.
- ⭐ **죽은 가드와 거짓 근거**(soundness 8): `use_lo == 0` 가 전 스위트+examples+bench+80설계에서 **0회 히트**이고, 그 주석이 주장한 *"`.*` 를 막는다"* 는 실제로 **range 가드**가 하는 일이었다.
- ⭐ 거짓 주장 9건 전부 실측대로 정정(내가 쓴 *"boundary is the VALUE"* 가 코드와 반대 · *"straight-line"* 술어 doc · *"같은 walk"* · 2002 수치 등).

**⭐⭐ 그리고 테스트가 0개였다** — 리뷰어가 만든 뮤테이션이 **전부 생존**했다(신규 기능 7종에 테스트 없음). `crates/cli/tests/aes_report.rs` **33 테스트** 신설, 전부 **iverilog 값에 핀**(elaborate 성공이 아니라 값 — 잘못된 원소를 연결하거나 두 번째 import 항을 버려도 "동작"하므로).

**게이트** — 5245 tests green(+29) · clippy/fmt clean · MsgCode **61 → 64**(W4029·W1018·W3058) · **AST SchemaHash re-pin**(`AnsiPort.unpacked`·`PortDecl.unpacked` — 전 `.vu` stale, sim-ir format 26 불변) · `examples/` 4 + keccak 3 + picorv32 PRE↔POST 회귀 0(유일 차이 = picorv32 의 E4002 9건 → W4029, **stdout 바이트 동일**).

**⚠️ 미해결 1건** — §3.1 DPI-C 는 설계상 영구 비목표.

#### 4.5.312 ③층 S2 슬라이스 4 — `wprog` 가 런타임 배열 원소를 읽는다 (2026-08-07, branch feat-s2s4-runtime-array-index, format 26 불변) ✅

**한 줄** — 폭별 특수화 평가기가 **런타임 인덱스의 배열 원소 읽기**를 admit 한다(`WOp::LoadIdx`) → 런타임 인덱스 마이크로벤치 **4.013 s → 0.64 s**(native/vm **0.27× → 1.70×**)이고 picorv32 가 **0.83× → 0.97×** 로 올라온다.

**그라운딩이 표적을 정했다**

- S2 슬라이스 3 이후 native 는 **상수** 인덱스에서 1.76× 인데 **런타임** 인덱스에서 0.27× = **VM 보다 3.7× 느렸다**. 원인은 `wprog.rs` 의 `Expr::Signal` 팔이 워드 인덱스를 `Expr::Const` 로만 admit 하고 아니면 **트리 전체를 거절**하는 것 — 그러면 tier-3 는 VM 이 아니라 **인터프리터의 제네릭 평가기**로 떨어진다. 실제 RTL 은 루프 변수로 배열을 인덱싱하므로 이게 picorv32 가 0.83× 이던 이유다.
- **쓰기 쪽은 이미 받고 있었다**(S2 슬라이스 3 의 `IdxKind::Prog`, §4.5.307) — 빠진 것은 읽기 하나였다.

**공유, 재진술 아님** — `Value → 워드 인덱스`(X/Z 이거나 u32 초과면 `u32::MAX` 센티넬) 단계를 `eval::word_index_of` 로 **추출**해 제네릭 평가기와 `LoadIdx` 가 한 철자를 쓴다. 이 규칙은 **진단을 소유한다**(범위 밖 → all-X + `pending_range` 증가 = 런 루프가 드레인하는 E4002)이므로 §4.5.302 규칙이 그대로 적용된다.

**구현** — `WOp::LoadIdx { off, elems, m }` 가 스택 top 을 인덱스로 소비해 `off + idx*2` 의 두 평면 워드를 싣거나, 범위 밖이면 all-X + `arena.note_oob_read()`. 인덱스는 **자기 폭에서 인라인 컴파일**(per-op 마스크가 이미 있다 — §4.5.306 "한 프로그램이 두 폭을 갖는다"). ⚠️ 비교 피연산자가 인덱스 서브트리 안에 있으면 한 프로그램에 **세 폭**이 산다. `WProg::run` 은 `buf` 가 아니라 **아레나**를 받는다(진단 카운터에 닿아야 하므로). 유지되는 거절: multi-word 슬롯 · 비정수 net kind · 컴파일 안 되는 인덱스.

**⭐⭐ 라운드 1 differential 이 내 수정 안에서 진단 중복을 잡았다**

- `fast_offsets` 가 슬롯을 **하나씩 컴파일하며 즉시 실행**했다. 첫 슬롯이 admit + 범위 밖이면 **보고**하고, 두 번째 슬롯이 decline 하면 전체를 `None` 으로 되돌리는데 — 그 뒤 제네릭 resolver 가 **같은 접근을 다시 보고**한다. E4002 는 **런당 8개 cap** 이라 중복이 cap 을 먹고 뒤의 진짜 보고를 지운다.
- 수정 = **결정과 실행의 분리**. 패스 1 이 모든 청크의 모든 인덱스를 `index_admits` 로 판정하고(컴파일+상수폴딩만, 실행 0), 패스 2 만 실행한다. **"decline 은 부작용이 없다"** 가 이제 코드의 성질이다.

**⭐⭐ 게이트가 바로 그 축에 눈멀어 있었다**

- `s2_specialized_offsets_match_the_canonical_resolver` 는 **오프셋만** 비교했다 — 같은 오프셋을 더 빨리 내면서 **보고 수가 다르면** 두 resolver 는 등가가 아니다. `Some` 팔에 보고 수 비교를, `None` 팔에 "decline 은 보고 0" 단언을 넣었다.
- ⭐ 그런데 그 비교조차 **공허할 수 있었다**: corpus 72 설계에 admit 되면서 범위를 벗어나는 인덱스가 **0개**라 `Some` 팔이 늘 0-vs-0 이었다. 라운드 2 가 짚어 판별 3문장(① admit+OOB 뒤 decline ② 전 슬롯 admit + 보고 ③ 2청크 concat 의 두 번째 **offset** 만 decline)을 심었고, 뮤테이션 둘(`LoadIdx` 가 보고 안 함 · 패스 1 이 `c.offset` 미검사)이 **둘 다 kill**.

**라운드 2: 제품 결함 0**

- differential **960설계 0 발산**, soundness *"complete and correct — 새 silent-wrong 0, 사다리 하강 0"*. 남은 것은 **주장 8건 + 테스트 공허 2건**뿐이라 거기서 끊었다(라운드 예산 3 중 2 사용).
- ⭐ 그중 하나는 내가 쓴 *"봉인이 모든 배열-워드 인덱스를 정확히 32비트로 만들므로 이 폭 가드는 도달 불가"* 인데 **측정이 반증**했다 — packed-element 도메인에서 **2비트 `Concat`** 이 도착하고, `packed.rs` 에 봉인을 우회하는 carve-out 이 셋 더 있다. 결과는 안 바뀐다(전부 `Concat` 에서 decline) — **철회되는 것은 가드가 아니라 그 이유**다.
- ⭐ 그리고 `note_oob_read` 의 doc 이 **자기 예시로 든 쓰기 퍼널이 정작 그 함수를 안 거치고** 제자리에서 카운터를 올리고 있었다 → 배선(동작 동일).

**게이트** — 5215 tests green(+32) · clippy/fmt clean · `examples/` 4 + keccak 3변종 + picorv32 **VM↔native stdout·VCD 바이트 동일**(keccak 셋 다 `backend: native`) · 소진 인덱스 differential(−4..=elems+4 + X/Z + u32 초과, **값·unk·보고 수** 3축) · admission census 재핀 `(131, 1)` · 특수화 오프셋 핀 `(2144,60) → (2160,64)`.

**속도(final tree, best-of-3)** — const 인덱스 native 0.52 / vm 0.90 = **1.73×** · **var 인덱스 native 0.64 / vm 1.09 = 1.70×**(PRE 4.013 s 대비 **6.3×**) · picorv32 native 1.03 / vm 1.00 = **0.97×** · keccak_f 0.46 / 0.54. ⚠️ picorv32 가 여전히 1.0× 근처인 것은 표현식이 아니라 **스케줄러가 지배**하기 때문이고, 그것이 S3 이후의 표적이다.

#### 4.5.311 ③층 S3a — 호출 흡수, 그리고 한 결함 클래스를 leaf 에서 primitive 까지 (2026-08-07, branch feat-s3a-frame-call-absorption, format 26 불변) ✅

**한 줄** — `NetArena::buildable` 의 blanket `func_table` 거부를 **측정된 부분집합**으로 좁히고 `NativeKernel` 을 **복합 `NetReader`** 로 만들어 호출을 엔진 프레임 실행기에 **위임**했다(재진술 0) → `bench/keccak` **호출형·배열형이 네이티브로 실행되고 VM 과 바이트 동일**. ⭐⭐ 그리고 적대 리뷰 **6라운드**가 곁가지 pre-existing 결함 하나를 **leaf 에서 primitive 까지** 몰아냈다.

**본체(S3a)**

- `native::frames::frames_admitted` = admission. byte-identity 논증은 **"본문이 자기 프레임 창 밖 넷을 안 부른다"** — 그러면 `run_frame_call` 이 모듈 스토어를 아예 안 만지므로 위임이 구조적으로 옳다. 거부 행마다 자기 이름(task·모듈 넷 참조·systask 인자 호출·delayed CA 호출·모듈 바디가 프레임 넷 참조·malformed 사이드카 3종).
- `impl NetReader for NativeKernel` — 모듈 넷은 아레나, 프레임 슬롯은 `SimState`, `eval_call`/`formal_width`/`formal_is_string`/`resolve_virtual_call` 은 위임. `ctx()` 가 `self` 를 넘기므로 **호출부 변경 0**.
- `NetArena::eval_call` 은 `None`(X) 대신 **panic** — seam 열거가 놓친 자리가 조용한 X 가 아니라 게이트에서 시끄럽게 죽도록. ~875 네이티브 런에서 한 번도 안 터졌다.
- ⚠️ **속도는 안 샀다**: native/vm = 1.41×(flat)·1.14×(호출형)·1.06×(배열형)·**0.83×(picorv32)**. verilator 대비 54×(flat)~722×(호출형). 호출 본문이 여전히 인터프리터 프레임 실행기에서 돈다 → **flat↔호출형 13× 가 통째로 S3(바디 코드젠)의 표적**이고, 이제 세 변종 전부에서 잴 수 있다(그것이 이 슬라이스의 측정 가치).
- ⚠️ 실측이 계획을 두 번 정정: **corpus 72 설계에 서브루틴이 0개**(전 검증이 전용 설계) · **비-`automatic` 모듈 함수도 본문에 제어 흐름이 있으면 프레임이 된다**(내 프로브가 직선 본문이라 "인라인된다" 고 오판 → static 슬랩 경로 커버리지 0 이었다).

**⭐⭐ 곁가지: 문자열→packed 변환, 6라운드에 걸친 root cause 추적**

라운드마다 **발견한 자리에서** 고쳤고, 그래서 라운드마다 다음 인스턴스가 나왔다:

| 라운드 | 고친 자리 | 다음이 찾은 것 |
|---|---|---|
| 1 | (본체) `eval_call` 이 지연 범위보고를 안 비워 E4002 가 콜리 출력 뒤로 | — |
| 2 | intercept(`frame_rhs_value`)가 목적지 net kind 로 게이트 | 폭이 안 맞음 |
| 3 | write funnel(`frame_or_class_write`) | equal-width + **부호** |
| 4 | 명시적 `is_str` clear | 두 번째 레인(lifted task) |
| — | `frame_write_lvalue` | (자체 감사) 인자 바인딩 |
| 5 | 공유 `bind_formal` | **모듈 레인**(class field·dyn·assoc) |
| 6 | **`Value::resize` 의 equal-width early return** | 제품 결함 0 |

- 뿌리: `Value::resize` 가 폭이 바뀌는 두 경로에서만 `is_str` 를 지우고 **early return 에서 안 지웠다**. 그 한 칸이 ⓐ bare `%s` 가 raw 바이트 ⓑ 하류 `resize_keep_sign` 이 같은 플래그에서 short-circuit 해 **부호 미각인** 을 만들었다(`integer p = s`, `s==32'hf0f1f2f3` → 프레임 4042388211 vs 모듈·iverilog −252579085).
- ⭐⭐ **내가 "등가 뮤테이션"이라 문서화한 것이 사실은 수정이었다** — 라운드 2에서 그 뮤테이션이 생존한 건 중복이 아니라 **결함의 증거**였고 정확히 반대로 읽었다. 두 렌즈가 독립으로 잡았다.
- ⭐ **보정 clear 를 소비자에 두면 서로를 가린다** — 셋 다 개별 뮤테이션으로 안 죽었다(실측). 규칙을 primitive 한 곳에 두고 소비자 보정을 제거하니 전부 kill.
- 오라클: iverilog 는 packed `$sformatf` 목적지를 거부하지만 **같은 §6.16 규칙의 합법 철자**(`reg [15:0] p = {"ab","cde"}` → `de`/25701)는 받는다 → **외부 앵커 확보**("오라클 없음"이라던 내 헤더도 정정).
- 부수 수정: `%c`·UTF-8, `loc[i:0]`, static 함수 로컬 E3010, 프레임 배열 OOB 무진단 → **ROADMAP §2/§3 기록**.

**게이트** — 5214 tests green(+31) · clippy/fmt clean · staged==one-shot · PRE↔POST **172설계 ×2모드에서 차이 46 전부 문자열 설계**(문자열 없는 diff 0) · 뮤테이션 **누적 30+ 중 kill 24·생존 6 전부 도달불가/등가 사유 실측 기록** · 차분 렌즈 누적 **2000+ 설계 0 발산** · 모듈 레인 **124설계 ×3백엔드 0 변화**.

**⚠️ 이번 반복의 진짜 교훈은 스코프다** — 본체는 라운드 1에 CLEAN 이었고 2~6은 곁가지였다. **pre-existing 이 서로 다른 경로에 2건 이상이면 인스턴스가 아니라 CLASS 이고, 그때는 공유 primitive 를 먼저 찾거나 슬라이스를 분리해야 한다**(LOOPROMPT §4·§8 에 반영). 리뷰 대기가 반복 시간의 ~30%(2.5h/9h)였다.

#### 4.5.310 배열-워드 인덱스: 두 번째 오라클이 "vita 가 앞서 있다" 를 반증했다 (2026-08-06, branch feat-array-word-index, format 26 불변) ✅

**한 줄** — unpacked 배열의 워드 인덱스를 **자기결정 식으로 평가한 뒤 32비트 정수로 읽도록** 고쳤다. 퍼널은 하나(`seal_index_unsigned`), 원인은 셋이었고, **verilator 5.050 을 두 번째 오라클로 들인 것이 결정을 뒤집었다**.

**그라운딩이 결정을 바꿨다**

- §4.5.308 은 이 축을 *"iverilog 는 런타임 배열 워드만 i32 로 재해석한다 · IEEE §5.2.1 에 재해석 단계가 없으므로 vita 는 값 의미로 답한다 = **vita-ahead**"* 로 기록하고 테스트로 고정했다. 이번엔 같은 설계를 **verilator 로도** 쟀다 — verilator 도 iverilog 와 같은 칸에 놓는다. **vita 는 앞선 게 아니라 혼자였다.**
- 36칸 3-오라클 매트릭스(선언범위 4 × 인덱스형 9): 두 오라클이 **일치하는데 vita 만 다른 칸이 10**, 두 오라클이 **갈리는 칸이 22** 이고 그 22 에서는 vita 가 iverilog 와 일치한다. ⭐ **verilator 는 범위 밖에서 오라클이 아니다** — 인덱스를 2의 거듭제곱으로 패딩한 배열에 **마스킹**해 아무 값이나 낸다(`m[0:5]`, `u8=250` → `250 & 7 = 2`). 그 22칸이 전부 그 형태다.
- 10칸의 원인은 셋: ⓐ **자기결정 미평가**(`m[~s8]` 가 `~` 를 넓힌 피연산자에서 계산해 `0xFFFF_FF05`) ⓑ **봉인이 자기폭+1 로 넓혀** 정규화가 33비트가 되고 음수 base 의 **랩이 사라진다**(`0xFFFF_FFFD + 3` 이 32비트에선 0 = `[-3]`, 33비트에선 4294967296 = 드롭) ⓒ **32비트 초과 인덱스 미절단**(`64'h1_0000_0002` 가 아무 데도 안 닿는다).

**구현** — 퍼널 한 함수: placeholder 면 거절(유지) · **상수면 옛 경로**(아래) · 32 초과면 `select_low(e,32)` · 정확히 32 면 통과 · 미만이면 `extend_to(e, w, 32, signed)`. 결과: 매트릭스 **10/10 착지**, 22칸은 그대로 iverilog 일치.

**⭐⭐ 두 번 틀렸고 두 번 다 측정이 잡았다**

- ⭐⭐ 첫 시도는 부호 인덱스를 `$signed(x)` 로만 감쌌다 — 폭은 고정되지만 **무부호 32비트 문맥이 0확장**해서 `m[s8 >>> 1]`(−3)이 **253** 이 되어 범위를 벗어났다. **PRE 에서 맞던 칸이 틀려졌다**(자체 스윕이 `WORSE=[c0135]` 로 즉시 표시). 확장은 폭과 **부호가 한 쌍**이고, 그 쌍을 이미 가진 헬퍼(`extend_to`)가 파일 안에 있었다.
- ⭐⭐ 그리고 **상수 인덱스가 조용해졌다**: `mg[$unsigned(-32'sd3)]` 이 PRE 에서 exit 1 + 진단이었는데 랩해서 **exit 0** 으로 썼다(= loud→silent, 사다리 하강). 여기서 두 오라클이 갈린다(iverilog 드롭+경고 / verilator 착지) — **다수결이 아니라 사다리가 정한다**: 컴파일러가 아는 인덱스가 컴파일러가 아는 범위 밖이면 그건 말해야 한다. → 상수는 옛 경로로 carve-out. ⚠️ 그 술어는 **의도적으로 얕다**(`Const`·`$signed`/`$unsigned`·단항 부호) — 못 알아본 상수식은 런타임으로 분류돼 랩하므로 값은 verilator 와 같고 **잃는 것은 진단뿐**(ROADMAP §2).

**검증** — 3-오라클 36칸 **10/10 fix · 회귀 0** · 자체 168 스윕 **FIXED 3 · WORSE 0** · 리뷰어 매트릭스 s7 **FIXED 38 · 회귀 0**, s2/s5 **무변화 · 회귀 0** · `examples/` 4 + keccak + picorv32 **PRE/POST 바이트 동일** · 신규 테스트 4(전부 두 오라클로 기대값 검증).

**⭐⭐ 적대 리뷰가 내 수정에서 silent-wrong 을 셋 더 잡았다 — 전부 이번 슬라이스가 만든 것**

- ⭐⭐ **절단이 x/z 를 버렸다**: `select_low` 는 비트 연산이라 31비트 위의 x/z 가 사라져, `reg [63:0] b; b[31:0]=3;`(상위 미구동=X)이 원소 3 을 **읽고 썼다**(iverilog 는 `x`+드롭) = loud→silent-wrong. 값 매트릭스 넷 중 **어느 것도 인덱스에 x 를 넣지 않아** 안 보였다. 수정 = 버린 절반을 `high * 0` 으로 되더한다(known 이면 0·x 면 전부 X — 두 시뮬레이터에서 동일 실측). 6행(z·저위 x·33비트·65비트·1비트 상위부·컨트롤) 전부 iverilog 일치.
- ⭐⭐ **상수 인식이 철자 기반이라 한 설계 안에서 두 철자가 갈렸다**: `m[64'h1_0000_0002]` 는 드롭(loud·오라클 일치)인데 **같은 값**인 `m[~64'hFFFF_FFFE_FFFF_FFFD]` 는 조용히 원소 2 에 썼다. 얕은 술어(`Const`·부호캐스트·리터럴 부호)가 나머지를 런타임으로 분류했기 때문. 수정 = **철자를 세는 대신 식을 걷는다**(`_`-free 화이트리스트·Const 리프+순수 연산자).
- ⭐⭐ **부호 확장이 인덱스를 두 번 평가했다**: `extend_to` 는 `Concat[Replicate(e[w-1]), e]` 라 `e` 가 **두 번** 놓이고, `m[byte'($urandom)]` 이 한 뽑기의 부호비트와 **다른 뽑기의** 하위비트로 인덱스를 만들었다(값이 틀리고 스트림도 밀린다). ⭐ **문맥에 맡기는 우회는 실패했다** — §5.5.1 부호 결정은 **아래로 전파**돼서 무부호 좌표 산술이 `$signed(e) * 32'sd1` 까지 무부호로 만든다(실측: 262≠6). 그래서 **복제가 관측 불가능할 때만** 확장한다(`index_is_repeatable`, `_`-free). ⭐ 그 술어의 SysFunc 팔은 정본 `sysfunc_is_stmt_effect` 를 **부르되**, 그 술어는 다른 질문에 답하므로(bare `$urandom` 을 순수로 본다) **뽑기 집합만 델타로** 명시한다 — 처음엔 SysFunc 전부를 닫았다가 `$time` 인덱스 4칸을 잃었다(측정이 되돌렸다).

**⭐ 그리고 리뷰가 내 테스트의 공허성 셋을 실측**: `S` 행이 **base 0 배열**이라 좌표 산술이 아예 없어 PRE 에서도 통과(→ `[2:5]` 로 이동) · 부호확장 테스트가 **PRE 에서도 통과**(→ PRE 가 틀리는 행 추가) · `one_bit_pattern_three_oracle_answers_vita_answers_by_value` 라는 **이름이 거짓**이 됐다(이제 세 경로 전부 iverilog 핀 → 개명).

**⭐⭐ 라운드 2 가 회귀 하나와 눈먼 축 셋을 더 잡았다**

- ⭐⭐ **퍼널이 packed 도 서비스한다** — `flatten_word` 의 주석이 *"배열 WORD 인덱스와 multi-dim-packed 말단 원소 인덱스가 둘 다 여기로 온다"* 라고 적어 두었는데 내 절단이 packed 오프셋까지 32비트로 옮겼다: `reg [3:0][7:0] p; p[64'h1_0000_0002] = 8'hA5` 가 iverilog 는 드롭인데 **원소 2 에 조용히 썼다**(24칸 중 12). 읽기는 고치고 쓰기를 깨는 형태라 값 매트릭스가 못 봤다. → 호출부가 **`IndexDomain` 을 명시**(프록시 `ascending.is_empty()` 대신)하고 packed 은 PRE 구성 그대로.
- ⭐ **복제 게이트가 wide 경로엔 필요 없었다** — `select_low(e,32) + (e[hi]*0)` 대신 **`select_low(e * 1, 32)`** 면 x 전파가 같고 `e` 를 한 번만 놓는다(두 시뮬레이터 실측). 게이트가 사라지자 `ma[fw(0)]`(순수 함수가 64비트 반환)이 loud→정답으로 함께 열렸다 — 같은 값을 넷으로 주면 되는데 함수면 안 되던 **두 철자 불일치**.
- ⭐⭐ **복제는 값 말고 진단 채널로도 관측된다** — `warn_run_range` 는 Error 이고 **8/run 로 제한**이라, 복제된 `m[ix[k]]` 가 예산을 먹어 **무관한 사이트의 진단이 사라졌다**(PRE errors=7 → POST 9). → 배열-원소 읽기는 복제 불가로. 그리고 SysFunc 팔이 *"정본 술어 − 뽑기"* 라 **fail-OPEN** 이었다(새 뽑기 id 는 누락으로 통과) → **긍정 화이트리스트**로 뒤집었다.
- ⭐ **placeholder 거절이 서브트리가 아니라 아레나 프리픽스** 였다(§4.5.309 가 그렇게 지었다) → `$display(u.k)` 를 한 줄 앞뒤로 옮기면 같은 인덱스가 loud/정답으로 갈렸다. 거절 판정은 **서브트리 walk** 로, 메모 건전성만 프리픽스로 남겼다.

**⭐⭐ 라운드 3 이 회귀 둘을 더 잡았다 — 도메인 라벨과 성능**

- ⭐⭐ **도메인 라벨이 저장 방식을 가리키고 있었다**: 서브루틴 지역/포멀 unpacked 배열은 `packed_dims` 에 등록되므로 그 워드 인덱스가 `lower_packed_read/write` 로 오는데, 거기서 `PackedElem` 이라 붙였다 → 함수 안 `lm[bg]` 가 모듈 쌍둥이 `gm[bg]` 와 다른 답을 냈고, **PRE 는 exit 1 이던 것이 exit 0** 이 됐다(loud→silent). 판별자는 이미 두 함수가 몇 줄 위에서 부르고 있었다(`frame_arr_formal_meta`).
- ⭐⭐ **새 성능 회귀 80×**: 프리픽스에 placeholder 가 있으면 인덱스마다 `0..=eid` 를 통째로 재계산했다(계층 읽기 한 줄 + 인덱스 12000 = 0.23 s → **34.5 s**). `examples/`·keccak·picorv32 에 계층 참조가 없어 아무 게이트도 못 봤다. 수정 = **서브트리만** 채운다(스크래치 재사용·같은 규칙·같은 드라이버) → 12000 에서 PRE 0.227 / POST 0.233.
- ⭐ **거짓 주석 셋**: 내 편집이 doc 블록 둘을 **다른 함수 위로** 옮겼고(`TWICE?` 가 placeholder walk 위로), 좁은 경로 주석이 *"문맥에 맡긴다"* 라고 **코드와 반대로** 적혀 있었다(그 우회는 측정으로 기각된 쪽이다). 전부 실제 코드대로 정정.
- ⚠️ 라운드 3 은 리뷰 중 트리가 안 바뀐 첫 라운드였고(§8 로 이관한 규칙), 그래서 처음으로 뮤테이션 6종을 완주했다 — 생존 2건(배열-원소 복제 게이트·서브트리 거절)이 곧 위 §2 기록이다.

**⭐⭐ 라운드 4: 내 성능 수정이 silent-wrong 을 하나 만들었다**

- ⭐⭐ **예산은 절벽이다** — 서브트리 워크에 넣은 `out.len() > 4096` 탈출이 서브트리를 잘라, 재사용 스크래치의 **stale 슬롯**(`{1, unsigned}`)을 자식 값으로 읽었다. 4096-리프 조건식 아래 부호 인덱스가 0확장 팔로 가서 `-3` 이 253 이 되고 **쓰기가 실제 원소에 착지(exit 0)** — 그것도 **무관한 계층 읽기 한 줄이 앞에 있을 때만**(그 줄이 이 경로를 켠다). 경계 스윕이 정확히 4096 에서 뒤집힘을 고정했다. 수정 = ⓐ 워크를 **fail-closed** 로(못 끝내면 봉인 거절) ⓑ 예산 자체를 **방문 스탬프**로 대체(예산이 있던 이유는 공유 부분식의 지수 폭발이고, 스탬프는 절벽 없이 그것을 없앤다) → 네 조합 전부 오라클 일치·성능 유지.
- ⭐⭐ **뮤테이션 생존 둘이 곧 "테스트 0"** 이었다: 프레임 호출부를 `PackedElem` 으로 되돌리는 뮤테이션이 **5183 전부 통과**(직전 커밋이 통째로 무테스트)하고, 서브트리 워크를 `[eid]` 하나로 줄이는 뮤테이션도 통과(provisional 경로 전체가 무테스트) → 두 설계를 테스트로 신설.
- ⭐ 리뷰가 **CLEAN 카테고리도 실측**했다: `frame_arr_formal_meta` 는 기하 판별자로 옳고(분류기가 packed 리스트 비었을 때만 등록·9 저장 클래스 스윕 전부 오라클 일치), 280칸 순서-의존 스윕은 **POST 0 발산**(PRE 20).

**⭐⭐ 라운드 10: 라운드 9 가 적은 두 기록이 반증됐다 — 그리고 내 주석의 대표 예시가 다른 퍼널에서 여전히 깨져 있다**

- ⭐⭐ **"등가" 라고 적은 dedup 이 load-bearing 이었다**: `index_is_repeatable` 의 스탬프를 빼면 5191 전부 통과하지만 **753 바이트** 설계가 `A 42`(두 오라클) → `A x` 로 뒤집힌다. ⭐ 내가 적은 **이유도 틀렸다** — DAG 는 기하가 아니라 **봉인 자신의 `extend_to`**(피연산자 2회)가 중첩 packed 원소 select 를 통해 워크에 재진입하며 만든다(레벨당 ≈6×). "도달 설계를 못 지었다" 는 **못 지은 것이지 없는 것이 아니다** → 테스트 신설. (`index_all_const` 는 진짜 등가지만 그 이유도 "입력이 트리라서" 가 아니라 **예산 소진과 `Signal` 리프가 같은 `false`** 를 쓰기 때문이다.)
- ⭐⭐ **§2 범위 문장이 반대 방향으로 또 틀렸다**(같은 문장 세 번째 수정): *"placeholder 를 담은 인덱스는 전부 거절"* 이라 썼는데 **계층 select 는 거절하지 않는다** — 그건 바로 라운드 9 가 심은 테스트의 형태다. 8칸 매트릭스로 실측한 축은 **바깥 select 가 계층인가**(계층 lvalue/rvalue 는 해상 시점에 다시 묻는다)이고, **쓰기 쌍둥이는 또 다르다**(착지하는데 읽기는 `x`).
- ⭐⭐ **pre-existing silent-wrong**: `byte'($urandom)` 이 **8번** 뽑는다(`coerce_two_state` 가 결과 비트마다 피연산자를 놓는다·iverilog 1번·값도 다르다). 하필 그 식이 **내 봉인 주석이 복제 가드의 대표 예시로 세 번 인용**하던 것이라, 주석이 코드에 없는 성질을 주장하고 있었다 → 주석 정정 + §2 기록.
- ⭐ 그리고 라운드 9 의 테스트는 teeth 가 **`u.k` 의 위치**에 달려 있다(뒤로 옮기면 되돌림이 다시 통과) → 주석에 명시.

**⭐⭐ 라운드 9: 내가 라운드 8 에 심은 테스트가 그 경로에 닿지 않았다**

- ⭐⭐ **수정을 되돌려도 5191 전부 통과**했다 — 테스트 설계가 계층 **select** 뿐이라 그것을 해상하는 패스가 곧 바깥 인덱스를 정규화하는 패스이고, 그래서 프리픽스가 이미 깨끗해 `index_has_placeholder` 자체가 **안 돌았다**. 필요한 건 **다음 패스가 해상하는 참조**(whole-net `u.k`) 한 줄이었다. 그 줄을 넣으니 되돌림이 즉시 죽는다. 교훈: **테스트가 도는 것과 테스트가 그 코드를 태우는 것은 다르다** — 새 가드를 심으면 **되돌린 바이너리로 한 번 돌려** 확인하라.
- ⭐ 그리고 그 커밋의 *"네 워크 어디서 dedup 을 빼도 통과"* 도 정확하지 않았다: 실측 **둘은 죽고 둘은 산다**. 산 둘(`index_all_const`·`index_is_repeatable`)은 기하가 인덱스를 복제하기 **전**의 원본을 걷고 `push_expr` 은 dedup 을 안 하므로 오늘 그 입력은 **트리**다 — 도달 설계를 못 지었고, **teeth 가 아니라 등가로 기록**했다(소스로 DAG 를 만들려 하면 20 MB 가 된다).
- ⭐⭐ **그리고 §2 에 내가 쓴 문장이 반증됐다**: *"계층 신호·localparam 은 해상 뒤에 다시 물어보는 경로가 있어 이미 정확하다"* → `mg[~u.a]` 는 `x`+E4002 인데 로컬 쌍둥이와 두 오라클은 42 다. 그 주장을 뒷받침한다던 핀은 32비트 `integer` 에 **bare** `mg[u.k]` 라 봉인이 필요 없는 형태였다 — **거절을 검증했지 봉인을 검증하지 않았다**. 실제 범위는 "정규화 시점에 placeholder 인 계층 참조를 담은 인덱스 **전부**"(36칸 중 18·240 퍼즈 중 31·전부 PRE==POST)로 정정.

**⭐⭐ 라운드 8: dedup 을 넣으면서 버퍼를 인덱스 루트 크기로 잡았다 — 아레나에는 역방향 간선이 있다**

- ⭐⭐ **back-edge**: 지연-계층 해상이 resolve 로 지은 노드를 **낮은 placeholder 슬롯에 복제 설치**하므로 그 노드의 자식 id 가 자기보다 **위**다. 세 워크가 그 간선을 따라가는데 스탬프 버퍼가 루트 크기라 `seen.get_mut()==None` → fail-closed → **모든 봉인 거절**. `u.p[~u.a[1]]` 이 exit 0 에 `xx`, 같은 설계 한 줄 옆의 로컬 쌍둥이는 `bb`(두 오라클 `bb`) — 그리고 **`+` 하나를 옮기면 뒤집힌다**(§4.5.309 가 없앴다던 순서 의존). 이미 스탬프를 갖고 있던 형제는 그 간선을 **필터**하는데(주석에 클론 설치라고 적혀 있다) 새 사용자 셋은 안 했다. 수정 = 버퍼를 **아레나 전체** 크기로(간선을 필터하는 건 틀리다 — 클론 서브트리는 placeholder 를 **합법적으로** 가질 수 있고 그게 이 워크의 존재 이유다).
- ⭐⭐ **라운드 7 의 dedup 수정은 teeth 0** — 네 워크에서 각각 dedup 을 빼도 5189 전부 통과(고친 DAG 폭발 설계를 테스트로 안 옮겼다). 그리고 **내가 그 라운드에 심은 진단 테스트도 공허**했다: 채널이 **진단 개수**인데 exit code 만 단언했고 `run()` 은 stdout 만 돌려준다(실측 3 vs 가드 제거 시 6) → `error_count` 헬퍼 신설.

**⭐⭐ 라운드 7: 내가 커밋 메시지에 쓴 *"there is no cap to exhaust"* 가 거짓이었다**

- ⭐⭐ **캡을 없앤 게 아니라 이름을 바꿨다** — 재귀 깊이 캡을 지우면서 그 자리에 **100만 노드 예산**을 넷 다 넣었고, 그중 `index_has_placeholder` 의 것이 **실제로 닿는다**: `m1[X][X]` **7단계**(소스 1.6 kB·elaboration 0.44 s)에서 쓰기가 조용히 사라진다(exit 0·errors=0). 뿌리는 숫자가 아니라 **dedup 부재** — 아레나는 DAG 이고 기하가 인덱스를 세 번 이름 붙이므로 워크가 **O(경로)** 다. 수정 = 이미 있던 방문 스탬프를 **세 워크 전부**에 태운다(`with_seen`) → 7·9·11단계 전부 오라클 일치. ⚠️ 라운드 6 의 새 깊이 테스트 셋은 **선형 패드 체인**(≤200)이라 100만에 세 자릿수 못 미쳐 구조적으로 못 본다.
- ⭐ **packed 분기의 복제 게이트가 teeth 0** — 11 뮤테이션 중 유일 생존. 두 채널이 살아 있다(뽑기 스트림·진단 카운트 3→6) → 판별 설계 둘 신설.
- ⭐⭐ **carve-out 의 근거 주석이 근거가 아니었다** — packed 이 32비트 절단을 안 받는 이유로 `p[64'h1_0000_0002]` 를 인용했는데 그건 **리터럴**이고 iverilog 는 그걸 **컴파일 에러**로 막는다. 런타임 값으로 주면 두 오라클 **모두 절단**하고 vita 만 `x` 다(네 철자·840케이스 차분의 **유일한** 잔여) → 주석을 실측대로 고치고 §2 기록.
- ⚠️ 그리고 *"elaboration 이 PRE 와 동률"* 도 과했다(리뷰어 실측 +12~33%). dedup 후 재측정 **+2~6%**.

**⭐⭐ 라운드 6: 내 라운드-5 수정이 no-op 이었다 — 네 개 중 하나만 고쳤다**

- ⭐⭐ **`grep 'depth > 64'` 를 안 했다.** 라운드 5 는 캡을 **네 개 중 하나**(`index_has_placeholder`)에서만 없앴고, **같은 커밋의 프리픽스 스킵이 그 함수를 계층 없는 설계에서 도달 불가로** 만들어 결과적으로 **수정이 관측 불가**였다(뮤테이션: 캡을 되돌려도 5187 전부 통과·전 프로브 바이트 동일). 절벽은 사라진 게 아니라 **형제로 옮겨갔다** — 그리고 그 형제의 주석은 *"`index_has_placeholder` 가 같은 깊이에서 캡한다"* 를 근거로 자기를 등가라고 주장하는데, **그 문장이 가리키는 캡을 같은 커밋이 지웠다**.
- ⭐⭐ 살아 있던 세 캡이 전부 실제 발화: ⓐ `collect_subtree_postorder` — 계층 참조가 앞에 있으면 64 패드에서 **조용히** 봉인 소실(중첩 2-D 배열 읽기 **11개**로도 닿는다) ⓑ `index_all_const` — `false` 가 "상수 아님" 을 뜻해 **정적으로 범위 밖인 상수 쓰기가 착지**(PRE 는 모든 깊이에서 loud → **POST 가 PRE 보다 나쁘다**) ⓒ `index_is_repeatable` — 좁은 **부호** 인덱스에서 봉인 소실(라운드 5 의 새 깊이 테스트는 **무부호** 인덱스라 이 팔에 구조적으로 못 들어간다). 셋 다 캡 제거로 수정(전부 반복 구조).
- ⭐⭐ 그리고 **라운드 5 가 silent-wrong 을 맞바꿨다**: 원소 비트축을 `PackedElem` 로 보내자 그 분기가 `!signed` 만 봉인해서 **좁은 부호 인덱스가 자기결정 봉인을 못 받는다** — `L[0][~v]`(byte v)가 라운드 4 에선 맞았고 라운드 5 에서 틀렸다(두 오라클 모두 32). 수정 = packed 분기에도 **자기결정 절반만** 준다(32비트 절단은 배열-워드 규칙이므로 제외) → 네 철자(vector·module array·md-packed·frame) 전부 32 로 수렴, pre-existing 이던 md-packed 까지 닫혔다.

**⭐⭐ 라운드 5(한도 해제 후 실제 실행)가 silent-wrong 둘을 더 냈다 — 둘 다 앞선 라운드의 수정이 만든 것**

- ⭐⭐ **도메인이 넷 단위라 원소의 비트축까지 워드축이 됐다**: 프레임 배열의 extent 목록은 `unpacked dim + 원소의 비트축` 이라(`array_formal_ext_dims`) 라운드-3 의 per-net 라벨이 비트 선택까지 32비트 절단했다 — `lm[0][b]`(64비트 b)가 비트 2 에 쓰고 **같은 설계의 모듈 쌍둥이 `gm[0][b]` 는 드롭**, 두 오라클 **어느 쪽과도** 안 맞는다(exit 0). 축은 **넷이 아니라 위치의 성질** → `IndexDomain::WordsThenElem(n)`.
- ⭐⭐ **라운드 4 가 "없앴다" 고 한 절벽이 한 호출 앞에 그대로 있었다**: `index_has_placeholder` 의 `depth > 64` 가 **모든 설계의 모든 seal 에서** 돌고 fail-closed 라, 깊이 64 를 넘는 인덱스에서 §4.5.308/309/310 의 봉인이 **통째로 사라진다**(계층 참조 불필요·63 은 정답·64 부터 오답·exit 0). 파서의 128 은 **괄호** 캡이라 좌결합 체인·삼항 체인은 무제한이고, elaborate 자신의 노드도 깊이에 셈해져 **중첩 배열 읽기 31개**로도 닿는다. 수정 = 워크를 **반복 구조**로(깊이 캡 없음·작업 상한만 10^6) + **프리픽스가 깨끗하면 워크를 건너뛴다**(그 워크가 계층 없는 설계에서도 인덱스마다 돌아 +6% 였다 → PRE 와 동률 복귀).
- ⚠️ 라운드 4 의 *"collect 의 캡은 도달 불가"* 논증은 **옳았고**(M3 생존), **바로 그것이** 형제 캡을 유일한 발화점으로 만든다 — 등가 논증을 적을 때 **그 논증이 다른 곳을 가리키는지** 확인해야 한다.
- ⭐ 그리고 리뷰가 내 주석 하나를 반증: 스탬프 테스트가 "공유 부분식 2^d 경로" 라 부르는 설계는 `push_expr` 이 dedup 을 안 하므로 실은 **8191 노드 트리**다(소스가 `i8` 를 4096번 적는다). 스탬프의 근거는 측정되지 않았다.

**⚠️ 라운드 5 는 처음엔 돌지 못했다 — 주간 사용 한도**. 그 항목들을 **자가 점검**으로 대신했고(적대 리뷰가 아님을 기록한다): 공유 부분식 3형(`(x+x)` 타워·동일 arm 삼항·반복 concat)과 **깊이 80** 인덱스를 계층 참조 유무로 각각 돌려 **순서 의존 0·오라클 일치**, 뮤테이션 4종 중 **M2(프레임 라벨 되돌리기)·M3(워크를 한 노드로) kill**, M4(깊이 fail-open)는 **등가**(`index_has_placeholder` 가 같은 캡으로 먼저 거절·자식 집합이 상위집합) → 논거를 코드에 기록, M5(스탬프 제거)는 컴파일 불가·스탬프는 성능 성질. **적대 라운드 5 는 이 슬라이스의 부채로 남는다**(다음 반복 착수 전 실행).

**⭐ ③층 게이트가 옛 입장을 두 군데 인코딩하고 있었다** — `s2_xz_index_is_dropped_matching_the_oracle` 의 행 F 가 *"iverilog 는 절단하지만 §5.2.1 엔 그 단계가 없으니 vita 가 앞선다"* 라는 **하드코딩된 hand-IEEE 핀**이었다(두 오라클 실측으로 갱신·teeth 논증은 그대로 유효). 그리고 커버리지 핀 넷이 **올라갔다**(퍼널이 Select/Concat/Replicate/Mul 을 만들어 더 많은 표현식을 태운다) — 그중 `probe B` 는 §4.5.308 이 **식을 다시 세지 않고 숫자만 바꿔** 주석의 산식(1782)과 단언(2862)이 어긋나 있었다 → 산식을 다시 유도해 적었다.

**⭐ 부수: 잔여 항목 하나가 같은 뿌리였다** — §4.5.309 가 남긴 "unpacked 부호 인덱스"(0c)는 별도 기하 슬라이스가 아니라 **같은 퍼널의 ⓐ** 였다.

#### 4.5.309 정본 폭/부호 규칙을 sim-ir 로 공유 — 봉인이 부호를 알게 되자, 아직 확정 안 된 답까지 믿었다 (2026-08-06, branch feat-share-selfsign, format 26 불변) ✅

**한 줄** — IEEE 1364-2005 §5.4.1/§5.5 자기결정 폭·부호 규칙을 `sim-engine/src/width.rs` 에서 **`sim-ir::selfwidth` 로 옮겨** 두 크레이트가 한 철자를 쓰게 하고, 그 규칙 위에서 인덱스 봉인이 **부호를 보존**하게 했다. §4.5.308 이 남긴 잔여 5분류 중 **넷이 닫히고**, 부수로 pre-existing 클래스-필드 폭 전파 silent-wrong 을 고쳤다.

**무엇이 왜 움직였나**

- **규칙 이동**(`crates/sim-ir/src/selfwidth.rs` 신설 · `width.rs` 564→101줄). `elaborate` 가 정규화하려는 인덱스의 부호를 알아야 하는데, 정본은 `sim-engine` 안에 있었고 `elaborate` 는 **손으로 쓴 보수 부분집합**(`expr_provably_unsigned`)을 들고 있었다. 그 술어의 세 팔이 측정상 틀렸다(`**` 는 밑의 부호만·클래스 필드 부호는 사이드카·real 리터럴은 signed). 엔진 드라이버는 `WidthTable::build` 가 규칙을 돌리는 얇은 껍데기로 남았다.
- **부호 보존 봉인**. `$signed({1'b0, idx})`(무부호) / `$signed(idx)`(부호) — concat 은 self-determined 라 폭을 고정하지만 **부호를 지운다**. 그래서 §4.5.308 은 부호 있는 인덱스를 아예 **거부**했고 그게 잔여 넷의 공통 전제였다.
- **닫힌 잔여 4분류**(3-way, 전부 iverilog 일치): ⓐ packed 반쪽(`byte`/`shortint`/`enum signed`) · ⓑ 32비트 부호 인덱스가 비-0 LSB 아래 · ⓒ 음수·상수산술 리터럴 · ⓓ 부호 있는 함수 반환. 남은 것은 ⓐ의 **unpacked 반쪽** 하나(`reg [7:0] ma[2:5]; ma[~s8]`) — **loud(E4002)** 라 사다리 위쪽.
- **부수 수정: 클래스 필드에 연산자가 붙으면 폭·부호가 32비트 핸들 넷으로 되돌아갔다**(pre-existing silent-wrong). `patch_class_fields` 가 **완성된** 표를 훑는 사후 스윕이라 리프만 고치고 그 위 부모는 이미 핸들로 계산돼 있었다 → `~c.sb`(byte −6)가 5 대신 **4294967045**, `c.sb + 0` 이 −6 대신 250, `(~c.si) < 0` 이 거짓. 수정 = 사이드카를 전방 패스 **안에서** 적용(`build_with`). 세 백엔드 동일·`examples/` 4 + `bench/keccak` PRE/POST 바이트 동일.

**⭐⭐ 적대 리뷰가 내가 넣은 결함 둘을 잡았다 — 둘 다 "아직 확정 안 된 답"**

- **⭐⭐ 계층 참조는 봉인 시점에 placeholder 다.** `u.k`/`u.P`/`u.f(-1)` 는 `Signal{net: POISON_NET}`·`Call{func: POISON_FID}` 로 낮춰지고 모든 인스턴스가 생긴 뒤에 **제자리 패치**된다. 정본 규칙에 그걸 물으면 없는 넷을 읽고 **1비트 무부호로 폴백**하는데, 봉인은 그걸 "무부호"로 읽어 곧 −1 이 될 인덱스를 0확장했다 → `mg[u.k]`(`reg [7:0] mg[-3:2]`)가 오라클 `aa` → **`x`+E4002+exit 1**(correct-support → loud-wrong, **사다리 하강**). **"모른다" 와 "무부호" 는 다른 답이다** → 질의를 `Option` 으로.
- **⭐⭐ 그리고 나는 그 답을 캐시했다.** 메모는 무효화가 없어서, placeholder 가 패치된 뒤에도 옛 답을 냈다 — **파일 끝에 무관한 세 줄을 더하면 그 위 쓰기의 결과가 조용히 바뀐다**(`A 00000001` → `00000000`, exit 0 양쪽). 수정 = 질의 시각에 `0..=eid` 를 훑어 placeholder 가 있으면 **캐시하지 않고 `None`**. 그 스캔 뒤엔 접두부가 전부 해상돼 있고 해상된 노드는 다시 안 바뀌므로 캐시가 **정확히** 유효하다. 내가 코드에 적은 *"expr 의 자기폭은 push 되면 안 바뀐다"* 가 반증된 것이다(제자리 패치 5곳).

**⭐⭐ 그리고 differential 이 `WORSE = []` 를 반증했다 — 내 스윕에 열이 없던 축**

- 정본 규칙이 `$stime`/`$urandom`/`$urandom_range` 를 **무부호로 증명**하기 시작하자, 옛 술어가 못 증명해서 **안 봉인하던** 그 인덱스들이 봉인됐다. 음수 base 에서 봉인 안 한 형태는 `raw + |k|` 를 **32비트로 랩**하는데 **그 랩이 iverilog 의 답**이다(`reg [7:0] ma[-3:2]` 를 `$stime + ii` 로 읽으면 `ma[-3]`). 봉인은 33비트라 랩이 없다 → **unpacked 12칸 loud 회귀 + packed 1칸 조용한 회귀**.
- ⭐ 뿌리는 이 슬라이스가 아니다 — **iverilog 는 `$stime + ii` 를 부호로 읽고**(`-3`) vita 는 IEEE 대로 무부호(`4294967293`)로 읽는다(PRE·POST 동일). PRE 가 맞았던 건 랩 덕분이다.
- ⭐⭐ **두 번 잘못 좁혔다**: 먼저 "음수 base 면 무부호 봉인 물러남" → s7 회귀 12→0 이고 **PRE 도 틀렸던 34칸까지 고쳤지만** packed 에서 5+6 칸이 깨졌다. 다음 "인덱스 자기폭 <32 일 때만 봉인" → 3칸 남았다. **폭·base 로는 두 무리가 안 갈린다** — `reg [31:0] ix` 와 `$stime` 은 같은 음수 base 아래 같은 32비트 무부호인데, PRE 는 앞은 **봉인해야** 맞고 뒤는 **안 봉인해야** 맞다. 결론: **무부호 절반은 봉인 결정 자체가 부호 문제가 아니다** — ROADMAP §2 에 이미 열려 있는 **배열-워드 i32 재해석** 문제다. → 무부호 절반을 **PRE 결정에 동결**(옛 술어를 `unsigned_seal_admitted` 로 한 가지 일만 하게 남김) → 네 매트릭스(s7 192·s2 416·s5 336·자체 168) **회귀 0 · FIXED 18+68+6**.

**⭐ 게이트가 공허했던 곳 셋(전부 리뷰 실측)**

- 하네스 갭 4번째: `build_with_opts` 가 `class_field_widths` 를 안 실어서 엔진 `patch_class_fields` 가 **빈 맵**으로 돌고 있었다 — **anti-vacuity 카운터가 잡았다**(실패가 아니라 "비교한 클래스 필드 0").
- `decl_range_norm` 아홉 테스트가 전부 `reg [-2:-33]`(음수 LSB)라 옛/새 형태가 **2³² 합동**이었다 → 봉인의 부호 절반을 되돌려도 **전부 통과**. 판별은 **오름차순 범위와 양수 비-0 LSB** 에 있다 → 6행 추가.
- `class_field_width_propagation` 이 `%0d` 뿐이라 **폭 축이 공허**(폭 32 강제해도 통과) → `%b` 로 **평범한 쌍둥이 변수와 비교**하는 테스트 추가.
- `s2_incremental_and_full_selfwidth_drivers_agree` 는 **규칙이 아니라 드라이버**를 시험한다(양쪽이 같은 함수를 부른다) → doc 을 그 사실로 정정하고 규칙의 앵커를 `pow_and_class_field_indices_keep_their_sign` 으로 명시.

**⚠️ 기록** — 계층 인덱스를 쓰는 테스트가 **저장소 전체에 0개**였다(두 결함이 전부 거기 살았다) → `cli/tests/hier_index_seal.rs` 신설. · `sim-ir::selfwidth::child()` 가 단언하는 post-order 불변식은 **이미 깨져 있다**(`hier_defer/read.rs` 가 상위 id 노드를 하위 슬롯에 복제 설치) — 오늘 패닉이 안 나는 이유는 그 두 형태가 `child()` 를 안 부르는 팔이기 때문(pre-existing·ROADMAP §2).

#### 4.5.308 선언 범위 정규화 — 무부호 32비트 낮추기가 두 갈래로 조용히 틀렸다, 그리고 내 수정이 세 번 더 틀렸다 (2026-08-05, branch feat-norm-offset-domain, format 26 불변) ✅

**뿌리 하나, 증상 둘.** `elaborate/src/packed.rs::norm_offset_for_range`/`norm_offset_for_net` 이 선언 범위 정규화(`idx − lsb` 하강 / `hi − idx` 상승)를 **무부호 32비트 `Sub`** 로 낮췄다: ⓐ 사용자 인덱스를 32비트로 **넓혀** 문맥 결정 연산자(`~`·캐리·borrow)가 자기 폭이 아니라 32비트에서 평가되고 ⓑ 비-0 선언 LSB 아래 인덱스에서 **랩**해 P0-IPU 부분 쓰기가 통째로 사라졌다. 축 전수 매트릭스(선언 5종 × 읽기/쓰기 × 인덱스 형태, 55 프로브) 그라운딩에서 **10/55 발산** → 수정 후 **55/55 iverilog 일치**.

**형태.** 인덱스를 **자기 폭에서 봉인**(`{1'b0, idx}` — concat 은 자기결정)한 뒤 정규화를 **부호 도메인**에서 한다(`$signed` + `const_s32_expr`). 상수 인덱스는 elaborate 가 접는다. **0-LSB 하강 fast path 는 손대지 않았다**(대다수 설계 바이트 동일).

**⭐⭐ 첫 수정이 사다리를 위반했고 스윕이 잡았다.** 봉인은 concat 이라 **부호를 지운다** — `~r5` 는 37→3 으로 고쳤지만 부호 있는 인덱스를 **악화**시켰다(`~k` 7→10·`k-1` 6→11). "silent-wrong 을 다른 silent-wrong 과 맞바꾸지 마라" 에 정면으로 걸려, **증명 가능하게 무부호일 때만** 봉인하는 fail-open 형태로 좁혔다 → PRE-wrong 127 → 92, **악화 0**. 그 술어의 건전성은 논증이 아니라 **크로스-크레이트 성질 테스트**(정본 `WidthTable` 의 부분집합)로 재고, **첫 실행에서 즉시 반증됐다**(`$clog2` 는 `integer` 반환이라 정본상 signed).

**⭐⭐ 같은 반복에서 unpacked 쌍둥이까지.** `reg [7:0] ma [2:5]` 의 `ma[~r5]` 가 같은 결함(iverilog 원소 3 / vita 읽기 x·쓰기 소실). 한 경로만 고치면 **divergence > uniform-wrong** 이므로 두 `flatten_word` funnel 에 봉인을 걸었다.

**게이트.** **5168 green** · clippy · fmt · format 26 · CLI 회귀 9(전부 iverilog 핀, 하나는 vita-내부 앵커) · 크로스-크레이트 성질 테스트(2019/951 핀) · 뮤테이션 **비등가 13/13 kill**.

**적대 렌즈 ① differential — 회귀 0.** 145설계 **11,285행 분류**: silent→correct **409** · unchanged-correct 10,389 · 잔여 487 · **REGRESSION 0**. ⭐ 리뷰어가 **자기 작업을 두 번 정정**했다 — 첫 unpacked 스윕이 `~r5` 를 범위 밖으로 시드해 **공허**했고(8→124), examples/keccak 의 `.velab` 이 **바이트 동일**임을 확인해 그 검사가 이 수정을 **구조적으로 검증 못 한다**를 밝혔다(→ 비공허 전체 설계 앵커 신설). ⭐ **2-D 0-based 배열도 원래 틀렸다**(가드가 `d≥2` 에만 있다)라 "0-based=회귀 축" 은 1-D 에만 참 · **picorv32 는 실제로 변경 경로를 지난다**(`regs [0:30]` 을 `~waddr` 로).

**⭐⭐ 적대 렌즈 ② soundness — 4라운드, 내 회귀 셋 + 내 거짓 주장 다수.** ① **`**`** 를 catch-all 이 "한쪽 무부호면 무부호" 로 처리(정본은 **밑수 부호만**) ② **클래스 필드**는 `Signal{핸들, word}` 라 부호가 `class_field_widths` 에 있는데 `word` 를 버리며 주석에 *"둘 다 같은 필드를 묻는다"* 라 적었다 ③ **두 funnel 중 하나만 봉인** — 패치 스크립트가 assert 로 죽어 아무것도 안 썼는데 나는 확인 없이 *"Both funnels must do it"* 이라 적었다(**내가 피하려던 divergence 를 내가 만들었다**). 셋 다 수정 후 3자 일치. ⭐ 라운드 2 가 **뮤테이션 셋 생존**을 냈다 — 음수 상수 인덱스·**봉인의 부호**(기존 테스트가 전부 상수라 봉인에 **아예 안 닿았다**)·>32비트 부호 인덱스 → 판별 설계 영구화. ⭐ 라운드 3 은 **설계 재구성**으로 셋을 한 번에 닫았다(클래스 맵을 술어 **안으로** → 래퍼와 그 미검증 재귀 소멸·무부호 필드가 봉인을 되찾음·`Real` arm 정정). ⭐⭐ 라운드 4 는 **리뷰어가 자기 반증을 철회**했다 — *"iverilog 가 beyond-i32 배열 워드를 절단한다"* 를 반증했다더니 **리터럴로 재고 런타임과 비교**한 것이었다(내가 재확인: 런타임은 `mem[0]=99`·`mem[2]=98`·`mem[1]=97` 로 절단, 리터럴은 드롭). 오라클은 자기모순이 아니라 **경로마다 한 규칙**이고, 그 잘못된 반증이 코드 주석에 들어가 있어 되돌렸다.

**⚠️ 이 슬라이스의 절차 교훈**: 파이썬 다중 편집 스크립트가 중간 assert 에서 죽어 **앞선 edit 이 전부 유실**되는 사고가 **네 번** 났고(봉인 미적용·doc 4건 ×2), 매번 "적었다" 고 믿었다. 편집마다 **독립 write + 즉시 grep 검증**으로 바꾼 직후 다섯 번째를 즉시 잡았다.

**잔여 5분류**(전부 "봉인=concat 이라 부호를 지운다" 를 공유·ROADMAP §2) + **오라클 3경로 규칙**은 CLI 층 vita-내부 앵커로 결정 고정(6행 표 포함).

#### 4.5.307 ③층 S2 슬라이스 3 — lvalue 오프셋 특수화: native 1.79 s(VM 대비 1.43×), 그리고 게이트가 자기 실패 모드를 못 봤다 (2026-08-05, branch feat-tier3-s2-offsets, format 26 불변) ✅

**프로파일이 표적을 지정했다**(호출자 귀속): `eval_ctx` 1136 샘플 중 **최대 호출자가 `FnOnce::call_once` 440** = `resolve_offsets` 안의 인덱스 평가 클로저. 대입마다 1회 도는 이 경로가 **상수 인덱스조차** 제네릭 평가기를 통과하고 있었다.

**형태.** 인덱스→비트위치 규칙을 `eval::offset_of_index_value` 로 **원문 그대로 추출**(재진술 금지)하고, 커널이 ExprId 별 `icache`(`Const` 접기 / W 프로그램 / 거절)로 앞지른다. **어느 한 인덱스라도 미admit 이면 lvalue 전체를 제네릭으로 되돌린다** — 부분 특수화가 존재할 수 없고, 그 성질이 곧 **E4002 기계 보존의 논증**이다(admit 된 인덱스 트리는 whole-net 로드와 **범위 안** 상수 원소만 담을 수 있어 `warn_run_range` 에 닿지 못한다).

**성능.** native **2.16 → 1.79 s**(슬라이스 1 시작점 4.49 대비 **2.51×**) — VM 2.56 대비 **1.43×**. ⚠️ 단 이것은 **표현식 무거운 설계**의 숫자다: picorv32 는 native 1.03 s vs vm 0.86 s = **0.83×(더 느리다)** — `--backend` 도움말이 그 경우를 *"~1.0x"* 라 적고 있어 실측값으로 정정했다.

**게이트.** **5158 green** · clippy · fmt · format 26 · 두 리졸버 **직접 차분**(코퍼스 + 전용 3설계 × 4상태·`(2156, 12)` 핀 — 거절 12건도 load-bearing) · 오라클 앵커(x/z 인덱스·행 F 는 hand-IEEE) · 뮤테이션 비등가 **12/12 kill**(등가 5 는 논증 기록).

**적대 렌즈 ① differential — 회귀 축 발산 0.** 타깃 68 + 인덱스 퍼즈 710 + keccak/examples/picorv32, 각 4-arm. **PRE-native == POST-native 전수 0 diff**(VCD 바이트 포함). ⭐ 비공허 증명이 강했다 — 리뷰어가 **shadow 바이너리**로 두 리졸버를 매 lvalue 동시 실행해 **6.02M admission·불일치 0**(picorv32 만 4.0M). ⭐⭐ 그리고 **pre-existing silent-wrong 둘**을 냈고 **한 뿌리**임을 내가 코드에서 확인했다 — `packed.rs::norm_offset_for_range` 가 선언 범위 정규화를 **무부호 32비트 `Sub`** 로 낮춘다: 비-0 LSB 하강 넷에서 LSB 아래 인덱스가 **랩해서 쓰기가 통째로 사라지고**(0-LSB 넷은 정상이라 경계가 정확히 "base < 선언 LSB"), 상승 넷에서는 **사용자 인덱스를 32비트로 넓혀** 문맥 결정 연산자(`~`·캐리)의 값을 바꾼다(읽기도 같이 틀린다). fix 형태는 기록된 규칙 그대로("변환은 leaf 가 아니라 **문맥 경계**에서") — **세 백엔드가 공유하는 elaborate 낮추기**라 드라이브-바이로 얹지 않고 메커니즘·경계·재현을 §2 에 정밀 기록하고 **다음 슬라이스 ①** 로 올렸다.

**⭐⭐ 적대 렌즈 ② soundness — 게이트가 이 슬라이스가 만든 유일한 실패 모드를 못 봤다.** 특수화가 새로 만든 위험은 **캐시 staleness** 하나인데, 내 테스트가 **상태마다 커널을 새로 만들어** icache 를 "그것을 채운 상태" 로만 조회했다 — `Prog` 결과를 첫 평가에서 `Const` 로 얼리는 순수 staleness 뮤테이션이 **이 슬라이스의 두 테스트를 모두 통과**했다(잡은 것은 상속받은 코퍼스/런 차분뿐). 커널을 **살려 둔 채** 아레나를 상태별로 흔들도록 고치니 그 뮤테이션이 **자기 게이트로** 죽는다. ⭐ 그리고 리뷰가 **주장 넷을 거짓으로 판정**: `wprog` 모듈 doc 이 *"진입점 둘"* 이라 적었는데 이 슬라이스가 **세 번째**(`index_of`)를 추가했다(그 문장은 4d-2 리뷰가 "커버리지 주장은 진입점을 대라" 며 쓰게 한 바로 그 문장이다) · `eval/mod.rs` 가 *"ROADMAP §2 가 이 노트를 싣는다"* 고 했으나 그 커밋에 문서 변경이 0 · `resolve_offsets` doc 이 *"THE offset resolver"* 이자 tier-3 가 호출자라 주장 · kind 가드의 **명시 이유가 실제 이유가 아니다**(아레나에서 `is_assoc*` 는 `NetReader` 기본값 `false` 라 **어떤 kind 로도** 그 분기가 안 열리고, `buildable` 은 **설계 전체**를 첫 비-정수 넷에서 거부한다 → 가드는 안전한 상위집합이자 **구조적으로 죽은 코드**). 전부 실측대로 고쳤다.

**CLEARED(측정)**: assoc 사이드채널 이중 도달불가 · >2 chunk `Heap` 경로 무변경(3-chunk 설계로 실측) · `c.width` 미사용 · icache 키가 구조적으로 self-width(=`EvalCtx::eval` 과 동일) · `Const` 시간불변(`ir.consts` 만 읽음·진단 무발화) · **진단 패리티 실측**(admit 된 인덱스 + OOB 쓰기/읽기 6/6 · 상수 x/z 인덱스 5회 반복 5건 — 접기가 중복제거하지 않는다) · `index_of` 루프 ≤2회 · 200-seed 인덱스 퍼즈 200/200.

**부수 기록(둘 다 pre-existing·세 백엔드 동일)**: 별칭 concat lvalue 쓰기 순서가 iverilog 와 반대(IEEE 미정의 가능) · CA 걸린 wire 의 `release` 가 강제값을 남긴다(IEEE §9.3.2 상 vita 오류) → §2.

#### 4.5.306 ③층 S2 슬라이스 2 — W 경로가 부호와 비교를 배운다: native 2.16 s 로 **처음 ②층을 앞선다**, 그리고 커버리지가 0인 중심 변경 (2026-08-05, branch feat-tier3-s2-dynidx, format 26 불변) ✅

**측정이 큐의 표적을 정정하고 시작했다.** 큐는 "쓰기 퍼널이 다음" 이라 적었지만 프로파일을 **호출자별로 귀속**해 보니 `resize` 859 중 511·`from_packed` 265 중 223 이 **거절된 트리를 도는 제네릭 평가기**였다(쓰기는 `write_lvalue` 경유 90). 경로 카운터를 심어 세니 대입당 **offsets 71.9k · 거절 rhs 26.8k · 조건 21.7k**, 그리고 거절 rhs 의 절반이 **`integer` 의 signed `x = x + 1`**, 나머지 절반이 동적 인덱스였다 — 즉 한 덩어리는 **부호와 비교**다.

**형태.** ① **signed admission**: 균일 폭에서 2의 보수는 And/Or/Xor/Not/Add/Sub/Shl/Shr 의 **비트를 같게** 만들고(넓힘이 없으므로 부호확장도 없다) 유일한 예외인 **signed `>>>`(AShr) 는 거절**한다. ② **비교**(Lt/Le/Gt/Ge/Eq/Ne): 두 피연산자가 이미 폭과 부호를 공유할 때만 admit(§11.8.1 mixed-sign 질문이 발생하지 않는다), 결과는 1비트, 그리고 **비교 자체는 재진술하지 않는다** — `eval::binops::relational`/`log_eq` 를 `EvalCtx` 메서드에서 **자유 함수로 추출**해 양쪽이 같은 철자를 부른다(메서드는 위임 · 리뷰어가 본문 바이트 동일 확인). ③ **`k_truthy` 라우팅**: 조건도 W 로 평가하되 **판정은 같은 `truthiness`** 가 내린다. 마스크는 프로그램이 아니라 **op 마다** 붙는다 — 한 프로그램이 두 폭을 갖기 때문(비교 피연산자 `ow` vs 결과 1).

**성능.** native **2.79 → 2.16 s**(슬라이스 1 시작점 4.49 대비 **2.08×**) — **처음으로 ②층(2.56 s)을 앞섰다**. 카운터: 거절 rhs 26.8k → 13.1k · 조건 21.7k 중 14.6k 흡수. ⭐ 그리고 **프로파일이 내 코드를 1위로 지목**했다 — 슬라이스 직후 최대 self 비용이 `wprog_for`(BTreeMap 조회 792 샘플 vs 프로그램 실행 367)라, ExprId 직접 인덱스 벡터로 바꿔 **2.62 → 2.16 s**. 특수화가 자기 오버헤드로 이득을 되돌려주고 있었다.

**게이트.** **5156 green** · clippy · fmt · format 26 · 소진 배터리 **36형 × 65,536 조합**(부호 7행·비교 12행·**복합 피연산자 비교 6행**·필수 거절 1행) · 코퍼스 admitted **2255 → 7575** · 적대 60 · census 핀.

**적대 렌즈 ① differential — 발산 0.** 손설계 51 + **생성 퍼즈 140**(폭 1~64 × 부호 × 전 연산 × `01xz`) + keccak 3변종 + examples, 각 4-arm. **PRE-native == POST-native 전수 바이트 동일**·폴백 0·계측으로 비공허 증명(퍼즈에서만 admitted 15,028·비교 4,841·signed 3,123·W-truthy 4,363). 뮤테이션 17 중 15 kill, 그중 **넷은 리뷰어가 설계를 새로 지어야 죽었다**(내 배터리는 그중 둘을 이미 죽였고, **상수 인덱스 경계 둘은 생존** — 범위 off-by-one 은 *다음 넷의 저장*을 읽는 조용한 오답이고 x-평면 검사 삭제도 같다 → 판별 설계 영구화로 4/4). ⭐ 리뷰가 **주장의 범위**도 정정: `$display("%b", a<b)` 는 `eval_task_arg` 로 가서 W 경로에 **안 닿는다**(리뷰어의 첫 광폭 비교 설계가 그래서 **컴파일 0 = 공허**였다) → 모듈 doc 에 진입점 둘을 명시.

**⭐⭐ 적대 렌즈 ② soundness — 이 슬라이스의 중심 변경에 커버리지가 0이었다.** `WOp` 를 per-op 마스크로 재편한 이유가 "한 프로그램이 두 폭을 갖는다" 인데, 전 스위트에서 컴파일된 **비교 프로그램 ~690 개가 전부 `ops.len() ≤ 3`**(양 피연산자가 단일 Load/Const), **복합 피연산자 비교는 0**. 그래서 피연산자 마스크를 슬라이스 1 의 단일 프로그램 마스크로 되돌리는 뮤테이션이 **전 스위트를 통과**했다 — 판별은 한 줄(`(a+b) < (a^b)` 에서 분기가 뒤집힌다·exit 0). 배터리에 복합 피연산자 6행 + `op_count() > 3` 단언을 넣어 kill. ⭐ 그리고 **`wait(e)` 술어가 두 철자로 갈렸음을 두 렌즈가 각각 도달**(바디 진입은 `k_truthy`, `fire_waiters` 재검사는 `ctx().truthy`) — 오늘 일치하지만 이 슬라이스가 만든 표면이고 `fire_waiters` 의 그 arm 은 **전 스위트에서 4회**만 실행돼 보이지 않았다 → 한 철자로 통합. ⭐ **내가 "핀" 이라 쓴 census 는 핀이 아니었다**(단언이 `admitted > 0` 뿐 · 게다가 "bench/ 없으면 skip" 주석은 거짓 — `.gitignore` 가 `/bench/keccak/` 를 un-ignore 한다) → 실수치 `(129, 3)` 를 단언. ⚠️ 전방 위험 기록: `Signal` arm 이 **kind 를 안 보고 shape 만** 봐서, 아레나가 `NetKind::Real` 거부를 푸는 날 f64 비트가 정수 경로로 들어가 `truthiness` 가 `-0.0` 을 **참**으로 읽는다 → `matches!` 가드를 **국소로** 추가(오늘은 도달 불가 = 반증 불가로 기록).

**CLEARED(측정)**: signed 관성 **43행 × 9폭 × ~52k 벡터쌍 ≈ 2020만 비교 0 발산** + 메커니즘 근거(등폭에서 `resize_keep_sign` 은 `mask_top` 뿐 · `bitwise`/`shl_grow`/`shr_fill` 는 `.signed` 를 안 읽는다 · 유일하게 읽는 `shift_right(arith)` 가 거절 arm) · 비교의 `ow`/`osigned` 가 제네릭의 `max(self)`/`both_signed` 와 동일 · truthy 열거(모든 조건 경로가 `Branch`→`k_truthy`) · 캐시 eid OOB 는 `wt.get` 이 먼저 패닉 · 추출 본문 바이트 동일.

**남은 표적(다음 슬라이스)**: 동적 인덱스 배열 읽기 2 형태(census 가 핀) · 대입당 offsets 71.9k(전량 제네릭) · 캐시 컨텍스트 가드와 kind 가드는 **오늘 반증 불가**(기록).

#### 4.5.305 ③층 S2 슬라이스 1 — 폭별 특수화 W-평가기: native 1.61×, 그리고 admission 을 건너뛴 arm 하나가 loud→silent 를 만들 뻔했다 (2026-08-05, branch feat-tier3-s2-width, format 26 불변) ✅

**측정이 계획을 확정하고 시작했다.** `/usr/bin/sample` 프로파일(keccak_f_flat·release): 네이티브 워크의 ~60% 가 `Value` 조작(mask_top 16%·resize 12%·has_xz/from_packed/to_u64 ~12%)+제네릭 재귀 평가기(eval_ctx 16%·eval_binary 12%)+read_net 박싱 6.5% — doc-21 §2 의 "표현 비용" 진단 그대로. 핫 코드 실측: 상수 인덱스 배열 원소(322회)·xor/and/or/not·상수 시프트·**전부 균일 64비트**.

**형태 = `native/wprog.rs`.** (rhs·ctx폭·부호) 별로 컴파일·캐시되는 스택 머신 `WProg` — `W=(val,unk)` 두 평면 u64, Load 는 컴파일 시점 확정 버퍼 인덱스 두 개. **admission 이 정확성 논증이다**: 균일 폭 ≤64·unsigned·{Numeric Const·whole/상수-인덱스 Signal·Not·And/Or/Xor/Add/Sub·상수량 Shl/Shr/AShr} 만 admit → **넓힘·부호확장·절단이 트리 안에 존재하지 않아 문맥 폭 규칙을 재진술하지 않는다**(classifier-must-match 함정 회피). 나머지는 전부 제네릭 경로(= 기존 경로 그대로). 남는 것은 4-state 비트 테이블뿐이고 그것은 **측정으로** 핀: 폭-4 소진 배터리(256×256×11 형·전 4-state 조합) + 코퍼스 admitted 스윕(**2255건 핀**) — 둘 다 제네릭 평가기와 0 발산.

**성능**: native **4.49 → 2.79s**(keccak_f_flat·N=5000·release) = **1.61×**, vm(2.55s)과 10% 이내. **중단 판정**: And arm 실측 = 로드 2 + **분기·호출·할당 0 + ALU 9**(Xor 8) — 문자 그대로의 "2 op" 는 2-state 전용이며 two_state 슬롯 컴파일 특수화(슬라이스 2)가 그 표적. 재설계 사유 없음 — 통과.

**⭐⭐ soundness 가 잡은 것: admission 을 건너뛴 arm.** `k>=w` 상수 시프트 지름길이 lhs 서브트리를 **방문하지 않고** `Const{0,0}` 을 내놓아, 거절돼야 할 트리가 admit 됐다 — ① 동적 OOB 인덱스가 lhs 에 있으면 **E4002 와 exit class 가 사라지고**(loud→silent) ② `$urandom` 이 lhs 에 있으면 draw 하나가 사라져 **이후 RNG 스트림 전체가 밀렸다**(exit 0 값 발산·실측 native z=e220a839 vs vm z=6e789e6a). ③ 그 arm 의 주석 *"exhaustive tests 로 측정했다"* 는 **거짓**이었다 — 배터리 시프트가 전부 k<w 라 Const{1,0} 뮤턴트가 58 테스트 전부를 통과했다. 수정 = **lhs 를 무조건 먼저 컴파일**(decline 전파 복원) 후 `Const{0,0}`+`And` 로 소멸(4-state 전 입력에서 definite-0 — w=64·k=64 의 u64 시프트 오버플로 엣지도 회피). 라운드 2 재검 4/4 CONFIRMED-FIXED · 배터리 k≥w 행 2 추가 · 판별 설계 2 영구화(적대 59) · 원 철자·상수 뮤턴트 **둘 다 kill**(각각 단일 앵커 — 분업 실측).

**적대 렌즈 ① differential(25설계 ×4실행·8축) — 발산 0·폴백 0.** admitted 고문(폭 1/5/32/63/64·12-op 체인·all-x/z·경계 캐리)·decline 경계 7설계·혼합(admitted↔declined 피딩·CA/blocking/NBA/**delayed NBA·delayed CA** 위치)·VCD 바이트·진단 인터리브·examples 4 — 전부 3-way 동일. **비공허 증명이 타이밍**: x/z-heavy 50k 사이클서 PRE-native 0.197→POST-native 0.112s(1.76×·바이트 동일) = W 경로가 실제로 그 형태를 돌린다. ⭐ 부수 하나는 **리뷰어의 프레이밍이 틀렸고 실측이 정정했다**: *"keccak 핸드셰이크가 vita 에서 1클럭 늦게 깬다"* 로 보고됐으나(퍼뮤테이션당 +1주기·값 무영향), 계측해 보니 **vita 결함이 아니라 TB 레이스**다 — 설계 계측(START/DONE 시각)에서 iverilog 는 t=25000 에, vita 는 t=35000 에 `start` 를 샘플한다. 원인은 TB 가 `start = 1'b1` 을 **설계가 `start` 를 샘플하는 바로 그 posedge 에서** 세우는 것: 같은 Active 리전에 깨어난 두 프로세스의 순서는 IEEE 1364 §11 이 **미정의**로 둔 것이고 두 시뮬레이터가 다른(둘 다 적법한) 선택을 한다. 레이스를 없앤 재현(`wait`/`@`·NBA·blocking 3형 + 핸드셰이크 최소형)은 **전부 정확히 일치**. 함의는 정확성이 아니라 **측정 비교가능성**(§7.3 이 keccak 위에 선다) — bench TB 를 레이스-프리로 고치는 것은 숫자를 다시 재는 슬라이스가 소유한다.

**적대 렌즈 ② soundness — CLEARED 측**: 균일 폭 admission 이 넓힘 전부를 배제(캐리 케이스 실측 decline)·Load 레이아웃=아레나 stride·signed 리터럴 decline(비공허 쌍 실측)·AShr=unsigned 서 논리 시프트(3-way)·캐시 수명=런 1회·RefCell 재진입 불가·깊이 계산은 용량 힌트일 뿐·하네스 슬롯 기하 72설계 동일. 뮤테이션 7 중 **6 kill·1 등가**(캐시 키에서 w 탈락 — 현 lowering 은 대입마다 새 eid 라 판별 설계 구성 불가·잠재 조건을 키 주석에 기록). ⭐ 리뷰어가 **수정 서술의 거짓도 잡았다** — 캐시 키 주석이 "넣었다" 고 서술됐으나 파일에 없었다(파이썬 다중 편집 스크립트가 마지막 write 전에 끝남 — 기록된 함정의 재발) → 확인 후 실제 기록.

**게이트.** **5155 green** · clippy 0(과복잡 타입 → `WCache` alias) · fmt · format 26 · 배터리 256×256×11 + 코퍼스 2255 핀 · 적대 59 · 뮤테이션 원 6/6 + kgew 2/2 kill.

#### 4.5.304 ③층 S1d-5 — `$value$plusargs` 배선: keccak_f_flat 네이티브 + ③층 기준선 실측, 그리고 변환기의 침묵 셋 (2026-08-05, branch feat-tier3-plusargs, format 26 불변) ✅

`stmt_effect` 가족의 첫 구성원이 배선됐다 — `k_value_plusargs` 가 공유 `exec::plusargs::effect`(파싱·매칭·변환)를 지나고 **쓰기만** 자기 스토어로, 게이트 행은 정본 술어 `value_plusargs_rhs` 로 정확히 그 구성원만 carve-out(분류 `rhs_is_stmt_effect` 는 불변 — tier-2 컴파일 게이트는 한 가족을 그대로 본다).

**측정이 큐의 전제를 정정했다.** "이것 하나로 keccak 둘이 적격" 은 절반만 참 — stmt_effect 는 **첫 거부 행**이라 그 뒤의 행을 가리고 있었고, 걷어내자 호출형·배열형은 **frame-local storage(S3)** 로 넘어갔다. **keccak_f_flat 만 네이티브로 돌고 바이트 동일**(N=5000). 그 위에서 ③층 기준선을 처음 쟀다(release·best-of-3): **interp 5.22s · vm 2.53s · native 4.49s · iverilog 7.05s** — 오늘의 ③층 워크는 **②보다 1.8× 느리다**. 예상된 값이다(S2 폭별 특수화·S3 코드젠이 표현 비용을 지우는 단계) — 그 단계들이 움직여야 할 정직한 시작점으로 기록한다.

**그라운딩이 pre-existing silent-wrong 을 둘 낳았다(커밋 분리).** ① **폭 축**: 변환 전체가 `u64::from_str_radix(..).unwrap_or(0)` 라 16자리 초과 %h·64자리 초과 %b·u64 초과 %d 가 전부 **조용한 0**(got=1), 음수 %d 는 64비트 초과 목적지에서 zero-extend. → 비트 radix 는 자릿수만큼 워드 누적·десimal 은 목적지 폭에서 wrap(부호 확장이 자동으로 떨어진다)·절단은 LOW bits(iverilog 실측 `+D=4294967297`→1). ② **4-state 축**(differential 렌즈 발굴): x/z 자리가 2-state 로 파싱돼 `1x2`→1, junk 접미사가 **선행 자리만** 조용히 파싱돼 `5x9`→5, `+5`→0. → 리터럴 관례 그대로(위치별 x/z·MSB 자리 종류가 목적지 폭까지 확장·밑줄 구분자·lone x/z = 전체 x/z) + invalid 는 **W4028 신설**(radix 명·값 인용) + all-X — 경고 없으면 철자 틀린 plusarg 가 exit 0 으로 X 만 남긴다. MsgCode 60→**61**. 검증 = 그라운딩 p1~p8 전 케이스 + 무작위 40값 퍼즈 3-way **0 diff**.

**게이트.** **5153 green** · clippy · fmt · format 26 · 오라클 앵커(양 백엔드·경고 라인 포함) · native_gate 의 거부 단언 → **수용 단언** 반전.

**적대 렌즈 ① differential(31설계+keccak 3종·3-way·PRE-A 12) — 배선 자체 발산 0·폴백 0·PRE/POST 0 diff.** 위 4-state 셋이 이 렌즈의 발견이고, 추가 기록 둘: no-% 퇴화형은 vita 확장(오라클은 런타임 거부 — no-oracle 유지)·`%0d` 는 vita 과잉거부(loud·§3).

**적대 렌즈 ② soundness — "공유했으면 소비자마다 앵커" 위반을 실측.** ⭐⭐ **native 경로에서 `$value$plusargs` 를 돌리는 테스트가 0** — native 쓰기를 통째로 지워도(M6)·status 를 1 로 박아도(M7)·negation 의 목적지 폭 스팬을 지워도(M4) **전 스위트 초록**. 공유 반쪽이 Scheduler 소비자로만 앵커돼 있었다. → iverilog 절대값 앵커 1개(5축: hit·miss[변수 불변+status 0]·음수 %h/%b[목적지 폭 negation]·24자리 %h) 로 3/3 kill. ⭐ **이 슬라이스의 게이트 스위트가 이 트리에서 빨강** — `native_gate` 의 plusargs 거부 단언이 배선으로 뒤집혔는데 **그 빨간 테스트가 carve-out 반전 뮤테이션(M8)을 가렸다**(어느 단언이 죽는지만 바뀐다). 빨강 수정 후 M8 은 두 킬러. ⭐ 하네스 함정 **잠재 확인**: `build_with_opts` 는 plusargs 를 설치 안 하고(소스만 파싱), 프로덕션은 `simulate` 가 복사 — 최초의 native plusargs 테스트가 MISS 경로만 잴 뻔했다 → plusargs 받는 helper 신설 + 주석. stale 산술 셋(18→17 refused·"eleven"→"ten") 재계수.

**경계(기록).** keccak 호출형·배열형 = S3 소유 · no-% 퇴화형 = vita 확장(no-oracle) · `%0d` = §3 · 4건 obs 병렬 flake(격리·재실행 초록·기지 클래스).

#### 4.5.303 ③층 S1d-4d-4 — multi-driver·wired 해상: S1 거부 행에서 cont-assign 이 사라졌다 (2026-08-05, branch feat-tier3-wired, format 26 불변) ✅

`wire`/`wand`/`wor` 다중 구동 넷의 4-state 해상을 ③층이 실행한다 → **S1 거부 행에서 continuous-assign 계열이 전부 사라졌다**(남은 행 = `final`·fork/`wait fork`·서브루틴/frame·`$monitor` 계열). 코퍼스 커버리지 **0**(실측 — multidriver·wired 둘 다)이라 검증 전부가 전용 설계다.

**형태.** §4.5.302 룰의 직접 적용 — 해상 **fold**(항등원 all-Z·kind 테이블 디스패치·`resolve_{wire,wand,wor}_into`)를 `resolve_md_group` 자유 함수로 **추출**해 두 settle 루프가 공유하고, 스토어를 만지는 반쪽(드라이버 RHS 평가·LHS 오프셋·쓰기)만 백엔드별로 남겼다. 그룹 분류도 한 철자 — 네이티브는 스케줄러의 `md_groups()`/`ca_is_md()` 를 읽는다(두 번째 유도 없음). 엔진 루프는 평가-후-fold 로 재배열됐지만 fold 는 진단 무발화라 스트림 불변(PRE/POST 44설계 0 diff 로 증명). 게이트에서 두 행과 `md_nets` 파라미터가 사라졌고, `lib.rs` 는 더 이상 `multi_driver_groups` 를 게이트에서 세지 않는다.

**게이트.** **5149 green** · clippy · fmt · format 26 불변 · 적대 설계 57(md 7 신규) · **오라클 앵커 4**(wire z/conflict·wand·wor·3-driver 항등원 — 전부 iverilog 13 절대값·양 백엔드) · 델타 예산 스윕 테스트 1 · 뮤테이션 **비등가 11/11 kill·등가 1 논증**.

**적대 렌즈 ① differential(45설계 3-way + PRE 4-way) — seam 발산 0.** 값 행렬 16셀×3종·시변 핸드오프·다운스트림 체인/웨이터/NBA·폭 64/65/96/128(워드 경계 straddle)·부호·E4002 스트림·혼합 가족·경계(E3001 4형태 동일 loud)·VCD 바이트·t0 — 전부 CLEAN, **폴백 0**(41설계 전부 run.json `backend:"native"`). ⭐ **pre-existing 발산 둘**을 냈다(양 백엔드+PRE 동일 = 이 슬라이스 무관·§2 기록): ⓐ **keeper 사이클이 t0 x-창을 영구 래치** — `assign p=q; assign p=a?1:z; assign q=p;` 에서 vita 의 구조 settle 이 initial 바디 **전에** 돌아 `a=x` 를 읽고, x 는 해상에서 흡수원이라 `a=1` 이 와도 회복 불가(iverilog `1/1` vs vita `x/x`·exit 0) ⓑ **CA 가 t0 에 z 로 settle 하면 `@(y)` 가 한 번 더 깬다**(vita 넷은 x 로 태어나 x→z 가 변화·iverilog 는 z 로 태어나 무이벤트) — §4.5.302 가 기록한 "초기값을 변화로 몰기" 가족의 세 번째 구성원.

**적대 렌즈 ② soundness — 뮤테이션 생존 2 가 게이트의 축을 다시 쟀다.** ⭐⭐ **M2: 드라이버 평가 순서가 미핀** — fold 가 가환이라 값-전용 설계로는 순서가 절대 안 보이는데, **순서가 관측되는 채널이 이미 열려 있었다**: `$random` 은 admitted(순수-평가 쪽)라 **한 그룹에 impure 드라이버 둘**이면 평가 순서가 곧 draw 순서다(`assign y=$random; assign y=~$random;` — 역순 뮤턴트에서 vm `x11x` vs native `x00x`). ⭐⭐ **M5: md 루프의 패스 내 위치가 미핀** — 어느 위치든 값은 수렴하므로 스트림이 안 움직이고, 움직이는 것은 **수렴에 드는 패스 수**다. 평범한 CA 가 그룹 드라이버를 **먹이는** 형태에서 뮤턴트는 한 패스를 더 쓰고, 델타 예산 스윕(1..=24)이 예산 2 에서 두 백엔드를 가른다(regime 경계 교차를 anti-vacuity 로 단언). 둘 다 teeth 를 지어 kill. M4(그룹 쓰기 lhs 를 first→last)는 **등가 논증 기록** — `multi_driver_groups` 가 whole-net 만 admit 하므로 전 멤버의 `Lvalue` 가 구조 동일. ⭐ stale 주장 둘 수정: `--backend` 도움말이 이 슬라이스 이전의 거부 목록을 광고 · **이 슬라이스가 수정한 파일의 모듈 doc** 이 "Still refused: delayed/multi-driven/wired" 를 유지(§4.5.300 이 `arm_t0` 상대로 기록한 바로 그 실패 모드 — 슬라이스가 자기 전제 문장을 안 고친다). CLEARED: all-sites(t0/델타 settle 동일 함수·엔진 쌍둥이)·사이드카 전 경로(원샷/staged trailer/하네스 — staged 는 fresh bins 로 실측)·E3001 경계 4형태·quiescence 스윕 1..=30·제거된 행의 소비자 0.

**경계(기록).** vita 는 부분/지연 겹침을 E3001 로 거부하지만 **iverilog 는 같은-범위 part-select 쌍·delayed+plain 겹침을 비트 단위로 해상**한다 — pre-existing loud-vs-support 갭, ROADMAP §3 1줄.


#### 4.5.302 ③층 S1d-4d-3 — delayed CA: **코퍼스 72/72**, 그리고 공유 코드는 차분이 못 지킨다 — 앵커 셋 (2026-08-05, branch feat-tier3-delayed-ca, format 26 불변) ✅

`assign #d` 를 ③층이 실행한다 → **코퍼스 72 설계 전부가 네이티브로 돌고 stdout·진단·VCD 바이트 동일**
(30 → 65 → **72**). 남은 거부는 `wand`/`wor`·multi-driver·`final`·fork·서브루틴·`$monitor` 계열뿐.

⭐ **이번엔 재진술이 아니라 추출을 골랐다.** inertial 취소(`ca_gen`)·전이별 rise/fall/turnoff 선택·
sole-driver X 창은 전부 **iverilog 로 핀된 미묘한 규칙**이라 두 철자가 갈리면 조용히 틀린다 → 엔진
로직을 `schedule_delayed_cas`/`take_due_delayed_ca`/`delayed_owes_initial_x` 로 꺼내고 **엔진도 그것을
통과**하게 했다(PRE/POST 179설계 0 diff 로 확인).

⭐⭐ **그런데 추출이 절반만 됐다 — differential 이 reachable silent-wrong 을 잡았다.** RHS 는 seam 으로
읽으면서 **LHS 오프셋은 `self.resolve_lvalue_offsets`**, 즉 엔진 스토어로 해석했다. 네이티브 런에서 그
스토어는 안 움직이므로 동적 인덱스(`assign #1 y[i] = v;`)가 X → out-of-range 센티널 → **쓰기가 통째로
사라진다**. exit code 동일·출력 동일·한 비트만 없다. 10 설계 재현(bit-select·`+:`/`-:`·배열 워드·VCD
레코드 누락 포함). ⭐ 진단이 살아 있었던 이유가 재미있다 — **같은 기능의 X-drive 반쪽은 아레나를 썼다**
(`native/run.rs`), 그래서 `x` 가 **맞는 비트 자리**에 찍혀 있었다. 한 기능, 두 반쪽, 두 스토어.

⭐⭐ **그리고 직전 슬라이스의 교훈이 곧바로 청구서로 돌아왔다** — 공유로 옮긴 부분은 **VM-vs-native
차분이 원리적으로 못 지킨다**(양쪽이 같이 움직인다). 실측: `take_due_delayed_ca` 의 **generation 필터를
지워도 전 차분 게이트 통과** · `transition_delay` 선택을 무시해도 통과. → **절대값 앵커 두 개**를 지었다
(iverilog 13 로 측정: 좁은 펄스는 LHS 에 절대 안 닿고[`narrow` 가 창 내내 x] 폭 == `d` 는 통과[t=4 에 9]
· rise 2/fall 7 이 t=13/t=23 에 발화) → 둘 다 kill. **공유는 드리프트를 없애지만 감도도 없앤다**를
규칙에서 실행으로 옮긴 것.

⚠️ 하네스 갭 하나 더: `ca_delays` 사이드카가 `build_with_opts` 에 미설치라 rise/fall 테스트가 **균일
지연을 재고 있었다**(`final_procs`·`wired_*` 와 같은 클래스 — 세 번째다).

**게이트.** **5144 green** · clippy · fmt · format 26 불변 · **코퍼스 72/72 거부 0** · dump 44/44 VCD 비교 ·
판별 설계 50 + **오라클 앵커 3** · differential **179 설계 중 175 native 확인, 수정 후 0 diff** + PRE/POST
기본 백엔드 179 **0 diff**(추출이 엔진을 안 바꿨다는 증명) · 뮤테이션 **라운드1 9/9 + 라운드2 6/6 kill**.

**적대 렌즈 ② soundness(라운드 2) — 뮤테이션이 게이트의 눈을 다시 쟀다.** 리뷰어가 제품 뮤테이션 21개를 지어 **14 생존**을 실측했고(각각 판별 설계로 비-등가 증명), 그중 6개가 이 슬라이스 확정분이었다. 재실행하니 **M10**(`next_delayed_ca` 를 시간 진행 min 에서 제거)·**M12**(휠 쓰기 뒤 `propagate` 제거)는 라운드 1 이 추가한 설계가 이미 죽이고 있었고 **셋이 남았다**:

- **M4 — 전이 지연의 BASELINE.** `last_ca_drv`(이 assign 이 실제로 **구동한** 마지막 값)를 `last_ca`(마지막으로 **본** RHS)로 바꿔도 코퍼스 72·적대 50·앵커 2개가 전부 통과한다. 둘은 **inertial supersede 에서만** 갈리는데(대기 중 쓰기가 착지하기 전에 RHS 가 또 바뀌면 취소된 값이 baseline 이 되어 rise 가 fall 로 뒤집힌다) 게이트의 어떤 설계도 한 대기 쓰기당 RHS 를 두 번 바꾸지 않았다. 판별 설계 = `assign #(2,9)` 에 00→11(t=20)→10(t=21) → 정답 **t=23**(rise) · 뮤턴트 t=30(fall) · **iverilog 13 = 23**. `take_due_delayed_ca` 는 **공유 코드**라 차분이 원리적으로 못 본다 → 세 번째 절대값 앵커.
- **M17/M18 — 네이티브 arm 의 평가 CONTEXT 는 숫자 둘(폭·부호)인데 둘 다 미핀.** 엔진 arm 은 `eval_cont_assign` 을 지나고 네이티브 arm 은 seam 에서 같은 규칙(`max(lhs, self(rhs))`·rhs 자신의 부호)을 **재진술**한다. 코퍼스의 유일한 지연 형태가 `assign #2 dly = a;`(양쪽 같은 폭·무부호)라 `lw.max(..)` 를 지워도(`4'hF+4'hF` 가 `1e` 대신 `0e`) 부호를 `false` 로 박아도(`-3` 이 `00001101`) **전부 통과**했다. 판별 설계 = 넓히는 캐리 + 부호 확장, 값은 iverilog 13.

세 앵커를 세운 뒤 **6/6 전부 사망** — **M20**(엔진의 `propagate_changes` 를 쓰기 앞으로 옮기는 **엔진측** 뮤테이션·리뷰어 측정에선 `sim-engine` 전체가 초록이었다) 포함. 엔진이 움직이고 네이티브가 안 움직이니 이제 차분이 잡는다.

⭐⭐ **앵커를 짓다가 오라클 발산 둘을 새로 쟀다.** 첫 M4 설계가 `always @(y)` 모니터를 썼더니 vita 가 iverilog 에 없는 **`t=0` 이벤트**를 하나 더 냈다 — 초기-X 창을 **넷 초기값이 아니라 t=0 의 변화**로 구현했기 때문이고, 같은 뿌리에서 **서로소 part-select 를 지연 구동하는 두 assign** 은 창 안이 x 가 아니라 **z** 다(iverilog `t=1 y=xxxx` / vita `zzzz`). 둘 다 pre-existing·양 백엔드 동일이라 이 슬라이스가 만든 하강이 아니고, 후자의 올바른 술어("넷의" 유일 드라이버가 아니라 **"이 lvalue 가 닿는 비트의"** 유일 드라이버)는 **S1d-4d-4 가 짓는 비트 단위 드라이버 맵**을 요구한다 → 측정과 함께 ROADMAP §2 에 기록하고 그 슬라이스에 넘겼다. ⭐ **교훈**: 기대 출력에 알려진 발산이 들어가면 **앵커가 앵커이길 그만둔다** — M4 설계를 엣지 모니터에서 **시각 지정 `$display` 샘플링**으로 바꿔, 뮤테이션이 움직이는 값만 재고 이벤트는 축복하지 않는다.

⭐ 리뷰어는 **왜** 게이트가 못 봤는지도 쟀다: 코퍼스의 지연 형태는 `gen_cont_assign_mixed` 의 `assign #2 dly = a;` **하나뿐**(전체 넷 lvalue·맨 신호 RHS·`a` 는 t0 에 한 번만 쓰인다)이라 `(72, 0)` 이 사는 것은 초기-X 창과 휠 쓰기 한 번이 전부다. 그리고 inertial 의미를 iverilog 로 핀해 두고 M1/M3 을 워크스페이스 전역에서 죽이던 `cli/tests/inertial_ca.rs` 는 **기본 백엔드만** 돌리는 데다 설계가 `$monitor`(③층 거부 태스크)를 써서 `--backend native` 로도 네이티브 경로에 **구조적으로 못 들어간다**.

CLEARED(측정): 시간 진행 순서가 엔진과 동일(`snapshot_preponed` 만 다르고 clocking 은 게이트 거부라 no-op) · settle 배치 동일 · 초기-X drive 의 술어/지점/폭 동일 · quiescence 소비처 2곳 대응 · RHS seam 값 9형태 · VCD 44/44 바이트 · **PRE/POST 엔진 0 diff**(코퍼스 72 + `examples/` 4 + picorv32 + keccak).


#### 4.5.301 ③층 S1d-4d-2 — VCD: **실사용 설계가 stdout+파형까지 바이트 동일** (2026-08-05, branch feat-tier3-vcd, format 26 불변) ✅

`$dumpfile`/`$dumpvars` 를 ③층이 실행한다 → **`examples/` 넷 전부가 네이티브로 돌고 stdout·VCD 둘 다
바이트 동일**. 원래 S1 게이트가 실사용 설계에서 처음 통과했다. 직전 반복의 그라운딩이 특정한 이음매
넷이 그대로 맞았다 — `full_snapshot_with` 제네릭 리더(**`$dumpvars` 안에서 스토어를 읽는 유일한
함수**) · `emit_vcd_change` → `vcd_id_for`+`emit_vcd_packed` · `note_change` 에 **`word` 복원**(배열은
원소마다 VCD id) · **값을 store 지점에서 캡처해 버퍼링**(아니면 슬롯 내 글리치가 한 레코드로 합쳐진다).

코퍼스를 **더 이상 스트립하지 않는다** — dump 를 가진 44 중 37 이 게이트 안에서 돌고 VCD 바이트까지
비교된다(나머지 7 = delayed-CA 행). body-walk 커버리지도 58/380 → **72/556**(전 코퍼스).

⭐⭐ **differential 이 내 주석의 주장을 반증했다** — `$dumpfile` 을 거부에서 뺀 근거로 *"`arg_string` 은
non-Const 에서 early-return 한다"* 라고 적었는데, 그 함수는 **값 렌더로 흘러가 엔진 스토어를 읽는다**.
`$dumpfile(nm)`(nm=42)이 네이티브에서 **`x` 라는 파일**에 썼다 — stdout·VCD 내용·exit code 전부 동일,
**파일 이름만** 다르다(그래서 바이트 비교가 못 본다). 수정 = `arg_string_with` 로 리더 threading +
`SimResult::vcd_path` 를 비교하는 전용 테스트.

⭐⭐ **soundness 가 게이트의 눈먼 축을 다섯 개 셌다** — 전부 "코퍼스에 그 형태가 없다": 폭 >64 dump 넷
0(코퍼스의 wide 템플릿엔 `$dumpvars` 가 없다) · **런 중** x/z 쓰기 0 · **두 드레인 사이 두 쓰기** 0
(글리치 템플릿은 세 문장이고 워크는 문장마다 드레인해 **capture-at-store 를 재현 불가**) · `$dumpoff`
0. 판별 설계 4개로 전부 kill. ⭐ 그리고 **`agree` 의 VCD 비교가 공허할 수 있었다** — `None == None` 이
통과하므로 `dumpvars_with` 를 양쪽 no-op 으로 만들면 "37 파형 일치" 가 **무 대 무 37건**이었다.

⭐ **한 뮤테이션은 원리적으로 못 잡는다**: `emit_vcd_packed` 는 **공유 코드**라 거기를 깨면 차분의
**양쪽이 같이 움직인다**(§4.5.293 의 "같음으로는 출처를 시험 못 한다" 가 한 층 위에서 재현). 잡는 것은
iverilog 앵커 테스트뿐 — 기록.

구조 개선(리뷰 지적): `$dumpvars` 인터셉트가 `dispatch.rs` 의 **쌍둥이**였다(그쪽이 이미 리더를
받는다) → 삭제 · **모든 `run` 종료 경로에 최종 드레인 + `debug_assert`**(9 경로 감사는 논증이고 이건
가드다) · `emit_vcd_packed` 에 자체 `dumping` 가드(단, `vcd_on` 이 `dumping` 을 추적하므로 **등가**임을
측정해 적었다).

**게이트.** 5140 green · clippy · fmt · format 26 불변 · **examples 4 stdout+VCD 바이트 동일** ·
코퍼스 dump 37 VCD 비교 · 판별 설계 47 · differential **502 런 중 498 native 확인, 수정 후 0 diff** ·
soundness **랜덤 150 + 손 38 + iverilog 3rd lens 0 신규 발산** · 뮤테이션 12 중 11 kill(생존 1 = 등가).

#### 4.5.300 ③층 S1d-4d-1 — cont-assign settle, 그리고 **byte-identity 논증이 값 축에서만 참이었다** (2026-08-05, branch feat-tier3-ca-settle, format 26 불변) ✅

zero-delay 연속대입 fixpoint 를 ③층이 돈다(t0 arm 前 + 매 델타 상단 · 움직였으면 전파). 거부는 blanket
"any cont-assign" 에서 **delayed `assign #d` · `wand`/`wor` · multi-driver** 셋으로 좁혔다. **picorv32
가 네이티브로 돌고 바이트 일치**(E4002 9건·8-cap 노트 포함) · `keccak_f_flat` 도.

⭐⭐ **내 byte-identity 논증이 절반만 맞았다.** "워크리스트 없이 매 패스 전 assign 방문 = 동일" 이라
적었고 엔진 주석도 그렇게 읽힌다(*"입력이 안 움직인 assign 은 이전 값을 재계산하고 퍼널이 같은 값
쓰기를 변경으로 안 친다"*). 그 논증은 **값**에 관한 것이다 — RHS 가 범위 밖 원소를 읽는 assign 은
재평가마다 `E4002` 를 또 낸다. **picorv32 에서 errors 6 → 9.** 워크리스트를 엔진의
`ca_of_net`/`ca_always` 에서 **재진술 없이** 가져와 연결했다.

⭐⭐ **두 렌즈가 같은 reachable silent-wrong 으로 수렴**: `arm_t0` 가 dirty 를 **통째로** 비워 t0
settle 의 변경 집합까지 버렸다 → `assign w = 1'b1;` + `always @(w)` 가 **안 뜬다**(exit 0, 조용히).
엔진 주석이 **바로 그 결함을 이미 고쳤다고 적어 둔** 자리이고, 내 주석은 *"여기엔 settle 이 없다
(거부됨)"* 였다 — **슬라이스가 자기 전제를 무효화하고 그 문장을 안 고쳤다**. 퍼즈 270 중 49 · 리뷰어
설계 18 발산(값·exit class·`$finish` 시각 전부). 수정 = 엔진과 같은 mark/split.

⭐⭐ **그리고 게이트에 이빨이 0 이었다** — settle 최상단 `panic!` 이 **전 워크스페이스를 통과**했다.
코퍼스의 CA 설계는 plain+delayed 를 **쌍으로** 내보내 전부 delayed 행에 걸리고, 판별 설계 34 개에
`assign` 이 **한 줄도** 없었다. 신호는 이미 단언 안에 있었다 — **`ran` 이 65 에서 안 움직였다**. 7 설계
추가로 뮤테이션 11/11 kill.

⭐ 리뷰가 **과잉거부**도 잡았다: multi-driver 행을 "lvalue 에 두 번 나오는 넷" 으로 근사해
**per-bit generate 관용구**(`for (g…) assign y[g] = ~x[g];`)·disjoint part-select·한 assign 의 concat
LHS 를 전부 거부했다 — 실 RTL 이 버스를 모는 가장 흔한 형태이고 전부 last-write-wins 로 충분하다.
엔진 자신의 `md_nets` 술어를 **자유 함수로 추출해 양쪽이 부른다**(내 doc 이 "한 철자" 라고 주장했는데
`Scheduler::new` 은 인라인 복사본을 갖고 있었다 — 그것도 리뷰가 잡았다).

⚠️ **예상이 빗나갔다**: 이 슬라이스가 `examples/` 넷을 열 줄 알았는데, cont-assign 행이 좁아지자
**다음 행(`$dumpvars`)이 발화**한다. 열린 것은 picorv32 와 keccak_f_flat 이고, examples 는
`$dump*` 를 지우면 **넷 다 네이티브로 돌고 바이트 일치**한다(리뷰 실측) — VCD 슬라이스가 마지막 문이다.

**게이트.** 5139 green · clippy · fmt · format 26 불변 · 판별 설계 **41** · differential **228 설계 중
218 native 확인, 수정 후 0 diff** + picorv32/examples 바이트 일치 · 뮤테이션 **11/11 kill**.

#### 4.5.299 ③층 S1d-4c-2d — in-body 웨이터, 그리고 **내 논증이 두 번 틀렸다** (2026-08-04, branch feat-tier3-inbody-wait, format 26 불변) ✅

`Wait{Edge|Level|Expr}` 를 ③층이 실행한다 — `k_suspend_on`(구현자가 웨이터 목록과 **arm 스냅샷**을
소유) · **공유 워크의 `Wait` 암**(`wait(e)` already-true 폴스루 포함 — WHEN 은 IEEE 규칙이라 한 번만
적는다) · `fire_waiters`(Level = arm 대비 차이 · Edge = 슬롯 내 마스크 · Expr = 술어 재평가 · 발화하면
**소비**).

⭐ **그라운딩이 큐를 정정했다** — 큐는 원인을 셋으로 적었지만 **`Named` 는 IR 에 아예 안 나타난다**
(elaborate 가 named event 를 64비트 카운터 넷으로 낮추고 `@(ev)` 를 평범한 `Level` 로 만든다). 코퍼스
커버리지는 여전히 **0**(정지 터미네이터 138 이 전부 `Delay`)이라 전 검증이 전용 설계다.

⭐⭐ **두 적대 렌즈가 같은 silent-wrong 으로 수렴했고, 그것은 내가 직전 슬라이스에서 고친 것과 같은
클래스였다** — `fire_waiters` 가 `wait(e)` 술어를 아레나로 평가하므로 **범위 진단의 세 번째 생산자**인데,
`propagate` 뒤에 드레인이 없어 quiescent·time-limit·delta-limit **세 종료 경로 전부에서 진단이
사라진다**. 배열 OOB 를 wait 술어에서 읽는 한 줄짜리 설계가 VM `exit 1` / native `exit 0` — **FAIL 이
PASS**. 직전 슬라이스가 NBA 경로에 대해 똑같이 고쳤고 그 쌍둥이 테스트가 두 개나 있는데, 새 생산자를
추가하면서 쌍둥이를 안 지었다.

⭐⭐ **그리고 내가 코드에 적은 "측정했다" 가 틀렸다** — `body_is_walkable` 의 `Fork` 암을 *"S0 `fork`
행이 먼저 거부하므로 커버 불가"* 라고 적고 그 근거로 뮤테이션 생존을 인용했는데, **`wait fork;` 는
`fork_modes` 를 하나도 만들지 않는다**(elaborate). 그 설계는 `eligible: true, buildable: true` 이고
**이 암이 유일한 거부자**다 — 통과시키면 아무도 못 깨우는 웨이터에 영원히 park(행). 거부 행에 케이스를
복원했고, 사용자 메시지도 **도달 가능한 원인**(`wait fork`)을 앞에 놓도록 고쳤다(옛 문구는 구성 불가한
named-event 를 광고하고 있었다).

⭐ **리뷰가 뮤테이션 다섯을 더 냈다** — 전부 "판별 못 하는 설계": in-body proc id 가 static 보다 **위**
라 정렬 삽입과 append 가 같았고 · arm 스냅샷이 **val 평면만** 옮기는 설계뿐이라 unk-only 변화를 못 봤고 ·
**배열**을 `@(mem)` 로 기다리는 설계가 없어 원소 0 스냅샷이 통과했고 · **다중 넷** `@(a or b)` 에서
둘째만 움직이는 설계가 없었다. 여섯 판별 설계로 전부 kill.

⚠️ **`in_body_level_wait_ignores_self_write` 는 이름이 거짓이었다** — 바디가 깨어난 뒤 넷을 쓰고 루프백
하므로 **재무장 스냅샷에 이미 그 쓰기가 들어 있다**. Level self-write 가드는 이 암에서 **도달 불가**
(정지 중인 프로세스는 쓸 수 없으므로 `cur != arm` 이면 작성자가 남이다)이고, 진짜 도달 가능한 것은
**Edge** 쪽이다(`clk = ~clk; @(posedge clk);` — 자기가 만든 엣지). 이름과 논증을 둘 다 고쳤다.

**게이트.** 5139 green(5138 +1) · clippy · fmt · format 26 불변 · 판별 설계 **34 + 전용 3** ·
differential **261 설계 중 246 native 확인, 수정 후 0 diff** · PRE(main) VM vs POST VM **193 설계 0 diff**
(VM 경로 무변경 증명) · 뮤테이션 18 중 16 kill(생존 2 = Level self-guard 도달 불가 · 폴스루 guard 부담이
**F4027 이 횟수를 안 실어** 관측 불가 — 둘 다 논증을 코드에 적었다).

⚠️ **프로세스 사고(기록)**: 리뷰어가 측정하는 동안 내가 바이너리를 두 번 재빌드해 리뷰어가 측정 시각별
바이너리를 명시해야 했다. 규칙은 있었는데(작업 트리 수정 금지를 리뷰어에게 지시) **내 쪽 금지가 없었다**.

#### 4.5.298 ③층 S1d-4c-2c — 런 루프: **③층이 처음으로 설계를 돌린다**, 그리고 그 순간 loud 하나가 조용해졌다 (2026-08-04, branch feat-tier3-delta-loop, format 26 불변) ✅

**계획의 슬라이스 경계를 측정이 옮겼다.** 계획은 "리전 큐 + 델타 루프 + in-body 웨이터 + `busy` +
`flush_postponed`" 였다. 코퍼스를 먼저 재니 **72 설계 중 0 개가 전 프로세스 정지-없음**이었다 — 즉
정지 모델이 **하나도 없는** 타임스텝 차분은 코퍼스 커버리지가 **0** 이고, 오직 그 게이트를 위해 쓴
설계 위에서만 돌 수 있다(직전 네 슬라이스가 구멍을 찾아낸 바로 그 게이트 모양). 그리고 코퍼스의
정지 터미네이터 **138 개가 전부 `Delay`/`Active`**(`Wait{Edge|Level|Expr}`·`Fork`·`Call` 은 0). →
**오라클이 있는 단위는 "루프 + `Delay`"** 이고 in-body 웨이터는 다음 슬라이스다. 반대로 쪼갰으면
코퍼스 설계를 **한 개도** 못 돌리는 게이트가 나왔다.

**`Delay` 를 워크에 쓰되 두 구현자가 공유한다** — `k_now`/`k_schedule_resume` 두 커널 호출을 더해
`Terminator::Delay` arm 을 `native/body.rs` 에 **한 번만** 적었다(리전 배정은 IEEE 규칙이고, 어디에
파일링하는지만 구현자의 몫). `simulate` 배선까지 해서 `--backend native` 가 **실제로 실행**한다;
세 번째 게이트 층 `native::run::executor_rows`(cont-assign·`final`·in-body 웨이터·거부 SysTask)가
막는 설계는 VM 폴백.

**⭐⭐ 두 적대 렌즈가 같은 결함으로 수렴했다 — 그리고 그것은 사다리 하강이다.** OOB/X 배열 인덱스에서
엔진은 `warn_run_range`(**`Severity::Error`** → `ExitClass::HadErrors` → exit 1)를 내는데 아레나는
**값만 맞추고 진단을 버렸다**. 소스에 리터럴 OOB 가 하나도 없는 평범한 FIFO(쓰기 포인터가 메모리를
지나친다)에서 **stdout 은 바이트 동일한데 `FAIL` 이 `PASS` 로** 뒤집혔다. 트리는 이것을 이미 적어
두고 있었으나(`native/write.rs`: *"값은 맞고 stderr 가 안 맞는다"*) **크기를 잘못 쟀다** — stderr 가
아니라 **exit class** 다. 수정 = 아레나가 **세고**(읽기 경로가 `&self` 라 sink 에 못 닿는다), 워크가
**문장 경계마다** 엔진 자신의 emitter 로 보고(`k_drain_diags`) → 캡·문구·severity 전부 재진술 0.

**⭐⭐ 그 결함이 살아남은 이유는 게이트의 축이었다**: `simulate_capture` 의 sink 는 `RtlOutput` 만
남기므로 stdout 비교는 **모든 진단에 구조적으로 눈멀어 있다**. `exit_class` 비교 한 줄이 즉시 잡는다.

**⭐ 내 뮤테이션 26건이 게이트의 이빨 없음을 먼저 드러냈다(생존 12)** — 그리고 원인은 전부
"판별하지 못하는 설계": `#0` 설계가 **순서를 뒤집지 못해** 리전 구분 3종이 통과 · 코퍼스에
**정지하는 `always @(posedge)`** 가 없어 `busy` 양쪽이 통과 · self-write 설계가 그 넷을
**감도 리스트에 안 넣어** `blocking_writer` 삭제가 통과 · 코퍼스 카운터는 **NBA 가 매 사이클
`reset_edge_seen` 을 대신해** 세 클러스터 리셋이 전부 통과. 판별 설계 9개를 지어 **생존 12 → 2**,
남은 둘은 등가 뮤테이션이라 **논증을 적었다**(`push_sorted` 의 `<=` 는 이 클래스에서 같은 proc 이
두 번 큐잉될 수 없어 관측 불가 — teeth 주장 자체를 철회 · 초기화자 본문은 멱등).

**리뷰가 더 잡은 것**: run.json 이 **두 층 판정을 실었다**(결정은 세 층으로 했는데) → 폴백마다
`refused: null` — 설명이 일인 G2 rail 이 "아무것도 거부 안 했다"고 답하고 있었다 · `arm_t0` 에
`fatal_init_proc_missing` 이 없어 잘린 `.velab` 트레일러가 **panic**(엔진은 loud) · `--backend`
도움말이 *"NOT executable yet"* 이라고 거짓말 · `arm_t0` 가 엔진에 없는 early-return 을 갖고 있었다 ·
`propagate` doc 이 **static Level 웨이터가 살아 있다는 걸 빠뜨렸고** 있지도 않은 prev-refresh 를
설명했다 · `scan_arm` 주석이 *"never a panic"* 을 약속하는데 네이티브 런은 `activities` 가 비어 있다.

**⚠️ 커버리지 정직**: `examples/*.sv` **4개 전부**와 `bench/` 둘이 **거부**된다(원인은 실 TB 가 전부
갖는 것들 — `$dumpfile`/`$dumpvars`·모듈 포트[cont-assign 으로 낮아진다]·in-body `@(posedge clk);`).
65/72 는 **루프의 커버리지**이지 아직 누가 쓸 설계의 커버리지가 아니다.

**⭐⭐ 그리고 첫 수정의 게이트도 공허했다** — `exit_class` 를 비교하도록 고쳤는데 `warn_run_range` 는
`st.had_error` 를 **세우지 않는다**(그 필드는 `$error` 계열 전용이고, 진단을 exit code 로 세는 것은
**CLI 자기 sink**). 즉 방금 닫은 결함을 그 단언이 다시 통과시킨다 — OOB-NBA 설계가 양쪽에서 `Ok` 로
읽히는 것으로 실측. 최종 형태 = **stdout 과 진단을 한 리스트에 인터리브해 비교**(sink 를 직접 짜서).
그것이 순서 잔차까지 게이트 안으로 들여왔다.

**⭐⭐ 그리고 재리뷰가 그 수정의 순서를 잡았다 — 두 번 정정해야 했다.** 아레나는 문장 경계에서
보고하므로 읽기와 출력이 한 문장 안에 있으면(`$display("%0d", mem[9])`) 진단이 줄 **뒤에** 나온다.
처음엔 "두 fd 를 합칠 때만 보이는 차이" 로 기록했는데, **`$error("%0d", mem[i])` 는 둘 다 stderr 로
나가서** 같은 스트림 안에서 순서가 뒤집혔다(E4003 → E4002, 엔진은 반대) — 즉 기록이 틀렸다. 고칠 자리는
아레나가 아니라 **포맷 엔진**이었다: `format_args_str_with` 는 리더와 (`&SimState` 를 통해) sink 를
동시에 들고 있는 유일한 지점이라, 인자 렌더 직후·호출자 emit 직전에 드레인하면 엔진과 같은 순서가
된다(`NetReader::take_deferred_range_reports`, 엔진은 기본 0 이라 그 경로는 구조적으로 no-op).
리뷰어 설계 **26 전수 merged+stderr+stdout+exit 전부 동일**. 교훈 = **"관측 불가" 라고 적기 전에
그 진단이 어느 스트림으로 나가는지 확인하라**.

**게이트.** 5138 green(5128 **+10**) · clippy · fmt · format 26 불변 · 코퍼스 적격 **65 설계
stdout+진단 인터리브 스트림 동일**(거부 breakdown 도 핀) · 판별 설계 19 + 거부 행 5 + 전용 6 ·
적대 differential **316 설계 `backend: native` 확인 후 전수 비교, Finding 1 외 0 diff** ·
PRE/POST 기본 백엔드 examples 4 **0 diff**(stdout·stderr·exit·VCD) · `--backend native` PRE/POST 6설계 0 diff ·
**뮤테이션 41 중 38 kill**(생존 3 = `push_sorted` 의 `<=`·초기화자 재실행·잘린 트레일러 가드 — 전부
등가이거나 소스에서 구성 불가, 각각 논증을 코드에 적었다. 그리고 `arm_t0` 의 드레인은 **중복임을
측정으로 확인**해 그렇게 적었다 — 워크가 문장마다 드레인하므로 초기화자 본문은 이미 덮인다).

#### 4.5.297 ③층 S1d-4c-2b — 바디 워크: **차분이 실행한 것의 과반을 못 보고 있었다** (2026-08-04, branch feat-tier3-s1d4c2b, format 26 불변) ✅

그라운딩이 4c-2 를 또 쪼갰다: **리전 큐를 짓기 전에 실행할 바디가 없다**(원래 "4b 바디 워크"가 dispatch
seam 으로 바뀌며 건너뛰어져 있었다). 4c-2b = **바디 워크** — ③층이 문장이 아니라 **프로세스를 처음
실행**한다. `run_process` 의 일부 크기인 이유는 영리해서가 아니라 **S0 게이트**다(fork 자식·join
배리어·콜스택이 엔진 루프의 대부분인데 전부 거부) → 두 단순화가 **load-bearing** 이라 거부 terminator 를
`_` 가 아니라 명시 arm 으로.

**⭐ 뮤테이션 10 중 5 생존, 전부 하네스 갭**: 실행 가능한 코퍼스 바디가 `Goto`/`Branch` 를 결정 지점으로
안 밟고 · 게이트가 **스토어만** 비교해 `k_rearm`/`enter_body`(넷을 안 쓴다)가 안 보이고 · `fresh_state`
가 **`SimOpts` 를 적용하지 않아** `enter_body` 가 설치할 게 없고 · step guard 예산이 반복 수보다 훨씬
작아 **`+1` 이든 `+100` 이든 똑같이 Fatal**.

**⭐⭐ 그리고 리뷰가 차분이 실행 대상의 과반에 눈멀었음을 실측했다** — 이 바디들이 돌리는 83 문장 중
**46 이 NBA** 인데 NBA 의 효과는 전부 큐 push 라 스토어 비교로는 하나도 안 보인다(**모든 NBA 를 버리는
뮤테이션이 세 테스트를 전부 통과**). 게이트 자신의 주장이 입력 과반에 대해 거짓이었다 → 큐를 항목별로
비교하고 **apply 후** 스토어를 본다.

**⭐ 전제조건이 워크와 다른 진입점을 검사했다**: `body_is_suspend_free` 는 `processes[proc].entry` 에서
스캔하는데 워크는 **호출자가 준 entry** 에서 시작한다(그 파라미터의 존재 이유가 재개인데도).
`disable <named block>`(`disable_fork` 행이 안 덮는다 → 적격·빌드가능)이 entry 에서 도달 불가한 블록에
`Delay` 를 놓아, 술어가 "정지 없음" 이라 답하고 워크는 **호출자가 통과한 검사를 탓하며** 패닉했다.

**⭐ step guard 의 네이티브 절반이 침묵이었다**: `k_mark_fatal` 이 **아무도 안 읽는** 로컬 플래그만
세웠다(엔진은 `RunBodyStepLimit` + exit class 비트). 가드의 가치가 곧 보고인데 보고가 없었다 → 엔진의
`mark_fatal` 호출 + 게이트가 양쪽 `had_fatal` 단언.

**기록(수정 대신 정정)**: `call_fatal` 체크는 게이트가 받는 클래스에서 **발화 불가**(latch 지점이 전부
frame 기계 → `func_table` → 아레나 거부)라 "커버한다" 던 테스트 doc 이 틀렸다. 코드는 **남긴다** — 루프가
제네릭이고 `K = Scheduler` 에는 load-bearing(리뷰어가 엔진을 이 워크로 라우팅해 뮤턴트가 자기 fatal 을
지나쳐 출력하는 것을 실측). 그 밖에 `vm_run_body` 가 여전히 `enter_body` 를 직접 불러 "한 철자" 가
**세 호출부 두 철자**였던 것 · `k_max_deltas` doc 이 델타 한도를 말하는데 두 구현자 모두
`max_body_steps` 를 돌려주던 것(**round-25 의 그 혼동을 문서로 적어 둔 것**) · 거짓 doc 4건 정정
("실행 가능한 코퍼스 바디는 전부 단일 블록" — 67 중 8 이 다중).

**측정**: 코퍼스 **72 중 30** 만 이 워크가 돌릴 수 있는 프로세스를 갖는다(나머지 42 는 모든 바디가
어딘가에서 정지) → 그 숫자를 핀으로 박아 커버리지가 절반 이하임을 보이게 했다.

**게이트.** 5128 green · clippy · fmt · format 26 불변 · PRE/POST 10설계 0 diff · 적대 differential 이
**491 job 0 diff** + **엔진을 이 워크로 전면 라우팅**(picorv32 542,907 진입)해 스위트 동일·**378 job
0 diff** · iverilog 오라클 일치.

#### 4.5.296 ③층 S1d-4c-2a — `k_rearm`: **재진술이 부분이었다** (2026-08-04, branch feat-tier3-s1d4c2a, format 26 불변) ✅

마지막 미배선 메서드. **`Kernel` 표면 미배선 0** — 단 그것이 백엔드 동작을 뜻하지는 않는다(모듈 독에
명시). 메서드는 `match` 하나지만 내용 전부가 **비대칭**: `Edge`/`Initial` 은 재등록하면 안 되고(엣지
등록은 소비 아닌 읽기 → k번째 엣지에서 **2^k 발화**) `Comb`/`Latch`/`Level` 은 반드시 해야 한다(웨이터가
발화 시 소비). 어느 쪽이 뒤집혀도 **값은 안 틀리고** 프로세스가 너무 자주 뜨거나 아예 안 뜬다.

**⭐⭐ 첫 게이트가 이름과 doc 으로 differential 을 주장하면서 하드코딩 기댓값과만 비교했다** —
`Scheduler::rearm` 을 한 번도 안 불렀다. 진짜로 지었더니 **즉시** `rearm_level` 이 `arm_sensitivity` 의
**부분 재진술**임을 잡았다: 엔진은 읽기 집합이 비면 **아무것도 등록하지 않는데**(`if !nets.is_empty()`)
커널은 무조건 arm — `always_comb o = 1'b0;`(적격·빌드가능)에서 발산. 지금 안 보이는 이유는 그
프로세스가 `net_to_level` 에도 없어서인데, **그건 설계가 아니라 우연**이고 4c-2 의 `busy`/quiescence 가
이 상태를 직접 읽는다.

**⭐ 그리고 그 수정이 다른 뮤테이션을 가렸다** — 새 가드를 sensitivity **kind 에도** 걸었더니 Edge 가
**두 이유로** arm 불가가 되어 "커널이 Edge 를 arm" 뮤테이션이 보이지 않게 됐다. **질문 하나에 조건
하나**로 분리(가드=읽기 집합 유무 · match=이 종류가 재등록하나). **`Latch` 는 커버가 0**이라
do-nothing arm 으로 옮겨도 전 패키지 통과(코퍼스는 `Comb`/`Edge` 뿐) → 개수 floor 를 **종류별 이름
단언**으로 교체. `Initial` 은 **동등 뮤턴트**임을 코퍼스로 측정해 기록(초기 블록은 읽기 집합을 안 갖는다).

**⭐ 리뷰가 differential 이 한쪽뿐임을 잡았다** — 엔진 관측자가 Level 웨이터만 보므로 **엔진의** `rearm`
이 Edge 를 재등록하는(=바로 그 2^k 버그) 뮤테이션이 통과했다. 양쪽 **엣지 등록 수**를 재-arm 전후로
비교하게 해서 kill.

**⚠️ 기록(미수정)**: 4c-1 패턴 반복 — `k_rearm` 이 loud → **아무도 안 읽는 쓰기**. 게다가 인터프리터는
`sched.rearm` 을 **직접** 부르므로(트레이트 경유 아님·VM/JIT 만 `k_rearm` 도달) 이 구현만으로는 네이티브
경로가 재등록을 얻지 못한다. `WakeTable` 은 reset 이 없고 t0 상태가 `kind` 파생이라 t>0 생성 시 어긋난다.

**게이트.** 5124 green · clippy · fmt · format 26 불변 · 엔진 추가 2 메서드는 `#[cfg(test)]` 라
프로덕션 바이너리에 **심볼 없음**(nm 확인) · PRE/POST 10설계 0 diff · 적대 differential 이 **1980 비교**
(one-shot·staged·cross-tree·probe·obs·3백엔드) **0 diff** + 28 설계로 엔진 자체 비대칭 확인.

#### 4.5.295 ③층 S1d-4c-1 — NBA 드레인: **delayed 버킷에 한 번도 안 닿고 있었다** (2026-08-04, branch feat-tier3-s1d4c1, format 26 불변) ✅

4c 는 리전 큐·델타·in-body 웨이터·`busy`·`flush_postponed` 를 다 담고 있어 **쪼갰다**(스케줄러를 한
슬라이스에 안 짓는다는 반복 교훈). 4c-1 = **드레인**: `delayed_nba` 시간 버킷 + `k_schedule_nba_at`
(미배선 2 중 하나) + `apply_nba`. 쓰기는 이미 차분 검증된 S1c 퍼널이라 새것은 **순서와 버킷팅**뿐.

**⭐ 뮤테이션 9 중 4 생존, 원인은 하나: `#0` 는 `k_schedule_nba_at` 에 도달하지 않는다**(`d > 0` 만
그 경로). `#0` 로 쓴 설계들이 delayed 버킷을 한 번도 안 채워 트랜스포트 뮤테이션 넷이 **공허 통과**.
실제 지연 + **틱 단위 드레인**으로 셋 kill. 넷째(`seq` 정렬)는 **구조적 도달 불가** — 델타 루프가
없으면 버킷이 **비어 있지 않은** same-tick 큐에 병합되는 순간이 없어 정렬이 항상 no-op. 그 구성을
직접 지었고 테스트가 **"병합된 큐가 실제로 seq 역순인지"를 먼저 단언**하게 해 공허화 차단.

**⭐⭐ 그리고 리뷰가 정렬이 프로덕션에서도 죽어 있음을 실측했다** — **엔진**의 `sort_by_key` 를 빼도
전 패키지 초록이다(런 루프가 시간 진행 前 NBA 를 비우므로 버킷은 **항상 빈 큐**를 확장하고 버킷 내
push 순서는 이미 seq 오름차순). 규칙은 IEEE 의 것이고 4c-2 가 도달 가능하게 만들지만, **동작하는
메커니즘이라고 쓴 주석이 틀렸다**. 추가 실측 둘: `now + ticks` 가 **`ticks` 와 구별 불가**였다(모든
하네스가 `now == 0`) — 프로덕션이면 **`now` 미만 키에 파일 → 영원히 안 드레인 → 조용히 소실** ·
**CONCAT 목적지 트랜스포트가 무커버리지**라 `NbaLhs::of` 를 단일-청크 arm 으로 강제해도 전 스위트
생존(same-tick 쌍둥이는 바로 그 hazard 때문에 존재한다).

**엔진 변경 1건, 그리고 내가 처음 쓴 것의 정반대다.** due 트랜스포트 이동을 `#[cfg(test)]` 헬퍼로
두고 주석에 *"테스트에서 재현하면 순서 규칙의 두 번째 철자가 된다"* 고 적었는데 — 런 루프가 인라인
`delayed_nba.remove` 를 그대로 갖고 있었으므로 **그게 바로 두 번째 철자였다**. 런 루프가 헬퍼를
부르게 해서 그 이동이 바뀌면 ③층 게이트도 같이 바뀐다.

**⚠️ 기록(미수정·사다리 하강)**: `k_schedule_nba_at` 이 loud 패닉 → **아무도 안 드레인하는 조용한
enqueue** 가 됐다(4c-2 까지). 엔진은 "미래 작업 있는가"를 **Scheduler 의** `delayed_nba` 로 정하는데
네이티브 런에서 그 맵은 **비어 있다** → 트랜스포트가 유일한 대기 작업인 설계는 **quiescent 로
보고되고 업데이트가 버려진다**. 4c-2 가 읽을 필드에 의무를 적었고, 살아남은 `k_rearm` 가드가 그걸
**안 덮는다**는 사실(VM/JIT 에서만 호출·VM 은 트랜스포트를 아예 거부)도 함께.

**게이트.** 5122 green · clippy · fmt · format 26 불변 · PRE/POST 10설계 0 diff · 적대 differential 이
one-shot 52 + staged 78 **0 diff** 이고 이 코드가 주장하는 NBA 의미(seq 순서·트랜스포트 인터리브·
스케줄 시점 인덱스 샘플·`#0` 는 트랜스포트 아님·concat/part-select 목적지·중첩 활성화·`$finish` 이후
지연)를 **iverilog 46 설계**로 검증.

#### 4.5.294 ③층 S1d-4b-2 — dispatch 배선: **포맷터를 배선한 게 절반이었다** (2026-08-04, branch feat-tier3-s1d4b2, format 26 불변) ✅

`k_dispatch_systask`·`k_sformatf` **구현**. 엔진은 `&mut Scheduler` 와 `sched.st` 재대여를 동시에 못
넘기므로 리더를 **대체 스토어 `Option`** 으로(=`None` 이 엔진 자신의 상태 → 모든 arm 이 기존 호출로
환원). 커널은 `&mut Scheduler` 를 든다(출력 싱크·파일 테이블·assertion 사이드테이블 · `sched.st` 는
여전히 `now`/`rng`/`timeformat` 의 유일 출처).

**⭐⭐ 두 리뷰가 서로 다른 방향에서 같은 갭에 수렴했다: 포맷터를 배선한 것은 태스크를 배선한 게
아니다.** 태스크 **자기 인자**는 포맷 엔진을 안 거치는 넷 읽기다 — `$fdisplay` 의 **fd** 가 넷이면
손대지 않은 엔진 스토어를 읽어 X 를 얻고 **줄을 통째로 버리며** bad-descriptor 경고를 냈다(`run.json`
이 eligible+buildable 이라 보고하는 설계에서). `$timeformat` 인자도 같은 모양이고 **거부로는 못
막는다** — `Display` + `timeformat_stmts` sid 로 낮춰지므로 **task id 로 키를 잡는 게 구조적으로
틀렸다**. 둘 다 `eval_task_arg`(render 헬퍼의 non-formatter 쌍둥이)로 배선. ⚠️ **내 모듈 독이
`$fdisplay` fd 를 wrong-store 목록에 적어 놓고 거부 목록에서는 빠뜨렸다**(§4.5.291 과 같은 형태).

**`$monitor`/`$strobe` 는 거부** — dispatch 는 **등록만** 하고(ExprId 캡처) 렌더는 `flush_postponed`
에서 일어나는데 그건 이 seam 이 안 닿는다(t0 한 줄 찍고 **다시는 안 뜬다**). 거부는 8 이 아니라 **10**
이고 전부 `eligible: true, buildable: true` — 그래서 설계 게이트가 아니라 **여기**가 거부할 자리다.

**⭐ 새 게이트의 anti-vacuity 가 틀린 방식이 기록할 값이 있다**: 비교 상대를 **세 번째 상태**로 지었다
(이미 advance 된 rng 재추첨 · 분포 설정도 다름 · A 의 유일한 사본을 덮어씀) → A 에 대해 **아무것도
인증 못 했고** 핀 주석의 인과 설명이 성립 불가였다. 진짜 A 로 고치자 인증 수가 **285 → 256/288** 로
떨어졌다(나머지 32 는 양쪽 다 `x` 로 렌더되는 X-heavy 패스 = 설계의 참인 사실). 4b-1 은 제대로 했는데
4b-2 가 두 반쪽을 다 흘렸다.

**⭐ 첫 게이트 뮤테이션 4개 생존, 전부 "코퍼스에 그 형태가 없다"**: `$write`(생성 설계에 없음) ·
`$fdisplay`(**다른 task id** 라 필터가 안 모음) · 거부 arm(미시험) · **severity**(`$error` 는
`Display`+`severities` sid 인데 하네스가 그 테이블을 안 깔아 `run_severity` 미진입). 넷 다 닫음.
그리고 `k_sformatf` 가 non-`SysFunc` rhs 에 **패닉**(엔진은 빈 문자열) — 양쪽 도달 불가지만 **두
구현자가 한 입력에서 갈리는 것**은 이 설계가 불가능하게 만들어야 할 바로 그것.

**게이트.** 5120 green · clippy · fmt · format 26 불변 · PRE/POST 10설계 0 diff · 적대 differential 이
엔진 바디 **기계적 동일성**까지 증명(선언한 5 사이트 치환 후 18390 == 18390) + 32 설계 · 590 랜덤
사이트 · staged 아티팩트 · 3백엔드×6플래그 · perf 무변화.

#### 4.5.293 ③층 S1d-4b-1 — 포맷 엔진의 리더를 파라미터로: **seam 은 문제의 절반이었다** (2026-08-04, branch feat-tier3-s1d4b, format 26 불변) ✅

4a 가 남긴 유일한 CORE 미배선 `k_dispatch_systask` 의 블로커는 **`builtins` 가 `&SimState` 로
렌더한다**는 것. `format_args_str_with(st, nets, …)` 로 리더를 파라미터화하고
`render_template`/`next_arg_with` 로 내려보냈다. **기존 진입점은 `st` 를 넘기는 리터럴 forward** 라
엔진 호출부가 하나도 안 움직였다 = byte-identity 가 구조적. **`&dyn` 이 아니라 제네릭**: `EvalCtx.nets`
는 **넷 접근마다 여러 번** 호출되므로 트레이트 객체면 ①②층 전부가 ③층의 seam 비용을 낸다(실측: 40만
`$display` 설계 wall-clock 변화 없음).

**⭐ 첫 게이트가 완전히 공허했고 뮤테이션 4개가 증명했다.** 두 스토어를 미러링해 놓고 "두 렌더가 같다"를
단언했는데, **스토어가 같으면 리더 인자를 무시하는 구현도 같은 문자열을 낸다** — **같음으로는 출처를
시험할 수 없다.** 스토어를 일부러 어긋나게(SimState=A · arena=B) 하고 arena 리더 렌더가 **B 의 렌더와
같아야** 한다로 재구성. 같은 원칙으로 **테스트가 못 죽이는 파라미터 셋은 되돌렸다**
(`expr_const_string`·`str_const_of_expr`·`arg_string` 은 `Expr::Const` early-return 이라 넷을 못 읽는다
— 못 죽이는 파라미터는 진짜 갭과 구별이 안 된다).

**⭐⭐ 그리고 리뷰가 seam 이 문제의 절반임을 실측했다.** 포맷터는 `now`·`cur_time_mult`·`rng`·
`timeformat`·`global_prec_exp`·`cur_scope` 도 `&SimState` 에서 읽는데 `NativeKernel` 이 **앞 셋의 자기
복사본**을 들고 있었다 → dispatch 를 배선하는 순간 `$time`·`%t`·`$random` 이 **넷과 다른 시계·다른
스트림**에서 나온다(`eligible: true, buildable: true` 로 실측된 설계에서 · 컴파일 에러가 아니라 **틀린
한 줄**). 수정 = 커널이 **`SimState` 를 빌린다**(복사 0 · `nets` 만이 다를 수 있는 필드). ⚠️ 그리고
`now`/`cur_time_mult` 는 첫 주석이 주장한 **"cold" 가 아니다** — 런 루프가 매 타임스텝, 프로세스 디스패치가
매 활성화마다 다시 쓴다. **이건 4a 가 이미 산 교훈이다**(4a 는 t=0 에서 안 보이던 것 때문에 패스마다
`now`/`cur_time_mult` 를 바꿨다) — **한 슬라이스 뒤에 같은 축에서 안 적용했다.**

**리뷰 나머지**: 집계 anti-vacuity → **사이트별**(한 사이트가 조용히 st_a 를 읽어도 다른 사이트가 가린다) ·
`$monitor`/`$strobe` 는 **0 사이트 수집**이고 애초에 `flush_postponed` 에서 렌더하므로 필터에서 제거 ·
못 죽이는 arm 이 **둘이 아니라 셋**(런타임 string trailing arm — 목록이 막으려던 실패를 목록 안에서 범함) ·
**렌더 사이트 열거가 또 양방향으로 틀렸다**(`$error`/`$fatal` 은 `dispatch.rs` 밖 `run_severity` 에서
렌더 · 정작 적어 둔 `$sformat` 사이트는 gate-refused).

**기록(미수정)**: `NetArena` 의 `fd_eof` 기본값은 **X-poison 구멍**인데 지금은 `$feof` 과잉표시가 가리고
있다 → 그 과잉표시를 고치는 사람이 override 를 빚진다 · `?Sized` 는 아직 **load-bearing 아님**(없어도
컴파일) — 4b-2 의 `k_nets()` 가 `&dyn` 을 돌려준다.

**게이트.** 5115 green · clippy `-D warnings` · fmt · format 26 불변 · PRE/POST **10 설계 stdout+VCD
바이트 동일**.

#### 4.5.292 ③층 S1d-4a — `impl Kernel`: 스텁하면 안 되는 절반은 술어다 (2026-08-03, branch feat-tier3-s1d4a-impl, format 26 불변) ✅

**52 를 실제로 구현하니 분해가 셋이 아니라 넷이었다** — ① 스토어 코어 13 · ② **분류 술어 17** ·
③ 게이트 거부 워커 18 · ④ **미배선 4**. ②가 계획에 없던 축이고 이 슬라이스의 결정이다: `k_*_rhs`
는 값을 만들지 않고 **`compute_effect` 가 어떤 문장을 짓는지** 정한다. 거부 가족이라고 `false` 로
스텁하면(가장 자연스러운 "못 온다" 처리) 그 문장은 **loud 해지지 않고 다른 문장이 되어** pure-eval
로 조용히 흘러간다. 그래서 술어는 전부 진짜로 답하고 **`exec::kpred` 로 엔진과 한 철자** — §4.5.291
교훈("쌍둥이는 쌍둥이의 성질을 상속하지 않는다")을 **강제 가능한 형태**로 바꾼 것이다(두 번째 철자를
안 쓰면 갈릴 수 없다).

**⭐ 두 리뷰 렌즈가 같은 뿌리에 수렴했다: 거부의 종류가 둘인데 하나로 썼다.** 18 은 게이트가 거부하고,
**4(`k_dispatch_systask`·`k_sformatf`·`k_schedule_nba_at`·`k_rearm`)는 적격 설계가 도달**한다 —
막는 것은 `native::runtime_gate` 의 VM 선택이지 설계 게이트가 아니다. `$sformatf` 는 그냥 오해가
아니라 **구멍**이었다: 정본 `sysfunc_is_stmt_effect` 가 일부러 `false` 라 어떤 행도 안 막는데,
**테스트의 admission 필터까지 그 술어를 써서** 커널이 패닉하는 설계를 워크에 들일 뻔했다.
**tier-2 는 같은 구멍을 이미 겪고 이유까지 적힌 명시 delta 로 막아 뒀는데 그걸 안 베꼈다.** →
`kpred::rhs_routes_to_worker`(`compute_effect` 가 실제로 분기하는 arm 의 논리합)로 교체 · `panic!`
매크로를 **`gate_refused!` / `not_built!` 둘로** 분리.

**⚠️ 그리고 내가 규칙을 옮겨 적었다.** `k_delay_ticks` 를 기억으로 재진술해 네 절 중 둘(X/Z 가드 ·
`u64::MAX` 포화)이 사라져 **무한 지연이 t+0 에 발화**했다 — 지연량은 저장된 비트가 아니라 값 비교로
안 보인다. 수정은 더 잘 옮겨 적기가 아니라 **`eval::delay_ticks_of` 공유**(`resolve_offsets` 옆).
`max_body_steps` 기본값 `u64::MAX` 는 "의견 없음"이 아니라 **종료 가드 없음**이라 생성자 인자로.

**⭐⭐ teeth 가 처음엔 절반이었다 → 그리고 리뷰가 뮤테이션이 못 보는 다섯을 더 찾았다.**
17 뮤테이션 중 **5 생존**: 컨텍스트 폭 규칙 + `compute_effect` 가 assign 에서 **안 부르는 표면 넷**
(진입 0회). control/VM 워크·폭 설계군·`class_new_sites` 주입으로 kill, 남은 둘(`prec_mult`·`now`)은
**코퍼스에 형태가 없어서**라 전용 설계로. 리뷰 추가분: **NBA 목적지를 한 번도 비교 안 했다**(문서는
비교한다고 적혀 있었고, 하필 그게 파일 유일의 손-재진술 함수 `k_schedule_nba_scalar` 를 지키는
필드였다 · `NbaLhs` 가 아무것도 derive 안 해서 구조 분해로 비교) · **`two_state` 를 엔진 쪽에 안
깔아** 아레나 퍼널이 지킨 두 arm 중 하나가 커버리지 0 · 합산 floor 가 control 워크의 반쪽 소실을
못 봄 · classify 의 집계 floor 는 **한 가족 스텁을 통과**(→ 가족별 단언) · `catch_unwind` 가
**아무 패닉이나** 수용(→ 메시지 검사).

**⚠️ 4b 크기 추정이 3× 이상 낮았다(리뷰 실측).** "정확히 4 read site" 는 grep 이 철자 셋만 봐서
`$timeformat` 인자·`$dumplimit`·`$fdisplay` fd·`$fclose`·**`$writemem*`(메모리 자체를 읽는다)** 를
놓쳤다. "쓰기 0" 도 넷 **값**에만 참 — `$dumpvars` 는 `st.nets[i].vcd_id` 를 쓰고 `arena::Slot` 엔
그 필드가 없다(**S1d-4d 의 VCD 바이트 게이트가 정면으로 만난다**). 정정 + "이름 목록이 아니라
**스토어 쪽에서** 재측정하라"를 모듈 독에 박음.

**부수 슬라이스(별도 커밋 `bee1371`) — pre-existing silent-wrong 2건.** ① u32 초과 CA delay 가
**랩**(`assign #5000000000` → t=705032704, **7.09× 조기**, errors=0): 폴드가 세 갈래 중 둘은 이미
포화인데 정수 갈래만 `const_eval_u32`(하위 32비트)를 거쳐 **포화 전에 잘렸다**. ② **음수 real
delay 가 즉시 발화**(음수 정수는 안 함 — 한 함수 두 답, iverilog 는 둘 다 안 함). ⭐ **첫 수정이
반만 맞았고 측정이 잡았다**: raw 곱의 부호로 판정하니 `#(-1e-9)` 가 죽는데 iverilog 는 **0 에
발화**한다 → 경계는 **반올림한 값**. 3-way 50 설계 × 5 타임스케일 = **POST vs iverilog 0 diff**.
⚠️ 잔여 기록: u32::MAX 초과 delay 는 **여전히 clamp**(표현=format bump · 보고=새 W-code · ROADMAP §2).

**게이트.** 5111(슬라이스) → **5113**(부수 포함) green · clippy `-D warnings` · fmt ·
format_version 26 불변 · 적격률/백엔드 선택 불변(examples 4/4 eligible · backend=vm).

#### 4.5.291 ③층 S1d-4a — 트레이트가 강제한 결정: 퍼널을 안 거치는 효과를 게이트가 답한다 (2026-08-03, branch feat-tier3-s1d4a-kernel, format 26 불변) ✅

**계획을 먼저 고쳤다(별도 커밋 `ceb63e3`).** S1d-4 를 "두 번째 실행기를 쓴다"로 잡고 있었는데
엔진이 이미 그 이음매를 **의도적으로** 만들어 뒀다(`exec/mod.rs`: *"`apply_effect` 의 커널 호출이
P7b 의 트레이트 표면이 된다"*). 실측: `compute_effect`/`apply_effect` 는 `K: Kernel` **제네릭**이라
문장 의미가 전부 재사용되고 byte-identity 가 **구조적**이 되는 반면, `run_process`(바디 워크)와
`builtins::dispatch` 는 **양 끝이 `Scheduler` 고정**이다 — 이음매는 **가운데만** 제네릭이다.
그리고 52 메서드가 셋으로 갈린다(코어 ~16 · 게이트 거부 ~9 · **~27 = 그 선결 과제**).

**그래서 이 슬라이스의 실물은 그 선결 과제의 종결이다.** `Kernel` 을 구현하면서 27개를 답하지 않을
방법이 없으므로 *"나중에 배선"* 이 더는 선택지가 아니다 → **`stmt_effect` 거부 행**을 추가하되
판정은 **tier-2 와 같은 술어**(`sim_ir::rhs_is_stmt_effect`)로. 두 철자면 두 백엔드가 같은 문장을
두고 갈릴 수 있다.

**⭐ 처음으로 적격률 숫자가 움직였다: 79/79 → 77/79.** 잃은 둘은 keccak 두 형태이고 이유는 하나 —
TB 의 `$value$plusargs("N=%d", nperm)` 가 `nperm` 을 **호출 안에서** 쓴다. 무시하면 초기화 안 된 값으로
도는 조용한 오답이므로 거부가 정답이다. ⚠️ **숫자보다 무거운 결과**: keccak 이 빠지면 ③층을 실제로
보일 설계가 examples 4 + picorv32 로 줄고, **②→③ 76×** 와 **§7.3 성공 기준**이 둘 다 keccak 위에
서 있다 → `$value$plusargs` 배선은 편의가 아니라 **재측정 게이트의 선행조건**(doc-21 §7.3.1 기록).

**적대 리뷰 — BLOCKING 1 + MAJOR 3, 전부 실질.** ① **`$cast` 의 TASK 형이 빠져 있었다**: 그것은
`$sformat` 과 **같은 메커니즘**으로 목적지를 쓰는데(리뷰어 실측: 적격 + reject 맵이 **비어 있음**)
내가 SysTask 쪽을 **3-id `matches!`** 로 써서 암묵 catch-all 로 흘렸다 — **내 주석이 여섯 줄 위에서
"`_` arm 없는 exhaustive 라 새 id 가 조용한 쪽으로 기본값이 될 수 없다"고 주장하는 동안**. 그 주장은
`sysfunc_is_stmt_effect` 에만 참이었다. 수정 = sim-ir 에 정본 `systask_net_write` 를 **`_` 없이**
신설(41 변종 전수) → `$cast` 포함. ② 그리고 그것이 **double-booking** 을 드러냈다(기존 테스트가 잡음):
heap 뮤테이터도 넷을 쓰지만 그 설계는 이미 storage 행이 거부한다 → 술어가 **`NetWrite{None,Flat,Heap}`**
을 답하게 하고 게이트는 **Flat 만** 센다(내 코드가 `*_dyn_nets` 에 대해 이미 지키던 규칙). ③ 주석 둘이
코드와 **반대**를 말하게 됐고(`write.rs` 가 *"`r=$random(seed)` 는 오늘 적격"*) ④ 최초 측정표의 keccak
✅ 두 줄이 stale — 전부 수정. teeth 도 보강: 3 id 중 **`$readmemh` 하나만** 밟히고 있었고 음성
케이스가 **상수 rhs**라 "순수 SysFunc 는 false" 를 증명 못 했다 → `$sformat`·`$readmemb`·`$cast` +
**순수 SysFunc 음성**(`$clog2`·무시드 `$random`/`$urandom`).

**기록(별도 슬라이스)**: `$feof` 가 정본 술어에서 **과잉표시**(`k_feof` 는 순수 읽기인데 true) →
`e = $feof(fd);` 는 거부·`while (!$feof(fd))` 는 통과. 한 소비자에서만 고치면 철자가 둘이 되므로
**정본을 고쳐야 하고 그건 tier-2 게이트도 넓히는** 별도 슬라이스(ROADMAP §5.1).

**게이트.** 전 스위트 **5103 green** · clippy `-D warnings` · fmt · format_version 26 불변.

#### 4.5.290 ③층 S1d-3 — wake 결정, 그리고 **게이트가 수정을 거부하도록 굳어 있던** 일 (2026-08-03, branch feat-tier3-s1d3-wake, format 26 불변) ✅

**무엇.** S1d-2 가 만든 변경 집합을 소비해 **어느 프로세스가 ready 가 되고 어떤 순서인가**를 답한다.
값이 아니라 **결정**이므로 게이트도 결정을 비교한다 — 엔진 `propagate_changes` 의 pass (a)(정적 엣지)와
pass (b) 의 `arm=None` arm(정적 레벨)이 오라클이고, Active 큐의 **델타**가 비교 대상이다.

**적격성이 사준 단순화**(문서에 명시): fork 가 S0 거부라 activity ≡ process 1:1 · `tie == proc` ·
`Ready` 가 proc id 하나로 붕괴 · `push_sorted` 의 tie 순서가 오름차순 proc 로 붕괴. clocking 거부라
`commit_clocking` 인터셉트도 없다. **둘 중 하나라도 열리면 두 단순화가 곧바로 load-bearing 이 된다.**

**게이트가 구현 갭을 두 번 잡았다.** ① 첫 실행에서 `always @(a)` 가 발산 — 그건 `SensKind::Level`
이고 엔진은 pass (b) 로 깨우는데 내 테이블은 Edge 만 등록했다. ② **적대 리뷰가 더 큰 것을 찾았다**:
`arm_sensitivity` 는 `Level | Comb | Latch` **셋 다** 같은 `arm=None` 웨이터로 만든다 —
`always_comb`/`always_latch`/`@(*)` 가 elaborate 가 추론한 read-set 을 같은 `edges` 에 담기 때문이다.
**즉 조합 프로세스 전체가 영구히 잠들어 있었다**(corpus 자체 템플릿 `gen_comb_chain` 이 바로 그 모양).

**⭐⭐ 그리고 게이트가 그 수정을 거부하도록 굳어 있었다.** 등록만 추가하면 테스트가 **깨진다** —
`arm_processes` 는 Comb/Latch 를 t0 에 **arm 하지 않고 Active 에 넣기** 때문에 `level_armed` 초기값이
전부 true 이면 안 되고 **`kind == Level` 인 것만** true 여야 한다. 두 줄이 짝이었고, 한 줄만 고치면
게이트가 그것을 회귀로 신고한다. 리뷰어가 그 짝을 실측으로 확정해 왔다.

**관측 granularity 는 선택이 아니라 축이다(§4.5.289 의 교훈이 여기서 확장됐다).** teeth 검증에서
쓰기마다 관측하면 델타당 변경 넷이 1개라 **순서 계약·level consume 이 도달 불가**, 배치로 관측하면
클럭이 반복 기록돼 마스크가 all-bits 로 누적돼 **mask 규칙이 무의미** — **서로를 가린다.** 둘 다
쓸어야 한다. 리뷰가 여기에 세 가지를 더했다: **dedup 리셋을 pass 당 1회로 두어** level+edge 동시
발화가 **0회**였고(순서 계약용으로 쓴 설계 둘이 그 주석대로 동작하지 않았다), **모든 쓰기에 저자를
붙여** 프로덕션의 **흔한 경우**(NBA·settle·clocking = 무저자 `u32::MAX`)가 한 번도 비교되지 않았으며,
granularity 와 상태 프로파일이 **같은 술어**를 공유해 행렬의 절반이 안 쓸렸다. 전부 수정 —
최종 **8개 규칙 전부 teeth**(하나씩 깨면 실패).

**기록(수정 안 함)**: ⚠️ `busy` 는 **적격 설계에서 실제로 참이 된다**(리뷰 실측:
`always @(posedge clk) begin … @(negedge rst); end` 이 S0 적격·아레나 빌드 가능이고 엔진의 busy 가드가
wake 를 막는다) — 유지자는 S1d-4 의 몫이고 같은 설계가 in-body 웨이터 모델도 필요로 한다.
그리고 **`NetSlot.prev` 는 워크스페이스 전체에서 읽는 곳이 0**(리뷰가 필드명 변경 빌드로 증명) —
엔진 핫 루프의 `clone_from` 2회/변경넷/델타가 죽은 일이다(ROADMAP §5.1 에 별도 슬라이스로 등재).

#### 4.5.289 ③층 S1d-2 — dirty/edge 채널, 그리고 **내 게이트의 teeth 를 세 번 재확인**한 일 (2026-08-03, branch feat-tier3-s1d2-dirty, format 26 불변) ✅

**무엇.** 스케줄링의 절반은 루프가 아니라 **쓰기에 달려 있다** — 엔진은 그것을 정확히 두 지점
(저장된 워드가 실제로 바뀌는 곳)에서 유지하고, S1c 쓰기 퍼널은 그 두 지점을 채널 없이 재현했다.
`native/dirty.rs` 가 그것을 붙인다: `dirty`(쓰기 순서·**멤버십 자체가 변경 집합**이라 한 슬롯 안의
A→B→A 왕복도 관측된다 — 끝점 비교는 정확히 그것을 놓친다) · `last_blocking_writer`(자기가 쓴 넷에
자기가 재점화되지 않게) · **`slot_edge`**(슬롯 내 bit0 엣지 요약 — 끝점이 잃은 **엣지 종류**를
복원하는 것이고, 값 비교로는 **절대** 볼 수 없다). 엔진의 edge-target 스캔은 **한 철자로 추출**
(`state::edge_target_nets`) — 두 번째 스캔이 func/task arena 를 빠뜨리면 증상은 틀린 값이 아니라
**안 뜨는 posedge** 다.

**⭐ 이 슬라이스의 값은 teeth 검증을 세 번 돌린 것이다.** 게이트를 처음 짰을 때(쓰기마다 채널을
take) 7개 행동 중 **2개가 통과**했다 — 즉 그 두 행동은 **테스트되지 않았다**: ① 매 take 가 dirty
길이를 1로 만들어 **정렬 계약이 공허**했고 ② 같은 슬롯 재기록이 없어 **glitch 누적**이 안 일어났다.
**배치로 바꾸자**(한 델타가 실제로 하는 일) 4/6 이 teeth 를 얻었고, 남은 둘을 위해 ③ `blocking_writer`
를 실제로 세우고 ④ edge-target 넷마다 `0→1→0→1` 을 **퍼널을 통해** 구동하며 사이사이 take(연속
배치의 마스크가 **달라야** cross-batch 독립성이 보인다)를 넣었다. 최종: **7개 행동(glitch reset·
posedge·negedge·anyedge·sort·writer tag·dedup) 전부 깨면 게이트가 실패한다.**

**교훈(ENGINEERING_RULES 병합)**: "차분 게이트를 짰다"와 "그 게이트가 무언가를 검사한다"는 다른
말이다. **각 행동을 하나씩 깨 보고 게이트가 우는지 확인**하기 전까지는 후자를 주장할 수 없다 —
그리고 공허함의 원인은 대개 **관측 시점**이었다(쓰기마다 관측하면 순서·누적이 정의상 사라진다).

**⭐⭐ 그리고 적대 리뷰가 그 규칙을 **내가 덜 적용했다**는 것을 찾았다.** teeth 목록 7개를
`dirty.rs` 의 **개념 셋**에서 뽑았는데, 뽑았어야 할 곳은 `write.rs` 가 열거하는 **두 store 지점**
이었다: **bit-serial 지점은 한 번도 진입하지 않았다**(리뷰어 실측 `ser_enter=0` — 아레나의 정렬
테스트가 `lsb==0 && width==net_w` 로 접혀 whole-element 는 전부 word-parallel 로 가고, bit-serial 은
**비트/부분 선택 lvalue** 가 있어야 들어간다. S1d-2 설계 목록엔 그런 lvalue 가 하나도 없었다).
증거는 결정적이다 — 그 지점의 채널 블록을 **통째로 지워도 패키지 전체가 초록**이었다. 즉 슬라이스가
막겠다던 바로 그 결함(값은 맞고 posedge 가 사라짐)이 **표면의 절반에서 무방비**였다. 수정 = 클럭이
`bus[0]` 인 설계 1개 추가(+부분선택 쓰기) → 이제 그 블록을 지우면 게이트가 운다. 부수 반영: 작성자
태그를 배치 **前**에 세움(뒤에 세우면 그 배치가 보는 태그는 직전 pass 의 잔값이라 non-vacuity 가
우연이었다) · `set_elem` 을 `#[cfg(test)]`(채널을 우회하는 유일한 writer 였고 doc 이 2 슬라이스 전
상태를 말하고 있었다) · `note_change` 단독 호출이 마스크를 **리셋만** 한다는 **호출자 의무** 명문화 ·
그리고 **VCD 는 gate-reject 가 아니다** — S1d-4 게이트가 stdout+VCD 바이트 동일이므로 `note_change`
는 `word` 를 되찾아야 하고 emitter 는 **store 지점**에 있어야 한다(sweep 시점 emitter 는 슬롯 내
A→B→A 를 한 레코드로 합쳐 `slot_edge` 가 지키려던 glitch 를 정확히 잃는다) — `dirty.rs` 에 기록.

#### 4.5.288 ③층 S1d-1 — 백엔드 선택과 런타임 게이트: 판정을 **두 층**으로 (2026-08-03, branch feat-tier3-s1d-gate, format 26 불변) ✅

**S1d 도 넷으로 갈랐다**(S1 분해가 결함 2건을 잡은 그 방식): **S1d-1 배선+런타임 게이트** →
S1d-2 스케줄러 코어+dirty/edge → S1d-3 settle+wired 해소 → S1d-4 바이트 동일 게이트. 이 조각은
**되돌리기 싸고**, 이후 전부를 end-to-end 로 측정 가능하게 만든다.

**무엇.** `Backend::Native` + `--backend native` + `native::runtime_gate`(**설계 게이트 ∧ 아레나
빌드**). 실행기가 아직 없으므로 요청은 **항상 VM 으로 폴백**하고, `simulate` 가 EFFECTIVE 백엔드를
한 번 결정해 `SimResult.backend` 로 싣는다 — run.json 은 **요청이 아니라 결과**를 적는다(요청을
그대로 적으면 **돌지도 않은 실행기를 보고**하게 된다). `scan_arm` 의 강제된 새 arm 은 패닉이 아니라
**레퍼런스 인터프리터**(안전 기본값).

**⭐ 판정을 두 층으로 나눈 것이 이 조각의 값이다.** `native{eligible, buildable, refused}`:
`eligible` = v1 **범위**가 받는가(설계 수준 상한) · `buildable` = 오늘의 **저장소**가 담을 수 있는가
(`NetArena::buildable` — `build` 의 **무할당 쌍둥이**로, `build` 가 그것을 먼저 부르므로 두 답이 갈릴
수 없다) · `refused` = 둘의 AND 가 거부한 이유 또는 `null`. **한 플래그로 접었으면 상한이 능력으로
읽혔을 자리다** — 서브루틴 설계는 eligible 이면서 buildable 이 아니고, 그 격차(79/79 vs 78/79)가
이제 손 계수가 아니라 **run.json 에서 기계로 읽힌다**.

**S1c 리뷰가 남긴 필수 4건 중 2건이 닫혔다**: ① 프레임 로컬(구조적 거부) ② 런타임 게이트 통일.
남은 둘 = 퍼널을 안 거치는 효과 배선 · dirty/edge 채널(+ `warn_run_range` stderr).

**적대 리뷰(soundness+differential 단일 리뷰어) — 핵심 안전 속성은 깨지지 않았다.** "native 를
선택해도 출력 바이트가 안 움직인다"를 **네 사분면 전부**(적격∧빌드가능 / 적격∧빌드불가 / 설계거부 /
fork)에서 stdout+stderr+VCD+exit code 로 실측, **staged 경로**(`vrun`)와 rc≠0 런까지 확인. 결정
지점 1개·`st.backend` 독자 1개·`Backend` 에 `_` fallthrough 0·artifact/해시 노출 0 도 코드로 확인.
**결함은 전부 보고 축**이었고 둘이 실질:

**① MAJOR — 폴백 이유가 run.json 에 안 실리는데 주석 둘이 실린다고 주장했다**(§4.5.287 과 **같은
실패 모드**: 슬라이스가 존재하는 이유인 바로 그 속성에 대한 거짓 주석). 깨끗한 설계에서
`--backend vm` 과 `--backend native` 의 run.json 이 **완전히 동일**해서, 매니페스트만 읽는 소비자는
폴백을 볼 수 없었다(`refused: null` 이 `backend: "vm"` 옆에 있으면 *"막은 게 없는데 vm 이 돌았다"*
로 읽힌다). 수정 = **`backend_requested` 필드 신설**(리뷰어 권고대로 `native.refused` 에 접지 않았다 —
그러면 판정이 `--backend` 에 의존하게 된다) + 주석 둘을 사실로.

**② MINOR — 새 게이트 테스트가 vacuous 였다**(리뷰어가 **측정**: corpus 72 에서 `elig_false=0,
storage_err=0, gate_err=0` — `runtime_gate` 가 `Ok(())` 여도 통과). 게다가 `buildable ≡ build` 는
`build` 가 위임하는 한 **항진명제**였다. 수정 = 세 형태(깨끗/설계거부/저장소거부)로 **모든 arm 을
실제로 밟고** 이유 문자열을 단언 + **arm 도달 카운트를 단언**(non-vacuity) + 위임 teeth 는 **거부
형태에서** 비교(위임을 빼면 그때 터진다). **teeth 검증**: 게이트를 `Ok(())` 로 부수면 새 테스트가
실패한다(옛 것은 통과했다).

부수 반영: `runtime_gate` 를 끼워 넣으며 `design_eligibility` 의 rustdoc 이 **다른 함수에 붙어
있었다**(destructure 를 설명하는 지시문이 그 destructure 없는 함수에) · `refused` 가 **어휘 둘**을
섞는다는 사실 명문화(설계=맵의 키·저장소=맵에 없는 문구) · `--help`/doc 주석의 `<interp|vm>` stale ·
`scan_arm` 의 Native arm 에 `debug_assert`(문서상 도달불가를 **테스트 가능**하게) · 기존 두 테스트를
**세 번째 값까지** 확장(backend-invariance·staged).

**게이트.** `--backend native` 가 **출력 바이트를 안 움직인다**(stdout+VCD, 네 사분면 + staged) ·
runtime gate 가 **모든 arm 에서** 두 반쪽과 일치 · 전 스위트 **5100 green** · clippy `-D warnings` ·
fmt · format_version 26 불변 · schema_ver 1 유지(기존 kind 위 필드 추가).

**기록(수정 안 함)**: staged 경로에는 obs 표면이 아예 없어(`vrun --obs-dir` 는 거부) `vrun --backend
native` 의 폴백은 **어느 표면에도 신호가 없다** — 선재 제약(obs 는 one-shot 전용)이고, 그래서 staged
바이트 비교를 테스트로 못 박았다.

#### 4.5.287 ③층 S1c — 쓰기 퍼널, 그리고 그 게이트가 **내 퍼널의 silent-wrong** 을 잡았다 (2026-08-03, branch feat-tier3-s1c-write, format 26 불변) ✅

**무엇.** `NetArena::write_lvalue` — 엔진 `write_lvalue`→`write_lvalue_general`→`write_chunk`→
`store_words` 사슬의 **Value 수준 미러**(적격 설계가 만들 수 있는 형태 한정). 살아남는 것은 기하:
chunk 폭 · concat LHS 의 MSB-first 소비 · **부호 있는** 최하위 비트 위치(`v[-2+:4]` 가 밑으로 새는
비트만 버리고 부분 기록) · 배열 워드 OOB **드롭**(클램프 아님) · 워드/비트 단위 change 판정.
사라지는 것은 적격성이 이미 답한 질문들(heap·class·assoc·string 레인, frame-local, `forced`).
아레나 원소는 **구성상 워드 정렬**이라 엔진의 정렬 테스트 3항이 "이게 원소 전체인가" 하나로 접힌다.

**⭐ 이 슬라이스의 핵심은 게이트가 잡은 결함이다.** 문장 단위 차분(같은 미러 상태 → 한 문장을
양쪽에서 실행 → `changed` 판정 + 전 넷/원소 읽기 비교)이 **내가 방금 쓴 퍼널에서 silent-wrong 1건**을
냈다: **real 값 rhs 인데 real 넷이 없는 설계**(`x = $itor(n)/2.0` · `$bitstoreal(…)`)는 적격인데,
엔진은 §6.2 대로 **반올림**하고 아레나는 **IEEE 비트를 그대로** 저장했다(32비트에서 1820910942 vs
1465909248). 원인은 코드가 아니라 **주석의 논증**이었다 — 모듈 독이 "real 강제 변환은 사라진다,
`NetKind::Real` 은 아레나를 못 짓는다"고 적었는데 그것은 **목적지** 쪽 답이고, 강제 변환의 조건은
**값** 쪽이다. 한쪽 반만 답하고 분기를 지웠다. 수정 = 도달 가능한 arm(real→int 반올림) 을 엔진에서
**축자 이식**(단일 chunk 는 chunk 폭이 아니라 **넷** 폭·부호로 반올림한다는 세부까지) + 그 형태를
적대 설계로 상시 핀.

**오프셋 해석기를 하나로.** `Scheduler::resolve_lvalue_offsets` 의 본문을
`eval::resolve_offsets<N: NetReader>` 로 옮기고 스케줄러는 위임(기계적 동일). "이 인덱스가 가리키는
비트 위치는 어디인가"의 철자가 둘이면 **X/Z 인덱스를 한쪽은 버리고 한쪽은 쓰는** 발산이 정확히
가장 아픈 자리에서 난다 — 그래서 아레나 퍼널도 같은 함수를 부른다.

**게이트 2행 추가(문장 스캔).** `force`/`release`·`disable` 은 사이드카가 **보고하지 않는다**
(`assign_ranks` 는 절차적 assign/deassign 만) → `ir.stmts` 스캔으로 판정. v1 은 force 기구가 전혀
없으므로 퍼널은 `forced` 플래그를 **일부러 안 든다**(기구 없이 플래그만 들면 "지원"으로 읽히고 모든
`force` 가 조용히 아무 일도 안 한다). **측정: 적격률 불변(79/79)** — 게이트가 엄격해졌는데 잃은 설계가
없다 · 두 행 모두 라이브 설계로 **발화 확인**(vacuous 아님).

**적대 리뷰(soundness) — BLOCKING 0, 그러나 MAJOR 3 이 전부 실질이었다.** ① **`disable` 게이트가
과잉 거부**였다: `disable <named block>`(break/continue 관용구)은 elaborate 가 **진단용 마커 + 형제
`Goto`** 로 낮추고 엔진은 `StmtEffect::Nop` 으로 돌린다(비-lexical 대상은 이미 loud) — tier-3 기구가
**하나도** 필요 없다. 코드를 읽어 확인하고 행을 **`disable_fork` 로 좁혔다**(②층은 한 바디를 잃지만
③층은 **설계 전체**를 잃는다 — 같은 과잉거부의 값이 다르다). ② **퍼널을 통과하지 않는 효과**
(`rhs_is_stmt_effect` 가족·효과 SysTask)를 내 주석이 "v1 기구 없는 문장 가족" 열거로 읽히게 써 뒀다 →
비-exhaustive 임을 명시하고 **S1d 의무 목록에 등재**(거부는 하지 않는다 — 무엇을 배선할 수 있는지가
미정인데 추측으로 거부하면 측정이 반대로 오염된다). ③ **프레임 로컬은 읽기 경로에도 있었다** →
prose 가 아니라 **`NetArena::build` 의 구조적 거부**로(`func_table` 비어있지 않으면 Err) — `Real` 거부와
같은 자리. 부수: `lvalue_width` 가 엔진의 `.max(1)` 을 빠뜨림(수정) · real arm 이 `chunks[0]` 을 무조건
인덱싱(빈 lvalue 에서 원본은 반환·미러는 패닉 → arm 안으로) · assoc `Offsets` 변종 명시 가드 ·
`Offsets` 를 `pub` 로 넓힌 것 되돌림(private 모듈 안이라 아무것도 못 얻는다) · init 마스킹 불변식
`debug_assert` · 리뷰가 지목한 **미도달 3형태**(2-state >64비트·`-:` 워드 경계 횡단·real 의 concat leg)
테스트 추가(228→**270**, 발산 0).

**적대 리뷰(differential) — 기하 미러는 442,202 twinned write 에서 발산 0.** 손으로 지은
`LvalChunk`/`Offsets`(오프셋 `i32::MIN/MAX`·2^30·`w±1`, 접힌 폭 0~4096, 원소 폭 8/12/64/65/66/128
배열, 반복 넷 concat, 모든 degenerate `offset`/`width` 조합)에서 값도 `changed` 도 갈리지 않았다 —
아레나가 **의도적으로 다른** 유일한 곳(엔진의 4항 정렬 테스트를 `lsb==0 && width==net_w` 로 접은 것)도
비정렬 배열 21 설계에서 바이트 등가. 대신 **미러 밖에서 BLOCKING 1건**(아래 별도 커밋) + 게이트
정합성 2건: **① 적격 판정이 `NetArena::build` 가 거부하는 설계를 포함**(`real` 넷) → 게이트에 `real` 행
추가 **② 프레임 레인이 게이트에 안 보였다** — `build_with_opts` 가 `two_state_nets` 만 설치해서 엔진의
`frame_local` 라우팅이 전부 false 였고 **미러가 틀린 이유로 일치**하고 있었다 → `func_table` 설치 +
"적격이지만 아레나가 거부한다"를 **세 사실 한 테스트로** 핀. 그리고 **측정을 두 층으로**(§7.3.1):
적격 79/79(설계 수준 상한) 옆에 **아레나 빌드 78/79**(keccak 호출형 거부)를 같이 적는다 — 게이트가
세 방향으로 정정됐는데 **측정 집합은 안 움직였다**.

**게이트.** 전 스위트 **5097 green** · clippy `-D warnings` · fmt · format_version 26 불변.

**S1d 착수 前 필수 3건을 기록**(preview/21 §5 S1 표 · `native/write.rs` 모듈 독이 정본): ① **프레임
로컬 쓰기** — 호출이 S0 에서 코어라 적격 설계가 frame-local 넷을 가질 수 있고, 아레나에선 평범한
슬롯이 되어 쓰기가 활성화 창 대신 슬롯에 떨어진다(S1d 는 프레임 저장을 짓거나 `func_table` 설계를
런타임 게이트에서 거부해야 한다) ② **dirty/edge 채널**(그 두 store 지점에 달린다 — 없으면 값은 맞고
posedge 가 사라진다) ③ **`warn_run_range` stderr**.

#### 4.5.286 ③층 S1a+S1b — R1 아레나가 서고, 기존 eval 이 그 위에서 돈다: 미러 차분 17,940건 발산 0 (2026-08-03, branch feat-tier3-s1-arena, format 26 불변) ✅

**S1 분해 결정이 이 슬라이스의 절반이다.** 기존 스케줄러는 측정으로 핀된 행동 수십 개를 담고
있어(glitch 엣지 마스크·inertial 세대 취소·edge collapse·waiter arm 스냅샷·self-write 억제 …)
"새 스케줄러 + corpus 바이트 동일"을 한 번에 지으면 검증이 구현을 못 따라간다 → **S1a(저장)
S1b(read-path) S1c(쓰기 퍼널) S1d(스케줄러+배선)** 로 갈라 각각 자기 게이트를 갖게 했다(정본 =
preview/21 §5 S1 분해 표). 또 하나의 재독해: **평가기 교체는 R2(=S2)의 일**이다 — S1 은
"인터프리터 형태" 그대로이므로 S1b 는 새 평가기가 아니라 **기존 `EvalCtx`(제네릭 `NetReader`)
밑에 아레나 read-path 를 꽂는 것**. 평가 의미가 공유되므로 eval parity 는 by construction 이고,
차분이 조준할 표면이 정확히 read-path(원소 인덱싱·OOB→X·top-word 마스킹)로 좁아진다.

**S1a — `native/arena.rs`.** 넷마다 컴파일 시점 확정 `Slot{off,words,width,elems,signed}` —
이 dense 레코드 하나가 flat store 의 per-access 질문(`NetSlot` 메타 + `class_is_handle`→
`frame_local`→`dyn_is_handle` 라우팅 비트맵)을 전부 대체한다. 레이아웃 = 원소당 `(val,unk)`
인접 2 plane, **원소가 워드 정렬**(기존 store 는 비트 연속 패킹이라 비정렬 원소가 bit-serial
폴백을 탄다 — 그 경로 자체가 없다). t0 init 은 엔진 `expand_init` 의 브로드캐스트 규칙 미러.
**R1 중단 판정("넷 저장이 폭별로 안 나뉘면 중단") = 통과** — corpus 72 전수 빌드 성공.

**S1b — `impl NetReader for NetArena`.** 엔진 `read_net` flat arm 의 Value-수준 미러(OOB all-X
비클램프 · scalar `word.unwrap_or(0)` · top-word 재마스킹). 의도적 부재 2건을 문서로 남김:
OOB 시 `warn_run_range` 진단(stderr 는 S1d 의 몫)과 `read_scalar_words` fast path(S2).

**게이트(전부 in-crate 차분·엔진이 오라클).** ① init parity: corpus **297넷 전수** 아레나 ≡
엔진(원소·whole·OOB 읽기) + 레이아웃 타일링 어서션 ② 워드 경계 사다리(1/7/63/64/65/127/128/
129/200b + 배열·signed) 마스킹 라운드트립 ③ **미러 차분 17,940건 발산 0** — 같은 랜덤 4-state
상태를 양 스토어에 미러하고 같은 평가기를 두 리더로 돌려 全 pure expr × 5 상태 × 2 문맥 폭
비교, 카운트를 정확 핀(커버리지가 조용히 줄면 핀이 운다).

**기반 정비 둘.** `extern crate self as sim_engine`(sim-ir 선례)로 통합 테스트의 공유 corpus
파일을 unit test 가 `#[path]` 재사용(중복 0·clippy `duplicate_mod` 때문에 include 는 native/mod.rs
한 곳) · 그 파일의 `tmp_dir` 를 `option_env!`+fallback 으로(unit test 에는 `CARGO_TARGET_TMPDIR` 가
없다 — 통합 테스트 쪽은 compile-time Some 그대로). 부수 발견: 공유 `Rng::range(0, u64::MAX)` 는
span 산술이 오버플로한다(hi-lo+1·loud) — 테스트 로컬 `r64()` 로 우회, 기존 호출부는 전수 유한 상한.

**적대 2렌즈 — BLOCKING/MAJOR 0.** soundness 가 read_net/init 을 엔진과 arm-별 대조(전 Value 필드
PartialEq 포섭·`array_len==0` 경계 동일까지)하고 **S2 의무 1건**을 냈다(OOB arm 이 엔진의
`is_real` 스탬프를 생략 — 오늘은 Real 거부라 sound·차분이 구조적으로 못 잡는 자리 → OOB arm 에
의무 코멘트로 핀). differential 은 **깰 수 없었다**: 자체 프로브 ~21,600점(광폭 배열 1~193b ·
엔진 비정렬 bit-serial 전 위상 0..63 스윕 · X/Z/OOB/2^64 인덱스 · signed×문맥 resize 행렬 ·
distinct-원소 whole-read) 3-way(ground truth 포함) 전부 동일 — 다만 **출하 차분의 측정 공백 2건**
(corpus 원소 폭이 4~16 뿐·whole-read 가 브로드캐스트 init 라 vacuous)을 지적했고, 리뷰어의 프로브
파일을 권고대로 **업스트림에 흡수**해 닫았다(`native/probe_tests.rs`·3 테스트·카운트 핀).
검증 = 전 스위트 5090 green · clippy `-D warnings` · fmt.

**무엇.** preview/21 §7.3 의 첫 두 단계. ① **T0** — run.json 에 `codegen{able,total,frame_bodies,
reject_reasons}`: ②층 VM 이 이 설계에서 무엇을 거부했고 왜인지. 그 전까지 유일한 관측 수단이
`--backend` A/B 타이밍이었다(round-26 이 keccak 호출형의 **VM 기여 0%** 를 그 방법으로야 발견한
이유). ② **S0** — `sim_engine::native::design_eligibility(ir, opts)` + run.json `native{eligible,
reject_reasons}`: "이 설계 **전체**를 ③층이 받을 수 있나"(§4.1 설계 단위 all-or-nothing 의 전제)를
IR + 사이드카만으로 답한다.

**구조 결정 셋.** ① 히스토그램은 **새 술어가 아니다** — `is_codegen_able` 을 reason-수집 walk
(`reject_reasons_into`) 위의 `is_empty()` 로 재구성해 REAL gate 와 obs 가 **한 walk 를 공유**한다
(분류기-드리프트가 구조적으로 불가능 — §4.5.276 의 "두 술어" 함정을 설계로 봉쇄). terminator match 는
`_`-free: frozen 타입에 변종이 늘면 컴파일 에러 = 강제 분류. ② S0 게이트의 SimOpts 분류는 **`..`
없는 전수 destructure** — 사이드카를 새로 달면서 게이트 분류를 잊는 실수가 컴파일 에러가 된다.
③ 검출기는 사이드카가 아니라 **NetKind 스캔** — plain `int q[$]` 는 사이드카가 없어서 `*_dyn_nets`
로는 영원히 안 보인다(doc-21 §4.3 의 의도를 구현이 정정). `func_table` 은 §4.3 초판의 거부 목록에
있었지만 **코어로 옮겼다**(개정 4 가 T1/T2 를 S3 에 흡수 — "호출을 삼키지 못하면 중단"인 계획이
호출을 거부하는 게이트를 가질 수는 없다). doc-21 §4.3 에 개정 주석.

**함정 하나(컴파일러가 잡음).** `design_eligibility(ir, &opts)` 를 SimResult 조립 지점(런 끝)에서
부르려 했으나 `opts.fork_modes` 가 `Scheduler::new` 로 **move** 된다 — 사전 grep 감사는 clone 만
보고 이 move 를 놓쳤다. 끝에서 읽었으면 fork 설계가 **빈 테이블 위에서 eligible 로 조작**될 뻔
(정확히 doc-19 §3 이 금지하는 wrong-log). verdict 를 스케줄러 생성 **전에** 채취.

**측정(= S0 의 산출물).** 실사용 7종(examples 4 · picorv32+TB · keccak 호출/인라인) **전부 적격**,
P6 corpus **72/72**(`native_gate.rs` 가 정확 수치로 핀) → **중단 판정("실사용 4종 0%") 통과, S1 go**.
keccak 호출형 행 = round-26 맹점의 계기화: `able 1/4 · frame_bodies 3 · user_call_in_expr` 가 JSON
한 줄. ⚠️ 이 100% 는 **설계 수준 상한**(문장 수준 능력은 S3 컴파일러 시점에 게이트 합류)이고,
성공 기준(리포터 워크로드 ≥30×) vs v1 string 거부의 **모순은 열어 둔 채 기록**(preview/21 §7.3.1,
S2 실측 후 판정 — 스펙 변경 2회+ 검토 룰).

**검증.** doc-19 R-L0 필드 목록 갱신(additive·schema_ver 1 유지). 신규 테스트 8(obs 핀 3 — 정확
카운트 문자열 = teeth + backend-invariance · native_gate 5 — 가족별 발화 + corpus 정확 수치). 기존
obs 결정성 골든이 신규 필드를 자동 포섭(wall-clock 제외 byte-diff). **적대 2렌즈 CLEAN-급**(BLOCKING/
MAJOR 0): soundness 가 술어 교체를 arm 별 의무 이관으로 전수 검증(불리언 등가 확인) + subsumption
주장을 populate site 4곳에서 검증, differential 이 25 설계 × 최대 4 실행기 구성에서 **바이트 발산 0**
+ run.json 값을 손 계수와 전수 대조. 리뷰 지적 3건 반영 = run.json `backend` 필드(정적 census 오독
방지·F1) · `sformatf` 키가 desugar 된 string concat 도 포함함을 명문화 · `native` 의 "(설계, 런 옵션)
별 정적" 정밀화(`--probe` 런은 설계상 부적격). 전 스위트 5084 green.

#### 4.5.279 백엔드 default 를 뒤집자 P5 게이트가 초록으로 통과시키던 39건이 드러났다 (2026-08-01, format 26 불변) ✅

**계기.** 실물 설계(picorv32+TB, 40000 cycle)의 리전 분해가 `bodies` 를 67%(interp)/53%(vm)로 지목했고, VM 은 이미
그 리전을 1.76x 로 만들고 있었다 — 즉 **가장 큰 남은 레버는 새 코드가 아니라 VM 을 기본값으로 만드는 것**이었다.
그 전제(“VM 은 바이트 동일하다”)를 실제로 검증하려고 `Backend` 의 `#[default]` 를 `Bytecode` 로 임시로 뒤집고
`cargo test --workspace --no-fail-fast` 를 돌렸다.

**결과: 18 타깃 39건 실패.** P5 게이트(`backend_equiv.rs`, 72 디자인)는 그 전부를 통과하고 있었다 — 그 corpus 는
9개 산술/클럭/구조 템플릿의 파라미터 스윕이라, 아래 네 뿌리가 건드리는 모양을 **하나도 만들지 않는다**.

**뿌리 1 — 네이티브 경로에 타입 게이트가 없었다.** 네이티브 프로그램의 레지스터는 `(val, unk)` u64 쌍이고 소비자는
평범한 정수 `Value` 를 재조립한다. 즉 `is_real`/`is_str`/힙 핸들을 **구조적으로 나를 수 없다**. 그런데
`try_compile` 의 유일한 거부 조건은 **폭**(`root_w == 0 || > 128`)이었다. `string` 의 넷 폭은 0 이므로
`lvalue_width().max(1)` 이 컴파일러에게 **1비트 문맥**을 넘겼고 `"a"`(0x61)는 비트 하나 `0x01` 이 됐다.
`q = r` 은 2.75 의 IEEE 비트 `4613374868287651840` 이 됐다. 클래스 필드 읽기는 `Signal{word:Some(field_id)}` 라
인덱스 퍼널이 **필드 id 를 배열 워드로** 읽었다.
→ `native_eval::ineligible_nets`(넷 kind **포지티브** allow-list) + `SimState::native_ineligible`(kind 로는 절대
드러나지 않는 사이드카 — class handle · dyn handle · `real r[]`/`string s[]` element — 를 OR). 범위 밖은 ineligible.
지연 채움인 이유: 그 사이드카들은 `SimState::new` **이후** out-of-band 로 설치된다.

**그 중 하나는 내가 만든 회귀다.** cont-assign RHS 를 네이티브로 컴파일한 직전 커밋(`7937c94`)이 모든 assign RHS 를
`try_compile` 에 태웠으므로, `assign w = r;`(real→64bit wire)이 **기본 인터프리터 경로에서** correct→silent-wrong 이
됐다. 3-way: iverilog `3` · PRE(`4abfcda`) `3` · POST `4613374868287651840`. `--backend` 는 태그된 v0.1.0 에 없으므로
릴리스된 버전은 무영향.

**뿌리 2 — 바디 prologue 가 복사되어 표류.** `run_process` 의 prologue 는 `cur_time_mult`/`cur_prec_mult`/`cur_scope`
셋을 세운다. `vm_run_body` 는 그 중 **첫 하나만** 손으로 옮겨 적은 발췌본을 들고 있었다. 빠진 둘은 둘 다 조용히
관측된다: 서브모듈의 `%m` 이 **다른 프로세스가 마지막에 남긴 스코프**를 렌더했고(`tb` vs `tb.u1`), 자기
`timescale` 정밀도를 가진 모듈의 `$time` 이 직전 정밀도로 렌더됐다. → `exec::enter_body` 하나로 합치고 양쪽이 호출.

**뿌리 3 — `c = new` 의 의미는 StmtId 사이드테이블에 있다.** IR 상으로는 placeholder const 를 rhs 로 가진 평범한
`BlockingAssign` 이고, `compute_effect` 가 `class_new_sites` 를 **먼저** 확인해 그 placeholder 를 아예 평가하지 않는다.
IR 만 보는 분류기에는 보이지 않는다 → VM 이 placeholder 를 컴파일 → 핸들이 X 인 채로 이후 필드 쓰기가 전부
"null/X 핸들 역참조(무시됨)" 경고와 함께 버려지고 **exit 0**. 명시적 생성자(`new(7)`)는 Call 이라 B1 이 이미 막고 있었다 —
**암시적 default `new` 만** 구멍에 닿았다.

**뿌리 4 — intercept 목록이 정본의 손복사본이었다.** `sim_ir::sysfunc_is_stmt_effect` 는 `_` arm 없는 exhaustive 정본이고
(새 `SysFuncId` 는 누군가 편을 정할 때까지 컴파일이 안 된다), `k_rhs_is_stmt_effect_family` 와
`compute_suspendable_tasks` 가 그것을 쓴다. `is_codegen_able` 만 **그 목록을 손으로 다시 적고 있었고**, seeded `$dist_*`
7개 중 `DistUniform` 하나만 담고 있었다. `v = $dist_normal(seed,…)` 는 VM 으로 컴파일돼 **시드를 되쓰지 않았고**, 이후
모든 draw 가 첫 값을 반복했다 — 같은 프로그램의 `$dist_uniform` 은 맞았기 때문에 RNG 특성처럼 보였다.
→ 손복사본을 지우고 정본을 호출. 차이는 **정확히 하나**(`$sformatf`)이고 그 이유를 주석에 박았다: 정본이 답하는 질문은
"프레임 실행기가 이걸 할 수 있나"이고 프레임 경로에는 전용 intercept 가 있다 — VM 에는 없다.

**결과.** 39건(18 타깃) → 24건(5 타깃) → **0**. 전 워크스페이스 스위트가 `Backend::Bytecode` 를 default 로 두고 통과한다.
default 를 실제로 뒤집을지는 별개의 오너 판정으로 남긴다(성능·기본 동작 변경).

**게이트를 non-vacuous 로.** P5 corpus 에 생성기가 만들 수 없는 hand shape 를 추가(`backend_equiv.rs::HAND_SHAPES`) ·
CLI 레벨 핀 5개(`backend_flag.rs`) — 라이브러리 하네스는 사이드카를 만들지 않으므로 그 절반은 **거기서는 공허하게 통과한다** ·
`backend.rs` 단위 핀 2개(seeded `$dist_*` 7종 전부 + `$sformatf` 델타 + `class_new_sites`). 각 핀은 게이트를 끄고 실패를 확인했다.

**교훈.** ① 분류기가 실행기의 판단을 **복사**하면 반드시 표류한다 — 정본을 호출해라(§4.5.276 과 같은 뿌리, 이번엔 두 번). ②
공유 헬퍼를 재사용해도 **호출자 쪽 전제조건은 따라오지 않는다**(`try_compile` 의 타입 계약은 호출자에게 있었다). ③
게이트의 힘은 게이트 코드가 아니라 **그 안의 모양**이 정한다 — 이번 세션 다섯 번째. ④ default 를 뒤집어 전 스위트를 돌리는
것은 72 디자인 differential 보다 **압도적으로 강한** 측정이고, 비용은 한 줄 + 10분이다.

#### 4.5.284 외부 round-28 — IEEE 1364-2005 §3.5 암시적 net 선언 (2026-08-03, format 26 불변) ✅

**리포터가 사이트별 표준 판정을 붙여 왔다** — 상용 ASIC 트리(VM107, 소스 109개)의 `E3010` 97 +
`E3009` 3 건을 고유 원인 **7개**로 접고, 각각 "표준 Verilog 에서 합법인가"로 분류했다. 결과:
**6개가 vita 미지원, RTL 실제 결함은 1개(8-D)**. 그리고 **7개 중 2개(8-A 표준셀, 9-B EFUSE)가
파운드리 납품 라이브러리 안**이라 사용자가 고칠 수 없다.

**§3.5 = 이 슬라이스의 본체.** IEEE 1364-2005 §3.5 는 미선언 식별자가 ⓐ 게이트/모듈 인스턴스의
**터미널 리스트**, ⓑ **continuous assignment 의 LHS** 에 나타나면 현재 `default_nettype` 의
**스칼라 net 으로 암시 선언**된다고 규정하고, 기본값은 `wire` 다. vita 는 두 위치 모두 거절해
**항상 `default_nettype none` 처럼** 동작했다 — 그리고 그것은 버그가 아니라 **doc-15 에 명문화된
정책**이었다("오타가 조용히 wire 가 되는 사고 클래스가 원천 차단되는 보수적 선택").

판정을 뒤집은 것은 두 가지다: ⓐ **비준수**이고 ⓑ **사용자 쪽 수정이 불가능**하다(파운드리 납품물).
refusal 이 사던 안전은 **`W2003`** 이 대신 산다 — doc-15 가 **정확히 이것을 위해 예약**해 둔 코드이고
(`emitter 0` 이라고 적혀 있었다), `-Werror=W-PARSE-IMPLICIT-NET` 이 hard error 로 되돌린다.

**경계가 전부이고, 추론이 아니라 iverilog 로 핀했다** — §3.5 밖 세 방향은 iverilog 도 error 다:

| 위치 | vita | iverilog |
|---|---|---|
| 게이트 터미널 `not (Ax, AN)` | ✅ 암시 net + W2003 | ✅ |
| cont-assign LHS `assign mid = ~a` | ✅ 암시 net + W2003 | ✅ |
| 인스턴스 터미널 `sub u(.o(IMPL))` | ✅ 암시 net + W2003 | ✅ |
| 평범한 rhs `assign y = TYPO` | **E3010** | error |
| procedural lvalue `initial TYPO = 1` | **E3010** | error |
| `` `default_nettype none `` 하 | **E3010** | error |

8형 3-way 전수 **POST = iverilog 8/8**.

**⭐ 순서 함정 — §3.5 는 사용이 아니라 선언이다.** 처음엔 사용 지점(cont-assign lower / 인스턴스
루프)에서 만들었다. vita 는 cont-assign 을 인스턴스보다 **먼저** 낮추므로
`sub u(.o(IMPL)); assign o = IMPL;` 이 **읽기에서 E3010, 터미널에서는 성공** — 한 설계가 phase
순서에 따라 두 판정을 냈다. 선언은 순서 무관이어야 하므로 **바디 전체를 도는 전용 pre-pass 하나**로
옮겼다(수집기를 둘 만들지 않는다 — ENGINEERING_RULES "N 번째 수집기").

**⭐ 8-D 를 loud 로.** §3.5 net 은 **스칼라**라 더 넓은 드라이버는 bit 0 만 남기고 버린다 —
**모든 시뮬레이터가 조용히** 하는 일이다(합법이므로). 리포터가 찾은 실사이트는 포트 선언이 비활성
`` `ifdef `` 뒤에 있고 `assign` 은 가드가 없어 **12비트가 1비트로** 들어갔다. vita 는 **값은
iverilog 와 같게 두고**(differential 이 이긴다) `W3056` 으로 **폭 두 개를 말한다**:
*"drives it with 12 bits and the top 11 are discarded"*. 리포터가 요청한 것보다 한 걸음 더 간 부분.

**`` `default_nettype `` 디렉티브.** 전처리기가 `timescale` 과 **같은 형태**로 region 을 기록하고
(`nettype_none: Vec<(offset, bool)>`), `resolve_module_nettype` 이 모듈별로 접고, 그 결과를 드라이버가
**AST 필드**(`ModuleDecl.nettype_none`)에 찍는다. AST 에 둔 이유는 **`.vu` 를 자동으로 타기** 때문이다 —
staged `velab` 은 소스를 못 보므로, 사이드테이블에 두면 `vcmp`↔`velab` 사이에서 정책이 조용히 바뀔 수
있다. 파일 순서 sticky(RULE S)까지 핀했다.

**9-A 진단 — EVENT CONTROL 을 lvalue 라 불렀다.** `always @(\`TOP.a_uVDC.RTRIM_I)` 가
`resolve_net` 의 lvalue 문구로 보고돼("a whole-net hierarchical write `tb.dut.x = …` is supported"),
리포터가 **존재하지 않는 계층 대입문을 찾느라 시간을 크게 썼다**. 문맥을 아는 호출자가 문구를
소유하도록 바꾸고(바로 위 N4 clocking arm 과 같은 형태), 해상 질문은 `resolve_net` 자신의 술어를
추출한 `lookup_dotted_net` 으로 던진다(복사하면 A2b-prereq F2 필터를 잃는다). 새 문구는 **동작하는
우회**(`always @(*) local = <hier>;`)를 제시하고, 테스트가 **그 우회가 실제로 동작하는지**까지 건다.

**`specparam`(부록).** 벤더 모델이 타이밍 상수를 `specparam` 에 두고 지연식에서 참조하는 것은 흔한
패턴인데 vita 는 모듈 레벨에서도 거부해서 리포터가 **키워드를 바꿔야** 했다. `localparam` 과 같은
경로로 받는다(차이는 SDF 백애노테이션뿐 — 기능 시뮬에 무관). `specify … endspecify` 블록 자체는
아직 loud = **follow-on**(리포터의 우회 스크립트가 "specify 를 지우되 specparam 은 살린다"로
단순해지는 것이 이번 요청이었다).

**남은 요청 3건은 기록**: `specify` 블록 수용 · 이벤트 컨트롤의 계층 참조 **실지원**(감도 등록만의
문제로 보이나 sensitivity 슬롯을 instance 확정 후 패치해야 한다) · cross-process `disable` 의
no-op 케이스. 그리고 E3010/E3009 의 file:line 은 **일관되지 않다**(있는 자리도 있고 없는 자리도
있다) — 별도 슬라이스.

**게이트**: 5076 tests green · clippy `-D warnings` clean · fmt clean · format_version 26 불변
(`.vu` 스키마 해시는 AST 필드 2개 추가로 re-pin — `ModuleDecl.nettype_none`,
`ContinuousAssign.from_gate`).

---

#### 4.5.283 ★★ 외부 round-27 — `@(*)` 가 attribute 로 렉싱되어 소스가 삼켜졌다 (2026-08-03, format 26 불변) ✅

**이 저장소가 받은 최고 심각도 리포트.** README 가 내거는 계약 — *"the simulator never
produces a silently wrong result"* — 이 정면으로 깨졌다.

**증상 (R1, silent-wrong):**

```verilog
always @(*) a = b;  // *)(b) begin a = 1'b1; $display("!! COMMENTED-OUT CODE EXECUTED !!"); end
```

PRE 출력: `!! COMMENTED-OUT CODE EXECUTED !!` · `a=1` · **`errors=0`**. iverilog 는 `a=0`.
사용자의 `a = b;` 는 사라지고 **주석이 실행 코드가 됐다.** 진단이 없었으므로 어떤 게이트도
잡을 수 없었다.

**뿌리.** attribute instance(IEEE 1800-2017 §5.12) 스킵이 **원문 정규식**이었다:

```rust
#[logos(skip r"\(\*([^*]|\*[^)])*\*\)")]
```

그 위의 주석은 이렇게 주장하고 있었다 — *"이 정규식은 `(*)` 를 매치할 수 없다. `(*` 뒤
본문은 `[^*]` 또는 `*[^)]` 이고 종결자는 `*)` 이므로, 남은 문자가 `)` 뿐인 `(*)` 는 종결자에
도달할 길이 없다."* **그 세 글자만 놓고 보면 참이고, 정규식이 하는 일이 아니다**: 본문
`([^*]|\*[^)])*` 이 그 `)` 를 먹고 **그 뒤 전부**를 먹으며 **컴파일 단위 어디든 다음 `*)`** 까지
간다. 그래서 —

| # | 증상 | 왜 |
|---|---|---|
| R1 | 주석이 실행 코드로, `errors=0` | 원문 스캔은 `//` 를 못 본다 |
| R2 | `@(*)` 두 개 → **두 번째**에 진단 | 두 번째의 `*)` 가 첫 번째의 짝이 된다 |
| R3 | **파일 경계를 넘어** 짝이 맞음 | 컴파일 단위 = 하나의 스트림 |
| R4 | `$display("… (*) …")` 가 unterminated string | 원문 스캔은 문자열도 못 본다 |
| — | 안 닫히면 **무진단 폴백** | 발현 여부가 **단위 전체의 `(*)`/`*)` 개수**에 달린다 = 비국소 |

**수정 — 정규식을 고치는 게 아니라 인식 위치를 옮겼다.** attribute 를 **토큰 스트림**에서
인식한다(`strip_attribute_instances`). 그 시점엔 **주석은 이미 사라졌고 문자열은 한 토큰**이라
코드가 아닌 텍스트가 구분자를 공급할 수 **없다**. 여기에 두 가지를 더 얹었다:

1. **`@` 직후의 `(*` 는 event control** — IEEE 1364-2005 A.6.5 가 `@ (*)` 를 정식 production 으로
   두고, attribute 는 event control 자리에 올 수 없다. `out.last()` 가 곧 직전 **유효** 토큰이라
   `@(*)`·`@ (*)`·`@ /* c */ (*)` 가 같은 검사 하나로 걸린다.
2. **안 닫힌 opener 는 loud**(`LexErrorKind::UnterminatedAttribute`). 닫는 스캔도 `@(` + `*` 를
   건너뛴다 — `@(*)` 안에 인접 `*` `)` 가 있어서, 그러지 않으면 **안 닫힌 attribute 가 다음 감도
   리스트까지 조용히 삼킨다**(같은 비국소 동작이 한 단계 뒤로 물러날 뿐).

**검증.** 3-way(PRE `bb8003b` / POST / iverilog 13) 16형 전수 — **POST 16/16 = iverilog ·
회귀 0 · 수정 8**. 형태 매트릭스 7종(`@(*)` `@ (*)` `@(* )` `@( *)` `@ ( * )` `@*` `@ *`)이
전부 동일 동작. 위치 3종(task 본문·generate body·`always_comb` 옆) 전부 PRE 실패 → POST/iverilog
일치. PicoRV32(attribute 19개) 출력 불변.

**리포터가 못 찾은 것 1건도 같이 고쳐졌다** — `(* keep = "*)" *)`, 즉 attribute **자기 문자열**
안의 구분자. 원문 정규식은 문자열 안에서 종결했다. 토큰 인식은 공짜로 맞는다.

**기존 테스트가 vacuous 였다.** `attribute_instances_are_skipped_without_eating_implicit_sensitivity`
는 정확히 이 선을 지키려고 쓰였고 **지키지 못했다** — 설계마다 `@(*)` 가 **하나**이고 뒤에 `*)` 가
없어서, 정규식이 조용히 실패하고 폴백이 구해줬다. 그 doc 주석의 논증(위 인용)도 틀린 채로
남아 있었다. 문구를 정정하고 진짜 핀은 `round27_report.rs` 12건으로 옮겼다. 세 가드
(@ 가드 · 닫는 스캔의 event-control skip · 토큰 인식)를 각각 되돌려 teeth 확인
(10/12 · 1/12 · 12/12 실패).

**게이트**: 5067 tests green · clippy `-D warnings` clean · fmt clean · format_version 26 불변.

---

#### 4.5.282 ③층 격차를 처음으로 쟀다(76×) — 그리고 ②층이 고갈되지 않았음 (2026-08-03, format 26 불변) ✅

**외부 round-26 리포트 대응 + 그 위에서 한 측정.** 리포터의 실질 결함은 **0건**이었고 minor 1건
(§3)만 남았다. 그래서 이 슬라이스의 무게는 수정이 아니라 **측정**에 있다.

**§3 — 진단의 꼬리 절이 잘못된 주어에 붙었다.** §4.5.281 이 E3009 의 네-갈래 오답 목록을 실제
구성요소를 짚는 설명으로 바꿨는데, **그 설명을 어디에 이어붙였는지**가 틀렸다. 템플릿은
`body uses {reason}, which is outside the frame-call subset (…)` 이고 새 reason 이
"…the same assignment in a `task` body, or in a module process, **does work**" 로 끝났으므로,
꼬리 절이 **그 "does work" 에 붙어** 동작하는 형태가 subset 밖이라고 말하게 됐다.

수정은 괄호 치기가 아니라 **구조 분리**다: 분류기가 `(what, detail)` 를 돌려주고, `what` 은
꼬리 절이 붙을 **짧은 명사구**(자기 관계절 없음), `detail` 은 그 뒤에 **독립 문장**으로 붙는다.
같은 형태의 terminator arm(§4.5.278 의 output-formal 호출 설명)도 같이 옮겼다 — 그쪽은 설명이
**괄호에 싸여 있어서** 우연히 읽혔을 뿐, 구두점 하나 차이로 같은 오독이었다. 핀 3건은 문구가
아니라 **순서**를 단언한다(꼬리 절 앞에 설명이 오면 실패). 두 arm 모두 되돌려 teeth 확인.

**부수 수정 — `-DNAME` / `-Idir` 붙임형.** 재현하다 발견: vita 는 분리형 `-D NAME` 과 `+define+`
만 받아서 iverilog/VCS 흐름에서 복사한 명령줄이 `unknown flag '-DFAST_MODE'` 로 죽었다. 호환 함정이지
플래그에 대한 이견이 아니다. exact-match arm 들 아래 · `-` 접두 catch-all 위에 두 arm 추가.

**측정 — 왜 새 벤치가 필요했나.** 리포터는 *"병목이 string 에서 DUT 로 옮겨갔다"* 고 보고했다.
그것은 **위치**이지 **원인**이 아니다 — §4.5.281 에서 똑같은 형태의 문장이 ③층과 무관한 알고리즘
결함으로 판명났다. PicoRV32 는 32-bit 스칼라 제어 설계이고 리포터의 DUT(Keccak/SHA3)는 **64-bit
레인 25개의 데이터패스**라 완전히 다른 경로를 때린다. 그래서 **Keccak-f[1600] 을 직접 썼다**
(`bench/keccak/`, 1st-party·committed). 오라클 넷 — Python 참조 · vita · iverilog 13 ·
**verilator 5.050** — 이 모든 N 에서 같은 다이제스트를 내고, 그 값은 공표된 참조값
`f1258f7940e1dde7` 이다(상호 일치만이 아니라 **외부 앵커**).

**① ②→③ 격차 = 76×** (순열 1회당 한계비용, interleaved, 같은 기계):

| | 서브루틴 호출 있음 | 서브루틴 인라인 |
|---|---|---|
| vita (②층) | 5340 µs | **498 µs** |
| iverilog 13 (②층) | 4450 µs | 1398 µs |
| verilator 5.050 (③층) | 6.6 µs | **6.56 µs** |

**이 저장소가 처음 자기 손으로 잰 ②→③ 격차다.** doc-18 은 오랫동안 *"격차 크기는 모른다 —
VCS/Xcelium 을 보유하지 않는다"* 라고 적어두고 있었는데, **verilator 는 무료이고 ③층이었다.**

**② 최대 발견 — VM 커버리지가 0% 다.** 같은 설계·같은 결과인데 위 표의 열 하나가 **10.7×** 이고,
차이는 사용자 함수 호출뿐이다. `--backend` A/B:

| 설계 | interp | bytecode | VM 기여 |
|---|---|---|---|
| PicoRV32 | 1.35 s | 0.86 s | 1.57× |
| **Keccak (호출 있음)** | **1.11 s** | **1.11 s** | **0%** |
| Keccak (인라인) | 0.21 s | 0.10 s | 2.1× |

`backend::is_codegen_able` 은 terminator 가 `Goto`/`Return` 이 아니면 프로세스를 통째로 거부하고
`Terminator::Call` 은 그 밖이며, `codegen_coverage` 는 `ir.processes` 만 순회한다 — **함수/태스크
바디는 애초에 컴파일 대상이 아니다.** unstripped release 프로파일이 `eval_ctx`(①층 트리워커)를
1위로 찍어 이를 확인했다. **즉 사용자 함수를 부르는 RTL — 대부분의 실 RTL 과 거의 모든 TB —
에서 vita 는 ②층을 쓰지 않는다.**

**③ 원시 연산 단가** (각 2M 반복, 차분법):

| 연산 | vita | iverilog | |
|---|---|---|---|
| 인라인 64-bit 산술 (루프 1회) | **110 ns** | 610 ns | vita **5.5× 빠름** |
| 사용자 함수 호출 오버헤드 | 650 ns | **375 ns** | vita 1.7× 느림 |
| 함수 지역 배열 원소 쓰기 | **514 ns** | **24 ns** | vita **21× 느림** |
| (대조) 모듈 레벨 배열 원소 쓰기 | 대등 | 대등 | — |

**판정.** 두 문장이 모두 참이다 — ⓐ **③층은 여전히 ③층에 도달하는 유일한 경로다**(vita 최선에서도
76×), ⓑ **그러나 ②층은 고갈되지 않았다**(측정된 10.7×, 그 중 2.1× 는 백엔드 플래그로 증명 끝).
doc-21 은 개정 3 으로 **T 단계(②층 청구 5개)를 S 단계(③층) 앞에** 넣었다 — 미루기가 아니라
**③층 예산 확정**이며, ③층 백엔드도 같은 호출 문제를 풀어야 하므로 T1/T2 의 콜아웃 ABI 설계가
§4.1(설계 단위 all-or-nothing)의 **리허설**이다.

**교훈(ENGINEERING_RULES 로 승격).** 성능 결론은 **벤치의 모양에 종속**된다. PicoRV32 하나로
"고갈"을 선언한 것이 두 라운드 연속 틀렸다(round-25 string 축 172×, round-26 호출 축 10.7×).
**"이 설계에서"를 문장에서 빼면 그 문장은 거짓이 된다.**

**게이트**: 5055 tests green · clippy `-D warnings` clean · fmt clean · format_version 26 불변.

---

#### 4.5.280 백엔드 default 를 VM 으로 뒤집었다 (2026-08-02, format 26 불변) ✅

§4.5.279 가 만든 근거 위에서 오너 판정으로 뒤집었다. `Backend::default()` = `Bytecode`,
`SimOpts::default().backend` = `Bytecode`. VM 이 못 먹는 바디는 **바디 단위로** 인터프리터로 떨어지므로
설계 종류를 가리지 않는다. `--backend interp` 는 남는다 — 속도용이 아니라 VM 결함을 레퍼런스와 한 플래그로
이분하기 위한 것.

**근거.** 전 워크스페이스 스위트(5000+)가 `Bytecode` default 로 통과한다(§4.5.279 에서 39→24→0).
72-디자인 P5 differential 만으로는 부족했다는 것이 §4.5.279 의 요지이므로, 근거는 스위트 쪽이다.

**게이트를 뒤집기에 맞춰 고쳤다 — 안 고쳤으면 셋이 공허해졌다.**
`selecting_the_vm_moves_no_output_byte` / `naming_the_default_explicitly_changes_nothing` /
`the_staged_run_honours_the_flag_and_still_matches` 는 전부 "플래그 없음" vs `--backend vm` 을 비교했다.
인터프리터가 default 이던 동안엔 진짜 differential 이었지만, default 가 `vm` 이 되는 순간 **VM 대 VM** 이 되어
공짜로 통과한다. 셋 다 양쪽을 **명시**하도록 고치고, default 값 자체는 별도 핀
(`backend_equiv.rs::the_default_backend_is_the_vm`)이 값으로 못박는다.
→ **differential 은 두 변 중 하나를 default 에서 가져오면 안 된다.**

**⚠️ 성능 주장 정정.** §4.5.279 커밋 메시지와 doc-18 에 "vita+VM 0.81 s vs iverilog 0.88 s" 라고 적었는데
**iverilog 수치가 틀렸다** — 갓 컴파일한 `.vvp` 의 cold 첫 실행이었다. 번갈아 6회씩 재면 iverilog 는 0.57–0.59 s.
정정: iverilog **0.61 s**(compile 0.03 + run 0.58) vs vita 기본 **0.78 s** — **vita 가 약 1.28x 느리다.**
parse/elaborate(~0.13 s)를 빼고 순수 시뮬만 비교해도 0.65 vs 0.58 로 여전히 뒤진다.
VM 전환은 **vita 자신에 대한 1.41x**(1.10 → 0.78)이지 iverilog 추월이 아니다.
교훈: 벤치 바이너리를 갓 만든 직후 한 번 재지 마라 — interleaved 반복 + best-of-N.

#### 4.5.278 외부 round-23 — 분류기가 호출의 copy-out 목적지를 아예 안 보고 있었다 (2026-07-31, branch feat-r23-report, format 26 불변) ✅

**리포트 = 2건.** §3.1 COPYOUT-NET-PANIC(프레임 본문 안의 bare call statement 의 output/inout actual 이 모듈 net 이면 `frame lvalue net is routed` 로 **exit 101 패닉, 진단·소스위치 없음**) · §3.2 PERF-COMB-DEPTH(같은 총 작업량인데 조합 깊이 1→6 이 4.3× 느림) · §4 진단 품질(같은 구성의 E3009 문구가 **패닉하는 형태를 "동작한다"고 명시**). 리포터의 재현 6개(패닉 2 + 경계 4) 전부 HEAD 에서 그대로 재현됐다.

**§3.1 근인 = `Terminator::Call` 의 copy-out 목적지는 `Stmt` lvalue 가 아니다.** `compute_suspendable_tasks` 는 문장의 lhs 만 걷는다(r18 이래 "이 문장이 프레임 창 `[lo,hi)` 밖을 쓰나"). 그런데 호출의 copy-out 목적지는 문장이 아니라 **call-site 사이드 테이블**(`task_calls_func`, Call 블록의 전역 id 로 키)에 산다 — 그래서 워크가 **한 번도 본 적이 없다**. `inner(a, gv)` 의 caller 는 "subset" 으로 남았고, 동기 `&self` `run_task` 가 copy-out 을 수행하다 `frame_write_lvalue` 에서 라우팅 안 된 net 을 만나 `.expect()` 로 프로세스를 죽였다. 수정 = 워크의 `Terminator::Call` arm 이 그 블록의 목적지 net 들을 보고 하나라도 창 밖이면 **suspend 신호**로 친다. 그러면 caller 가 `run_process` 로 라우팅되고 copy-out 은 `write_lvalue` 퍼널(프레임-로컬↔모듈 net 분기 + dirty 채널)을 탄다 — 같은 본문의 `gv = a;` 가 이미 쓰던 그 길이다.

**pure-function 계약**: 새 입력은 양쪽이 **같은 함수** `sim_ir::call_out_nets` 로 줄인다(elaborate 의 `TaskCallInfo` 와 엔진의 것은 서로 다른 struct 라 각자 루프를 쓰면 드리프트한다). 엔진 맵은 elaborate 맵의 **상위집합**이고 차이는 resolve-time 에 추가되는 deferred hier enable 뿐인데, 그 caller 들은 `FuncMeta.has_hier_call` 로 이미 양쪽에서 force-suspend 되므로 **없는 엔트리는 신호 아님**이 두 계산을 일치시킨다(§4.5.208 선례).

**"옆 문장이 답을 정한다"가 마지막까지 남아 있었다.** `#5 inner(a, gv);` 도 `if (c) inner(a,gv); else gv = 0;` 도 **PRE 에서 이미 동작**했다 — 전자는 `Delay` terminator 가, 후자는 else arm 자신의 창-밖 쓰기가 무관한 이유로 태스크를 suspendable 로 만들었기 때문이다. 호출이 메모리를 쓰는지가 **옆에 무슨 문장이 있는지**로 갈렸다. round-22 와 같은 탐지 규칙(무관한 문장을 넣고 빼서 답이 바뀌면 분류기가 문장의 일부만 보고 있다)이 두 라운드 연속 적중했다.

**§3.1 을 고치자 loud 였던 이웃 4형태가 correct-support 로 올라갔다.** `stmt_main` 의 직접-rhs arm 과 general hoist 의 stand-down 은 둘 다 "프레임 본문의 쓰기는 프레임-로컬이어야 한다"를 근거로 걸려 있었는데, 그 근거가 바로 방금 없앤 패닉이었다. 태스크 본문은 조건을 통째로 뺐고(`frame_task_lowering`), 프레임 **함수** 본문만 남겼다 — 함수는 `Expr::Call` 로 표현식 평가 중에 진입하므로 **자기 소유의 call terminator 가 없다**(라우팅할 대상이 없다). 이건 신중함이 아니라 형태의 문제다.

**§4 진단 3건.** ⑴ E3009 문구에서 *"a BARE call statement there does work"* 를 걷어내고 실제로 남은 것(프레임 **함수** 본문)과 동작하는 것(`task` 본문·모듈 프로세스)을 적었다 — 문구를 믿고 식을 bare 문장으로 바꾸면 loud 가 **crash 로 바뀌던** 자리다. ⑵ `classify_frame_body` 가 거부한 terminator 를 **전부** "a timing/suspend/fork control (#delay, @, wait, fork)" 로 보고하고 있었다 — 타이밍 제어가 한 줄도 없는 본문이 그렇게 보고됐다. `Call` arm 을 갈라 진짜 이유를 적는다. ⑶ `frame_eval.rs` 의 `.expect("frame lvalue net is routed")` 를 `fatal_frame_unrouted_write` 로 교체 — 남은 어떤 경로가 여기 닿아도 vita 내부 file:line 이 아니라 **net 이름과 두 가지 우회**를 말하는 fatal 이 나온다(rc=101 abort → `FinishReason::Error`).

**loud 를 걷자 그 밑의 silent-wrong 이 드러났다 (`StrPutC`).** `s[i] = f(a, o)` 를 프레임 본문에서 허용하자 문자열이 **조용히 안 바뀌었다**. 뿌리는 호출과 무관했다: `SysTaskId::StrPutC` 가 `dyn_heap[net]`(모듈 문자열 저장소)를 무조건 썼는데, **프레임-로컬 `string` 은 프레임 슬롯에 slab-저장**된다(§4.5.167). 그래서 `task automatic tk(); string s; s = "zz"; s[0] = 65;` 는 호출도 output formal 도 없이 `"zz"` 를 유지한 채 exit 0 이었다 — **pre-existing**. `str_putc` 로 저장 위치를 먼저 묻게 했다(`read_net` 이 늘 물어온 그 질문을 쓰기 쪽에서도). 게이트를 걷으면 그것이 가리던 것은 내 것이 된다.

**§3.2 는 성능이고, 리포트의 뿌리 가설이 측정으로 반증됐다.** 리포터는 "깊은 조합 cone 을 levelize 하지 않아서"로 진단했다. 그런데 **총 라운드 수를 1200 으로 고정**한 깊이 스윕(UNROLL=1/2/3/6/12, 조합 단을 인스턴스로 체인)에서:

| UNROLL | cycles | iverilog | rel | vita | rel | vita/iv |
|---|---|---|---|---|---|---|
| 1 | 1200 | 0.137 s | 1.00× | 0.128 s | 1.00× | 0.93× |
| 3 | 400 | 0.187 s | 1.37× | 0.184 s | 1.44× | 0.99× |
| 6 | 200 | 0.281 s | 2.05× | 0.273 s | 2.14× | 0.97× |
| 12 | 100 | 0.569 s | **4.15×** | 0.453 s | **3.55×** | 0.80× |

**iverilog 가 같은(오히려 더 가파른) 깊이 스케일링을 보인다.** 깊이 비용은 vita 의 결함이 아니라 **인터프리티드 이벤트구동 델타사이클의 성질**이다 — 리포터의 비교 대상(Xcelium)은 컴파일-타임에 levelize 하는 컴파일드 시뮬레이터다. 원인도 배치 추적으로 특정했다: UNROLL=6 에서 사이클당 프로세스 활성이 `7 6 5 4 3 2 1…` 삼각형(D²/2)이고, **배치 정렬은 지렛대가 아니다**(오름/내림차순 정렬이 4.86/4.85 s 로 동일) — 단 사이의 전파가 cont-assign settle, 즉 **델타 경계**를 거치기 때문이다. 상수항은 따로 측정했다: trivial flop 200k 사이클에서 vita 1.03M cyc/s vs iverilog 1.96M cyc/s(**1.91×**). 프로파일이 가리킨 `Value::resize`/`mask_top` 원워드 fast-path 를 시험 구현했으나 **측정 이득 0**(0.190→0.188 s)이라 두 번째 코드 경로를 남기지 않고 폐기했다. `-j` 는 파형 라이터 예산이라 실제로 시뮬레이션을 빠르게 하지 않으므로 `--help` 문구를 그렇게 고쳤다(리포트 §3.2 ④).

**검증.** 3-way(iverilog / PRE=`b05b69d` / POST) 70 프로브 — **회귀 0 · 패닉→정답 15 · loud→정답 11**, 나머지는 전부 PRE 와 동일하고 iverilog 와 일치. 적대 프로브(내 변경이 **만드는** 위험 전용: hoist temp 재진입·lvalue special 우선순위·over-marking·pure-function 계약·copy-out 시점)에서 `StrPutC` silent-wrong 1건 적발·수정. 테스트 `crates/cli/tests/r23_frame_copyout_escape.rs` 14건 신규. `format_version` 26 불변(직렬화 형상 무변경 — 새 분류기 입력은 이미 out-of-band 인 사이드 테이블에서 유도).

**남은 loud(의도적).** 프레임 **함수** 본문 안의 output-formal 호출(위 형태 사유 — 진단이 이제 그 이유를 말한다) · `repeat(<비상수>) @(edge)` 가 프레임-로컬을 읽는 형태(공유 카운터 위험, §4.5.14) · 프레임 본문의 `return $fgets(...)`(직접-rhs 아님).

#### 4.5.277 외부 round-22 — 실행기 선택이 무관한 `$display` 한 줄에 달려 있었다 (2026-07-31, branch feat-r22-report, format 26 불변) ✅

**리포터가 뿌리를 한 줄로 좁혀 왔고, 그 한 줄이 정확했다.** 세 증상(`$fgets`/`$fscanf`/`$sscanf` 가 F4004 · `$value$plusargs` 가 진단 없이 default 유지 · `$random(seed)` 가 0 반환·seed 미갱신)은 전부 **같은 뿌리**였고, 경계는 **본문에 무관한 `$display("x")` 한 줄을 넣느냐**였다. 리포터의 3 재현 전부 HEAD 에서 그대로 재현됐다.

**근인 = 분류기가 문장을 목적지로만 봤다.** `compute_suspendable_tasks` 의 `stmt_signal` 은 blocking assign 에 대해 **lhs 가 프레임 창 밖을 쓰나**만 물었다. `rc = $fgets(line, fd)` 의 lhs 는 in-frame output formal 이므로 "subset" 으로 읽혔고 태스크가 동기 `&self` 프레임 실행기에 남았다 — 거기서 같은 `SysFunc` 는 순수 `eval` 경로로 떨어져 **0 을 돌려주고 목적지를 안 건드린다**. 효과는 목적지가 아니라 **rhs** 에 있었다. `$display` 가 고친 이유도 정확히 이것이다: `Stmt::SysTask` 가 `_ => true` arm 으로 떨어져 태스크가 suspendable 로 표시되고 본문 전체가 효과를 수행할 수 있는 `&mut` 실행기로 옮겨간다. 수정 = **단일 정본 술어** `sim_ir::sysfunc_is_stmt_effect`(`SysFuncId` 전수 match, `_` arm 없음 — 새 id 는 컴파일 에러로 결정을 강제)를 만들고 `stmt_signal` 의 blocking arm 이 rhs 도 묻게 했다. `exprs` 를 인자로 추가(elaborate `&self.exprs` == engine `&st.ir.exprs`, `driver.rs` 에서 동일 arena 확인 — pure-function 계약 유지).

**함수도 같은 뿌리였고, 이유는 "IEEE 가 함수의 타이밍 제어를 금지한다"가 아니었다.** `compute_suspendable_tasks` 는 `is_task == false` 를 건너뛰고 있었는데, 이 집합이 실제로 주는 것은 **`&mut` 실행기**이지 suspend 가 아니다. 그리고 프레임 함수는 이미 `validate_frame_body` → `classify_frame_body(allow_call=false)` 가 **모든** 본문에 대해 Delay/Wait/Fork/Call terminator·프레임 밖 쓰기·print 아닌 `$systask` 를 거부하므로 **엔진에 도달하는 프레임 함수는 이미 leaf·non-suspending** — 태스크가 lift 前에 증명받는 바로 그 속성이다. 그래서 skip 을 걷었다.

**input-only 함수는 호출 형태부터 달랐다.** output formal 이 있으면 `emit_frame_func_out_call` 이 `Terminator::Call` 을 내주는데(엔진 라우터가 볼 수 있는 유일한 형태), input-only 면 `r = rd(fd)` 가 `Expr::Call` 로 낮춰져 `eval` → 동기 `run_task` 로 간다. `inout_func_names` 에 합류시켜 copy-out 경로로 보냈다(out-bind 는 return slot 하나).

**static task 의 `string` 로컬 — 같은 개념의 세 번째 수집기.** body-local 넷 kind 수집기가 셋인데(모듈 스코프 · frame body-local · **inline/static task** body-local) `string` arm 이 있는 곳은 앞의 둘뿐이었다(`map_net_kind_or_wire` 는 String arm 이 없어 `_ => Wire`). 그 하나의 Wire 가 두 가지 실패를 냈다 — 평범한 `s = "hi"` 는 **loud**(E3018 "procedural assignment to net `t.$itask$w$L.s`"), `$fgets(s, fd)` 는 **silent**(시스템 함수의 목적지 쓰기는 E3018 을 내는 lvalue 검사를 지나가지 않는다). 이것이 리포터의 "lifetime 키워드를 떼면 진단이 사라지면서 rc=0 은 그대로"(§3.1 마지막 문단)의 정체다. `frame_local_net_kind` 로 교체.

**bare sys-read 문장도 열었다.** `$fgets(line, fd);`(반환 버림)가 프레임 태스크 본문에서 `W3056 … skipped` + exit 0 + 목적지 미변경이었다. 옛 제외의 근거("`run_frame_call` 이 어차피 못 한다")는 분류기가 그런 태스크를 표시하지 않아서 참이었을 뿐이고, 이제 자기충족적으로 거짓이다 — discard temp 가 모듈 넷이라 rewritten 문장은 **두 겹으로** suspend 신호다. frame **FUNCTION** 본문은 그대로 둔다(거기서는 프레임 밖 쓰기를 `classify_frame_body` 가 거부하므로 사용자가 쓰지 않은 대입에 대한 E3009 로 바뀔 뿐).

**§4 fatal 이 멈추지 않던 것.** `&self` 문맥의 fatal 은 표현식 한가운데서 `Step::Fatal` 을 못 돌려주니 `call_fatal` Cell 에 래치하는데, 스케줄러가 그 래치를 **본문 실행 前** 세 자리에서만 폴링했다. 그래서 자기 본문에서 fatal 을 켠 프로세스가 **자기 `$finish` 까지 달려** `FinishReason::Finish` 로 끝났다 — 진단은 찍히지만 시뮬레이션은 그 위를 계속 갔고 TB 는 자기 PASS 를 출력했다. 시간 순서가 결정한다: (a) `run_body` **직후** 폴링해 body 가 돌려준 step 보다 fatal 이 이기게 하고 (b) `run_process` 의 **문장 루프**에도 폴링을 넣어 fatal 지점에서 프로세스를 세웠다. 부수 효과로 리포터의 "진단이 1건으로 합쳐진다"가 **정답**이 된다(첫 건에서 멈추므로).

**진단 문구가 실측과 반대였다.** F4004 는 *"a task vita can inline (no output/inout formals, no `automatic` lifetime)"* 을 권했는데 `automatic` 은 판별자가 아니었고(오히려 lifetime 을 떼면 **silent** 가 됐다), "inline 할 수 있는 task" 는 `string` 목적지에서 틀렸다. 남은 자리를 **측정해서** 다시 썼다 — 클래스 메서드 본문 · 연속 재평가 위치(`assign`/`force`/`wait` 조건) · intra-assignment delay. 그리고 그 자리들에서 **file-read 계열만 loud 이고 `$fopen`/`$value$plusargs`/seeded `$random`/`$dist_*`/`$cast` 5종은 silent** 였다(pre-existing, 클래스 메서드 15-probe 행렬로 PRE==POST 확인) → 게이트를 정본 술어로 교체해 전부 loud.

**적대 리뷰 2 렌즈가 내 수정에서 3건을 잡았다(전부 내가 만든 것).** ①**과대표시 비용을 소비자별로 안 물었다**: 함수 라우팅을 suspendable 집합으로 키잉했더니 `foreach (b[i])` 가 `b.first(i)`/`b.next(i)` 로 desugar 되는 바람에 dyn-formal 함수 `packk` 이 재라우팅됐고 copy-out 경로는 dyn 배열 formal 을 바인딩 못 해 **동작하던 설계가 loud**(`dyn_formal_wrapped_call::blocking_ternary` 가 잡았다). 과대표시는 **라우팅 superset 에는 공짜, 호출 SHAPE 에는 유료**다 → 술어를 둘로 갈랐다(`sysfunc_frame_executor_cannot_perform` = 계열 − assoc 반복; assoc 반복은 **효과이지만** 키가 body-local 이면 `&self` 가 해내고 `fatal_frame_assoc_iter` 가 그 조건을 정확히 묻는다). ②**삽입 지점이 늦었다**: 라우팅 집합을 태스크 reject 단계에서 채웠더니 프레임 **태스크** 본문의 호출 자리는 이미 낮춰진 뒤였다(프레임 함수 본문과 달리 태스크 본문은 중첩 Call 이 허용된다) → 함수 본문 lowering **직후**·태스크 본문 lowering **직전**으로 이동. ③**`lower_lvalue` 를 부르는 새 arm 을 체인 맨 위에 놨다**: `s[i] = f(…)` 가 §6.16.3 이 경고하는 **조용한 packed BIT-write** 가 될 자리였다 → string-element/array 특수형 **아래**로 이동(실측 loud 유지).

**그리고 그 리뷰가 "이름 없는 조건"에 이름을 붙였다.** §4.5.275/276 이 두 번 시도해 두 번 되돌린 frame-body copy-out 패닉(rc=101 `frame lvalue net is routed`)의 진짜 조건은 `in_frame_body` 가 **아니라** **copy-out 목적지가 프레임 창 밖인가** 였다 — 5-probe 로 측정: frame-local 목적지(body-local·자기 output formal)는 PASS, 모듈 넷 목적지만 패닉. 중첩 TASK 호출의 `out_binds` 가 이미 하는 그 쓰기이므로 안전한 쪽은 처음부터 안전했다. 그래서 `x = f(args)` 가 프레임 본문에서도 동작한다(직접 호출 rhs · delay/event 없음 · 목적지 frame-local 일 때). 이것으로 **프레임 태스크 본문이 statement-effect 함수를 호출하는** 마지막 형태가 열렸다.

**검증(4994 green).** 행렬 = statement-effect 15종 × subroutine 6형(90) + 클래스 메서드 15 + 상호작용 8 + 패닉 5. **PRE(218dba2)/POST/iverilog 3-way 로 전수** — **회귀 0 · fixed 35**. 예제 4종 stdout+VCD **바이트 동일**(효과 없는 설계 무영향). 신규 테스트 20(`r22_stmt_effect_executor.rs`) + 사다리를 올린 r19 pin 1건 갱신(`a_file_read_inside_a_framed_body_is_fatal_not_a_quiet_zero` → `…_reads_the_line`). drift pin = `compute_effect` fall-through 의 `debug_assert!(!k_rhs_is_stmt_effect_family(rhs))` — 정본 계열과 실행기 arm 이 갈리면 debug 빌드가 잡는다.

**§3.4 `task static`/`function static`.** `static` 은 이 lexer 에서 **예약어가 아니다**(Verilog-2005 식별자 · per-decl lifetime arm 이 그 사실에 의존). 그래서 `Ident` 로 도착하고 **`static` 이라는 이름의 subprogram** 과 구별해야 한다 — 판별자는 **뒤 토큰**이다(lifetime 뒤엔 헤더가 더 오고, 이름 뒤엔 `;` 또는 `(` 가 온다). 결과는 양쪽 콜러에서 버린다: `static` 은 (non-`automatic` 모듈의) subprogram 기본 lifetime 이라 `automatic = false` 가 이미 주는 동작이다 = 순수 파서 수정.

#### 4.5.276 외부 round-20 — 내가 만든 회귀 하나와, 그것을 고치다 만든 silent-wrong 5건 (2026-07-30, branch feat-r20-report, format 26 불변) ✅

**리포트의 3건 중 §3.1 은 §4.5.275 가 만든 회귀였고, PRE 이분탐색으로 그것을 증명했다.** 리포터가 잰 것("`TB=partial` 85/85 → 진단 8 · `TB=sha2` 24/24 → 진단 11 · 진단 57 중 32건이 한 가족")은 정확했다. 세 SHA 로 바이너리를 짜서 재보니 `7dfd36f` PASS · `a5baeb3`(=§4.5.274) PASS · `8cf4165`(=§4.5.275) **error** — 내 직전 슬라이스다.

**근인 = 게이트의 SCOPE 와 KEY 가 둘 다 틀렸다.** §4.5.275 는 `hoist_stmt_top` **맨 위**에 `if self.in_frame_body() && !self.inout_func_names.is_empty() { return None; }` 를 뒀다. 프레임 본문에서 copy-out 을 못 내는 것은 **inout arm 의** 사정인데 함수 전체를 세웠고(SCOPE), 키는 `inout_func_names` 즉 **모듈 전역** 속성("이 설계에 output-formal 함수가 하나라도 선언됐나")이었다(KEY). 그래서 같은 match 의 **무관한 dyn-formal arm 전부**가 꺼졌고, **호출되지도 않는 함수의 선언 하나**가 `if (b2h(d) != "61")` 를 PASS→E3009 으로 바꿨다. 리포터가 잰 9개 경계(void·output 없음·task·다른 모듈=PASS / package·non-`automatic`·`inout`=error)는 전부 그 술어의 그림자였다. 수정 = stand-down 을 **두 inout arm 의 guard** 로 이동(`&& !self.in_frame_body()`) — 거기서는 arm 자신의 `expr_has_inout_call` 이 이미 "그 hazard 가 있다"를 말한다. 리포터의 실사이트 대역(package 에 `path_to_sha2_mode(input string, output logic[4:0])` 선언 + `bytes2hex` 8 호출)은 **E3009 7건 → 0건**, 값 전부 정답(iverilog 는 function output formal 을 거부해 hand-IEEE). **`|`검출법**: 게이트가 **statement 를 안 보고 답할 수 있으면** statement 단위 결정에 쓸 수 없다.

**§3.2 = "읽지만 그 값이 죽었다".** `inout` formal 의 copy-in 은 실제로 actual 을 읽지만(IEEE §13.5.2), callee 가 formal **전체**를 보기 전에 덮어쓰면 그 값은 아무도 관측하지 않으므로 v1 flatten 잔값이 보일 수 없다 → 그 호출은 actual 의 **쓰기**다. 함정은 재사용할 술어였다 — `automatic_local_definitely_assigned` 의 계약은 "**첫 쓰기 前에 읽기 없음**"(블록 로컬용 질문)이고, `if (c) r = 1;` 은 그것을 만족하면서 살아있는 경로에 쓰기가 없어 copy-out 이 잔값을 caller 에 돌려준다. 필요한 계약은 "**definitely written**"이라 전용 prefix walk 를 짰다(쓰기 前 모든 문장이 `stmt_cannot_escape` · 쓰기 後는 무제한). 스칼라·unpacked struct(멤버별) 양쪽 실측.

**§3.3 = trip-count 와 escape 는 별개 증명이고, `break` 는 walk 의 join 이 이미 지웠다.** `for (int j=0; j<3; j++) fill(cur);` 는 본문이 반드시 도는데 그것만으로는 부족하다 — `if (c) break;` 는 **루프 바로 뒤**에 착지하지만 `If` arm 의 `merge(Jumps, Falls(x)) = Falls(x)` 가 그 경로를 떨어뜨려 body 의 `DaOut` 에 흔적이 없다(`Jumps.state()` 가 join 항등원 `true` 인 것도 같은 함정 → `.state()` 대신 **`Falls(true)` 직접 매치**). 그래서 구문적 `stmt_cannot_escape` 를 옆에 세웠다. loop 4형(`for`/`while`/`repeat`/`forever`)을 **한 번에** 다뤘다 — `repeat (3)` 과 `for (…; j<3; …)` 은 같은 trip count 에 대한 같은 진술이고, 한쪽만 고치면 사용자가 바꿔 쓸 때 elaborate 여부가 바뀐다.

**적대 리뷰 1라운드 — 두 렌즈가 같은 구멍 하나에서 수렴하고, 각자 다른 구멍을 하나씩 더 찾았다(전부 loud→silent).** ①**body block 의 top-level `decls` 를 버렸다**: `match body { Block{stmts,..} => stmts }` 가 `decls` 를 떨어뜨려 `int save = r; r = fd;` 가 통과했고, **같은 읽기를 문장으로 쓴 `save = r;` 은 올바르게 거부**됐다 — 한 hazard 의 두 철자가 갈렸다(`da_stmt` 의 `Block` arm 은 중첩 블록의 decl-init 을 바로 그 이유로 검사한다). 실측: `obs` 가 caller 의 잔값 999(per-activation 참조는 0), exit 0. ②**`loop_runs_once` 가 루프가 실행되지 않는 도메인에서 답했다**: `const_eval_in_scope` 는 i64 부호 있는 fold 라 루프 변수의 선언 폭도 IEEE §11.8.1(피연산자 하나가 unsigned 면 비교 전체가 unsigned)도 안 본다 → `for (byte j=200; j>100; j--)`(200 이 signed byte 에서 -56) 과 `for (int j=-1; j<4'd3; j++)` 는 vita·iverilog **둘 다 0회**인데 fold 는 "≥1"이라 했다. `always @(posedge clk)` 2 활성으로 값 증명: 참조(automatic task local)=`act2 size=0`, flatten=`act2 size=2`(직전 활성 잔값). ③**resolver 가 lowering 과 달랐다**: `const_eval_in_scope` 의 `Ident` arm 은 `lookup_scoped`(params 전용·**net 무인식**)이라 generate 스코프 net `K` 가 localparam `K` 를 가려도 fold 는 localparam 을 봤다 — 기록된 classifier-vs-lowering-resolver 함정 그대로. ④**그 folder 를 callee 본문에 들고 갔다**: `inout_copy_in_is_dead` 가 `loop_once` 를 넘겨서 callee 의 formal/local 이 **caller 의 param 스코프**에서 접혔다 — `LIM`/`N`/`WIDTH` 를 module localparam 과 formal 양쪽에 쓰는 것은 평범한 코드다(실측 `a=1 q=999`). ⑤중첩 블록이 formal 을 **재선언**하면 그 쓰기는 다른 변수의 쓰기다(오늘은 flatten aliasing 때문에 안 틀리지만 증명이 그 우연에 기대면 안 된다). ⑥`inert_or_unknown` 이 **생략된 formal 의 default** 를 안 봤다(default 는 caller 스코프에서 낮춰지므로 formal 을 읽을 수 있다).

**수정의 핵심 결정 = fold 를 shadow-aware 로 만들 수 없다.** `walk_scopes_key_shadowed` 의 계약이 **"`const_eval_in_scope` 에서 도달하는 consumer 는 절대 opt-in 하지 마라"**고 못박고 있다 — `symbols` 는 elaboration 中에 채워지므로 순서 의존 답이 되고, 과거에 그것이 **generate body 를 통째로 조용히 삭제**했다(exit 0). 그래서 순서 무관한 답을 골랐다: **식별자를 아예 허용하지 않는다**. `fold_domain_is_literal` = allow-list(unsized decimal `0..=i32::MAX` · Paren/Unary/Binary/Ternary만) + `for` init 은 **정확히 0**(0 은 모든 폭·부호에서 0 이므로 절단 축이 사라진다) + sized/based 리터럴 금지(부호 축이 사라진다). 리뷰어가 만든 counterexample 5종 전부 loud 화되고, 부수적으로 `repeat (8'd128 + 8'd128)`(vita 256 / iverilog 0 = **pre-existing** engine trip-count 결함)에 의존하지도 않게 됐다. **대가는 정밀도**: `for (int j=0; j<NN; j++)` 의 localparam 경계는 이제 정직한 loud 다(승격 전제 = 순서 무관·AST-gathered 이름 집합 = ROADMAP §2 의 inner-net-shadow 와 같은 선행조건). callee 쪽은 `loop_once: &|_| false`.

**재리뷰 2라운드가 또 5건을 냈다 — 설계를 바꿨으니 필수였고, 그 값이 여기서 나왔다.** ①**decl 리스트를 잘못 읽었다**: tf-body 의 top-level 선언은 body `Block` 이 아니라 `FunctionDef/TaskDef::body_decls` 에 산다(`tf_body` 가 래퍼를 `decls: Vec::new()` 로 만든다) — 그래서 1라운드에서 넣은 decl-init 검사가 **항상 빈 리스트**를 보는 죽은 코드였고, 정본 철자인 top-level `int save = r;` 는 여전히 통과해 caller 의 심어둔 잔값을 돌려줬다(`q=1009`, per-activation 로컬은 10). 같은 읽기가 **한 블록 더 깊으면 거부·문장으로 쓰면 거부** — 한 hazard 의 세 철자 중 둘만 맞았다. 형제 술어 `callee_body_cannot_touch` 는 처음부터 옳은 리스트를 읽고 있었다. ②**리터럴만 허용한 것으로는 부족했다 — LEAF 를 묶고 RESULT 를 안 묶었다**: `const_binop` 은 i64 `checked_*` 라 `65536*65536`·`2147483647+1`·`2**32`·`1<<32` 가 32비트 도메인을 벗어나고, 엔진은 그것들을 **0회**로 돈다(iverilog 일치). 5가지 철자 전부 loud→silent 였다. 그래서 `const_eval_in_scope` 를 아예 버리고 **checked i32 자체 평가기**(`fold_i32`)로 바꿨다 — `**`/`<<`/`>>`/`/`/`%` 는 도메인이 다르다고 `const_binop` 자신의 주석이 증언하므로 **연산자 목록에서 제외**. ③**omitted formal 의 DEFAULT 를 놓쳤다**: `call_effect` 의 `Inert` 에는 R19 이 넣어둔 명시 절이 있었는데(default 는 caller 스코프에서 낮춰지므로 formal 을 읽을 수 있다) `callee_body_cannot_touch` 는 `ports[i].default` 를 안 본다 → `q = rd() + nxt(5,o)`(`rd(input int x = o)`)가 조용히 틀린 값. default 는 이 표현식 밖(callee 의 포트)에 있어 스냅샷이 닿지 못하므로 **loud** 가 정답이다. ④**내가 만든 loud 회귀**: `enclosing_call_cannot_read` 가 `call_is_inert` 의 **2-세그먼트 arm** 을 안 물려받아 `q = qq.size() + nxt(5,o)`·`q = ss.len() + nxt(5,o)` 가 동작→loud 로 내려갔다. 그 옛 arm 은 **클래스 메서드에는 부당**했지만(본문이 계층 경로로 모듈 넷에 닿는다 — PRE 에서 `q=12`, 정답 11 = **pre-existing silent-wrong**) **컨테이너 메서드에는 타당**하다(유저 본문이 없다). `container_method_is_pure` 로 갈라 built-in 은 회복하고 클래스 메서드는 정직하게 loud — 즉 한 수정이 회귀를 되돌리면서 pre-existing silent-wrong 하나를 loud 로 올렸다. ⑤**pre-existing 패닉**: `lower_loop_cond_operand` 에 frame-body stand-down 이 없어 루프 조건이 프레임 본문에서 copy-out 을 낼 마지막 경로였고, 오보가 아니라 **엔진 assert 를 밟았다**(rc=101). PRE 바이트 동일. 물러나게 해서 호출 자신의 진단으로 바꿨다. **그리고 리뷰어의 오라클도 실측 대상이다** — ③에서 리뷰어는 정답을 `q=56` 이라 했지만 그것은 post-call 값이고, iverilog task 쌍둥이는 `q=13`(pre-call 읽기)을 준다. 메커니즘은 옳고 숫자는 틀렸다.

**3라운드가 또 6건을 냈고, 그중 셋은 2라운드 수정이 만든 것이었다.** ①**내 decl 검사가 여전히 뚫렸다 — 이번엔 깊이로**: 의무 3("쓰기 前에 아무도 formal 을 못 읽는다")을 구문 워커로만 걸었는데 `expr_no_ref_deep` 의 `Call` arm 은 callee 본문에 안 들어가고 `da_stmt` 는 `stmt_no_ref` 가 "이 이름 언급 없음"이라 하면 문장을 통째로 건너뛴다 → `int save = rd();`(`function rd(); return r;`)가 copy-in 을 읽고 caller 의 잔값 999 를 돌려줬다(**직접 철자 `int save = r;` 는 계속 거부**). 2단계(`rd2()→rd()`)까지 확인. 닫는 법 = 이미 `_`-free-exhaustive 인 `stmt_may_write_ident` 를 **direct-lvalue 테스트만 opt-out** 하도록 파라미터화해 재사용(`stmt_may_observe_via_call`) + 벳한 callee 가 **또 호출하면 거부**(`stmt_makes_any_call`). ②**2-세그먼트 완화가 이름만 봤다**: `container_method_is_pure` 는 **메서드 이름 문자열 whitelist** 일 뿐 수신자가 컨테이너인지 안 본다 → `size`/`len`/`min`/… 로 이름 지은 **유저 클래스 메서드·자식 인스턴스 함수·모듈 자기 함수**가 34개 중 30개 이름에서 통과했다(PRE 도 12 = pre-existing 이지만 2라운드 수정이 `get` 하나만 닫았다). 수신자를 **적극 식별**(`lookup_net_scoped` → 컨테이너/문자열 `NetKind`)해서 닫았다 — 클래스 핸들은 정수 넷이고 인스턴스·모듈 이름은 넷이 아니다. ③**내 frame-body stand-down 이 과발화**: `lower_loop_cond_operand` 는 이름과 달리 §4.5.216 의 `?:`/단락-rhs 변환 피연산자도 낮춘다 — 그리고 **그것들은 frame task 본문에서 정상 동작한다**(선택되지 않은 경로에서 copy-out 이 안 터지는 것까지). 공유 헬퍼에 가드를 달아 4개가 loud 로 내려갔다 → **두 진짜 루프 조건 자리**로 이동(§3.1 과 같은 교훈의 한 단계 아래). ④**패닉의 진짜 마지막 경로는 따로 있었다**: `emit_frame_func_out_call` 의 세 호출부 중 **문장 위치**(`f(out);`/`void'(f(out));`)가 무가드라 frame task 본문에서 rc=101(PRE 동일) — 내 2라운드 주석의 "마지막 경로" 주장이 틀렸다. 가드해서 loud 화. ⑤`Forever => true` 는 **외부 `disable`** 이 뒤 문장에 닿을 수 있어 검사 불가 → `false`(비용 0). ⑥진단 2건: `?:` 는 **위치가 아니라 callee 부작용**이 판별자였고(문장 수준 `$display` 가 있으면 loud), "select 또는 lvalue INDEX"는 실제로는 select 의 **BASE** 였으며 "재귀 호출"은 **rhs 전체일 때는 동작**한다.

**4라운드가 또 4건 — 그리고 그중 둘은 내가 3라운드에 넣은 수정 자체였다.** ①**`body_decls` 교훈을 바깥 callee 에만 적용했다**: 중첩 callee 를 보는 `stmt_makes_any_call` 이 `f.body` 만 걸어서 tf-body **top-level decl-init** 에 적힌 호출(`int tt = rd();`)이 안 보였다 → 3라운드가 막 닫은 silent-wrong 이 한 겹 안쪽에서 그대로 재현(활성 2 에서 심어둔 999). 같은 파일 30줄 위에 "**두 리스트 다, 그리고 중요한 건 `body_decls`**"라고 내가 적어 둔 채로. ②**`array_len != 1` 이 순수 부채였다**: vita 는 **unpacked-array 넷을 head 로 가진 `x.m()` 을 계층 함수 호출로 라우팅**하므로 그 절이 임의의 유저 본문을 통과시켰다(generate 안 인스턴스 `mem` 이 배열 `mem` 을 가리는 실측 케이스 = iverilog `q=11` / vita `q=12`). 그리고 **얻는 것이 없었다** — 고정 배열 메서드 9종은 전부 loud 라 진짜 built-in 은 그 절이 필요 없다. 삭제. ③④**frame-body stand-down 을 두 번 시도해 두 번 되돌렸다**: `in_frame_body()` 는 패닉의 조건이 **아니다** — 12 형태 중 **10 개가 PRE 에서 정답**이었고(iverilog 오라클) 그중엔 리포트의 `.rsp` 워커를 `task automatic` 으로 옮긴 것도 있었다. 패닉은 호출이 프레임 본문의 **첫 문장**일 때만 나고 앞에 문장이 하나만 있어도 사라진다. 패닉은 loud 이지 silent-wrong 이 아니므로 **정직한 pre-existing 으로 되돌리고**(ROADMAP §2 에 측정된 형태와 함께 등재) 10개의 correct-support 를 지켰다. 남은 정밀도 손실 하나는 **의도된 교환**으로 기록했다 — 2-세그먼트 호출 중 내장 컨테이너만 통과시키므로 자기 멤버만 읽는 클래스 메서드와 `pkg::f()` 가 loud 다(PRE 는 그 자리에서 **무조건 조용히 틀렸다**).

**5라운드가 1건 — 그리고 그것도 같은 함정이었다.** 수신자를 적극 식별하려고 `lookup_net_scoped(&recv.name)` 를 썼는데, **라우팅된 고정 `string` 배열**은 넷이 **mangled 이름**(`<name>$sad`)으로 등록된다(선언 이름을 모듈 네임스페이스에 **비워 두는 것이 의도**다 — v1 이 블록 로컬을 bare name 으로 flatten 하므로). 그래서 선언 이름이 아무것도 못 찾고 `string rv[3]; rv.size()` 가 동작→loud(48 형태). 즉 **또 classifier-vs-lowering-resolver** 였다. 수정 = lowering 자신의 리졸버(`dyn_handle`, 사이드맵을 shadow-aware walk 로 먼저 보고 `symbols` 로 폴백)를 쓰고, dyn 핸들이 아닌 스칼라 `string` 만 따로 해소. 5 수신자 클래스 전부 PRE==POST 로 복구되고 4개 unsound arm 은 그대로 loud. 리뷰가 **내 주석의 근거도** 잡았다 — "앞에 문장이 하나라도 있으면 패닉이 사라진다"는 실측 반례가 있었고, 그 주석은 트리에 패닉을 남겨 두는 **근거**였으므로 지우지 않고 정정했다(조건은 아직 이름 붙이지 못했다고 명시).

**6라운드가 2건 — 그리고 둘 다 "이미 증명된 것을 다시 증명하려 했다"였다.** ①**`pkg::f()` 를 통째로 잃을 뻔했다**: 2-세그먼트 arm 을 "컨테이너 수신자만"으로 조인 것이 모든 패키지 호출을 잡았는데, `inline_pkg_function` 은 **self-contained·straight-line 패키지 함수만** 받아들이고 모듈 넷을 읽거나 제어 흐름이 있는 본문은 **거기서 이미 loud** 다(PRE·POST 실측) — 그게 바로 이 술어의 의무이고 **상류에서 이미 이행됐다**. arm 추가로 `q = pk::h(3) + nxt(5,gv)` 가 PRE 값(12)으로 복귀. ②**callee 본문을 caller 쪽 규칙으로 걸었다**: `da_stmt` 는 `expr_no_ref`(head-segment only = **caller 쪽** 규칙)로 이름을 풀고, 같은 함수의 형제 검사들은 all-segments 규칙을 쓴다 → 또 **한 hazard 의 두 철자가 갈렸다**(`save = t.r;` 는 통과·`int save = t.r;` 와 `save = r;` 는 거부, 통과한 쪽이 죽었다고 선언한 copy-in 을 읽었다). 쓰기 문장만 면제하고 prefix 에 deep 규칙을 걸어 세 철자를 일치시켰다. (리뷰어도 **합법 SV 증인은 못 만들었다** — `t.<block-local>` 은 iverilog 가 거부하는 vita 의 기존 laxity라, 오늘은 silent-wrong 이 아니라 "거부했어야 할 설계를 받아들임"이다. 그래도 ACCEPT 게이트의 명시된 의무는 지켜야 한다.)

**진단 문구가 한 번 더 갈렸다** — `?:` arm 을 무조건 지원으로 적었는데, 실제 판별자는 **위치가 아니라 callee** 였다: 본문에 문장 수준 `$display` 가 있는 dyn-formal 함수는 `?:` arm 에서 loud 다(조건부 평가를 hoist 하면 선택되지 않은 arm 에서 부작용이 일어난다). 같은 위치·같은 caller·다른 callee 두 개로 핀했다.

**리뷰가 진단 품질 3건도 잡았고, 그중 하나는 내가 방금 쓴 문구였다.** 새 dyn-formal 메시지가 "한 표현식에서 같은 함수 2회"를 원인으로 열거했는데 **실측 지원·정답**이다(`h={b2h(d), b2h(e)}`→`6162` · `{b2h(d), b2h(d)}`→`6161` · iverilog 일치 — 각 호출이 자기 temp 로 hoist 되어 슬롯이 공유가 아니라 **순차 재사용**). 그리고 실제로 남은 자리 중 `while`/`for` **조건** · **delay 표현식** · **system FUNCTION 인자**(`$sformatf`)가 어느 목록에도 없었다. **첫 행렬이 틀린 이유가 교훈이다** — `function int cnt(input byte b[]); return b.size(); endfunction` 은 **INLINE** 되어 이 기구를 아예 안 타므로 그것으로 만든 행렬은 아무것도 측정하지 못한다. 프레임화되는 callee(`string` 반환+`foreach`+`$sformatf`)로 25-위치를 다시 재서 양쪽 목록을 고쳤다.

**frame FUNCTION 본문의 진단이 하강했던 것도 리뷰가 잡았다.** dyn arm 이 프레임 본문에서 도달 가능해지자 `fresh_ret_temp` 의 **모듈 넷** temp 로 hoist 했고, frame function 본문은 그것을 쓸 수 없어 "body uses an assignment to a net outside the function"(사용자가 쓰지 않은 temp)을 냈다. 네 dyn arm 에 `!self.frame_fn_lowering` 을 달아(`S::Return` arm 이 원래 갖고 있던 것과 동일) 정확한 dyn-formal 메시지로 되돌렸다 — **능력 손실 0**(그 자리들은 어느 쪽이든 loud) 이고 PRE 의 no-trigger 열보다도 개선이다. frame **TASK** 본문은 게이트하지 않는다(그게 §3.1 이 되살린 것).

**pre-existing 1건을 doc 주장 검증 중에 발견해 같이 닫았다.** 매뉴얼이 "another call's argument" 를 지원으로 적었지만 `q = other(nxt(fd,r2));` 는 PRE 에서도 loud 였다(`$signed(nxt(…))` 와 `tk(nxt(…))` 는 동작). 근인 = `order_walk` 의 opacity 검사가 `call_effect` 를 물었고 그 `Inert` 는 **그 호출 자신의 actual** 이 후보 free 일 것도 요구하는데, 여기서 actual 이 **바로 그 쓰기를 하는 중첩 호출**이다 → 감싸는 호출의 inertness 가 계획 중인 hoist 때문에 부정되고 위험이 unrepairable 로 읽혔다. opacity 의 질문은 **callee 본문뿐**이므로(actual 은 이미 `shape` 자식으로 걸어진다) `callee_body_cannot_touch` 로 바꿨다. 판별자가 살아 있음도 확인 — 본문이 `r2` 를 읽는 `reads(nxt(fd,r2))` 는 계속 loud.

**검증.** 4078-design PRE/POST 스윕 **차이 13 = 전부 진단 문구**(에러 코드·에러/경고 카운트까지 동일)(회귀 0; 그중 `f1233` 은 위 frame-function 개선이 코퍼스에 나타난 것). corpus 커버리지 실측(`automatic` 795 · `for` 200 · `repeat` 43 · `inout` 52)이라 스윕은 vacuous 하지 않다. 리포트 3건 + 실사이트 대역 + staged `vcmp→velab→vrun` 패리티. `cargo test` **4973**(신규 31 · 그중 23개가 리뷰 발견 핀) · clippy 0 · fmt clean · format_version 26·frozen 타입 불변. 모듈 캡: `da/loops.rs`(80)·`block_local/proofs.rs`(434) 신설로 `da/mod.rs` 957 · `gate.rs` 608. DA 워크의 4 전달 인자는 `DaCtx` 구조체로 묶었다(clippy `too_many_arguments` 가 같은 것을 가리켰다).

#### 4.5.275 값을 반환하는 output-formal 호출을 아무 표현식 위치에서나 (2026-07-30, branch feat-r19-value-call-out-general, format 26 불변) ✅

**round-19 리포트(§3.1/§3.2/§3.3)는 HEAD `a5baeb3`에서 전부 재현되지 않았다** — §4.5.274 가 이미 닫았고, 34건 각각을 다시 돌려 확인했다(`go = nxt(5,r)` · `if (nxt(5,r)==1)` · `while (n<lim && rsp_next(fd,r)==1)` · `void'(nxt(5,r))` · named arg). **남아 있던 것은 §4.5.274 가 닿지 못한 *자리*들**이었고, 그것을 35-케이스 행렬로 측정하니 **pass 14 / honest-loud 21 / wrong 0** 이었다. correct-or-loud 는 지켜져 있었으나 **21건이 false-loud** 였다.

**진단의 핵심 = §4.5.274 가 연 자리는 전부 "문장 모양"의 부분집합이었다.** 맨몸 호출 문장 · 직접 rhs · 맨몸 조건 · while/for 조건의 top-level `&&`/`||` 한 피연산자. 호출이 값을 반환하면 표현식이 갈 수 있는 **아무 데나** 가는데, 나머지 전부(concat 조각 · 다른 호출의 인자 · select 인덱스 · cast 피연산자 · `case` scrutinee · `repeat` 카운트 · `$display` 인자 · task 인자 · NBA rhs · `return` 값 · lvalue 인덱스 · 더 깊이 묻힌 `?:` arm 과 `&&` 우변)는 hoist site 가 아예 없었다.

**수정 = 범용 hoist(`hoist/general.rs` 신규 + `hoist/general_stmt.rs`).** 노드마다 "어떤 자식을, 어떤 순서로, 어떤 조건에서 평가하는가"를 **한 곳(`shape`)에만** 적고, 탐지기·도달성 게이트·평가순서 게이트·변환기 **네 워커가 그것을 공유**한다 — 분류기와 lowering 이 갈리는 실패(ENGINEERING_RULES 의 기록된 함정)를 구조적으로 막는 배치다. 극성은 **답할 수 없으면 물러난다**: `Opaque` 노드는 탐지기가 "있다"로 보수 응답하고 도달성 게이트가 거부하므로, 모르는 payload 안의 호출은 조용히 빠지지 않고 loud 로 남는다.

**조건부 자리는 노드를 그대로 두고 copy-out 만 guard 블록에 넣는다.** 좌항/조건의 진리값을 1비트 temp 에 포착하고, `포착값 !== <단락값>`(`&&`=0 · `||`=1 · `?:` then-arm=0 · else-arm=1)일 때만 들어가는 블록에서 copy-out 을 emit 한다. case-inequality 이므로 **x 는 블록에 들어간다** — 그것이 바로 IEEE 가 평가하는 경우다(`log_and(x,B)` 는 B 가 필요하고, x 조건의 `?:` 는 두 arm 을 다 평가한다). 건너뛴 경로에서 temp 는 기본값으로 읽히지만 선택되지 않는다(`log_and(0,·)=0` · `log_or(1,·)=1` · 확정 조건의 `?:` 는 반대 arm). **노드를 유지하는 것이 `?:` 에서 결정적이다** — arm 이 바깥 문맥으로 결정되는 성질이 그대로 남아, arm 을 고립시키던 변환이 막아야 했던 §4.5.217 부호/폭 발산이 **애초에 생기지 않는다**.

**그래서 §4.5.217 이 loud 로 막아 둔 3건이 loud → correct-support 로 올라갔다.** 등가 차분으로 값을 확인했다: 부호 불일치 arm = **`x=0a`**(§11.8.1 unsigned zero-extend, 고립 변환이 냈을 `0xfa` 아님) · 폭 불일치 arm = **`x=fd`**(통합 폭 16 에서 sign-extend 후 `>>1`, 고립이 냈을 `0x7d` 아님) · coercion-unsafe 통합 = `mine=7`(호출 없는 쌍둥이와 `===`). §4.5.216 특수 경로는 `sc_rhs_owned` 로 **소유권을 그대로 유지**했고(원래 guard 를 그 함수로 추출해 특수 경로와 범용 경로가 **같은 술어 하나**를 본다), 범용 arm 은 `hoist_stmt_top` 의 **맨 마지막**에 둬서 위의 모든 arm 이 바이트 동일하게 남는다.

**평가 순서는 읽기가 호출의 어느 쪽인지로 갈린다(실측).** 오른쪽 읽기는 안전하다 — 소스도 호출 뒤에 읽는다(`q = nxt(5,o) + o` = `6+50` · `if (nxt(5,o)==6 && o==50)` = taken). 그것을 거부한 것이 리포트의 `.rsp` 워커에 `r.len` 읽기를 붙이면 loud 가 되던 이유였다. **왼쪽 읽기는 pre-call 스냅샷으로 살렸다** — 호출 앞에 `snap = v`(넷 전체 복사)를 emit 하고 왼쪽 읽기를 `snap` 으로 바꾼다(`q = o + nxt(5,o)` = `7+6` = **11**, iverilog 일치). 못 고치는 두 경우는 **정직한 loud** 로 남겼고 이유를 메시지에 적었다: 스냅샷 대상이 평범한 비트벡터 넷이 아닐 때(unpacked 배열/struct 루트는 넷 하나로 복사 불가) · **한 표현식에서 두 호출이 같은 루트를 쓸 때**(세대가 둘이라 스냅샷 하나로 안 된다).

**게이트를 열자 DA 워커의 false-loud 가 드러났다.** `expr_da` 의 `&&` arm 은 좌항이 `Clean`(= 조건부 쓰기 포함)이고 우항이 `Reads` 면 `Reads` 로 끝냈는데, 중첩 `&&` 가 정확히 그 모양이다(`(n<10 && rsp_next(n,r)==1) && r.len>=0` — 안쪽 우항이 쓰므로 안쪽 판정이 `Clean`). 우항의 읽기는 **그 조건부 쓰기가 일어난 경로에서만** 도달하므로 read-before-write 가 아니다 → `expr_writes_when(lhs, ·, op==LogAnd)` 로 물어 `Writes` 로 승격. **x 좌항까지 건전한 이유를 실측했다**: `expr_writes_when` 이 참을 낼 수 있는 길은 좌항 자신의 단락 체인을 타고 "평가되면 반드시 쓰는" 잎까지 내려가는 것뿐이고, **이 우항에 도달했다는 것 자체가 좌항 체인이 단락하지 않았다는 뜻**이라 그 잎도 평가됐다(좌항 x → `r=77` 로 쓰기 확인 · 좌항 0 → 우항 자체가 평가되지 않아 읽기도 없음).

**진단 메시지를 실측 기준으로 다시 썼다.** 옛 문구는 지원 위치를 열거하며 "`?:` arm 과 더 깊은 중첩은 미지원"이라고 적었는데 그게 더 이상 사실이 아니다(반대 방향의 stale claim). 새 문구는 **남은 것만** 적는다 — 연속 재평가 표현식(`assign`/`force`/`wait` 조건: copy-out 이 변화마다 다시 터질 수 없다) · intra-assignment delay · `min:typ:max`/제약·`with` · 위의 평가순서 2경우 — 그리고 우회로(`t = f(...);`)를 제시한다. 남은 목록은 **추측이 아니라 10형태 실측**으로 정했다(`foreach` 는 loud 로 보였지만 실은 무관한 pre-existing DA 게이트였다 — 모듈 레벨 배열로 바꾸니 `o=20 a2=3`).

**적대 리뷰 6 라운드 × 2 렌즈 — 내 수정에서 silent-wrong 6건 + 하강 2건이 나왔고, 두 렌즈가 가장 중요한 것 하나에서 수렴했다.** 그 하나는 **DA 판정을 너무 강하게 준 것**: 중첩 `&&` 의 조건부 쓰기에 `ExprDa::Writes` 를 줬는데 그 판정의 계약은 "**모든** 평가가 쓴다"이고 좌항이 단락하면 아무것도 안 쓴다. 그래서 게이트가 단락 경로에서도 로컬을 assigned 로 봐 **같은 이름 형제 블록의 잔값을 exit 0 에 읽었다**(vita `else r=777` / 신선한 automatic 이면 기본값). 옳은 판정은 `Clean` — 읽기는 안전하고 쓰기는 약속하지 않는다. 나머지 5건: ①`$bits` 는 피연산자를 **평가하지 않는데**(IEEE §20.5) hoist 가 copy-out 을 태워 소스에 없는 부작용을 냈다(`o` 가 7→50) ②`$monitor`/`$strobe` 는 인자를 **나중에 다시 렌더**하므로 hoist 가 temp 를 얼려 매 변화마다 같은 값을 찍었다(§4.5.250 이 이미 닫아 둔 클래스 — 목록을 **한 벌로 공유**해 해결) ③인자 리스트·rhs↔lvalue 인덱스·인덱스↔인덱스를 **따로** 분석해 경계를 넘는 위험을 못 봤고(`$display("%0d %0d", o+0, nxt(5,o))` 가 `50 6`, iverilog `7 6`) 두-호출 가드까지 무력화했다 → **문장 전체를 한 시퀀스로** ④포착 피연산자의 스크래치 집합을 **빈 것으로 시작**해 그 안의 호출이 왼쪽 읽기를 못 봤다(`q = o + (nxt(5,o) && 1)` 가 51, iverilog 8) ⑤frame body 안에서 hoist 가 copy-out 을 내자 분류기가 **소스에 없는 이유**를 보고했다("timing/suspend/fork control"). 하강 2건은 리뷰가 잡은 게 아니라 **리뷰가 열어 준 것**이다 — 계층 읽기(`top.o`)와 **callee 본문 안의 읽기**는 PRE 에서도 조용히 post-call 값이었고(`q=56` / iverilog 13) 둘 다 **pre-existing silent-wrong**이었다.

**그 두 개를 닫으면서 안전 게이트가 하나로 합쳐졌다.** `hoist_is_safe`+`reads_ident_outside_inout`(이름별·비방향·단일 세그먼트만·callee 본문 안 봄)을 **삭제**하고, 좁은 hoister·§4.5.216 arm 변환·범용 hoister 전부가 `order_clean`(= `order_plan` 이 위험 0) 하나를 본다. 수리 불가 위험은 별도 채널(`opaque`)로 분리했다 — 계층 경로, 그리고 `call_effect` 가 `Inert` 를 증명하지 못하는 callee 본문. **`Inert` 증명이 있는 callee 는 비용이 없다**(`q = h(5) + nxt(5,o)` 는 계속 동작) — R16 §3.2 의 기존 리졸버를 그대로 재사용했기 때문이다. 클래스 메서드 본문은 해소 불가라 loud 로 남고(`y = c.m(x) + f(x)`), 그 테스트는 값 핀에서 loud 핀으로 되돌렸다(사유를 주석에).

**리뷰가 드러낸 pre-existing 2건도 같이 닫았다**: ①포착 진리값이 `x || x` — 같은 expr id 를 **두 번** 읽으므로 `$random` 피연산자가 두 번 뽑혀 시퀀스가 어긋났다 → `!!x`(같은 4-state 축약, 평가 1회)로. §4.5.216 의 두 자리도 같이. ②`fresh_ret_temp` 이 문자열 아닌 모든 반환에 `Reg` 넷을 만들어 **`real` 반환이 정수 도메인으로 반올림**됐다(`return 1.5` → `2.000000`, **직접 rhs 에서도**, exit 0) → `NetKind::Real` f64 넷으로(iverilog `1.500000` 일치).

**재리뷰(설계를 바꿨으니 필수) — 2 렌즈가 또 6건, 그중 하나는 PANIC.** ①**frame body 판별자가 틀렸다**: `frame_fn_lowering` 만 봐서 `task automatic` 본문(=`frame_task_lowering`)에서 copy-out 이 emit 되고 엔진의 `debug_assert!(frame_local[net])` 를 밟았다 — **진단 없이 exit 101**, release 면 assert 가 빠져 **다른 넷에 쓴다**. 좁은 경로에도 같은 구멍이 **pre-existing** 으로 있었다(`r = nxt(5,gv);` 가 PRE 에서도 패닉) → 두 flag 를 다 보는 `in_frame_body()` 를 **양쪽 arm 에** 걸어 panic→loud. ②**`subst_pre_call_reads` 가 output actual 을 스냅샷으로 바꿨다** — 문장 자기 call 의 write **목적지**를 읽기로 취급해 callee 의 copy-out 이 스냅샷 넷에 떨어지고 **사용자 변수는 낡은 값**(vita `o=50` / iverilog `o=60`). **쓰기 사라짐**. 방향을 물어(`callee_arg_dirs`) `input` actual 만 시퀀스에 넣고, `inout` 은 copy-in 이 **읽기이면서 목적지**라 스냅샷으로 못 고치므로 stand-down. ③**`Opaque` 가 아무것도 기록하지 않았다** — 그 정당화("범용 경로는 어차피 거부")는 `order_clean` 이 **좁은 경로의 게이트이기도** 하기 때문에 성립하지 않았고, 삭제된 `reads_ident_outside_inout` 의 `_ => true` catch-all 이 막던 것이 그대로 열렸다(`(1:gv:3) + nxt(5,gv)` 가 12, iverilog 56). `Opaque` 를 **`NoHoist`(평가됨·읽음) / `Unevaluated`(`$bits` — 평가 안 됨)** 로 쪼개고 자식을 **명시**했다. ④그 쪼갬이 **false-loud 도 고쳤다**: 탐지기가 `Opaque` 에서 무조건 "있다"고 답해 **호출이 없는** `$bits`/`min:typ:max` 가 인자로 있기만 해도 문장 전체가 stand-down 했다(PRE 는 `nxt($bits(gv), gv)` 를 정상 처리). ⑤**계층 읽기를 세그먼트 철자로** 오염시켜 무관한 자식 스코프(`sub.gv`)가 부모의 `gv` 를 죽였다 → **self-path 만** 별칭일 수 있다(플래튼은 bare name 이므로 `t.gv`≡`gv`, `sub.gv`는 다른 넷). ⑥**candidates 를 좁은 워커로** 모아 concat/인자 경유 호출에서 집합이 **비었고**, 그것만 소비하는 callee-body/method-body 불투명 검사가 아예 안 돌았다 → shape 기반 + **named-arg 인식**(`.formal(o)` output actual 이 위치-zip 때문에 통째로 안 보였다 — pre-existing, 두-호출 가드까지 무력화).

**그리고 재리뷰가 파서의 pre-existing silent-wrong 을 하나 더 드러냈다** — typedef 된 return 타입을 return 필드로 옮기는 자리가 `int`/`integer` 만 `ParamType` 에 매핑하고 **`real`/`realtime` 은 안 했다**. `ParamType` 이 반환의 실수성을 기록하는 **유일한** 곳이라 `typedef real myreal; function myreal f(…)` 는 정수 반환이 됐다(inline 은 반올림, frame return temp 를 지나면 **0.0**). AST **값**만 바뀌므로 format/해시 불변.

**검증**: 35-케이스 행렬 **pass 35 / loud 0 / wrong 0**(기대값은 전부 iverilog 실측 — 함수 output formal 은 iverilog 가 거부하므로 §4.5.274 와 같은 **합성 오라클**: 모듈 넷을 쓰는 함수로 순서·조건부 평가를 고정, 매핑은 task 로) · **하베스트 코퍼스 4074 designs PRE/POST 2회**(PRE = `git archive main` 별도 빌드) — **회귀 0**, 차이는 전부 loud→정답 · silent-wrong→loud · 진단 문구 · 등가 차분(hand-hoisted 쌍둥이) 다수 일치 · iverilog 직접 대조(`y=11 x=6` · `o=60` · `arr1=3 o=0` · `q=8` · `q=13 gv=6` · `1.500000` · `3.750000`) · **4942 tests green**(신규 `output_formal_any_position.rs` **33** — 그중 **17개가 두 리뷰 라운드의 발견 핀**, 옛 loud 핀 7건을 값 핀으로 전환하고 각 주석에 이유) · clippy 0 · fmt clean · **format_version 26·AST 스키마 해시 불변**. 모듈 사이즈: `hoist/mod.rs` 977 → 689 · 신규 `general.rs` 859 · `general_ast.rs` 339 · `general_query.rs` 215 · `general_stmt.rs` 205(전부 1000 이하).

**잔여**(전부 정직한 loud·진단 메시지에 명시): 연속 재평가 표현식(`assign`/`force`/`wait` 조건) · intra-assignment delay · `min:typ:max`/제약/`with` **안의** 읽기(치환이 못 닿는다) · 계층 self-path 또는 해소 불가 callee 본문에서의 왼쪽 읽기 · 비-비트벡터 위험 루트 · 한 시퀀스에서 같은 루트를 쓰는 호출 2개 · frame body 안의 모든 위치 · `inout` actual 의 루트를 형제 인자가 쓰는 경우. **후속 후보**(ROADMAP §3): 클래스/2-세그먼트 메서드 본문 해소 · frame body 안 copy-out(엔진 쪽 작업).

#### 4.5.274 외부 round-19 — 값을 반환하는 호출의 output actual, 그리고 그 밑의 silent-wrong 2건 (2026-07-29, format 26 불변) ✅

**리포트 34건 중 33건이 §3.1 하나였고, 트리거는 리포트가 짚은 그대로 "호출이 값을 반환하는가"였다.** BL4 가 output actual 의 쓰기를 인정하는 자리는 딱 둘 — 맨몸 호출 **문장**, 그리고 `!`/`(…)` 를 벗기면 호출인 **조건 전체**(`cond_out_writes`) — 이고, 그 둘은 *문장 모양*의 부분집합이다. 호출이 값을 반환하는 순간 그것은 표현식이 갈 수 있는 아무 데나 갈 수 있는데, 거기서 워크가 묻는 것은 `expr_no_ref` 하나뿐이었고 **그 워커에게 "언급"은 언제나 읽기**다. 그래서 `go = nxt(5, r);` · `if (nxt(5,r) == 1)` · `while (n < lim && rsp_next(fd, r) == 1)` 셋 다 거부됐고, R18 이 `automatic` unpacked struct 의 lifetime 을 되살리자마자 그 게이트가 도달 가능해져 **`TB=sha2` 가 24/24 → elaborate 불가**로 회귀했다.

**수정 = 표현식 단위 효과 워크**(`da/expr_effect.rs` 신규). `ExprDa::{Clean, Writes, Reads}` — `Clean` 은 "이 표현식을 평가하는 동안 읽기는 일어날 수 없다"이고 **조건부 쓰기는 Clean 에 포함**된다(쓰기는 읽기가 아니고, 주장하지 않는 쪽이 보수적이다). 노드 집합은 **로워링이 copy-out 을 hoist 하는 집합과 같게** 맞췄다(`hoist_inout_calls` = Paren/Unary/Binary, +Ternary) — 그 밖의 노드는 pre-R19 답(`expr_no_ref`)을 그대로 낸다. `&&`/`||` 만 평가 순서가 고정이라(IEEE 1800 §11.4.7) 좌항의 쓰기는 우항의 읽기보다 **먼저**임을 말할 수 있고, 그 밖의 이항 연산자는 순서가 안 정해져 있어 반대편에 읽기가 있으면 주장을 접는다.

**그리고 분기는 조건의 *값*을 안다**(`expr_writes_when`). `a && f(r)` 이 **참**이면 두 피연산자가 모두 평가됐다 — 그래서 루프 **본문**은 `r` 이 쓰였음을 알고, 루프 **탈출**은 모른다(거짓은 단락일 수 있다). `a || f(r)` 이 **거짓**이면 둘 다 평가됐다 → `else` 가지가 안다. 이 4가지 전제를 iverilog 로 **실측**했다(출력 부작용을 output-formal copy-out 대신 세워서): `BODY se=1/11/21` · `EXIT se=-1` · `LEFT se=91` · `OR-else se=21` — 네 규칙이 정확히 그대로다. 그것이 리포트의 실제 사이트(`.rsp` 워커 = CAVP/Monte 벡터 순회의 표준형)를 여는 규칙이다.

**§3.2 는 우회로가 막혀 있었다는 리포트의 지적이 맞았고, 문장 위치는 가장 쉬운 경우였다.** `void'(nxt(5, r));` 와 맨몸 `nxt(5, r);` 는 같은 `Stmt::UserTaskCall` 로 파싱되는데, `lower_stmt` 의 그 arm 이 `lower_expr` 로 보내 `emit_frame_call` 의 **무조건** out-formal 거부에 걸렸다. 반환값을 버리는 자리이므로 R5-B copy-out 이 버리는 temp 하나만 있으면 된다(`inout_call_target` → `emit_frame_func_out_call`). `inout` 의 copy-IN/copy-OUT 도 같이 산다(실측 `o=11 x=6`). 그리고 **stale 문구**를 고쳤다 — 그 메시지가 지원 위치 목록에서 문장 위치를 빼먹은 것이 리포트가 없는 우회로를 찾아 헤맨 이유다.

**§3.3 은 매핑이 안 쓰여 있었을 뿐이다.** DA 리졸버 둘(`call_out_actual_writes`/`call_only_reads`)이 **첫 `NamedArg` 를 보면 그대로 포기**해서, 나머지 기본값을 건드리지 않으려고 named 를 쓴 호출이 통째로 `Unknown` 이 됐고 워크는 거기서 멈춰 **몇 인자 왼쪽의 로컬**을 지목했다. `callee_arg_dirs`(위치 인자 → 이름 인자, IEEE 1800 §13.5.4)로 한 번에 풀린다. **그걸 만들자 결함이 드러났다**: `emit_frame_func_out_call` 은 G10 named-arg 재정렬을 **아예 안 하고 있었다**(inline 경로와 plain frame 경로만 했다) — `f(.a(1), .o(x))` 가 `NamedArg` 노드를 단 채 루프에 들어가 원인을 안 가리키는 진단 2개("named argument is only valid in a user function/task call" + "output/inout arg must be a simple net")를 냈다.

**R19-X1 SILENT-WRONG — 기본 인자 값의 스코프.** vita 는 채워 넣은 default 를 **호출자** 스코프에서 낮추는데 IEEE 1800 §13.5.4 는 **서브루틴이 선언된** 스코프에서 평가한다. 자기 `g` 를 선언한 태스크 본문에서 모듈 태스크를 부르면 callee 의 default `g` 가 **호출자의 `g`** 를 집었다 — **vita `91` / iverilog `6`, exit 0, 진단 없음**. 같은 위험을 **클래스 메서드에서는 이미 닫아 뒀다**(`default_is_scope_safe`, "IEEE §13.5.3 는 메서드의 CLASS 스코프에서 푼다"라고 주석까지 달아서) — 평범한 함수/태스크 쌍둥이만 안 닫혀 있었다. 수정은 **이름을 금지하는 대신 바인딩을 비교**한다: 금지하면 generate 블록/서브루틴 본문에서 모듈 넷을 가리키는 **정상** 케이스(바깥으로 걸어 같은 넷을 찾는다)까지 죽는다. 스코프 프리픽스가 같고 subst 가 비었으면 두 낮추기는 같은 낮추기라 O(1) 로 통과하고(대부분의 호출), 다를 때만 자유 이름들의 net/param 바인딩을 `tf_decl_scope` 에서 다시 조회해 비교한다. **실측 대조**: 그림자 → loud, 모듈 프로세스/generate 블록 → 둘 다 `6`(iverilog 일치).

**R19-X2 SILENT-WRONG — frame body 안의 파일 읽기가 조용히 0.** `$fgets`/`$fscanf`/`$sscanf`/`$fread`/`$fgetc`/`$ungetc` 의 실제 일(목적지 쓰기)은 **프로세스 실행기만** 하는 문장 수준 효과(`StmtEffect::Fgets`/…)다. frame body 는 `run_frame_call` 이 같은 `SysFunc` 를 순수 `eval` 로 돌리는데 그 arm 은 **X 를 내고 아무것도 안 만진다** → `rc = $fgets(line, fd);` 가 `function automatic` 안에서 **rc=0 + 빈 문자열**. **vita `inside: rc=0 loc=[]` / iverilog `inside: rc=9 loc=[Len = 16]`, exit 0.** 그게 바로 §3.1 이 방금 열어준 `.rsp` 워커의 모양이라 같이 닫았다. **elaborate 게이트로 먼저 시도했다가 실측으로 기각**했다 — `task automatic` 은 frame 과 inline **두 벌**로 낮춰지고 호출자가 실제로 쓰는 건 inline 쪽이라, frame 사본에 건 게이트가 **정상 동작하는 설계를 false-loud** 로 만들었다(`useglobal()` 이 `rc=9` 를 내던 것이 죽었다). 그래서 `fatal_frame_heap_write`/`fatal_frame_assoc_iter` 와 같은 **런타임 fatal** 채널로 — frame 사본이 **실제로 실행될 때만** 터진다.

**적대 리뷰(2 렌즈) — 자기 수정 1건.** soundness 렌즈가 `expr_da` 의 `Call` arm 을 잡았다: 처음엔 `CallEffect::{Reads,Unknown}` 을 그대로 `ExprDa::Reads` 로 보냈는데, `call_effect` 는 **해결 못 하거나 본문을 못 훑는 모든 callee** 에 `Unknown` 을 주므로 `while (b < 3 && g(1)) … a …` — `a` 를 **언급조차 안 하는** 조건 — 이 `a` 의 읽기가 된다(동작하던 코드의 false-loud). 문장 위치는 R16 부터 엄격했고 표현식 위치는 아니었다 — 그 **비대칭은 pre-existing** 이고 이번에 건드리지 않았다(엄격화는 loud 표면이 크고 이 슬라이스에서 검증 불가). PRE/POST 로 확인(불일치 0).

**측정으로 기각한 조임 1건**: `call_out_actual_writes` 는 callee **본문**이 플래튼 넷에 닿는지는 안 본다(BL4 이래의 극성). 조여 보려다 기각했다 — callee 의 unpacked-struct **formal 도 같은 `$unp$r$len` 로 fan-out** 되므로 본문 워커가 항상 "닿는다"고 답해 §3.1 을 통째로 false-loud 로 만든다.

**잔여**: ①package 함수의 default 는 여전히 caller 스코프에서 낮춰진다(`tf_decl_scope` 가 import 한 모듈 프리픽스로 기록됨) — 가드는 개선이지 완결이 아니다. ②R19-X2 의 correct-support = `SimState::files`/`read_state` 를 interior-mutable 로 만들어 `&self` frame 실행기가 몰 수 있게(=`dyn_heap` 이 frame-local `new[]` 에 해준 것, §4.5.194) → ROADMAP §3.

**검증**: 하베스트 코퍼스 **256 designs PRE/POST**(불일치 5 = **전부 이번 신규 테스트**, 회귀 0) · iverilog 차분 2종(**named arg + default + output formal on tasks** = 12/11/8/9 · 7/6/3/4 완전 일치 · **단락 평가 전제 4가지**) · **4909 tests green**(신규 `r19_value_call_out.rs` **19** — soundness 핀 7: `&&` 우변은 탈출 경로를 안 쓴다 / 같은 식의 다른 읽기 / `?:` arm / `inout` actual / 생략된 default 가 로컬을 읽음 / caller-그림자 default / frame body 안의 `$fgets`, + **회귀 핀** 인라인된 태스크는 여전히 파일을 읽는다) · clippy 0 · fmt clean · **format_version 26 불변**. 모듈 사이즈 정책: `frames_call.rs` 1152 → `frames_call/{mod,args,emit}.rs` 125/197/851.

#### 4.5.273 외부 round-18 — suspend 하는 callee, struct 멤버, 그리고 그 밑의 silent-wrong (2026-07-29, format 26 불변) ✅

**리포트 12건 중 11건이 §3.1 하나였다.** `stmt_no_ref_deep` — callee 본문을 보는 **깊은** 참조 워커 — 에 `DelayCtrl`/`EventCtrl`/`Wait`/`WaitFork` arm 이 **아예 없었고**, 대입문 arm 은 `delay.is_none() && event.is_none()` 을 요구했다. 참조 워커에서 `_ => false` 는 "모르겠다"가 아니라 **"이 노드는 무엇이든 참조할 수 있다"** 이고, 그 답은 **질문한 이름과 무관하다**. 그래서 `@(posedge clk)` 한 줄이 든 callee 를 **한 번이라도 부르면** caller 의 뒤쪽 블록 로컬이 전부 못 쓰게 됐다 — `preload()`/`run_scenario()` 같은 표준 클럭 드라이버 태스크가 정확히 그 모양이다. **R17 이 얕은 `stmt_no_ref` 에서 고친 것과 똑같은 실수가 깊은 쌍둥이에 남아 있었다.**

**그 게이트를 열기 전에, 밑에 깔린 silent-wrong 을 먼저 닫아야 했다(R18-X1).** 공유 플래튼 넷 + **suspend 하는 호출**: 블록 A 가 로컬을 쓰고, suspend 하는 헬퍼를 부르고, 다시 읽는 사이에 같은-이름 형제 블록 B 가 그 한 넷을 덮어쓴다. `c8ad2b4` 와 `46b9816` 에서 **실측 동일** — vita `A v=99` / iverilog `A v=1`, **exit 0** = pre-existing. 두 겹으로 새고 있었다: ①R17 의 공유-넷 규칙이 **문법적 타이밍만** 봤다(한 줄짜리 `task tick(); @(posedge clk); endtask` 가 suspend 를 가린다) ②최상위 워크가 **대입되면 早期 return** 해서 그 규칙에 **닿지도 않았다**(R17 의 주석은 "대입 뒤의 타이밍 문장은 전에도 허용했고 지금도 허용한다"고 적었는데, 그건 *참조* 질문의 논리였고 suspend 는 다른 질문이다 — inert 한 callee 도 스케줄러를 넘긴다). 수정 = `CallSuspends` 리졸버(`da` 는 callee 본문을 못 보므로 `OutActualWrites` 와 같은 클로저 패턴, **모든 미해결은 `true`** — REJECT 쪽만 읽으므로) + 공유 넷일 때는 早期 return 대신 계속 걷기.

**§3.2 는 멤버 세기가 아니라 비트 커버리지로 풀린다.** struct 는 **파서 desugar** 라 DA 워크 시점엔 멤버가 없고 비트만 있다 — `rm.c = 5` 는 단일 멤버 struct의 **전 비트**를 쓰는 상수 part-select다. 그래서 `const_bit_span_write` + 커버리지 집합(`elem_bounds` 배열 커버리지의 한 단계 아래)으로 하니 필드별 쓰기(`rm.a=…; rm.b=…;`)와 손으로 쓴 `x[31:16]=a; x[15:0]=b;` 가 **공짜로 따라왔다**. 폭은 `decl_bit_width`(unpacked 없음 + 4096비트 상한)로 caller 가 계산.

**§3.3 의 뿌리는 DA 가 아니라 파서였다.** `automatic rec_t r;` 에서 **unpacked struct 타입명은 `typedefs` 에 없어** `parse_automatic_block_decl` 이 이름을 못 풀고 `None` 을 냈고(`automatic` 키워드는 이미 소비된 뒤), 그 다음 순번의 멤버 fan-out(`try_block_unpacked_struct_decl`)이 **lifetime 을 안 찍은 채** 선언을 파싱했다 → **`automatic` 이 조용히 static 으로 강등**. 그래서 같은-이름 struct 로컬 2개가 한 플래튼 넷을 공유했고, **같은 모양의 `automatic int`/enum/typedef-alias 는 전부 `$blk$` 스코프를 받았다**(실측 대조 3종으로 격리 — 구조체만 `auto_per_name={}`). 한 줄 수정으로 세 형태가 같아졌다.

**부수로 순서 의존 하나를 없앴다.** coalesce 게이트는 **"넷이 이미 존재할 때"** 켜지므로 **두 번째 이후 선언 블록만** 검사한다 — 첫 블록은 자기 넷이 사설(private)인 줄 알고 통과했다. 그건 의미가 아니라 순서다(두 블록이 같은 넷을 쓴다). `compute_coalesced_block_locals`(순수 AST 사전 계산, `compute_scoped_block_locals`/`compute_per_entry_block_locals` 와 같은 패턴)를 신설해 모든 선언 블록이 같은 답을 받게 했다. **`$blk$` 스코프를 받은 블록은 자기 넷을 가지므로 세지 않는다** — 처음엔 그걸 빼먹어 이미 동작하던 2단계 struct 쌍을 false-loud 로 만들었다(`block_scope_two_level` 핀이 잡았다).

**SoA 호출 인자.** unpacked struct 는 파서가 `$unp$<var>$<member>` 로 fan-out 하지만 **호출 인자는 레코드 이름 그대로** 남는다 — 그래서 `rsp_next(fd, rm)` 이 `$unp$rm$count` 를 건드리는 것이 워크에 **안 보였다**(formal 을 통한 쓰기는 false-loud, 그리고 `inout` 의 copy-**IN** 읽기는 잠재적 unsound). `actual_is_record_of` 로 두 철자를 같은 저장소로 인식(var 부분은 **정확 일치**).

**잔여(정직한 loud, 1건)**: 리포트 §3.3 의 정확한 모양 — `while (a && f(fd, rm))` 처럼 **쓰기가 `&&` 우변에 있는** 경우. `cond_out_writes` 는 단락 평가 때문에 Binary 를 의도적으로 풀지 않으므로 그 호출이 반드시 실행된다고 증명할 수 없다(`cnt_done=0 < 2` 가 첫 평가에서 참임을 알려면 상수 전파가 필요하다). 공유 넷 문제는 사라졌고 남은 것은 per-entry 질문뿐이며, 진단은 포기 지점을 정확히 가리킨다.

**검증**: 4890 tests green(신규 `r18_suspend_and_members.rs` 10) · clippy 0 · fmt clean · format_version **26 불변**. 신규 핀에는 soundness 3건(공유 넷 + suspend 호출 / 인라인 suspend / 부분 멤버 커버리지)이 포함된다.

#### 4.5.272 `-v` 유효 invocation echo — 그리고 그것이 드러낸 filelist 플래그-값 결함 (2026-07-29, format 26 불변) ✅

**질문이 먼저였다.** "Makefile/셸 스크립트로 vita 를 돌리면 환경변수가 흩어져 최종 호출 인자의
*변수명이 아닌 최종 형태*를 확인하기 어렵다 — 터미널에 해석돼서 출력되나(그래서 log 로 확인
가능한가)?" 실측 답은 **아니다**. `-v` 는 `defines:`/`incdirs:` **두 줄만** 찍었고, 나머지는
전부 전사(transcript)에 없었다 — 어떤 소스가 실제로 컴파일됐는지(`--dump-filelist` 는 **exit
하므로** 실행 로그와 공존 불가) · 런타임 plusarg(`+SEED=$(SEED)` — Makefile 에서 가장 자주
틀리는 것) · 출력 경로 · **`VITA_THREADS`(argv 에 아예 없다)** · 어떤 `.f` 들이 전개됐는지 ·
cwd · 원본 명령줄.

**근본 인식**: 사람이 *읽는* 인자와 프로세스가 *받는* 인자는 다른 텍스트이고, **후자만이 실행을
결정했다**. 그리고 후자는 로깅 시점엔 이미 복원 불가다 — 셸이 `$(WIDTH)` 를 치환했고, filelist
전개기가 `-f` 프레임을 없앴고, env 는 argv 에 흔적이 없다. **그 사본을 남기는 곳이 없었다.**

**구현**(`cli/src/echo.rs` 신규 · `Invocation` 레코드): 전개 **前** argv + cwd 를 `run()` 에서
포착해 `VitaOpts` 로 흘리고, `filelist::expand_argv` 가 **실제로 연 `.f` 목록**을 반환하게 했다
(`Expander.opened`, 깊이우선 순서). echo 는 `Progress` 이벤트라 `--log` tee 가 **같은 writer**로
같은 순서에 담는다(doc-13 단일 writer) — 로그 파일이 완전한 실행 기록이 된다. 행: `invocation`
(셸 인용·붙여넣기 가능)·`cwd`·`filelists`·`sources`·`incdirs`·`defines`·`plusargs`·`tops`·
`output`·`obs-dir`·`probes`·`timeout`·`threads`(**출처 표기** `--threads`/`VITA_THREADS`/`auto`)·
`log`·`env`. **빈 행은 생략**(평범한 `vita tb.sv -v` 는 4줄) · 값 컬럼 정렬 + 92열 줄바꿈 ·
위아래 빈 줄 · 단일 값이 마진보다 길면 **쪼개지 않는다**(끊긴 경로가 긴 줄보다 나쁘다). 전
applet 이 자기 단계를 찍는다(vcmp=define 표면, velab=root/library, vrun=plusargs). 순수 보고 —
bucket C, 해시 무진입.

**echo 를 만들자 결함이 하나 드러났다 — `takes_value` 가 원래 5개뿐이었다.** filelist 전개기는
값 받는 플래그의 다음 토큰을 건너뛰어야 하는데, 그 목록이 `-o/--threads/--timeout/-D/-I/-l/
--verbosity` 시절 그대로였다. 그 뒤 추가된 **모든** 플래그(`--top`·`-L`·`--work`·`--workdir`·
`--upstream`·`--obs-dir`·`--hier-tree`·`--inst-paths`·`--probe`·`--probe-file`)의 **값이 소스
positional 로 취급**돼 `-F` 프레임 안에서 프레임 디렉터리 기준으로 재작성됐다. PRE/POST 실측:
`ip/build.f` 의 `--top top` → PRE `error: top module '/abs/ip/top' not found`(**false-loud**) /
POST 정상 실행 · `--hier-tree h.txt` → PRE 가 **exit 0 으로 조용히** `ip/h.txt` 에 씀(호출자가
지정한 위치가 아니다) / POST `h.txt`. 후자가 **silent** 였다. 수정 = 목록 완성(정본은
`cli/src/filelist.rs::takes_value`, 주석에 재발 방지 근거) — `-f`/`-F` 는 **제외**(전개기가 직접
소비하고 그 값은 진짜 경로라 해소돼야 한다).

**부수 규칙 하나**: argv 행의 줄바꿈은 **플래그와 값을 절대 가르지 않는다**(`argv_atoms`) — `-D`
가 줄 끝에 남고 `W=32` 가 다음 줄 머리에 오면 맨몸 플래그 + 떠도는 소스 파일로 읽혀서, echo 의
목적과 정반대가 된다.

**검증**: 신규 `cli/tests/invocation_echo.rs` 7 테스트(Makefile 형태 전 경로 해소 · env-only knob
의 출처 표기 · `--log` tee 포착 + 순서 · `-v` 없으면 부재 · staged 3단계 각자 · 그리고 filelist
결함 2건은 **PRE 바이너리로 재현 후** 회귀 핀). `echo.rs` 인라인 6 유닛 테스트(줄바꿈·과대
단일값·빈 행·셸 인용·플래그 접착·꼬리 플래그). 4880 tests green · clippy 0 · fmt clean ·
format_version **26 불변**(bucket C 라 산출물 무영향).

**남은 것(별개 슬라이스)**: 상용 3단계 knob 중 **elaborate 단계 파라미터 override**(`-G`/
`-pvalue+`/`-P<path>=`)만 여전히 미배선 — doc-14 §RULE B 에 **스펙은 이미 있고** 코드가 없다.
ROADMAP §0 T2-14 에 근거·범위·오라클과 함께 등록했다.

#### 4.5.271 오라클을 만들다 나온 silent-wrong 2건 + 진단 품질 (2026-07-29, format 26 불변) ✅

**§3.1 의 오라클을 만들려다 `atoi` 계열이 통째로 틀린 걸 발견했다.** iverilog 는 체인(`s.substr(a,b).atoi()`, IEEE §8.13)을 **파싱조차 못 하므로** 분해 오라클(`t = s.substr(a,b); t.atoi()`)을 만들었는데, 그 분해본에서 vita 3 / iverilog 0 이 갈렸다. `parse_radix_prefix` 는 **`strtol` 파서**였다 — 공백을 건너뛰고, 부호를 먹고, `_` 에서 멈춘다. IEEE 1800 §6.16.9 는 정확히 반대다: *"scans all leading **digits and underscore characters** (`_`) and stops as soon as it encounters any other character"*. 공백도 부호도 스캔을 **즉시** 끝내고, `_` 는 **스캔되며 값에 기여하지 않는다**.

두 렌즈가 같은 편이었다 — LRM 과 iverilog 13 이 8개 입력 전부에서 일치하고 vita 만 6개에서 달랐다: `" 3"`→3(0이어야) · `"-7"`→-7(0) · `"+7"`→7(0) · `"1_0"`→1(**10**) · `" ff".atohex()`→255(0) · `"f_f".atohex()`→15(**255**). 그런데 코드에는 **"iverilog 13 drops it, its bug"** 라는 주석과 `atoi_negative_is_ieee_signed` 라는 핀이 붙어 있었다 — LRM 인용을 단 `strtol` 이었다. 이건 **리포터의 코드 한복판**이다: `.rsp` 헤더 리더가 `line.substr(a,b).atoi()` 로 필드를 읽는데, 부분문자열이 `"[L = 32]"` 의 공백을 포함하면 값이 조용히 달라진다. §3.1 이 그 경로를 여는 슬라이스라 같이 닫지 않으면 **loud 를 silent-wrong 으로 바꾸는 셈**이었다. 17개 입력 × 2 메서드 = **34 측정이 iverilog 와 바이트 동일**(`atooct`/`atobin` 은 iverilog 미지원 → hand-IEEE).

**R17-X1 — 참조 워커가 메서드 리시버를 못 봤다.** `expr_reads_ident` 의 `K::Call` arm 이 **인자만** 보고 콜 경로의 **머리**를 안 봤다. 2 세그먼트 이상이면 머리가 리시버이므로 `s.atoi()`·`q.size()`·`a.push_back(x)` 는 전부 `s`/`q`/`a` 의 읽기인데 하나도 안 잡혔다. 이 워커 위에 **블록 로컬 scope-leak 검출기**가 서 있다 — 블록 밖에서 메서드 호출로만 참조되는 블록 로컬은 검출되지 않았고, 로컬이 바깥 바인딩의 넷으로 coalesce 되어 **블록 밖 읽기가 블록 안 값을 돌려줬다**. 측정: vita `B 1234` / iverilog `B 9999`. 체인(`s.substr(0,1).atoi()`)은 리시버가 경로가 아니라 표현식이라 `MethodCall` arm 이 따로 필요했다 — 같은 구멍의 한 겹 아래.

**진단 품질(§4.1)** — scope-leak 거부는 **위치가 아예 없었다**. 선언 span 을 이름마다 실어 에러를 선언에 앉히고, 워커가 `Option<Span>` 을 돌려주게 해 **바깥 참조 자리**에 note 를 붙였다. 둘 중 하나만으론 부족하다 — 둘은 보통 다른 문장에 있다. deferred-hier 3개 패스(read·write·sel-write)도 레코드에 span 을 실어, 그 패스가 내는 **모든** 진단(§23.9 신규 포함, 기존 "undeclared hierarchical name" 도)이 위치를 갖는다.

**모듈 사이즈 정책** — `block_local.rs` 가 1248 줄이 되어 `block_local/{mod,gate,hoist}.rs` 로 분할(430/687/155). 코드가 이미 갈라져 있던 선을 따랐다 — **평탄화해도 되는가**(gate) 와 **평탄화한다**(hoist).

#### 4.5.270 안 쓴 로컬은 per-entry 저장과 바이트 동일 — 그리고 그걸 열자 §23.9 구멍이 드러났다 (2026-07-29, format 26 불변) ✅

**§3.2(1)** — 일부러 안 쓴 `automatic byte exp[];` 를 `input` formal 로 넘기는 것. 리포터의 말이 맞다: **"쓰기 전 읽기"가 아니라 "쓸 필요가 없는" 경우**다. 근거는 경로 추론이 아니라 **한 줄**이다 — 어디서도 쓰이지 않는 로컬은 평탄화된 static 넷이 타입 기본값으로 한 번 초기화된 뒤 **아무도 바꾸지 않으므로**, 매 진입마다 기본값이다. `automatic` 이 주는 것과 정확히 같다. 둘 다 안 변하기 때문에 같다.

문제는 "쓰이지 않는다"를 **누가** 판정하느냐였다. 시그니처를 안 보는 기존 writer 검출기는 user-call actual 에 이름이 있으면 전부 "쓸 수 있음"이라고 답한다 — 그게 맞는 기본값이지만, 그러면 §3.2 의 형태에 대해 never-written 논거를 **아예 못 만든다**. 그래서 `CallEffect` 에 네 번째 판정 `Reads` 를 넣었다: callee 가 resolve 되고, 이름을 언급하는 actual 이 **전부 `input` formal** 이며, 본문이 평탄화된 넷에 닿을 수 없음이 증명된 호출. writer 검출기에는 그 리졸버를 **opt-in 파라미터**로 넣었다(리터럴 `None` 이면 기존 그대로). 단 리졸버가 **있을 때는** `Unknown` 의 극성이 뒤집힌다 — never-written 을 주장하려는 참이므로, 본문을 볼 수 있는 유일한 것이 못 증명하면 정직한 답은 "쓸 수 있음"이다(round-16 이 실측한 `task poke; t.a = 99;` 가 정확히 이 경우다).

**주장의 범위를 `sole_writer` 로 명시**했다. never-written 은 **이 블록에 대해서만** 하는 말이다. 같은-이름 coalesce 게이트에서는 **다른 블록의 쓰기가 곧 읽히는 leftover** 이므로 성립하지 않는다 — 그 호출 자리는 `false` 를, 새 넷 게이트는 `true` 를 넘긴다.

**그리고 게이트를 열자 밑에 있던 게 드러났다.** 다른 모듈이 `tb.a = 99` 로 평탄화된 블록 로컬을 쓰면 vita 는 `a=99` 를 찍는다. IEEE 1800 §23.9 는 **automatic 변수로의 계층 참조를 금지**한다(정적 주소가 없다) — iverilog 는 같은 프로그램을 거부한다("Hierarchical reference to automatically allocated item"). 측정해 보니 **pre-existing** 이었다: 확실히 대입된 쌍둥이(PRE 도 받아들이던 형태)에서 PRE·POST 둘 다 `a=99` 였다. 그래도 내 규칙이 **그 도달 범위를 넓혔으므로** 내 것이다. `automatic_local_nets` 를 두고(선례 = `clocking_hold_nets`) 계층 read·write·sel-write 세 funnel 에서 거부한다. 같은 모듈 안 `t.a = 99` 도 같은 이유로 같이 loud 가 된다.

**§3.3 의 타이밍 arm 이 잃어버린 loud 도 여기서 복원**했다. `#1`/`@()`/`wait` 를 모델링하자 **generate 스코프 넷과 이름이 겹치는 static 블록 로컬**의 회귀 핀이 깨졌다 — 그 핀은 `#1` 이 catch-all 로 떨어지는 **우연**에 기대고 있었고, 그 우연이 실은 옳았다. 정식 규칙으로 바꿨다: **넷이 공유될 때만**, 시간을 진행시키는 문장은 거부한다(스케줄러가 다른 블록으로 넘어가 그 하나뿐인 넷을 쓴다). 새 넷이면 다른 writer 가 없으니 무관하다. 검사는 **재귀적이고 `stmt_no_ref` 빠른 경로보다 먼저** 온다 — 로컬을 언급조차 않는 `begin #1 y = 3; end` 도 똑같이 스케줄러를 넘긴다.

#### 4.5.269 외부 round-17 §3.1/§3.1b/§3.3 — arm 하나가 없었고, catch-all 하나가 이미 쓴 걸 잊고 있었다 (2026-07-29, format 26 불변) ✅

외부 round-17 리포트(2026-07-29, base `c8ad2b4`)는 `TB=top` 의 **진단 34 건**을 3 가족으로 나눴다: 체인 메서드 12 · 안 쓴 dyn input 1 · **축소 실패 21**. `TB=sha2`/`sha3`/`partial` 은 무수정 green 이고 **값이 틀린 사례는 하나도 없었다**.

**§3.1/§3.1b(12) — `expr_no_ref` 에 `MethodCall` arm 이 없었다.** `v = line.substr(4,5).atoi();` 의 체인은 리시버가 **콜 결과**라 `Call` arm 의 경로 검사가 닿지 않고 `_ => false` 로 떨어진다. 그 답은 **모든 이름에 대해** "참조할 수 있음"이므로, 블록 어딘가에 체인이 하나 있으면 그 체인의 **대입 대상 자신**과 그 뒤에 선언된 모든 로컬이 거부된다. 리포터가 "체인을 두 문장으로 쪼개면 통과"를 측정한 것이 정확히 이 구조다. §3.1b(체인이 callee 본문에 있으면 **다른 파일의** caller 지역변수를 지목)는 callee-inertness 워크가 **같은 워커**를 쓰기 때문이고, arm 하나로 둘 다 사라진다. 패키지의 체인 23곳을 분해해 34→22 로 줄였다는 리포터의 정량 근거와 일치한다.

**§3.3(21) — "축소 실패"의 근인은 in-situ 관찰 그대로였다.** 리포터는 세 가지를 측정했다: ①문제 지역변수에 더미 대입을 **바깥 `while` 앞**에 넣으면 진단이 사라진다 ②같은 대입을 **그 `while` 본문 안**에 넣으면 안 사라진다 ③그런데 격리하면 루프 본문의 쓰기는 정상 반영된다. 셋을 모두 만족하는 원인은 하나다 — `da_stmt` 의 `_ => None` catch-all 이 **이미 확실히 대입된 상태를 무시**하고 있었다. 최상위 스캔은 문장 사이에서 `if assigned { return true }` 를 하지만 **중첩 구조 안에서는 아무도 안 했다**. 그래서 루프 밖 대입은 최상위 상태를 켜서 통했고, 루프 안 대입은 그 뒤 문장 하나가 모델링 안 된 형태이기만 하면 그대로 거부됐다. 한 번 확실히 쓰였으면 관찰하려던 per-entry 리셋은 **이미 덮여 있고**, 그걸 되돌리는 연산은 없다.

같은 자리에서 **타이밍이 붙은 첫 쓰기** 5형태를 모델링했다 — `#1 x = 7;` · `@(posedge clk) x = 7;` · `x = #1 7;` · `#1 begin x = 7; end` · `wait (c) x = 7;`. 전부 blocking 쓰기라 프로세스는 쓰기가 끝날 때까지 다음으로 못 간다. `stmt_no_ref` 도 같이 고쳤다 — 타이밍 접두는 **표현식이 더 있을 뿐**이라 `#1 y = 3;` 은 `y = 3;` 만큼도 `name` 을 참조하지 않는데, `delay`/`event` 가 있다는 이유만으로 `false` 를 답해 무관한 문장이 워크 전체를 끝내고 있었다.

**리포트의 요청(§3.3 ★ / §4.2)은 note 로 이행했다.** E3009 는 원인을 둘("초기화자" 또는 "첫 쓰기 전 읽기")만 말하는데, 실제로는 세 번째("분석기가 여기서 포기했다")가 대부분의 실제 이유였고 인쇄되는 위치는 **선언뿐**이었다. `da_stmt` 가 `Result<DaOut, DaGiveUp>` 을 돌려 포기 지점의 span 과 이유를 실어내고, 게이트가 그 자리에 `note:` 를 붙인다. 이유는 6가지로 구분된다 — 여기서 읽힘 · 부분(select) 쓰기 · 모델링 안 된 문장 형태 · 증명 못한 호출 · `input` actual 읽기 · 공유 넷에서 시간 진행.

**검증**: harvested 4266 설계 PRE/POST 스윕 = **불일치 12, 11건은 note 추가뿐 · 1건은 §23.9 신규 loud(iverilog 도 거부하는 프로그램), 회귀 0**. round-17 조합 코퍼스 134 케이스 3-way(iverilog/PRE/POST) = **39건 loud→iverilog 와 일치, 회귀 0, 불일치 3 = 전부 같은 iverilog 크래시**(automatic queue 의 `.size()` 에서 `vthread.cc:938` assertion — 모듈 스코프 static 이면 정상, vita 는 IEEE §7.10.2 대로 0). 체인 72 케이스는 iverilog 가 파싱을 못 하므로 **분해 오라클 3연쇄**로 검증 — chained(vita) == decomposed(vita) == decomposed(iverilog), **72/72**.

**적대 리뷰가 자기 수정에서 하강 3건**을 잡았다: ①타이밍 arm 이 `stmt_no_ref` 의 불변식("여기 오면 한쪽이 name 을 참조한다")을 깨서 무관한 `#1 y = 3;` 에 *"이 대입은 일부만 쓴다"* 는 엉뚱한 메시지를 달 뻔했다 → `stmt_no_ref` 쪽에서 해결 ②`NonBlocking` 을 guard 로 처리해 읽기 사유가 "모델링 안 된 형태"로 잘못 보고됐다 ③공유 넷의 타이밍 loud 상실(§4.5.270 에 기록).

#### 4.5.268 외부 round-16 §3.4~§3.7 + §4 — 두 단계가 스코프를 다르게 중첩하고 있었다 (2026-07-29, format 26 불변) ✅

**§3.4(17+1)** — 형제 블록 트리가 **두 중첩 레벨에서** 같은 이름을 재사용하면 실패했다. 한 레벨만이면 통과. 근인은 **두 단계의 불일치**다: Logic 단계는 스코프 있는 블록의 본문을 `with_scope("$blk$<lo>")` 안에서 낮추므로 두 레벨이 다 스코프면 세그먼트가 **중첩**되는데, Nets 단계 hoist 는 **평평하게** 재귀했다. 안쪽 블록의 넷은 `$blk$<inner>` 에 생기고 본문은 `$blk$<outer>.$blk$<inner>` 아래에서 해석돼 못 찾고 모듈 넷으로 떨어졌다. 분류기는 그걸 피하려고 **다른 후보 안에 중첩된 후보를 전부 버렸고**, 그래서 한 레벨은 되고 두 레벨은 안 됐다. hoist 가 lowering 과 같은 방식으로 중첩하자 그 drop 자체가 필요 없어졌다.

그 직후 **미방출 가드가**(테스트가 아니라) 두 번째 것을 잡았다 — 중첩 키(`t.$blk$A.$blk$B`) 아래 기록된 블록 로컬 초기화자를 **아무 flush 도 claim 하지 않았다**. claim 규칙이 **직계** `$blk$` 자식만 받았기 때문이고, 블록이 스코프를 중첩할 수 있게 된 이상 "이 스코프 안"은 `$blk$` 세그먼트가 **몇 겹이든**을 뜻한다. 리포트가 지목한 **거짓 주장 2건**도 여기서 사라진다 — "this one is static"(SoA 멤버) 과 "this one is `automatic`, so the OTHER is not" 을 내던 형태가 그냥 **동작한다**. 이 모양이 중요한 이유는 그게 표준 table-driven `.rsp` 워커이기 때문이다.

**§3.5(9)** — 콤마 선언은 **독립 declarator N개**인데 충돌 검사가 `d.names.first()` 하나를 읽고 그 판정을 전부에 적용했다. 모듈 넷 `n` 옆의 `automatic int n = 0, n_skip = 0;` 이 `n_skip` 까지 거부하면서 **설계 어디에도 없는** 같은 이름 넷과 충돌한다고 말했고, 선언이 통째로 버려져 이후 모든 사용이 **자기 선언 한 줄 아래에서** E3010 "undeclared" 였다. 8건 → 1건, declarator 3개면 11건 → 1건, 순서 의존도 사라졌다. 분할은 declarator 들이 **의견이 갈릴 때만** 발동하므로 균일한 선언은 원래 경로 그대로다.

**§3.6(2)** — named-parameter 폭 멤버를 가진 struct 는 파서가 멤버 오프셋을 계산할 수 없어 **필드별**(`$unp$q$f`)로 저장된다. record-queue fan-out 에 `pop_*` arm 이 없어서 **결과를 버리는** pop 이 일반 2-세그먼트 enable 로 떨어졌고 `q` 라는 넷이 없으니 "unsupported hierarchical task call `q.pop_front`" 로 표면화됐다 — 인스턴스 경로 이야기를 하는데 대상은 둘 다 아니다. `void'()` 도 fan-out 을 거치지 않고 자기 문장을 직접 만들고 있었다. 필드 큐가 **보조를 맞추는지**는 머리를 두 번(한 번 버리고 한 번 받아서) 뽑아 확인했다.

**§3.7(1)** — dyn-array formal 의 caller 배열은 표현식 직전에 방출되는 마커가 callee formal 슬롯에 **스냅샷**한다. 직접 blocking-assign rhs 만 마커를 냈으므로 `return f(arr);` 이 loud 였다. `return` 은 **바로 그 대입**이라 같은 경로를 탄다. concat 에 **묻힌** 호출도 마찬가지 — 프레임 밖이면 temp 로 hoist, 프레임 안이면(temp 를 둘 곳이 없다) 마커를 제자리에서 방출. 스냅샷이 틀릴 자리는 거부한다: 자기 본문 안의 **재귀** 호출(기존 핀이 잡았다 — 모든 레벨이 같은 배열을 넘길 때만 우연히 맞았다)과, 한 프레임 본문 표현식 안 같은 함수 **2회** 호출.

**§4** — 계층 task call 거부는 리포트 84건 중 **유일하게** `file:line:col` 이 없었다(그 TB 에선 그게 유일한 진단이라 로그 전체에 위치가 없었다). resolve 패스에서 뜨므로 enable 의 span 을 deferred 레코드에 실었다. dyn-formal 메시지는 r17 이 이미 없앤 "module-process level" 제약을 더는 주장하지 않고, 같은-이름 메시지는 상대 선언의 lifetime 을 **추론하지 않는다**.

**§3.8(리포트의 "미분류 1건")도 같은 뿌리다** — 구조체 로컬이 멤버별 넷으로 분해되며 `automatic` 플래그를 잃어 STATIC coalesce 분기의 다른 문구를 달았을 뿐이고, 두 레벨에서 스코핑을 잃은 것이 정확히 그 멤버들이었다. 같은 문구의 이웃 형태(초기화자를 가진 static 블록 로컬 둘이 한 이름)는 **loud 로 남는다** — 평탄화된 넷 하나에 pre-arm 초기화가 둘 걸려 뒤가 앞을 덮으므로 받아들이면 앞 블록이 뒤 블록 값을 읽는다(iverilog 7/9 → 9/9).

**검증**: harvested 4266 설계 PRE/POST 스윕 = **불일치 20, 전부 loud→동작, 회귀 0**. 조합 코퍼스 100 쌍 differential(불일치 20 = 전부 iverilog 결함) · staged==one-shot 14/14 · VCD PRE==POST.

**적대 리뷰**: escaped identifier 는 `$` 를 담을 수 있어 사용자가 `\$break$77` 이라는 블록을 만들 수 있다 — 파서의 `break` 합성 라벨과 **같은 철자**다. 충돌했다면 그 `disable` 이 loop jump 로 읽혀 join 에서 빠지고 미기록 읽기가 조용히 통과했을 것. 판별자가 둘을 구분함을 핀으로 고정했다.

#### 4.5.267 고정 크기 `automatic` unpacked 배열 — 리셋은 측정으로 기각됐다 (2026-07-29, format 26 불변) ✅

두 형태가 이 타입을 사실상 못 쓰게 만들고 있었다. **(a)** `'{…}` 선언 초기화자가 loud 인데 **같은 내용·같은 원소 타입**을 dyn `[]` 로 쓰면 통과했다 — 빠진 건 per-entry 분류기 arm 하나뿐이었다. 고정 배열의 재초기화는 전체 대입 `a = '{…}` 이고 그건 `automatic` 아래에서도 이미 올바로 낮아졌으므로(실측) 방출 경로는 그대로 재사용했다. **(b)** 초기화자 없이 원소별로 채우는 형태. DA 워크가 **전체 배열 쓰기만** 첫 쓰기로 셌기 때문에, 원소 4개를 다 쓰고 읽어도 read-before-write 로 거부됐다.

**(b)의 뻔한 수정은 구현했다가 측정으로 기각하고 되돌렸다.** per-entry 로 표시해 블록 진입마다 타입 기본값으로 리셋하는 방식인데, automatic 저장은 **블록 진입이 아니라 ACTIVATION** 마다 만들어진다. iverilog 로 재보면 automatic task 안의 블록을 루프 3회로 진입할 때 `xx, 10, 11` — **잔값이 살아남는다** — 반면 **호출 3회**는 `xx, xx, xx` 다. 초기화자는 진입마다 다시 돈다(`w=11` 매회). 그래서 (a)는 per-entry 이고 (b)는 아니다.

(b)는 대신 **커버리지를 증명**해서 닫았다: 리터럴 인덱스 · 블록 최상위(무조건) · 배열을 읽을 수 없는 rhs. 계산된 인덱스 · 조건부 쓰기 · 자기참조 rhs · 불완전한 집합은 전부 loud 로 남고, 리셋이 답을 바꿨을 **엔트리 간 잔값** 케이스도 loud 다. 핀 하나가 전제와 함께 뒤집혔다 — `automatic_block_local_init_stays_loud` 는 "`$blk$` 스코프 경로가 decl-init 수집기를 안 돌리므로 받아들이면 배열이 빈다"에 기대고 있었는데 §4.5.255 가 수집기를 스코프 안으로 옮겼다. 이제 `[aa][bb]` 를 찍고 그건 iverilog 가 찍는 값이다.

#### 4.5.266 definite-assignment 이 제어 흐름과 callee 본문을 본다 (2026-07-29, format 26 불변) ✅

블록 로컬 DA 워크의 false-loud 두 모드, 리포트 84건 중 **53건**.

**§3.1(49)** — 워크가 assigned **bool 하나만** 들고 다녔고, 그건 "다음 문장으로 흘러간다" 와 "뛰어서 나간다" 를 구분하지 못한다. 그래서 첫 쓰기 **앞의** `break`/`continue` 가 나중 읽기에 미기록으로 도착하는 살아있는 경로로 읽혔다 — 실제로 그 읽기에 도달하는 **모든** 경로는 이미 쓰기를 실행했다. 두 상태 `DaOut`(Falls/Jumps)을 들고 join 을 표준 방식으로 병합하면 끝이고, 단순 passthrough 로는 여전히 거부됐을 arm 형태(`if (c) continue; else x = …;`)까지 덮는다.

진짜 `disable` 은 **일부러** jump 가 아니다: 조상이 아닌 블록을 지목하면 이 문장은 계속 흐르므로, 그 경로를 join 에서 빼면 **진짜** read-before-write 를 조용히 받아들이게 된다. 파서의 `break`/`continue` 는 합성 라벨 `$break$`/`$continue$` 로 구분한다.

**§3.2(4)** — 문장 위치 user call 은 **일부러** 미검증이었다(r19 리뷰 F5): callee 본문이 call 머리도 인자도 언급하지 않은 채 flatten 된 넷 이름을 부를 수 있다. 그 위험은 **실재한다** — `$display(a)` 로도, 계층 `t.a = 99` 로도 재현된다. 그래서 가정하는 대신 **callee 가 그 이름을 건드릴 수 없음을 증명**한다. call 리졸버가 세 판정을 돌려주고(Writes/Inert/Unknown), Inert 는 참조 없는 실인자 + 해석 가능한 callee + 깊이 예산 안에서 중첩 호출까지 재귀하는 보수적 deep walk 를 요구한다.

deep walk 은 **모든 세그먼트** 경로 규칙이 필요하다(caller 자기 문장에는 맞는 head-segment 규칙이 `t.a` 를 참조 없음이라 부른다). 사본을 만드는 대신 기존 워커에 **opt-in 파라미터**로 넣었다 — `false` 를 넘기면 오늘 동작으로 정확히 단락된다.

**부수(R16-X1·pre-existing)**: 프레임 로컬 `string f[3] = '{…}` 이 자기 합성 `f = new[3]` 때문에 user-resize 가드에 걸렸다. 프레임 경로가 pre-size 만 선언 초기화자로 표시하고 뒤따르는 초기화자는 안 했다. **user 가 쓴** `new[…]` 는 측정으로 제외했다 — 그것까지 면제하니 `string f[3] = new[5];` 가 6b6b8ef 의 loud 거부에서 exit 0 의 빈 원소로 바뀌었다.

#### 4.5.265 초기화자 소유권을 bool 이 아니라 랭크 경로로 (2026-07-28, format 26 불변) ✅

마무리 리뷰가 마지막 한 건을 찾아 **이 시리즈의 5f76d55** 로 이등분했다. generate 스코프 안에서 그 스코프 **자신의 블록 로컬**이 **중첩 generate 스코프보다 뒤**에 돌았다 — 단, 중첩 쪽이 **prefix 세그먼트를 안 만드는 경우**에만(`case` arm(라벨 유무 무관)·라벨 없는 `if`/`else` 본문). iverilog 는 감싸는 스코프의 블록 로컬이 먼저이고, 시리즈 전 vita 도 그랬다.

소유권 판별자가 **bool**("generate 본문 안에서 수집됐나")이었고, 그건 **중첩된 두 generate 스코프를 못 가른다** — 둘 다 yes 이고 prefix 없는 쪽은 부모의 키까지 공유한다. 그래서 안쪽 flush 가 **부모의 항목을 claim** 해 자기(더 늦은) 랭크로 방출했다. 랭크 슬롯은 이미 의도한 순서를 담고 있었다(`RANK_GEN_BLOCK_LOCAL` 2 < `RANK_GEN_NESTED` 3) — **소유권이 그걸 우회**한 것이다.

이제 항목이 **소유 스코프의 랭크 경로**를 들고 다닌다(양쪽 pending 맵). 그건 랭크 자체와 **똑같이** 단계 간 안정적이고(전체 순서가 기대는 바로 그 성질), prefix 유무·중첩 깊이와 무관하게 스코프를 가른다.

바꾸자마자 **미방출 가드가 곧바로 두 번째 관련 버그를 loud 로** 드러냈다 — 인터페이스 인스턴스가 자기 랭크 스코프 **밖**에서 넷을 만들고 **안**에서 flush 해서 아무 flush 도 그 항목을 claim 하지 않았다(선언 시점이 pre-size 와 초기화자가 기록되는 자리이므로 넷 선언 루프를 스코프 안으로). 검증: 중첩 generate × 블록 로컬 **98 조합**(바깥 7 × 안쪽 7 철자 × 소스 순서 2) 0 불일치.

#### 4.5.264 gen-item 리스트의 맨몸 `begin…end` 도 문법이다 (2026-07-28, format 26 불변) ✅

리전보다 한 겹 아래. 파서가 `if`/`for`/`case` 본문의 `begin…end` 를 벗기고 라벨을 끌어올리므로, `GenItem::Block` 은 **오직 gen-item 리스트의 자유 항목**으로만 도달한다 — 그게 **anachronistic surround**(iverilog 가 경고하며 문법으로 취급)다. `elaborate_gen_scoped` 의 unlabeled arm 이 `true` 하나로 **두 역할**을 겸하고 있었다.

경계는 실측으로 정확하다: 맨몸 `begin` = 투명(선언 순서) · `begin : lb` = 스코프(먼저) · `if (1) begin` = 라벨 유무와 무관하게 스코프. **호출부가 어느 역할인지 알고 있으니** 호출부가 말한다. 확인 리뷰가 리전 수정 자체는 이미 clear 했고 이걸 찾았다 — 리전과 **같은 오분류·같은 모양의 수정**. 이후 재스윕: 모듈 스코프 {모듈 변수·자식 인스턴스·인터페이스·generate 블록·인스턴스 품은 블록·generate 리전} **720 순열 0 불일치**.

#### 4.5.263 generate REGION 은 스코프가 아니라 문법이다 (2026-07-28, format 26 불변) ✅

`generate … endgenerate` **리전**은 순수 문법이다(IEEE 1800 §27.3) — `if`/`for`/`case` 밖에, 리전에 **직접** 쓴 항목은 평범한 모듈 항목이다. 그런데 함수 하나가 **리전과 generate 블록 본문 둘 다**를 담당하는데 이 시리즈가 거기에 랭크 스코프·`in_generate_body`·자체 flush 를 달아서 **리전이 블록처럼 굴었다**.

실측·이등분: `generate if(1) begin : g int gv = 7; end int mv = g.gv;` 가 iverilog 7 / HEAD **0**(f4369c4 에서 시작) · 리전의 맨몸 인스턴스가 옆 블록보다 먼저(5fc6262 에서 시작). **둘 다 1fe06e7 에선 맞았으므로 이 시리즈가 만든 회귀**다. 리전은 `is_scope=false` 로 다시 투명해지고, 스코프는 if/for/case/block 본문뿐.

부수로 모듈 sweep 과 generate VarInit walk 을 **선언 순서 한 바퀴**로 합쳤다 — 루프 두 개로는 표현할 수 없는 것이 있었다: `int a; generate int b; endgenerate int c;` 는 a,b,c 여야 하는데 시리즈 전엔 a,c,b, 시리즈 후엔 b,a,c 였다. generate 블록은 도달 시점에 flush 하고 그건 sweep 자체의 flush(맨 끝)보다 앞이므로, **generate 스코프가 모듈 자기 변수보다 먼저**라는 규칙은 소스 위치와 무관하게 유지된다(120 순열 재검).

#### 4.5.262 bind band 를 인스턴스 경계에서 리셋 (2026-07-28, format 26 불변) ✅

band 는 **이 인스턴스가 어떻게 도달됐는지**를 말하는데, 모듈이 **스스로 선언한** 자식은 부모가 어떻게 도달됐든 평범한 본문 항목이다. `rank_band` 를 bind 루프 주위에서만 세팅하고 `elaborate_instance` 가 자식으로 들어갈 때 리셋하지 않아 **bound 서브트리 전체로 샜다** — bind 로 도달한 모듈이 자기 본문 자식까지 band 1 로 키잉했고, 그 모듈이 **자신도 bind 대상**이면 본문 자식과 bound 자식이 band 에서 충돌해 **본문 오프셋 대 컴파일 유닛 오프셋** 비교로 떨어졌다(band 가 막으려던 바로 그것). 증상도 §4.5.261 이 고친 것과 같고 한 겹 깊을 뿐 — 안쪽 `bind` 줄 위치와 파일 나열 순서가 답을 바꿨다.

**이 시리즈가 두 라운드 전에 이미 적어둔 규칙**(경계에서 리셋 안 한 상태 플래그는 경계를 넘어 거짓말한다)을, 그 뒤에 추가한 플래그에 적용한 것이다. iverilog 는 `bind` 를 파싱조차 못 하므로 테스트는 오라클 값이 아니라 **불변식(레이아웃 무관성)** 을 핀한다.

#### 4.5.261 인스턴스 랭크는 성분 3개 — 오프셋 하나가 세 질문에 답하고 있었다 (2026-07-28, format 26 불변) ✅

최종 리뷰가 소스 오프셋 하나를 랭크 키로 쓰는 것이 **세 가지 방식으로 깨진다**는 걸 전부 재현했다.

**① root 의 오프셋은 순서가 아니다.** `--top zz --top aa` 는 준 순서대로 elaborate 하고, `-L` 라이브러리 모드는 유닛마다 따로 컴파일하므로 **다른 유닛의 오프셋은 비교 자체가 무의미**하다(한 파일에 400바이트 주석을 넣자 답이 바뀌었다). root 는 **root 리스트에서의 위치**로. 이건 이번 라운드 **직전엔 맞았고 §4.5.260 이 깨뜨린 것**.

**② `bind` 디렉티브는 컴파일 유닛에 산다** — 대상 모듈 본문 안의 위치가 아니다. 둘을 비교하니 **`bind` 줄을 어디 썼느냐**로, 파일을 나누면 **명령줄 나열 순서**로 답이 바뀌었다. bind 체커는 **자기 band**(대상이 스스로 선언한 모든 것 뒤).

**③ 인스턴스 배열 원소가 키를 공유했고, §4.5.260 커밋 메시지의 근거가 그냥 틀렸다** — "`init_procs` 가 ProcId 로 tie-break 한다"는 **랭크 벡터가 동일할 때만** 성립한다. 원소의 자식 스코프는 `[…,1,K']`, 자기 변수는 `[…,2,0]` 로 **다른 벡터**라 사전식 정렬이 **슬롯 기준으로 원소들을 가로질러** 묶어 서브트리를 뒤섞었다(`tb.u[0].own + tb.u[1].own` 이 iverilog 5 인데 6). 원소 인덱스를 키에 포함.

교훈: **한 값이 세 질문에 답하고 있으면 성분을 나눠라**. 그리고 tie-break 를 근거로 쓸 땐 **정말 tie 인지** 확인하라 — 여기선 tie 가 아니었다.

#### 4.5.260 인스턴스 랭크를 pass 가 아니라 선언 위치로 (2026-07-28, format 26 불변) ✅

재리뷰가 §4.5.259 의 F4 수정이 **문제를 고친 게 아니라 틀린 절반을 맞바꿨다**는 걸 잡았고, 그 hunk 하나만 되돌려 증명했다. direct-body **인터페이스는 Nets pass**, 모듈 자식은 **Instances pass** 에서 elaborate 된다 — 인터페이스에 랭크 스코프를 준 순간 둘이 **같은 per-scope 카운터**를 뽑는데 그 카운터는 (F2 때문에) walk 사이에 리셋되지 않으므로, 소스가 뭐라 하든 **모든 인터페이스가 모든 모듈 자식보다 낮은 번호**를 가져갔다. 수정 전엔 "인터페이스가 먼저 쓰인" 절반이 맞고 나머지가 틀렸는데, 수정 후 정확히 반대가 됐다(210 설계 순열 행렬에서 양쪽이 **겹치지 않게** 실패).

**카운터로는 표현 불가** — 둘이 서로 다른 pass 에서 세어지기 때문이다. 둘 다 **선언 이름의 소스 오프셋**을 쓴다(그게 "선언 순서"의 정의이고 모든 pass 에서 같다). 인스턴스 배열 원소는 오프셋을 공유하고 `init_procs` 가 ProcId(= unroll 순서)로 tie-break. 직접 재측정: 모듈 스코프 {모듈 변수·자식 인스턴스·인터페이스·generate·인스턴스 품은 generate} **120 순열 전부** iverilog 일치(양방향·계층 읽기 `int iv = tb.u0.c + 1` 포함), staged 동일.

#### 4.5.259 초기화 phase 적대 리뷰 — 하강 4 + false-loud 1 (2026-07-28, format 26 불변) ✅

두 렌즈가 **반대편에서 상위 2건을 각각 독립 발견**했다.

**F1(최광범위·silent)**: 초기화 자신의 이벤트를 죽이려고 dirty 리스트를 **통째로** 비웠는데, 그 리스트엔 **arm 이전에 도는 t0 cont-assign settle** 의 쓰기도 올라와 있었다. 그건 복구 불가다 — run 루프 안의 두 번째 settle 은 같은 값을 쓰고 `note_change` 는 **실제 변화만** 기록하므로 `assign w = 1'b1;` 에 걸린 `always @(w)` 가 영영 안 뜬다. 게이트가 "설계 어디든 초기화자 하나라도"였으므로 **무관한 `reg r = 1'b0;` 하나로 전 설계 발화**. phase 가 **자기가 쓴 것만** un-dirty 하도록.

**F2(silent)**: 랭크 카운터가 스코프당 하나였는데 네 generate walk 중 **Instances walk 만** `Instance` 항목을 방문한다 → 인스턴스 **뒤에** 쓰인 generate 가 VarInit 과 Instances 에서 **다른 번호**를 뽑고, 그 자식 인스턴스가 자기 generate 의 flush 와 안 맞는 경로에 실렸다. 인스턴스를 옮기면 답이 바뀌는 것으로 differential 렌즈가 잡았다. **슬롯당 카운터**로 — 한 walk 만 방문하는 슬롯은 그 walk 안에서 계속 세고, generate 슬롯은 영향 없음.

**F3(silent)**: `package.rs` 가 크레이트에서 **유일하게 랭크 없는 flush** 였다 → 패키지 초기화자가 phase 에 아예 없어서 모든 모듈 초기화자 **뒤에** 돌고(모듈 `int m = p::pv + 100;` 가 0 을 읽음) 쓰기가 이벤트를 냈다(패키지 `logic pclk = 1'b1;` 에 **가짜 posedge**). 그 자리 주석이 예전엔 안전을 보장하던 불변식("패키지가 먼저 elaborate 되므로 ProcId 가 앞선다")을 여전히 주장하고 있었다 — **초기화가 phase 가 된 순간 그 보장은 무효**다. 실제로 "낮은 ProcId = 먼저 실행"에 의존하던 유일한 곳.

**F4(silent)**: 인터페이스 인스턴스는 스코프인데 랭크 스코프를 아무도 안 밀어서, 그 flush 가 **바깥 스코프의 자기-변수 슬롯**을 빌려 썼다. 게다가 두 호출부가 모듈 자기 flush 와 **다른 pass** 에서 도니 랭크 벡터가 그냥 충돌 → 순서가 ProcId tie-break 로 결정됐다. 모듈 자기 초기화자가 **두 인터페이스 사이**에 끼고, generate 안 인터페이스가 그 generate 의 자기 변수 뒤로 갔다.

**F5(false-loud)**: 중첩 관계인 span 이 **이름 전체**를 실격시켜서, 죽은 `generate if (0)` arm 이 자기 안의 중첩 `k` 로 **다른 곳의 살아있는 쌍**의 스코핑을 철회시켰다. 후보에서 **그 span 만** 빼도록(리뷰 S3 규칙의 일반화). 뺀 span 은 예전 그대로 flat net 을 쓰고 살아남은 것들은 서로 다른 `$blk$` net 을 받으므로 alias 불가 · 진짜 중첩 쌍은 생존자가 2 미만이라 여전히 loud. 프로세스 walk 가 **branch path** 를 들고 다녀 한 generate if/case 의 서로 다른 arm(동시 존재 불가)은 서로 shadow 하지 않는다.

부수: 잘린 사이드카가 IR 에 없는 프로세스를 지목하면 **조용히 skip 이 아니라 fatal**(20줄 위 fork-mode 게이트와 같은 급) · 두 렌즈가 나열한 낡은 주석 전부 교정(상수 fold 가 남긴 `net.init` 전제 · tie-permutation 설계를 아직 설명하던 사이드카 문서 3곳).

#### 4.5.258 generate 안 블록 로컬도 모듈과 같은 같은-이름 규칙 (2026-07-28, format 26 불변) ✅

마지막 형태: generate **안**에서 같은 `string s[2]` 를 선언한 두 블록이 loud 였고, 가족 전체(queue·dyn·assoc·스칼라 string)가 거기서만 그랬다. 원인은 이번 라운드가 다뤄온 string 배열 기계가 아니라 — **분류기 둘이 `module.body` 의 `ModuleItem::Proc` 만 훑어서** generate 안 프로세스가 분류 대상 집합에 아예 없었다. generate 를 아는 공용 walk 으로 둘 다 고쳤다(둘은 순수 AST 함수이고 Nets 단계 hoist 와 Logic 단계 lowering 이 **같은 집합**을 봐야 하므로 공유가 필수). generate-for 본문은 몇 번을 unroll 하든 **AST 하나**이고 각 unroll 이 자기 prefix 로 elaborate 되므로, 루프 안에서 한 번 선언된 이름은 **한 블록** — 이건 한계가 아니라 정답이다(복사본끼리는 충돌 불가·서로 다른 두 블록만 충돌).

#### 4.5.257 초기화는 프로세스가 아니라 PHASE (2026-07-28, **format 25 → 26**) ✅

iverilog 실측 둘이 같은 결론을 강제했고 **둘 다 순서 얘기가 아니다**: `reg clk = 0;` 의 `always @clk` 가 iverilog 2 / vita **3**, 비상수 `int nc = src+1;` 의 `always @nc` 가 iverilog 0 / vita **1**. **선언 초기화자는 이벤트를 만들지 않는다.** §6.21 의 "before any initial or always block starts" 는 문자 그대로 **arm 이전 phase** 이며, 초기화자 프로세스를 올바른 **순서**로 돌려도 그때는 이미 arm 이 끝난 뒤라 못 고친다. 엔진이 arm 루프 **앞에서** 초기화 순서대로 실행하고 `final` 블록처럼 arm 에서 제외한다(`run_body` 는 `run_finals` 가 이미 쓰던 경로·합성 초기화자 본문은 직선이라 suspend 불가).

그 phase 가 있어야 **상수 fold 제거**가 안전해진다. 상수 초기화자는 net 생성 시점에 `net.init` 으로 접혀 **초기화 순서 밖**에 있었고, 그래서 vita 가 **자기와 모순**했다 — generate 초기화자가 모듈 `int mm = 77;` 을 읽으면 77, 같은 읽기가 `int mm = f();` 이면 0(iverilog 는 둘 다 0). phase 없이 fold 만 뺐다면 하강이었고 **테스트 29개가 그렇게 말했다**(초기화 엣지를 죽이고 있던 게 그 fold였다).

옛 핀 2개가 뒤집혔고 **둘 다 작성 당시 스스로 "발산"이라 적어둔 것**: `cross_scope_module_read_is_a_documented_init_order_race`("iverilog 는 0 … oracle-matched 아님" → 이제 0, 이름도 교체) · `forward_ref_init_is_documented_leniency`(패키지 `int a = b + 1; int b = 5;` — iverilog 는 아예 거부라 오라클 없음 — `a=6` → `a=1`, 비상수 쌍둥이가 원래 주던 값).

#### 4.5.256 t0 정적 초기화 순서를 pass 순서가 아니라 데이터로 (2026-07-28, format 25 불변) ✅

**축이 둘인데 자료구조가 하나**였던 게 이전 시도들이 계속 실패한 이유다. **소유권**(어느 flush 가 어느 pending 을 가져가나) = innermost-first, **초기화 순서** = 완전히 다른 축. `$random` 증인으로 실측한 규칙:

- **모듈**: ①generate 스코프 ②자식 인스턴스 ③자기 변수 ④자기 블록 로컬
- **generate**: ①자식 인스턴스 ②자기 변수 ③자기 블록 로컬 ④중첩 generate

14 프로브가 각 규칙의 **양방향**을 핀한다(모듈 generate 는 마지막에 써도 먼저·자기 변수는 처음에 써도 마지막; generate 의 중첩 generate 는 앞에 써도 자기 변수 뒤·자식 인스턴스는 앞). **어떤 pass 순서로도 안 나온다** — 자식 초기화자가 부모보다 먼저인데 부모 프로세스는 자식이 존재하기도 전에 생성된다. 그래서 `(slot, seq)` **랭크 경로**로 기록하고 per-ProcId 키로 내보내 스케줄러가 소비한다. elaborate pass 는 **그대로 뒀다**.

부수로 cross-scope 읽기 없이도 보이는 것 하나: vita 는 §6.21 을 "낮은 ProcId" 로 근사했는데 그 근사는 **인스턴스 경계를 못 넘는다** — 부모 `initial` 이 자식의 `string` 을 읽으면 빈 기본값(자식 초기화자 프로세스는 pass 8, 부모 자기 프로세스는 pass 7).

#### 4.5.255 같은-이름 `string` 배열 correct-support + 그 리뷰가 잡은 2연 하강 (2026-07-28, format 25 불변) ✅

**S1 은 형태를 제외해서 답했었다.** §4.5.253 이 `dyn_storage` 를 스칼라로 되돌리자 두 블록의 `string s[2]` 는 다시 loud 가 됐다 — 하강은 막았지만 정답은 아니다. **제외가 결함이 아니었다**: 라우팅된 string 배열은 원소 저장소를 **선언 prefix** 에 등록하는데, 그걸 채우는 두 곳(decl-init 수집기의 `has_fixed_string_array_storage`, `new[n]` pre-size)이 **모듈 prefix** 에서 돌고 있었다. 수집기를 `with_scope` **안**으로 옮기고 pre-size 를 거기 기록하니 비대칭이 사라졌고, `string` 로컬은 unpacked 모양과 무관하게 전부 스코프 자격을 얻는다. iverilog 대조: 양쪽 초기화자 · `string s[2][2]` row-major · 내림차순 `s[3:1]` · fixed+dyn 한 이름 · 루프 재진입 · **fork 두 arm**(전엔 loud). 안 여는 2개(모듈 넷 이름충돌 · 블록 중첩)는 loud 로 핀. 의도적 불일치 1건 = 미대입 원소는 `""`(§6.16) — iverilog 는 공백 한 칸을 찍고 `.len()` 을 2 라 하는데 **한 블록만 있어도** 그러니 규칙이 아니라 미초기화 메모리다.

**리뷰 1차 — generate 본문에 prefix 가 없다는 걸 순서 규칙이 몰랐다.** iverilog 는 `case` arm·라벨 없는 `if`/`begin` 도 스코프로 보고, generate 스코프 static 을 **모듈보다 먼저** 초기화한다(`begin : g int gm=$random;` 가 1번, 모듈 `int mm=$random;` 가 2번 — 소스 순서 무관). vita 는 그것들을 "블록 로컬이니 맨 뒤"로 분류해 **잘 되던 3형태가 조용히 틀어졌다**. 소유권 플래그(`in_generate_body`)를 달고 generate VarInit walk 을 모듈 sweep **앞으로** 옮겼다 — 블록 로컬과 무관한 **기존 모듈-먼저 역전**도 같이 닫혔다. 두 번째 하강은 pre-size: walk **시작 때** 배출하니 안쪽 generate 본문의 `new[n]` 이 바깥 프로세스로 가서, 안쪽이 이미 내보낸 원소 쓰기를 **지웠다**(빈 배열·exit 0). flush 시점 배출로 — flush 순서가 곧 소유 순서다.

**리뷰 2차 — 두 렌즈가 반대편에서 같은 회귀를 찾았다.** prefix 만으로 claim 하는데 소유권은 플래그가 되었으니, walk 재배치 후 **generate 의 flush 가 모듈 자신의 블록 로컬을 가져가** 모듈 sweep 앞에서 방출했다. **빈 generate 하나·`if(0)` 하나로도 발화**(flush 가 무조건이라). 결과 둘 다 exit 0: 모듈 블록 로컬 초기화자가 모듈 변수를 **초기화 전에** 읽고(`(mm!=0)?"SET":"ZERO"` → `ZERO`), 라우팅 string 배열이 `new[n]` 과 **분리**됐다(presize 배출은 이미 소유권으로 나뉘어 있었으므로 쓰기만 가져가고 presize 는 남아 뒤늦게 배열을 지움) — 이 작업이 없애려던 바로 그 silent-empty 가 반대편에서 재도달. **prefix 는 어느 스코프냐를, 플래그는 누구 것이냐를 답한다.** 부수로 soundness 렌즈가 계측으로 `child`/`front` 분기가 **한 번도 안 탄다**는 걸 보였고(순서는 전적으로 walk 재배치가 만든다), 인스턴스 경계에서 플래그를 리셋하지 않아 **generate 안에 인스턴스화된 자식이 자기 본문 전체를 generate-owned 로** 태깅하던 것도 찾았다. 테스트가 놓친 이유가 정확히 진단됐다 — 두 파일 다 generate 가 있고 둘 다 모듈 블록 로컬이 있는데, **둘을 함께 가진 테스트가 없었다**.

**잔여(실측·전부 pre-existing)** → ROADMAP §2: 중첩 generate 안팎 순서 · 상수 초기화자가 순서 모델 밖 · 자식 인스턴스가 부모보다 뒤 · generate 안 같은-이름 `string s[2]` 는 아직 loud(문구도 위치를 틀리게 지목).

#### 4.5.254 t0 정적 초기화자 순서 — iverilog 실측 규칙으로 (2026-07-28, format 25 불변) ✅

리포트의 S4 는 "블록 로컬 string 초기화자가 항상 같은 블록의 비-string 뒤"였는데, **`$random` 을 순서 증인으로** 재보니 그건 한 면이었다. 실측 규칙 = **모듈 스코프 초기화자 전부(선언 순서) → 그다음 블록 로컬 전부(선언 순서)**, 블록이 모듈 선언보다 앞/뒤인지·string 인지·`$blk$` 스코프를 받았는지와 무관.

vita 는 앞쪽 절반이 거꾸로였다. `hoist_block_local_nets` 가 `collect_var_init_drivers` **보다 먼저** 돌기 때문에 `pending_var_inits` 로 직행한 블록 로컬이 모든 모듈 초기화자를 앞질렀다. r19 가 그걸 **string 사례로만** 보고 string 만 끝으로 미뤄 고쳤고 — 모듈 관계는 고쳤지만 블록 안 관계를 깼다(=S4). HEAD 와 1fe06e7 에서 **똑같이** 조용히 틀린 형태 3개: 모듈-vs-블록로컬 · 한 블록 안 string-vs-비string · 스코프-vs-플랫 인터리브.

리스트 3개(`pending_block_local_string_inits`·`pending_scoped_bl_strings`·`pending_blk_inits`)를 **하나**로 합쳤다 — 키는 선언이 사는 전체 prefix, 값에 **선언 오프셋**. 각 flush 는 자기 키 + 직속 `$blk$` 자식을 claim 하고 오프셋 정렬 후 **연속 같은-prefix run** 을 방출한다(t0 `initial` 은 ProcId 순 실행이라 run 을 순서대로 방출하면 초기화자가 순서대로 실행된다). **선행 run 은 새 프로세스를 만들지 않고 sweep 의 initial 에 합류** — 스코프 블록 로컬이 없는 모든 모듈이 그 경우라 IR 불변(골든 무이동). `pending_block_local_string_inits` 는 이미 **writer 가 하나도 없는 죽은 필드**였다. 초기화자가 하나라도 미방출로 남으면 성공 경로에서 **loud** 로 실패한다 — 누락은 구조상 안 보이는 결함이라서.

#### 4.5.253 §4.5.251 적대 리뷰 — 하강 4건 수정 (2026-07-28, format 25 불변) ✅

**S1(최광범위·loud→silent)**: `dyn_storage` 를 **kind-only**(`matches!(d.kind, String)`)로 적었는데 바로 옆 주석은 "스칼라 `string` 이 합류한다"고 말한다. 그래서 `string s[2]` 가 통과했고, **원소 저장소는 선언 prefix 아래**라 pre-size `new[n]` 과 원소 쓰기를 해석하는 모듈 prefix 에서 보이지 않는다 → 길이 0, 모든 쓰기 폐기, **exit 0**(PRE 는 loud). 버그 두 줄 위에는 r19 주석이 "초기화자 있는 fixed string array 는 여기 못 온다"고 여전히 보증하고 있었다 — 가드가 `automatic` lifetime loud 였을 땐 참, static 이 자격을 얻은 뒤엔 거짓. **술어가 자기 주석과 다르면 그 간극이 곧 버그다.**

**S2(§6.8 선언 순서·loud→silent)** 두 갈래: ① 한 블록의 초기화자를 메인 sweep 과 후행 그룹으로 **쪼개서** 인터리브가 사라졌다(`int a = $random; int q[$] = '{$random};` → `a` 가 1번째, `q` 가 **4번째** draw). 이제 스코프가 있는 블록은 **모든** 초기화자를 그 그룹으로 보낸다(스코프 없는 이름도 `$blk$` prefix 아래서 바깥으로 걸어 해석되므로 안전). ② 그룹을 **ASCII** 로 정렬해 `"$blk$148" < "$blk$32"` — 뒤 블록이 먼저 돌았다 → 숫자 정렬.

**S3(회귀)**: 감싸는 선언을 **gather 에 넣은 것만으로** 그 이름이 shadowed 로 보여, 이미 잘 돌던 inner/sibling 쌍의 스코핑이 **철회**됐다. 같은 이름의 다른 선언 span 을 **포함하는** widened span 은 후보에서 제외한다 — 스코프를 얻지도 않지만 남의 것을 빼앗지도 않는다.

**S4** 는 리뷰가 기존 결함으로 확인(블록 로컬 string 초기화자가 항상 같은 블록의 비-string 뒤에 배치 — `pending_scoped_bl_strings` 설계 그대로). 기록만.

**게이트**: 4733 → **4736** tests · clippy/fmt clean.

#### 4.5.252 frame executor 가 temp 를 못 나르는 자리의 `$sformatf` (2026-07-28, format 25 불변) ✅

round-20 인접 마지막 갭(frame **function** 본문의 `$sformatf`)을 그라운딩하자 **근인이 frame 이 아니었다**: `$sformatf("<%s>", $sformatf("%0d",7))` 은 **모든 문맥에서** `<   >` 로 빈다. `eval` 의 `SysFuncId::Sformatf` arm 이 **포맷 문자열을 무시**한다 — concat desugar(`"%s"×N`) 전용으로 쓰였고 코드 주석이 그렇게 적고 있다. **hoist 자체가 그 갭의 우회**였다.

수정 3개(전부 실측): `%s` 인자 리더가 중첩 `$sformatf` 를 `format_args_str` 로 재귀 · `expr_is_string_ast` 가 `$sformatf` 를 문자열 도메인으로(§6.16 — 반환형이 string; 없으면 `{"x", $sformatf(…)}` 가 packed concat 으로 샌다) · **formatter 를 반드시 거치는 위치**(문자열 대입의 직접 rhs·문자열 `return`)에서만 표현식 노드로 낮추기.

마지막 게이트는 **위치 하나씩 측정해서** 잘랐다 — ternary arm 과 태스크 인자는 generic `eval` 을 거쳐 빈 문자열/쓰레기가 나오므로 플래그를 세우지 않고 loud 유지. 부수 승격: `{2{$sformatf("a%0d",1)}}` = `a1a1`(iverilog), `{0{…}}` = 빈 문자열 — hoist 로는 표현 불가였다(한 번 렌더해 temp 를 반복하니까).

**남은 뿌리 = 그 degenerate `eval` arm.** 렌더러를 `EvalCtx` 에서 쓸 수 있게 만들면 ternary·단락·`$monitor`/`$strobe`·태스크 인자가 함께 닫히고 hoist 를 은퇴시킬 수 있다.

#### 4.5.251 `$blk$` 스코프 경로의 decl-init 수집 (2026-07-28, format 25 불변) ✅

같은-이름 확장이 제외해야 했던 것들은 **전부 한 가지 이유** 때문이었다 — `$blk$` 경로가 decl-init 수집기 **前에 return** 해서 스코프된 `byte m[] = '{…}` 가 **비어서** 나왔다(loud→silent 0). 그래서 §4.5.249 는 초기화자 있는 선언과 스칼라 `string` 을 빼고 나갔고, §4.5.250 은 리뷰가 잡은 multi-name 가드를 그 위에 더 얹어야 했다.

그건 진짜 제약이 아니다. 스코프 경로가 이제 자기 초기화자를 수집하고(`collect_block_local_decl_inits` — 두 경로가 push 규칙 하나를 공유하도록 추출), `flush_pending_blk_inits` 가 **각 그룹을 자기 prefix 로 복원해** 재생한다(일반 리스트와 string 리스트 양쪽 — 후자는 모듈 스코프 string 을 읽을 수 있어 따로 배수된다).

그러자 제외 3개가 전부 사라지고, 각 형태가 loud 가 아니라 **정답**이 된다(전 줄 iverilog 일치):

| 형태 | 결과 |
|---|---|
| `byte m[] = '{1,2}` / `byte m[] = '{7}` | A=2 02 · B=1 07 |
| `int q[$] = '{1,2,3}` / `int q[$] = '{9}` | Q=3 3 · R=1 9 |
| `string s = "aa"` / `string s = "bbb"` | A=aa 2 · B=bbb 3 |
| `byte m[], n = g` / `byte m[], k = g` | M=2 9 · N=3 9 ← §4.5.250 F1 형태 |

**남는 loud**: 모듈 넷과 이름이 겹치는 블록 로컬(선언 블록이 하나이고 모듈 이름이라 두 조건 모두 탈락) · 한 블록이 다른 블록을 감싸는 쌍(단일 레벨 hoist 로는 중첩 세그먼트를 못 맞춘다).

#### 4.5.250 §4.5.248/249 적대 리뷰 — 사다리 하강 6건 수정 (2026-07-28, branch feat-round20-report, format 25 불변) ✅

두 렌즈(**differential** = PRE 바이너리 vs POST vs 라이브 iverilog · **soundness** = IEEE 1800 대비 코드경로 독해)가 §4.5.248/249 에서 **loud→silent-wrong 5건 + 순수 회귀 1건**을 잡았다. 전부 수정 + 노출 형태 그대로 핀.

**5건의 공통 근원 = 평가를 옮긴 것.** "`$sformatf` 는 순수하니 옮겨도 공짜"가 틀린 프레임이었다 — 중요한 건 **몇 번 도는가 · 형제 대비 언제 도는가 · 도는 동안 무엇을 읽는가**.

| 축 | 형태 | 관측 |
|---|---|---|
| **몇 번** | `$monitor`/`$strobe` 인자 | 렌더가 문장 시점 1회로 굳어 `$monitor` 가 t=0 에만 출력(§21.2.3), `$strobe` 가 타임스텝 끝 아닌 값 보고(§21.2.2). **최상위 hoist 는 애초에 deferred 가족을 제외**했는데 새 중첩 pre-pass 가 안 했다 |
| | 단락 `&&`/`||` 우변 | `$sformatf` 는 순수해도 **인자는 아니다** — `c && (s == $sformatf("%0d", $random))` 가 `c` 거짓인데 시드를 소비(iverilog 차분) |
| | replication 값 | `{0{…}}` 은 0회 |
| **순서** | `show($urandom, {"<", $sformatf("%0d",$urandom), ">"})` | 인자1이 **두 번째** draw, 포맷이 **첫 번째** — 두 인자 값이 뒤바뀜 |
| **무엇을 읽나** | `q = '{q[0], 9}` | 확장이 먼저 clear 하므로 비워진 큐를 읽어 **X** |

수정: deferred 가족 제외 · 단락 우변/replication 미하강 · **왼쪽-접두 규칙**(`expr_is_inert` — 왼쪽이 전부 무해할 때만 hoist, 아니면 그 리스트는 거기서 멈춤) · 큐 원소를 clear 前 temp 로 스냅샷.

**의도적 divergence 1건**(기록): `q = '{q[1], q[0]}` 은 iverilog 가 스냅샷 없이 제자리 기록이라 `6 6`, vita 는 §10.7(우변 전체 평가 후 대입)대로 `6 5`. 나머지 줄은 전부 iverilog 일치.

**나머지 2건**:
- **multi-name 선언**: `$blk$` 스코핑은 **선언 단위** 결정인데 "초기화자 없음" 제외를 **이름 단위**로 물었다 → 자격 있는 `m` 이 초기화자 있는 형제 `n` 을 스코프 arm 으로 끌고 갔고, 그 arm 은 decl-init 수집기 前에 return 한다 → `int m[], n[] = '{1,2,3}` 이 `n.size()==0`, exit 0.
- **`'{}` actual 이 output/inout 에도 통과**: §13.5.2 는 lvalue 를 요구 — copy-out 이 조용히 버려지고, temp 넷이 **콜사이트당 1개**라 루프 안 호출이 **직전 활성의 write** 를 봤다(`size=0,1,2`).

**회귀 1건 = 게이트 극성**: static-init 게이트가 **REJECT** 결정을 `expr_no_ref` 로 했다. 그 워커의 "모르면 참조할지도"는 **ACCEPT** 게이트에서 옳고 여기선 뒤집힌다 — `pkg::PARAM`·시간 리터럴·`new()` 같은 미검증 초기화자가 전부 거부됐고, **메시지는 초기화자에 없는 변수를 지목**했다. 양의 쌍둥이 `expr_definitely_refs` 신설. **극성은 워커가 아니라 게이트의 속성**이다.

저severity 3건도 처리: DA 워크의 새 arm 을 **2-세그먼트 컨테이너 메서드**로 한정(v1 은 블록 로컬을 모듈 넷으로 publish 하므로 "태스크는 output actual 로만 쓴다"는 IEEE 스코핑 논거가 **정확히 IEEE 스코핑이 안 통하는 자리**에서 쓰였다) · pop 싱크를 `$`-fence(합법 SV 식별자라 VCD 에 덤프되고 사용자 선언과 충돌했다) · static-init 게이트가 자기 span 을 실어 위치 표시 · same-name 메시지에서 자기 슬라이스가 낡게 만든 규칙 문장 정정.

**게이트**: 4720 → **4731** tests · clippy/fmt clean · format 25 불변 · `hoist/mod.rs` 1000줄 초과로 `hoist/sformatf.rs` 분리.

**교훈**: **평가를 옮기는 최적화는 세 질문을 통과해야 한다 — 몇 번·언제·무엇을 읽고.** 하나만 봐도 나머지 둘에서 새는데, 이번엔 세 축 전부에서 샜다. 그리고 **워커의 극성은 게이트가 정한다**: 같은 보수적 워커가 accept 게이트에선 안전하고 reject 게이트에선 정상 코드를 거부한다.

#### 4.5.249 외부 round-20 §6 + §4.11 — elaborate 진단에 file:line, 같은 이름 동적 로컬 분리 (2026-07-28, branch feat-round20-report, format 25 불변) ✅

**§6 는 결함 목록이 아니라 요청이었다** — "`E3009`/`E3010` 에 file:line 이 없다. TB=top 은 같은 문구가 81 번 반복되는데 어느 선언인지 알 수 없고, **이것 하나가 §4.11 을 못 닫는 유일한 이유**다." lex/parse 진단은 처음부터 위치가 있었고 elaborate 만 없었다 — elaborator 가 전처리기의 `SourceMap` 을 볼 수단이 없었기 때문이다.

**구현**: `diag::SpanResolver` 트레이트(양쪽이 의존하는 `diag` 에 둠 — elaborate 는 전처리기에 의존하지 않는다)를 프런트엔드가 넘기고, `lower_stmt`/`elaborate_netvar_decl` 이 `cur_span` 을 앵커한다. 그래서 헬퍼 깊은 곳에서 난 진단도 **사용자가 쓴 구문**을 가리킨다. `hdl-ast` 에 `Stmt::span()` 추가(전 변형 exhaustive — 타입 형상 불변이므로 SchemaHash 무영향).

같은 절의 나머지 둘: **same-name dyn 메시지에 식별자가 아예 없었다** → 이름 + **판별 규칙**("둘 다 `automatic` 이고 서로 감싸지 않을 때만 분리 저장")을 말하게 했고, **"frame task"** 는 내부 용어다 → `task`.

**§4.11**: `$blk$` 스코핑이 **둘 다 `automatic`** 을 요구해서 static 이 하나만 섞여도 그 이름 전체가 loud 였다(리포트가 측정한 트리거 중 하나). 서로 겹치지 않는 블록의 같은 이름 **동적 저장** 로컬은 lifetime 과 무관하게 IEEE 상 **서로 다른 변수**이고, 어느 한쪽이 static 이면 **항상 loud** 였으므로 스코핑은 **순수 loud→support**다. 3-블록 dyn array 가 iverilog 와 일치(`A1 B2 C0`), 선언만 한 블록은 앞 블록 원소가 아니라 **빈 배열**을 본다 — loud 가 막으려던 바로 그것.

**제외 2 건은 가정이 아니라 측정**: static + **초기화자** 는 `$blk$` 경로가 decl-init 수집기를 건너뛰어 `size()==0` 이 나왔다(첫 시도에서 실측) → loud 유지. 스칼라 `string` 도 같은 이유 + 자체 coalesce 가드가 따로 있어 제외.

부수 수정: `$blk$` arm 안의 per-entry lifetime 게이트가 `d.lifetime` 로 감싸여 있지 않아, static 선언이 "`automatic` block-local ... per-entry lifetime" 이라는 말을 들었다.

**남은 79 건**은 리포트 자신의 16-블록 충실 복제본도 재현 못 한 트리거이고 이 체크아웃에서는 볼 수 없다 — 위치 정보가 그들이 좁힐 수단이다.

**게이트**: 4708 → **4717** tests · clippy/fmt clean · format 25 불변 · `block_local.rs` 가 1000 줄을 넘어 순수 AST 분류기 2 개를 `block_local_class.rs` 로 분리.

#### 4.5.248 외부 round-20 리포트 8 가족 — 블록 로컬·queue 관용구·named arg·$sformatf (2026-07-28, branch feat-round20-report, format 25 불변) ✅

외부 round-20 리포트(2026-07-27, base `1fe06e7`)가 **Xcelium sign-off 되는 실 TB** 로 12 가족을 self-contained 재현과 함께 격리했다. **2 건은 리포트 기준 커밋 이후 슬라이스가 이미 닫았다** — CRITICAL part-select 바운드 폴딩(`d[2*W-1:W]` 1-bit 붕괴)은 §4.5.229, enum task input formal 의 `.name()` 은 §4.5.234(sized 리터럴 enum 라벨). PRE(`1fe06e7`)로 3-way 확인 후 각자 자리에 재핀. 나머지 8 가족:

| § | 형태 | 근인 |
|---|---|---|
| 4.1 | fork arm 블록 로컬 + write | **BL1 의 규칙이 틀린 축이었다** ↓ |
| 4.2 | `automatic string s = "a"` | 스칼라 중 string 만 per-entry 가족에서 빠져 있었다(재초기화 emission 은 동일) |
| 4.3 | `q.push_back('{1,2})` · `void'(q.pop_front())` | 리시버 타입 기반 필드 concat 이 없었다 / pop 이 표현식 op 라 소비처가 필요 |
| 4.4 | 컨테이너 메서드가 형제 대입을 지움 | `stmt_no_ref` 에 `UserTaskCall` arm 부재 → DA 워크 통째 중단 |
| 4.5 | dyn-array formal 에 `'{}` actual | bare Ident 만 허용 |
| 4.6 | 문장 레벨 `'{…}` 대입 | 선언 초기화자와 **같은 확장기**가 있는데 문장 철자만 없었다 |
| 4.8 | task enable 의 named arg | 함수 경로엔 있고 태스크 경로에만 없었다 |
| 4.9 | `new[N]` decl-init | 거부가 **핸들 등록을 건너뛰어** 선언 1개가 진단 8개(그중 2개가 "undeclared") |
| 4.10 | `$sformatf` 위치 제한 | 문장 레벨 hoist 가 시스템태스크 인자에만 있었다 |

**§4.1 이 핵심 교정**: BL1(§4.5.228)의 규칙은 "값이 상수라 concurrency-immune"이었는데, 진짜 불변식은 **블록의 살아있는 활성이 하나**다. 그리고 그것은 **프로세스가 한 번만 도달하는 fork 의 모든 arm** 에서 성립한다. 그래서 `fork_multi` 를 **spawn 지점**(루프 조상, 반복 프로세스)에서 전파하도록 바꿨다 — fork 의 존재 자체가 아니라. `repeatable`(이 문장이 여러 번 실행되나)과 `fork_multi`(spawn 이 여러 번인가)가 **두 플래그인 이유**: arm 안의 루프는 한 스레드라 순차이므로 위험을 되살리지 않는다. 그 결과 거의 모든 TB 의 표준 워치독

```systemverilog
fork begin automatic int t = D; void'($value$plusargs(…, t)); #(t*1ns); end join_none
```

이 **arm 되고, 오버라이드를 받고, 그 시각에 발화**한다. 루프 안 fork · `always` 안 fork · fork 안 루프 안 fork 는 loud 유지.

**§4.8 은 오진이 두 겹**이었다: named arg 자체는 함수 호출에서 이미 동작했고, 리포트가 본 cascade("block-local `r` … read before its first write")의 근인은 **보수적 참조 워커에 `NamedArg` arm 이 없어서** 명백한 whole-var write 를 "이 rhs 가 `r` 을 참조할지도 모름"으로 답한 것이었다.

**§4.10 의 안전 경계**: `$sformatf` 는 순수하므로 **몇 번 도는지만** 지키면 된다 — 그래서 하강은 **ternary ARM**(한쪽만 실행)과 미검증 노드에서 멈추고, 그것들은 기존 loud 를 유지한다.

**fork 제한을 걷어내자 그 밑의 silent-wrong 이 드러났다**(메모리 규칙: "loud 가드를 걷으면 그것이 가리던 것이 드러나고, 그건 loud→silent 이며 네 책임이다"). **static** 블록 로컬의 초기화자가 같은 블록 `automatic` 형제를 읽으면, static init 은 t0 에 도는데 automatic 은 블록 진입 시 초기화라 값이 없고, 거기서 output/inout 으로 되쓴 값은 진입 초기화에 덮인다. 동일한 non-fork 형태는 **PRE 도 exit 0 에서 copy-out 前 값을 찍고 있었다** → 이제 loud(=상승).

**옛 fork 규칙을 인코딩한 핀 5 개**를 실제로 loud 여야 하는 것(다중 spawn)과 이제 지원되는 것의 **값**으로 재작성.

**게이트**: 4680 → **4708** tests · clippy/fmt clean · format 25 불변.

**교훈**: **제약의 근거가 "X 라서 안 된다"일 때, X 가 진짜 필요조건인지 다시 물어라.** BL1 은 "상수라서 안전"을 규칙으로 삼았는데 그건 충분조건이었을 뿐이고, 필요조건("활성이 하나")을 직접 쓰자 실전 관용구가 통째로 열렸다. 그리고 **리포트의 진단명을 믿지 마라** — §4.8 은 "named argument 미지원"으로 보고됐지만 근인은 워커의 빠진 arm 이었다.

#### 4.5.247 §4.5.246 회귀 수정 — flatten 된 블록 로컬이 generate 스코프를 shadow 하던 문제 (2026-07-28, branch fix-shadow-generate-leak, format 25 불변) ✅

**적대 리뷰가 잡은 BLOCKING 회귀**(§4.5.246 머지 직후 도착). v1 은 절차 블록 로컬을 **둘러싼 prefix 의 bare name** 으로 flatten 하는데, generate 블록 안에서는 그 키가 `t.g.W` 로 **모듈 상수 `t.W` 와 다르다**. 그래서 §4.5.246 의 inner-net-wins 술어가 이것을 **정당한 shadow 로 오인**했고, 그 generate 스코프의 **다른 모든 reader**(형제 `initial`·continuous assign·중첩 generate)가 **한 프로세스의 사설 변수**를 집었다.

**영향은 §4.5.246 이 만든 것**이다 — 아래는 전부 변경 前 byte-correct 였다:

| 형태 | iverilog | §4.5.246 |
|---|---|---|
| `assign y = W`(블록 로컬 W 는 write-only) | 4 | **9** |
| **per-instance override** `#(.W(4))`/`#(.W(5))` | 4 5 | **9 9**(override 무력화) |
| **genvar** `i` | 0,1 | **77,77** |
| enum label · package import · `#W` delay · `assert` | 정상 | 전부 오염 |

**수정**: flatten 경로(`hoist_block_local_nets` 의 `elaborate_netvar_decl(..., true)`)가 만든 FQ 키를 사이드셋 `hoisted_block_local` 에 기록하고, shadow 술어에서 **그 키를 제외**한다. 리뷰어가 제안한 판별자 그대로다 — 대안(블록 로컬 leak 게이트를 generate 스코프 전체로 확대)은 오늘 byte-correct 인 설계들을 loud 로 만든다.

**범위 정정도 함께**: §4.5.246 의 커밋/테스트가 "블록 로컬 해소"라고 적었으나 실제로는 **서브루틴 안의** 블록만이다 — 모듈 레벨 프로세스의 `initial begin int W; … end` 는 flatten 키가 param 과 같아 술어가 구분할 수 없다(iverilog 9 / vita 4·§2 잔여로 기록). 테스트 주석을 정정했다.

**게이트**: 4679 → **4680** tests(회귀 3형: continuous assign·genvar·per-instance override) · clippy/fmt clean · format 25 불변.

**교훈**: **"이름이 안쪽 스코프에 있다"는 것과 "그 스코프의 것이다"는 다르다.** flatten 된 블록 로컬은 키만 안쪽일 뿐 **한 프로세스의 사설 변수**다 — shadow 판정에 쓸 집합은 "안쪽 키" 가 아니라 **"그 스코프가 실제로 선언한 것"** 이어야 한다. 그리고 이번 세션에서 **유일하게 리뷰 없이 머지한 변경**이 바로 회귀를 담고 있었다.

#### 4.5.246 inner NET 이 outer PARAM 을 shadow — 마지막 ①-급 silent-wrong 해소 (2026-07-28, branch feat-inner-net-shadow, format 25 불변) ✅

**착수 근거**: §4.5.244 판단표의 **C**, §4.5.245 가 메커니즘을 확정한 항목. 남아 있던 **유일한 ①-급 silent-wrong**.

**결함**: 서브루틴 로컬이 모듈 param 과 이름 충돌하면 **param 이 이겨서 로컬 값이 조용히 사라졌다** — `localparam W=4` + `function int f(); int W; W=9; return W;` = vita **4** / iverilog 9. function·**task**(output 경유)·**begin/end 블록**·`localparam int` 전부 동일(`9 4 4 3` vs `9 9 9 8`).

**근인 = 규칙이 절반만 적용돼 있었다**. `lower_expr` 은 이미 `walk_scopes_key` 로 **결합 집합**(params|nets|string-param|real-param) 위에서 **innermost 바인딩을 재도출**하고, 주석에도 "inner net 이 이긴다"고 적혀 있었다. 그런데 fall-through 가 `lookup_scoped` 를 부르는데 **그건 params 만 도는 자기 walk** 라서 방금 도출한 키를 무시했다 — 그래서 OUTER param 이 INNER net 을 이겼다. **fix = 도출된 innermost 키가 net(≠param)이면 param 분기를 건너뛴다**(16줄, 한 site). 새 머신러리 0 — §4.5.227/241 과 같은 패턴("퍼널은 이미 있고 호출부가 안 쓴다")이 **세 번째** 반복됐다.

**실패 전력에 대한 방어**: §4.5.218 S1(같은 fix 시도가 **중첩 generate body 를 조용히 삭제**)을 회귀 테스트로 직접 박았다 — generate-scope localparam 이 내부 for 바운드와 if 를 구동하는 형태가 iverilog 와 완전 일치. 반대 방향(로컬 없을 때 outer param 이 계속 이겨야 함)과 충돌 없는 로컬(항상 정상이었음)도 컨트롤로 핀.

**게이트**: 4675 → **4679** tests(신규 `inner_net_shadow.rs`×4) · clippy/fmt clean · format 25 불변 · diff = **1 파일 16줄**.

**적대 리뷰가 BLOCKING 회귀 1건을 잡았고 즉시 수정했다(§4.5.247)** — 아래 참조. 머지 후 도착이라 후속 커밋으로 처리.

#### 4.5.245 inner-NET shadow 메커니즘 확정 (2026-07-28, branch feat-shadow-grounding, format 25 불변) ✅

**착수 근거**: §4.5.244 판단표의 **C** — 유일하게 남은 ①-급 silent-wrong. 구현 전 §2 그라운딩(예측 금지)을 먼저 했고, **기록보다 넓고 트리거는 더 좁다**는 것이 드러났다.

**트리거 = 모듈 스코프 param 과의 이름 충돌, 그것뿐**(실측): 충돌 없는 로컬(`int X`)은 **정상**(9). 충돌하면 **param 이 이긴다**.

| 형태 | iverilog | vita |
|---|---|---|
| 로컬 `int X`(충돌 없음) | 9 | **9** ✓ |
| function-local `int W`(`localparam W=4`) | 9 | **4** ✗ |
| `begin…end` 블록 로컬 `int W` | 9 | **4** ✗ |
| function-local `int E`(`localparam int E=3`) | 8 | **3** ✗ |
| **task**-local `int W` → `output` | 7 | **4** ✗ |

**기록보다 넓다**: ROADMAP 은 function-local 만 적어뒀으나 **task-local 과 블록 로컬도 동일 발화**다.

**근인 확정**: `lower_expr` 의 이름 해석 순서가 **subst → out_subst → string-param → `lookup_scoped`(params) → net** — 서브루틴 로컬 net 보다 **param 이 먼저** 잡힌다. 충돌이 없을 때 정상인 이유도 이것으로 설명된다(param 조회가 miss 하면 net 으로 내려간다).

**선행조건 정정**: ROADMAP 은 `gather_local_decl_names` 패턴을 쓰라고 했는데, **그 함수는 모듈 레벨(params+ports)만 모은다** — 필요한 것은 **서브루틴별 로컬 이름 집합**이라 신규다(패턴은 재사용하되 함수는 새로). 순서 독립·AST 순수함수여야 §4.5.218 S1(중첩 generate body 조용한 삭제) 재발이 없다.

**이번 반복은 여기서 멈춘다** — 이름 해석은 blast radius 가 가장 큰 영역이고 실패 전력이 1회 있다. 절반만 바꾼 상태로 남기는 것보다, **메커니즘·범위·선행조건을 확정한 체크포인트**가 다음 착수에 더 안전하다.

**게이트**: 4675 tests green(변경 없음) · **코드 변경 0**.

**교훈**: **DEEP 항목은 "구현"과 "그라운딩"을 별도 반복으로 쪼개도 된다.** 근인·범위·선행조건이 확정되면 그 자체가 산출물이고, 특히 실패 전력이 있는 영역에서는 **절반 구현보다 확정된 체크포인트가 낫다**.

#### 4.5.244 남은 대형 항목 3건 착수 판단표 — 비용·payoff·선행조건 실측 (2026-07-28, branch feat-remaining-assessment, format 25 불변) ✅

**착수 근거**: §4.5.229~243 으로 소형·중형 잔여가 소진돼 남은 것이 전부 대형 항목이 됐다. 셋 중 무엇을 먼저 할지가 **다음 착수의 실제 병목**이므로, 추측 대신 **실측해서 판단표를 만들었다**(§4.5.233/240 이 보여준 대로, 잘못된 크기 추정은 다음 반복을 오도한다).

**핵심 실측 3건**:
- **A(파일위치 함수군)**: `$ftell`/`$fseek`/`$rewind` 는 **format bump 확정**이다. §4.5.228 의 "동결 enum 에 변종 더하기 전에 사이드카를 보라"를 적용해 봤으나 **여기엔 안 통한다** — `$fmonitor` 는 `$monitor` 의 destination 변종이라 기존 id 재사용이 가능했지만, `$ftell`/`$rewind` 는 **의미가 겹치는 기존 id 가 없다**. 반면 `$feof`/`$fgetc`/`$ungetc` 는 **이미 `SysFuncId` 가 있으므로** 그 범위만 여는 것은 bump 없이 가능하다(부분 착수 경로 발견).
- **B(literal 공유 크레이트)**: **payoff 가 예상보다 작다**. 현재 파서가 거부하는 형태는 절단 리터럴과 unsized+`s` 인데, **절단 형태는 iverilog 도 거부**한다 → 능력 이득이 거의 없고 얻는 것은 *두-술어 위험의 구조적 제거*뿐. 게다가 단순 이동은 **hdl-parser 가 sim-ir 을 보게 되는 레이어링 역전**이라, 올바른 분해는 digit→bits(중립)/ConstVal 패킹(IR) 2단이다.
- **C(inner NET shadow)**: 셋 중 **유일한 ①-급 silent-wrong**이고 오라클도 있다 → §1 우선순위 룰상 **1순위**. 선행(order-independent AST-gathered name set)과 실패 전력(§4.5.218 S1: 중첩 generate body 조용히 삭제)을 함께 기록.

**권장 순서 = C > A > B**를 근거와 함께 §0-C 표로 남겼다.

**게이트**: 4675 tests green(변경 없음) · **코드 변경 0**.

**교훈**: **"다음에 뭘 할지"가 병목이 되면, 그 판단 자체가 슬라이스다.** 단 판단은 **실측**이어야 한다 — 사이드카 우회 가능성(A)과 실제 능력 이득(B)은 코드를 열어보기 전엔 둘 다 반대로 추정하고 있었다.

#### 4.5.243 generate case 의 real scrutinee = 비목표 확정 (2026-07-28, branch feat-gencase-nongoal, format 25 불변) ✅

**착수 근거**: §4.5.242 가 "real→정수 문맥" 을 닫으면서 유일하게 남겼던 항목. §4.5.241/242 가 generate 스코프 선언과 if/for **조건**을 real 도메인으로 라우팅했으니 case scrutinee 도 따라가야 하는가 — 라는 공정한 질문이었다.

**답 = 아니다.** `generate case (R)` 는 **iverilog 도 거부**한다("Cannot evaluate genvar case expression: R"). 오라클이 같은 소스를 거부하므로 **수렴할 대상이 없고**, vita 의 loud 가 정직하다. 갭이 아니라 **비목표**.

**전달물**: 비목표를 핀(신규 `generate_case_real_nongoal.rs`×2) — real 형태가 loud 라는 것과 **정수 scrutinee 는 동작한다**는 것을 **함께** 박았다. 후자가 없으면 real 쪽 loud 가 나중에 조용히 넓어져 정수형까지 삼켜도 아무도 모른다.

**게이트**: 4673 → **4675** tests · clippy/fmt clean · format 25 불변 · **코드 변경 0**.

**이로써 "real→정수 문맥" 가족이 완전히 닫혔다**: §4.5.232(파라미터 바인딩) · §4.5.241(generate 스코프 선언) · §4.5.242(generate if/for 조건) · §4.5.243(case scrutinee = 비목표).

**교훈**: **"형제 경로도 따라가야 하는가"는 오라클에게 먼저 물어라.** 일관성 논증만으로 확장하면 오라클조차 거부하는 형태를 지원하게 되고, 그건 검증 불가 영역을 자발적으로 늘리는 것이다. 그리고 **비목표를 핀할 때는 반대편(동작하는 형태)도 같이 핀하라** — 그래야 loud 가 나중에 조용히 번지지 않는다.

#### 4.5.242 generate 제어식의 real 도메인 라우팅 — "real→정수 문맥" 항목 완결 (2026-07-28, branch feat-generate-real-control, format 25 불변) ✅

**착수 근거**: §4.5.241 이 남긴 나머지 절반. generate 제어식이 정수 도메인으로만 접혀서 `generate if (R/2 > 2)` 와 `for (i = 0; i < R/2; …)` 가 **"is not a constant" loud** 였다(iverilog: then · 3회 반복).

**수정 = 문맥 경계 하나**: 신규 `const_truth_in_scope` — 정수 도메인 먼저, 거부되고 **expr 이 real 을 언급하면** real 도메인에서 접고 **거기서** 진리값을 취한다. if-cond·for-cond 두 site 가 이것을 공유한다. `R/2 > 2` 는 real 비교이고 **1비트 결과만** 정수 세계로 넘어오는 게 핵심 — `R` 을 먼저 정수로 만들면 2.5 가 아니라 2 로 분기를 정하게 되고, 그게 이 도메인이 존재하는 이유인 leaf 변환 실수다.

**분수부가 실제로 분기를 정한다는 것**을 오라클로 핀했다: `R>2` 가 2.5→then · 2.0→else · 1.9→else(전부 iverilog 일치). for 바운드도 `i < R/2`(=2.5) 가 0/1/2 세 번 — 절단됐다면 두 번이다.

**비대상 불변**: 정수 제어식은 정수 도메인이 **먼저** 시도되므로 그대로이고, 진짜 비상수(`if (v)`, v=net)는 여전히 loud — real fallback 이 "못 접는다"를 추측으로 바꾸지 않는다(테스트로 핀).

**게이트**: 4669 → **4673** tests(신규 `generate_real_control.rs`×4) · clippy/fmt clean · format 25 불변.

**§4.5.232→241→242 로 "real→정수 문맥" 항목이 닫혔다**: 파라미터 바인딩(§4.5.232) · generate 스코프 선언(§4.5.241) · generate 제어식(§4.5.242). 셋 다 근인은 같았다 — **`param_real_value` 라는 퍼널은 처음부터 있었고, 호출부가 하나씩 그것을 안 부르고 있었다.**

#### 4.5.241 generate 스코프 `localparam real` loud→correct-support (2026-07-28, branch feat-generate-real-param, format 25 불변) ✅

**착수 근거**: §4.5.232 리뷰가 남긴 "real→정수 문맥" 항목의 **선행 인프라 중 실제로 분리 가능한 조각**. 큐가 "§11.8.1 순서를 공통 퍼널로 먼저"라고 했는데, 그 퍼널은 이미 `param_real_value` 로 존재했고 **generate 스코프만 그것을 안 부르고 있었다**.

**결함**: `generate.rs` 의 Param arm 이 `const_eval_in_scope`(정수 도메인) **하나만** 썼다. real 은 None 이므로 `localparam real X = 2.5;` 조차 loud 였고, 이름을 읽는 곳마다 "undeclared net/variable" 2차 오류까지 났다. iverilog 는 `X=2.50`.

**수정**: 모듈 스코프와 **동일하게** `param_real_value` 를 먼저 호출 — §11.8.1 순서(피연산자에 real 있으면 real 도메인)와 i64-twin 규칙(초기화식이 전부 정수일 때만 twin)이 **두 스코프에서 같은 규칙 하나**가 된다. generate phase 마다 재실행돼도 idempotent(정수 arm 과 동일).

**전달 범위**(전부 iverilog 일치): 리터럴(`2.5`)·외부 real param 식(`R/2`)·순수 real 산술(`1.5+1.0`)·**genvar 의존 real**(`1.5*(i+1)` → 인스턴스별 1.50/3.00, 같은 블록의 정수 localparam 과 공존). **규칙 동일성도 핀**: 정수형 초기화(`real W = 4`)는 generate 안에서도 `logic [W-1:0]` 로 쓸 수 있고, 비정수(`2.5`)는 정수 문맥에서 여전히 loud.

**게이트**: 4666 → **4669** tests(신규 `generate_real_param.rs`×3) · clippy/fmt clean · format 25 불변.

**교훈**: **"공통 퍼널을 먼저 만들어야 한다"고 적힌 선행조건이, 알고 보니 퍼널은 이미 있고 한 호출부만 안 쓰는 경우가 있다.** 인프라 착수 전에 **그 인프라가 이미 존재하는지, 누가 안 부르는지**부터 세라 — §4.5.227 의 "제약이 머신러리 부재로 보이면 대개 가정이다"가 스코프 축에서 반복됐다.

#### 4.5.240 `$value$plusargs` 크기 추정 정정 + if-condition 배치 핀 (2026-07-28, branch feat-plusargs-record, format 25 불변) ✅

**착수 근거**: §4.5.237 이 §3 에 "소형 loud→supported 후보"로 적어둔 항목을 실제로 집어들었다.

**정정 — 소형이 아니었고, 애초에 갭도 거의 아니었다**: 관용적 배치 **둘 다 이미 동작**한다 — `ok = $value$plusargs(…)`(기존 핀)와 **`if ($value$plusargs(…))`**(iverilog 일치 확인, 이번에 핀). 남은 loud 는 `$display("%0d", $value$plusargs(…))` 같은 임의 expression 위치뿐인데, 이는 **side-effect sysfunc 패밀리 전체의 의도된 설계**다 — seeded `$random`·`$fopen`·`$sformatf`·fd-advancing 파일읽기(`$fgetc`/`$ungetc`/`$fgets`/`$fread`/`$fscanf`/`$sscanf`)가 전부 **single-eval 보장을 위해 statement-form 으로 lower** 된다(ENGINEERING_RULES "side-effect sysfunc expr=statement-form desugar"). 임의 expression 위치엔 desugar 할 statement 가 없으므로 **loud 가 정답**이고, 넓히려면 패밀리 전체의 desugar 확장이 선행이다.

**전달물**: if-condition 배치 핀(관용 배치 중 유일하게 테스트가 없던 것) + §3 항목을 "소형 후보" → "패밀리 설계·소형 아님"으로 정정.

**게이트**: 4665 → **4666** tests · clippy/fmt clean · format 25 불변 · **코드 변경 0**.

**교훈**: **내가 큐에 적은 크기 추정도 다음 반복을 오도한다.** §4.5.233 이 "근인이 한 줄이어도 그 줄이 사는 레이어가 크기를 정한다"였다면, 이번은 그 자매편 — **"loud 하다"를 갭으로 적기 전에 그 loud 가 어느 패밀리의 규칙인지 보라**. 여기선 실제 갭이 거의 없었고, 남은 것은 고칠 대상이 아니라 지켜야 할 불변식이었다.

#### 4.5.239 스캔 4차 완료 — `$typename` 핀 + `%u`/`%z`/`%l` 정리 (2026-07-28, branch feat-typename-pins, format 25 불변) ✅

**스캔 마무리**: §4.5.237 이 남긴 후보를 소진했다.
- **`$typename`** — iverilog 13.0 은 **미구현**("not defined by any module")인데 vita 는 동작하고 정확하다: atom 9종(`logic/int/integer/byte/shortint/longint/real/time/string`)·packed 벡터(`logic[7:0]`·`bit[3:0]`)·**unpacked 배열은 IEEE `$[lo:hi]` 표기**(`logic[7:0]$[0:2]`). 테스트 0건이었다 → **핀**(신규 `typename_pins.rs`×3).
- **`%u`/`%z`** — iverilog 는 raw 바이너리 바이트를 뱉고(텍스트 로그에선 깨진 문자) vita 는 무출력. 둘 다 **문서화된 선택**이며 silent-wrong 아님. `%l` 은 vita 가 리터럴 통과, iverilog 는 `<%l>`+warn — cosmetic.

**잔여 기록(ROADMAP §3·무오라클)**: `$typename` 의 **enum·packed struct** 가 base 타입으로 렌더된다(`logic[1:0]`·`logic[3:0]`; IEEE 는 `enum{...}`·`struct packed{...}`). **타입 이름 렌더링 단순화**일 뿐 값·다른 사용처엔 영향 없음. 무오라클이라 추측 대신 **현행을 핀**해 가시화했다.

**게이트**: 4662 → **4665** tests · clippy/fmt clean · format 25 불변 · **코드 변경 0**.

**스캔 전략 4회 총평**: 1회차 `%p` **실제 결함 1건**(real→정수 반올림) · 2회차 `$sformat`/plusargs clean → 핀 · 3회차 파일위치군 honest-loud → §3 · 4회차 `$typename` clean → 핀. **결함 1 / 무오라클 능력 핀 3 / loud→supported 후보 1** — "안 보던 곳"을 기계적 신호로 소진하는 전략의 실제 수율이다.

#### 4.5.238 스캔 3차(파일위치군·`$sscanf`) = honest-loud 확인 + LOOPROMPT §8 압축 (2026-07-28, branch feat-loop-compress, format 25 불변) ✅

**스캔 결과**: 파일 위치 함수군과 `$sscanf` 는 **결함이 아니다** — `$ftell`/`$sscanf` 는 E3009("unsupported system function in expression"), `$fseek` 는 **W3056 warn+skip**. iverilog 는 전부 동작하므로(`A=6 B=0 C=6 D=0` · `$sscanf`→`2 12 34`) **② loud→supported 후보**로 §3 에 기록. `%p` 처럼 조용히 틀린 것은 없었다. 스캔 3회차 만에 나온 이 결과는 전략의 **정상 분포**다 — 1회차 1건 결함(`%p`), 2·3회차는 clean/honest.

**§8 압축**(상한 20480B 근접 20142B → **18934B**): 규칙은 하나도 지우지 않았다. ① NEXT 큐에 쌓인 **"이전 시도가 왜 실패했는지" 서사**를 제거하고 포인터만 남김(그 서사는 ROADMAP/ARCHIVE 가 정본) ② §4 에 섞여 있던 **코드베이스 규칙 3개**(loud→silent 확대 금지·"모든 site 를 덮는다" 주석은 테스트로 고정·loud gate 우회 경로 전수)를 `ENGINEERING_RULES` 로 이관. 루프 절차만 남기니 1.2KB 가 줄었다.

**게이트**: 4662 tests green(변경 없음) · **코드 변경 0**.

**교훈**: **큐는 현재형만 담는다.** "왜 지난번에 실패했는가"는 큐가 아니라 아카이브의 일이고, 큐에 두면 매 반복 읽히면서 상한만 먹는다. §8 압축의 1순위는 규칙 삭제가 아니라 **서사 제거와 코드베이스 규칙 이관**이다.

#### 4.5.237 테스트 0건 스펙 스캔 2차 — `$sformat`/`$swrite`/plusargs 핀 (2026-07-28, branch feat-untested-spec-pins, format 25 불변) ✅

**착수 근거**: §4.5.236 이 세운 전략(`grep -rl '<spec>' crates/cli/tests/` 가 비면 후보)을 **체계적으로 1회 실행**. 포맷 스펙 13종·시스템 함수 12종을 스캔해 커버리지 0인 것을 뽑았다 — 포맷 `%u`·`%z`·`%l`, 함수 `$typename`·`$value$plusargs`·`$test$plusargs`·`$sformat`·`$swrite`·`$ferror`·`$rewind`·`$fseek`·`$ftell`·`$sscanf`·`$ungetc`·`$feof`.

**결과**: 오라클 있고 실사용 빈도 높은 것부터 검증 — `$sformat`(string 대상·**packed reg 벡터 대상**)·`$swrite`·`$test$plusargs`(hit/miss)·`$value$plusargs`(`%d`/`%s`, 실제 `+arg` 공급) 전부 **iverilog 일치**. §4.5.236 의 `%p` 와 달리 **결함은 없었고**, 위험은 "아무도 안 보고 있었다"는 것뿐이었다 → **핀**(신규 `sformat_plusargs_pins.rs`×3).

**기록한 제약(결함 아님·ROADMAP §3)**: `$value$plusargs` 는 **blocking 대입의 직접 rhs 로만** 지원 — `$display("%0d", $value$plusargs(...))` 는 loud(iverilog 는 허용). `$test$plusargs` 는 제약 없음. plusargs 미공급 시 목적지가 **호출 전 값을 유지**하는 것(IEEE)도 함께 핀.

**게이트**: 4659 → **4662** tests · clippy/fmt clean · format 25 불변 · **코드 변경 0**.

**교훈**: **스캔 전략은 재사용 가능한 자산이다.** §4.5.236 이 우연히 `%p` 를 잡은 게 아니라 "커버리지 0" 이라는 기계적 신호가 잡았고, 같은 신호를 한 번 더 돌리자 12개 후보가 즉시 나왔다. 남은 후보(`$sscanf`·파일 위치 함수군·`$typename`·`%u`/`%z`/`%l`)는 큐에 남긴다.

#### 4.5.236 `%p` 가 real 을 정수로 반올림하던 silent-wrong (2026-07-28, branch feat-fmt-p-real, format 25 불변) ✅

**착수 근거**: §4.5.235 가 남긴 fresh 후보(`%p`·assoc 메서드 등) 스윕. assoc 메서드는 int-key 전 메서드(`size/exists/first/next/last/prev/delete`)가 hand-IEEE 정확했고 기존 테스트도 있었다. **`%p` 는 테스트가 0건**이었고 — iverilog 13.0 은 `%p` 를 아예 미지원(warn + `<%p>`)이라 **차분으로는 영원히 못 잡는 자리** — 실제로 결함이 있었다.

**결함**: `%p` 가 모든 인자를 정수 경로(`fmt_dec`)로 렌더했다. `$display("%p", 2.5)` → **`3`**(반올림, 값 소실). 정수엔 맞지만 real 엔 값이 통째로 사라진다 = silent-wrong.

**수정**: `is_real` 이면 `%g` 형태로 렌더(assignment-pattern 의 real 철자이자 되읽으면 같은 수가 되는 최단형). 정수 경로는 그대로 → 기존 동작 바이트 불변. `2.5`/`-0.125`/`1e+06` 로 정확, `%g` 와 일치.

**잔여(ROADMAP §3 기록·무오라클)**: **string** 은 packed 바이트 값으로(`"hi"`→26729), **unpacked struct** 는 필드 연접 정수로 렌더된다(IEEE 는 `"hi"`·`'{x:7, y:-2}`). 렌더러가 받는 `Value` 에 `is_real` 은 있어도 **string/struct 마커가 없어서** 이 층에서 구분 불가 — 철자 문제가 아니라 **타입 정보가 안 내려오는 문제**라 별도 슬라이스. unpacked 배열/queue/dyn 은 whole-value 표면이 없어 **loud 유지**(정직).

**게이트**: 4656 → **4659** tests(신규 `fmt_p_spec.rs`×3 — real/정수/packed+aggregate) · clippy/fmt clean · format 25 불변.

**교훈**: **오라클이 미지원하는 스펙은 결함이 있어도 영원히 안 보인다.** §4.5.235 가 "무오라클 능력을 핀하라"였다면 이번은 그 뒷면 — **무오라클 스펙은 핀이 없으면 결함조차 발견되지 않는다**. 테스트 0건인 포맷 스펙을 찾는 것 자체가 유효한 탐색 전략이었다.

#### 4.5.235 fresh-area 스윕 = CLEAN + 무오라클 능력 2건 핀 (2026-07-28, branch feat-freshsweep-teeth, format 25 불변) ✅

**착수 근거**: 큐 1번(real→정수 문맥·literal 공유 크레이트)은 **둘 다 선행 인프라**가 필요하고, 2번(inner NET shadow)은 DEEP + 실패 전력(§4.5.218 S1). 그래서 3번 **fresh-area probe 로 신규 ① 발굴**.

**결과 = CLEAN**. 스윕한 영역(iverilog 오라클 대조):
- 배열 질의 `$left/$right/$low/$high/$size/$increment/$dimensions/$bits` — unpacked·**非0-LSB 하강**(`[15:4]`)·**상승**(`[0:7]`) 전부 일치(`$increment` 부호 포함)
- 비트 질의 `$countones/$onehot/$onehot0/$isunknown/$countbits`·`$clog2(0|1)`·`do…while`·sized 패턴 `case` — 전부 일치
- `$sformatf` 4포맷·`$signed`/`$unsigned` 왕복 — 일치

**vita 가 iverilog 보다 앞선 2건(무오라클)** — iverilog 13.0 이 **구문 자체를 거부**한다: modport 타입 포트(`sub(ib.mp p)`) · **함수 결과의 part-select**(`f(0)[7:0]`). 둘 다 hand-IEEE 로 값이 정확한데 **핀이 없었다** — 오라클이 못 보는 능력은 리팩터 한 번에 조용히 사라진다. 그래서 이번 전달물은 **teeth**(신규 `fresh_sweep_pins.rs`×4). 스트리밍 연산자(`{<<8{x}}`)는 양쪽 다 거부라 vita 의 loud 가 정직하고 핀할 것이 없다.

**게이트**: 4652 → **4656** tests · clippy/fmt clean · format 25 불변 · **코드 변경 0**.

**교훈**: **스윕이 clean 이어도 산출물은 있다** — 오라클이 거부하는 영역에서 vita 가 앞서 있으면 그 능력은 **오라클로 회귀를 감지할 수 없으므로** 테스트가 유일한 방어선이다. clean 스윕의 결론은 "할 일 없음"이 아니라 "핀 없는 무오라클 능력을 찾아 핀하라".

#### 4.5.234 sized-literal enum label — enum 메서드 전부 loud→correct-support (2026-07-28, branch feat-enum-sized-label, format 25 불변) ✅

**착수 근거**: §4.5.233 이 defer 한 항목(§1 "연속 defer 면 다음 반복은 그 항목 전념"). 근인은 이미 확정돼 있었다 — `hdl-parser::const_lit` 이 unsized **decimal** 만 접어서 sized 라벨 하나면 `foldable=false` → enum 이 `enum_defs` 에 미등록 → 파서의 `.name()` 케이스 함수 합성 자체가 안 일어나고 hier-call fallback 으로 떨어진다(진단이 "hierarchical function call" 로 오도된 이유).

**선택**: §4.5.233 이 적어둔 두 안 중 ②(파서에 based-literal 폴드 추가). ①(literal.rs 공유 크레이트 분리)은 hdl-parser 가 `sim_ir` 을 보게 되는 레이어링 변경이라 더 큰 슬라이스. **`const_lit` 자체는 건드리지 않고** 신규 `const_lit_based` 를 만들어 **enum 라벨만 opt-in** — `const_lit` 은 packed-struct 멤버 레이아웃·typedef 범위도 결정하므로 전역 확장은 파스타임 레이아웃을 통째로 움직인다.

**②의 위험(값 술어가 둘)을 "합의 가능한 부분집합"으로 봉인했다.** 자체 프로브가 실제 발산을 찾았다 — `'h1FFFFFFFF`(unsized) 를 elaborate 는 33비트로 키우고 파서는 32비트로 마스킹해서, `.name()` 표와 상수가 **다른 라벨**을 가리키고 **이름만 빈 문자열**이 됐다(iverilog 도 우연히 같았지만 우연에 기대는 건 취약). → **절단이 일어나면 아예 거부**(`masked != acc` → None). 이제 파서가 받아들이는 값은 두 구현이 **반드시 일치하는 것들만**이고, 나머지는 enum 이 미폴드로 남아 기존 loud 유지. iverilog 도 같은 자리에서 더 엄격하다("Extra digits given for sized constant").

**부수 발굴 — 범위검사의 근거는 폭이 아니라 출처였다**: 라벨을 접기 시작하자 `64'sh7FFF…` 다음의 **자동증가 wrap**(i64::MIN)이 unsigned 64비트 base 범위검사에 걸려 **false-loud** 가 됐다(iverilog 는 수용). 그런데 명시적 `-1` 은 loud 여야 한다(선행 soundness 리뷰가 핀). → 검사 조건을 `w < 64 || explicit` 로. 폭이 아니라 **명시/자동증가**가 판별자다.

**게이트**: 4646 → **4652** tests(신규 `enum_sized_label.rs`×6 — teeth 는 **내부 차분**: `x.name()` 과 `x` 값을 **함께** 출력해 두 술어의 불일치가 숨지 못하게) · 기존 2건 갱신(sized 라벨 out-of-range 는 이제 iverilog 처럼 loud) · clippy/fmt clean · format 25 불변.

**적대 리뷰가 blocking 2건을 더 잡았고 둘 다 같은 근인 — 부호를 리터럴의 `s` 마커에서 가져온 것**. §6.19 는 라벨을 **base 타입의 값**으로 정의하므로 부호는 **base** 가 정한다: `enum integer{A=32'hDEADBEEF}` 는 −559038737 인데 마커 기준으로는 범위초과 **false-loud**(correct→loud 하강), `enum bit[7:0]{A=8'shFF}` 는 255 인데 표는 −1 로 키잉돼 **이름만 빈 문자열**. → 폴드를 **W비트 패턴 + 폭**으로 바꾸고 **호출부에서 `enum_signed` 로 해석**. 리뷰가 나열한 valid-but-rejected 9형 전부 iverilog 일치. 추가로 **unsized + `s`** 는 폭 규칙이 elaborate(`natural.max(32)`)와 달라 거부(`'sd2147483648`) — 절단 규칙으로는 안 잡히는 축.

**교훈**: **두 술어가 불가피하면 "합의 가능한 부분집합"으로 좁혀라** — 두 구현의 규칙이 일치하길 바라는 대신, **불일치가 가능한 입력을 거부**하면 남는 것은 정의상 안전하다. 그리고 teeth 는 두 술어의 결과를 **한 줄에 같이 찍는 것**이다.

#### 4.5.232 `real` const-fold — `localparam real R = 2.0+3.0` loud→correct-support (2026-07-27, branch feat-real-constfold, format 25 불변) ✅

**착수 근거**: §0 T2 승격 큐 최상단(오라클 ✓). `localparam real` 은 흔한 관용구인데 **실수 산술이 통째로 E3009** 였다 — vita 런타임은 같은 식을 정확히 계산하는데도.

**근인**: `param_real_value` 가 실수 **리터럴**만 접거나, 선언이 `real` 이면 **정수** 상수 도메인으로 폴백했다. 실수 **산술**은 둘 다 도달 못 해 loud. → 신규 `const_real.rs::const_eval_real_in_scope`(f64 일관: `+ - * / **`·비교/논리(0.0/1.0)·ternary·정수 피연산자 승격(§11.8.1)·`real_param_val` 조회), 0 나눗셈/비유한은 None=loud.

**순서가 핵심이었다**: `expr_mentions_real` 이면 **실수 폴드를 정수 폴드보다 먼저**. §11.8.1(피연산자에 실수가 하나라도 있으면 실수 도메인)이며, 순서를 틀리면 `localparam real HALF = CLK/2`(CLK=5.0)가 **정수 도메인에 잡혀 2.00** 이 된다 — 슬라이스 도중 자체 프로브가 잡았다.

**적대 리뷰가 5건의 blocking silent-wrong 을 잡았고, 근인은 전부 하나 — 내가 중간에 넣은 "정확히 정수인 real 리터럴에 i64 twin 등록"** 이었다(`real R = 4` 는 정수 경로로 twin 이 생기는데 `real R = 4.0` 은 안 생기는 **철자 비대칭**을 없애려던 확장). twin 이 생기는 순간 **정수 상수 도메인이 real 을 언급한 식에 성공**하고, §11.8.1 순서 규칙을 적용하는 곳은 `param_real_value` **하나뿐**이라 나머지 네 곳이 전부 절단했다 — generate-if 가 **반대 분기**(`R/2>2`, R=5.0 → ELSE) · generate 스코프 `localparam real X=R/2` → **2.0** · 정수형 localparam · 실수 도메인이 거부한 식의 정수 재시도. **twin 을 도로 뺐다**: 핵심 성과(실수 산술 폴드)는 twin 과 무관하므로 100% 유지되고, 5건은 전부 PRE 의 loud 로 복귀했다(그중 ternary 조건 건은 오히려 **정답 9.5** 가 됐다).

**전달 범위**: 실수 산술·실수→실수 alias·실수 체인·정수 피연산자 승격(`10/4.0`=2.5)·`3/2`는 정수나눗셈 1.0 유지. **의도적 loud 유지**: 실수를 정수 문맥에(폭/`$clog2`/replication/정수 localparam), `1.0/0.0`, `<<` 등 실수 미정의 연산자, 실수 override.

**게이트**: 4642 → **4646** tests(신규 `real_const_fold.rs`×4 + `real_params.rs` 4건 재작성) · clippy/fmt clean · format 25 불변 · 3-way: **의도한 6형은 전부 iverilog MATCH**, 나머지 **16형은 PRE==POST 바이트 동일**.

**교훈**: **철자 비대칭을 없애려는 확장이 다섯 곳의 silent-wrong 을 열 수 있다.** 규칙(§11.8.1 순서)을 적용하는 site 가 하나뿐인데 그 규칙의 **전제를 무너뜨리는 능력**(twin)을 전역에 풀면, 규칙을 모르는 모든 consumer 가 조용히 틀린다. 확장 전에 "이 능력을 소비하는 site 가 몇 개이고 각자 이 규칙을 아는가"를 세라.

#### 4.5.231 모듈 스코프 상수식 = 비목표 판정 + 자기일관성 teeth (2026-07-27, branch feat-constexpr-selfconsistency, format 25 불변) ✅

**착수 근거**: §4.5.230 이 상수함수 인터프리터를 폭 인식으로 만든 뒤 남긴 큐 최상단 — "같은 규칙을 `const_eval_in_scope`(모듈 스코프)에도". 큐에 **착수 전에 spec-correct 타깃부터 정하라**고 적어둔 그대로 그라운딩부터 했고, 그 결과 **항목이 해소됐다**.

**측정 결과 = iverilog 13.0 의 untyped-param 접기가 세 갈래로 자기모순**:

| # | 증거 | iverilog |
|---|---|---|
| ① | `(8'd200+8'd100)>>2` vs `((8'd200+8'd100)>>2)+0` | **11** vs **75** — 같은 shift 를 `+0` 으로 감쌌더니 값이 바뀐다 |
| ② | `32'd100000*32'd100000` vs `32'd1<<32'd33` | **1e10**(무제한) vs **0**(32비트 wrap) — 연산자마다 도메인이 다르다 |
| ③ | `4'd15+4'd15`=30 · `8'd200+8'd100`=300 (좁은 피연산자 미절단)인데 `(4'd15+4'd15)>>1`=**7**(4비트 절단) | 같은 자리에서 폭 답이 갈린다 |

**어떤 단일 폭 모델도 이 답들을 전부 재현하지 못한다** = 오라클 부재. vita 는 **한 도메인을 균일 적용**해 자기일관적이므로(위 7케이스 전부 일관), §4 룰("iverilog INCONSISTENT 면 vita spec-correct 타깃")에 따라 **현행 유지**가 정답이다. 예전에 §2 에 "상수 폭 잔차 ①/③"으로 적어둔 것은 **vita 결함이 아니었다** — 재분류.

**전달물 = teeth**. 오라클이 없으므로 **vita 대 vita 항등식**을 회귀로 박았다(`const_expr_self_consistency.rs`×2): ① 값 보존 래퍼(`+0`·`*1`·괄호·이중부정)가 상수식 값을 바꾸면 안 된다 ② `*` 와 `<<` 가 "문맥이 32비트보다 넓은가"에 **서로 다른 답**을 내면 안 된다. 두 성질은 **나중에 폭 인식 도메인을 넣어도 여전히 참이어야** 하므로 모델을 바꿔도 살아남는다.

**잔여 실질 갭 1건**: select 바운드의 `v[7:((32'd1<<32'd33)>>32'd30)]` 는 §4.5.229 가드가 접기를 거부해 조용한 1비트를 남긴다(iverilog `ef`). 가드의 **의도적 decline** 이라 새 결함은 아니나 loud 가 더 정직할 자리로 §2 에 1줄 기록.

**게이트**: 4640 → **4642** tests · clippy/fmt clean · format 25 불변 · **코드 변경 0**(판정+teeth+문서만).

**교훈**: "오라클과 다르다"를 결함으로 접수하기 전에 **오라클의 자기일관성을 측정**하라 — 한 모델로 오라클의 답 전부를 재현할 수 없으면 그건 우리 갭이 아니다. 그리고 오라클이 없을 때의 teeth 는 **자기 대 자기 항등식**이다.

#### 4.5.230 상수함수 인터프리터 폭 인식 — `localparam W=f()` 의 조용히 틀린 파라미터 값 (2026-07-27, branch feat-constfn-width, format 25 불변) ✅

**착수 근거**: §4.5.229 가 남긴 "폭 인식 상수 접기" 3건 중 **도달성이 가장 높은 것**(ROADMAP §2 잔차 ②). 인터프리터가 모든 본문 식을 폭 무제한 i64 로 계산해 **좁은 대입 대상이 절대 잘리지 않았다**.

**결정적 신호는 오라클이 아니라 내부 불일치였다** — vita 의 **런타임은 같은 함수를 이미 정확히 실행**하고 있었다. 9가지 형태 중 **6개에서 인터프리터가 자기 엔진과 달랐고**, iverilog 는 런타임 편이었다:

| 본문 | iverilog = vita 런타임 | vita 인터프리터(PRE) |
|---|---|---|
| `bit [3:0] t = 4'd15+4'd15` | 14 | 30 |
| `byte t = (8'd200+8'd100)>>2` | 11 | 75 |
| `bit [3:0] t = 4'd8*4'd3` | 8 | 24 |
| `byte t = 8'sd100+8'sd100` | −56 | 200 |
| `bit [3:0] t = 20` | 4 | 20 |
| 좁은 formal `t = a+a` | 14 | 30 |

**구현(IEEE §11.6 / Table 11-21)**: 대입은 RHS 를 `max(self-width(RHS), 대상 폭)` 에서 계산하고 대상으로 coerce · 문맥결정 연산자는 그 폭에서 마스킹 · **부호는 문맥 루트에서 한 번 정해 아래로 전파**(§11.8.1) · **자기결정 위치는 스스로 크기를 정한다**(shift COUNT·`**` 지수·ternary 조건·비교 피연산자[서로 폭·부호 통일]·`$clog2` 인자·`!` 피연산자·전 statement 조건/`repeat` 카운트) · 폭을 알면 shift 는 **비트패턴**으로 계산(음수의 논리 `>>` 가 더는 거부되지 않음). 폭 env `(name→(width,signed))` 를 formal·body decl·**중첩 블록**·`for` init·함수명 반환변수·**중첩 호출**까지 전부 관통.

**적대 리뷰 2라운드가 9건을 잡았다**(전부 수정 후 재리뷰). 특히:
- **부호를 노드마다 다시 계산한 것**이 최악이었다 — 부호 있는 하위식이 부호 없는 부모 밑에서 sign-extend 돼 `bit[7:0] r=(b+b)/u` 가 **정답 100 → 228** 로 하강했다(§11.8.1 은 문맥당 **한 번**).
- 폭 63 대상에서 `(1i64<<63)-1` **오버플로 패닉**(공유 `coerce_int_width`, 릴리스면 wrap).
- `bit [f()-1:0]` 이 `const_eval_in_scope` 로 재진입하며 호출 깊이를 0 으로 리셋 → **스택 오버플로**(PRE 는 깨끗). 깊이 캡이 아니라 **호출을 포함한 바운드는 접지 않는다**로 구조적 제거.
- `return e` 가 폭 규칙을 우회해 `f = e` 와 **같은 식이 다른 값**(15 vs 7).
- 접기를 넓히면 **폭·부호 술어도 같이** — `int unsigned` 강제 signed, multi-packed 를 첫 dim 으로 마스킹, `Call` 의 반환 타입을 `const_expr_signed`/`param_decl_width` 가 모름(−56 → 4294967240).
- **3라운드에서 2건 더**: ① 좁은 **signed** 리프가 **unsigned** 문맥에 들어갈 때 자기 폭에서 zero-extend 돼야 하는데(§11.6.1) i64 에 이미 sign-extend 돼 있어 비교가 뒤집혔다(`(a*8'sd1)>LIMIT` 0→1) ② `const_decl_wsign` 이 **거부**한 선언을 읽는 쪽이 `(32, unsigned)` 로 **추측**해 64비트 multi-packed 를 조용히 잘랐다 — 거부는 "모름"이어야 하고 모름은 **마스킹 안 함**으로 전파돼야 한다.

**게이트**: 4629 → **4640** tests(신규 `const_fn_width.rs`×11) · clippy/fmt clean · **format_version 25 불변** · 3-way 스윕 **11 대상타입 × 15 RHS = 165 포인트 전부 iverilog MATCH**(PRE 는 **84 불일치**).

**교훈**: 오라클이 없거나 모호할 때 **자기 엔진이 오라클**이다 — 인터프리터 대 런타임 차분이 이 슬라이스 전체를 이끌었다. 그리고 **부호는 값·폭과 함께 움직이는 세 번째 술어**다(§4.5.229 의 교훈이 `Cast` 에서 `Call` 로 그대로 반복됐다).

#### 4.5.229 상수식 BOUND/COUNT 단일 퍼널 — part-select 폭·replication count·indexed part 폭의 silent-wrong 8가족 (2026-07-27, branch feat-constfold-bounds, format 25 불변) ✅

**착수 근거**: ROADMAP §1 NEXT 2번(§2 오라클-有 silent-wrong "part-select 바운드 silent-0 + replication count silent-0, 동근이니 한 슬라이스로"). 선택 전 재현에서 **기록보다 훨씬 넓었다** — 기록은 2가족이었으나 실측은 **8가족**(전부 exit 0·진단 0건).

**근인 = 하나**. 선택 BOUND 와 replication/part COUNT 는 IEEE 상수식(§11.4.12.2·§11.5.1)인데, vita 는 파라미터에 쓰는 정본 상수 도메인(`const_eval_in_scope`) 대신 **훨씬 약한 두 folder** 를 썼다 — 리터럴 전용 자유함수 `const_eval_u32`(IntLit/Paren/unary±) 와 엔진의 얕은 `const_u32_of_expr`(Const·폭 트리의 Add/Sub·Const 인자 `$clog2`). 나머지 상수형(`* / % ** << >>`·ternary·cast·상수함수 호출·`$clog2(식)`·`$bits(x)/k`·`pkg::X*k`)은 전부 **조용히 열화**했다:

| 가족 | 예 | iverilog | PRE |
|---|---|---|---|
| part-select READ 폭 | `v[int'(11):int'(8)]` | `c` | `0` (폭 1) |
| part-select WRITE 폭 | `w[int'(11):int'(8)]=0` | `f0ff` | `000f` (lsb 위 전부 clobber) |
| ascending net | `va[int'(4):int'(7)]` | `c` | `0` |
| md-packed outer | `mp[int'(2):int'(1)]` | `bbcc` | `0` |
| md-packed leaf | `mp[2][int'(7):int'(4)]` | `b` | `1` |
| 배열 원소 | `mem[1][int'(7):int'(4)]` | `f` | `1` |
| indexed part 폭(R/W) | `v[8+:int'(8)]` | `be` | `0` |
| replication count | `{P*2{1'b1}}` 외 10형 | `1111` | 빈 결과 |

**구현**: 단일 퍼널 `const_bound_u32` — 리터럴 folder 를 **먼저**(음수 리터럴의 의도적 `wrapping_neg` 포함, 기존 shape 전부 바이트 동일) 시도하고 실패할 때만 강한 도메인. 폭 사이트는 `lower_const_width_expr`(이미 reducible 이면 그 노드 verbatim). `const_eval_in_scope` 에 **`Cast` arm** 신설(`cast_prim_wsign` 공유 테이블 + `coerce_int_width`; `Size(N)` 은 부호 무관이 증명될 때만; `Signing`/`Named` 은 loud 유지).

**적대 리뷰가 6건을 잡았다(전부 수정 후 재리뷰)**. 초판 가드는 "리프가 전부 ≥32비트"였는데 **두 방향으로 틀렸다**:
- `x[-1:0]` → `0xFFFF_FFFF-0+1` **u32 오버플로 패닉**(release 면 0폭). → `folded_part_width` u64 checked + `MAX_NET_WIDTH` 상한.
- `(32'd1<<32'd33)>>32'd30` → 리프는 32비트인데 **중간값이 32비트를 넘었다 돌아온다**. SV=0, i64=8. PRE 는 폭 1 = **정답**이었으므로 correct→silent-wrong 하강이었고, `v[7:그것]` 은 **false loud** 까지 냈다.
- `localparam P = int'(-300)` → `const_expr_signed` 에 `Cast` arm 이 없어 **unsigned 로 바인딩**(4294966996). loud→silent-wrong.
- `function byte g8(); g8=(8'd200+8'd100)>>2;` → `Call` 이 width-growing 이 아니라 판정돼 **리프 검사 자체를 건너뛰었다**(75 vs SV 11).
- 반환은 `int` 인데 **로컬이 narrow**(`bit [3:0] t`)면 여전히 발산(30 vs 14) — 인터프리터가 대입을 선언 폭으로 coerce 하지 않는다.

**최종 가드 = 3조건**(공유 traversal `const_fold_children` 하나로 구동): ① 이름이 inline `subst`/`out_subst` 에 묶이면 거부(lowering resolver 와 불일치 방지) ② **모든 하위식 값이 `0..=i32::MAX`**(중간 오버플로를 리프로는 못 잡는다) ③ width-growing 연산이 있으면 리프 전부 ≥32비트이고, **`Call` 은 항상 growing**(시그니처+전 body decl 이 ≥32비트여야 통과).

**교훈**: 폭 정확성 가드를 **리프**로 세우면 중간값 오버플로를 놓치고 max-폭 규칙을 과잉거부한다 — 판정은 **값**(모든 하위식)으로. 그리고 "이 잔차는 기존 Add/Sub 도 갖고 있다"는 정당화는 **측정 없이 쓰면 거짓**이었다: 엔진은 `<<`/`*`/`%` 를 애초에 접지 않아 폭 1 로 떨어졌고 그게 정답이었다.

**게이트**: 4613 → **4627** tests(신규 `const_fold_bounds.rs`×14) · clippy/fmt clean · **format_version 25 불변**(sim-ir·hdl-ast·artifact 무변경) · 3-way(iverilog/PRE/POST) 18 상수형 × 15 문맥 = 270 포인트 **전부 iverilog MATCH**(PRE 는 16형 불일치), 리터럴/괄호 2형은 PRE==POST 바이트 동일.

#### 4.5.228 round-20 8항목: fork-arm 재개·음수 하한(unpacked/packed)·동시 활성화 dyn·generate/interface 스코프·VCD 선언범위·`$fmonitor`/`$fstrobe` (2026-07-27, branch feat-round20, format 23→25) ✅

**착수 근거**: "못 고친 3가지"의 선행을 실측으로 재구성한 결과, ROADMAP 이 적어둔 전제 2개가 **틀렸다**.
① fork 건은 "깊은 스케줄러 rework"가 아니라 **bb 번호공간 충돌 한 곳**이었고(task CFG 에 dead `if` 9개를 넣어 resume bb 를 밀면 같은 설계가 통과 — 이 판별 실험이 근인을 지목),
② 음수 하한 건은 "E4002 로 loud"가 아니라 순수 `foreach` 형태에서 **silent**(iverilog 3 / vita 2, 진단 0건)였다.
추가로 **미기재 선행 1건**을 발굴 — generate 스코프 *프로세스 안*의 block-local 이 E3010(모듈 스코프·generate 스코프 선언은 정상 = 순수 스코프 비대칭).

**1. fork arm 이 부른 suspendable task 미재개 (🔴 silent-wrong)** — in-frame child-완료 intercept 가 프레임 `bb`(전역 `ir.blocks`)를 barrier `join_bb`(top-level fork 에선 **프로세스-로컬**)와 비교. 충돌하면 자식을 완료 처리해 죽이고 `exit_arm_frame` 이 arm 아닌 window 를 헐었다. `FrameRec::is_arm` 으로 두 공간을 절대 비교하지 않게. `join_none`+`wait fork`·`join_any` 생존 arm 도 동반 수정. 중첩 프레임·output copy-out·활성화별 격리는 **이미 동작**하고 있었다.

**2. 동시 활성화 frame-local dyn 배열 (F4004 해제)** — net-keyed 슬롯의 entry-stash 는 구간이 **nest** 할 때만 건전. fork 는 overlap 시킨다. 새 머신러리가 아니라 **AUTOMATIC window 의 수명을 그대로** 부여: 서스펜드 중 힙에서 park, 재개 시 unpark, 같은 두 지점(`stash_frame_windows`/`restore_frame_windows`). TOP 프레임만 park(바깥 활성화 값은 위 프레임의 `dyn_stash` 에 있다 — 재귀가 그것에 의존). **회귀 1건은 16k 스윕만 잡았다**: 부모가 fork barrier 에서도 park 하는데 parked 배열은 arm 에게 *공유*가 아니라 *부재* → arm 이 부모 `a[0]` 을 x 로 읽음(`FrameRec::forked` 로 해결). entry 가드는 **불변식** 가드로 개명·메시지 정정(더 이상 "동시 활성화 미지원"이 아니다).

**3~6. generate/interface 스코프** — `allow_string_init=false` 는 "그 스코프는 flush 를 안 돈다"고 설명돼 있었으나 flush 는 있었다. 진짜 결함: string/handle 선언이 **선언 시점에** 하는 쓰기(스칼라 t0 init, 라우팅된 배열의 `new[n]` pre-size)가 **bare-name lvalue 로 모듈 스코프 pending 리스트**에 들어가고, 그 리스트는 `cur_prefix` 가 빈 채로 flush 된다 → `t.g[0].s` 대신 `t.s`. 선언 스코프 prefix 로 키잉(`pending_scoped_presize`/`pending_scoped_bl_strings`)하고 각 스코프의 기존 flush 지점에서 drain. 모듈 스코프는 `""` 키·같은 위치 = 바이트 동일. 그 결과 한꺼번에 열림: generate/interface 스칼라 string decl-init(T2-9 양쪽) · string ARRAY decl-init+런타임 인덱스 · generate 스코프 queue/dyn/string-queue `'{…}`(별도 follow-on 이었다) · generate 프로세스 안 block-local(항목 3). **항목 3 이 항목 4 를 드러냈다**(E3010 → 오도하는 `new[n]` 에러; loud→loud 라 하강은 아니지만 "게이트를 걷으면 그 밑이 드러난다" 패턴).

**7~8a. 음수 하한** — unpacked: `array_dim_extents` 가 `clamp_bound_u32` 로 lo 를 0 으로 깎으면서 **dim 자체가 줄었다**(`[-1:1]`→lo 0/size 2). `lo` 를 i64 로(`array_dims`·`net_dim_extents`·`flatten_word` 쌍둥이·`net_dims` 사이드카). packed: plain net 은 warn+clamp-1(whole-value 손상), **multi-packed inner 는 silent**(8비트 vs 12비트). 저장은 정규화 `[w-1:0]`(`NetVar.msb`/`lsb` 동결 u32) + 선언 바운드 사이드맵, **폭과 선택 정규화를 함께 켜는 opt-in**(리터럴 `false` 단락 ⇒ 나머지 호출부 바이트 동일·loud 유지). 부수로 잡힌 pre-existing 3건: `$bits(a[0])` 가 `(lo,SIZE)` 를 `(lo,hi)` 로 읽던 오프바이원 · 엔진 file-I/O base 의 같은 오독(`$readmemh` 비-0 base 레인지 폼) · `string s[i64::MAX:-i64::MAX]` PANIC(크래시는 loud 아래).

**8b~12. bump 배치(24→25)** — VCD `$var` 가 정규화 범위 `[5:0]` 를 찍어 파형 뷰어가 비트를 잘못 라벨(값은 정확). `net_decl_ranges` 사이드카로 선언 범위 전달, staged 경로 확인. `$fmonitor`/`$fstrobe` 는 W3056 skip = 파일 출력 무성 소실 → **동결 `Monitor`/`Strobe` id 재사용**(SysTaskId 변종은 SimIr 해시 flip) + `file_directed_stmts` 사이드카. 술어 하나(`is_file_monitor_strobe`)가 fmt/args 분할과 사이드카 기록을 **둘 다** 몰아 fd 위치에 대해 이견이 생길 수 없게. 모니터는 **destination 별**로 유지 — 공유 슬롯은 `$fmonitor` 가 서 있던 `$monitor` 를 밀어내 stdout 이 조용히 멈췄다(iverilog 는 둘 다 계속 찍는다).

**적대 리뷰 2렌즈**: differential = 스크래치패드 **16,316 설계** PRE vs POST 전수(항목마다 재실행) — 최종 115건 변경, 비-프로브 24건 전수 개별 검증(15건 loud→iverilog 정확 일치, 나머지 loud→loud 또는 iverilog 가 컴파일 못 하는 hand-IEEE). **회귀 1건 검출·수정**(위 §2). soundness = 각 항목별 형상 매트릭스(재귀×동시성, 다차원×방향, 스코프 대칭, drain 순서, fd 유효/무효).

**오라클 결함 2건 추가 기록**(vita 가 IEEE 정답): 같은 fd 에 `$fmonitor` 2회 → iverilog 는 **누적**(자기 싱글턴 `$monitor` 와 모순) · 빈 string **배열 원소**의 `%s` → iverilog 는 공백 1칸(스칼라는 빈 문자열).

**잔여(정직 기록)**: 음수 바운드의 **PART select**(바운드 접기가 unsigned — §2 의 part-select 상수 접기와 **같은 자유 함수**가 근인) · **포트/formal** 음수 바운드(의도적 opt-in 비대칭·warn+clamp 유지) · 인터페이스 queue 원소의 **계층 read**(`u.q[0]`) · generate 내 block-local 이 generate-net 과 이름 충돌하며 **시간상 겹칠 때** loud(모듈 스코프 쌍둥이는 여전히 silent-wrong·pre-existing).

**게이트**: 4613 tests green(+63) · clippy 0 · fmt 0 · 3-OS 미실행(로컬) · format_version **25**(사이드카 2개 append-only, sim-ir 불변 — 골든 무영향, wire-shape fixture 재핀) · 1000줄 정책 복구(4개 파일 분리: `exec/frame_window.rs`·`elaborate/{var_init,iface_inst,limits}.rs`).

#### 4.5.227 §0 T1 잔여 11항목 loud→supported (임의 bounds/방향·multi-dim foreach·중첩 decl-init·bounded string queue·계층 write·계층 assoc·frame-local 재귀·SoA whole-element·format 23 불변) (2026-07-27, branch feat-t1-residual) ✅

**4535→4550 green** · 근인이 **4갈래**였고, 하나의 "string-array 잔여"가 아니었다.

**① GEOMETRY (항목 1·2·3·4·5·7)** — 라우팅된 컨테이너는 원소를 `0..n-1`로 번호매기는데 그건 `string s[1:3]`/`s[3:1]`/`s[2][2]`의 선언 인덱스 공간이 아니다. 1차 슬라이스(§4.5.222/224)가 zero-based ascending만 라우팅한 이유. **해제한 것은 새 머신러리가 아니라 "가정" → "적용"**: `flatten_word`는 이미 dim마다 `idx - lo`를 row-major stride로 정규화하고, `lower_fixed_foreach_step`은 이미 선언 bounds를 선언 방향으로 걷는다. 그래서 선언 시점에 `StrArrayGeom{extents, desc}`를 기록하고 **모든 접근 + foreach가 그걸 조회**하게 했다. zero-based 1-D는 `lo==0`이면 `Sub` 없음·stride 1이면 `Mul` 없음이라 **IR이 바이트 동일**로 보존된다.

**② BOUND (항목 6)** — queue bound는 **엔진에서** net-keyed로 강제되고 원소 타입을 안 본다(`int q[$:1]`은 이미 동작했다). `string q[$:3]`을 막고 있던 건 선언 패턴이 `Queue(None)`만 매치한 것 하나뿐. 둘이 `queue_dim_bound`를 공유하게 하고 정수 쌍둥이를 같은 테스트에 핀으로 박았다.

**③ HIERARCHICAL (항목 7·8·10)** — deferred READ와 deferred WRITE는 **별개 sentinel 공간의 별개 pass**이고, write를 여는 위험은 정확히 "같은 소스 텍스트인데 read와 **다른 원소**를 가리키는 것". 그래서 주소 규칙을 `hier_dyn_container_word` **하나로** 만들고 둘 다 그걸 부른다(routed 배열은 `flatten_word_eids` — 로컬 funnel이 쓰는 `flatten_word`의 pre-lowered-eid 쌍둥이 — 로 같은 산술). assoc도 같은 1-인덱스 철자로 합류: **keyed vs positional 구분은 net이 downstream에서** 정하기 때문(`resolve_lvalue_offsets`가 `is_assoc(net)`이면 같은 word EID를 `AssocKey`로 다시 읽는다).

**④ PER-ACTIVATION (항목 9)** — §4.5.171의 frame-local dyn 배열 fatal. heap slot은 net-keyed로 두고, frame **ENTRY가 바깥 활성화의 내용을 stash로 가져가고** EXIT가 되돌린다. stash는 **활성화와 함께** 이동한다(동기 호출은 지역변수, suspendable은 `FrameRec`) — 전역 스택이면 A가 suspend→B 진입→A 재개·종료 순서에서 LIFO가 깨져 A가 B의 배열을 되돌려 받는다. 구간이 **nest**할 때만 건전하므로 재귀는 지원, **overlap하는 동시 활성화는 fatal 유지**(문구를 실제 이유로 교체).

**⑤ SoA (항목 11)** — non-packable record 컨테이너는 자기 이름의 net이 없어(멤버당 하나) whole-element `o = q[i]`에 표면이 없었다. `pop_front()`/스칼라 whole-copy가 이미 하던 대로 멤버별 fan-out. 리뷰가 `q[i] = d[j]`(element↔element)가 남은 걸 잡아 같이 닫았다.

**과정에서 내가 만든 silent-wrong 4건 — 전부 리뷰가 아니라 측정이 잡았다**:
1. decl-init collector 2개가 여전히 **1개 dim으로** 펼쳐서 중첩 `'{'{…},'{…}}`가 string 원소에 assignment-pattern을 대입 → exit 0에 빈 문자열 4개. 두 collector가 full `unpacked`를 **하나의 공유 확장**에 넘기도록.
2. frame-dyn fatal 제거가 **선행 silent-wrong을 드러냄**(fork 동시 활성화) → loud→silent 하강. nesting 판별자로 그 형태만 loud 복원.
3. 재귀 호출이 **자기 dyn FORMAL을 actual로** 넘기면 이미 비워진 slot을 복사 → `sz=0` + 가짜 OOB warn at exit 0. 순서를 capture → stash → install로.
4. INOUT dyn formal 재귀에서 caller net == formal net이라 in-place copy-out이 restore에 덮여 3/2/1(iverilog 3/3/3). copy-out을 restore **뒤로** 옮김. **적대 리뷰의 soundness 렌즈가 잡았고 differential matrix는 못 잡았다.**

**적대 2렌즈**: differential = 프로브 **168파일 3-way**(iverilog/PRE/POST) — 28건이 이 브랜치로 바뀌었고 **전부 loud→iverilog 일치**, 회귀 0. soundness = routed 원소의 모든 read/write 문맥(12종)·frame 스코프 foreach·bounded queue 전 표면·shadowing·fork-in-recursion·inout/output formal 재귀·cross-type SoA 가드·무오라클 2건(계층 assoc·SoA record)의 vita-internal 등가성.

**오라클 결함 2건 규명(vita가 IEEE 정답·회귀 테스트로 고정)**: iverilog의 string **배열 원소** `.len()`이 **배열 크기**를 낸다(`string s[5]; s[0]="abcdefg"` → 5; 같은 텍스트가 스칼라면 7) · 동시 fork 활성화가 automatic string 배열을 공유한다(`A!` 대신 `A!!`).

**열지 않고 기록한 것**: generate 스코프 라우팅 — `allow_string_init` 플래그만 뒤집는 건 **불충분함을 실측**(`new[n]` pre-size가 그 스코프에서 핸들을 못 찾음)해서 반쯤 열지 않고 되돌린 뒤 ROADMAP §0에 남겼다.

신규 테스트 파일 없음(기존 6파일 확장); loud를 핀으로 박고 있던 테스트 8개를 correct-support 기대값으로 교체.

#### 4.5.226 §0 T1-6: 계층 dynamic-container element READ loud→supported (`u.s[0]`·`u.d[0]`·`u.q[0]`·format 23 불변) (2026-07-27, branch feat-string-array-t1) ✅

**4526→4535 green** · **string 전용 아님**: `resolve_deferred_hier_sel`이 dynamic-storage handle을 전부 거부했는데("dyn element read는 lowering의 1-seg base로만 라우팅된다"), 그건 **lowering 경로**에만 참이고 resolve된 element read는 word-indexed `Signal`일 뿐이라 엔진은 이름을 어떻게 도달했는지 신경쓰지 않는다. `int d[]`·`int q[$]`도 같은 이유로 loud였고 같이 열렸다.

**string 배열만 추가로 필요했던 것**: 라우팅된 배열은 **맹글된 net 이름**(§4.5.222)이라 symbol table로 `u.s`를 못 찾는다 → 로컬 resolver가 쓰는 같은 사이드맵을 **같은 commit-to-scope walk로, symbol table 다음에** 조회(부모의 동명 배열이 자기 저장소를 유지하는 것을 테스트로 고정).

**범위 = 위치가 곧 인덱스인 형태만**: 인덱스 1개 + DynArray/Queue. **assoc는 키드**라 bare index가 다른 연산이고, **multi-index on routed multi-dim**은 이 pass가 안 들고 있는 dims로 row-major flatten을 해야 해서 첫 인덱스를 flat으로 읽으면 조용히 틀린 원소다. 계층 element **WRITE**도 별개 deferred 머신이라 loud 유지(read와 비대칭이나 PRE도 loud → 회귀 아님). OOB 인덱스는 warn + iverilog와 바이트 동일. 신규 `hier_dyn_container_read.rs`×9.

#### 4.5.225 §0 T1-7: task/function body-local string ARRAY loud→supported (frame-entry pre-size·format 23 불변) (2026-07-27, branch feat-string-array-t1) ✅

**4515→4526 green** · frame-local 스칼라 `string`(§4.5.167 slab) 과 frame-local `int a[2]`(md-packed slot)는 이미 동작했으나 **string 컨테이너는 어느 표현에도 안 맞는다**(string은 packed width가 없어 `count*elem_w`가 무의미).

**fix**: §4.5.171이 만든 frame-local `DynArray` heap handle + per-activation lifecycle의 `ast_kind_is_bit_vector` 가드를 **ELEMENT 타입에 한해** 해제(컨테이너는 동일 heap handle·`string_elem_dyn_nets`가 원소를 byte-string으로 만든다). 그 가드는 §4.5.171이 **측정된 회귀** 때문에 넣은 것이라 조심해서 갈랐다 — frame-local 스칼라 string은 unpacked dim이 없어 이 코드에 **도달조차 안 하므로** 혼동 불가(테스트로 고정). FIXED 형태는 `emit_frame_local_inits`가 **frame ENTRY에 `new[n]` pre-size**를 추가로 emit(module-scope 쌍둥이는 t0 var-init flush에서 받지만 frame local은 거기 도달 안 함).

**공짜로 따라온 것**: frame-local net을 module-scope와 **같은 `fixed_string_dyn` set**에 등록 → multi-dim row-major chain walk · `new[]` reject · partial-index reject가 두 번째 구현 없이 적용(두 스코프가 갈릴 수 없음).

**적대 프로브**: 2회 호출 per-activation 격리 · `@(posedge clk)` suspend 생존 · `foreach`/런타임 인덱스 · framed function body · **여전히 loud**=recursion(§4.5.171 per-net heap F4004·iverilog는 돌림=기록된 갭)·fixed에 `new[]`. 신규 `frame_local_string_array.rs`×11.

#### 4.5.224 §0 T1-5: multi-dim fixed string array loud→supported (row-major flat 컨테이너·format 23 불변) (2026-07-27, branch feat-string-array-t1) ✅

**4510→4515 green** · string은 heap handle이라 `int s[2][2]`를 굴리는 md-packed 표현이 안 맞는다. §4.5.222 라우팅 위에 **1개 flat row-major 컨테이너**로 태우고 `s[i][j]`를 접근마다 `s[i*n1+j]`로 flatten(기존 `flatten_word` 재사용).

**양 퍼널이 Ident base로 끝난다** — 1-D면 충분하지만 중첩 select는 절대 Ident가 아니다. 그래서 read(`routed_md_string_elem`)/write(`routed_md_string_lval`) 각각 체인을 걷고, **의도적으로 동일한 조건에서 declaine**한다(read는 flatten하는데 write는 안 하면 `s[i][j]`가 서로 다른 원소에 앉는다).

**PARTIAL 인덱스는 loud**: 2-D의 `s[0]`은 행 전체 선택이라 값 표면이 없고(iverilog도 거부), flat 컨테이너는 **행 번호를 원소 번호로** 받는다 — 실측서 exit 0에 빈 문자열이었다. **양 퍼널에 다 걸었다**(read만 걸면 write가 조용히 남는다). 모든 차원이 zero-based ascending이어야 하고(flat이라 한 축만 어긋나도 조용히 renumber), multi-dim `'{…}` decl-init도 loud.

**row-major 순서는 NON-SQUARE로 고정**: `s[2][3]`이라야 transpose된 flatten이 `s[0][2]`와 `s[1][0]`을 충돌시킨다(정사각은 구분 불가). 3-D 검증. 라우팅 헬퍼를 strings.rs→string_array_route.rs로 이동(1009→934줄, 정책 복귀).

#### 4.5.223 §0 T1-4: `string q[$]` loud→supported + dynamic-container element 단일 퍼널 (format 23 불변) (2026-07-27, branch feat-string-array-t1) ✅

**4497→4510 green** · **차원을 수용하는 게 작업이 아니었다**: 이 브랜치의 첫 시도가 정확히 그것만 하고 **silent-wrong을 냈다** — 컴파일되고 `q.size()`도 맞는데 **원소가 전부 빈 문자열**(iverilog `2 aa bb` → 조용한 `2   `).

**근인**: queue push/insert가 **각자 `.resize(w)`** 를 했고 string handle net은 width 0 → `max(1)` → 바이트 문자열이 1비트로 절단. dyn-ARRAY element write에는 N3 Phase 2부터 이 함정을 설명하는 주석과 함께 분기가 있었으나, queue 경로엔 필요한 적이 없었을 뿐이다.

**fix = 분기를 세 번째로 복사하지 않고 단일 퍼널** `SimState::coerce_dyn_elem` — `dyn_str_elem`(엔진이 애초에 그 원소를 byte-string으로 잡게 만드는 바로 그 플래그) 키잉이라 자기가 변환해 주는 저장소와 불일치 불가. write 3 site(queue push·queue insert·`'{…}` decl-init 확장) + 추출원 dyn-array write가 공유. read는 원래 resize를 안 해서 무수정.

**두 번째 silent-wrong도 같이 있었다**: decl-init collector 2개가 컨테이너 차원으로 게이팅해서 `string q[$] = '{…}`가 **조용히 drop**(int-queue 쌍둥이는 정상 초기화). 두 collector가 이제 술어 하나를 공유.

**계획에 없던 결과 — 측정 후 수용**: non-packable record queue(string 멤버 struct)가 동작하게 됐다. 그런 컨테이너는 **SoA**(멤버당 컨테이너)라 string 멤버가 곧 `string q[$]`이고, string-queue reject가 record queue 전체를 loud로 만들고 있었다. iverilog가 전부 거부하므로 teeth=**내부 등가**(그 멤버가 이 슬라이스가 iverilog로 직접 검증한 저장소). **10형 전수 후 수용** — 전부 correct-or-loud, loud는 whole-element read 하나. SoA 멤버가 push_front/insert/delete에서 어긋나면 잡히는 정렬 테스트도 고정.

**퍼널 회귀 teeth**: `byte q[$]`에 `push_back(300)`=44 유지 · int-queue 표면 바이트 불변 · bounded `[$:N]`은 string도 loud.

#### 4.5.222 §0 T1 부분 — FIXED string array의 RUNTIME 인덱스 + `foreach` loud→supported (zero-based ascending만 라우팅·format 23 불변) (2026-07-26, branch feat-string-array-t1) ✅

**4477→4494 green · format 23 불변 · MsgCode 59 불변**

**갭**: `string s[2]`가 원소 net N개(`s$sae$<i>`)라 **런타임 인덱스가 표현 불가**(인덱스가 net들 중 하나를 골라야 함). `s[i]`·`foreach(s[j])` 全 E3009. iverilog 오라클 有.

**★ 먼저 로드맵의 전제가 틀렸음을 측정으로 확인**: §0 T1은 6종을 "한 가족·머신러리 공유"로 묶었으나, **근인이 4개로 갈린다** — T1-2/3만 라우팅으로 풀리고 T1-4(queue)·T1-5(multi-dim)·T1-6(hier)·T1-7(frame-local)은 각각 별개다. 특히 **T1-6/T1-7은 DYN 배열에서도 똑같이 loud**(`u.s[0]`=dynamic-storage handle의 hier 게이트·frame subset)이라 라우팅과 무관하다.

**★★ 저장-클래스 통합 전에 capability parity를 23종 실측**(`measure-parity-before-unifying-storage` 규칙): decl-init·const idx·byte select·원소 `.len()`/`.getc()`/`.toupper()`/`.substr()`·원소간 복사·함수 인자·ternary·`$sformatf`·`case`·compare·concat·empty read 전부 **fixed ≡ dyn**, 그리고 dyn만 runtime idx·`foreach`·runtime write·`.size()`를 답한다 = **dyn ⊋ fixed**. **fixed가 이기는 항목 0개**라 통합이 사다리 상승으로 정당화됨(§4.5.220 前이었다면 byte select가 119 vs 0이라 silent-wrong 맞바꾸기였을 것 — 전제 슬라이스가 먼저여야 했던 이유).

**구현**: `fixed_string_dim_zero_asc`(신규 `string_array_route.rs`)가 **zero-based ASCENDING만** 통과 → `DynArray` net 1개 + `string_elem_dyn_nets` 마킹 + `fixed_string_dyn`(net→길이) + t0 var-init flush에 `new[n]` pre-size. 초기화 검증(원소 COUNT)은 기존 `fixed_string_array_init_pairs` 재사용, 확장은 기존 두 collector가 담당(스코프별 리스트 유지).

**non-zero-base/descending은 라우팅 제외 — 인덱스 공간이 조용히 renumber되기 때문**: `foreach`는 **선언 인덱스**를 내고 descending은 **역순**이다(iverilog 실측 `int a[1:3]`→1,2,3 / `a[3:1]`→3,2,1·vita도 이미 정확). 0-base dyn으로 보내면 0,1,2가 된다. 그 둘은 원소-net 경로 유지 = 오늘과 동일 loud(§4.5.206 fail-closed 부분집합 선례).

**`new[]`는 loud 유지**: 라우팅된 net은 "dyn-backed일 뿐 고정 크기"라 `s = new[5]`가 조용한 resize가 되면 하강이다. `fixed_string_dyn` 멤버십으로 거부(같은 크기 `new[2]`도 거부 — 연산 자체가 틀림). 합성된 pre-size만 `lowering_decl_init`로 면제(유저 코드는 그 flush에 도달 불가).

**★★★ 적대 리뷰가 내가 만든 회귀 2건을 잡았다 — 둘 다 "라우팅이 배열을 자기 BARE 이름 위에 올린 것"이 근원**:

- **FALSE-LOUD 회귀(PRE 정상→POST loud)**: v1은 block-local을 **bare 이름**으로 module net에 flatten한다. 초판이 배열을 선언 이름으로 등록하니 block-local `logic [7:0] sa`가 충돌해 dynamic-storage collision reject 발화 — **iverilog가 돌리고 PRE도 맞히던 설계 2형**이 loud. **fix**=원소-net 경로가 `sa$sae$i`로 이름을 비워두던 성질을 그대로 복원(`sa$sad` 맹글링 + `fixed_string_dyn_key` 사이드맵). 해석 순서도 **사이드맵 먼저, symbols 나중**이어야 한다(반대로 하면 hoist된 block-local이 배열의 이름을 가져가 pre-size가 핸들을 못 찾음) — `walk_scopes_key_shadowed`가 각 스코프 레벨에서 사이드맵을 net 바인딩보다 먼저 보므로 shadowing은 유지된다(function-local `int sa[]`가 여전히 이김, 실측).
- **SILENT-WRONG 회귀(PRE loud→POST 조용)**: §4.5.218의 collision 가드가 `string_array_elems` 키라 라우팅 후 발화를 멈췄고, **alias가 새 형태로 되살아났다** — 모듈 자신의 `sa[0]="zz"`와 read-back이 **서로 다른 resolver**를 타서(write는 block-local scalar, read는 라우팅된 배열) iverilog `R=zz,yy`가 조용한 `R=,`(exit 0). **fix**=가드를 `has_fixed_string_array_storage`(두 표현 모두)로 재키잉.

**T1-4(`string q[$]`)는 실측 후 되돌림**: `Dim::Queue` 수용 + 같은 `string_elem_dyn_nets` 마킹은 컴파일되고 `q.size()`도 맞지만 **원소가 전부 빈 문자열**(iverilog `2 aa bb` → 조용한 `2   `). queue push/read 경로가 string VALUE를 string 원소 저장으로 라우팅하지 않는다 = 게이트 확장이 아니라 엔진 작업. correct-or-loud로 **loud 유지**하고 이유를 코드 주석+ROADMAP에 기록.

**3-way 실측(iverilog/PRE/POST·20 probe)**: 라우팅 대상 14형이 LOUD→iverilog 일치, 나머지 PRE==POST. staged(vcmp→velab→vrun) 파리티 확인·per-instance 격리 확인·generate 스코프 불변·format 23 불변(`string_elem_dyn_nets`가 v21 트레일러 사이드카에 이미 있어 bump 불요).

**노출된 pre-existing(신규 아님·ROADMAP §2 기록)**: `{"e", "0"+k}`처럼 concat 원소가 **정수 산술식**이면 4바이트로 렌더(`e\0\0\01` vs iverilog `e1`). **PRE와 바이트 동일**이고 스칼라 string·non-zero-base 배열에서도 동일하게 발화 = 라우팅이 만든 게 아니라 **철자 하나(런타임 인덱스 write)를 더 도달 가능하게** 했을 뿐. 올바른 concat(`{s[k],"!"}`)은 iverilog와 일치하므로 blanket 가드는 false-loud가 되어 거부.

**진단 품질 하강 1건(기록)**: 상수 OOB 인덱스(`string s[2]; s[5]`)가 PRE는 elaborate 에러였는데 POST는 런타임 W4020 경고 + iverilog와 같은 값. 둘 다 non-silent이고 POST 쪽이 값은 정확하나, 컴파일 타임에 잡히던 진단이 런타임으로 내려갔다 — `fixed_string_dyn`이 길이를 알고 있으므로 복원 가능(follow-on).

**잔여(전용 슬라이스)**: T1-4 queue-of-string(엔진 queue 원소 저장) · T1-5 multi-dim string · T1-6 hier `u.s[0]`(cross-instance dyn handle) · T1-7 frame-local string array · non-zero-base/descending의 런타임 인덱스 · 상수 OOB 진단 복원.

**교훈**: (1) **"한 가족"이라는 그룹핑은 근인 측정 전엔 가설일 뿐** — T1 6종은 4개 근인이었고, 2종은 대체 표현(dyn)에서도 똑같이 막혀 있었다. (2) **저장 클래스를 통합하기 전에 capability parity를 전수 측정하라** — 한쪽이 지배하지 않으면 통합은 silent 퇴행이다(여기선 지배가 성립해서 진행). (3) **라우팅이 이름을 차지하면 네임스페이스 전제가 깨진다** — 기존 표현이 이름을 비워두고 있었다면 그건 우연이 아니라 계약이다. (4) **표현을 바꾸면 그 표현을 키로 쓰던 가드가 조용히 죽는다** — §4.5.218 가드가 정확히 그렇게 죽고 silent-wrong이 되살아났다; 가드는 표현이 아니라 **저장(의미)** 에 키잉하라. (5) **게이트만 열어 되는지 반드시 실행해 보라** — T1-4는 컴파일되고 `.size()`도 맞았지만 원소가 전부 비어 있었다. 신규 `fixed_string_array_routing.rs`×17.


#### 4.5.221 real-valued parameters (`parameter real`) loud→supported — 정수-상수 문맥은 loud (적대 리뷰 6라운드·후속 브랜치서 잔여 해소) (2026-07-25, branch feat-real-params → fix-real-param-residual, main `1d2d7eb`) ✅

**4463→4474 green · format 23 불변 · MsgCode 59 불변**

> **머지 이력**: 1차 브랜치 `feat-real-params`는 리뷰 4라운드 시점에 **미해결 3건**으로 보류했고, 후속 브랜치 `fix-real-param-residual`이 5·6라운드로 그 3건 + 새로 발굴된 것들을 닫은 뒤 main에 머지(`1d2d7eb`). 아래 ★ 블록은 라운드 순서대로 읽으면 된다 — **매 라운드가 직전 라운드의 fix에서 blocking을 찾았고, 매번 다른 축이었다**(도메인 → resolver → index → size/count → **생산자 판별자** → **술어 자체**).

**갭**: `parameter real R = 1.5;` / `localparam real CLK = 5.0;` 가 elaborate서 거부. `params: BTreeMap<String,i64>` 뿐이라 real 값을 담을 곳이 없었음. iverilog 오라클 有(강함).

**구현**:
- **`real_param_val: BTreeMap<String,f64>`** — elaborate-local 사이드맵(§4.5.217 `str_param_raw` 와 동형). `params` 와 분리하는 이유 = real 은 i64 로 표현 불가하고, 섞으면 정수 fold 가 조용히 반올림한다.
- **`param_real_value(ty, value)`** — **선언 타입**(Real/Realtime)으로 키잉. 값 모양이 아니라 선언으로 판정해야 `parameter real R = 3;`(정수 리터럴 초기화)이 real 로 바인딩된다.
- **`real_literal_value`** — 파싱된 f64 를 부정. 초판은 `format!("-{inner}")` 텍스트 재조립이라 중첩 unary minus 가 `"--1.25"` → 파싱 실패 → **0.0**(측정으로 포착).
- **바인딩**: 모듈 body/header · interface header · program header · instance array · `realtime`. **loud**: generate scope · package · interface body · defparam · hierarchical `dut.R`.
- **`lower_index_expr`**(신규 `packed.rs`) — 인덱스/오프셋/바운드를 lower 한 뒤 **IR 위에서** `expr_is_real` 로 거부. 12개 select 사이트 전부 경유. 모양이 아니라 값으로 판정하므로 `v[R+0]`·lvalue `v[R]=0` 까지 잡는다(구조적 완전성).
- **override**: real 리터럴 override 를 warn→**error** 승격. `ResolvedOverride.had_value: bool` 을 5개 생성 사이트 전부에(=`Default` 없는 bare bool 이라 컴파일러가 누락 강제).

**★ 리뷰 2라운드 — leaf conversion 은 더 나쁜 오답이었다**: S5(`$clog2(R)` 가 **조용히 1-bit width**)를 고치려 `const_eval_in_scope` 의 **leaf** 에서 real→i64 변환을 넣었더니 두 리뷰어가 독립적으로 NOT-CLEAN 반환:

| 소스 | iverilog | vita(leaf 변환) |
|---|---|---|
| generate-if `R > 2`, R=2.4 | taken | **NOT taken** — generate body 통째 삭제, exit 0 |
| `(R == 2)` / `(S != 0)` | `0 1` | **`1 0`** |
| `localparam real B = A;` (A=1.5) | 1.50 | **2.00** |
| `localparam real HALF = CLK/2;` | 2.50 | **2.00** (클럭 주기 관용구) |
| `localparam int A = R*2;` | 3 | **4** |
| `logic [R/2-1:0]` R=5.0 | 3 bits | **2 bits** |

**근본**: IEEE §11.8.1 은 real operand 를 포함한 식을 **real 도메인**에서 평가하고 문맥 경계에서 **한 번** 변환한다. leaf 변환은 **둘러싼 연산자가 무엇을 필요로 하는지 결정하기 전에** real 값을 파괴한다 — 조용한 1-bit 를 고치려다 조용한 **잘못된 분기·잘못된 값**을 만든 것(사다리에서 내려감).

**최종 해법 = 되돌리고 loud**: `check_const_range_bound` 가 `count_reads_real_param(e)` 를 **먼저** 검사해 에러. 기존 `nonconst_bound_reason` 은 (`$bits(net)` false-loud 회피 때문에) system-call 인자로 내려가지 않아 `$clog2(R)` 가 진단 없이 `clamp_bound_u32(None)` → width 1 이 됐던 것. **어디서도 변환하지 않고 여기서 loud.** iverilog 는 변환을 지원하므로 이건 기록된 **capability gap** 이지 correctness 결함이 아니다.

**★★ 재감사 2라운드 — revert 만으로는 부족했다(PRE 바이너리 3-way 실측)**: 리뷰어가 HEAD에서 **PRE 바이너리**를 빌드해 iverilog/PRE/POST 3중 측정 → **양방향 사다리 하강** 4건 발견.

- **BLOCKING #1 SILENT-WRONG(PRE loud → POST 조용)**: `count_reads_real_param` 에 **`Call` arm 이 없고** `nonconst_bound_reason` 도 call 인자로 안 내려가 **const-function 이 real param 을 밀반입**. `logic [f(R)-1:0]`→**조용히 1-bit**(iverilog `w=8`)·`arr [f(R)]`→`$size`가 R 과 무관한 **8**(iverilog 4)·`{f(R){1'b1}}`→**조용히 빈 값**(iverilog `111`). 셋 다 PRE 는 loud 였음 = **내가 loud 를 silent 로 만든 것**. **fix**=`Call{args}` arm 추가.
- **BLOCKING #2 FALSE-LOUD 회귀 8형**: `parameter real R = 4`(정수 상수 초기화)를 real-ONLY 로 바인딩해 **정수 능력 전부 상실** — `logic [R-1:0]`·`arr [R]`·`$clog2(R)`·`localparam int W = R`·`generate if (R>2)`·`generate for (i<R)`·정수 override `#(.R(i+2))`·positional `#(5)` 가 전부 PRE 에서 **byte-correct 였는데** POST 에서 loud. **fix**=초기화가 **정확한 i64 로 fold 되면 `real_param_val` 과 `params` 양쪽에 등록**(두 표현이 정확히 일치하므로 안전) → `R/2`는 real 도메인서 **1.5**, `logic [R-1:0]`는 정수 도메인서 **4**. 판별자도 `real_param_is_non_integral`(= real_param_val 에 있고 params 에 **없음**)로 재키잉해 정수 쌍둥이가 있는 param 은 bound guard 가 안 문다. folded override 는 `v as f64` 로 **적용**(거부 아님).

**실측 결과(14형 전수·iverilog 대조)**: S1/S2/S3 = silent→**LOUD** · L1/L3/L4/L5/L6/L7/L8/L8b = 전부 **iverilog 와 일치 복구** · `parameter real R = 3; R/2` = **1.5000**(real 도메인 이득 보존) · 진짜 non-integral(`R = 2.4` generate-if·`R = 8.5` width) = 여전히 **LOUD**(silent 로 안 내려감).

**교훈(추가)**: (6) **PRE 바이너리 3-way 측정이 양방향 하강을 잡는다** — iverilog 대조만으로는 "PRE 에서 되던 게 POST 에서 loud" 를 못 본다(둘 다 "vita 가 loud" 로만 보임). 새 기능이 기존 바인딩 분류를 바꾸면 **PRE 대조 필수**. (7) **저장 클래스를 새로 만들면 기존 클래스의 능력을 상속시켜라** — real 로 재분류하는 순간 정수 능력이 통째 사라졌다(두 표현이 **정확히 일치**할 때는 양쪽 등록이 정답). (8) **loud gate 를 추가하면 그 gate 를 우회하는 간접 경로(함수 호출·계층 이름)를 같은 반복에 전수** — gate 가 "유일한 그물" 이라고 문서에 썼다면 실제로 유일한지 측정하라.

**미해결(기록)**: 계층 real param `logic [$clog2(u.R)-1:0]` 는 PRE loud → POST 조용히 1-bit(`Ident` arm 이 `segments.len()==1` 요구). 좁고 iverilog 도 거부하나 **하강은 하강** — ROADMAP §2 에 기록. 그리고 pre-existing(PRE==POST): 파라미터 구조적 지연 `assign #P y = x;` 가 P 가 정수여도 **조용히 무시**(real param 이 클럭 주기 관용구라 이 슬라이스가 도달성을 크게 넓힘)·real→`input int` formal 미강제.

**★★★ 재감사 3라운드(PRE 3-way) — 2라운드 fix 가 새 구멍 3개**: 
- **B1 SILENT-WRONG(PRE loud→POST 조용)**: real param 을 **정수 formal 의 override** 로 쓰면 자식이 조용히 default 유지. `sub #(R) u(o)`(R=4.5) → iverilog `W=5`·PRE loud·POST **`W=8` warn 만, exit 0**(포트 폭까지 조용히 바뀜). 근인=guard 가 `expr_is_real_literal`(**구문적 리터럴**) 테스트인데 이 슬라이스가 **비-리터럴 real 식**(`R`·`R+1`·`R*2`)을 새로 도달 가능하게 만들어 warn-and-keep-default 로 낙하. **fix**=`count_reads_real_param` 으로 게이팅(4 site).
- **B2 — `&& !params.contains_key` 절이 실제로 구멍을 냄**: `{R{1'b1}}`(R=4) → **2²⁴ bits(~16 MiB) exit 0**(PRE `1111`·iverilog 거부). `real_param_is_non_integral` 은 i64 쌍둥이가 있어 **false** 를 주는데, `lower_expr` 의 Ident arm 은 **`real_param_val` 을 `params` 보다 우선**하므로 real `Const` 가 `ir::Expr::Replicate` 에 도달. **= 내가 기록해 둔 `classifier-must-match-lowering-resolver` 교훈에 그대로 걸림.** **fix**=**같은 술어의 두 resolver 판**을 두고 consumer 가 자기 resolver 에 맞는 쪽을 고른다 — const-**fold** consumer(`check_const_range_bound`)=`real_param_is_non_integral`(`params` 기준) · **lower** consumer(replication)=`count_lowers_real_param`(`real_param_val` 기준).
- **B3 — `lower_index_expr` 가 "모든 index site" 를 안 덮음**(내 doc 주석이 거짓): `+:`/`-:` 의 **WIDTH** 가 미게이팅 → `v[0 +: R]` 이 **2²⁴ x-bits exit 0**. i64 쌍둥이 유무와 무관하게 발화. **fix**=`expr_main` 2 site·`lvalue` 2 site·`strings` 2 site 를 전부 `lower_index_expr` 경유.

**실측(10형)**: b1/b1n silent→**LOUD** · b1ok(정확한 쌍둥이 `R=4`)=**W=4 iverilog 일치**(false-loud 없음) · b2/b2b/b3/b3b/b3l/b3s 16MiB 폭주→**LOUD** · 합법 능력(`logic [R-1:0]`·`arr [R]`·`R/2`)=**iverilog 일치 유지**.

**교훈(추가)**: (9) **"같은 술어" 를 여러 consumer 가 공유할 때, consumer 마다 resolver 가 다르면 술어도 갈라야 한다** — 하나로 합치면 한쪽이 반드시 틀린다(fold=`params` vs lower=`real_param_val` 우선). (10) **"모든 site 를 덮는 단일 래퍼" 라고 주석에 쓰면 그 주장을 테스트로 고정하라** — 주석은 6 site 중 4 site 만 참이었고, 남은 2 site 가 16MiB 폭주였다. (11) **가드를 구문(리터럴 모양)으로 세우면 새 슬라이스가 그 구문 밖의 값을 도달시키는 순간 뚫린다** — 값 기반으로.

**★★★★ 4라운드(PRE 3-way) — 3라운드는 select/index 축만 닫았고 "size/count/position 인자" 축이 통째로 남아 있었다**: 메서드·시스템태스크의 크기/개수/위치 인자가 여전히 raw `lower_expr` → real `Const` 가 엔진에 도달해 **f64 비트패턴이 정수로 읽힘**(전부 exit 0):
- **`new[R]`(R=3) → 2²⁴ 원소 할당 = 피크 RSS 1.22 GB·`size()` 가 16777216**(iverilog·PRE 는 `n=3`). R=4.5 면 **PRE loud → POST silent** 순수 하강.
- `s.substr(R,3)`→**빈 문자열**(PRE `ell`) · `s.getc(R)`→**0**(PRE 101) · `s.putc(R,"Z")`→**쓰기 조용히 소실**(PRE `hZllo`).
- `q.insert(R,…)`·`q.delete(R)`·assoc `a.delete(R)` → **엉뚱한 슬롯**(PRE 정답).
- **B5 = 3라운드 fix 가 한 구문 층 모자랐음**: `count_lowers_real_param` 의 `_` arm 이 `count_reads_real_param` 으로 폴백 = **const-fold resolver 로 되돌아감** → i64 쌍둥이가 있는 real param 에 `false` → `{$clog2(R){1'b1}}`(R=4) 가 **조용히 0**(iverilog·PRE `3`). `SysCall`·`Ternary` 가 정확히 그 `_` arm 의 shape 였음.

**fix**=`new[]`·queue/assoc index·string-method 인자 6 site 를 `lower_index_expr` 경유(그 인자들은 전부 정수라 안전) + `count_lowers_real_param` 에 `Ternary`/`SysCall` arm 을 **위임이 아니라 미러링**. 9형 실측 전부 LOUD 전환·정수 param(`new[P]`·`substr(1,3)`·`{$clog2(P){1'b1}}`)은 `4 ell 3 3` 로 불변.

**미수정 1건(ROADMAP §2 기록)**: `$readmem*` 주소 인자. 그 lowering site 는 **모든** systask 인자를 처리하는 공용 경로라 통째 게이팅하면 `$display("%f", R)` 가 false-loud — `$readmem*` 전용 인자-위치 게이트 필요.

**교훈(추가)**: (12) **"인덱스 축을 닫았다" 는 "정수를 요구하는 모든 인자를 닫았다" 가 아니다** — select/index 와 **size/count/position** 은 별개 축이고, 후자는 엔진이 f64 비트패턴을 정수로 읽어 **할당량 폭주**까지 간다. 정수를 요구하는 인자를 **축이 아니라 전수**로 세어라. (13) **폴백 `_` arm 이 다른 resolver 의 함수로 위임하면 그 지점에서 resolver 가 뒤바뀐다** — 술어를 resolver 별로 갈랐으면 **모든 arm 을 미러링**하라(위임 1줄이 전체 분리를 무효화). (14) 공용 lowering 경로에 게이트를 걸 땐 **그 경로를 쓰는 다른 consumer 를 먼저 세어라**(전부 정수 인자면 안전·아니면 위치별 게이트).

**★★★★★ 5라운드(후속 브랜치 `fix-real-param-residual`) — 4라운드까지는 전부 *호출 지점*만 고쳤고 *판별자*는 안 고쳤다**: `lower_index_expr`가 아무리 많은 site를 경유해도 그 안의 `expr_is_real`이 **real 값의 생산자를 못 알아보면 게이트 ~40곳이 전부 열린 채**다. 실측으로 4종 발견:

- **`.atoreal()`** · **dyn `real d[]`의 원소** · **`ArrSum`/`ArrProduct`**(`.sum()`/`.product()`) → `expr_is_real`에 arm 추가(dyn 원소는 `real_elem_dyn_nets` 키잉).
- **real 반환 FUNCTION** → **IR에는 real 마커가 없다**(inline은 본문을 식으로 접고, `func_return_dims`는 Real kind를 계산만 하고 버려서 반환 net이 `NetKind::Real`이 아니다 — 양 경로 동일). `lower_index_expr`는 **AST를 들고 있으므로** 선언 타입을 직접 조회(`call_returns_real`)해서 해소.

같은 라운드에서 **누락 site 축**도 재차 드러남 — 4라운드까지 문서 주석이 "모든 index site를 덮는다"고 세 번째로 거짓이었다: **쓰기/계층/배열-원소 11형**(`lvalue.rs` part-select 바운드·hier idx chain·indexed part / `expr_main.rs` hier idx chain / `arrays.rs` 배열 word index r/w)을 전부 `lower_index_expr` 경유로 전환. 경계 변환 덕에 **정수-값 real은 correct-support**(`v[7:R]`=f8·`m[1][R]`=08·`u.v[R]`=ad), 비-정수는 loud. 추가로 replication count의 `MinTypMax` 무한 루프(최종 else를 `lower_index_expr` 경유 = **lower된 IR로 판정**하니 AST arm 열거의 비수렴이 구조적으로 소멸)와 `$readmem*` 주소 인자(공용 systask 경로라 **인자 위치** argi≥2만 게이팅 — 통째 게이팅은 `$display("%f",R)`를 false-loud)를 해소.

**★★★★★★ 6라운드 — 내 fix 2건이 되돌려졌다(되돌린 것이 결론)**:

- **`call_returns_real` 근사 복제본 = BLOCKING false-loud**: 5라운드에서 `ast_has_real_call`을 재구현했는데 **연산자 가드를 빠뜨린 near-copy**여서 `v[fr(2) > 2.0]`(비교 결과는 정수 1비트)처럼 real이 **소비되고 끝나는** 식까지 real로 판정 → 8 site false-loud, 프로브 하나는 VCD 자체를 잃음. **fix = 복제본 삭제하고 위임**.
- **`**`(Pow)가 두 쌍둥이 술어 양쪽에 없었음** — real-propagating 연산자인데 `+ - * /`만 있었다. 양쪽에 추가(한쪽만 고치면 §4.5.221 B5와 같은 resolver 갈림).
- **되돌린 것 2건**: (a) "바운드는 folded `Const`여야 한다"는 더 강한 요구 → 정수 param의 `$clog2(P)`를 false-reject(엔진이 elaborate-time fold 없이 상수로 평가)해서 **revert**, 이유를 site에 주석으로 고정. (b) part-select 바운드에 `const_bound_u32` 폴백 추가 → 테스트는 통과했으나 프로브 f1/f5가 **불변**(`const_eval_in_scope`에 `Cast` arm이 없어 애초에 발화 안 함) → **측정된 이득 0이므로 hot path 변경을 revert**.

**최종 3라운드 리뷰 결과 = CLEAN**(4092 설계·false-loud 0·280-case fuzz·panic/hang 0). PRE 바이너리에서 발견된 진짜 hang 2건도 수정.

**교훈(추가)**: (15) **호출 지점을 전수했다는 것이 판별자가 맞다는 뜻은 아니다** — 게이트를 40곳에 심어도 그 안의 술어가 생산자를 모르면 전부 no-op다. 술어를 넓힐 때는 **값의 생산자 목록**(변환 함수·컨테이너 원소·리덕션·함수 반환)을 세어라. (16) **IR에 마커가 없으면 AST에서 조회하라** — "IR 위 판정이 구조적으로 완전"(교훈 4)은 **IR이 그 정보를 보존할 때만** 참이고, real 반환 타입은 보존되지 않는다. (17) **기존 술어를 재구현하지 말고 위임하라** — near-copy는 원본의 가드를 빠뜨리고, 그 차이가 false-loud로 나타난다. (18) **이득이 측정되지 않은 hot-path 변경은 되돌려라** — "도움이 될 것"은 근거가 아니다(테스트 통과는 무해함의 증거이지 유용함의 증거가 아니다).

**correct-or-loud 잔여(LOUD)**: 정수 상수 문맥의 real param(width/range/`$clog2`) · `localparam real B = A;`(real→real alias) · real 식 override · generate/package/interface-body/defparam 바인딩 · **바운드가 산술식**인 경우(`v[R+2:R]` — 직접 real `Const`가 아니라 경계 변환 불가·iverilog도 거부) · `$clog2(<real>)`(iverilog는 3) · 계층/package-scoped callee의 real 반환(다른 테이블로 resolve되어 미청구·보수적).

**교훈**: (1) **타입 변환은 leaf 가 아니라 문맥 경계에서** — 조용한 1-bit 를 고치려 leaf 변환하면 조용한 잘못된 분기가 된다(두 silent-wrong 을 맞바꾼 셈). (2) **"고쳤다"를 리뷰 없이 믿지 말 것** — differential 과 soundness 가 **서로 다른 진입점**으로 같은 근본을 잡았다(전자=generate-if/비교, 후자=real→real alias 와 `R*2`); 한 렌즈만 돌렸으면 나머지 절반이 배포됐다. (3) **선언 타입 키잉**이라야 `parameter real R = 3;` 이 real 로 바인딩. (4) **IR 위 판정이 구조적으로 완전**(`lower_index_expr`) — AST walker 는 shape 를 놓치지만 lower 된 값은 못 숨는다. (5) 리터럴 부정은 **파싱된 값**을 부정(텍스트 재조립은 중첩에서 깨짐). 신규 `real_params.rs`×27.

#### 4.5.220 SILENT-WRONG 수정: DYN string-array element의 byte select가 0 → supported + write-twin 3형 loud화 (format 23 불변) (2026-07-25, branch feat-dyn-string-elem-byteselect) ✅

**컨텍스트**: §4.5.219가 기록한 **전제조건 슬라이스**. fixed→dyn 라우팅(T1-2/3)을 하려면 먼저 **dyn ⊇ fixed**여야 하는데, byte-select만 fixed가 우세(119 vs dyn 0)했다.

**오라클**: iverilog가 `d[0][0]`을 거부(no-oracle)→**vita-내부 등가 차분**이 teeth — 같은 "world"를 담은 FIXED 원소(`s[0][0]`=iverilog 119 111·오라클 검증)와 SCALAR(`p[0]`=119 111)가 기준점. hand-IEEE=§6.16.2(byte select·범위 밖=0).

**근본 원인(READ)**: `handle_is_str_readable`(엔진이 이 handle의 BYTE를 읽을 수 있는가)가 `Signal{word:None}`·`Const`만 수용. dyn 원소는 **word-indexed** `Signal`이라 거부→width-0 handle의 packed bit-select로 낙하해 **조용히 0**. 정작 엔진 `handle_str_bytes`는 그 shape를 eval fallback으로 항상 읽을 수 있었다(`%s`·`d[i]` 비교가 쓰는 바로 그 경로)—**gate가 자기 술어를 under-approximate**한 것.

**fix(format 23 불변·elaborate-local)**: gate에 `Signal{net, word:Some(_)}` + `string_elem_dyn_nets` arm 추가. **판별자 건전성 논거**(soundness가 제시한 더 강한 형태): 그 집합은 **엔진의 `dyn_str_elem` flag를 구동하는 바로 그 집합**이라, 엔진이 문자열로 저장하지 않는 net을 gate가 admit할 수 없다. 또 **이미 lower된 노드의 net-id로 판정**하므로 자기가 분류하는 lowering과 불일치 불가. **load-bearing 불변식**: 원소 Value가 `is_str`을 들고 있고 `resize_keep_sign`이 `is_str`에서 early-return한다(string dyn net은 width 0이라 아니면 1-bit로 truncate).

**의도보다 넓게 고쳐짐(differential 발굴)**: dyn record-array의 **SoA string member**(`r[0].nm[0]`·파서 `$unp$r$nm[]` desugar가 같은 marker에 도달)·`$bits`(1→8)·`%c`·continuous-assign/NBA/generate/frame-task 문맥·nested `d[0][1][2]`·`d[0][0][3:0]`·`$signed` 全 0→정답.

**★ 적대 2-lens가 write-twin family를 발굴(전부 pre-existing·PRE==POST·fixed/scalar 쌍둥이는 이미 loud)**: (1)`d[0][3:0]=4'hF`→`worle`(**엉뚱한 바이트**) (2)`d[0][15:8]`→**조용한 no-op** (3)runtime offset `d[0][j+:4]`→`worle`(별개 silent 경로) (4)`{d[0],x}=8'hAB`→원소를 **조용히 비움** (5)`real r[]; r[0][3:0]`→조용한 no-op. 근인=엔진이 원소를 byte-string/f64로 **재도출**하며 offset/width를 버림.

**★ soundness의 배치 논거가 핵심 기여**: 초판 guard를 `lval_part_base`에만 뒀는데, `lower_lvalue`가 **문서화된 단일 퍼널**이고 이미 형제 string guard 2개를 갖고 있으며 그것들이 `is_string_net`(=`NetKind::String`) 키라 **dyn 원소의 `DynArray` net에 장님**—그래서 concat 형제가 열려 있었다. → 공유 술어 **`is_string_valued_net`**(scalar String ∪ `string_elem_dyn_nets`)로 퍼널 guard 2곳을 재키잉해 concat 축을 **구조적으로** 닫고, part-select는 더 구체적 메시지를 위해 조기 reject 유지하되 **같은 술어 계열**을 쓰게 해 2-site drift 제거(§4.5.183 dual-collector와 동일 함정). `real`은 `is_non_bit_addressable_elem_net`(string ∪ real element)로 동반 처리.

**재감사 양 lens CLEAN**: differential=**644 파일 sweep·panic 0·empty-output-at-exit-0 0·PRE-worked→POST-loud 40건 전부 두 guard 중 하나에 귀속·미귀속 0**·fuzz 60·staged==one-shot(신규 guard는 velab서 발화). soundness=`offset:Some` LvalChunk **13 생성 site 전수**해 `lval_part_base`가 유일 choke point임을 구조적으로 증명·bypass(hier/force/`$sscanf`/streaming/nested-packed) 全 차단 확인.

**리뷰어 자기정정 2건**(교훈 가치): differential이 `inline_fn` consumer를 "inert"라 했으나 soundness가 반증—**4-state return** inline 함수(`function [7:0] f(input string s)`)면 formal이 verbatim 바인딩돼 word-indexed handle이 gate에 도달, `f = s[0]`이 PRE **0(silent-wrong)**→POST 119이고 `.len()/.getc()/.atoi()` 등 loud→supported. differential은 `function int`(2-state→framed)·`automatic`만 시험해 놓쳤다고 스스로 진단. 또 `(p)[0]` 초기 보고를 "일반 paren-select 갭"에서 **string 전용**으로 정정.

**correct-or-loud 잔여(ROADMAP §3 기록)**: `(p)[0]` paren base(동일 실패 클래스·한 gate 옆) · scalar `real x[3:0]` write(이제 scalar가 뒤처진 **반대 방향** 비대칭) · `string q[$]`/assoc/multi-dim/frame-local/package/class(선언 loud).

**교훈**: (1) **gate가 "엔진이 X를 못 한다"고 말할 때 엔진을 직접 확인하라** — 여기선 엔진이 이미 할 수 있었고 gate만 under-approximate였다. (2) **판별자는 그것이 구동하는 저장소와 같은 집합을 써라**(`string_elem_dyn_nets`가 엔진 `dyn_str_elem`도 구동→admit 불가 논거가 "집합이 int를 배제한다"보다 강함). (3) **READ를 넓히면 WRITE twin을 같은 반복에서 전수하라** — 여기선 twin이 5형이었고 전부 pre-existing silent였다. (4) **guard는 문서화된 단일 퍼널에 두고 술어를 공유하라**(별도 site에 두면 형제 축이 열린 채 남는다). (5) **리뷰어의 "도달 불가/inert" 주장도 측정 대상**(경로 분기 1개[return 타입 2-state/4-state]가 consumer 도달성을 가른다). 신규 `dyn_string_elem_byte_select.rs`×23. **4412→4435 green**·clippy/fmt clean·format 23 불변.

#### 4.5.219 FIXED string-array decl-init `string s[N] = '{…}` loud→supported (t0 pre-sweep per-element 확장·format 23 불변) (2026-07-25, branch feat-fixed-string-array-init) ✅

**컨텍스트**: 오너 지시로 재정렬한 **§0 correct-support 승격 큐 T1-1**. DYNAMIC 형태(`string s[] = '{…}`)는 이미 동작하는데 FIXED만 전 스코프 blanket-loud였음 — **그 비대칭 자체가 갭**(§4.5.198 논리). iverilog 오라클 有.

**설계 결정(실측이 방향을 바꿈)**: 처음엔 "fixed를 dyn 표현으로 통째 라우팅"이 유력해 보였으나, **capability-parity 매트릭스 실측** 결과 **어느 쪽도 우세하지 않았다** — dyn이 init/runtime-index/`foreach`/`.size()`에서 앞서지만 **byte-select `s[0][0]`는 fixed만 정확**(119 vs dyn 0). 라우팅했으면 byte-select가 silent 0으로 **퇴행**했을 것(저장-클래스 변경의 전형적 함정). → 라우팅 포기, **additive 확장**으로 축소.

**fix(format 23 불변·elaborate-local 3파일)**: `'{…}`를 선언 인덱스별 `s[k] = <elem>` 로 전개해 기존 t0 var-init pre-sweep에 push(const-index element 경로가 그대로 소비). 공유 헬퍼 `fixed_string_array_init_pairs` 하나를 module-scope collector와 block-local collector가 **동시에** 사용(두 collector가 갈라져 init을 조용히 흘린 전례=§4.5.183).

**핵심 난점 = fill order**: 패턴 원소 k는 **선언 범위의 LEFT 경계부터 오른쪽으로** 채워진다(IEEE §10.9.1). `string s[3:1] = '{"a1","b2","c3"}`→s[3]=a1·s[1]=c3 (iverilog 확인). `string_array_elems`는 `min`부터 오름차순 저장이라 descending은 패턴을 역순으로 walk해야 함.

**★ 적대 2-lens가 내가 넣은 결함 5건 포착(양 lens가 서로 다른 것을 잡음)**:
- **F1(differential·SILENT-WRONG)**: mapping은 맞았으나 **element write의 실행 ORDER**가 틀림. iverilog는 오름차순 declared-index로 대입하는데 전개는 패턴 순서(descending 선언에선 내림차순)로 방출→**원소 initializer가 형제 원소를 읽으면** 값이 갈림(`'{"AA", peek(), "CC"}`에서 peek→s[3]가 이미 쓰인 값을 읽어 `[CC][AA][AA]` vs iverilog `[CC][ ][AA]`). fix=`step<0`이면 pair vector reverse(mapping 불변·ascending byte-identical).
- **F2(differential·loud→silent 확대)**: hier 원소(`'{u.p,…}`)가 조용히 `""`. 근인은 pre-existing(scalar `string z = u.p;`도 동일)이나 **loud였던 것을 silent로 넓히면 안 됨**→`_`-free-exhaustive `expr_mentions_hier_path`로 loud화.
- **negative bound(soundness)**: `IntLit{raw:"-2"}`가 decimal parse에서 거부돼 폴드 불가→오도하는 loud+부분 전개. fix=`Unary{Minus, IntLit}`.
- **i64 overflow(soundness)**: `left-right`가 pathological bound에서 **panic**(main은 깔끔한 진단이었음). fix=`checked_sub`+원소 수 cap(4096, `ARRAY_PATTERN_UNROLL_CAP` 미러).
- **decl/collector 재판정(soundness)**: collector가 `allow_string_init`을 못 봐서 interface/generate/package에선 decl은 loud인데 collector가 push→**cascading E3010**. fix=두 collector를 **decl의 실제 산출물(`string_array_elems`)** 로 게이팅→불일치가 구조적으로 불가.

**리뷰 후 추가 발견+수정(false-loud)**: F2 게이트가 `q.size()`/`p.substr()`까지 삼킴 — 파서가 `recv.method(...)`를 `Call{HierPath[recv,method]}`로 인코딩해 `u.f()`와 **같은 2-segment 형태**이기 때문. fix=게이트를 Elaborator 메서드로 올려 **head가 local net으로 resolve되면 method call, 아니면 cross-instance**(=method lowering과 동일 resolver·§4.5.217 교훈 적용). `u.p`/`u.f()`는 loud 유지.

**재감사**: soundness **CLEAN**(prefix hole 없음·double-flip 없음·`$blk$` 잔여는 내 주장을 정정해줘 comment로 고정) · differential은 false-loud 외 silent-wrong 0·105-file 코퍼스 재실행·repo examples PRE≡POST byte-identical. **잔여 divergence 1건은 pre-existing**(uninit string-array element 렌더 `[ ]` vs `[]`·init 없는 배열에서도 동일).

**correct-or-loud 잔여**: generate/interface/package scope(=`allow_string_init` false·문서화된 follow-on) · `automatic` block-local · hier 원소 · 4096 초과 원소 · count mismatch(iverilog도 거부).

**교훈**: (1) **저장-클래스 통합 전 capability-parity를 실측하라** — "새 표현이 더 낫다"는 직관이 틀려 byte-select가 퇴행할 뻔했다(어느 쪽도 우세하지 않으면 통합이 아니라 additive가 답). (2) **mapping이 맞아도 실행 ORDER가 관측 가능하면 별도 검증 대상**(soundness가 mapping을 4중 검증하고도 F1을 놓친 이유=order-sensitive probe를 ascending에서만 만들어 두 순서가 구분 불가했음). (3) **loud를 silent로 넓히지 마라** — 근인이 pre-existing이어도 내 변경이 표면을 넓히면 그건 내 책임. (4) **파서 인코딩이 의미를 가린다**(method call과 hier call이 같은 2-segment AST)→**분류는 lowering과 동일 resolver로**. 신규 `fixed_string_array_init.rs`×25. **4387→4412 green**·clippy/fmt clean·format 23 불변·staged 패리티 ✓.

#### 4.5.218 SILENT-WRONG 수정: inner-scope local이 OUTER string-array side-map을 shadow 못 하던 문제 → supported (opt-in shadow-aware scope walk·format 23 불변) (2026-07-25, branch feat-scope-shadow-sidemap) ✅

**컨텍스트**: §4.5.217 적대 2-lens 리뷰에서 **양 reviewer가 독립적으로 CONVERGE**해 발굴한 항목(ROADMAP §0 NEXT 최상단). 오라클 有(iverilog가 전부 지원하는 구문)·read/write 양 경로·**非-string 로컬까지 오염**.

**근본 원인**: `walk_scopes_key`(유일한 outward scope walk)는 **`hit`이 probe하는 그 한 map 안에서만** innermost-wins다. 그런데 function/task/block/generate 로컬은 **NET**(`symbols`)이고, 이 walk를 쓰는 여러 consumer는 **다른 keyspace**(`string_array_elems`·`array_const_vals`·`pkg_var_aliases`·`iface_insts`·`genvar_decls`)를 probe한다→walk가 inner net binding을 **그냥 지나쳐** inner 로컬을 OUTER side-map 엔트리로 resolve. 조용히, 그리고 string이 아닌 로컬에서도.

**silent-wrong(iverilog 차분 확정)**: 모듈 `string sa[2]` + inner 로컬 `sa`
- function-local `string sa`: `sa[0]<sa[1]`이 vita 0 vs iverilog 1(byte-select가 배열 비교로) · inline(static) function 동일
- task-local `logic [15:0] sa`: `{sa[0],sa[1]}`이 vita 1515673431 vs iverilog 1
- task-local `logic [7:0] sa[2]` **WRITE**: 모듈 string 배열 원소를 덮어씀(vita `a,YY` vs `ZZ,YY`)
- generate-local `logic [15:0] sa`: `sa[8]`이 **bogus LOUD**(“string-array index 8 is out of the declared range [0:1]”)—외부 배열 선언 범위로 range-check. **이 케이스가 “string 전용이 아니다”의 결정적 증거**.
- block-local `string sa`: 모듈-scope `sa[0]="zz"`가 block-local 스칼라로의 `putc` byte-write로 격하→읽으면 빈 문자열

**fix(format 23 불변·elaborate-local·4파일 +82줄)**: (1) `walk_scopes_key`는 **원본 그대로 복원**하고, shadow 검사를 **opt-in** `walk_scopes_key_shadowed`로 분리(둘 다 private `walk_scopes_key_inner(name,hit,stop_at_net_binding)`에 위임). (2) `string_array_elems` 3 site만 opt-in(`strings.rs` read 2 + `lvalue.rs` write 1). (3) block-local이 모듈 string-array 이름과 충돌하면 **`d.kind == String`일 때만** loud.

**★ 적대 2-lens가 내가 넣은 결함 2건 포착(둘 다 배포됐으면 심각)**:
- **S1(soundness·CRITICAL)**: 초판은 shadow 검사를 `walk_scopes_key` **전체**에 넣었는데, 이 검사는 **elaboration 중 채워지는 `symbols`에 의존=순서 의존적**이다. `elaborate_gen_item`은 generate control expr를 **phase마다 재-fold**하면서 fold 실패는 **Nets phase에서만** 진단한다→sibling net이 Nets 시점엔 없고 Logic 시점엔 있어서, 중첩 generate가 Nets서 fold되고 Logic서 실패→body가 unroll된 뒤 **lower되지 않고 통째로 사라짐(exit 0·errors=0·무진단)**. 고치려던 버그보다 **더 나쁨**. → shadow walk를 opt-in화해 `params`/`param_meta` 등 13 consumer를 byte-identical로 되돌려 해소(부수적으로 soundness가 측정한 **~20% elaboration perf 회귀**도 소멸).
- **R1(differential)**: 초판 fix B가 이름 충돌만 보고 loud→**11개 byte-correct 설계를 false-reject**(`logic [7:0] sa;`·`logic [7:0] sa[2];`·`int`/`real`·multi-name 등, 전부 iverilog 정상+vita PRE 정상). PRE 빌드로 hazard set을 실측해 **STRING kind gate**로 축소.

**리뷰어 제안을 실측으로 개선**: reviewer가 제안한 gate는 “String **또는** block이 이름을 index-select”였으나, reviewer 자신의 반례 `c2`가 index-select(`sa[0]=8'hAA`)하면서 정상이라 그 술어는 c2를 계속 reject한다. 반대로 read-predicate(`new_str_read` 방식)를 넣었으면 **write-only string block-local**(재감사서 발견한 `n4`)이 이미 silent-wrong이라 놓쳤을 것. **bare `NetVarKind::String` gate가 정답**(string 로컬이 bare name을 점유하는 순간, 읽든 안 읽든 모듈-scope `sa[i]` write가 `putc`로 격하). 재감사 verdict: “both halves of my original suggestion were wrong, and your gate is better”.

**재감사 결과 양 lens CLEAN**: differential=PRE/POST/iverilog 3-way **114 설계**(false-reject 13 전부 복원·string-array fix 10 유지[신규 발굴 2 포함: function **formal** shadow·중첩 generate]·String-gate loud 6개 전부 PRE서 이미 broken/loud=**false-reject 0**·param class 5개 `PRE==POST`·over-rejection sweep ~70 regression 0)·**`##EMPTY-OUTPUT-EXIT0##` 전용 detector로 S1 시그니처 0건**.

**correct-or-loud 잔여(전부 `PRE==POST`=pre-existing·ROADMAP §2)**: inner **net이 outer PARAM/enum-label을 shadow 못 함**(order-INDEPENDENT AST-gathered name set이 필요→전용 슬라이스) · block-local이 **imported package 변수**를 clobber · block-local scalar vector를 block이 index-select하는 경우 · fork-arm/for-body/unnamed block(별도 메시지로 이미 loud).

**교훈**: (1) **공유 walk에 semantics를 추가할 땐 default가 아니라 opt-in**—consumer마다 순서 의존성·안전 전제가 다르다(13/14가 무관한데 전부 리스크를 지게 됨). (2) **mutable elaboration state(`symbols`)에 name resolution을 걸면 phase 재실행과 충돌**한다: 진단이 특정 phase에만 있으면 **조용한 삭제**가 된다(diagnostic gate가 phase-limited인 곳을 먼저 확인). (3) **REJECT gate는 “이름 충돌”이 아니라 실측한 hazard set으로 잘라라**—collision만 보면 correct 설계를 대량 loud화(11건). (4) **리뷰어의 제안 gate도 실측 검증 대상**(제안한 두 술어 모두 반례 존재·내 gate가 더 정확). (5) 슬라이스 축소가 정답일 때가 있다—param class를 포기하고 cleanly-verifiable subset(string-array)만 지원. 신규 `scope_shadow_sidemap.rs`×15(S1 회귀 가드 2개 포함). **4372→4387 green**·clippy/fmt clean·format 23 불변.

#### 4.5.217 SILENT-WRONG 수정: string-ARRAY ELEMENT가 concat/replicate/relational에서 packed로 새던 문제 → supported (AST 문자열-도메인 분류기 BitSelect arm·format 23 불변) (2026-07-25, branch feat-string-array-elem-domain) ✅

**발굴 경위**: ROADMAP §3 소형 큐의 loud→supported 후보(FIXED string array decl-init `string s[2]='{"a","b"}`)를 그라운딩하다, fixed 경로와 dyn 경로의 capability parity를 같은 문맥 매트릭스로 비교하는 probe에서 **두 경로 모두** `{s[0],s[1]}`을 빈 문자열로 렌더하는 것을 관찰. scalar string은 정상(`{a,b}`=abcb) → 배열 ELEMENT 전용 갭. iverilog가 fixed string array를 지원하므로 **live oracle 有**.

**silent-wrong 6형(iverilog 차분 확정)**: `string s[2]; s[0]="abc"; s[1]="b";`

- `{s[0],s[1]}` → vita `""` vs iverilog `abcb` · `{2{s[0]}}` → `""` vs `abcabc` · `{s[0],"!"}` → `!` vs `abc!`
- `r={s[0],"-",s[1]}` → `b`(len 2) vs `abc-b`(len 5) · `s[0]<s[1]` → 0 vs 1 · `s[0]>s[1]`/`<=` → 1/0 vs 0/1
- dyn 배열 runtime index `{d[i],"!"}` → `" !"` vs `cd!`

**근본 원인**: `expr_is_string_ast`(elaborate 공유 AST-레벨 문자열-도메인 분류기)의 match에 **인덱스 표현식 arm이 부재**(`Ident`/`Call`/`Paren`만)→`sa[i]`가 `_ => false`로 떨어짐. 이 분류기를 게이트로 쓰는 **모든** consumer가 element를 PACKED 경로로 lower: `Concat`(expr_main 747)·`Replicate`(764)·relational/equality(384)·`string_concat_special`(strings 415/421)·context-width Concat(expr_ctx 594). packed 경로에서 String-kind net은 width 0이라 NUL 바이트만 기여→leading-NUL strip 후 빈 문자열. **storage는 정상**(`%s`·`.len()`·`.substr()`·task arg·`q=s[0]` 전부 동작)→**access 라우팅(shallow) 갭**(LOOPROMPT §2 "element-select silent-wrong=WHOLE-value 연산 먼저 측정" 규칙대로 진단).

**fix(format 23 불변·elaborate-local)**: (1) `expr_is_string_ast`에 `BitSelect { base, .. } => is_string_array_elem_base(base)` arm. (2) 신규 `&self` 헬퍼 — base가 single-segment Ident이면서 **FIXED** string array(`string_array_elems`) OR **DYNAMIC** string array(net이 `string_elem_dyn_nets`)일 때만 true. (3) **empty-container early-return**(두 컨테이너 모두 비면 즉시 false)→string array 없는 디자인은 scope walk 0회·byte-identical이 prose가 아닌 기계적 보장.

**disjointness**: SCALAR string의 byte select `str[i]`(§6.16.2·8-bit integral)는 packed로 남아야 하는데, fixed string array는 배열 이름으로 net을 등록하지 않고(per-element `<name>$sae$<i>`만) dynamic은 `DynArray` net이라 둘 다 `NetKind::String` handle이 아님→`string_handle`/`string_base_expr_net` 도달 불가.

**★ adversarial review(2-lens 병렬)가 신규 silent-wrong 1건 포착**: soundness lens가 **분류기와 lowering의 resolver 불일치** 발견 — dyn clause를 `lookup_net_scoped`로 resolve했으나 같은 `base[i]`의 **lowering은 `dyn_handle_read`**(=`dyn_subst` ALIAS를 먼저 consult·R2-inline body에서 dyn-array formal이 outer same-named net을 SHADOW). 모듈 레벨 `string b[]`가 inline된 `int b[]` formal의 `b[0]<b[1]`을 **텍스트 비교**로 만들어 `256<255`가 1(iverilog 0). **fix**=dyn clause를 `dyn_handle_read`로 교체→**각 clause가 자기 lowering과 동일 machinery로 resolve**→분류기가 자신이 분류하는 표현식과 불일치 불가(re-audit이 "AST classifier가 이제 IR twin(`ir_expr_is_string`)의 정확한 AST-레벨 projection"이라 판정). **주의**: 리뷰어 원본 repro는 재현 안 됨(`task automatic`=framed→`dyn_subst` 미사용·`%s`가 두 라우팅을 동일 렌더)—**inline 경로 + 텍스트/수치 비교가 갈리는 값**이 필요했음(측정 검증의 가치).

**differential lens**: PRE(pre-fix 빌드)/POST/iverilog 3-way를 생성 매트릭스 900+행(compare 100×18·concat 64×8·dyn 36×8)+수작업 ~75 케이스에 적용→**regression 0·신규 divergence 0**·잔여 divergence는 전부 main에서 byte-for-byte 재현. 최종 verdict 양 lens **CLEAN**.

**측정으로 기각된 리뷰 findings**(이론→측정 필수 원칙): frame-local string array가 `.getc` byte-read로 silent하다는 지적=**LOUD**(static task E3018·function/automatic E3009)→미지원 기능(안전)이지 silent 아님 · context-width Replicate ungated=**no-oracle**(iverilog가 string concat→packed target 대입 자체를 거부)·vita 답은 IEEE-sensible.

**동반 수정(별도 커밋)**: byte select on an ELEMENT `s[0][0]`(pre-existing·PRE 빌드 동일)→vita 0 vs iverilog 119. `string_index_read`가 bare Ident base만 수용해 element base가 width-0 String signal의 packed bit-select로 샘. **fix**=`BitSelect` base 수용(두 gate 순서 유지: `expr_is_string_ast`가 lowering **전** 판정·`handle_is_str_readable`가 StrGetC-consumable만). dyn element는 word-indexed라 gate 미통과→불변(follow-on·iverilog도 거부). **write twin은 loud 유지**(`s[0][0]="W"`=nested lvalue select)→read/write divergence 없음.

**테스트**: 신규 `string_array_elem_domain.rs`×24. 핵심 설계=`both()` 하네스가 **fixed-array 형태와 scalar-string 형태를 둘 다 실행해 동일 출력을 요구**(oracle 값 + vita-내부 등가 차분 동시 pin). 경계 pin=scalar byte-select packed 유지·정수 fixed/dyn 배열 packed 유지·runtime-index fixed array loud 유지·inline dyn-formal shadowing 수치 비교 유지. **4348→4372 green**·clippy/fmt clean·format 23 불변(elaborate-local·IR shape 무변경)·staged `vcmp→velab→vrun` 패리티 확인.

**교훈**: (1) **loud→supported 후보를 그라운딩하다 같은 자료구조의 silent-wrong을 발굴** — capability-parity probe(형제 경로 A vs B를 같은 문맥 매트릭스로 비교)가 두 경로 공통 버그를 노출. (2) **AST-레벨 분류기의 누락 arm 1개 = consumer 전체에 동형 silent-wrong 복제**(1 arm 부재 = 5 consumer × 6 구문형)·역으로 1 arm 추가로 전부 해소. (3) **분류기는 자신이 분류하는 표현식의 lowering과 동일 resolver를 써야 한다** — 다른 resolver를 쓰면 shadowing에서 조용히 갈린다(F1). (4) **리뷰어 이론은 측정 후 채택**: 4개 finding 중 1개만 재현(그마저 repro는 틀렸고 메커니즘만 옳았음)·2개는 loud/no-oracle로 기각. (5) 기존 N6 테스트가 `==`와 `.len()`만 검증해 이 family가 숨었음 — **동치가 우연히 맞는 연산(equal-length equality)만 테스트하면 분류기 갭이 은닉**. (6) staged 바이너리는 `--features separate-bins`라 stale하기 쉬움(미빌드 시 fix 전 동작 재현→오진 유발).

#### 4.5.216 round-19 follow-on: F-record-out short-circuit output-formal call — if-cond + ?:/general-expr 확장 (adversarial review가 ternary sign/width silent-wrong 포착·format 23 불변) (2026-07-24~25, branch round19-followons) ✅

**컨텍스트**: §4.5.215 F-record-out가 while/for LOOP condition의 short-circuit `&&`/`||` output-formal 호출을 지원. 오너 "잔여 follow-on 구현·correct-support가 핵심". 문서화된 follow-on(if-cond·`?:`·general-expr) 구현. **외부 오라클**: iverilog/verilator 모두 function output port 거부→hand-IEEE(§11.4.7 short-circuit·§11.4.11/§11.8.1 arm unification·§13.5.2 copy-out)+call-free bare form 内부 differential.

- **if-cond**(`9a4c2f8`): `lower_shortcircuit_loop_cond`이 실은 loop-agnostic(short-circuit cond를 두 target block으로 라우팅)임을 관찰→`lower_shortcircuit_cond`(true_bb/false_bb)로 rename·If arm(then_bb/else_bb)에 재사용. guarded call copy-out은 taken path에서만 발화(gate=0→미호출·`||` done=1→미호출 검증). 기존 loud test(r5b `if(g && f(x))`) legit-flip(g=0→f 미호출→x=0 검증·silence 아님). **while/for/if 全 condition position 커버 완료**.
- **?:/general-expr**(`d719da5` + fix `ac95d59`): `shortcircuit_rhs_special` intercept(Blocking special-rhs chain·hoist.rs)가 whole-rhs `?:` arm / top-level `&&`/`||` output-formal 호출을 explicit control flow로 lower(각 path서 lhs 대입). `&&`/`||`=inherently 1-bit bool(4-state X left operand 올바름·call 여전히 발화·§11.4.7). `?:`=3-way(def-true→THEN·def-false→ELSE·c=X→both arms 평가+real `Ternary` node로 engine `merge_x`). **★ adversarial review가 CRITICAL silent-wrong 포착**: ternary DEFINITE-arm을 isolated `x=T`(plain blocking assign)로 lower해 sibling arm의 width/sign unification(§11.4.11/§11.8.1) 소실→`c ? signed[3:0] : unsigned[7:0]`(lhs≥8)이 §11.8.1(mixed→unsigned) zero-extend(0x0A·iverilog 확인)해야 하는데 silent sign-extend(0xFA). implementer의 within-vita differential이 narrow-lhs(divergent bit가 truncate됨)라 miss. **fix**=coercion-safety gate(양 arm same effective signedness AND lhs width ≥ max arm self-width일 때만 transform·아니면 fall-through→loud). same-type common case(flipped `?step(n,calls):0`, 양 signed int, lhs int) 지원 유지·divergent(sign/width mismatch)는 loud.
- **Q string-member hardening**(`cbac94f`): §4.5.215 whole-branch review Minor(Q string/real by-value가 reduction으로만 검증)→direct test 2개(string-member deep-copy `q=p; p.s="changed"`→q.s 불변·string-member NBA `q<=p`). 통과(값 직접 확인).

**correct-or-loud 잔여(LOUD)**: coercion-unsafe ternary(sign/width mismatch)·call ONLY in ternary cond(`f(out r)?T:E`)·NBA-rhs/intra-delay-rhs·part-select/concat/hier lhs of ternary(`ternary_lhs_width`가 whole-net Ident만)·buried(`(A&&f())+1`)·deeper-nested operand(`A&&(B||call)`). 별도 dedicated slice 대상(low-ROI/deep): F-record-out nested-in-operand·F-struct call-index(`kats[nxt()]`)·BL assoc/multidim block-local·BL under-fork non-const(§4.5.214-scale module-process per-activation arena).

**format 23 불변**(elaborate CFG lowering·reuses TaskCallInfo copy-out·diff는 elaborate+cli test만·no sim-ir/header/schema). **4333→4348 green**(+15). **교훈**: (1) value-producing conditional-expr lowering은 arm width/sign unification이 silent-wrong 원천(isolated per-arm coercion ≠ unified ternary context). (2) adversarial review **5회째** silent-wrong 포착(round-19 4건+이번 ternary)·implementer의 self-differential이 narrow-lhs corner를 truncate로 miss→independent review가 wider-lhs로 재현. (3) correct-or-loud gate(coercion-safe subset 지원+나머지 loud)가 안전한 merge 선택(support-preserving unified-context lowering은 riskier follow-on). (4) `lower_shortcircuit_cond` rename이 loop/if 공유. 상세=이 항목.

#### 4.5.215 round-19 리포트 4-가족(BL·Q·F-struct·F-record-out) loud→correct-support (리포트 2-오진 정정·adversarial review가 silent-wrong 4건 포착·format 23 불변) (2026-07-24, branch round19-families) ✅

**컨텍스트**: 외부 리뷰어 round-19 리포트(base §4.5.213/214·`5dd897b`)의 잔여 7-가족+m.name. round-18 8-가족은 §4.5.213서 이미 해소. brainstorm→spec(`docs/superpowers/specs/2026-07-24-round19-families-design.md`)→plan(`docs/superpowers/plans/2026-07-24-round19-families.md`)→subagent-driven(가족별 implementer + adversarial review·전체-브랜치 최종 review). **iverilog·verilator 모두 이 구문 거부**(`sorry: Overriding the default variable lifetime`·`Unpacked structs not supported`)→**외부 오라클 無**→hand-IEEE(§6.21 lifetime·§13.5.1/2 pass-by-value) + 통과 boundary 교차검증.

**fresh-probe 트리아지가 리포트 2-오진 정정**: (1) **Q** — 리포트의 "member-width param-aware로 만들라"가 **UNSOUND**(parse-time frozen width→`#(.ADDR_W())` override 시 silent-wrong·`$bits`=8 vs 48 per-instance 실측). 실제 갭=whole-record scalar copy 뿐(net/queue/`size()`/`pop_front()`/field access 다 이미 동작). (2) **F-record-out** — "string 멤버 ≥2" 경계 **존재 안 함**(리포터 두 파일 다 동일 실패·2-string copy-out 값-정확). 실제 트리거=short-circuit `&&`/`||` RHS의 output-formal 호출(멤버 수 무관·records 문제 아닌 hoist 문제). (3) **m.name** — 별개 버그 아님·F-struct의 downstream 증상(`run_test`가 struct-with-string-member input formal→binding 실패로 `.name()` 미도달)→F-struct 고치면 자동 해소(검증).

**6-가족(BL 4-face + Q + F-struct + F-record-out)**:
- **BL1**(`9b6adf8`) const-init block-local under a fork: init이 상수 fold + never-reassigned면 concurrency-immune(모든 activation이 shared flattened net서 같은 상수 read)→loud gate skip. `stmt_never_assigns_ident` write-scan(da.rs).
- **BL2/BL3**(`6b19a8c`) dyn-storage block-local `'{…}` init: `compute_per_entry_block_locals`가 dyn-storage 기록·`emit_per_entry_block_inits`가 block-entry서 `dyn_decl_init_stmts`(`new[N]`+elem writes) 재방출. BL3 same-name은 기존 `$blk$` distinct-net scoping 재사용(coalesce guard 불변).
- **BL4**(`a714bb8`) output/inout-actual write를 definite-assignment이 인식: `da::OutActualWrites` resolver(func/task port dir)·unconditionally-evaluated output-actual=definite assignment. inout-first-ref는 copy-in이 actual을 read→flatten leftover≠fresh automatic이라 genuine divergence→LOUD 유지(sound deviation).
- **Q**(`0e9858b`) non-packable record whole-scalar copy: `try_soa_assign`(parser soa.rs)에 same-type-name gate branch·per-member fan-out(field↔field by NAME). mixed-state/string-member/param-width 커버·기존 2 stays-loud test flip(값 검증 후).
- **F-struct**(`4fc5f4d`) SoA-record-array element actual: `expand_soa_array_elem_arg`가 `arr[i]`→per-member `$unp$arr$field[i]`·formal side와 same-list order(aligned by construction·all-or-nothing). m.name 해소.
- **F-record-out**(`8bc1d27`) short-circuit loop-cond output-formal 호출: `lower_while`/`lower_for`가 top-level `A&&B`/`A||B`를 explicit branch chain으로 lower·copy-out Call을 `eval_b`(non-shortcircuit edge서만 도달)에 방출→call이 skip path서 절대 안 fire.

**★ adversarial review가 silent-wrong 4건 포착(3 latent·solo-pass면 ship됐을 것)**: (a) **BL1-fix**(`d243058`) write-scanner가 output/inout call in expr + assert action 놓침(under-detecting `expr_reads_ident`)→`_`-free-exhaustive walker(`expr_call_may_write_ident`)로. (b) **BL4-fix**(`d62b506`) reads-check가 same-call member/method read(`f(obj, obj.v)`) 놓침(같은 under-detecting walker)→conservative `expr_no_ref`. (c) **F-struct-fix**(`c7924fb`) array index를 per-member로 clone→side-effecting index(`kats[nxt()]`) N회 평가=torn record read→call-bearing index reject(`expr_has_call`). (d) **harden**(`f0142f6`) `expr_has_call`을 `_`-free-exhaustive화(whole-branch review Minor·defense-in-depth). Q review·F-record-out review·whole-branch review(7 probe)는 전부 CLEAN.

**correct-or-loud 잔여(LOUD)**: BL under-fork non-const/reassigned·assoc·multi-dim dyn·BL4 inout-first-ref·Q cross-type copy·F-struct call-bearing index·F-record-out `?:`arm/if-cond/general-expr/nested-in-operand(`A&&(B||call)`). F-record-out `(A&&B)&&call`은 correct-by-construction이라 supported(review 판정).

**format 23 불변**: 전부 parser-desugar(`.vu` AST hash만)·elaborate-transient·CFG-lowering·message-only. `CURRENT_FORMAT_VERSION`·sim-ir/header/schema/hdl-ast/vita-artifact 전부 미변경(whole-branch review `git diff` 확인). **4262→4333 green**(+71).

**교훈**: (1) **외부 리포트도 fresh-probe 재트리아지 필수**(2-오진 정정·리포트대로 구현했으면 Q는 override-silent-wrong 도입). (2) **accept-gate의 under-detecting walker(`expr_reads_ident`)가 반복 silent-wrong 원천**·conservative(`expr_no_ref`)/`_`-free-exhaustive가 정답(BL1/BL4/F-struct 3회 동형 발현). (3) **가족별+전체-브랜치 2단 adversarial review가 silent-wrong 4건 포착**(3 latent·§4.5.214 "whole-branch review 필수" 재확인). (4) "context-bound 격리 불가" 리포트 항목(m.name)이 실은 인접 가족(F-struct)의 downstream 증상. 신규 test 8파일 + 71.

#### 4.5.214 fork…join[_any|_none] inside a suspendable task body loud→supported (Case A/B split + interior-mutable window arena) (2026-07-24, branch fork-in-frame) ✅

**컨텍스트**: §4.5.213(C1 part 2)이 "`fork a(); b(); join`을 suspendable task 내부에서 실행하는 것은 깊은 스케줄러 rework(shared frame-window model)라 blast radius=frame 서브시스템 전체"라 판단해 correct-or-loud LOUD 유지·ROADMAP §0 NEXT로 남긴 항목. 브레인스토밍→설계(`docs/superpowers/specs/2026-07-24-fork-in-frame-design.md`, approved)→구현 계획(`docs/superpowers/plans/2026-07-24-fork-in-frame.md`, staged 1→2→3)→4-task subagent-driven 구현(+ 커밋별 리뷰 + 전체-브랜치 최종 리뷰)으로 진행. iverilog가 리포트 repro(`fork a(); b(); join` of separate no-arg tasks)를 실제 실행하는 real oracle.

**핵심 재발견(§0 NEXT의 우려가 과보수적이었음)**: `exec_fork`(propagate.rs)가 top-level process를 전제한다는 진단은 맞았으나, frame window 모델을 다시 읽으니 **스케줄러가 단일-스레드**임이 결정적이었다 — 정확히 하나의 activity만 임의 순간에 실행되고, `stash_frame_windows`/`restore_frame_windows`가 **실행 중인 activity의 window만** `frame_stack`에 두도록 보장한다. 그래서 concurrent fork children과 parked parent는 **결코 동시에 stack에 공존하지 않는다** — 교대로 실행될 뿐이다. "single-owner move-based window라 concurrent children 불가"라는 최초 framing은 **parent가 parked인 동안 child가 parent window에 도달해야 하는 경우에만** 진짜 문제이지, 전면적인 문제가 아니었다.

**Case A/B split(설계의 핵심)**: 모든 fork arm이 *self-contained*(부모 task의 frame-local range `[base_net, base_net+locals_len)`의 어떤 net도 read/write 안 함·called task는 자기 frame을 소유하므로 하강 안 함) = **Case A** → **기존 owned-window 모델**(신규 인프라 0)로 즉시 동작 — 리포트 repro `fork a(); b(); join`(별개 no-arg task 2개)이 정확히 Case A. 적어도 하나의 arm이 부모 frame-local을 read/write = **Case B** → parent가 parked인 동안 그 window가 필요하므로 **shared, interior-mutable arena**로 전환해야 함. 경계는 elaborate에서 결정 가능(각 arm의 reachable block을 걸어 `[lo,hi)` net 참조를 검사·called task 내부는 하강 안 함).

**3-stage 전달(각 단계 독립 검증 가능·미도달 stage는 항상 LOUD — 부분 전달도 항상 correct-or-loud)**:

- **Stage 1(Case A, commit `609bc1b` + review-fix `06bb53a`/`6f003c5` + regression-fix `13e947d` + breadth `1ab8f66`)**: elaborate 신규 `frames_classify_fork.rs`(review Finding 2로 1155줄 파일에서 분리)의 `enum ForkAdmit{CaseA,CaseB,Loud}` + `fork_arms_self_contained`/`classify_one_arm`(각 arm subtree를 `join_bb` sentinel까지 걸으며 BlockingAssign/NonblockingAssign/SysTask/Branch/Wait/Delay-amount/lvalue-chunk-index/Call in·out-bind의 `[lo,hi)` 참조를 conservative하게 CaseB로, nested Fork/`disable fork`/미인식 stmt/누락 bind-table을 Loud로 판정). 엔진: `exec_fork`(propagate.rs)가 in-frame 부모를 감지해 각 child를 synthetic arm `FrameRec`(callee=arm task, bb=arm entry, ret_bb=join, window=`Owned(empty)` 또는 `None`[static enclosing task])로 spawn·`exec/process.rs`의 loop-top intercept가 in-frame child 완료를 `call_stack.len()==1 && bb==join_bb`로 gate·신규 `FRAME_FORK_KEY`(u32::MAX)가 task-body fork의 join-mode를 **globally-unique** `join_bb`로 키잉(process-keyed가 아님 — 어느 process가 그 task를 실행하든 동일해야 함). **latent root-cause 버그 fix**: `rebase_terminator`의 `Terminator::Fork` arm이 `join`/`resume_bb`는 rebase하면서 **`children`을 빼먹은** 버그(모든 in-frame fork가 이전엔 loud라 잠들어 있었음 — 그대로 뒀으면 `PASS @15`가 `@25` 대신 나오는 mis-timing + 단일-arm false-Loud)를 발견해 수정. **2건의 review-fix**: **Finding 1**(classify_one_arm이 Delay-amount/lvalue-chunk-index/Call-out-bind의 프레임로컬 참조를 놓쳐 CaseA로 오분류 → automatic task는 uncontrolled panic[`frame_eval.rs:80` index-out-of-bounds], static task는 **SILENT-RUN**[정적 슬랩을 조용히 오염] — 3곳 수정). **Finding 2**(파일이 1000줄 cap을 넘겨 `frames_classify_fork.rs`로 분리). 이후 **backstop 회귀 발견+수정**: Finding-1 수정이 `frame_task_has_unsafe_construct`라는 **전체 suspendable task에 무조건 도는 일반 backstop**에도 Delay-amount 체크를 잘못 미러해, fork와 무관한 평범한 variable-delay task(`#(d)`)까지 false-E3009 — 즉시 되돌리고 정확한 주석("이 hazard는 `classify_one_arm`이 이미 정밀하게 잡는다")으로 교체. **breadth**(`1ab8f66`): `join_none`·연속 2회 fork(상태 leak 없음 확인)·`wait fork`(**진짜 gap 발견** — 이전엔 컴파일-타임 E3009가 아니라 오해를 부르는 runtime `VITA-F4016`[delta-limit exceeded] fatal이었음 → elaborate에서 clean E3009로 전환)·`disable fork`(이미 loud, 회귀-lock만 추가).
- **Stage 2(Case B `join`-all, commit `1ce524c`)**: `enum WindowSlot{Owned(Vec<Value>), Shared(u32)}`을 `frame_stack`에 도입(Owned 경로 byte-identical — 기존 모든 frame/task/function이 이 분기만 탐)·신규 arena `frame_windows: RefCell<Vec<Option<Vec<Value>>>>` + free-list(dyn_heap/class_heap과 동일 interior-mutable 패턴)·`FuncMeta.contains_shared_fork: bool`(`#[serde(default)]` — `has_hier_call`과 동일한 staged-trailer sidecar 패턴·**format 23 불변**)이 `enter_task_frame`서 arena window 할당을 트리거. join-all이므로 **모든 child가 parent 재개 전에 완료** → parent가 자기 `Return`에서 arena를 직접 free(refcount 불필요, Stage 3에서 추가). `fork_arms_self_contained`가 join-mode gate를 획득(Case B는 `JoinMode::All`일 때만 admit·`join_any`/`join_none`은 Stage 3까지 Loud).
- **Stage 3(Case B `join_any`/`join_none`, commit `c138329`)**: surviving child(join_any의 surplus, join_none의 detached child)가 parent보다 오래 살 수 있음 → arena slot에 **`frame_window_rc: RefCell<Vec<u32>>`** refcount(불변식: `alloc 1 + N retain(스폰된 Case-B child당 1회) = N release(child 완료) + 1 release(parent Return)` → rc가 0에 도달하는 순간 free, parent 또는 마지막 child 중 나중). `join`(Stage 2)은 byte-identical하게 여전히 parent `Return`에서 free(모든 child가 먼저 release해 rc가 거기서 0에 도달하므로). `debug_assert!(rc>0)`를 모든 arena 접근·release 전·retain 전에 배치(use-after-free/underflow가 나면 즉시 panic — 실제 rc soundness 테스트 2건이 이 assert 활성 상태로 통과). `fork_arms_self_contained`가 join-mode gate를 제거해 Case B가 모든 join 모드에서 admit.
- **final-review fix(silent-wrong 회귀 차단, commit `30410cd`)**: 전체-브랜치 리뷰가 fork arm 내부의 `return`이 (fork-lowering이 `return`을 `goto(exit_bb)`로 lower하는데 그 block이 `Terminator::Return` 소유) **admit되어** 런타임에서 in-frame `Return` 핸들러가 (`exec_fork`가 `enter_task_frame`을 우회해 spawn한) 합성 arm frame에 대해 parked **parent 자신의 FuncId**로 `exit_task_frame`+`frame_dyn_free`를 실행 — parent의 frame scope를 parked 상태에서 pop하고 그 dyn-array local을 free하는 **silent frame-corruption**(exit code 0, 값이 조용히 깨짐: 정답 `q.size=3 q0=77 q1=88` 대신 `q.size=0 q0=x q1=x`)이 admit되고 있음을 발견 — loud→silent-wrong 회귀 중 최악 등급. `classify_one_arm`의 `Terminator::Return => {}` 한 줄을 `=> return ForkAdmit::Loud`로 바꾸는 것으로 즉시 E3009화(iverilog도 fork-join 내부 `return`을 컴파일 거부 — 오라클이 loud를 확인). 정상 arm은 항상 `goto(join_bb)`로 끝나 이 arm에 걸리지 않음(false-reject 0 — 기존 24개 fork 테스트 全 통과). 별도의 `fork_arms_self_contained`(whole-body fork-finder walk)의 `Return => {}` arm은 의도적으로 안 건드림(그건 task 자기 body의 끝을 가리킴).

**적대 검증(2-lens)**: **iverilog 13.0이 실제 실행하는 모든 형태**(join/join_any/join_none·비대칭 arm 타이밍·인라인-block arm·**Case-B 공유 변수 가시성**[arm write→parent post-join read]·sibling이 다른 arm의 write를 봄·arm이 부모 local을 task-call 인자로 전달·arm `#(d)` delay-amount가 부모 local 참조·arm이 `mem[d]` 인덱스로 부모 local 사용·**두 concurrent Case-B 활성화의 arena 격리**·동일 Case-B task 2회 호출[arena free+realloc]·Case-A→Case-B 혼합 fork) 全 MATCH. **iverilog가 크래시하는 경우**(surviving/detached fork child — `Assertion failed: (child->wt_context==0 || thr->wt_context!=child->wt_context), function of_JOIN_DETACH, file vthread.cc, line 3793`, join_none/join_any surplus 자체가 건드리는 iverilog의 알려진 버그)는 **hand-IEEE(§9.3.2 automatic lifetime)**로 검증 — `join_any` 2-surplus 순차 완료(rc 4→3→2→1→0)·`join_none` loop에서 매 호출이 별개 arena handle(별칭 없음, 3개 distinct 값으로 실증)·detached child가 parent 리턴 한참 후(`#20`)에도 부모 local을 정확히 읽음. rc soundness는 debug-assert 활성 상태로 모든 테스트가 panic 없이 통과.

**correct-or-loud 잔여(전부 검증된 LOUD, silent 아님)**: nested fork(fork arm 내부의 또 다른 fork — 기존 `in_fork` 가드)·frame body의 `wait fork`/`disable fork`(Phase-4 follow-on, 후자는 이전엔 misleading runtime F4016이었으나 이번에 clean E3009로 전환)·**fork arm 내부 `return`**(final-review catch)·static(non-`automatic`) task의 Case B(정적 slab을 재귀 활성화가 공유하는 clobber 위험 — arena는 automatic-only)·fork하는 task를 호출하는 fork arm(`fork inner(); join` where `inner`가 자체 fork — child의 `parent_tie`가 항상 `≥65536>0xFFFF`라 tie-encoding cap `F4004`가 항상 발화해 안전하지만, elaborate-time에 명확히 잡는 cleaner reject는 follow-on).

**게이트**: `cargo test --workspace --locked` **4236→4262 green**(+26 = Stage1 vertical-slice+review-fix 12[7+3+0+2]·breadth 5·Stage2 arena 3·Stage3 refcount 4·final-review return-fix 2)·`cargo clippy --workspace --all-targets --locked -- -D warnings` clean·`cargo fmt --all -- --check` clean·`format_version`(header.rs) **23 불변**(diff 확인: `crates/sim-ir/`·`crates/vita-schema/`·`crates/diag/`·header.rs 전부 미변경 — 전부 런타임 엔진 타입[`WindowSlot`/`frame_windows`/`frame_window_rc`] + `FuncMeta.contains_shared_fork` 사이드카뿐이라 SimIr golden root 무변화).

**교훈**: (1) **"single-owner move-based frame window라 concurrent children 불가"라는 최초 framing이 과보수적**이었다 — 단일-스레드 스케줄러 + 기존 stash/restore가 이미 Case A를 공짜로 해결하고, 진짜 새 인프라가 필요한 건 parked-window-cross-sharing(Case B)뿐. (2) **interior-mutable arena(dyn_heap→RefCell·class_heap과 동형 패턴)가 shared frame state의 반복되는 정답**이고, refcount가 join_none/join_any의 survivor lifetime을 해결하는 자연스러운 확장. (3) **2-stage 구현 + 전체-브랜치 리뷰가 솔로 패스였다면 그대로 배포됐을 실제 이슈 3건**을 잡았다 — classifier의 correct-or-loud gap(Delay-amount/lvalue-index/Call-out-bind 프레임로컬 참조 누락 → panic 또는 silent-run), fix-유발 회귀(변수-delay task가 fork와 무관하게 false-loud), 그리고 **전체-브랜치에서만 보이는 `return`-in-arm silent-wrong**(elaborate classifier와 엔진 Return 핸들링에 걸쳐 있어 개별 task 단위 리뷰로는 안 보임). (4) **Owned-path-byte-identical 규율**이 hot-path 타입 변경(`Vec<Value>`→`WindowSlot`)을 안전하게 만든 핵심 축 — 모든 기존 frame/task/function 테스트가 그대로 통과. 신규 test 파일(`crates/cli/tests/fork_in_frame.rs`, 24개 fork 시나리오 + `suspendable_tasks.rs`/`suspendable_const_repeat.rs` 확장)×26·format 23 불변. **ROADMAP §0 NEXT 완전 소진**(round-18 리포트 8-가족 + C1 part1/2 全 RESOLVED — 남은 것은 소형 follow-on(redundant classifier re-walk·`enter_task_frame` 공유 arm 주석 보강·forking-task-calling-arm의 elaborate-time reject·same-instant zero-delay sibling visibility 미검증)뿐).

#### 4.5.213 round-18 리포트 대응: 8-가족 loud→supported + C1 const-repeat (2026-07-24, branch feat-round18-report) ✅

**컨텍스트**: 외부 리뷰어 round-18 리포트(base `6cf1fd8`=§4.5.212)가 지목한 잔여 8-가족(A/G·C1·D·E1·E2·F1·F2·F3)을 fresh-probe로 재현·트리아지(iverilog 오라클 有=차분·거부=hand-IEEE) 후 순차 구현. **오너 지시: "리포트된 모든 내용을 수정 & 검증·defer 없이 무조건 correct-support·silent issue 즉시 수정"**. 전 파이프라인 green(**4178→<count> green·format 23 불변**). 조사 3-agent(A/G 파서 record-array·C1 fork·D block-local) 병렬.

- **F2 (severity in frame body)**: `unique case`의 no-match arm이 합성 `$warning`(SysTaskCall→Display+`severities` sidecar)인데 `classify_frame_body`가 `frame_print_stmts`만 허용→pure `unique case` 함수를 통째로 loud(false-positive·arm 미실행이어도). **fix**: `had_error: bool`→`Cell<bool>`(4 site·`dyn_heap` RefCell 선례)·신규 `&self` `frame_emit_severity`(sink.emit diag·$error→`had_error.set`·$fatal→`call_fatal` latch[스케줄러가 error finish로 전환])·run_frame_call/run_task에 severity render arm(plain Display print arm 앞·안 그러면 stdout로 샘)·classify에 `severities.contains(&sid)` 허용 arm. **iverilog 오라클 全 MATCH**: `$error`(exit 1)·`$fatal`(abort·후속 stmt 미실행)·`$warning`/`$info`(continue)·unique-case 위반 warning 실발화(W4007 r=0). 신규 `severity_in_frame_body.rs`×9·flip 1(§4.5.196 `_stays_loud`→`_now_supported`).
- **E1 (enum method on tf-port formal)**: `.name()`/`.next()`/`.first`가 파서 desugar(`enum_method_expr`)인데 `var_enum` gate가 tf-port 형식인수 미등록→`m.name()`이 generic Call로 새어 hier-call loud. **fix(파서-only·format 불변)**: `TfPortType` 4-tuple→5-tuple(enum-type-name 추가)·`parse_tf_port`가 `try_tf_port_typedef` 前 `enum_defs.contains_key(type_name_key())` 감지→port name을 `var_enum` 등록(snapshot/restore가 이미 `var_enum` 포함→tf-scope 격리)·bare continuation `input e_t a,b`는 inherited_type로 전파. **iverilog 오라클 全 MATCH**: fn/task formal·multi-continuation·`.next()`. 신규 `enum_method_formal.rs`×7.
- **E2 (method on struct member + string dyn element)**: (a) `r.name.substr(8,15)`(struct string-member)가 3-seg hier-call로 파싱→loud. **fix**: 파서 `unpacked_member_method_recv`가 3-seg `var.field.method`의 receiver `r.name`→member net `$unp$r$name`로 재작성→`$unp$r$name.substr` 2-seg(elaborate가 이미 처리). (b) 배열 site `kats[i].name.substr`은 `$unp$kats$name[i]`(string dyn/queue ELEMENT)인데 `ir_expr_is_string`이 whole-string `word:None`만 인식→chain loud(plain `sa[i].method()`도 동일 pre-existing 갭). **fix**: `ir_expr_is_string`에 `Signal{net, word:Some}` && `string_elem_dyn_nets.contains(net)` arm(엔진 `handle_str_bytes`가 eval fallback로 element read·`%s`/compare와 동일 경로). hand-IEEE(iverilog unpacked struct 거부)·bare string-var/dyn 교차검증. 신규 `struct_member_method.rs`×5·record_array_soa에 array-site 포함.
- **A/G (queue/array of non-packable record + foreach)**: `pkt_t q[$]`의 멤버가 param-width(`[ADDR_W-1:0]`·`const_lit` 리터럴만 fold) 또는 mixed 2-/4-state면 `packable_record_layout=None`→queue/fixed 분기 SoA fallback 부재로 E2002 derail. **fix(파서-only·SoA)**: (A) queue/fixed 분기에 dyn 분기와 동일 SoA fallback(per-member `$unp$q$field[$]`/`[N]`·raw range→elaborate param-aware width·packed-vector 대신 SoA가 per-instance width 정답)·queue method fan-out `try_soa_queue_method_stmt`(push_back/push_front/insert/delete·all-or-loud[field desync 방지])·`try_soa_assign`에 `rec = q.pop_front()/pop_back()` per-field·`soa_rewrite_method_recv`를 read-only `size`/`num`만 field-0로(mutating은 fan-out). (G) `parse_foreach`가 SoA record array의 `.first`/`.next` iterator receiver를 field-0 net으로 재작성. hand-IEEE(iverilog unpacked struct 거부)·literal-width record 교차검증. **적대**: event-member record array=loud. 신규 `record_array_soa.rs`×13·flip 2(`fixed_record_array`·`round9_report_gaps`).
- **D (automatic block-local with init)**: 모듈 프로세스의 `automatic int lim = 20`(init)이 loud(§4.5.189 per-entry init은 `in_frame_body`-only·모듈 프로세스는 t0-once static 경로만). **fix(scoped_block_locals 패턴 미러)**: `compute_per_entry_block_locals`(block span.lo→qualifying names·**under_fork 추적**=concurrency guard·모듈 프로세스 loop는 sequential이라 single net이 §6.21 정답·fork만 동시 활성화)·Nets phase 2 reject site skip + t0-push skip·Logic phase `emit_per_entry_block_inits`(block entry에 per-entry init만). **hand-IEEE(iverilog가 automatic-lifetime override 거부)**: per-entry re-init(100,101,102 not ...103)·loop-var init(0,10,20)·always block·**fork 케이스 STAYS LOUD**(concurrency aliasing)·read-before-write(no init) loud. 신규 `block_local_automatic_init.rs`×8·flip 1(`round3_report_gaps`).
- **F1 (output-formal fn in loop condition)**: `while(getnext(i,v)==1)`(output formal fn in while cond)이 loud(hoist가 while-cond 미지원). 함수 output formal 자체는 R5-B copy-out로 이미 지원. **fix**: `lower_while`/`lower_for`의 `head` 블록(매 iteration 재진입)에서 `expr_has_inout_call && hoist_is_safe`면 `hoist_inout_calls`(copy-out temp emit)→condition은 temp read. hoist_stmt_top의 while-loud arm 제거. **hand-IEEE(iverilog가 function output port 거부)**: while/for cond·whole-cond·direct-rhs 교차검증. 신규 `output_formal_fn_in_loop_cond.rs`×5·flip 1(`round11_report_gaps`).
- **F3 (wrapped dyn-formal call)**: `reg_out <= en ? packk(b) : 64'd0`(NBA rhs + ternary arm)이 loud(§4.5.179 hoist=blocking rhs·unconditional position만). **fix**: (a) hoist_stmt_top에 NBA arm(direct+buried·`<=`는 §4.5.177 marker 없음→direct도 hoist). (b) Ternary를 hoistable화—단 `?:` arm은 conditional-eval이라 **pure 함수만**(impure=$display가 arm 미선택 시 spurious 발화). `dyn_formal_expr_all_pure`(body에 SysTaskCall/UserTaskCall 없음)·`stmt_has_observable_effect`. **iverilog 오라클 全 MATCH**: NBA-ternary(en=1/en=0)·NBA-direct·blocking-ternary·**impure fn in ternary STAYS LOUD**. 신규 `dyn_formal_wrapped_call.rs`×6·flip 1(`frame_func_dyn_formal_nested`).
- **C1 part 1 (const-repeat in suspendable task)**: `repeat(2) @(posedge clk)` in suspendable task가 loud(`ast_has_repeat_with_timing`이 count 무관 flag). `lower_repeat`이 const-small을 straight-unroll(shared counter 없음→suspend-safe)·non-const/large만 shared `$repeat_cnt$` net(진짜 hazard). **fix(1줄)**: `ast_has_repeat_with_timing`을 `const_eval_u32(count)<=REPEAT_UNROLL_CAP`면 unsafe 아님으로. **iverilog 오라클 全 MATCH**: direct(done=1 @15)·nested(PASS @25)·repeat(3)(cnt=3 @25)·**runtime `repeat(n)` STAYS LOUD**. 신규 `suspendable_const_repeat.rs`×5·flip 1(`suspendable_tasks` 근처).
- **C1 part 2 (fork in frame) = LOUD 유지(correct-or-loud·deep follow-on)**: `fork a(); b(); join` in suspendable task는 **깊은 스케줄러 rework 필요**라 loud 유지. 조사(exec_fork·frame window 모델 직독) 확인: fork spawner가 top-level process 전제(child empty call_stack·process-local block arena·top-level tie encoding·process-keyed fork_modes)·child-completion intercept `!in_frame` gate·frame window는 single-owner move-based(stash/restore). concurrent children이 부모 frame window 공유하는 shared-window 모델(Rc<RefCell> 등)로의 전환은 모든 framed task read/write 경로를 건드림(blast radius=frame 서브시스템 전체)→rush 시 silent-wrong 위험. correct-or-loud=clean loud 유지. **ROADMAP §0 NEXT = fork-in-frame(shared frame-window model)**.

**교훈**: (1) **"&self executor can't do X"류 blocker 재검증**(F2=`had_error`를 Cell로·§4.5.194 dyn_heap 선례). (2) **파서 desugar gate가 formal 미커버**(E1 var_enum·E2 member-method rewrite). (3) **SoA가 param-width/mixed record의 정답**(packed-vector는 parse-time width 필요→override silent-wrong·SoA는 per-instance elaborate width)·all-or-loud(field desync). (4) **shared-set 패턴(scoped_block_locals)이 Nets/Logic 2-phase 협조에 재사용**(D per_entry). (5) **hoist를 loop head/NBA/ternary로 확장·conditional position은 purity gate**(F3 impure=loud). (6) **const-repeat는 unroll-aware**(shared counter만 hazard). (7) **fork-in-frame은 deep-scheduler(shared window)·correct-or-loud로 loud 유지가 rush보다 안전**. 신규 test 8파일×58·flip 8. 상세 각 가족 위.

#### 4.5.212 SILENT-WRONG 수정: size-cast `N'(expr)` context-width 미전파 → supported (size-cast 전용 재귀 ctx-width lowering) (2026-07-23, branch feat-size-cast-context-width) ✅

**컨텍스트**: 오너 "가장 높은 우선순위 진행". ROADMAP §1 최우선=오라클 있는 CRITICAL silent-wrong. 큐 소진 후 **fresh-area iverilog 차분 probe**(§4.5.180/185 방식·~80 관용구 스윕)로 재선정→vita가 대부분 견고했으나 **size-cast `N'(expr)` context width 미전파**가 재현(문서화된 §2 DEEP 항목·`8'(a*b)`=13 vs iverilog 45·공통·高임팩트). size cast가 inner 산술을 operand self-width로 계산 후 result만 resize→carry 소실.

**ORACLE 있음(iverilog)**: size-cast width/sign은 iverilog가 정확→exhaustive 차분 가능(array-formal 슬라이스와 대조).

**"DEEP" 재평가**: §2는 "새 IR Resize 노드(format bump) 또는 fill-only ctx 머신을 26-caller에 확장(회귀 위험)"이라 defer했으나, **size-cast 전용 재귀 헬퍼로 격리하면 공유 `lower_ctx_or_plain`/`lower_expr_ctx` 미변경**→회귀 표면 없음. `lower_expr_ctx`가 이미 연산자별 ctx 전파 구조를 갖췄으나 **non-fill operand를 plain `lower_expr`(self-width)로 lower**하는 게 핵심 갭이었음.

**signedness 규칙 확정(iverilog 실측)**: `8'((signed -1 * signed 1)+unsigned 0)`=15(sign-ext면 255)·`8'(signed -3 * unsigned 5)`=65(sign-ext면 241)→**§11.8.1: 표현식에 unsigned operand 하나라도 있으면 전체 unsigned→모든 leaf(signed 포함) zero-extend**. all-signed면 sign-extend. 즉 **단일 top-level sign이 모든 context-det leaf에 균일 적용**(per-op mixed 처리 불필요).

**설계(format 23 불변·全 elaborate-transient·IR-0)**: (1) **`is_size_ctx_operation(e)`**—operand가 context-det 연산(arith/bitwise/shift/`**`·unary +/-/~·ternary)인지(bare leaf/select/concat/cmp=self-det→기존 경로 byte-identical). (2) **`ast_ctx_signed(e)->Option<bool>`**—전체 sign(arith=`&&` of operands·shift/pow=left·cmp/concat/select=unsigned·IntLit=`parse_int_literal().signed`·Ident=net.signed)·param/call leaf=None. (3) **`lower_size_ctx(e,n,ext)`**—context-det 연산자 재귀(arith=양 operand ctx-det·shift/pow=base ctx-det+amount self-det·unary±~=operand·ternary=branches·cond self-det), 각 self-det leaf는 **`lower_size_leaf`**로 N에 extend(`extend_to(_,ext)`)+sign 재-stamp(`ext`면 `$signed`·아니면 signed leaf에 `$unsigned`→signed division/ashr/comparison 의미 보존). (4) Size + Named-const cast arm 배선: `is_size_ctx_operation && ast_ctx_signed=Some(ext)`면 `lower_size_ctx`·아니면 기존 `lower_ctx_or_plain`(param/call leaf fallback=회귀 0).

**적대 differential 全 MATCH(iverilog·+23 tests)**: unsigned mul(45)/add-carry(10000)/shl(011110)/sub-borrow(11111)/mul16(fe01)/shift-var(0111100)/nested(33)/divmod(28 4)/power(27)·signed mul(-15)/ashl(-16)/division(-14 -2)/ashr(-16)/unary-neg(-3)·**mixed-sign(15·65·zero-ext all)**·ternary(45)/nested-cast(90)·narrowing(e)/leaf-unchanged(05 b)/cmp-1bit(1)/fill(11111111)/signed-add-no-overflow(-2). **랜덤 값 sweep 14/14 MATCH**(unsigned+signed 다양 폭). 전 workspace suite regression 0(leaf-operand size-cast는 self-det→byte-identical).

**교훈**: **"DEEP"이 "공유 인프라 변경 필요"를 의미할 때, 전용 재귀 헬퍼로 격리하면 회귀 표면 없이 tractable**(§4.5.211 "no-oracle≠not-verifiable"와 동형: prior defer 근거 재평가)·`lower_expr_ctx`의 연산자별 ctx 구조를 재사용하되 non-fill leaf widening만 추가·**signedness는 §11.8.1 top-level 균일 규칙(iverilog 실측으로 확정)이라 per-op mixed 불필요**·sign 재-stamp가 signed division/shift 의미 보존·param/call leaf fallback이 회귀 0 보장·iverilog 오라클로 exhaustive+random 차분 검증. 신규 `size_cast_context_width.rs`×23. **잔여 residual**: param/call leaf(`8'(P*a)`·`4'(f(x))`)는 fallback→self-width(sign 판정 불가·follow-on).

#### 4.5.211 NON-ZERO-BASE DESCENDING unpacked-array formal loud→supported (over-conservative reject removed) (2026-07-23, branch feat-nonzero-base-descending) ✅

**컨텍스트**: ROADMAP §0 재개 follow-on #3. 오너 질문 "해당 방안이 구현 리스크가 너무 커? 시도해 볼만한 요소가 없나?"—§4.5.206이 non-zero-base DESCENDING(`int m[4:1]`)을 "base+direction 상호작용이 mental model로 예측 불가·cleanly-verifiable 불가"라며 loud로 gated하고 "재개하려면 상용 simulator 오라클 확보 선행"이라 기록했음. 오너 push-back에 "blocked"로만 두지 않고 실제 tractability를 조사.

**핵심 발견(오라클 부재 ≠ 검증 불가)**: iverilog가 unpacked subroutine port를 거부(no direct formal oracle)하고 whole-unpacked-array copy(`b=a`)도 거부하지만, **IEEE §13.5.1이 same-declared-range formal에 `m[k]==a[k]`를 강제**하고 이건 완전 관측 가능. **구별 자릿값 differential**(`m[4]*1000+m[3]*100+m[2]*10+m[1]`으로 caller `a[4:1]={4,3,2,1}` 넣어 forward면 4321·reversal이면 1234·**non-square 2×3 `[2:1][3:1]`=654321이면 dim-swap/reversal 즉시 가시화**)이 clean verification. element-index 의미 자체는 iverilog-checked(`int m[4:1]; m[k]=..` read-back이 iverilog·vita 동일). §4.5.206의 "검증 불가"는 verification 방법을 못 찾은 것이지 실제 불가가 아니었음.

**근본 원인(저자가 자기 머신러리를 불신)**: §4.5.206이 도입한 per-dim `(lo, size, ascending)` tuple + `array_formal_ext_dims`가 packed dim에 `lo`를 세팅→`flatten_word`가 pack(`lower_array_actual_packed`가 actual flat words를 position order로 읽음)과 read(`m[k]`→coord `k-lo`) **양쪽서 `idx-lo`를 일관 정규화**→`m[k]==a[k]` for same-range formal(direction 무관 forward). §4.5.206은 이 머신러리를 만들어놓고 descending에 대해 "derivation이 empirical과 어긋남"이라며(당시 mental model 혼동) 과보수적으로 `lo != 0 && !ascending` reject를 넣었으나, 실제로는 그 머신러리가 descending도 올바르게 처리함.

**fix(format 23 불변·1줄 제거)**: `classify_unpacked_array`의 Range arm에서 `if lo != 0 && !ascending { return Err(...) }` reject 제거. 나머지(`(lo, size, ascending)` 계산·flatten·dim-check)는 그대로—이미 descending을 handle. classify는 formal + frame-local 공유라 hier(§4.5.207/209)·forward(§4.5.210)·frame-local 경로 전부 자동 커버.

**적대 differential 全 MATCH(hand-IEEE §13.5.1·distinct-digit)**: INPUT 1-D `[4:1]`(4321)·offset base `[5:2]`(4321)·**non-square 2×3 `[2:1][3:1]`(654321 reversal clincher)**·**mixed `[1:2][3:1]`(654321)**·3×2 `[3:1][2:1]`(654321)·signed byte `[3:1]`($signed -40)·task copy-in(mem[k]=a[k])·OUTPUT `[4:1]`(44 33 22 11·copy-out `caller[lo+pos]`)·INOUT RMW(41 31 21 11)·hier INPUT(4321)·hier OUTPUT(44 33 22 11)·frame-local(40 30 20 10)·forward frame-formal(4321). **correct-or-loud 全 LOUD**: direction MISMATCH(ascending `[1:4]` actual/descending `[4:1]` formal→`lower_array_actual_packed` per-dim 체크가 진짜 §7.6 positional-copy guard)·base MISMATCH(`[3:0]` actual/`[4:1]` formal). 전 array-formal suite regression 0·flip 2(`multidim_array_formal.rs`·`hier_task_output_array.rs`의 `_stays_loud`→`_now_supported/_supported`).

**교훈**: **"오라클 없음"이 "검증 불가"로 잘못 등치되면 tractable한 걸 과보수 gate한다**—iverilog가 formal을 거부해도 IEEE spec(§13.5.1 `m[k]==a[k]`)이 관측 가능한 correctness 계약을 제공하고, 구별 자릿값(non-square 654321)이 clean differential. **prior "correct-or-loud" 결정도 재검증 대상**(§4.5.205와 동형: 과보수 gate가 자기 머신러리를 불신한 경우)·저자가 만든 `idx-lo` flatten이 이미 direction-agnostic이라 reject만 제거하면 됨·direction/base MISMATCH guard는 별개로 유지(진짜 §7.6 guard). 신규 `nonzero_base_descending_array.rs`×14·flip 2. **이로써 배열 formal shape 스토리 완전 종결**(any-base × any-direction × 1-D/multi-dim × input/output/inout × local/hier/forward/frame-local)·**ROADMAP §0 재개 큐(#1·#2·#3) 소진 완료**.

#### 4.5.210 forwarding a frame task/function's OWN unpacked-array FORMAL into a nested hierarchical TASK enable loud→supported (whole md-packed net forward) (2026-07-23, branch feat-hier-task-forward-array) ✅

**컨텍스트**: ROADMAP §0 재개 follow-on #2(§4.5.208 잔여). §4.5.207/209가 hier-task array formal에 STATIC array actual(`byte a[4]`)을 지원(pack/unpack element-by-element)했으나, frame task/func가 **자기 array formal**을 nested hier enable로 forward(`task driver(input int a[]); u.tk(a);`)하면 loud였음. 갭 원인: forward된 formal `a`는 md-packed FRAME net(not static array)이라 (1) defer gate(`inline_task`)가 static array만 `arg_arrays`에 record→`a`는 else 분기로 새서 `lower_expr(a)`(whole array formal을 value로 lower)=loud, (2) resolve서 `arg_arrays[i]`=None→"needs bare whole-array actual" loud.

**NO ORACLE**(iverilog subroutine array port 거부)→hand-IEEE §13.5.1 pass-by-value.

**핵심 통찰(UARR2 forwarding 재사용)**: frame array formal의 whole md-packed net 값 = callee slot의 packed 표현과 **동일 `array_formal_ext_dims` layout**(양쪽 다 element 0=LSB position order). 따라서 static array처럼 element-by-element pack(Concat)할 필요 없이 **whole net을 그대로 forward**(Signal{net, word:None})—`lower_array_actual_packed`의 UARR2 caller-formal forwarding(array_formal.rs §51-91)과 정확히 동일. per-dim `dims` + `elem_w`만 MATCH하면 됨.

**설계(format 23 불변·全 elaborate-transient)**: (1) **defer gate 완화**(`inline_task`)—bare Ident actual이 `net_is_static_array` OR `frame_arr_formal_meta.contains_key`면 `arg_arrays`에 record(둘 다 whole array net·frame formal은 md-packed). (2) **`hier_array_shape_ok` 확장**—actual이 `frame_arr_formal_meta`에 있으면(forwarded formal) `caller_af.dims == af.dims && caller_af.elem_w == af.elem_w`, 아니면 static array 검사(기존). (3) **`pack_hier_array_actual` 디스패치**—`net_is_static_array`면 Concat(기존), 아니면(frame formal) whole `Signal{net, word:None}`. → resolve INPUT 경로는 `pack_hier_array_actual` 호출만으로 static+forwarded 둘 다 처리. (4) OUTPUT/INOUT arm은 `!net_is_static_array(caller_net)`면 loud("forwarding a frame array formal to an OUTPUT/INOUT array formal is unsupported")—copy-out은 caller ELEMENT write(static array)라 md-packed frame net writeback은 frame-executor part-select 필요(follow-on).

**적대 differential 全 MATCH(hand-IEEE·+11 tests `hier_task_forward_array.rs`)**: INPUT 1-D(1 4)·multi-dim(10)·signed byte(-40)·**frame-LOCAL array forward(18·frame local도 frame_arr_formal_meta 커버)**·**mutated-value forward(acc=102 arr0=1·driver가 formal `a[0]=100` 후 forward→callee가 mutated 값·caller `arr` 불변 실증 §13.5.1)**·**chained 2-level(24·t.driver→m.relay→lf.tk)**·same-formal-to-two-enables(7 7). **correct-or-loud 全 LOUD**: shape mismatch·OUTPUT forward·INOUT forward·**non-hier whole formal in `$display`(defer gate 완화가 non-hier 경로 over-relax 안 함 실증)**. 기존 hier-task(static array §4.5.207/209·scalar) regression 0·flip 1(§4.5.208 `nested_hier_forwarded_array_formal_stays_loud`→`_now_supported`).

**교훈**: **forward = whole md-packed net 값 전달**(UARR2 forwarding 재사용)—양 slot이 동일 layout이라 repack 불필요·`pack_hier_array_actual`을 static(Concat)/forwarded(whole Signal) 디스패치로 통합해 resolve INPUT 경로가 두 소스 투명 처리·shared `hier_array_shape_ok`가 static/frame-formal 양쪽 gate·defer gate 완화는 hier enable actual 경로만(non-hier whole-formal use는 여전히 loud)·mutated-value 테스트가 pass-by-value + current-value read 동시 실증·OUTPUT/INOUT forward는 frame net writeback 필요라 loud(input forward만). 신규 `hier_task_forward_array.rs`×11·flip 1. **이로써 §0 follow-on #1·#2 완료**(남은 #2=non-zero-base descending은 오라클 확보 blocker).

#### 4.5.209 hierarchical TASK enable OUTPUT/INOUT unpacked-array formal loud→supported (deferred copy-out synthesized at resolution) (2026-07-23, branch feat-hier-task-output-array) ✅

**컨텍스트**: ROADMAP §0 재개 follow-on #1(§4.5.207 잔여·hard). §4.5.207이 hier-task INPUT array formal(`u.load(a)` where `task load(input int d[4])`)을 defer actual-net + resolve-time pack(`pack_hier_array_actual`)으로 지원했으나, OUTPUT/INOUT array는 loud였음. 갭 원인: **copy-OUT을 resolve 시점에 합성**해야 함—호출 시점엔 callee array shape 미지(child instance 미elaborate)이고, 로컬 §4.5.204 output-array copy-out은 lowering 중 `ret` 블록에 AST+`lower_stmt`로 unpack을 emit하는데, resolve 시점엔 ProcessBuilder도 caller scope도 없음.

**NO ORACLE**(iverilog subroutine array port 거부)→hand-IEEE §13.5.2 pass-by-value-result. write→read-back self-consistency + 로컬 output-array 경로(`frame_task_output_array.rs`·differential-verified)와 교차검증.

**핵심 통찰(scope/lower_stmt 회피·direct IR)**: copy-out unpack `caller[i] = temp[i*ew +: ew]`에서 **position i ↔ caller flat word i는 `pack_hier_array_actual`(copy-IN)의 정확한 역대칭**—pack이 caller flat word k를 slot position k에 넣으므로(`(0..count).rev()` Concat·position 0=LSB), unpack은 slot position i를 caller flat word i에 쓴다. caller_net은 이미 defer 시 resolve됨(`arg_arrays[i]`)이라 scope resolution 불필요·`collect_array_write`가 unpacked 배열에 `flatten_word(..&[])`(ascending 빈 slice→coord=idx-lo)를 쓰므로 flat word=row-major position i. → LHS chunk=`{net:caller_net, word:Some(const i)}`, RHS=`Select{Signal{temp}, offset:const(i*ew), width:const(ew), kind:PartIdxUp}` **직접 IR**(silent-wrong 위험한 flatten_word 재구현 회피).

**설계(format 23 불변·全 elaborate-transient)**: (1) **hier_tasks gate 완화**(`reserve_frame_task`)—`is_fixed_unpacked_array_formal`을 방향 무관 admit(input array뿐 아니라 output/inout array도 hier-callable·string/dyn은 여전히 제외). (2) **`hier_array_shape_ok(&self)`** 추출—copy-IN(`pack_hier_array_actual`)과 copy-OUT이 "matching"에 동일 gate 사용(divergence 방지·pack을 이걸로 리팩터). (3) **`resolve_deferred_hier_task_call`**의 array-formal 분기를 방향별로: INPUT=pack→in_bind(§4.5.207)·OUTPUT/INOUT=`deny_const_param_write`+fresh packed-temp net 예약(`__houtpack$..$<nets.len()>`·width=count×elem_w)+scalar out-bind(`(slot, whole_net_lvalue(temp))`·엔진이 callee md-packed slot→temp를 exit서 복사)·INOUT은 pack→in_bind(copy-IN, §13.5.2). (4) 루프 후 out_array_unpacks마다 unpack stmt(`caller[i]=temp[i*ew+:ew]`)를 direct IR로 빌드해 `unpack_sids` 수집. (5) terminator patch 시 `Call.ret_bb` 포획→ret 블록 `stmts.splice(0..0, unpack_sids)`로 **앞에 prepend**(process 경로=`processes[proc].body[ret_bb]`·§4.5.208 func_block 경로=`func_blocks[ret_bb]`). copy-out은 task exit(out-bind가 temp 씀) 직후·후속 user stmt 前 실행.

**correct-or-loud BY CONSTRUCTION**: 지원 array shape만 `frame_arr_formal_meta` 엔트리 有→resolve서 array로 인식; **미지원 shape**(descending non-zero-base 등)은 classify가 Err→엔트리 없음→resolver가 scalar로 취급→whole-array actual이 "scalar formal에 array" loud. frame-LOCAL array actual(md-packed·not static array)은 `arg_arrays[i]`=None→"needs bare whole-array actual" loud(follow-on #2 = frame-formal forward 유지). string/dyn output array=gate 제외→loud.

**적대 differential 全 MATCH(hand-IEEE·+17 tests `hier_task_output_array.rs`)**: OUTPUT 1-D(10 20 30 40)·INOUT 1-D(6 7 8 9)·INOUT RMW(2 4 6·copy-in이 body 前 landed 실증)·2×2(10 20 30 40)·**non-square 2×3 byte(0 1 2 10 11 12·reversal clincher)**·3-D(sum=28)·signed byte(-100 50)·mixed scalar-out+array-out(3 11 22 33)·**partial write(0 55 0 0·IEEE §13.5.2 unwritten=slot default 0)**·**nested-in-frame-body(7 8 9·func_block ret 주입)**·**cross-suspend `#5`(t=5 a=1 2)**·deep path `m.lf.gen`(100 200 300)·per-instance 격리(100 101 200 201). **correct-or-loud 全 LOUD**: descending non-zero-base·shape mismatch·scalar actual·string/dyn output array. 기존 hier-task(input array §4.5.207·scalar §4.5.201) regression 0·flip 1(§4.5.207 `hier_task_output_array_stays_loud`→`_now_supported`).

**교훈**: **resolve 시점 copy-out 합성 = direct IR가 lower_stmt보다 안전**—caller_net이 이미 resolve됨 + position↔flat-word 대칭이 copy-IN의 역이라 scope/context 의존 없이 correct-by-construction(flatten_word 재구현이 아니라 pack의 mirror). **하나의 shape gate(`hier_array_shape_ok`)를 copy-IN/OUT 공유**→두 방향이 "matching" 정의로 갈라질 silent-wrong 차단. ret-block `splice(0..0)` prepend가 §4.5.204 로컬 경로의 "start_block(ret) 직후 emit" 순서를 resolve 시점에 재현. gate가 미지원 shape를 array-meta 없이 두면 resolver의 scalar-mismatch가 자동 loud(correct-or-loud by construction). 신규 `hier_task_output_array.rs`×17·flip 1. **잔여 follow-on**: frame-formal array를 nested hier로 forward(§0 #1)·non-zero-base descending(§0 #2).

#### 4.5.208 hierarchical TASK enable NESTED in a frame-task body loud→supported (format_version 22→23 · FuncMeta.has_hier_call force-suspend) (2026-07-23, branch feat-hier-task-nested) ✅

**컨텍스트**: 오너 "둘 다 순차로"(H-C)·format bump 명시 승인. §4.5.197/201이 hier enable을 TOP-LEVEL process에서만 defer(nested-in-frame-body는 loud "hierarchical task call (deferred)"). 근본 원인: nested enable의 placeholder `Call.target`은 finish-phase resolve서만 patch되는데, **per-instance `resolve_frame_task_rejects`(`compute_suspendable_tasks`)가 그 前에** 실행→elaborate(pre-resolve·placeholder target→callee suspend 미전파)와 engine(post-resolve·patched→전파)의 suspend 분류 **divergence**→§4.5.197 pure-function 계약 위반(P0 suspend-misclassification).

**ORACLE**: array formal과 달리 iverilog가 scalar hier enable 지원→iverilog-differential(**suspendable callee 포함**).

**설계(sound over-approximation·format 22→23)**: **`FuncMeta.has_hier_call: bool`**(B1 func_table staged-trailer sidecar·`#[serde(default)]`)—frame task body에 deferred hier enable 있으면 set. `compute_suspendable_tasks`에 `force_suspend: &[bool]` param 추가(func_metas/func_table서 derive·양 caller 동일)→has_hier_call task를 **BOTH computes서 일관되게 suspendable 강제**(callee suspend 여부 무관·over-approximation SOUND[suspendable `&mut` path는 non-suspending callee도 실행]·placeholder-vs-patched target divergence 무의미화). frame-body defer machinery: `inline_task`의 `!frame_task_lowering` gate 제거→frame body서도 defer·`DeferredHierTaskCall.func_block: Option<u32>`(Some=frame-body→func_blocks/task_calls_func·None=proc)·`pending_hier_task_calls`(body별 수집·finish서 +base rebase[pending_task_calls 미러]→deferred_hier_task_calls로 이동+has_hier_call set)·resolve가 func_block 있으면 func_blocks patch+task_calls_func insert.

**적대 differential 全 MATCH(iverilog oracle)**: basic(cnt=1)·repeated(cnt=3)·**★suspendable callee `#5`(at 5 t0=9·caller 강제 suspendable 검증)**·input formal passthrough(acc=42)·output formal copy-out(x=42)·local work+hier(lc=20 cnt=2)·per-instance isolation(105 205). **correct-or-loud**: named args·frame-formal array를 nested hier로 forward(actual이 md-packed frame net·static array 아님→follow-on). **flip 2**(§4.5.197 `hier_task_nested_in_frame_body_stays_loud`→supported·`obs.rs` format_version 22→23).

**교훈**: **pre-resolve vs post-resolve compute divergence는 sidecar flag로 over-approximate하면 sound+consistent**(has_hier_call을 양 caller가 동일 FuncMeta서 derive→placeholder target 무관하게 일치)·**suspend 분류(P0)는 iverilog oracle로 suspendable callee 필수 검증**·frame-body defer는 pending_task_calls rebase 패턴 미러·**format bump는 staged 흐름(func_table 직렬화) 때문 불가피**(engine이 post-resolve로 "hier call이었는지" 재-derive 불가). 신규 `hier_task_nested.rs`×9·flip 2. **잔여 follow-on**: frame-formal array를 nested hier로 forward.

#### 4.5.207 hierarchical TASK enable with an INPUT unpacked-array formal loud→supported (defer actual-net + resolve-time pack) (2026-07-23, branch feat-hier-task-array-formal) ✅

**컨텍스트**: 오너 "둘 다 순차로"(H-B hier-task array formal + H-C nested-in-frame). §4.5.197/201이 hier-task scalar formal(input/output/inout)을 지원했으나 array formal은 loud("scalar formals — no array"). 갭 원인 둘: (1) 호출 시점 whole-array actual을 value로 lower 불가, (2) callee array shape이 child instance elaborate 전까지 미지.

**NO ORACLE**(iverilog subroutine array port 거부)→hand-IEEE §13.5.1 copy-in.

**설계(§4.5.202~206 array machinery 재사용·defer→resolve 역할 분리)**: (1) **hier_tasks gate 완화**—INPUT fixed-array formal도 hier-callable(scalar non-string OR input-fixed-array·output/inout array·string·dyn은 non-callable→gate 제외→loud). (2) `DeferredHierTaskCall`에 `arg_arrays: Vec<Option<u32>>` 추가—defer 시 bare whole-array Ident actual의 static-array net을 **caller scope서 resolve해 저장**(value lower 회피). (3) resolve 시 callee formal net(`base_net+slot`)이 `frame_arr_formal_meta`에 있으면 array formal→신규 **`pack_hier_array_actual`**(`lower_array_actual_packed` static-array path의 **resolve-time twin**·shape[per-dim base+size+direction·elem_w·count] MATCH 검증→Concat 빌드·position 0=LSB)로 md-packed slot value 생성해 in_bind. INPUT만(output/inout array=loud)·array-formal↔scalar-actual/scalar-formal↔array-actual 미스매치=loud·shape mismatch=loud. per-instance callee af라 격리 자동.

**적대 differential 全 MATCH(hand-IEEE)**: 1-D input(`u.load(d[4])`=1 4)·**mixed scalar+array register-file writer**(`u.wr(addr,d[4])`=a0 a1 a2 a3)·2-D(`u.p(m[2][2])`=42)·per-instance 격리(103 203)·signed byte + 3-seg deep path(`m.lf.p`=-40). **correct-or-loud 全 LOUD**: OUTPUT array over hier(gate 제외)·shape mismatch·array-formal+scalar-actual·scalar-formal+array-actual. 기존 hier-task(scalar·§4.5.197/201) regression 0.

**교훈**: **defer 시점 미지 정보(callee array shape)는 resolve로 미루되, caller-scope 의존 정보(actual net)는 defer 시 미리 resolve해 저장**(defer↔resolve 역할 분리가 핵심)·인접 슬라이스(§4.5.202~206 array machinery)의 pack 로직을 resolve-time twin(`pack_hier_array_actual`)으로 재사용→신규 표면 최소·per-instance `frame_arr_formal_meta[base_net+slot]` 조회로 callee shape 격리·gate가 output/inout array 배제→copy-out 미구현이 자동 loud(correct-or-loud). 신규 `hier_task_array_formal.rs`×9. **잔여 follow-on**: hier-task output/inout array formal(deferred copy-out·hard).

#### 4.5.206 NON-ZERO-BASE ASCENDING array formal loud→supported (per-dim base threaded into the md-packed slot) (2026-07-23, branch feat-nonzero-base-array-formal) ✅

**컨텍스트**: 오너 "남은 follow-on 계속·correct-support가 핵심". 배열 formal shape 마지막 갭=non-zero-based(`int m[1:4]`). §4.5.202~205는 zero-based만 지원(`classify_unpacked_array`의 `m.min(l) != 0` reject).

**설계 결정(correct-support vs silent-wrong)**: base+direction 상호작용이 (§4.5.205 descending처럼) mental model로 예측 불가(내 derivation이 empirical fact와 계속 어긋남)→**cleanly-verifiable subset만 지원**: non-zero-base **ASCENDING**(`[1:4]`)만, non-zero-base DESCENDING(`[4:1]`)은 loud 유지(correct-or-loud). ascending은 direction flip 없어 derivation 신뢰 가능(m[i]=a[i] forward). **correct-support가 핵심이어도 검증 불가한 걸 억지 지원하면 silent-wrong(silent ≪ loud).**

**fix(format 22 불변·tuple widening)**: `ArrayFormal.dims`를 `(size, ascending)`→`(lo, size, ascending)` 확장(lo=per-dim base). (1) `classify_unpacked_array`: Range의 `m.min(l) != 0` reject 제거·lo=min(m,l)·**non-zero+descending은 loud**·zero-based는 lo=0. (2) `array_formal_ext_dims`: packed dim에 lo 세팅(`(lo, size, false)`)→`flatten_word`가 `idx-lo` 정규화(zero-based lo=0→no `Sub`·byte-identical). (3) `lower_array_actual_packed` dim-check: actual lo(=min(msb,lsb))도 MATCH 요구(base mismatch=loud). (4) copy-out unpack: declared index=`caller[lo+pos]`(zero-based lo=0→pos·byte-identical). classify는 formals+frame-locals 공유→frame-local non-zero-base ascending도 자동 지원(bonus)·tuple widening의 모든 reader 사이트는 compiler가 build-error로 잡음.

**적대 differential 全 MATCH(hand-IEEE·forward)**: non-zero ascending 1-D `[1:4]`(1234)·2-D `[1:2][1:3]`(123 456)·OUTPUT `[2:5]`(20 30 40 50·copy-out가 `caller[lo+pos]`)·mixed zero+non-zero `[0:1][1:2]`(11 22 33 44). **LOUD**: non-zero DESCENDING `[4:1]`·base mismatch(`[1:4]` formal/`[0:3]` actual). zero-based regression byte-identical(全 array/formal test green). flip 2(round6·static non-zero `_stays_loud`→`_supported`).

**교훈**: **base+direction 상호작용이 예측 불가할 때(mental model 신뢰 불가)=cleanly-verifiable subset만 지원(ascending non-zero)+나머지는 correct-or-loud(descending non-zero)**—correct-support가 핵심이어도 검증 불가를 억지 지원하면 silent-wrong·tuple widening은 compiler가 모든 reader 사이트 잡아줌(build-error)·zero-based는 lo=0으로 byte-identical 보장·classify 공유로 frame-local도 bonus 커버. 신규 6 tests·flip 2. **이로써 배열 formal shape 스토리 완성**(zero-based any-dir + non-zero-base ascending·잔여=non-zero-base descending[correct-or-loud·drop]).

#### 4.5.205 DESCENDING / mixed-direction multi-dim array formal loud→supported (over-conservative gate removed) (2026-07-23, branch feat-descending-multidim-array-formal) ✅

**컨텍스트**: 오너 "남은 follow-on 항목을 계속 해·가치 떨어져도 correct-support가 핵심". §4.5.202가 descending multi-dim array formal을 loud로 gated("md-packed read는 index-major인데 actual 물리저장은 declaration-major→dim 내 element reverse"). **그러나 1-D descending formal은 이미 forward 동작**(§4.5.188·기존 `uarr_matching_descending_direction_ok`)—§4.5.202의 reversal 우려는 **1-D가 forward라는 증거를 무시한 잘못된 mental model**이었음.

**경험적 검증(correct-support 우선)**: 1-D descending distinct-value probe가 forward 확인(m[i]=a[i]·10 20 30 40·acc=1234) 후 gate 임시 해제→descending multi-dim 全 forward: descending 2-D `m[1:0][1:0]`(m[i][j]=a[i][j]·11 22 33 44)·**non-square 2×3 descending(123 456·reversal이면 명백히 다른 값→forward 확정 clincher)**·3-D descending(sum=28·spot 정확)·mixed direction `[0:1][1:0]`(11 22 33 44)·descending OUTPUT(1 2 3 4). **direction MISMATCH**(formal desc/actual asc·역)=LOUD(§4.5.202 `lower_array_actual_packed` per-dim direction 체크가 **실제 correctness guard**).

**근본 원인**: direction이 formal read와 actual pack **양쪽에 일관되게** 적용됨(per-dim direction MATCH 요구가 declared index→동일 logical element 보장)→forward pass-by-value(§13.5.1). classify gate는 redundant + wrong(correct 케이스를 reject).

**fix(format 22 불변)**: `classify_array_formal`의 descending multi-dim reject gate **제거**(per-dim direction match가 진짜 guard·non-zero-based/dynamic/non-simple-element는 여전히 `classify_unpacked_array`서 reject). iverilog는 구문 거부→hand-IEEE. flip 3(descending multi-dim input/output·static descending `_stays_loud`→forward).

**교훈**: **"correct-or-loud"의 loud가 실제로는 correct-support 가능한데 잘못된 mental model로 과보수 gate한 것일 수 있다→correct-support 우선이면 경험적 재검증**(distinct-value·특히 **non-square가 reversal 판별 clincher**)·§4.5.202의 reversal 우려는 인접 사실(1-D descending이 이미 forward)을 무시한 것·진짜 correctness guard(per-dim direction MATCH)는 이미 별도 존재했음(gate는 redundant). 신규 2 tests·flip 3.

#### 4.5.204 OUTPUT/INOUT multi-dim array formal loud→supported (multi-index copy-out unpack) (2026-07-23, branch feat-output-multidim-array-formal) ✅

**컨텍스트**: §4.5.202(input multi-dim)·§4.5.203(static task array formal) 후 오너 "계속". §4.5.202가 output/inout multi-dim을 loud로 남겼음 — §4.5.193 copy-out unpack이 `caller[i] = packed[i*ew +: ew]`로 **1-D 인덱스**라 multi-dim caller엔 partial-index(sub-array)→"assigning a non-array value to an unpacked array"(E3009).

**NO ORACLE**(iverilog subroutine array port 거부)→hand-IEEE §13.5.2 pass-by-value-result.

**fix(format 22 불변·copy-out unpack 1곳만)**: §4.5.193 unpack loop의 LHS를 1-D `caller[i]`에서 fully-indexed `caller[i0][i1]…`로 일반화 — row-major flat index `i`를 `af.dims`(outer→inner)로 분해(`strides[k]=∏sizes after k`·`idx[k]=(i/strides[k])%size[k]`)해 nested `BitSelect` chain 빌드. packed temp는 flat row-major(`array_formal_ext_dims`)라 `packed[i*ew +: ew]`가 정확히 element `i`·ascending zero-based(유일 지원 multi-dim shape)는 declared index=coord라 digit이 직접 인덱싱. 1-D는 `strides==[1]`·single digit=`i`→byte-identical. reserve(§4.5.202 `array_formal_ext_dims`)·out-bind push·INOUT copy-in(§4.5.202 multi-dim `lower_array_actual_packed`)은 이미 multi-dim 처리→**unpack만 잔여 갭**이었음.

**적대 differential 全 MATCH(hand-IEEE)**: output 2×2(10 20 30 40)·INOUT 2×2 round-trip(6 6 7 9)·**static output 2×2**(force-frame §4.5.203+copy-out·1 2 3 4)·non-square 2×3(0 1 2 10 11 12)·3-D 2×2×2(sum=28)·signed byte output(-100 50·caller read 부호 유지)·**INOUT read-modify-write**(doubles·2 4 6 8·old 읽고 new 씀)·1-D output regression(7 8 9). **LOUD**: descending output multi-dim(§4.5.202 gate). flip 3(`multidim_array_formal.rs` output/inout multidim `_stays_loud`→`_supported`·`static_task_array_formal.rs` `static_output_multidim_stays_loud`→`_supported`).

**교훈**: multi-dim formal 스토리를 **copy-out unpack 1곳 일반화로 대칭 완성**(reserve/copy-in은 §4.5.202가 이미 multi-dim 처리·unpack만 1-D였음—갭을 정확히 좁혀 최소 수정)·row-major decomposition(strides)이 nested `BitSelect`로 자연·1-D byte-identical(strides=[1]·single digit=i)·§4.5.203 force-frame과 자동 조합돼 static output multidim 커버. 신규 4 tests·flip 3.

#### 4.5.203 STATIC (non-`automatic`) task with a FIXED unpacked-array formal loud→supported (force-frame·§4.5.200 골격 재사용) (2026-07-23, branch feat-static-task-array-formal) ✅

**컨텍스트**: §4.5.202 적대 검증 중 발견한 인접 갭(T11/T11b). `task load(input int m[4]);`(static·array formal)이 vita E3009 "task `X` has an unpacked-array formal — unsupported" — 1-D·multi-dim·output/inout 전부. array formal의 md-packed value slot(§4.5.188 input/§4.5.193 output-inout/§4.5.202 multi-dim)이 FRAME 경로에만 존재하고 inline(static-task) 바인딩 경로엔 slot이 없어 `task automatic`만 지원됐음.

**NO ORACLE**: iverilog가 subroutine array port 구문 거부 → hand-IEEE §13.5.1/2 pass-by-value/value-result(static-local 저장은 framing 후에도 유지).

**fix(format 22 불변·§4.5.200 force-frame 골격 재사용)**: `build_task_frame_set`에 트리거 1개 추가 — 신규 `&self` 헬퍼 `is_fixed_unpacked_array_formal`(port에 unpacked dim 有 & 全 dim `Size|Range`=fixed·dyn/queue/assoc 자동 제외)이 true인 formal을 가진 task를 force-frame. frame ⊇ inline(§4.5.198/199/200 완성)이라 static task를 framing해도 local caller 능력 손실 0(§4.5.200 hier force-frame과 **동일 안전 논거**). framed되면 `is_framed=true`→inline reject(19740) 우회→md-packed 경로가 array formal 바인딩. reject shape(descending·non-zero-based·output/inout multi-dim)은 framed reserve가 `classify_array_formal=Some(Err)`이라 여전히 loud(correct-or-loud). 헬퍼는 direction/base/element 미검사(framing 여부만·supported/loud 판정은 classifier가 reserve 시=관심사 분리).

**적대 differential 全 MATCH(hand-IEEE)**: static 1-D input(acc=10)·static 2-D input(§4.5.202 조합·acc=42)·**static-local accumulation across calls**(cnt=6·framing이 static 저장 의미 보존)·output 1-D(10 20 30)·inout 1-D(6 17)·**register-file writer**(array formal+module-array element write+loop·a0 a1 a2 a3)·**array formal + `#5` suspend**(§4.5.168 suspendable·at 5 acc=7)·**inline caller→framed callee**(g=34·frame⊇inline call 경계)·**pass-by-value**(body input write=local·caller a0=1 불변·g=1004)·signed byte $signed(-45)·**actual refreshed each call**(r1=3 r2=30·stale snapshot 아님). **correct-or-loud 全 LOUD**: output multi-dim(copy-out reject)·descending·non-zero-based. full suite 회귀 0(array-formal static task는 이전 전부 loud라 regress할 working 케이스 자체가 없음).

**교훈**: **인접 슬라이스(§4.5.202) 적대 검증이 인접 갭(static task array formal) 발굴**·**§4.5.200 force-frame 골격이 트리거 1개로 재사용**(frame ⊇ inline 완성이 static-task force-frame 안전의 전제·hier든 array-formal이든 동일 안전 논거)·헬퍼는 framing 여부만 결정하고 supported/loud는 classifier가 reserve 시 판정(관심사 분리→헬퍼 단순)·framing이 static-local 저장 의미 보존(accumulation/refresh로 검증). 신규 `static_task_array_formal.rs`×14.

#### 4.5.202 multi-dimensional array FORMAL on framed function / `task automatic` loud→supported (hand-IEEE·N-D md-packed slot 재사용) (2026-07-23, branch feat-multidim-array-formal) ✅

**컨텍스트**: hier-task 완성(§4.5.197~201) 후 오너 "추천을 hand-IEEE로 진행"(§4.5.198/199/200에서 기록해둔 "잔여 loud follow-on: multi-dim FORMAL(1D binding)"). §4.5.199가 frame-LOCAL multi-dim array를 N-D md-packed slot(`array_formal_ext_dims`+`flatten_word`)으로 지원했으나 subroutine array FORMAL은 1-D 유지(`classify_array_formal`이 `dims.len() > 1` blanket-reject)였음. `task automatic proc(input int m[2][2])`가 vita E3009.

**NO ORACLE**: iverilog 13.0이 구문 자체 거부(`sorry: Subroutine ports with unpacked dimensions are not yet supported` — 1-D도 거부·vita는 §4.5.188로 1-D 이미 초과지원)→모든 supported case = **hand-IEEE §13.5.1 pass-by-value**(`m[i][j] = a[i][j]`·body는 자기 copy를 write해도 caller 무영향). memory [[no-oracle-not-a-defer-reason]].

**핵심 통찰**: §4.5.199의 N-D md-packed 머신러리가 이미 존재(frame-local array element access = `packed_dims`+`flatten_word` row-major part-select)→FORMAL도 **동일 slot을 재사용**하면 자연 확장. FORMAL과 frame-local의 유일 차이 = FORMAL은 call-site서 whole-array actual을 slot에 pack-in(`lower_array_actual_packed`)한다는 것뿐.

**fix(format 22 불변·전부 elaborate-transient)**: (1) **`classify_array_formal`**: `dims.len() > 1` blanket-reject 제거→**ASCENDING zero-based(`[N]`/`[0:N-1]`) multi-dim만 수용**·DESCENDING dim은 loud(md-packed read는 index-major인데 actual 물리 저장은 declaration-major→passthrough가 dim 내 element 조용히 reverse=silent-wrong). (2) **`reserve_frame_func`/`reserve_frame_task`** formal reserve 2곳: `array_formal_ext(count, elem_w)`→`array_formal_ext_dims(&af.dims, elem_w)`(1-D는 byte-identical·N-D는 dim당 packed_dims entry+elem_w). (3) **`lower_array_actual_packed`**: (a) actual dim-check을 `unpacked_n == 1`→`af.dims`와 per-dim (size+direction) 일치로 일반화(dim table clone→`self.error` borrow 회피)·1-D 기존 direction 체크 흡수·2-D actual for 1-D formal + descending actual 여전히 loud, (b) caller-formal 포워딩 조건 `caller_af.dims.len()==1 && count && ascending`→`caller_af.dims == af.dims`(1-D byte-identical·multi-dim 동일 layout whole-net passthrough 추가). (4) dead가 된 `array_formal_ext` 제거.

**적대 differential 全 MATCH(hand-IEEE — iverilog 구문 거부)**: 2×2 task(acc=10)·2×2 function place-weighted(5678)·3-D 2×2×2 sum(28)·non-square 2×3(726189)·**signed byte $signed 재-stamp**(-45)·`logic [7:0]` packed-vector element(AA^0F^F0^55=00)·2-state `bit` X/Z→0 coerce(255)·runtime index `m[i][j]`(11 22 33 44)·**body element WRITE pass-by-value**(local write r=1004·caller `a00=1` 불변)·**mixed 배열 input+scalar output**(g=77·`a10=3` 불변)·**cross-suspend value immunity**(`#5` 중 caller mutation→entry snapshot 42)·multi-dim formal 포워딩(123)·1-D regression(10). **correct-or-loud 全 LOUD**: descending dim(전용 message)·shape mismatch·partial index `m[i]`(§4.5.199 guard)·**STATIC(non-framed) task array formal**(pre-existing·1-D도 동일 loud)·hier call w/ array formal(§4.5.197 hier gate scalar-only·cross-boundary array copy=별개 follow-on)·**OUTPUT/INOUT multi-dim array formal**(§4.5.193 copy-out가 packed temp를 caller에 whole-array assign→multi-dim caller가 "non-array→unpacked array" 거부·별개 follow-on·silent mis-map 아님). full suite 회귀 0·flip 1(`frame_multidim_array.rs` `whole_multidim_array_arg_stays_loud`→`_now_supported`, s=7).

**교훈**: **인접 완료 슬라이스(§4.5.199 N-D md-packed)의 머신러리를 FORMAL로 재사용**하면 신규 표면이 formal-reserve 2줄 교체+actual-check 일반화로 축소(md-packed slot·`flatten_word`·part-select 재사용)·**ascending-zero-based gate가 direction-reversal silent-wrong의 근본 차단**(descending은 md-packed index-major vs actual physical declaration-major 비대칭→correct-or-loud로 loud)·**copy-out path(§4.5.193)가 multi-dim caller에 whole-array assign을 이미 거부**→output/inout multi-dim이 신규 코드 없이 자동 loud(silent 아님)·no-oracle=hand-IEEE(§13.5.1 pass-by-value를 body-write/caller-immunity/cross-suspend로 직접 검증). 신규 `multidim_array_formal.rs`×19·flip 1.

#### 4.5.201 hierarchical task OUTPUT/INOUT scalar formal loud→supported (cross-boundary copy-out) (2026-07-23, branch feat-hier-task-outformal) ✅

**컨텍스트**: hier-task 완성(§4.5.197~200) 후 오너 "진행해"(follow-on). §4.5.197 hier-task은 **INPUT-only scalar formal**만 지원(defer 시 callee port 방향 미지라 copy-out 불가)·output/inout formal은 loud였음. iverilog 오라클 有(`u.compute(x,r)`=r=31·`u.inc(r)`=r=15).

**핵심 난점**: hier call `u1.tk(x, r)` **defer 시점**(pass 7·callee instance 미elaborate)엔 callee의 port 방향(input/output/inout) 미지→어느 arg가 copy-IN(value)이고 어느 게 copy-OUT(caller lvalue)인지 결정 불가.

**해결(dual-lower at defer + port-dir routing at resolve)**: (1) **defer 시 각 arg를 value(`arg_ids[i]`·copy-in expr)와 lvalue-able이면 caller lvalue(`arg_lvals[i]`·copy-out target)로 둘 다 lower**(`expr_to_lvalue`+`lower_lvalue`·non-lvalue arg는 None·부작용 無[Lvalue 생성만]). (2) `reserve_frame_task`가 hier-callable task의 port 방향을 `hier_task_port_dirs[fid]` sidecar에 저장(callee def이 resolve 시점 scope 밖이라 재조회 불가). (3) `resolve_deferred_hier_task_call`이 방향별 라우팅: Input→`in_binds`(value)·Output→`out_binds`(lvalue)·Inout→both. `TaskCallInfo{in_binds, out_binds}`는 엔진이 이미 적용(§4.5.194 `emit_frame_task_call` copy-out machinery 재사용·process Call arm이 out_binds를 task 종료 후 caller lvalue에 write). `hier_tasks` gate를 Input-only→Input|Output|Inout scalar non-string으로 완화. **wire 무변화**(format 22 불변·DeferredHierTaskCall/hier_task_port_dirs 全 elaborate-transient).

**적대 differential 全 MATCH(iverilog 오라클)**: output(r=31)·inout(r=15)·**multi in+out STATIC task per-instance param**(divmod 17/5: 4 2 3 2·§4.5.200 force-frame과 조합)·output to array-elem select(arr2=42)·output+instance-net write+control flow static(lookup: a=4/b=-1/hits=1)·inout to array-elem select(m1=123)·**partial output write(r=0·IEEE §13.5.2 pass-by-value-result: unwritten output formal이 default 0을 copy-out해 caller의 999 덮음)**. correct-or-loud: non-lvalue output arg(`u.f(3)`)=loud·array/string formal=loud. input-only(§4.5.197) regression 0.

**교훈**: **defer 시점 callee 정보 미지 문제를 dual-representation(value+lvalue) lower + resolve 시 port-dir sidecar로 선택**하는 패턴으로 해결(§4.5.196 hier-FUNCTION의 "engine이 formal width로 coerce" 트릭과 다른 접근—copy-out은 실제 caller lvalue가 필요라 defer 시 미리 잡아둬야 함)·기존 `out_binds` copy-out machinery 재사용으로 엔진 무변화·IEEE partial-write 자동 준수(엔진 copy-out이 formal default 반영). 신규 test 확장(`hier_task_call.rs` output/inout supported+non-lvalue loud·`hier_task_call_static.rs` static output+divmod).

#### 4.5.200 STATIC-task hierarchical call `u1.tk()` loud→supported (frame↔inline parity, step 3 — hier-task 완성) (2026-07-23, branch feat-static-hier-task) ✅

**컨텍스트**: §4.5.199로 **frame ⊇ inline 완성** 후 오너 "진행해"(step 3). §4.5.197이 AUTOMATIC task hier call을 지원(automatic task는 이미 framed→per-instance FuncId 有)했으나 STATIC(non-automatic) task는 inline→FuncId 없어 hier defer/resolve가 못 bind→loud. **이전엔 static task force-frame이 그 task의 LOCAL caller를 frame-subset에 종속시켜 회귀 위험이었으나**(오너와의 정확도-사다리 논의), §4.5.198(module array-element write)+§4.5.199(multi-dim frame-local)가 frame⊂inline 갭을 닫아 frame⊇inline→**force-frame이 회귀 없이 안전**.

**fix(format 22 불변·§4.5.197 머신러리 재사용)**: 전역 1-회 **pre-scan**(`collect_hier_task_stmt`가 모든 모듈의 procedural block body walk→2+세그 `UserTaskCall`의 last segment=task 이름을 `hier_called_task_names`에 수집·`run` 시작서 framing 전 1회)·`build_task_frame_set`에 `|| self.hier_called_task_names.contains(name)` 추가로 force-frame·기존 §4.5.197 `reserve_frame_task`의 `hier_tasks` 등록(input-only scalar)+`inline_task` defer+`resolve_deferred_hier_task_call`이 자동 bind. **name-based**(framing이 instance elaboration보다 먼저 실행되므로 `u1`→module 해소 불가)라 동명 무관 task도 framed되나 frame⊇inline이라 **무해**. under-collection(generate block·nested-in-task-body hier call)=correct-or-loud(loud, not silent-wrong).

**적대 differential 全 MATCH(iverilog 오라클)**: static `bump`(cnt=2)·register-file `mem[a]=d`(mem[3]=aa/mem[7]=bb·§4.5.198+200 조합)·**★local+hier 동시 호출**(cnt=3·force-framed static task의 LOCAL 호출이 frame path로 회귀 0)·2D local+nested for(acc=20·§4.5.199+200)·timing `#5`(t0=9)·name-collision 2모듈(a=11/b=110·over-application 무해)·per-instance param isolation(206/205). **correct-or-loud**: output/inout/string/array formal=loud(hier-callable 아님). automatic hier(§4.5.197) regression 0.

**교훈**: **frame ⊇ inline 완성이 force-frame 안전화의 전제**(정확도 사다리를 다 올라온 뒤에야 static hier-task가 회귀 없이 가능)·**전역 pre-scan이 framing↔hier-call 순서 역전 해결**(framing이 먼저라 hier-call 대상을 미리 수집)·name-based over-application은 parity라 무해·under-collection은 correct-or-loud·기존 §4.5.197 defer/resolve 머신러리에 **force-frame 1줄만 추가**로 완성. **이로써 hier-task 완성**(automatic + static 모두 커버). 신규 `hier_task_call_static.rs`×9.

#### 4.5.199 multi-dim frame-LOCAL array loud→supported (frame↔inline parity, step 2 — 마지막 갭) (2026-07-23, branch feat-frame-multidim-local) ✅

**컨텍스트**: §4.5.198 후 `task automatic`(framed) vs `task`(inline) body를 ~25 구성으로 differential-sweep→frame⊂inline 비대칭의 **유일한 잔여 갭 = multi-dim frame-LOCAL array**(`int m[2][2]` in task)로 규명(module 쓰기·control flow·1D local·queue/dyn·timing·$display 全 parity·string/sformatf local은 오히려 inline이 약함). 오너 "step2로 진행".

**갭**: `int m[2][2]`가 `classify_unpacked_array`의 `unpacked.len()!=1` reject로 `frame_array_local` 1-elem net化→`m[0][0]=v`가 base `m[0]`(BitSelect)를 `lval_base_net`이 못 받아 "nested lvalue select (v1: single-level)". **모듈-scope `m[i][j]`는 동작**→frame-lowering-specific.

**핵심 통찰(subagent 정밀 트레이스)**: frame-local array element access는 md-packed **part-select**(`packed_dims`+`flatten_word`)로 라우팅되고 이 **N-D 머신러리는 이미 존재**(`build_hier_packed_read`·`lower_packed_read`·`collect_packed_write`). frame md-packed net은 `array_len=1`이라 `net_is_static_array=false`→READ(`expr_packed_chain`→`lower_packed_read`)·WRITE(`lval_packed_chain`→`collect_packed_write`) **모두 `packed_dims` 소비**(`net_dim_extents`/`array_dims` 미사용). nested `m[i][j]` chain도 packed chain이 이미 walk·"single-level" 제약은 `lval_base_net`에만 있고 packed 경로는 우회. 즉 **`packed_dims`를 multi-dim으로 세팅하면 동작**.

**fix(format 22 불변)**: (1) `classify_unpacked_array`가 모든 zero-based-const dim 수용(single→loop·`count=∏sizes`·per-dim `(size,ascending)`→신규 `ArrayFormal.dims`·multi-dim-PACKED-element reject는 유지). (2) `reserve_frame_local_decl`가 `array_formal_ext_dims(&af.dims, elem_w)`로 md-packed slot 예약(dim당 `packed_dims` 1 entry + trailing elem_w entry·`w=count*elem_w`). `flatten_word`가 `m[i][j]`→offset `i*∏inner*elem_w + j*elem_w`·width `elem_w`(stride가 elem_w 포함). **1D는 byte-identical**(`dims=[(count,asc)]`→`array_formal_ext_dims`=`array_formal_ext`). `ArrayFormal` Copy→Clone(1 site `&caller_af`→`.cloned()` + `lower_array_actual_packed` sig `&af`).

**★핵심 silent-wrong guard(subagent 발굴)**: partial index `m[i]` on 2D(indices < unpacked dims)는 multi-element sub-array slice(`width=∏remaining dims`)를 조용히 반환→`lower_packed_read`/`collect_packed_write`에 `idxs.len()+1 < dims.len() && frame_arr_formal_meta.contains(net)` loud guard(scalar element까지 index 강제). genuine multi-dim PACKED net(`reg [3:0][7:0] x; x[i]`)은 `frame_arr_formal_meta` 밖이라 legal partial sub-select 무영향·1D(`dims.len()==2`)는 `idxs.len()<1` 불가라 **절대 미발화=byte-identical**. bit-select `m[i][j][bit]`(idxs.len()==dims.len())는 통과(guard는 `<`라).

**correct-or-loud**: FORMAL은 1D only(`classify_array_formal`이 `dims.len()>1` reject→formal binding은 single-dim pack·multi-dim FORMAL이 유일 loud follow-on)·whole 2D array as arg(`f(m)`)=loud(`lower_array_actual_packed`의 `caller_af.dims.len()==1` guard). (element-part-select `m[i][j][7:4]`은 READ·WRITE 모두 packed chain으로 동작·iverilog MATCH[0xA5→10/5·write→192]—subagent가 loud로 예측했으나 실제 supported.)

**적대 differential 全 MATCH(iverilog 오라클)**: 2D basic(5678)·3D non-square runtime-loop(726)·descending dims `[1:0][3:0]`(189)·signed byte(-105·md-packed whole-unsigned→`$signed` restamp)·runtime index both dims(77)·bit-select of element(11)·function body(30)·across-`#5`-suspend(14·per-activation 격리). partial-index/whole-arg 全 loud. 1D regression(g=18/14) clean. full suite green·format 22 불변·clippy/fmt clean.

**교훈**: **frame-local array=md-packed part-select이라 N-D packed 머신러리(`flatten_word`) 재사용이 자연**(word-based `net_dim_extents` 아님)·subagent 정밀 트레이스가 read/write 모두 `packed_dims` 소비 확인→`net_dim_extents` 우회 안전·**partial-index guard가 핵심 silent-wrong 방지**(subagent가 "too many만 reject, too few는 silent multi-slice" 발굴)·`packed_dims`/`frame_arr_formal_meta`=elaborate-transient라 format 불변·1D byte-identical(golden churn 0). **이로써 frame ⊇ inline 완성**→STATIC task hier-call force-frame이 회귀 없이 안전(step 3). 신규 `frame_multidim_array.rs`×10·flip 1(`frame_local_array_multidim_task_loud`→`_supported`).

#### 4.5.198 frame task MODULE array-element write loud→supported (frame↔inline parity, step 1) (2026-07-23, branch feat-frame-array-write) ✅

**컨텍스트**: §4.5.197 후 오너와 "static hier-task force-frame 회귀 리스크" 논의 → "정밀 RTL 분석·정확도 최우선이면 어떤 선택이 올바른가?" 질문. 결론=**정확도 사다리**(silent-wrong ≪ loud ≪ correct-support·"올라가되 내려가지 마라"). naive force-frame은 동작하던 local 호출을 loud로 만듦(내려감)→오답. 올바른 방향=**frame 경로를 inline과 동등하게**(loud→correct-support). 그리고 frame⊂inline 비대칭은 **hier-task와 무관하게 오늘 이미 존재하는 정확도 갭**: `task automatic; mem[i]=v;`가 vita E3009인데 iverilog·verilator·vcs 정상. 오너 "진행해"→step 1 착수.

**갭**: §4.5.197 Option A가 `compute_suspendable_tasks`의 signal 규칙을 `word.is_none() && out-of-frame`로 게이트해 module ARRAY-element write(`mem[i]=v`·`word=Some`)를 제외→그 task가 non-suspendable subset로 분류→`classify_frame_body`가 "part-select/array-element assignment"로 loud-reject. whole/part-select(word=None)만 §4.5.197서 지원됐었음.

**fix(1-line·format 22 불변)**: signal 규칙에서 `word.is_none()` 조건 제거→`lhs.chunks.iter().any(|c| c.net < lo || c.net >= hi)`. 이제 **모든 out-of-frame write chunk**(whole·part-select·array-element·concat `{a,b}=x`)가 suspend-signal→module-array-writing task가 suspendable `&mut` process path로 lift. suspendable executor는 `write_lvalue`(full &mut)로 array-element write 수행(module-process 일반 경로와 동일).

**적대 differential(iverilog 오라클) 全 MATCH**: `mem[a]=d`(aa bb)·runtime-index loop `for(i) mem[i]=base+i`(100 103 107)·hier `u.write_mem`(aa bb·hier-called automatic array-writer도 자동 커버)·concat `{a,b}=v`(a b). **class-field via MODULE handle over-lift 검증**: `obj.f=v`(module handle·word=Some·out-of-frame)가 이제 over-mark돼 suspendable로 lift되나 **무해**(f=42·suspendable `&mut`도 class_heap[RefCell] write 동일 수행·superset). 회귀 probe 全 clean(whole cnt=2·part reg8=fa·subset-only r=31). **full suite 4028→4029 green(0 fail)**·class-method 회귀 0.

**correct-or-loud 경계**: IN-FRAME word-indexed write(frame-local array element·in-frame handle의 class-field)는 미표시→subset 유지(`&self` 실행). **multi-dim frame-LOCAL array**(`int m[2][2]` in task)는 여전히 loud[frame nested-lvalue-select "v1: single-level" + `frame_task_has_unsafe_construct` frame-local-array guard]—단 **모듈-scope `m[i][j]`는 동작**하므로 frame-lowering-specific 갭(step 2·별개·드묾). single-dim frame-local array는 이미 동작(§4.5.169 md-packed).

**교훈**: **정확도 최우선=frame⊂inline 비대칭 자체가 갭**(hier-task 우회 아니라 정공법)·§4.5.197 Option A의 `word.is_none()` 게이트만 제거하면 array-element까지 커버(suspendable `&mut`가 array write 이미 수행)·class-field over-lift은 무해(suspendable=`&self` subset의 superset)·**정확도 사다리는 올라가되 내려가지 마라**(naive force-frame=correct→loud=오답). 신규 test 1(runtime-index)·flip 1(`module_array_element_write_stays_loud`→`_supported`).

#### 4.5.197 hierarchical TASK call `u1.tk(x)` loud→supported (+ frame-aware suspend-classification infra) (2026-07-22, branch feat-hier-task-call) ✅

**컨텍스트**: §4.5.196의 명시적 follow-on. 오너 지시="인프라를 지어서 hier-task까지 가는게 맞아. 이 방향으로 진행해". §4.5.196이 hierarchical FUNCTION call을 지원하며 "hier-task는 framed-task-write-to-instance-net 인프라(L)가 필요"로 defer했던 것을 해소.

**진짜 blocker 진단(초기 가정 뒤집힘)**: 사전 조사에서 "framed task가 callee instance net을 WRITE=frame-subset 제약"이라 봤으나, fresh-probe가 **비-hier automatic task도 module net write가 이미 loud**임을 규명(P1: `task automatic bump(); cnt=cnt+1;`가 E3009 "net outside the function" vs iverilog `cnt=2`). 즉 hier-특정 문제가 아니라 **`&self` subset frame executor가 module net을 못 쓴다**는 일반 갭. 근본 원인=`compute_suspendable_tasks`의 `stmt_signal`이 blocking assign을 무조건 non-signal로 봐, module net만 쓰는 automatic task를 NON-suspendable(subset)로 분류→`run_task_call`(`&self`)이 module write 불가라 loud. P3(`$display`+module write)는 `$display`가 signal이라 이미 suspendable→`cnt=2` 정상 = **suspendable `&mut` path는 module write 가능** 실증.

**Option 비교**: (B) `&self` executor가 module net 쓰게 하려면 `SimState.nets`(hot-path plain Vec)를 RefCell화=perf/scope 과대 → 기각. (A·채택) **suspend 분류를 frame-aware화** — blocking assign이 자기 frame window 밖 net을 쓰면 signal→suspendable path로 lift.

**인프라(Option A·format 22 불변)**: `compute_suspendable_tasks(funcs, blocks, stmts)` → `+base_nets: &[u32]`. `base_nets[fi]`=func frame base. 엔진 `func_table`은 elaborate `func_metas`를 `std::mem::take`로 verbatim thread(lib.rs:713)이므로 양측 base_net 불변 동일→"pure function of the arenas" 계약 유지(base_net이 입력의 일부가 됨·serialized FuncDef 필드 추가 회피=format bump 없음). signal 규칙: `BlockingAssign`의 chunk가 `word.is_none() && (net < lo || net >= hi)`(lo=base, hi=base+`f.locals_len`)면 signal. **`word.is_none()` 게이트 핵심**: class-field heap write(`obj.f=v`·`word=Some`·`&self`-safe)는 제외해 class-method subset task의 동기 routing 유지·module ARRAY-element write(`mem[i]=v`·`word=Some`)도 제외→그건 subset part-select reject로 loud 유지(correct-or-loud). 두 call site(engine lib.rs:652·elaborate `resolve_frame_task_rejects`:16305) 모두 `base_nets` 전달. **soundness**: 새로 suspendable 되는 task는 이전에 loud(subset reject)였으므로 회귀 불가·같은 lift-guard(`frame_body_is_leaf_nonsuspending`·`frame_task_has_unsafe_construct`)를 통과하므로 unsafe(frame-local array multi-dim 등) 여전히 loud.

**hier-CALL defer(§4.5.196 미러)**: `hier_tasks: BTreeMap<String,u32>`(`reserve_frame_task`서 no-`::`+input-only scalar non-string만 FQ `<inst>.<tname>`→per-instance fid 등록)·`struct DeferredHierTaskCall{proc, call_block, prefix, path, arg_ids}`·`inline_task`의 `segments!=1` 즉시-reject를 defer로 교체(top-level process만·`!frame_task_lowering`·named-arg 제외→args를 caller-scope self-width로 lower·placeholder `Terminator::Call`+ret block seal)·`resolve_deferred_hier_task_call`(finish서 `resolve_deferred_hier_call` 뒤 호출: `hier_resolve(prefix, path, hier_tasks)`→fid·arity guard·`TaskCallInfo{callee:fid, in_binds:positional, out_binds:[]}` into `task_calls_proc[(proc,call_block)]`·`processes[proc].body[call_block].term` target patch).

**엔진 무변화로 동작하는 이유**: process `Terminator::Call{ret_bb, ..}`가 `target`을 무시하고 callee를 `task_calls_proc[key].callee`에서 얻음(exec.rs:629)→defer 시 target=placeholder OK(faithful IR 위해 resolve서 patch만). `enter_task_frame`이 in-value를 per-instance formal width로 `resize_keep_sign`(state.rs:3535)→arg를 self-width로 lower해도 엔진이 coerce. per-instance `reserve_frame_task`가 instance net/param을 fid에 baked→hier fid가 곧 올바른 instance 상태.

**적대 검증(iverilog 오라클)**: 全 MATCH — instance-net write(`u1.bump()`×3→cnt=3)·input scalar formal(acc=42)·2-instance param 격리(K=100 acc=206·K=200 acc=205)·deep path(`m.lf.set(77)`)·multi-arg signed/wide(s=66786)·`#5` cross-boundary suspend(at 5 t0=9)·body-local+instance RMW fib(a=8 b=13). P1 infra(cnt=2)·part-select module write(reg8=fa)·RMW(sum=15). **correct-or-loud 경계 全 loud**: output/inout/string formal·unknown task·wrong arity·named args·nested-in-frame-body·module array-element write. **flip**: `frame_subset_overscan.rs::out_of_frame_write_still_loud`→`_now_supported`(g=5, iverilog parity).

**잔여 loud follow-on(correct-or-loud)**: (1) output/inout/array/string formal hier-task[cross-boundary pass-by-ref copy-out·별개 슬라이스]. (2) STATIC(non-automatic) task hier-call[non-framed이라 hier_tasks 미등록·force-framing은 그 task의 LOCAL caller를 frame subset에 종속시켜 module array-elem write 등서 회귀=large·오너 direction 필요]. (3) module ARRAY-element write in framed task[`word=Some` out-of-frame·`&mut` array-elem path를 lift에 plumb하면 가능하나 별개]. (4) nested-in-frame-body hier enable[`task_calls_func` 키·`Call.target`이 suspend transitivity에 walk됨].

**교훈**: **초기 "executor가 net을 못 쓴다"는 가정을 fresh-probe가 "분류 함수가 frame-aware가 아니다"로 정정**(P1이 hier와 무관하게 이미 loud임을 발견→infra가 hier-특정이 아님)·§4.5.194의 "executor `&self`가 진짜 제약 아니다" 교훈과 동형[그땐 dyn_heap plain-Vec, 여기선 suspend 분류]·**base_nets를 param으로 전달**(func_metas verbatim-thread라 divergence 불가)로 format bump 회피하면서 pure-function 계약 유지·`word.is_none()` 게이트가 class-field(heap·`&self`-safe) vs module-net write 구분·엔진 process Call이 target 무시+`enter_task_frame` width-coerce라 hier defer가 엔진 무변화로 동작·소운드니스=새 suspendable은 이전 loud라 회귀 불가+기존 lift-guard 재사용. 신규 `hier_task_call.rs`×14·`frame_task_module_write.rs`×6.

#### 4.5.196 round-17 D-가족(minor 잔여) tractable 4항목 loud→supported (2026-07-22, branch feat-round17-d-family) ✅

**컨텍스트**: §4.5.195(3-가족 A/B/C + CLI) 완료 후 오너 지시="D-항목도 모두 구현". 리포트 §5 "minor 잔여"(repro 미제공)를 fresh-probe로 트리아지: iverilog 오라클 有→구현, iverilog도 거부→hand-IEEE, 이미 동작→걸러냄. 결과=tractable 4항목 구현 + 4항목 follow-on 문서화(전부 loud·correct-or-loud). format 22 불변·3994→4008 green·3-agent 병렬 매핑(hier-call·$display-in-fn·function-reforward).

**① import-in-pkg (iverilog ✓)**: 패키지 body 내 `import base::*`가 elaborate E3009("imports inside a package are outside the v7 scope")였음. 타입(`base::byte8_t`)은 파서 unit-global typedef map이 이미 resolve(그래서 parse 성공·E3009는 elaborate)·패키지는 선언순 elaborate라 base가 derived 前 완료(`pkg_consts[base]` 존재). fix=`elaborate_package` 루프의 `Import` arm이 reject 대신 `apply_import_consts`(모듈 import와 동일 머신·`saved`로 restore) 호출→base 상수를 derived fold 스코프에 주입(`DW=W*2`=16). 패키지-INTERNAL 루틴 호출(derived가 base 함수 호출)은 call-time resolution이라 follow-on(loud). `round17_d_family.rs`(import ×2).

**② $display/$write-in-subset-function (iverilog ✓)**: 함수 body 내 `$display`가 E3009("$systask outside frame-call subset")였음(B1 cut·eval-path가 silently drop 우려). **positive gate 설계(custom-table API 회피)**: `lower_systask`의 general push서 genuine Display/Write sid를 신규 `frame_print_stmts`(Elaborator 필드·validator-only·serialize 불요)에 기록(severity/$timeformat/stage/marker는 각자 table에 早期 return→set 밖). `classify_frame_body`가 그 set의 Display/Write 허용·`run_frame_call`/`run_task`의 marker arm 뒤에 render arm(`format_args_str`[이미 `&self`]+`sink.emit(LogEvent::RtlOutput)`[sink interior-mutable→`&self`-safe·module-process와 byte-identical]). engine은 handle_copy_stmts(marker) 소비 후 남은 Display/Write render(gate가 genuine만 통과 보장). **적대**: `$display` in fn=iverilog MATCH(`dbg x=5`/`r=10`)·`$writeh` radix(`000000ff`)·`$error` in fn=loud 유지(severity=set 밖·correct-or-loud)·Family C marker+$display 공존(collision 無). format 무변화(radixes/handle_copy_stmts 재사용).

**③ hierarchical function call `u1.f(x)` (iverilog ✓·M-subset)**: E3009("hierarchical function call (deferred)")였음. **hier-net read defer 미러**(entirely elaborate-side·engine `Expr::Call`은 location-independent·format 무변화): (a) `hier_funcs: BTreeMap<String,u32>`(FQ `<inst-path>.<fname>`→per-instance fid·persistent)—`reserve_frame_func`가 HIER-CALLABLE(bare name·input-only SCALAR formal·non-string formal·non-string return)만 등록[`cur_prefix`=instance path]; (b) collection—`inline_function`의 2-seg reject 前 arm이 `POISON_FID` placeholder `Expr::Call` emit+`DeferredHierCall` push[child instance未elaborate라 defer·hier-net과 동일]; (c) resolution—`resolve_deferred_hier_call`(finish서 resolve_deferred_hier 옆)이 `hier_resolve<u32>`(§23.6 commit-to-scope walk 재사용·FQ-key라 그대로 동작)로 fid 찾아 patch·arity guard·unresolved→loud. **핵심 관찰**: per-instance `lower_frame_funcs`가 각 instance의 net/param을 별도 fid에 이미 baked→deferred call이 올바른 fid만 가리키면 body의 net/param read 자동 정확(`u1.addk`/`u2.addk` K=1000/2000=1005/2005·callee net `base=100`=105 全 iverilog MATCH). **correct-or-loud**: output/inout formal·string return·array formal·unknown func·arity mismatch·inlined callee(hier_funcs 無)=전부 loud. **잔여 loud=hierarchical TASK call**(L): task는 callee instance net WRITE→frame-subset "net outside function" 제약(function READ는 per-instance라 OK·task WRITE는 framed-task-write-to-instance-net 인프라 필요).

**④ task inout unpacked-array formal (iverilog 거부→hand-IEEE)**: §4.5.193이 OUTPUT fixed array 열었으나 INOUT은 loud였음(guard가 Input|Output만·`reserve_frame_task`/emit/post-pass 3곳). fix=3곳에 Inout 추가+`emit_frame_task_call`의 output-array arm(`Output|Inout`)서 Inout이면 copy-OUT(§4.5.193 packed-temp+unpack) 前에 copy-IN(`lower_array_actual_packed`→in_bind·§4.5.188 input과 동일) 추가=IEEE §13.5.2 pass-by-value-result. iverilog는 unpacked subroutine port 거부("Subroutine ports with unpacked dimensions")→hand-IEEE·**round-trip 자기검증**(inout `a[i]+=5`·caller 10/20/30→15/25/35이 copy-in[old값 read] 증명·copy-in 없으면 0+5=5). output array regression 정상.

**follow-on(전부 loud·문서화)**: **hierarchical TASK call**(L·framed-task-write-to-instance-net) · **function이 자기 dyn-formal 재전달**(`return sum(c)` where c=이 함수 formal·framed callee·agent 매핑=Part1 frame-route+Part2 Return marker지만 mutual-recursion soundness hole[naive framing이 terminating silent-wrong]→신중 guard 필요·beyond report·`nested_in_frame_body_loud` 유지) · **block-local automatic lifetime**(loop-body `automatic int j=k*10`·per-activation storage=deep block-local-flatten 인프라·iverilog도 "Overriding default variable lifetime" 거부) · **chained-method** `q.min()[0]`(vague·iverilog syntax-error·no-oracle). **이미 동작(vita>iverilog)**: output-fn-in-`while`(D4)·task OUTPUT array(D5b).

**교훈**: **fresh-probe 트리아지가 4버킷 분류**(iverilog-oracle→구현·iverilog-거부→hand-IEEE·이미-동작→skip·deep/vague→document)·**hier-net defer 머신(`hier_resolve` FQ-key)을 hier-CALL에 재사용**(entirely elaborate-side·per-instance fid가 net/param 자동 baked)·**positive gate(`frame_print_stmts`)가 custom-table API 의존 회피**(genuine print를 lowering서 mark·special Display는 set 밖이라 자동 loud)·**function은 per-instance net READ라 hier-call 동작하나 task는 WRITE라 frame-subset 제약**(hier-func=M·hier-task=L의 경계)·**§4.5.188 copy-in + §4.5.193 copy-out 조합이 inout**(no-oracle라 round-trip 자기검증). 신규 `round17_d_family.rs`×14·기존 flip 2(`inout_array_formal`·`frame_body_loud_rejects` display sub-assertion). **+14 tests(3994→4008)·format 22 불변·MsgCode 59 불변.**

#### 4.5.195 round-17 리포트 3-가족(A/B/C) + CLI 이원-top 전부 loud→supported (2026-07-22, branch feat-round17-families) ✅

**컨텍스트**: 외부 리뷰어 VITA_TEST_REPORT round-17(base `f906da2`/§4.5.194)이 "남은 벽 = 세 가족(A/B/C)"으로 지목 + CLI top-selection 이원 모델 요청. 3-agent 병렬 매핑(A/B/C 코드영역) 후 순차 구현. 공통: format_version 22 불변, 각 단계 full-suite green(3969→3994). 오너 지시=defer 없이 전부 구현하되 step이 크면 문의.

**A — queue/dynamic of unpacked record**: `pkt_t q[$]`가 parse E2002였음. **트리아지 정정**: dynamic array(`pkt_t a[]`)는 이미 동작·**QUEUE만** 갭(ROADMAP "queue-of-struct 이미 동작"은 PACKED struct queue 한정이었음). **PRIMARY**(저위험 1-line)=파서 `parse_unpacked_struct_decl`의 fixed-array match에 `Dim::Queue(None)` 추가→§4.5.191 packed-vector lowering 재사용(packed-struct queue 등록과 byte-identical). **end-to-end**(`q.push_back(p)`·`r=q[i]`)엔 packable struct를 whole-value로 표현해야 하는데, block-local packable struct가 per-member net이라 bare `p`가 undeclared. **초판 시행착오(revert)**: `expand_struct_call_args`에서 whole-vector struct actual을 member part-select로 확장 시도→**파서가 user-tf call(`f(r)`, 확장 필요) vs queue method(`q.push_back(p)`, whole 필요)를 구분 못 함**(파서가 tf-name registry 없음·`q.push_back`도 2-seg Call)→push_back·R5-B 회귀→revert. **오너 승인 후 unified whole-vector**(R5 매핑 agent 플랜, 파서-only 2-edit atomic): (Edit A) `try_tf_port_typedef`가 packable unpacked-struct tf-port를 packed struct처럼 **single-vector formal**(4th slot=struct_name→`bind_tf_port_struct`)로 라우팅[non-packable=per-member 유지]; (Edit B) `parse_unpacked_struct_decl` scalar 게이트가 packable 변수(모듈+블록 동시)를 whole-vector(`var_struct`+`struct_scalar_vars`)로 lowering[§4.5.192 tf-body 경로와 동일]. 이로써 packable struct=전 스코프 whole-vector로 통일→`f(structvar)` whole 전달(expand 불필요)·`'{…}` pattern·output/inout copy-out 全 자동 커버·`expand_struct_call_args` 무수정(packable은 이제 `var_struct`라 자동 skip). **적대 검증**: iverilog가 unpacked struct 자체 미지원→PACKED-struct oracle(module-var addr=30 MATCH)+hand-IEEE(whole-copy `q=p` value semantics: q mutate 후 p 불변). 신규 `queue_of_record.rs`×10.

**B — non-packable(string 멤버) unpacked-struct tf-local**: `{int;string} r;`를 tf-body 최상위에 두면 파서 derail(E2002 "expected '=' after lvalue"). **트리아지**: elaborate per-member 저장 기계는 이미 존재(begin/end **블록** 안에 두면 오늘도 PASS·module-scope도 PASS)·tf-body **최상위 decl 루프**만 `np_t r;`를 미인식. **fix(파서-only)**: block-local 청사진(`try_block_unpacked_struct_decl`=`peek_unpacked_struct_decl`+`parse_unpacked_struct_decl`)을 tf-body decl 루프에 미러—`parse_body_unpacked_struct_local`(packable)이 non-consuming None 반환 후 fallback으로 non-packable per-member 경로. Edit B(A의) 덕에 packable은 §4.5.192 whole-vector·non-packable은 per-member($unp$r$field·string 멤버=NetKind::String heap slot §4.5.167 재사용). correct-or-loud: whole-struct op(`q=r`)·dyn array=loud 유지. `frame_local_unpacked_struct.rs` 확장(§4.5.192 loud-pin test를 supported로 갱신).

**C — dyn-array-formal FUNCTION call inside frame body**: `s=sum(b)` in TASK body(리포트 케이스·KAT driver bytes2hex 형태)가 E3009였음. §4.5.177/179가 module-process level만 지원(handle_copy snapshot marker가 `&mut` executor 필요·gate `in_frame_body`). **§4.5.194 RefCell dyn_heap**이 `&self` frame executor의 marker heap→heap deep-copy를 가능케 함(reconcile: §4.5.194 "framed-nested 동작"은 dyn LOCAL·task→task forward였고 Family C는 별개로 여전히 loud였음). **3-change(format 무변)**: (1) elaborate `emit_frame_dyn_formal_markers` gate서 `in_frame_body` 제거(delay만 유지)+marker sid를 전용 `dyn_formal_marker_stmts`(Elaborator 필드·validator 전용·serialize 불요)에 기록; (2) `classify_frame_body`가 그 set의 Display marker 허용(§7.10 whole-copy Display는 set 밖→frame body서 loud 유지); (3) `run_frame_call`/`run_task`의 `_ => {}` 앞에 marker arm(`handle_copy_stmts` 멤버십 gate→`frame_dyn_copy_out(src,dst)`+`enforce_queue_bound`·frame body엔 dyn-formal marker만 존재하므로 executor는 handle_copy_stmts로 keying해도 정확·직렬화된 sidecar 재사용으로 staged flow도 커버). **경계**: TASK-body direct/buried·`$display(sum(b))`·function-**local**-arg·module-process 全 iverilog MATCH; recursion=기존 §4.5.173/194 reentry guard로 F4004 loud. **잔여 loud**: FUNCTION이 자기 dyn-formal `c`를 다른 함수로 재전달(`fsum(c)`)—framed function formal이 heap-resident라 `dyn_array_actual_net`이 re-forward source로 resolve 못 함(기존 "array formal 재전달" 갭·`nested_in_frame_body_loud` test가 커버). 신규 `frame_body_dyn_formal_call.rs`×7.

**CLI — 이원 top 모델**: `reject_worklib_flags`에 `allow_tops` param 추가→one-shot `vita`(및 `-f`)가 `--top <M>` 수용(velab로 pass-through·`run_vita_str_gated`가 `opts.tops`→elaborate roots)·vcmp/vrun은 거부 유지. auto-top ambiguity: elaborate `pick_roots`가 2+ root 반환 시(명시 top 없을 때) 신규 **W3057(W-ELAB-AUTOTOP-AMBIGUOUS)** 경고—iverilog-parity multi-root elaboration은 유지(v3_2b test)·silent-pick만 loud화(correct-or-loud). MsgCode 58→59(code.rs+doc-15 entry+bijection count+CLAUDE.md). 신규 `multi_top.rs` dual-top tests×6.

**교훈**: **두 자료표현(struct per-member vs whole-vector)이 충돌하면 context-blind expansion을 패치하지 말고 표현을 통일하라**(파서는 call-site서 user-tf vs built-in method 구분 불가)·**PRIMARY(저위험)와 full-close(고위험 R5 재라우팅) 분리 후 오너 승인**이 안전(owner "ask if large" 준수)·**인접 완료 슬라이스 재사용**(§4.5.191 lowering·§4.5.192 whole-vector·§4.5.194 RefCell·§4.5.167 string slot·block-local 청사진)이 신규 표면 최소화·**리포트 갭도 fresh-probe 재트리아지**(dynamic array 이미 동작·B는 파서-only·queue triage 불일치 정정)·**전용 marker set이 validator over-relax 방지**(handle_copy_stmts 3-producer 중 dyn-formal만 frame body 허용)·**executor는 직렬화된 handle_copy_stmts로 keying 가능**(frame body엔 dyn-formal marker만 존재→sidecar 추가·format bump 회피). **잔여 follow-on**: function dyn-formal 재전달·D-가족(hierarchical fn call `u1.f(x)`=large·$systask-in-subset-fn·task inout array formal). design agents=4(A/B/C 매핑 + R5 tf-port reroute). 신규 test 3파일(`queue_of_record`·`frame_body_dyn_formal_call` + `multi_top` 확장)+기존 갱신. **+25 tests(3969→3994)·MsgCode 59·format 22 불변.**

> **round-16 리포트 대응 7-슬라이스(§4.5.187~193, 2026-07-21)** — 외부 리뷰어 VITA_TEST_REPORT round-16(base `4cd6f54`/§4.5.185)의 잔여 갭을 현재 HEAD(§4.5.186)에서 iverilog로 재현·트리아지 후 tractable 항목을 순차 구현. 공통: correct-or-loud, format_version 22 불변, 각 슬라이스마다 full-suite green. **남은 리포트 항목(executor-bound)=§4.5.194서 전부 RESOLVED**(dyn_heap RefCell interior-mutable화).

#### 4.5.194 round-16 executor-bound gaps 전부 loud→supported (dyn_heap RefCell interior-mutable화 · 6-phase) (2026-07-22, branch feat-dyn-heap-refcell-executor) ✅

**발굴 경위**: round-16 리포트 대응(§4.5.187~193) 후 남은 4개 갭이 전부 **executor-bound**로 분류됨 — `&self` 동기 frame executor(`run_frame_call`=함수·`run_task`=subset task)가 heap-op(`new[]`·dyn element write)을 **구조적으로** 수행 불가. 사용자가 `&mut` frame-executor 확장을 지시. 3-agent 아키텍처 조사로 **진짜 원인**이 executor의 `&self` 자체가 아니라 `dyn_heap: Vec<Option<DynObj>>`가 **plain Vec(비-interior-mutable)** 임을 규명(`frame_stack`/`static_store`/`class_heap`은 이미 `RefCell`). 사용자가 4-선택지 중 **"dyn_heap → RefCell"**(class_heap 선례) 채택.

**설계(design doc=`docs/superpowers/plans/2026-07-21-dyn-heap-refcell-executor.md`·format 22 불변[dyn_heap=런타임 SimState·미직렬화])**:
- **Phase 0(RefCell 배관·무행동변경)**: `dyn_heap`→`RefCell<Vec<Option<DynObj>>>`. 47개 접근 site를 `borrow()`/`borrow_mut()`로 재범위(§C6 borrow discipline: heap guard를 nested-call 넘어 유지 금지). `&mut DynObj` 반환 `dyn_entry`→closure-scoped `with_dyn_entry`(escaping ref는 RefCell 못 넘김). 3944 green 불변.
- **Phase 1a(V2A-dyn·subset task input dyn formal)**: pure-compute subset task(관찰문 없음)의 `input byte b[]`. elaborate reject 병합→`validate_frame_body`. exec.rs 2 subset 분기(+run_task nested arm)서 `frame_dyn_snapshot_formals`(caller→formal deep-copy·pass-by-value IEEE §13.5.1)·`frame_dyn_free`. **nested subset call(run_task 내부)이 exec.rs 미경유→별도 snapshot 필요**(적대 nesting서 r=0 silent-wrong 발굴·수정). `frame_dyn_reentry_ok`를 `&self`화(call_fatal 래치).
- **Phase 1b(write-path)**: `&self` executor의 dyn element write(`loc[i]=v`·formal local-copy write). `dyn_write`/`enforce_queue_bound` `&self` 완화. `frame_write_lvalue`의 dyn-handle 분기가 fatal 대신 **인덱스 해소(OOR_DROP 2^30 sentinel·module 경로 동일)+`dyn_write`**. 함수·태스크 공통→§4.5.178 function write-loud 테스트가 pass-by-value supported로 전환(caller 격리 검증·iverilog는 by-ref).
- **Phase 2a(V5·function/task body dyn LOCAL + new[])**: `new[]`=`SysTaskId::DynNew`가 classifier reject+executor skip이었음. `alloc_dyn_array(&self)` 공유 코어(builtins DynNew도 위임)·`frame_dyn_new(&self)`. `run_frame_call`/`run_task` BB loop에 SysTask(DynNew/DynDelete) arm. lifecycle=`frame_dyn_reentry_ok_from`/`frame_dyn_free_from(first_slot)`—**함수 경로는 `first_slot=np`로 FORMAL 제외**(§4.5.177 함수 input formal은 caller가 pre-snapshot→진입 시 legitimately Some이라 reentry false-fire 방지). classifier가 in-frame dyn net 타겟 DynNew/DynDelete 허용.
- **Phase 2b(V2B·output/inout dyn formal)**: body는 이미 씀(V5/write-path)→**copy-OUT**만 부재. `frame_dyn_copy_out`(formal→caller deep-copy·snapshot의 역방향)·`frame_dyn_out_bind`(DynArray out-slot 감지). 4 apply site(exec.rs suspendable Return + subset top + subset nested, state.rs run_task nested)서 감지·copy-out. **free-순서 위험**(Site 1·4는 free가 copy-out보다 먼저)→reorder(copy-out 먼저). reentry/snapshot/free를 무조건 호출(self-gating)로 pure-output-dyn 커버. `is_output_or_inout_dyn_array_formal`·gate/reserve/emit×2·`build_task_frame_set` framing(output/inout dyn formal task 강제 frame). **함수 output formal도 이미 run_task 경로**라 동일 수정이 커버.

**적대 검증(iterate-until-no-issue·2-lens)**: differential(live iverilog·macOS `perl -e 'alarm N; exec'` 래핑)+soundness(hand-IEEE). V2A-dyn(sum 60·pass-by-value 격리·signed·nested forward 12·write local-copy r=999 caller 격리) · V5(sum 60·dyn-index 14·fresh-per-call·new[](src) copy 29·delete 0·nested func 60·subset task 6) · V2B(task output sz=3 1 2 3·replace-larger sz=2 7 8·inout modify 11 12 13·inout resize sz=3 1 2 99·mixed i/o/io·function hand-IEEE). **divergence(전부 correct-or-loud/IEEE-correct·silent-wrong 0)**: unwritten output formal=IEEE §13.5.2 empty copy-out(vita sz=0·iverilog by-ref sz=3=非준수) · string/real/class-handle element dyn formal=loud(input helper도 동일 제외) · recursion with dyn local/formal=F4004(per-activation heap stash follow-on) · queue/assoc formal=loud(≠dyn array).

**교훈**: **executor `&self`가 진짜 제약이 아니라 dyn_heap의 plain-Vec가 원인**(class_heap RefCell 선례가 해법 제시) · aggressive 조사가 "executor 재작성"을 "RefCell 배관"으로 축소 · **인접 feature 적대 검증이 pre-existing/신규 silent-wrong 발굴**(nested subset r=0) · lifecycle `first_slot`이 함수 formal-vs-local 비대칭(caller pre-snapshot) 해결 · copy-out free-순서가 미묘(formal을 free 前 읽어야) · **함수 output formal이 이미 run_task 경로라 task 수정이 자동 커버** · unwritten-output/string-element 등 divergence는 전부 IEEE-correct 또는 일관된 loud 경계. 6 커밋(Phase 0/1a/1b/2a/2b)·+25 tests(3944→3969)·format 22 불변·clippy/fmt clean.

#### 4.5.193 output unpacked-fixed array formal on TASK loud→supported (md-packed slot → packed-temp out-bind → post-call unpack) (2026-07-21, branch feat-task-output-array) ✅

**발굴 경위**: round-16 리포트의 "unpacked-array formal on task"(sha2 16) 중 §4.5.188(input)이 처리하지 못한 **OUTPUT half**(`shaN_compute`/`hex2bytes`-style `output byte digest[N]`). vita loud("an OUTPUT/INOUT array formal is pass-by-reference").

**설계(§4.5.188 확장·heap/format 무변화)**: body는 input과 동일한 md-packed `[count][elem_w]` slot을 씀(md-packed element WRITE §4.5.97). (1) `reserve_frame_task`의 md-packed 예약을 `Input|Output`으로 확장. (2) UARR guard를 output fixed-array도 면제(framed 한정). (3) `emit_frame_task_call` Output arm에 array branch: 신규 caller-side packed temp net 생성→**out-bind(md-packed slot → temp whole-net)**[기존 scalar copy-out 그대로]. (4) `ret` block(call 후·exit copy-out 뒤 동기·suspendable 양쪽서 실행)에 **unpack** 방출: `for i in 0..count { caller[i] = packed[i*ew +: ew] }`(AST+`lower_stmt`로 array-element write/part-select read를 정상 lowering 재사용·bit-faithful copy라 signedness는 caller 배열 것·later read서 적용).

**correct-or-loud**: INOUT array formal·non-bare array actual(slice)·classifier-reject=loud. **IEEE §13.5.2 pass-by-value**: output formal은 default(2-state 0·4-state X)서 시작→body가 안 쓴 element는 그 default가 copy-out(caller 이전 값 덮어씀). nested call의 module-net unpack write는 frame subset check가 포착.

**적대 differential(element-wise iverilog ref)**: byte[4] suspendable·input+output 병용 synchronous·wide logic[15:0]·signed(−5/−100)·two-call isolation·partial-write default(00/22/00). LOUD=INOUT/non-bare. `frame_task_output_array.rs`×8·§4.5.188 pinned test 갱신·**3944 green**(+8).

#### 4.5.192 packable unpacked-struct scalar body-local in task/function loud→supported (V8) (2026-07-21, branch feat-frame-local-unpacked-struct) ✅

**발굴 경위**: round-16 V8(TB=partial·tb_partial:372 task-local unpacked struct). `rec_t p;`를 function/task body에 두면 E2002("expected '=' after lvalue")—`rec_t`가 statement lvalue로 파싱됨. packed struct body-local은 이미 동작·**unpacked만** gap.

**근본 원인**: unpacked struct 타입은 `unpacked_struct_layouts`에만 있고 `typedefs`엔 없음(주석 명시)→tf-body decl loop의 `peek_block_typedef_decl`이 miss→decl loop break→statement 파싱.

**설계(parser-only·§4.5.190/191 재사용)**: (1) tf-body decl loop에 `parse_body_unpacked_struct_local` branch: packable unpacked-struct scalar를 인식→**packed-vector frame-local**(`logic/bit [W-1:0] p;`)로 lowering+`var_struct`/`struct_scalar_vars` 등록. (2) `struct_field_select`(scalar `s.field` geom)에 `packable_record_layout` fallback 추가(§4.5.190 `struct_array_field_geom`와 동일 패턴)→read+write 양쪽 `p.field`가 part-select desugar.

**correct-or-loud**: non-packable record(string/real/nested member)·array body-local·decl-init `'{…}`=loud. module-scope record는 member-net(`$unp$var$field`) 표현 유지(이 branch는 tf-body 한정). **적대**: task/function local field write→read·per-call frame re-init(mixed-width)·runtime `'{…}` pattern·4-state read-before-write=X. 회귀: module member-net·packed body-local 불변. `frame_local_unpacked_struct.rs`×8·**3936 green**(+8).

#### 4.5.191 FIXED 1-D array of packable unpacked struct loud→supported (V6) (2026-07-21, branch feat-fixed-record-array) ✅

**발굴 경위**: round-16 V6(TB=top·axi_mem_model:134 memory model `mem[addr].field`). FIXED unpacked-struct 배열(`entry_t mem[N]`)이 loud("array of unpacked structs unsupported")—dynamic record array(record_array_vars)·SoA·scalar record만 지원.

**설계(parser-only·§4.5.190 재사용)**: `parse_unpacked_struct_decl`의 fixed-array loud 앞에 branch: packable record 고정배열을 **packed-vector 고정배열**(`logic/bit [W-1:0] mem[N]`)로 lowering+`struct_1d_array_vars`/`var_struct` 등록→`mem[i].field`가 §4.5.190 packed-struct-array desugar 재사용. `struct_array_field_geom`에 `packable_record_layout` fallback(StructLayout `.field()` 동일 shape). `bit`/`logic` element로 2-state-0/4-state-X default.

**correct-or-loud**: non-packable record·multi-dim·decl-init=loud. queue-of-struct·dynamic-record-array 무영향. **적대**: axi_mem_model shape {addr,data,valid} round-trip(field write→read·whole-element MSB-first packing)·runtime-index·2-state 0/4-state X default·`mem[i]='{…}` asymmetric pattern. `fixed_record_array.rs`×8·round-10 pinned test 갱신·**3928 green**(+8).

#### 4.5.190 arr[i].field on packed-struct 1-D array loud→supported (2026-07-21, branch feat-struct-array-member) ✅

**발굴 경위**: fresh-area probe서 packed-struct 1-D 배열 element field access(`arr[i].field`·register-file/memory idiom)가 read/write 모두 E3010("undeclared hierarchical name"). scalar `s.field`·dynamic record array/SoA는 동작·**packed-struct 배열만** gap. iverilog는 이 접근서 assertion abort/reject(오라클 없음).

**설계(parser-only·기존 packed-struct member 머신러리 일반화)**: (1) `struct_member_expr`(READ)를 base=EXPR로 일반화[`struct_member_expr_of`]—BitSelect element `arr[i]`에 part-select. (2) `parse_struct_field_lval`(WRITE)를 base=`HierPath`→base=`Lvalue`로 일반화. (3) postfix loop(read)+lvalue loop(write)에 branch: `arr[i].field`(arr∈`struct_1d_array_vars`)→field geom(`var_struct`→`struct_layouts`)로 element part-select `arr[i][off+w-1:off]`(whole-field sign wrap·trailing sub-select·RMW field write 상속).

**검증(no oracle→hand-IEEE+self-consistency)**: field offset을 plain packed-vector part-select(iverilog가 pin)와 교차검증(`arr[0].a`==`arr[0][15:8]`==11)·vita↔vita self-consistency(field R/W==manual part-select)·signed sign-extend·3-field offset·trailing sub-select·RMW가 다른 field 보존. 회귀: whole-element·`arr[i]='{…}` pattern 불변. AST/format 무변화. `struct_array_member.rs`×10·**3920 green**(+10).

#### 4.5.189 SILENT-WRONG 수정: loop-body block-local initializer ran once, not per-entry (2026-07-21, branch feat-frame-block-init) ✅

**발굴 경위**: round-16 "automatic block-local w/ initializer" 조사 중 iverilog 차분서 SILENT-WRONG 발견. automatic task/function의 LOOP body에 `int t = f(k);`(initializer 有)를 두면 vita가 frame ENTRY서 **1회만** init(loop var를 entry 값으로)—iverilog는 매 block entry 재실행. `for(k) begin int x=k*10+1; end`가 vita `1,1,1` vs iverilog `1,11,21`.

**근본 원인**: `lower_frame_task_body`/`lower_frame_func_body`가 `collect_block_local_decls`로 모든 nested block-local init을 frame ENTRY(활성화당 1회)에 방출—**코드 자체 주석이 "single-entry approximation that is exact for the common single-entry case"** 라고 자인. 루프는 block을 N회 재진입하나 init은 1회.

**설계(IEEE §6.21 automatic lifetime)**: nested block-local init을 각 block의 OWN entry(`Block` arm of `lower_stmt`·`in_frame_body` gate)에 방출→루프 내부 init이 매 반복 재실행. top-level body_decls는 frame entry 유지(파서가 outermost body block을 `decls: []`로 wrap→중복 없음). storage slot은 여전히 지속→no-init read-before-write는 iverilog parity(`int acc; acc=acc+1;` 누적). MODULE-process block-local은 static(once-at-t0) init 유지(`in_frame_body`-only).

**적대 differential 全 MATCH(이전 MISMATCH)**: loop init in task(1/11/21)+function(33)·nested loops(0/1/10/11)·sibling decl ordering·while-loop·init-from-function-call. 회귀 MATCH: no-init persists(1/2/3)·non-loop block once-per-call(10/14)·module initial static(1/1/1). content-only(golden churn 0·format 22). `frame_block_local_init.rs`×9·**3910 green**(+9).

#### 4.5.188 input unpacked-fixed array formal on TASK loud→supported (2026-07-21, branch feat-task-unpk-formal) ✅

**발굴 경위**: round-16 리포트의 단일 최대 잔여 task 클래스(sha2 16 `shaN_compute`/`hex2bytes`/`shaN_block`·sha3 3). `input byte b[4]`·`input logic [63:0] w[80]` 같은 unpacked-FIXED array formal이 task서 blanket-loud("task has an unpacked-array formal")—FUNCTION 경로는 md-packed `[count][elem_w]` frame slot으로 §4.5.82/97부터 지원.

**설계(함수 경로를 frame-TASK 경로로 미러·3 site)**: (1) `reserve_frame_task`: `input` classify_array_formal-Ok formal을 `reserve_frame_func`와 동일 md-packed value slot 예약(packed_dims+dim_desc+frame_arr_formal_meta+2-state whole-slot coercion). (2) `emit_frame_task_call`: whole-array actual을 `lower_array_actual_packed`로 slot value에 pack(call-site concat·`emit_frame_call`과 동일). (3) task-call UARR guard: `input` unpacked-fixed formal 면제—단 task가 FRAMED(`task_frame_idx`)일 때만(static task inline binding엔 md-packed slot 無→silent truncation 대신 loud). value slot이라 동기 `run_task_call`·suspendable 양쪽 동작.

**적대 differential(element-wise iverilog ref·iverilog는 unpacked subroutine port reject)**: byte[4] sum(suspendable+non-suspendable+output-scalar)·signed byte(−5/10/−3)·wide logic[63:0]×4. 적대: input element write가 caller에 no-leak(pass-by-value IEEE §13.5.1)·two-call isolation·OUTPUT/INOUT array formal=loud(pass-by-ref)·static task unpacked formal=loud·size mismatch=loud. round-6 pinned test 갱신. `frame_task_unpk_formal.rs`×9·**3901 green**(+9).

#### 4.5.187 $fopen runtime filename loud→supported (string literal → variable/concat/packed-reg) (2026-07-21, branch feat-fopen-runtime-name) ✅

**발굴 경위**: round-16 리포트의 새 file-I/O 층(sha2 CAVP walker `load_vector`/`rsp_next` 9 site). CAVP walker가 vector-file path를 `string` 변수에 만드는데 `$fopen`이 non-literal arg를 loud-reject("$fopen arguments must be string literals (v7)")—file-driven testbench 전면 차단.

**근본 원인**: elaborate `fopen_special` gate가 `ExprKind::StrLit` arg만 수용·engine `k_fopen`이 `Const{StrUtf8}` expr만 resolve(둘 다 vita `string` 타입[P2-C] 이전).

**설계(§21.3 iverilog parity)**: (1) elaborate: StrLit-only gate 제거·name/mode를 일반 expr로 lower. (2) engine `k_fopen`: `resolve` helper가 각 arg를 Const{StrUtf8}→`const_string`·runtime STRING value(`is_str`)→`to_str_bytes`·기타 packed value→ASCII in reg(`fmt_packed_chars_min` NUL-strip)로 resolve(3형 모두 valid 파일명).

**적대 differential 全 MATCH(iverilog 13.0)**: string-variable path(write+reopen+$fgets)·concat path `{"pre_", base, ".txt"}`·CAVP reader($fopen(var)→$fscanf %d/%h)·open-failure→0(완화된 gate서 silent success 없음). 기존 file_io(9)+sysread_fgets(12) 스위트 green. `fopen_runtime_name.rs`×4·**3892 green**(+4).

#### 4.5.186 constant-function evaluation in const contexts loud→supported (elaborate-time integer function-body interpreter) (2026-07-21, branch feat-const-function-eval) ✅

**발굴 경위**: §4.5.185 후 fresh-area probe($cast·let·const-func 차분) 중 `localparam W = clog(256)`(clog=while-loop 함수)가 vita E3009 "not a foldable constant expression" vs iverilog `8` 정상으로 발견. SV의 흔하고 중요한 idiom(파라미터 계산용 상수 함수·커스텀 `$clog2`). 사용자가 4 선택지 중 이 feature 선택.

**근본 원인**: `const_eval_in_scope`(param fold 도메인)가 `$clog2`/`$bits`/Binary/Ternary/Ident(param) 등은 처리하나 **사용자 함수 `Call`은 미처리**→`_ => None`→param fold 포기→E3009.

**설계(elaborate-time 정수 함수-body 인터프리터·format 무변화)**: (1) **ordering**: 모듈 param fold(pass 3)가 func_table populate(pass 3.5)보다 먼저라 신규 `const_func_table`을 param fold **전에** populate(save/restore는 func_table과 동일 스코프). (2) `const_binop` 추출(Binary 폴딩을 free fn으로·`const_eval_in_scope`와 인터프리터가 동일 폴딩 공유·중복 0). (3) `const_eval_in_scope`에 `Call` arm 1개 추가→`eval_const_call`. (4) 인터프리터 3함수: `eval_const_env`(env-aware expr·single-seg Ident는 local env 우선 후 param scope)·`eval_const_call`(input formal→arg값 bind·body-local→folded init/0·body 실행·return을 declared width로 coerce[`coerce_int_width`])·`exec_const_stmt`(Block+decls·blocking `=`·if/else·for/while/repeat·return; ConstFlow=Normal|Return). **i64 도메인**은 기존 const_eval와 동일(intermediate-width 부정확은 §2 기지 residual과 동일 클래스·신규 아님).

**★correct-or-loud 극도 엄격(param 값 silent-wrong=P0-5 최악·width 무흔적 오염)**: 정수 도메인 밖은 전부 None→LOUD. real/string return·formal·local, output/inout/ref formal, unpacked-array formal, 런타임 신호 참조(param/local 아님→`lookup_scoped` None), system task/NonBlocking/timing/fork/case/미모델 statement, arity mismatch, **recursion depth cap(64)**, **loop step cap(100K→~1s에 loud·비종료 루프가 elaboration hang 대신 loud)** 全 loud. i64 오버플로는 `checked_*`→None.

**적대 differential(vita vs iverilog)**: 지원 全 MATCH — clog while-loop(width서·8)·return expr(42)·for-loop sum-of-squares(55)·recursion factorial(120)·function-name return(42)·multi-arg+nested(9)·byte-return coercion(300→44)·param-arg(7)·chained localparam·negative arg abs(42)·**param+runtime 동일 함수**(runtime path 무변화 회귀). **LOUD 유지**: real func·non-terminating loop(step cap·~1s)·system task in body·array formal·runtime-signal ref. format 22 불변(elaborate value-only·AST/sim-ir 무변화·schema_hash 무관). **3888 green**(+15). ⭐교훈: ① **const_eval에 Call arm + 자체 인터프리터로 상수함수 지원**(func_table를 param fold 전 populate하는 ordering이 핵심). ② **`const_binop` 추출로 폴딩 공유**(const_eval와 인터프리터 동일). ③ **param 값 silent-wrong=P0-5라 correct-or-loud 극도 엄격**(정수 도메인 밖·비종료·런타임 참조 全 loud). ④ **step/depth cap이 비종료·과재귀를 hang 대신 loud**(cap은 unopt build서 prompt하도록 100K). ⑤ **i64 도메인이 기존 const_eval와 일관**(intermediate width는 동일 §2 residual). 상세=본 엔트리·ROADMAP §3.

#### 4.5.185 `$bits(TYPE)` loud→supported (parser type-size fold) (2026-07-21, branch feat-bits-of-type) ✅

**발굴 경위**: §4.5.184 후 fresh-area probe($display 포맷·X/Z·cast·$bits 차분) 중 `$bits(logic[15:0])`·`$bits(s_t)`가 vita 거부(타입 keyword→E2002 parse error·typedef name→E3010 "undeclared variable") vs iverilog `16`·`7` 정상으로 발견. `$bits(변수)`·`$bits(struct변수)`·`$bits(mem[i])`는 동작(elaborate `bits_prescan`). 즉 **`$bits`의 TYPE 인자만** gap — SV §20.6.1의 흔한 idiom(`logic [$bits(T)-1:0]`).

**근본 원인**: `$bits`는 elaborator서 처리(변수명 키 `bits_prescan`)인데, TYPE 인자(`logic[15:0]` keyword·`s_t` typedef name)는 **valid expression이 아님**→파서가 `call_args()`서 파싱 실패(keyword) 또는 elaborate서 미등록 변수로 loud(typedef name).

**설계(파서 type-size fold·format 무변화)**: `$bits`는 compile-time 상수이고 파서가 타입 폭 정보(`struct_layouts`·`typedefs`·atom kinds) 보유→**파서서 `$bits(TYPE)`를 IntLit로 fold**. SystemTask arm 인터셉트: `name=="$bits"` & `(` 뒤에 `parse_bits_type_arg`가 (a) 데이터 keyword(`net_var_kind`) + opt signedness + packed dims 또는 (b) bare type NAME(다음 토큰 `)`)이면 폭 계산 후 `dec_lit(w)` return; 아니면 pos 복원→normal `$bits(expr)` 경로(elaborate). `bits_of_type_name`: struct=Σfield·union=MAX field·typedef=`member_width_kind × ∏packed`. **`logic[$bits(T)-1:0]` decl range·`parameter W=$bits(T)`도 동작**(range/param이 `expr(0)`→expr_primary 경유).

**★correct-or-loud(silent-wrong 방지)**: 초판이 `$bits(real)`=**1**(silent-wrong·`member_width_kind(Real)`=atom 아님→1) 유발→적대 probe서 발견. **fix**: 두 분기 모두 **INTEGRAL 타입만 fold**(`member_kind_is_integral`)—`real`/`realtime`/`string`/`event`/class는 None→loud(correct-or-loud). `$bits(real_var)`=64는 elaborate 경로라 무영향.

**적대 differential(vita vs iverilog)**: 지원 全 MATCH — struct(7)·union(8·equal-width)·int(32)·byte(8)·shortint(16)·longint(64)·time(64)·integer(32)·logic[15:0](16)·logic[1:0][3:0](8)·bit[7:0](8)·logic(1)·word_t typedef(12)·signed alias(8)·enum(3)·decl-range idiom·parameter idiom. **회귀 clean**: `$bits(var/struct/mem[i]/member/real_var)`. **LOUD 유지**: real/realtime/string(초판 silent 1→loud)·scoped `pkg::T`(follow-on)·unknown name. iverilog는 `$bits(real)`·non-uniform packed union 자체를 거부(vita가 union=MAX로 초과·SV-correct). format 22 불변(파서 fold·AST/sim-ir 무변화·schema_hash 무관). **3873 green**(+11). ⭐교훈: ① **`$bits`는 elaborate 처리지만 TYPE 인자는 파서 fold가 자연**(파서가 타입 폭 보유). ② **적대 probe가 초판 silent-wrong(`$bits(real)`=1) 즉시 발견→INTEGRAL gate**(비-integral은 loud). ③ **파서 fold라 decl-range/param 등 모든 expression 위치서 동작**. ④ **iverilog 초과분(union MAX·real reject)은 vita가 SV-correct**. 상세=본 엔트리·ROADMAP §3.

#### 4.5.184 multi-dimensional packed array struct/union member loud→supported (parser flat-width + element-stride desugar) (2026-07-21, branch feat-multidim-packed-member) ✅

**발굴 경위**: §4.5.183 후 fresh-area probe(packed struct/union 연산 차분) 중 `typedef union packed { logic [7:0] byte_v; logic [1:0][3:0] nib; }`가 vita E2002(parse error·2번째 `[3:0]`서 "expected identifier") vs iverilog `u.nib[0]=b·nib[1]=a` 정상으로 발견. 격리: **독립 multi-dim packed net(`logic [1:0][3:0] x`)은 지원**(MATCH)·오직 **struct/union 멤버**만 거부(파서 `parse_struct_member_type`가 단일 `Option<Range>`만 파싱·6817 주석에 "multi-dim packed member is unsupported in v1" 명시된 의도적 v1 한계).

**핵심 통찰(파서-only·format 무변화)**: struct 멤버 access(`s.m`/`s.m[i]`)는 **파서서 part-select Expr/Lvalue로 desugar**(`struct_member_expr`/`parse_struct_field_lval`)→elaborator/IR은 일반 part-select만 봄→multi-dim 지원은 **전부 파서에** 국한·**AST(StructMember)만 확장·sim-ir/format 22 불변**.

**설계**: (1) AST `StructMember`에 `packed_dims: Vec<Range>`(inner 2nd+ 차원·single-dim은 empty). (2) `parse_struct_member_type`가 첫 range 뒤 추가 `[a:b]` 차원 loop 수집. (3) 신규 `member_flat_dims`가 `flat = base × ∏(inner widths)`·`elem_stride = ∏(inner widths)` 계산(single-dim=stride 1→byte-identical). (4) `StructLayout` field tuple에 `elem_stride`(8번째)·3 build site(struct/union/record). (5) `struct_field_select`→`struct_member_expr`→`parse_struct_field_sel`(READ)·`parse_struct_field_lval`(WRITE) 전부 stride threading. (6) `parse_struct_field_sel`가 `elem_stride>1`이면 bare `s.m[i]`→`Indexed{offset:i*stride, width:stride, PlusColon}`(기존 IndexedPart 머신 재사용). **correct-or-loud**: element WRITE(`s.m[i]=x`)·element RANGE(`s.m[i:j]`)·ascending/non-zero-base outer·nested `s.m[i][j]`·record-array multi-dim member는 전부 loud(follow-on·never silent-wrong). runtime index `s.m[i]`는 지원(iverilog 13.0은 constant만 요구→vita 초과·hand-verified).

**적대 differential(vita vs iverilog)**: 지원 全 MATCH — union(`ab b a`)·struct+neighbor(`abc c b a`)·whole R/W(`5a a 5`)·3-dim first-level(`1234 34 12`)·signed(`0 f`)·copy/compare(`1 a 5`)·3-byte(`[2:0][7:0]`)·offset-around-multidim·arithmetic·runtime(sum 38·hand-verified). **회귀 clean**: single-dim member+bit-select·non-zero-LSB member sub-select 全 MATCH. **LOUD 유지**: element write·range·ascending·`s.m[i][j]`(E3009)·record-array(E3010). format 22 불변(파서 desugar만·AST StructMember 확장은 sim-ir 미도달·단 AST `SourceUnit` `.vu` 해시는 re-pin—`hdl-ast/tests/schema_hash.rs`). **3862 green**(+13). ⭐교훈: ① **struct 멤버가 파서서 part-select로 desugar됨을 파악→다층 예상이 파서-only로 축소**(elaborator/IR 무변화·format bump 회피). ② **기존 IndexedPart 머신 재사용**(element select=`i*stride +: stride`). ③ **elem_stride=1 sentinel이 single-dim byte-identical 보장**. ④ **correct-or-loud 경계 다수**(write/range/ascending/nested/array=follow-on loud). ⑤ **runtime index가 iverilog 초과**(oracle 없어 hand-verify). 상세=본 엔트리·ROADMAP §3.

#### 4.5.183 SILENT-WRONG 수정: block-local `string s[] = '{…}` dyn-array init이 조용히 drop → supported (module-scope parity) (2026-07-21, branch feat-block-local-string-dyn-init) ✅

**발굴 경위**: §4.5.182(queue/dyn `{…}` concat) 적대 검증 중, string dyn-array가 concat에서 loud 유지됨을 확인하려고 baseline `string s[] = '{"a","b","c"}`(apostrophe 형태)를 테스트하다 **vita SZ=0(빈 배열)·빈 문자열 vs iverilog SZ=3(a b c)** 발견. int dyn(`int s[]='{4,5,6}`)은 SZ=3 정상. 축소 격리: **module-scope 선언은 정상**(SZ=3)이나 **동일 선언을 initial/always BLOCK 내부(block-local)에 두면 SZ=0**.

**근본 원인**: block-local decl-init 수집기(`hoist_block_local_nets` 내부, 8667-8705)의 `scalar_string` 판정이 **base range/packed dims만** 보고 per-name unpacked dims는 무시→`string s[]`(dimensioned)도 `scalar_string=true`로 분류. 이어 push gate가 `name.unpacked.is_empty()`라, dimensioned string(`s[]`, unpacked=`[Dim::Dyn]`)은 **push=false→`'{…}` init이 t0 pre-sweep에 수집 안 됨→조용히 drop**→배열이 빈 채. module-scope 수집기(`collect_var_init_drivers`, 12391-12403)는 이미 `is_dyn_str_init` 특례(unpacked.len==1 && Dim::Dyn && AssignPattern)로 push→정상. 즉 **두 scope 수집기의 비대칭**이 원인(module엔 특례 有·block-local엔 無)→block-local만 pre-existing silent-wrong.

**fix(supported·module parity)**: block-local `scalar_string` push gate에 module-scope `is_dyn_str_init`와 동일 조건 미러 추가—`name.unpacked==[Dim::Dyn]` & init이 `AssignPattern`이면 push. 기존 `pending_block_local_string_inits`(deferred string list·module string init 뒤 flush) 라우팅을 그대로 타고 flush(`dyn_decl_init_stmts`: `new[N]`+element writes)로 확장. **왜 loud 아니라 supported**: module-scope가 이미 동작(supported)하므로 block-local도 parity로 supported화가 맞음(loud면 비대칭 온존). **correct-or-loud**: fixed(`s[2]`)/multi-dim/non-`'{…}` string은 조건 미매칭→push 안 됨(기존 loud 유지)·`{…}` concat string은 §4.5.182가 handle-gate서 loud 유지(silent-empty 방지).

**적대 differential(vita vs iverilog)**: 全 MATCH — block-local basic(foo bar baz 3)·named block(x y)·single(only 1)·`new[](copy)`+element write(a b c)·always block-local(p q)·module-scope 회귀(a b c)·scalar string 회귀(hello)·int dyn 회귀(4 5 6). **LOUD 유지(silent-wrong 0)**: string `{…}` concat block-local·fixed `string[2]`(iverilog는 compile OK지만 별개 gap·correct-or-loud). format 22 불변(elaborate collect gate만·AST/IR 무변화). **3849 green**(+9). ⭐교훈: ① **인접 feature(§4.5.182 string 제외)의 적대 검증이 pre-existing silent-wrong 발굴**(string dyn baseline 확인하다). ② **scope별 수집기 비대칭이 silent-wrong 원천**(module엔 `is_dyn_str_init` 有·block-local엔 無→drop)·분류 flag(`scalar_string`)가 per-name unpacked dim을 무시(base만 봄). ③ **module-scope가 이미 supported면 block-local도 supported로**(loud 아님·parity 복원). `block_local_string_dyn_init.rs`×9. 상세=본 엔트리·ROADMAP §2.

#### 4.5.182 queue / dynamic-array `{…}` (unpacked-array concat) decl-init loud→supported (Concat → `'{…}` flush 라우팅) (2026-07-21, branch feat-queue-dyn-brace-init) ✅

**발굴 경위**: §4.5.181 후 fresh-area probe 계속 — queue 메서드(`.insert`/`.delete(idx)`) 차분 중 `int q[$] = {1,2,3,4};`가 vita E3009 vs iverilog 정상으로 발견. 격리하니 메서드는 동작하고 **initializer 자체**(`{…}` 비-apostrophe 형태)가 원인. `int d[] = {1,2,3};`(dyn array)·`{a,b,c}`(scalar vars)·`{42}`(single) 全 동일 loud.

**근본 원인**: vita가 queue/dyn-array decl-init을 **`'{…}` assignment-pattern 형태로만** 수용(handle-gate가 `matches!(init.kind, AssignPattern)` 게이트). SV §10.10 **unpacked array concatenation** `{e0,e1,…}`도 queue/dyn-array의 합법적 initializer인데(iverilog ✓) `ExprKind::Concat`이라 게이트 통과 못 해 E3009. `fixed unpacked array`(iverilog가 `0 0 0`으로 비표준 처리)·`{n{x}}` replication(iverilog `5 0 0` dubious)·nested array concat(`{a,3}` iverilog compile-fail)은 경계 밖.

**설계 (Concat → `'{…}` flush 라우팅)**: scalar-element target에선 `{…}`와 `'{…}`가 같은 element list를 표현하므로, Concat init을 기존 `'{…}` var-init flush 확장(`dyn_decl_init_stmts`: queue=push_back sequence·dyn=`new[N]`+element writes)에 **그대로 라우팅**. 신규 헬퍼 `dyn_pattern_elems(init)`가 `AssignPattern(parts)|Concat{parts}`에서 균일하게 element slice 추출. 3개 게이트 중 **2곳만** 수정: handle-gate(Concat 수용, **string 원소 제외**)·flush dispatch(Concat도 라우팅). collect 경로는 `fold_init`(IntLit만)·`const_eval_in_scope`(Concat arm 無) 둘 다 Concat에 None이라 자동으로 pending에 태움(무수정).

**correct-or-loud BY CONSTRUCTION**: 라우팅이 `{…}`를 `'{…}`와 byte-identical하게 만들어 **모든 원소 타입에서 기존 `'{…}` 경로의 correct-or-loud 상속**. 유일한 의미 차이 = array-typed 원소(concat flatten vs pattern positional)인데, 그 케이스는 scalar surface가 없어 `'{…}`가 이미 loud(`'{a,3}`→E3009 "no whole-value surface")→`{…}`도 상속해 loud. replication `{n{x}}`는 별도 `Replicate` node(never `Concat`)라 안 새어 loud 유지. **STRING 원소 배열**은 `'{…}` string 경로가 별도 buggy(§4.5.183 발굴)이라 `string s[] = {…}`도 handle-gate서 loud 유지(silent-empty 구조적 방지).

**적대 differential(vita vs iverilog)**: 지원 全 MATCH — queue literal(1 2 3 4)·dyn literal(5 6 7)·scalar vars(10 20 30)·single(42)·expr elements(`{1,2+3,4*2}`=1 5 8)·signed byte(-1 2 -3·iverilog는 elab assert-crash라 vita>iverilog)·`.size()`=4·init후 push_back/delete·module-scope·block-local. **LOUD 유지(silent-wrong 0)**: array-element concat(`{a,3}`=E3009)·replication(`{3{5}}`)·string concat. 전 suite green(gate가 Concat 없는 설계엔 byte-identical).

**결과**: `int q[$]={…}`·`int d[]={…}` 흔한 idiom 동작. `queue_dyn_brace_init.rs`×14. format 22 불변(elaborate 라우팅만·AST/IR 무변화). **3840 green**(+14). ⭐교훈: ① **fresh-area probe가 흔한 idiom의 loud gap 발굴**(메서드 차분→initializer 격리). ② **proven 경로(`'{…}` flush) 재사용이 correct-or-loud 상속**(라우팅=byte-identical→새 silent 표면 0). ③ **string 원소 제외가 핵심**(string `'{…}` 경로 자체가 buggy→인접 silent-wrong §4.5.183 발굴). ④ **replication은 별도 AST node라 자동 배제·`fold_init`/`const_eval`이 Concat에 None이라 collect 경로 무수정**. 상세=본 엔트리·ROADMAP.

#### 4.5.181 enum `.next(N)` / `.prev(N)` CONSTANT-step loud→supported (parser N-step ternary-chain desugar) (2026-07-21, branch feat-enum-step-n) ✅

**발굴 경위**: §4.5.180 fresh-area probe 스윕 중 enum method bisect서 발견한 oracle-backed loud gap. `.next()`/`.prev()`/`.first`/`.last`/`.num`/`.name()`는 전부 동작하나 **`.next(2)`처럼 STEP 인자**가 있으면 E3009("hierarchical function call (deferred)")—iverilog는 지원(`.next(2)`=2 스텝 전진·wrap).

**근본 원인**: 파서(`expr_primary`)의 §6.19.5 enum-method desugar(1652)가 **arg-less 전용**(`peek != LParen || empty_call` 게이트). `x.next(2)`는 peek==LParen & non-empty→desugar 건너뜀→generic `Call{x.next, [2]}`→elaborate `inline_function`이 2-segment name을 hierarchical call로 loud. 즉 `.next(N)`의 N 인자를 처리하는 경로가 없음.

**설계 (파서 constant-step 분기)**: arg-less enum block 뒤에 신규 분기 — peek==LParen & path=`var.{next|prev}` & `var_enum.contains_key`면 `call_args()` 파싱 후 single literal arg(`const_lit` fold)면 신규 **`enum_step_n_expr(path, is_next, n)`**가 N-step ternary chain 생성: 각 label i→`vals[(i+offset) mod len]`(`offset = next?N:len-N`, `rem_euclid`로 N≥len·N=0[identity]·부호 정규화). arg-less `enum_method_expr`(1-step)는 **불변**(byte-identical 유지)—별도 함수라 golden churn 0. **correct-or-loud**: non-literal step(`x.next(k)`)·wrong arity는 generic Call로 fall-through→elaborate loud. AST/IR 무변화(기존 Ternary/`==`/IntLit node만).

**적대 differential(vita vs iverilog)**: 全 MATCH — next(2)=C·next(3)=A(full-cycle)·next(4)=B(4 mod 3)·next(0)=identity·next(2) of B wraps=A·prev(2)=A·prev(3)=C·prev(2) of A=B·arg-less `.next()` 불변. **LOUD**: non-constant step(`.next(k)`)·iverilog는 지원하나 vita는 correct-or-loud. 전 suite green(파서 변경이 비-enum 경로 무영향·arg-less 불변).

**결과**: enum step-arg 흔한 형태 동작. `enum_methods.rs`+5. format 22 불변(파서 desugar만). **3826 green**(+5). ⭐교훈: ① **fresh-area probe가 loud gap도 발굴**(silent-wrong뿐 아니라 oracle-backed loud→supported 후보). ② **arg 유무로 갈라지는 desugar는 arg 형태를 놓치기 쉽다**(게이트가 arg-less 전용→arg-form이 generic call로 새어 loud). ③ **1-step 경로 불변 유지가 golden churn 0의 핵심**(신규 N-step은 별도 함수). ④ **constant-step subset이 common case**(testbench의 `.next(2)`)·runtime step은 correct-or-loud. 상세=본 엔트리·ROADMAP.

#### 4.5.180 SILENT-WRONG 수정: same-named STATIC block-locals in DISJOINT procedural blocks → loud (type-mismatch + definite-assignment guard) (2026-07-21, branch feat-block-local-collision-loud) ✅

**발굴 경위**: "fresh-area silent-wrong probe"(추천 방향)—iverilog 라이브 차분을 최근 세션이 안 건드린 영역(signed 산술·shift·select·concat·sys-func·width·cast·array·X/Z·power/mod)에 ~90 비교 스윕. 대부분 CLEAN이나, `signed [3:0] x=-3; y=x`를 두 sibling 블록에서 `y`를 다른 부호로 선언한 케이스서 **vita -3 vs iverilog 253** 발견. 축소하니 격리 시엔 정상(253)이라 컨텍스트 의존—두 블록이 같은 이름 `y`를 선언하는 게 트리거.

**근본 원인**: v1은 절차 블록-로컬을 module net으로 **flatten**(`hoist_block_local_nets`·per-block frame 없음). 같은 이름이 다른 블록에서 또 선언되면 `existing` net을 찾아 **coalesce**("scalar local safely coalesces—net just reused in time"). 이 가정은 **둘째 블록의 TYPE이 같고 read 전 assign될 때만** iverilog(distinct-per-scope 변수)와 byte-identical: (1) **타입 불일치**(sign/width)면 공유 net을 잘못된 부호/폭으로 read/write—`%0d` 부호 뒤집힘(signed net에 unsigned 값)·`y>>>1`이 산술 shift(-1 vs 127)·`%h` 폭(0c vs c); (2) **read-before-write**면 첫째 블록의 leftover value를 봄(vita 5 vs iverilog x=fresh var default). sibling `begin…end`·named block(`begin:ba`/`begin:bb`)·**cross-process**(두 initial) 全 발생(첫째의 flatten `top.y`에 둘째가 충돌). dyn/queue/string local과 automatic collision은 이미 loud(각각 heap-per-block·per-entry storage 불가)—**plain static scalar만 gap**.

**fix (correct-or-loud LOUD)**: `hoist_block_local_nets`의 `existing`(collision) 경로, automatic 분기 뒤 static `else`에 guard 추가: 각 name에 대해 (a) module-scope 선언(`local_decl_names.contains`)이면 skip(legitimate SHADOW—struct/enum shadow-scoping 또는 `check_block_local_scope_leaks`가 이미 처리), (b) dyn/string이면 skip(위서 이미 loud), (c) 새 decl의 `range_to_dims` type vs 기존 net width/signed 비교→**불일치면 E3009**, (d) 같은 타입이면 `automatic_local_definitely_assigned(stmts, nm)` 아니면 E3009(read-before-write). **SAFE same-type+assigned-first coalesce는 무영향**(흔한 두 `for` 블록의 `int i` 재사용). **★적대 검증서 회귀 발굴+수정**: 초판(module-shadow 미제외)이 `block_struct_var_shadow.rs` 3 test FAIL—struct/enum var가 module var를 SHADOW하는 SUPPORTED 케이스(distinct scoping)를 오판 loud. `local_decl_names.contains(nm)` 제외로 수정(충돌 이름이 module-scope 선언=shadow=skip·순수 block-local=sibling collision=fire). plain-scalar module-shadow는 `check_block_local_scope_leaks`가 이미 E3009라 gap 없음.

**적대 differential(vita vs iverilog)**: 修正前 MISMATCH 全 재현(diff-sign -3/253·diff-width 0c/c·`>>>` -1/127·named -3/253·cross-proc·stale 5/x)→修正後 全 loud(E3009). SAFE(same-type `for` 재사용=6·unique names=253·single local=-5) 정상. probe 다른 ~80 비교(shift·select·concat·sysfunc·width·cast·array·X/Z·power/mod) 全 MATCH. 전 suite green(guard가 module-shadow·same-type-coalesce 무영향).

**결과**: plain static 블록-로컬 이름 충돌의 silent-wrong 봉쇄. 신규 `block_local_name_collision.rs`×9. format 22 불변(elaborate guard만). **3821 green**(+9). ⭐교훈: ① **fresh-area iverilog 차분 probe가 pre-existing silent-wrong 발굴**(정본 correct-or-loud 활동·컨텍스트 의존 버그는 격리로 사라져 "격리해도 재현되는 최소형" 필요). ② **"safely coalesces" 류 최적화 가정은 조건부**(same-type+DA일 때만 byte-identical·타입/DA 어긋나면 silent-wrong)—가정의 전제를 명시적 guard로. ③ **적대 검증이 fix의 over-reject 회귀 잡음**(struct/enum shadow는 SUPPORTED·`local_decl_names` 제외가 sibling-collision과 module-shadow 분리). ④ **correct-or-loud=loud 먼저·SUPPORTED follow-up 기록**(`$blk$` scope를 static에 확장하면 iverilog-parity). 상세=본 엔트리·ROADMAP §2.

#### 4.5.179 FRAMED function dyn-formal call BURIED in an expression loud→supported (R5-B hoist reuse → temp = f(a) direct-rhs) (2026-07-21, branch feat-frame-func-dyn-formal-buried) ✅

**발굴 경위**: §4.5.177(direct-rhs `r=f(arr)`)의 명시된 follow-on(추천 항목). framed dyn-formal function을 큰 expression 안에서 부르는 흔한 형태 — `$display("%0d", fsum(a))`·`r=fsum(a)+100`·`if(fsum(a)>10)` — 가 §4.5.177의 marker가 direct-rhs blocking assign에만 붙어 E3009였음(iverilog ✓: 15/115/big).

**핵심 관찰**: hoist하면 §4.5.177을 그대로 재사용할 수 있다. buried call `f(a)`를 fresh temp에 `__t = f(a)`(=**direct-rhs blocking assign**)로 뽑아내면, `lower_stmt` 재진입 시 `emit_frame_dyn_formal_markers`의 direct-rhs 경로가 발화→snapshot marker+blessed bind. 이후 원 expression은 `__t` 읽기로 lower. R5-B inout-function hoist(`hoist_inout_calls`/`hoist_stmt_top`)와 **정확히 같은 골격**(그것도 buried call을 temp로 뽑음)이라 재사용.

**설계 (parallel to R5-B, but pure)**: (1) `dyn_formal_func_names` set을 `lower_frame_funcs`서 `inout_func_names`와 나란히 채움(frame_set 중 input dyn-array formal 보유·R2-inlinable straight-line 함수는 frame_set 밖이라 자동 제외). (2) detector `dyn_formal_call_target`/`expr_has_dyn_formal_call`/`has_unhoistable_dyn_formal_call`(inout판의 mirror). (3) hoister `hoist_dyn_formal_calls`: buried call을 `ast::Stmt::Blocking{__t = f(a)}`로 `lower_stmt` emit(→§4.5.177 marker) 후 `Ident(__t)`로 치환. (4) `hoist_stmt_top`에 3 arm 추가(If-cond·Blocking-rhs[**nested만**·`dyn_formal_call_target(rhs).is_none()`으로 direct-rhs 제외→§4.5.177이 처리]·SysTaskCall args). (5) gate=`!inout_func_names.is_empty() || !dyn_formal_func_names.is_empty()`. **eval-order guard 불요**: framed function은 pure(function은 output/inout formal reject)라 평가를 앞당겨도 다른 operand 값 불변→R5-B의 `hoist_is_safe`/`collect_inout_mutated`가 dyn-formal엔 불필요(mutated set 항상 empty). **무조건-평가 위치만 hoist**: short-circuit `&&`/`||` RHS·`?:` arm은 조건부 평가→hoist하면 unconditional화→`has_unhoistable`이 decline→loud(correct-or-loud). `while`/`case` scrutinee·Concat-buried·frame-body 내부 call(`in_frame_body`서 marker 발화 안 함)도 loud 유지. **각 hoist가 자기 snapshot marker emit**→같은 배열을 두 번 부르거나 사이에 mutate해도 pass-by-value freshness 유지.

**적대 differential(vita vs iverilog)**: 全 MATCH — `$display(fsum(a))`=15·`r=fsum(a)*2+1`=31·`if(fsum(a)>10)`=big·`fsum(a)+fmax(b)`=35·two-args `15 20`·`-(fsum(a))`=-15·**snapshot freshness** `fsum(a)` 후 `a[0]=99` 후 `fsum(a)`=15→110·for-loop body·signed byte=3. **LOUD(correct-or-loud)**: short-circuit `&&` RHS·`?:` arm·`while`/`case` scrutinee·frame-body 내부(`wrap`서 `$display(fsum(c))`). **경계**: R2-inlinable straight-line dyn-formal(`return c[0]+c[1]+c[2]`)은 inline alias 경로라 이미 buried 지원(15)→hoist가 안 건드림(frame_set 밖). gate 덕에 dyn-formal function 없는 설계는 byte-identical(golden 불변).

**결과**: framed function dyn-formal이 흔한 buried 위치서 동작. 신규 `frame_func_dyn_formal_nested.rs`×14, `frame_func_dyn_formal.rs`/`inline_foreach_dyn_formal.rs`의 구 "stays_loud" 2 test를 supported로 갱신. format 22 불변(elaborate hoist만·엔진 무변화). **3812 green**(+14). ⭐교훈: ① **proven 슬라이스(§4.5.177 direct-rhs marker)를 hoist로 재사용**(buried→`__t=f(a)` direct-rhs로 정규화→기존 경로 재발화)·신규 표면 최소. ② **인접 기존 골격(R5-B inout hoist) 재사용**(같은 "buried call→temp" 문제·detector/hoister mirror). ③ **pure 함수라 eval-order guard 생략 가능**(R5-B의 mutation-safety 검사가 dyn-formal엔 vacuous)·**무조건-평가 위치만**이 correct-or-loud 경계. ④ **direct-rhs 제외 guard가 핵심**(Blocking arm이 `dyn_formal_call_target(rhs).is_none()` 없으면 §4.5.177 direct-rhs를 재-hoist→무한 루프/회귀). 상세=본 엔트리·ROADMAP.

#### 4.5.178 SILENT-WRONG 수정: writing a dynamic-array `input` formal in a synchronous frame body → loud (runtime F4004 guard) (2026-07-21, branch feat-frame-func-dyn-formal-buried) ✅

**발굴 경위**: §4.5.179(buried-call hoist) 적대 검증 중, §4.5.177 test `r2b_write_element_is_loud`(formal write=framed→loud 가정)가 hoist로 supported화되며 **깨짐**. 조사하니 `function automatic int f(input int b[]); b[0]=9; return b[0];`가 vita `1` vs iverilog `9`(IEEE §13.5.1 pass-by-value: 로컬 copy write→9). clean HEAD(0efb78f·§4.5.177)의 **direct-rhs** `r=f(a)`도 `rvd=1`로 동일→**§4.5.177 pre-existing SILENT-WRONG**(hoist가 buried 위치로 확대할 뻔).

**근본 원인**: §4.5.177은 formal을 heap snapshot으로 READ 지원했고 소운드니스 논증이 "`&self` executor는 dyn-array 변형 불가라 snapshot=alias 안전"이라 했으나, body가 formal을 WRITE하면 그 write가 **조용히 유실**된다. `b[0]=9`는 `frame_or_class_write`→`frame_write_lvalue`(`&self`)로 가는데, 이 경로는 frame-local을 **scalar slot**으로 다룸(주석: "array-element write는 elaborate서 reject: frame locals are scalar"). dyn-array formal은 scalar가 아니라 heap-backed DynArray net→element write가 unused scalar window slot에 떨어지고 READ는 heap(snapshot)서 옴→write 증발. `write_lvalue`(`&mut`)의 heap store(`dyn_write`)는 `&self` frame executor(`run_frame_call`=함수·`run_task`=subset task)서 호출 불가.

**설계 (runtime guard·sound by construction)**: `frame_write_lvalue` 상단에 guard 추가 — target net이 `dyn_is_handle`(heap-backed dyn/queue/assoc) && `kind != String`이면 `fatal_frame_heap_write`(F4004) 후 return. **실제 write 시도 지점**서 포착하므로 element/whole/`new[]` write를 **균일하게** 커버(elaborate-time에 mutation 형태를 열거하다 하나 놓치는 unsound 위험 없음). string은 `dyn_is_handle`이나 slab-store(§4.5.167)라 여기서 올바로 write→제외. READ-ONLY body는 이 write 경로 자체를 안 타 무영향. §4.5.171 V5 frame-local dyn array(suspendable task)는 `run_process`(`&mut`)+`write_lvalue`가 dyn-handle을 `frame_write_lvalue`서 제외(heap 경로)→guard 미발화(회귀 0).

**적대 differential**: write 케이스 全 LOUD(exit≠0·F4004) — `b[0]=9;return b[0]`·`b[0]=9;return b.size()`·`foreach(b[i]) b[i]=b[i]+1`·whole-copy `b=c`(elaborate E3009). read-only 全 정상(foreach-sum=15). iverilog는 caller 배열을 by-ref write(`a0=9`)—IEEE §13.5.1 pass-by-value 위반이므로 vita는 wrong 대신 loud(no-oracle 아님·hand-IEEE).

**결과**: §4.5.177 pre-existing silent-wrong 봉쇄·§4.5.179가 buried로 확대하지 않도록 선행. 신규 `frame_func_dyn_formal_write_loud.rs`×4. format 22 불변(runtime guard만). **3798 green**(+4). ⭐교훈: ① **feature 확장(§4.5.179 hoist) 적대 검증이 인접 pre-existing silent-wrong 발굴**(loud-가정 test가 supported화되며 노출). ② **`&self` frame executor는 heap side-effect(dyn-array write) 불가**(§4.5.175 assoc-iter key-write 교훈의 재확인·read는 OK·write는 조용히 유실). ③ **runtime guard가 write 시도 지점서 포착→모든 mutation 형태 균일 커버**(elaborate 열거의 unsound miss 회피·§4.5.175와 동일 선택). ④ **soundness 논증의 암묵 가정(read-only body) 검증 필요**—"변형 불가라 안전"이 "변형을 조용히 버림"으로 새는지 write path까지 확인. 상세=본 엔트리·ROADMAP §2.

#### 4.5.177 FRAMED function `input` dynamic-array formal loud→supported (DynArray reserve + direct-rhs snapshot marker) (2026-07-21, branch feat-frame-func-dyn-formal) ✅

**발굴 경위**: "순차적으로 전부"의 원래 목표(§4.5.174 boundary·§4.5.175/176 prerequisite 해소 후 도달). `function automatic int fsum(input int c[]); foreach(c[i]) s+=c[i]; return s;`가 formal `c`를 `classify_unpacked_array`가 md-packed로 분류 시도→`Dim::Dyn` reject(E3009)였음. iverilog ✓(function dyn formal 지원→강한 oracle).

**핵심 소운드니스**: framed function은 `&self` `run_frame_call`로 실행→`new[]`/element-write(둘 다 `&mut` heap) 불가→**함수 body가 dyn-array를 변형할 수 없음**. 따라서 caller 배열을 formal에 snapshot(deep-copy)하든 alias하든 동일하게 안전(R2 inline path가 straight-line 함수에 alias 쓰는 것과 같은 논리·framed는 control-flow body). §4.5.176이 함수 内 `foreach`를 동작시켜 read path 준비 완료.

**설계 (DynArray reserve + snapshot marker + correct-or-loud by construction)**: (1) **`reserve_frame_func`**: `is_input_dyn_array_formal` formal을 md-packed 대신 `NetKind::DynArray` net으로 reserve(V2A-frame `reserve_frame_task`와 동일·2-state면 `two_state_heap_handles`)→`read_net` dyn 분기가 `c[i]`/`c.size()`/`foreach` 라우팅. (2) **`emit_frame_call`**: dyn formal을 **blanket-loud** — `dyn_formal_call_ok` flag 없으면 E3009. flag 있으면 placeholder arg bind(실 데이터는 marker가 heap에 채움). (3) **`lower_stmt`(`emit_frame_dyn_formal_markers`)**: direct-rhs `x=f(arr)` at module-process level(`!in_frame_body`)이면 각 bare dyn actual에 대해 `handle_copy` snapshot marker(no-op Display·`handle_copy_stmts[sid]=(formal_net, caller_net)`) emit 후 flag set→`lower_expr`(→`emit_frame_call`)이 bind→flag clear. **correct-or-loud BY CONSTRUCTION**: marker를 emit하는 blessed path만 flag set→**marker 없는 call은 반드시 `emit_frame_call`서 loud**(silent-wrong 구조적 불가). marker는 `&mut` process executor서만 실행되므로(Display) `!in_frame_body` gate가 `&self` subroutine body caller를 loud화.

**적대 differential(vita vs iverilog·iverilog가 function dyn formal 지원)**: 全 MATCH — direct-rhs int foreach(15)·signed byte(-5+10-2=3)·two-call 격리(3/60)·pass-by-value(r=7·later mutation 무관)·for-loop over size(15). **LOUD(correct-or-loud)**: non-direct-rhs `$display(f(a))`(marker 자리 없음)·nested caller `outer`서 `inner(c)` 호출(`in_frame_body`→`&self` executor·marker 실행 불가)·recursion(재귀 call이 body 内=`in_frame_body`)·sign-mismatch(`byte`←`byte unsigned`)·non-bare actual. 전 suite green(reserve/emit/lower_stmt 변경이 non-dyn-formal 설계엔 no-op→golden 불변).

**결과**: framed function의 `input` dyn-array formal이 direct-rhs module-level call서 동작. 신규 `frame_func_dyn_formal.rs`×9. format 22 불변(엔진 무변화·기존 handle_copy_stmts+§4.5.176 read path 재사용·reserve/lower elaborate만). **3794 green**(+9). ⭐교훈: ① **여러 슬라이스의 누적이 원래 목표를 unblock**(§4.5.171 V5 net-range·§4.5.176 frame-aware iteration·§4.5.170 dyn_array_actual_net·handle_copy_stmts—전부 재사용). ② **correct-or-loud BY CONSTRUCTION**(blanket-loud + bless-with-marker→marker 없으면 반드시 loud·silent-wrong 구조적 불가)가 복잡한 routing(어느 executor가 marker 실행 가능한가)의 안전한 처리법. ③ **소운드니스 근거가 mechanism을 단순화**(framed function이 dyn-array 변형 불가→snapshot=alias 등가→free/reentry-guard 불필요·non-recursive는 overwrite-safe). ④ **direct-rhs subset이 common case 커버**(testbench의 `r=f(arr)`)·나머지 correct-or-loud. **잔여 follow-on**: non-direct-rhs(pending-marker buffer 필요)·nested/recursion(per-activation + `&mut`-capable executor)·automatic task dyn-array LOCAL(V5 §4.5.171 done)·function dyn-array LOCAL(`new[]`=`&mut`·구조적). 상세=본 엔트리·ROADMAP.

#### 4.5.176 `foreach` over dynamic array/queue/assoc inside a FUNCTION / SUBSET-task body loud→supported (frame-aware iteration in the &self executors) (2026-07-21, branch feat-frame-foreach-supported) ✅

**발굴 경위**: §4.5.175가 loud화한 silent-wrong의 최종 해소. "function-frame dyn-formal"의 고레버리지 prerequisite—`&self` 동기 executor(`run_frame_call`=함수·`run_task`=subset task)가 dyn/queue/assoc `foreach`의 iteration key를 advance하게 만들면 §4.5.175 loud 케이스가 supported로 전환된다.

**핵심 관찰**: §4.5.175 분석서 "`&self` executor는 key-write(`&mut write_lvalue`) 불가"라 loud했으나, **`foreach` desugar의 iteration key(`__foreach_i`)는 항상 BODY-LOCAL**(frame-local net)이다. frame-local write는 `frame_write_lvalue`(interior-mutable frame window·RefCell·`&self`-OK)로 가능→`&self` executor도 key를 advance할 수 있다. 즉 loud는 필요 이상으로 보수적이었고, frame-aware write로 supported 가능.

**설계 (compute/write 분리·process path byte-identical)**: (1) `Scheduler::assoc_iter_step`(`&mut`)의 **read/compute half**(handle heap read·current key read·located key 계산·kval+fits+status)를 side-effect 없는 `SimState::assoc_iter_compute(&self) -> (Option<(knet, kval)>, status)`로 추출. 두 executor가 동일 compute로 key를 locate. (2) `Scheduler::assoc_iter_step`을 thin wrapper로 리팩터(`assoc_iter_compute` call 후 `write_lvalue`로 key write)→**process path byte-identical**(전 모듈-레벨 foreach/assoc/queue test + golden determinism 불변). (3) 신규 `SimState::frame_assoc_iter(&self)`: compute→key를 `frame_or_class_write`(→`frame_write_lvalue`, frame window)로·status를 `__st` lhs로 write. (4) `run_frame_call`/`run_task` BB 루프서 `rhs_is_assoc_iter`면 `frame_assoc_iter` 호출(§4.5.175 fatal 대체). **correct-or-loud 잔여**: key net이 frame-local 아니면(direct `st=aa.first(module_net)` in function·module-net write=`&mut` 필요) `fatal_frame_assoc_iter`로 fallback(guard: `!frame_local[knet]`).

**적대 differential(vita vs iverilog)**: iverilog-oracle 케이스 全 MATCH — function dyn foreach(15)·subset task dyn foreach(15)·queue foreach(7)·signed byte(-5+10-2=3)·nested function call per element(2*(1+2+3)=12)·suspendable task 회귀(15)·module process 회귀(60/assoc/queue byte-identical). **hand-IEEE(iverilog가 assoc-in-function compile 불가)**: assoc int foreach(2*10+5*20=120)·direct first/next while-loop(3+4+5=12)·reverse last/prev(3*1+2*10+1*100=123)—전부 손계산 검증. 전 suite green(process path 회귀 0·golden 불변).

**결과**: 동기 frame executor의 dyn/queue/assoc `foreach`(+direct first/next/last/prev iteration loop)가 correct. 신규 `frame_foreach_dynamic.rs`×10(구 `_loud.rs`×8 rename+supported화). format 22 불변(엔진 실행 경로만·IR/serialize 무변화). **3785 green**(+2). ⭐교훈: ① **loud가 필요 이상 보수적일 수 있다—핵심 제약(key-write=`&mut`)을 재검토하니 `foreach` key는 항상 frame-local이라 `&self` frame window write로 supported 가능**. correct-or-loud는 즉시 loud(§4.5.175)→여유 있을 때 supported(§4.5.176) 2단계가 이상적. ② **compute/write 분리로 process path byte-identical 유지하며 코드 공유**(read/compute를 `&self` helper로 추출→process는 `&mut write_lvalue`·frame은 `frame_write_lvalue`가 각자 write·drift 없음). ③ **§4.5.175 fatal 채널·`rhs_is_assoc_iter` guard 재사용**(loud→supported가 같은 detection 지점). ④ **function-frame dyn-formal의 남은 blocker 좁혀짐**—이제 함수 内 dyn `foreach`는 동작·dyn FORMAL만 classification서 rejected(formal을 DynArray net으로 reserve하는 별개 follow-on). 상세=본 엔트리·ROADMAP §2.

#### 4.5.175 SILENT-WRONG 수정 — `foreach` over dynamic array/queue/assoc inside a FUNCTION / SUBSET-task body → loud (runtime fatal) (2026-07-21, branch feat-frame-foreach-loud) ✅

**발굴 경위**: "function-frame dyn-formal"(control-flow body function의 dyn-array input formal) 조사 중, feasibility de-risk로 "framed function이 module dyn-array를 `foreach`로 읽는" probe를 돌렸더니 **vita `gsum=0` vs iverilog `gsum=15`**—clean HEAD의 **pre-existing SILENT-WRONG** 발굴. 적대 differential이 정확히 짚음.

**근본 원인**: 파서가 `foreach(a[i])`를 `__st = a.first/next(__foreach_i)`로 uniform desugar하고, elaborator가 array KIND로 재작성(fixed→plain index walk·dyn/queue/assoc→`lower_iter_special`의 dense/sparse walk via `AssocFirst/Next` SysFunc). 이 walk step은 **iteration KEY(`__foreach_i`)를 side-effect로 WRITE**한다. process executor는 `k_assoc_iter`(`Scheduler::assoc_iter_step`)서 `&mut write_lvalue`로 key를 쓰지만, **동기 frame executor**—`run_frame_call`(FUNCTION)·`run_task`(SUBSET task)—는 `&self`라 `write_lvalue`를 못 부른다. 이들의 BB 루프는 BlockingAssign을 plain `frame_rhs_value`(eval)로만 처리→`AssocFirst` eval은 status만 반환하고 key는 안 써짐→key가 stuck→walk 미진행→loop body 0회 실행→**조용히 0 반환**. **범위 매핑**: FUNCTION(`run_frame_call`)+SUBSET task(`run_task`)=silent-wrong; SUSPENDABLE task(`run_process`·`&mut`)+module process=정상; direct `marr[1]`/`marr.size()`(순수 `&self` read)=정상; fixed-array `foreach`(`lower_fixed_foreach_step`·compile-time 인덱스)=정상.

**fix (correct-or-loud LOUD·runtime)**: `run_frame_call`·`run_task` BB 루프서 각 BlockingAssign의 rhs가 assoc-iter SysFunc(`rhs_is_assoc_iter`: `AssocFirst/Next/Last/Prev`)면 `fatal_frame_assoc_iter`로 `call_fatal` 래치(depth-limit fatal과 동일 채널→scheduler가 `FinishReason::Error`로)+명확 메시지("use a `for (int i=0;i<a.size();i++)` loop, or iterate in a process/suspendable task"). runtime 방식 선택 이유: elaborate-time은 function/task/class-method 구분+suspendable 판정(post-pass)이 얽혀 over-reject 위험; runtime guard는 **정확히 broken executor가 dense-walk에 도달할 때만** fire→zero over-rejection. **왜 fix(frame-aware key-write)가 아니라 loud**: key-write를 `&self` executor서 하려면 `frame_slot_write`(RefCell·`&self`-OK) 경유 assoc/dyn/queue 인덱스-step 로직을 `Scheduler::assoc_iter_step`(`&mut`·`write_lvalue`)서 SimState `&self` 버전으로 복제해야 함—2 executor·assoc/dyn/queue·string-key·fits 로직 전부→substantial+risk. loud가 즉시 silent-wrong 제거(follow-on = frame-aware iteration).

**적대 differential 全 검증**: LOUD(정확) — function dyn foreach(F4004)·function queue foreach·function direct `m.first(k)`·subset task dyn foreach. WORKS(회귀 0) — suspendable task foreach(t=15)·function `for(i<size())`[workaround·g=15]·fixed-array foreach in function(15)·direct dyn index+size in function(g=8)·module process foreach. **other heap-mut in function 이미 loud 확인**(queue pop/push=E3009·silent 아님)→이 walk-step만이 subset-check를 통과해 silent-run하던 유일 gap. 전 suite green(기존 test가 old silent-0을 assert하지 않음 확인).

**결과**: 동기 frame executor의 dyn/queue/assoc `foreach`(+direct first/next)가 silent-0 대신 loud fatal. 신규 `frame_foreach_dynamic_loud.rs`×8. format 22 불변(엔진 실행 guard만·IR/serialize 무변화). **3783 green**(+8). ⭐교훈: ① **feasibility de-risk probe가 인접 silent-wrong을 발굴**—"framed function이 dyn array를 읽나?" 확인이 `foreach`→0 silent-wrong 노출. **가정 검증(does my approach's read path even work?)이 pre-existing 버그를 드러냄**. ② **`&self` 동기 executor는 side-effecting op(key-write) 불가**—read(`dyn_read`/`read_net`)는 OK지만 write side-effect(assoc-iter key)는 `&mut` 필요→동기 executor서 조용히 no-op. **side-effect를 품은 "expression"(SysFunc)이 subset-check를 통과하는 게 위험**. ③ **correct-or-loud=silent 즉시 loud화**(full fix=frame-aware iteration은 follow-on). runtime guard가 elaborate-time over-reject보다 정밀. ④ **원래 목표(function-frame dyn-formal)의 prerequisite 노출**—그 slice는 formal의 `foreach`가 동작해야 하는데, 이제 그건 loud(일관)·frame-aware iteration이 선행 필요. 상세=본 엔트리·ROADMAP §2(RESOLVED).

#### 4.5.174 inline static-task `foreach`-on-dyn-formal loud→supported (dyn_handle → dyn_handle_read at the foreach dispatch) (2026-07-20, branch feat-inline-foreach-dyn-formal) ✅

**발굴 경위**: §4.5.173 V2A-frame 적대 probe서 발굴해 ROADMAP §3에 기록해둔 §4.5.170 gap. static(non-`automatic`) task의 `input b[]` formal에 `foreach(b[i])`를 쓰면 E3009("first/next/last/prev are only supported as the DIRECT rhs of a blocking assignment"). `for(int i=0;i<b.size();i++)`는 동작·`automatic`(frame) 경로도 동작→`foreach`+inline-static 조합만 loud. iverilog ✓.

**근본 원인**: 파서가 `foreach(a[i])`를 `__st = a.first/next(__foreach_i)`로 **uniform desugar**(surface는 assoc-only 표기지만 내부는 통일). elaborator(`lower_*_call` foreach dispatch)가 array의 KIND로 재작성: fixed→plain index walk(`lower_fixed_foreach_step`)·dyn/queue→dense walk(`lower_iter_special`)·assoc→sparse first/next. dyn/queue dispatch가 array를 **`dyn_handle`**(=`lookup_net_scoped`만)로 resolve. **inline static-task의 dyn-array formal `b`는 `dyn_subst` alias**(§4.5.170·read-only input formal은 real net 아니라 caller net으로 별칭)→`dyn_handle("b")`=None→dispatch가 `return false`(특수 lowering 안 함)→desugar된 `b.first(__foreach_i)`가 generic method-call 경로로 falling through→`("first"|"next"|.., _)` arm이 expression-position first/next로 보고 E3009. AUTOMATIC 경로는 formal이 real `DynArray` net(§4.5.171 reserve)이라 `dyn_handle` 직접 hit→정상.

**fix (1-liner 본질·2 site)**: foreach는 배열 **READ**이므로 dispatch를 **`dyn_handle_read`**(§4.5.170 R2: `dyn_subst` alias 우선 consult 후 `dyn_handle` fallback)로 교체 — (a) fixed-array early gate(`...is_none()`·dyn formal을 non-dyn로 오판해 fixed 시도하던 것 차단·shadowing edge까지 방어) (b) dyn/queue resolution(`let Some((net,kind))=...`). formal이 caller의 real DynArray net으로 resolve→**module dyn-array와 동일한 dense walk**(`lower_iter_special`)로 라우팅. `dyn_subst`는 inline dyn-formal body 밖에서 empty라 `dyn_handle_read`≡`dyn_handle`→**fixed/module-dyn/queue/assoc 全 byte-identical**(회귀 0).

**적대 differential(vita vs iverilog)**: 全 MATCH — static task foreach sum(60)·index+element(`b[0]=5..`)·signed byte(-5+10-2=3)·two-foreach-same-formal(6/6). **regression 全 MATCH**: module dyn foreach(60)·fixed-array(12)·queue(7)·assoc(vita `s=3` correct·iverilog compile-fail=iverilog 한계). **boundary LOUD(별개 follow-on)**: FUNCTION+`foreach`(control flow→R2-straight-line 아님→framed→framed function dyn formal 미지원=function-frame dyn-formal 슬라이스 소관·`foreach` fix와 직교)·clean HEAD 동일 확인.

**결과**: static task의 `input` dyn-array formal `foreach` 동작(sum/index/element/signed/multi-loop). 신규 `inline_foreach_dyn_formal.rs`×8. format 22 불변(순수 lowering resolve 경로·IR/serialize 무변화). **3775 green**(+8). ⭐교훈: ① **read-position resolve는 일관되게 `dyn_handle_read`**(§4.5.170이 만든 alias-aware helper를 read 경로 전반에 적용해야 formal이 module array와 동일 동작)—dispatch 한 곳이 `dyn_handle`을 쓰면 formal이 조용히 다른 경로로 샘. ② **uniform desugar(`foreach`→`first/next`)의 KIND-dispatch가 alias-blind면 formal이 fall-through**—handle resolve를 alias-aware로 하면 기존 dense-walk 재사용. ③ **correct-or-loud 경계 명확화**(task=supported·function+foreach=framed-loud[별개]). 상세=본 엔트리·ROADMAP §3(RESOLVED).

#### 4.5.173 V2A-frame — AUTOMATIC task `input` dynamic-array formal loud→supported (per-activation heap slot + entry deep-copy) (2026-07-20, branch feat-v2a-frame-dyn-formal) ✅

**발굴 경위**: "순차적으로 전부" V5 follow-on. §4.5.170(V2A)은 STATIC task의 dyn-array input formal을 inline `dyn_subst` 별칭 + snapshot으로 지원했으나 AUTOMATIC(framed) task는 "formal이 fixed scalar slot이라 `b[i]`/`b.size()` mis-lower" 이유로 loud-defer("V5 handle-in-slot 필요"). §4.5.171(V5)이 frame-local `DynArray` net 머신러리(net-range lifecycle: `func_has_dyn_local`·reentry guard·free-at-exit)를 만들어 이제 formal에 재사용 가능. `task automatic consume(input byte b[]); b.size(); b[i];` = E3009였음. iverilog ✓(`sum=60 size=3`).

**설계 (per-activation heap slot + ENTRY deep-copy·wire 무변화)**: (1) **`reserve_frame_task`**: `is_input_dyn_array_formal` formal을 scalar 대신 `NetKind::DynArray` net으로 예약(2-state면 `two_state_heap_handles`)—V5 lifecycle이 `0..locals_len` 스캔이라 formal 슬롯 자동 포함(`func_has_dyn_local`=true→free/reentry 커버). (2) **`emit_frame_task_call`**: dyn input actual을 `dyn_array_actual_net`(element width+sign 매치·불일치 loud)으로 resolve해 caller dyn-array net을 읽는 **bare `Signal` in-bind**로 push. (3) **엔진 `split_frame_in_binds`**(sched.rs): in_binds를 scalar `(slot,Value)` copy-in과 dyn `(slot,src net)` snapshot으로 분리(formal net kind==DynArray면 in-bind Signal서 src net 회수). (4) **`frame_dyn_snapshot_formals`**(state.rs `&mut self`): `enter_task_frame` 直後 `dyn_heap[src].clone()`→`dyn_heap[base+slot]`(pass-by-VALUE deep-copy·IEEE §13.5.1). exec.rs 2 suspendable site(nested·process)만 수정. **wire 무변화**: `TaskCallInfo` 필드 추가(=`.velab` StagedExtraSidecars trailer 변경→format bump 필요)를 회피하고 순수 엔진-side(ir.exprs Signal 검사) 구동→**format 22 불변·기존 designs byte-identical**.

**correct-or-loud 경계**: 스냅샷은 **SUSPENDABLE 경로**(`enter_task_frame`)서만 일어남. **subset(non-suspendable) dyn-formal task**는 동기 `run_task_call`(`&self`, heap 못 채움)로 가므로 빈 배열 silent-read 위험→**post-pass loud**(`resolve_frame_task_rejects`서 `frame_task_has_dyn_formal` && !suspendable→E3009·"use a function or add $display/@/#/wait"). `frame_body_is_leaf_nonsuspending`이 `Fork`만 reject하고 `@`/`#`/`Call`은 lift라, `$display`/timing/nested-call 있는 dyn-formal task는 lift→suspendable→스냅샷 동작.

**적대 2-lens differential(vita vs iverilog)**: 全 MATCH — basic(60/3)·signed byte(-5+10-2=3)·4-state X 보존·twice-call 격리·**★snapshot-immune-across-suspend**(task `#10` suspend 중 caller가 `a=new[3];a[0]=777` resize+mutate→formal은 여전히 `b0=100 sz=2`·pass-by-value 누출 0)·re-forward to nested task(inner sum=7)·mixed scalar+dyn+output(106)·empty(0). **LOUD(safe)**: recursion(F4004·per-net heap 2-activation 불가)·concurrent fork(F4004)·non-suspendable subset(E3009)·sign-mismatch(E3009·iverilog compile-fail)·queue actual(E3009·iverilog 자체 assertion crash). **회귀 0**: static task inline 경로 무변화(가드 flip 1: `v2a_automatic_task_dyn_array_loud`→`_supported`).

**부수 발굴(pre-existing·§4.5.170 gap)**: static(non-`automatic`) task의 `foreach(b[i])`-on-dyn-formal이 E3009("first/next...")—clean HEAD 동일(내 변경 무관). frame 경로는 `foreach` 동작. ROADMAP §3 기록(별개 슬라이스: inline foreach→size-loop desugar).

**결과**: automatic task의 `input` dyn-array formal(size/element r/across-suspend/re-forward/mixed) 동작·pass-by-value 격리. 신규 `frame_dyn_formal.rs`×13. format 22 불변. **3767 green**(+13). ⭐교훈: ① **인접 완료 슬라이스(V5 net-range lifecycle)를 재사용해 신규 표면 최소**—formal을 frame DynArray net으로 예약만 하면 free/reentry/routing이 자동. ② **wire-format bump 회피**=snapshot src를 새 IR 필드 대신 기존 in-bind Signal서 엔진-side 회수→format 22 유지·golden churn 0. ③ **suspendable vs synchronous 라우팅이 correct-or-loud 경계**—subset 경로는 heap 못 채우므로 loud(적대 아닌 routing 분석이 silent-wrong 예방). ④ **snapshot-immune-across-suspend가 pass-by-value 핵심 실증**(§4.5.170 alias→snapshot 교훈의 frame 판). 상세=본 엔트리·spec §2.

#### 4.5.172 frame-body validator over-scan false-REJECT 수정 (linear scan → reachable-block CFG walk) (2026-07-20, branch feat-frame-overscan-fix) ✅

**발굴 경위**: §4.5.171 V5 적대 리뷰 agent가 발굴해 ROADMAP §3에 기록해둔 pre-existing false-REJECT. frame-TASK subset reject 결정은 **POST-PASS**(`resolve_frame_task_rejects`)로 미뤄지는데 이는 **모든** frame body가 lower된 뒤 실행된다. `classify_frame_body`가 `block_base..self.func_blocks.len()`를 linear 스캔 → post-pass서 `func_blocks.len()`가 **전체 블록의 끝**이라, **뒤에 정의된 task/func들의 블록까지 over-read**→그들의 (합법적으로 out-of-frame인) output-formal write를 검사 대상 task의 것으로 오판→E3009("assignment to a net outside the function")로 subset task를 false-reject. **repro**(clean HEAD): `task automatic p(output int r); int x; x=6; r=x;` 뒤에 `q`를 하나 더 정의하면 iverilog `p=6 q=7`인데 vita가 `p`를 reject. **task-only 버그**(함수는 자기 lower 직후 inline validate돼 뒤 func가 아직 없음→over-scan 없음).

**설계 (reachable-block CFG walk)**: linear range 스캔을 **entry(`self.funcs[fid].entry`)서 자기 CFG 엣지만 순회하는 워크리스트**로 교체(`frame_task_has_unsafe_construct`/`frame_body_is_leaf_nonsuspending`이 이미 쓰는 패턴). 엣지: `Goto`→target·`Branch`→then/else·`Call`→`ret_bb`(**callee entry 아님**→함수 밖으로 안 나감)·`Delay`/`Wait`→`resume`·`Fork`/`Return`→없음(`Fork`는 이미 `why=Some`). 3 call site(class-method·frame-func·post-pass task) 전부 `funcs[fid].entry` 전달로 통일·`frame_task_pending` 튜플서 dead `base` 필드 제거. **correct-or-loud**: reachable-only가 스킵하는 블록은 (a)다른 함수 것 [버그 원인·제거가 fix] 또는 (b)entry서 도달 불가능한 dead code [런타임 실행 안 됨]뿐이라, verdict는 false-reject만 DROP할 수 있고 Some→None(silent-wrong)은 불가. `Fork`가 항상 `why=Some`이라 fork children 미방문도 verdict 안 뒤집음.

**적대 differential(vita vs iverilog)**: 全 MATCH — nested subset task call(caller not-last·`42 9`)·control flow if/case/for(`23 7`)·suspendable `#delay` task mixed(`1 2 @5`)·5-task+function interleave(`1 2 3 4 5`)·2/3-task not-last repro(`p=6 q=7`/`1 2 3`). **still-loud 유지**: automatic task가 모듈 net에 write하면 여전히 E3009(walk가 in-func reachable write를 정확히 검출→over-relax 아님). 신규 `frame_subset_overscan.rs`×4(repro 2-task·middle-of-3·function not-last·still-loud).

**결과**: subset task/func를 정의 순서와 무관하게 정확히 분류. format 22 불변(elaborate-time 검증 로직만·IR/serialize 무변화)·golden churn 0. **3754 green**(+4). ⭐교훈: ① **deferred 검증(post-pass)은 linear range 가정이 깨진다**—lower 순서 의존 상한(`len()`)이 전역 끝이 됨→CFG reachability가 유일하게 견고한 경계. ② **correct-or-loud를 fix 방향으로 활용**: over-scan은 reject 사유만 ADD(§4.5.171 기록)했으므로 reachable-only 축소는 false-reject DROP만 가능·신규 silent-wrong 표면 0—안전하게 좁힐 수 있음이 사전 보장. ③ **기존 CFG-walk 패턴 재사용**(`funcs[fid].entry` 워크리스트)으로 신규 순회 버그 표면 최소. 상세=본 엔트리·ROADMAP §3(RESOLVED).

#### 4.5.171 V5 — frame-local (task-body) DYNAMIC array loud→supported (per-net heap + reentry guard) (2026-07-20, branch feat-v5-frame-dyn-array) ✅

**발굴 경위**: "순차적으로 전부" 3순위(V5 frame dyn-array·V2A-frame도 해소 목표). `task automatic mk(...); int loc[]; loc = new[n]; loc[i]; loc.size();`가 frame-local `loc[]`이 DynArray handle 아닌 1-elem net(`frame_array_local`)이라 `new[]`서 E3009. iverilog ✓. 사용자 결정=**"do the full per-activation heap now"**(AskUserQuestion).

**설계 (per-net heap + free-at-exit + reentry guard·엔진 30-site 재키잉 회피)**: 조사서 `dyn_heap`이 **net-id 키**임을 확인(per-activation 아님). 30-site 재키잉 대신 **`dyn_heap[net]`=현재 활성화의 배열**로 두고 lifecycle로 격리: (1) **elaborate** `reserve_frame_local_decl`서 `[Dim::Dyn]`+simple bit-vector element면 `NetKind::DynArray` net 예약(2-state면 `two_state_heap_handles`)→`new[]`/`loc[i]`/`.size()`가 dyn 머신러리로 라우팅(`is_dyn_handle_net`·엔진 `dyn_is_handle` auto). (2) **엔진 read_net**: `frame_local` 분기 안에서 dyn-handle(≠String)이면 `dyn_read`(heap)로(String은 §4.5.167 slab 유지). (3) **write_lvalue**: frame-local dyn-array element write를 frame-slot 경로서 제외→`write_chunk`→`dyn_write`(heap). (4) **classify_frame_body**: in-frame dyn-array element assign 허용. (5) **lifecycle**: `enter_task_frame` 前 **reentry guard**(`frame_dyn_reentry_ok`: frame dyn-array의 `dyn_heap[net]`이 이미 Some=부모/동시 활성화 live→**fatal F4004**), `exit_task_frame` 後 **free**(`frame_dyn_free`: `dyn_heap[net].take()`→다음 호출 fresh·guard가 None 보게). `func_has_dyn_local`로 scan 게이팅. **correct-or-loud**: sequential/single-activation 정확·recursion/concurrent-suspend=fatal loud(per-activation stash는 follow-on).

**적대 differential(~18 probe)**: **task 全 MATCH** — basic(311)·dynamic-index loop r/w(14)·signed byte(95)·resize `new` after `new`(49)·delete(0)·sequential fresh(330/220)·`.size()` before `new[]`=0·across-suspend `#`(88·single activation survives)·module dyn-array 공존 no-alias(115)·nested different-tasks(24). **LOUD(safe·silent 아님)**: recursion(F4004·이전 silent r=3→loud)·concurrent fork+suspend(F4004)·**function** dyn-array local(동기 `run_frame_call`/`run_task`가 `&self`라 `new[]`[=`&mut` heap] 실행 불가→loud·follow-on)·multi-dim/packed/non-bit-vector element. **회귀 발굴+수정**: read_net/write_lvalue 재정렬이 **frame-local STRING 파손**(String도 `dyn_is_handle`이나 §4.5.167 slab-store)→10 test fail→String 제외 가드(`kind != String`)로 수정·guard/free/`func_has_dyn_local`도 `kind==DynArray`로 정밀화. **적대 soundness 2-lens 백그라운드 agent 병행**.

**결과**: task-body single-dim simple-element dynamic array 동작(`new[]`/element r/w/resize/delete/size·across-suspend·sequential fresh)·recursion/concurrent/function=loud. 신규 `frame_dyn_array.rs`×13. format 22 불변(기존 DynArray/dyn_heap 머신러리 재사용·신규 IR 없음)·golden churn 0. **3750 green**(+13). ⭐교훈: ① **조사가 30-site 재키잉을 회피**—`dyn_heap` net-key를 그대로 두고 lifecycle(free-at-exit + reentry guard)로 per-activation 격리→기존 dyn 머신러리 최대 재사용. ② **read_net/write 재정렬은 sibling 도메인 파손 리스크**—String도 `dyn_is_handle` 공유라 frame-local string이 heap으로 오배선(적대 아닌 suite가 발굴·`kind` 정밀 가드로 수정). ③ **correct-or-loud by construction**(reentry guard=fatal·미지원 shape=E3009). ④ **&self 동기 executor 한계**로 function은 loud(구조적·follow-on). 상세=본 엔트리·spec §3.

#### 4.5.170 V2A — TASK `input` dynamic-array formal loud→supported (static task·R2 dyn_subst 재사용) + re-forward 공통 fix (2026-07-20, branch feat-v2a-task-dyn-array) ✅

**발굴 경위**: "순차적으로 전부" 2순위(V2A/V5 frame dyn-array). round-14 리포트 V2A. `task consume(input byte b[]); b.size(); b[i]; endtask`가 task formal 전체 blanket-reject(`task.ports.any(|p| !p.unpacked.is_empty())`)로 E3009. iverilog ✓.

**설계 (R11 function dyn-array 머신러리 재사용 + 태스크는 pass-by-value SNAPSHOT·inline path 한정)**: R11이 **함수** input dyn-array formal을 read-only 별칭(`dyn_subst`: formal name→caller `DynArray` NetId)으로 지원(`is_input_dyn_array_formal`·`dyn_array_actual_net`·body read는 `dyn_handle_read`가 `dyn_subst` 우선). 이걸 **static(inline) task** 경로에 확장: (1) blanket-reject를 `!p.unpacked.is_empty() && !is_input_dyn_array_formal(p)`로 완화(input dyn-array만 통과). (2) 프레임 dispatch 직전 input-dyn-array formal이면 **loud-defer**(automatic/recursive task=frame formal이 scalar slot·handle-in-slot 인프라=V5 필요). (3) inline task input 루프서 input-dyn-array면 **pass-by-value SNAPSHOT**(아래 적대 발굴로 확정)—`dyn_array_actual_net`로 caller net 확인→`alloc_dyn_snapshot`(caller의 element type[width/sign/2-state] 미러한 fresh `DynArray` temp)→entry서 `handle_copy_stmts`(whole-handle deep-copy)로 caller 배열을 snapshot에 복사→`dyn_binds`에 formal→**snapshot** 별칭. body lowering 전후 `dyn_subst` extend/truncate. **공통 개선(re-forward)**: `dyn_array_actual_net`이 `lookup_net_scoped` 前 **`dyn_subst_lookup` 우선 consult**(`dyn_handle_read`와 대칭)→`outer(input b[]){ inner(b); }` 전이 pass-by-value가 **함수·태스크 양쪽** 동작(기존 함수 경로의 pre-existing loud 한계도 해소). **format 22 불변**(기존 dyn_subst/DynArray/handle_copy 머신러리 재사용·신규 IR 없음).

**적대 2-lens = ★ 핵심 silent-wrong 발굴+수정(alias→snapshot)**: 초판은 함수처럼 caller handle을 **직접 별칭**(no copy). 적대 soundness agent(87 probe)가 **IEEE §13.5.1 pass-by-value 위반 silent-wrong 발굴**: 함수 R2 경로는 **statement body를 loud-reject**(expression-reducible만)라 mid-body mutation 불가라 별칭이 안전하나, **task는 full statement body**라 body(또는 callee)가 aliased 배열을 **읽는 중 변형**하면 별칭이 그 변형을 `b`로 누출(`consume(input b[]){ a[0]=999; ... b[0] }`서 vita b0=999 vs iverilog 10·resize·callee `poke()` indirect도 동일). **수정=pass-by-value snapshot**(entry deep-copy). 재검증 differential: pass-by-value 3형(직접 mutate=`b0=10 a0=999`·resize=`sz=2`·indirect poke=`b1=20`) 全 iverilog MATCH·4-state X/Z snapshot·signed byte 음수·twice-call 격리·forward-isolation(`x=10`) 全 MATCH. 그 외 static task 全 MATCH(`.size()`·element byte/int/logic·loop sum·signed·empty·mixed formals=1052·re-forward 1~2 level·함수 re-forward=11[기존 loud]). **LOUD(safe·silent 아님)**: automatic task dyn-array(V5 defer)·write-to-formal(`b[i]=x`→E3010·caller 조용한 오염 없음)·sign mismatch·non-bare·queue actual. **Finding 2(pre-existing·V2A 무관)**: 2-state dyn OOB read→X(모듈 레벨 `int a[99]`도 동일·W4020 warn·IEEE §7.4.6=0)·일반 dyn-array gap이라 snapshot이 그대로 상속·별도 추적.

**결과**: static task input dyn-array formal **pass-by-value** 동작(re-forward 포함)·automatic은 V5까지 loud-defer. `dyn_subst` extend/truncate 균형. 신규 test 10건(`round11_report_gaps.rs`: supported 6[pass-by-value 2·re-forward 2·mixed/signed]+loud 4). format 22 불변·golden churn 0. **37xx green**. ⭐교훈: ① **proven 경로(R11 함수 별칭)를 인접 도메인(task)에 재사용할 때 도메인 차이가 soundness를 깨뜨릴 수 있다**—함수=expression body(mutation 불가)로 별칭 안전, task=statement body라 **동일 별칭이 pass-by-value 위반 silent-wrong**. 적대 differential(특히 "body가 aliased 저장소를 변형" 시나리오)이 정확히 이걸 발굴. ② **fix가 correct-or-loud by construction**—snapshot=`handle_copy_stmts`(element type 불일치시 loud) + `dyn_handle`(미등록시 loud)이라 misconfiguration이 silent 아닌 loud로 귀결. ③ **한 경로 완화가 인접 경로 pre-existing 한계 노출**(re-forward 공통 fix). ④ **correct-or-loud 경계 분리**=inline 즉시·frame(automatic)은 V5 필요→절반 lift+명확 loud-defer. 상세=본 엔트리·spec §2.

#### 4.5.169 frame-local unpacked ARRAY loud→supported (md-packed frame slot, array-FORMAL 재사용) (2026-07-20, branch feat-frame-local-array) ✅

**발굴 경위**: §4.5.168 V3/V4의 **잔여 loud 첫 항목**(frame-local unpacked array·1-elem net collapse로 correct-or-loud화됨). 사용자 지시 **"순차적으로 전부"**(defer 슬라이스 순차 구현·frame-local array가 1순위)로 착수. 기존엔 함수/태스크 body-local unpacked array(`int arr[0:2]`)가 프레임 슬롯=스칼라(1 Value)라 element access가 1-bit로 collapse → `frame_array_local` 마킹으로 loud 유지.

**설계 (array-FORMAL md-packed 표현 재사용)**: 배열 FORMAL은 이미 single-dim zero-based unpacked array를 **flat packed vector**(width `count*elem_w`)로 저장하고 `arr[k]`를 packed part-select `[k*elem_w +: elem_w]`로 lowering(`lower_packed_read`·signed-element `$signed` 재스탬프·`frame_arr_formal_meta`). frame-LOCAL도 **동일 표현**으로 통일: (1) `classify_array_formal`을 field-based **`classify_unpacked_array(unpacked,range,packed,signed,kind)`**로 추출(FORMAL은 `packed=&[]`로 위임·동작 불변) — supported slice = single-dim·zero-based·simple bit-vector element. (2) 신규 **`reserve_frame_local_decl`**로 4개 프레임 예약 site(class-method·func body·task body·block-local) 통일 — `Ok(af)`면 md-packed net(`packed_dims=[(0,count),(0,elem_w)]`·`frame_arr_formal_meta`·2-state면 `intro_kind`) 예약·**`frame_array_local` 미마킹**, `Err`면 기존 loud 경로(1-elem net + `frame_array_local`). **핵심 관찰**: 엔진 `frame_local`은 **net-range 기반**(`[base_net, base_net+locals_len)`)이라 프레임 범위에 예약한 md-packed net이 **자동으로 frame_local** → `arr[k]=v`(frame_local net의 part-select write)가 `write_lvalue`의 frame-local 분기→`frame_write_lvalue`로 라우팅(§4.5.168 Phase-2 lane). **FORMAL 머신러리 그대로 재사용**(신규 실행경로 표면 최소). suspendable-task 가드(`frame_task_has_unsafe_construct`)는 `frame_array_local` 검사라 supported array는 미마킹→자동 lift.

**적대 differential 검증(~20 probe·iverilog 13.0)**: **common 전부 MATCH** — const-index(sum=18)·dynamic-index(`arr[i]` r/w)·signed 음수(`arr[0]=-5`→-11·$signed 재스탬프)·descending `[3:0]`·byte 음수 sign-extend·4-state logic X-init·sequential static/automatic(r1=21 r2=41). **across-suspend**: 1~3 concurrent activation 각 격리(fork 2개=r1=21/r2=41·3개=6/60/600) + **재귀**(depth 3=36) 전부 per-activation/level 격리 MATCH. **vita가 오라클 초과(hand-IEEE 정당)**: (a) **automatic array 동시활성화 격리** — iverilog는 automatic ARRAY를 격리 못 함(shared·known bug: base=20 활성화가 arr0=10 읽음)→vita의 격리 답이 IEEE §6.21 정답. (b) **배열을 서브루틴에 전달**(`sum3(a)`=15) — iverilog는 unpacked-array subroutine port 미지원("sorry: not yet supported")·vita 정답. **OOB**: 2-state→0·4-state→X(IEEE §7.4.6·모듈스코프 컨벤션과 동일·iverilog는 2-state도 x). **잔여 loud(safe gap·silent 아님)**: multi-dim(`arr[0:1][0:1]`)·non-zero-based(`arr[1:3]`)·string/real/handle/packed-struct element·whole-array copy(`b=a`)·`foreach`·NBA-to-element(`arr[k]<=v`)·`'{…}` initializer. **적대 soundness 2-lens(백그라운드 agent·87 probe)=silent-wrong 0**: 全 PASS(정확·iverilog MATCH) 또는 LOUD(정당 reject)·vita가 iverilog보다 **더 정확한** 케이스 다수 재확인(signed 서브-element select는 unsigned 유지[§11.5.1·`$signed` 재스탬프는 whole-element read `idxs.len()+1==dims.len()`에만]·2-state OOB→0·automatic-array 격리·재귀 fib10=55 vs iverilog 5). 유일 **benign cosmetic**(silent-wrong 아님)=2-state `int` array OOB read가 frame에 4-state array가 공존하면 0 대신 x(둘 다 uninitialized-default 계열·**실데이터 누출 없음**·x는 iverilog와 일치)—OOB는 프로그래밍 오류 영역이라 무해.

**결과**: frame-local single-dim zero-based simple-element unpacked array 동작(함수/태스크·automatic/static·suspend across·재귀·concurrent 격리). 가드 테스트 3건 flip(`frame_local_array_element_write_supported`·`frame_block_local_array_element_write_supported`·`frame_local_array_task_supported`)+narrowing 가드 1건 신설(`frame_local_array_multidim_task_loud`). elaborate value-only(예약/lowering)·엔진 프레임 머신러리 재사용·format 22 불변·golden churn 0. **3727 green**(+1). ⭐교훈: ① **verified 머신러리(array formal)를 인접 도메인(frame local)에 재사용 = 신규 silent 표면 최소** — 표현·read/write·signed·OOB 전부 검증된 formal 경로 상속. ② **엔진 frame_local이 net-range 기반**이라 프레임 범위 예약만으로 per-activation 격리 자동 획득(별도 배선 불필요). ③ **correct-or-loud narrowing** — zero-based single-dim simple-element만 lift, 나머지 loud(`classify` `Err`). ④ **vita가 오라클(iverilog) 초과 케이스**(automatic-array 격리·subroutine array port)는 hand-IEEE로 확정(no-oracle≠defer). 상세=본 엔트리.

#### 4.5.168 V3/V4 suspendable tasks — `@`/`#`/wait/NBA/$systask/재귀/nested in a task body run (2026-07-20, branch feat-suspendable-tasks-v3v4) ✅

**발굴 경위**: 외부 round-14 리포트의 **최우선·gating 항목 ①**(V3=task 内 `@`/`#`/wait·V4=NBA/$systask). vita는 automatic task를 **frame-call subset**(동기 `run_task`·blocking-assign만)으로 실행해 timing/systask를 **E3009 loud-reject** — 리포트의 4개 TB(KAT driver=`task drive(v); @(posedge clk); sig<=v;`)를 전부 막던 벽. 사용자 지시로 **brainstorming→spec(`docs/superpowers/specs/2026-07-20-round14-deferred-items-design.md`)→writing-plans(`docs/superpowers/plans/2026-07-20-suspendable-tasks-v3v4.md`)→executing-plans** 정식 흐름으로 진행.

**설계 (spec §1·3접근 中 채택)**: **suspendable frame call via scheduler activity call-stack**(splice/동기-executor-확장 기각). non-subset automatic task를 스케줄러(`run_process`)가 per-activity **call-stack**(`FrameRec{callee,bb,ret_bb,out_binds,window}`)으로 실행 → task body의 `Delay`/`Wait`가 **caller process를 suspend**. subset task는 기존 동기 `run_task_call` 유지(오버헤드 0). **함수 제외**(LRM상 timing 불가). **format_version 22 불변**: 사용자 결정대로 엔진이 startup시 SimIr func 아레나에서 suspendable set을 **recompute**(`sim_ir::compute_suspendable_tasks`·transitive fixpoint)—사이드카/트레일러 없음·SimIr golden 불변.

**단계별 구현(feat 7커밋)**: **1a** classifier 추출(`validate_frame_body`→`classify_frame_body`). **2a** `compute_suspendable_tasks`. **2**(Phase 2) leaf non-suspending task 실행(call-stack + `run_process` 재구성: block-fetch frame-aware·suspendable `Call`push/`Return`pop·`enter/exit_task_frame`). **3**(Phase 3) `@`/`#`/wait suspend — **핵심 통찰**: suspend시 frame window를 공유 `frame_stack`→`FrameRec`로 **stash**, resume시 **restore**(`stash_frame_windows`/`restore_frame_windows`) → `frame_slot_read/write` 불변으로 per-activity 격리. **4a** 재귀+nested suspendable call(call-stack depth>1·`MAX_CALL_DEPTH` 가드). **5a** 적대 리뷰 correct-or-loud 가드.

**적대 2-lens 검증(백그라운드 2 agent·~85 probe)**: **CORE SOUND** 실증 — window isolation(동시-2활성화 각 body-local across `@`: r1=11/r2=21)·재귀 50-level(1275)·nested A→B→C·output copy-out·depth 가드(8192 loud, no stack overflow)·`write_lvalue` frame-local lane 전부 iverilog MATCH. **적대가 5 over-lift 결함 발굴→전부 correct-or-loud화**: ①frame-local unpacked ARRAY(1-elem net→element 1-bit collapse silent-wrong·pre-existing를 lift가 노출) ②NBA-to-frame-local(illegal·panic) ③`wait(<frame-local>)`(level-wait re-eval서 window 미복원 panic) ④`repeat(<non-const>) @`(hidden counter가 SHARED net→동시활성화 오염·AST서 검출) ⑤`disable fork`(suspend-signal로 count돼 route되나 in-frame 가드 없어 silent). 신규 `frame_task_has_unsafe_construct`+`ast_has_repeat_with_timing`가 lift 술어를 좁혀 5건 전부 E3009 loud.

**적대 발굴 loud→silent twin(구현 中 자체 발굴·수정)**: `write_lvalue`에 frame-local 분기 없어 routed task의 output/body-local 쓰기가 조용히 flat-store로(o=0)→`read_net`과 대칭인 frame-local write lane 추가.

**결과**: `@`/`#`/wait/NBA/$systask/재귀/nested를 쓰는 leaf task 동작(iverilog MATCH: p_at=15·p_delay=10·p_seq[KAT driver]=22·p_recur=25·nested=33·output=18). **잔여 loud(correct-or-loud)**: frame-local array·wait-frame-local·NBA-frame-local·repeat-timing·disable-fork·`fork`/`wait fork`-in-task. format 22 불변·golden churn 0. **3726 green**(+누적). ⭐교훈: ① **suspend은 frame_slot 재배선 대신 window stash/restore로 저침습 격리**(공유 stack은 suspend 경계에서만 관리). ② **저장-클래스/실행경로 확장은 적대 differential 필수**—inline 검증(전 케이스 MATCH)+3721 green에도 적대가 5 over-lift silent/panic 발굴(특히 concurrent-activation window isolation·pre-existing 결함 노출). ③ **recompute(no sidecar)로 format bump 회피**·엔진/elaborate 동일 함수로 route/lift 집합 불일치 원천 차단. ④ **correct-or-loud narrowing이 정답**—대형 기능은 verified-safe subset만 lift하고 나머지는 loud(silent보다). 상세=spec/plan 문서.

#### 4.5.167 round-14 loud→supported: 64-bit MSB-set 리터럴 fold(V9) + frame-local string(V1) + 엔진 str_bytes frame-aware twin (2026-07-20, branch feat-round14-v9-v1) ✅

**발굴 경위**: 외부 Reviewer **round-14 리포트**(`1c46c3e` 기준)는 지난 §4.5.166(★V10)의 **confirmation 리포트** — V10 silent-wrong이 정확히 수정됨을 SHA3-256("abc") native KAT `Finish@215` PASS로 재확인, **DUT-side silent-wrong=0** 선언. 나머지 V-항목(V1~V9 minus V7/V10)은 전부 **loud** gap. **iverilog 오라클 재검증으로 스코프 재조정**: (a) **차분 오라클 있음**(iverilog PASS)=V1·V2A·V3·V4·V5·V9 — 실 gap. (b) **iverilog도 reject**=V2B(fn `output` formal="must be input ports")·V6(queue of unpacked struct="Unpacked structs not supported")·V8(task-local unpacked struct) — vita loud=correct-or-loud 정당(차분 오라클 없음→hand-IEEE 미착수). 이 슬라이스는 오라클-backed 中 **소형 2건(V9·V1)** 소진.

**V9 — 64-bit MSB-set scalar-localparam 리터럴 fold (loud→supported)**: `localparam logic[63:0] B=64'h8000000000000000`→E3009 "not foldable" vs iverilog 수용. 근원=`const_eval_i64_lit`이 값을 i64로 fold하는데 unsigned 64-bit MSB-set 값(=i64::MAX+1)은 `i64::try_from` 실패→None→loud. explicit-signed width-64 arm은 이미 `v as i64`(bit-reinterpret) 수용했으나 unsigned는 미수용(비대칭). fix=`if explicit_signed && cv.width==64`→`if cv.width==64`로 확장(폭-64 리터럴=64-bit bit container→bit-preserving reinterpret). **이것이 §4.5.151서 defer됐던 "bit63-set unsigned 64-bit param 리터럴"의 해소** — 당시 보류 사유는 "`v as i64`가 downstream 부호 비교/산술을 silent-wrong化할 위험"이었으나, **`cv.width==64` gate가 그 위험을 봉쇄**: 폭-64 리터럴만 hit(narrow는 bit63 불가), 폭-64를 magnitude(index/bound)로 쓰는 건 pathological이고 downstream서 negative→loud로 걸림. 적대 검증으로 全 magnitude sink 추적(bound=clamp→width 1[before/after 동일]·array index=E4002 loud·delay=별도 u64 경로·repeat/replication=0 iter IEEE-correct·i64::MIN=2^63 abs_diff이 全 cap 초과→loud) 확인.

**V1 — function/task/block/class-method-local `string` 변수 (loud→supported)**: `string s; s=$sformatf(...)`→E3018 "procedural assignment to net `t.$func$f.s`". 근원=`map_net_kind_or_wire(String)`가 `_=>Wire`로 fall-through(string INPUT formal용 Wire+str_params 경로엔 정답이나 frame-local DECL엔 오답). fix=신규 `frame_local_net_kind(k)`(String→NetKind::String·else 위임)를 frame-local **DECL 4 사이트**(class-method/func/task body_decls + block-local `reserve_frame_block_locals`)에 적용. input-formal `formal_net_kind`(Wire+str_params) 경로는 불변.

**V1-twin — 엔진 str_bytes frame-aware (심층검증서 발굴한 loud→silent 회귀)**: 저장(NetKind::String)만 고치면 value-use(return/concat/compare/copy-out)는 동작하나 **string method(`.len()`/`.substr()`/`.getc()`)가 frame-local slot을 못 읽어 조용히 0 반환** — 초판이 loud(E3018)→silent(0)로 회귀할 뻔. 근원=`str_bytes(net)`가 static `dyn_heap[net]`만 읽음(frame-local string 값은 frame slab의 `frame_slot_write`에 있음·`dyn_heap[net]` 공백). fix=`str_bytes`에 frame-local 분기 추가(`read_net(net,None).to_str_bytes()`·`read_net`의 frame-local 분기 미러). guard-first(`kind!=String→None`)라 class `this` slot(Integer) 등 non-string frame-local 제외. 이 twin이 mandate "검증 中 발생 silent issue도 all clear"에 정확히 해당.

**적대 2-lens 검증**: **differential**(iverilog 13.0 ~40 probe)=**CLEAN·zero silent-wrong** — rc=0-both 全 byte-identical, 유일 값-divergence는 `"-99".atoi()`(-99 vita=IEEE-correct vs 0 iverilog=documented bug·pre-existing·직교). **magnitude-misuse safety 실증**: `mem[64'h8000...]`서 오히려 **iverilog가 silent-wrong**(mem[5] 반환), vita는 loud E4002. **soundness**(코드-경로 완전성)=**4 change 全 SOUND·loud→silent 회귀 0** — V9 全 magnitude sink 추적, str_bytes 분기 guard-first·panic-free(method eval시 frame active)·borrow-safe.

**잔여(defer·전부 loud=안전)**: ① **static(non-`automatic`) task inline string local**(`hoist_inline_task_locals` lib.rs:18119)=여전히 Wire→E3018 loud — inline 경로는 frame-slot이 아니라(`$itask$g$L` net·frame_local 아님) 순진하게 String化하면 str_bytes twin fix 미적용→silent 0 회귀 위험 → **별개 inline-path string 저장 슬라이스 필요**(static func·automatic task는 지원). ② **V2A**(task input dyn-array formal)·**V5**(frame dyn-array local `new[]`)=frame heap dyn-array 인프라(§3). ③ **V3/V4**(task body 内 `@`/`#`/wait/NBA/$systask)=**구조적 DEEP** — `run_frame_call`/`run_task`가 한 time-step 内 동기 평가라 시간축 suspend 불가, task-into-process splicing/coroutine 필요(다일 슬라이스)·**리포트 gating chain상 V3/V4가 TB를 gate**(V2A/V5는 그 다음)라 이것 없이 TB 미구동. ④ **V2B/V6/V8**=iverilog도 reject(차분 오라클 없음).

**결과**: format_version **22 불변**(elaborate value-only + engine eval-only·SimIr shape 불변·golden churn 0), 신규 `round14_report_gaps.rs`×9(V9 fold+magnitude-loud·V1 return/method/task-local/block-local/input-formal guard/module guard), 기존 guard 2건 flip(`frame_string_local_now_supported`·`g4_string_return_now_supported`). **3710 green**(+9). ⭐교훈: ① **§3 defer 사유가 재검토서 무효화될 수 있다** — §4.5.151 "부호 메타 배선 필요"는 실은 `width==64` gate로 우회 가능(defer 근거=위험이 실재하나 좁은 gate로 봉쇄 가능한 경우 재도전 가치). ② **저장 클래스 fix는 read 경로까지 확인해야** — NetKind만 바꾸면 value-use는 되나 method/특수-read 경로가 별도 저장을 읽어 loud→silent 회귀(str_bytes=static heap vs frame slot). ③ **오라클 재검증이 스코프를 절반으로** — "리포트가 원한다"≠"차분 가능", iverilog도 reject하면 hand-IEEE 없인 silent-wrong 위험만 짊 → 정직하게 loud 유지+기록. ④ **적대 differential이 오라클 자체 결함 노출** — magnitude-misuse서 iverilog가 silent-wrong(mem 인덱스 truncate)·vita loud가 우월.

#### 4.5.166 comb sensitivity 읽기집합 완전성 — LHS-index + 계층 ref silent→correct (SV §9.2.2.2.1) (2026-07-20, branch feat-comb-index-sensitivity) ✅

**발굴 경위**: 외부 Reviewer round-13 리포트(`5158f41` 기준)의 **★V10 silent-wrong**(최우선 ask). `always_comb`/`@(*)`/`always_latch`에서 변수가 **LHS bit/part/word-select 인덱스로만** 등장(`always_comb mask[idx*8 +: 8]=v`)하면 추론 감도목록에서 누락→인덱스 변경에 무감→조용히 stale. 리포트 실피해=`sha3_core.sv`의 `pad_mask[pad_byte_pos_q*8 +: 8]`가 pad_pos 변경에 무감→SHA3-256("abc") 오답(0x06이 byte0 고착). 재현: `mask[idx*8 +: 8]=8'hAB`→vita 全 idx `mask=0` vs iverilog `000000ab`/`0000ab00`/`00ab0000`. **적대 2-패스 soundness 렌즈가 같은 클래스의 2 잔여 twin을 추가 발굴**(Part B·C).

**Part A — 로컬 인덱스(리포트 V10)**: `comb_read_set`(elaborate)이 RHS + branch 조건만 수집하고 **LHS 인덱스/offset/word를 누락**. fix=신규 `collect_lval_reads`가 각 `LvalChunk`의 `word`(배열 워드)·`offset`(비트/part offset)·`width`를 `collect_expr_reads`로 수집(쓰기 base net 제외·RHS `Signal{word}`/`Select`와 대칭). `BlockingAssign`/`NonblockingAssign`/`Force`/`Release` 全 lvalue-arm 적용(Release=latent 대칭·force/release는 loud-reject).

**Part B — 계층 INDEXED twin(soundness 렌즈 1)**: `y=dut.mem[idx]` / `dut.mem[idx]=v`(계층 인덱스 read/write)는 대상 net이 미-elaborate child라 **deferred sentinel chunk(write)/placeholder expr(read)** 뒤로 lowering→lowering시 `comb_read_set`에 인덱스 불가시→감도 누락(read·write **양 twin**). **Part C — 계층 WHOLE-NET read(soundness 렌즈 2)**: `y=dut.q`(인덱스 없는 계층 whole-net read)도 `Signal{POISON_NET}` placeholder라 동일 누락. fix(B+C)=① comb-추론 ProcId를 `comb_inferred_procs`에 기록(**bare self-timed `always #5 clk=~clk`는 is_comb_inferred 아님→미기록→clock 감도 불변**·landmine 회피). ② deferred-hier resolver들이 real net+idx eids를 stmt/expr 아레나에 in-place 패치한 **후**(`fn run`) `recompute_comb_sensitivity_after_hier`가 기록 proc들의 read-set 재계산(superset-only·POISON 제거+실 net 추가·절대 narrow 안 함). **가드=4 deferral 레인 전부**(`deferred_hier`/`_sel`/`_write`/`_sel_write`)—whole-net 레인 누락 시 `y=dut.q`가 "무관한 indexed ref 존재 여부"에 정확성이 의존하는 비국소 버그(렌즈 2 발굴). non-hier 설계는 가드로 재계산 skip→golden churn 0.

**형식/결정성**: elaborate value-only(감도 edges 확장)·**IR shape 불변→format_version 22 유지**(SchemaHash 구조적). `comb_inferred_procs`=elaborator 임시 필드(비직렬화). `comb_read_set`=BTreeSet 결정적·3-OS 불변.

**적대 2렌즈**: differential=**CLEAN**(에이전트 39-probe: 유일 2 divergence[p36/p37]=pre-existing comb-loop settling·인덱스 0 scalar `y=y+1`서도 재현→직교; 로컬 10 + 계층 5[multi-instance·2D·part-select·branch-cond·mixed] + whole-net probe 全 MATCH). soundness=**2-패스**: 패스1=Part A SOUND(FINDING=Release 비대칭[대칭화]+Part B[계층 indexed twin] 발굴); 패스2(recompute 기전)=Q1~Q5 SOUND(multi-instance/idempotence/ordering/**clock 안전**/borrow)·**Q6 발굴=whole-net 레인 가드 누락**(Part C·즉시 broaden).

**§2 deep-defer 발굴(별개 클래스·pre-existing)**: **함수/태스크 body-내부 net read가 caller comb 감도 미기여**(`always_comb y=f(); function f; f=a^..`=iverilog `a` 추적[ef/df] vs vita xx). `collect_expr_reads`의 `Call` arm이 **인자 read만** 수집·callee body의 transitive net read(재귀/중첩/callee-내 hier·index) 미분석. 리포트 V10(comb 자기 body)과 다른 클래스·전용 슬라이스(→ROADMAP §2). task-call은 iverilog도 미재발화(murky·no-clean-oracle).

**검증**: 신규 `comb_lhs_index_sens.rs`×9(로컬 5: indexed-partsel·bitsel·array-word·explicit-`@(*)`·latch + 계층 4: hier-write·hier-read·**clock-survives-recompute**[landmine 회귀]·whole-net-read). **3701 green**(+9). clippy/fmt clean.

#### 4.5.165 enum label 범위검증 — out-of-range label을 silent-truncate→loud (SV §6.19) (2026-07-20, branch feat-enum-label-range) ✅

**발굴 경위**: fresh-area sweep 6 probe(산술 div/mod/pow/shift·format 지정자·bit-vector sysfunc·부호/폭·real 포맷·제어흐름)이 全 MATCH→코어 견고 확인→NEXT item 2(loud→supported)로 전환. 후보 3 그라운딩 中 **enum label 범위검증 부재**(§4.5.153 기록·§3)가 실은 **silent-wrong**: `enum logic [3:0] {X=16}`을 vita가 조용히 truncate(`e=0`)·iverilog는 compile-reject("value too large"). `{X=-1}`(unsigned base 음수)=vita 15·iverilog "negative value"·auto-inc overflow(`[1:0] {A..E}` E=4)=vita 0·iverilog "inferred value overflowed".

**silent→loud 구현**(parser-only·IR-0·format 22 불변): `parse_enum`의 label-folding 루프에 range-check 추가. `base_w: Option<u32>`=enum base 폭(base-less `enum{}`=int 32·explicit vector=fold·atom=synth `dec_range`·`[N-1:0]` param/≥65-bit=None→**skip=fail-open**). 각 label의 const-foldable 값 `v`를 **i128 경계**로 검사: signed=`[-2^(w-1),2^(w-1)-1]`·unsigned=`[0,2^w-1]`. 폭 1..=64 검사(64-bit도: signed longint=any i64 OK·**unsigned time/`bit[63:0]`=음수 reject**). foldable 값·foldable 폭에만 발화→legit label은 무영향(correct-or-loud·never over-reject). `enum_signed`(§4.5.153/154 resolved sign)이 storage sign과 일치.

**적대 2렌즈**: differential(~60 probe)=**CLEAN**(over-rejection 0·value-divergence 0·全 accept/reject 판정이 iverilog와 byte-identical: unsigned/signed vector·byte/shortint/int/longint atoms·ascending `[0:3]`/non-zero-lsb `[7:4]` 폭 정확·auto-inc·expr label·typedef/package base·boundary ±1). soundness=**SOUND** + **FINDING 1**(narrow silent-wrong 발굴·즉시 수정): 초판 cutoff `<63`이 width-64를 전부 skip→**unsigned `time`/`bit[63:0]` 음수 label을 silent-accept**(iverilog reject)→cutoff `<64`로 확장(signed longint=any i64 통과 유지·unsigned-64=음수만 reject)·재현 CLEAN. FINDING 2(진단 `found` 토큰이 post-`;`=cosmetic·span은 정확·기존 semantic-reject 선례 동일)=비수정.

**잔여(§2·pre-existing·fail-open·invalid-program 한정)**: sized/based-literal label(`{A=8'hFF}`·`{A='d20}`)·param-width base(`[N-1:0]`)의 out-of-range=`const_lit` 미fold(decimal-only)→skip=silent-truncate 잔존. iverilog는 reject하나 **유효 프로그램 무영향**·fail-open이 over-reject보다 안전(§4.5.164 sized-literal 잔여와 동일 클래스·const_lit 확장 or elaborate-time 검사=별개 슬라이스). i64::MIN label(`-(2^63)`)=별개 pre-existing E3009(non-foldable).

**검증**: 신규 `enum_label_range.rs`×10(signed-overflow·unsigned-negative·too-large·auto-inc-overflow·in-range-pass·atom/default-base·param-width-failopen·sized-literal-failopen·**unsigned-64-negative-loud**[FINDING 1 회귀]·**signed-longint-±i64::MAX-pass**). **3692 green**(+10). clippy/fmt clean.

#### 4.5.164 enum `.name()`/`.name` — SV §6.19.5 label-string method (loud→supported) (2026-07-20, branch feat-enum-name-method) ✅

**발굴 경위**: 6연속 param 슬라이스 후 **도메인 전환** fresh-sweep으로 §3에 그라운딩·기록한 enum `.name()`(vita E3009 vs iverilog label string) 착수(전용 슬라이스). `first/last/num/next/prev`는 i64 fold로 지원되나 name()은 dynamic string이라 `enum_method_expr`이 의도적 None이었음.

**loud→supported 구현**(parser-only·IR-0·format 22 불변): `enum_method_expr`(→`&mut self`)이 `x.name`/`x.name()`를 **synthetic string-return function 호출**로 desugar — `function string $enum_name$<T>(input signed?[63:0] x); case(x) <val>: return "<label>"; … default: return ""; endcase`. enum type별 1회 생성(`pending_enum_name_fns` **BTreeMap**=결정적 주입 순서)·container-end(`parse_module_like`)서 body에 주입(forward-ref OK·module-scoped·container별 자체 copy). port sign=`any label<0`·width 64(large enum 정확). **string-return function이 assign AND `$display("%s",…)` 양 context서 EXACT-length**(§4.5.163 그라운딩 확정: 핵심 통찰=packed string-literal **ternary는 result-width=max branch로 짧은 label PAD=silent-wrong**→ternary 금지·string function만 정답).

**적대 2렌즈 CLEAN**: differential(48 probe)=全 exact-length·no-pad(signed/negative·sparse·base-less·single-label·wide·property-form·concat·compare·sformatf·for-loop·two-enum-types·**same-enum-two-modules**·first/next regression·next+name compose·package·uninit→""·determinism byte-identical)·DIFF 전부 non-finding(`LABEL.name()`=iverilog assertion crash·**longint≥2^32=iverilog 버그[32-bit truncate]·vita CORRECT**[64-bit port 정당]·sized-literal-label=pre-existing). soundness(7 concern SOUND)=determinism(BTreeMap sorted·HashMap-into-AST 없음)·injection 완전(drain unconditional·stranding 없음)·collision(`$` prefix=simple-ident 불가·escaped `\$`는 backslash 포함 별개)·`&mut` refactor(first/next 등 불변·labels/ename clone)·schema/golden 불변(Func instance는 SchemaHash 무변).

**잔여(§3·pre-existing·NOT regression)**: sized-literal enum label(`{A=8'hFF}`)=non-foldable→`enum_defs` 미등록→`.first`/`.name` 全 enum-method loud(diagnostic 품질 minor)·function-port receiver `.name`=E3010(`var_enum`이 tf-port 미bind·enum-method-family 공통).

**검증**: `enum_methods.rs` `name_method_is_loud`→`name_method_returns_label_string`(옛 loud 인코딩→새 정답·§5) + 신규 `enum_name_method.rs`×7(exact-both-context·signed/sparse·property-form·next+name·oob→""·two-types·**staged `vcmp→velab→vrun`**[.vu round-trip=synthetic fn 직렬화 검증·soundness 리뷰어 coverage-gap 지적 반영]). **3682 green**(+7). clippy/fmt clean.

#### 4.5.163 untyped param — PACKAGE-scoped alias 값-결정 타입 (§4.5.162 후속) (2026-07-20, branch feat-pkg-scoped-param-signedness) ✅

**발굴 경위**: §4.5.162가 §2에 남긴 "pkg-scoped ident" residual(`localparam C = p::X`·X signed → C unsigned) 착수. `package p; localparam X=7; module t; localparam C=p::X; s<p::X`=1 vs `s<C`=0(vita) — const_expr_signed이 single-seg ident만 sign 상속·pkg-scoped(`p::X`) 미처리.

**silent-wrong fix (2 파트·elaborate-local·IR-0)**: ① **consumption**: `const_expr_signed`에 `PkgScoped{pkg,name}` arm(`pkg_const_meta[pkg][name].signed`) + `param_decl_width`에 bare-`pkg::X` inherit(`pkg_const_meta.get().and_then().copied()`·hit=full meta 상속·miss=value-inferred·single-seg ident 미러). ② **population**(2nd 리뷰어 발굴 pre-existing): `param_meta`가 `elaborate_package` fold 中 미population → **intra-package** alias/expr(`localparam B=A`·`E=A+1` in-pkg)이 sibling meta miss → C unsigned. fix=fold loop서 `param_meta` set-or-CLEAR+save/restore(`saved_meta`·self.params 미러·restore로 pollution 방지)→ident/const_expr_signed arm이 sibling 해소.

**적대 2렌즈**: 1st(differential+soundness·60 probe)=consumption CLEAN(scope-gate 정상·cross-pkg collision·import 일관·全 DIFF pre-existing). 2nd(soundness 8-concern)=consumption CLEAN(ordering=miss시 E3009 loud·key no-collision·miss→value-inferred 안전)+**intra-package population 갭 발굴**→§4b 같은 반복 즉수정·재검증(pollution: same-name module param 미오염·multi-pkg[pa::V/pb::V] no-collision·narrow/signed intra-pkg alias width 상속·cross-pkg scoped chain 全 MATCH).

**잔여(§2·pre-existing)**: time param value-inferred sign·expr self-determined width(§2 size-cast DEEP 동근·§4.5.162 residual)·pkg-internal `import`(E3009 honest-loud·별개).

**검증**: `untyped_param_value_signedness.rs` +5(pkg-scoped alias/expr·narrow-alias width·**intra-pkg alias/expr·narrow·pollution**). **3675 green**(+5). clippy/fmt clean. format_version 22 불변.

#### 4.5.162 untyped param value-determined type — IDENT/EXPRESSION initializer (§4.5.161 완성) (2026-07-19, branch feat-untyped-param-expr-signedness) ✅

**발굴 경위**: §4.5.161이 LITERAL initializer만 §6.20.2 적용 → §2에 남긴 residual(ident/expr-valued positive→unsigned) 착수(NEXT item ①·soundness 리뷰어가 "내부 inconsistent" 지목). `localparam D=7; localparam C=D; localparam E=3+4; s<D`=1(signed·§4.5.161) vs `s<C`=`s<E`=0(vita·C/E value-inferred unsigned) vs iverilog 1 — 같은 값이 bare literal이면 signed·ident/expr면 unsigned.

**silent-wrong fix**(elaborate-local·IR-0): `param_decl_width` untyped 분기에 **`const_expr_signed`**(AST-level §11.8.1 sign·IR-level `expr_self_signed` 미러: arith/bitwise=both-signed·shift/pow=left·compare/logical=unsigned·unary +/-/~=operand·single-seg ident=`param_meta` sign) 추가 → (a) **bare ident**(`C=D`·peeled `-D`/`~D`)=`param_meta.get(fq).copied()`(hit=source의 full `(width,signed)` 상속[narrow/typed alias width 보존]·miss=None→value-inferred) (b) **일반 expr**(`E=3+4`)=`(min_signed_bits(folded).max(32), const_expr_signed)`. 全 `Implicit`-gate(time 제외).

**적대 2렌즈 — 발굴 2 regression 즉수정**: soundness=`time C=D`(D signed)가 D sign 상속(ident-inherit이 Implicit-gate 누락)→**블록 전체 Implicit-gate**로 수정. differential=`C=N`(N time·large)가 expression path서 34-bit로 축소(정답 64·time 폭)→**bare-ident MISS=`.copied()`로 None 반환**(value-sizing 안 함→value-inferred가 wide source 폭 유지). 재검증: `time C=D`(unsigned 유지)·`C=N`(%b/concat 64 복원·`$bits({C,C})=128`)·longint/bit64 alias(64 개선)·int-signed/unsigned/mixed §11.8.1·narrow alias(width 상속)·nested chain 全 MATCH·회귀 0.

**잔여(§2·전부 pre-existing·stash-rebuild 확인)**: (a) `time` param value-inferred sign(`time C=SD` negative=32-bit signed vs 64-unsigned·main 동일) (b) `$bits(time-alias)`=32 vs 64(net은 64·`$bits`-path만·main 동일) (c) pkg-scoped ident(`C=p::X`)=unsigned(single-seg만 sign 상속) (d) expr self-determined width(sub-expr truncation·`8'hFF+8'hFF`·§2 size-cast DEEP 동근). const_expr_signed의 `_=>false`는 concat/select/`$signed`가 const서 E3009 loud라 moot(too-conservative 잔여만).

**검증**: `untyped_param_value_signedness.rs` +6(ident-alias·expression·unsigned-expr 불변·narrow-alias width·nested chain·**time-alias sign-guard**). **3670 green**(+6 이번·§4.5.161 11 포함 총 17). clippy/fmt clean. format_version 22 불변.

#### 4.5.161 untyped param value-determined signedness+width (IEEE §6.20.2) (2026-07-19, branch feat-untyped-param-value-signedness) ✅

**발굴 경위**: §4.5.160 적대 발굴로 §2 기록한 "untyped param 값-결정 타입" 재그라운딩(NEXT item ① oracle-backed). `localparam A=-1, B=2; A < B`=vita `ge`/0 vs iverilog `less`/1 — 무장식 `localparam`이 IEEE §6.20.2("타입은 값이 결정") 미적용: positive decimal `B=2`가 value-inferred fallback(`const_u32_expr`=v≤u32면 32-bit UNSIGNED)로 unsigned화 → §11.8.1 collective(둘 다 unsigned) 오답. param↔literal은 정상(literal이 signed)이라 param↔param 전용.

**silent-wrong fix**(elaborate-local·IR-0): `param_decl_width` untyped 분기가 값 LITERAL을 `parse_int_literal`로 타입 결정 — plain DECIMAL=signed·width=`min_signed_bits(folded).max(32)`(§3.5.1·값 자체를 담는 최소 signed 폭)·SIZED/UNSIZED-BASED=literal의 명시 width·sign·leading unary `-`/`+`+paren peel(inner sign 상속). `value_determined` 가드=SIZED는 현행(全 param-type)·DECIMAL/BASED는 `Implicit` 한정(`time`=64-bit 유지). 값-inferred fallback(비-리터럴 expr)·enum-label(`v<0` 별경로)·ranged/int/real 全 불변.

**적대 2렌즈**: soundness=CLEAN(8-concern: width/value 일관·coerce no-op·param_range 불변·`value_determined`가 time 제외·unary-peel IEEE-correct·enum-label 미접촉·hier/pkg 일관·residual 미악화). differential이 **-2^k regression 발굴**(내 초기 minus-peel이 magnitude 리터럴 width 사용→`-2^31`=33 vs 32·`-2^k` mag≥2^31 off-by-one·width-only): `-2^k`는 `+2^k`보다 1비트 적게 필요 → **`min_signed_bits(folded 값)`로 즉수정**(§S "differential이 이긴다")·全 boundary(±2^31/2^32/2^33·i64 extremes·-(2^31±1)) 재검증 MATCH. `$unsigned(param)` DIFF=iverilog const-fold quirk(plain var는 MATCH·vita spec-correct).

**잔여(§2·narrower residual)**: expression/ident-valued untyped param(`C=A+B`·`C=D`)은 value-inferred 유지→positive면 unsigned(내부 inconsistent: `D=7` signed vs `C=D` unsigned) · interface-member positive param(`i.A<i.B`) unsigned(interface-specific) · sized-literal `time` param `$bits`(pre-existing·byte-identical). 전부 pre-existing·미악화.

**검증**: 신규 `untyped_param_value_signedness.rs`×11(sign compare·small/large·neg-power-of-two·boundary·based-sign·sized-unchanged·width/replicate use·comma-list·typed-unchanged). **3664 green**(+11). clippy/fmt clean. format_version 22 불변.

#### 4.5.160 body param/localparam comma-list — `localparam A=1, B=2;` (loud→supported) (2026-07-19, branch feat-body-param-comma-list) ✅

**발굴 경위**: fresh-area sweep(signed mod/div·$countones/$onehot/$isunknown·indexed part-select·4-state X 전파·real-conv·do-while/break/continue·ternary-x·$sformatf 11종 全 MATCH or no-oracle→코어 견고 재확인) 후 item ② loud→supported로 pivot — §4.5.159서 §3에 기록한 body comma-list 갭 착수. `localparam A=1, B=2;`=vita E2002 loud(module-item 경로가 param 1개만 파싱 후 `;` 기대)·iverilog 수용(매우 흔한 구문).

**loud→supported 구현**(§4.5.159 인프라 재사용·parser-only): body param 경로가 `parse_param_prefix`(type prefix 1회) + `finish_param_assignment` comma-loop → 첫 name inline emit·나머지 **`pending_module_items` FIFO 큐**. drain=collection LOOP 최상단(module/iface/program/package body loop·`parse_gen_items_until`) + `parse_gen_branch` single-item(explicit)·**loop 조건에 `!pending.is_empty()` 추가**(comma-list가 end 키워드 직전 마지막 항목이면 첫 name이 cursor를 end로 전진→drain 전 loop 종료되는 edge). `param_item_to_module_item` 헬퍼(localparam const_locals 기록+`ConstArrayVar`→`NetVar`)가 **모든** item에 실행(첫+큐). dead `parse_param_decl` wrapper 제거. AST/schema/sim-ir/골든/format 全 불변.

**적대 2렌즈 CLEAN**(양 서브에이전트 확정 버그 0): differential(~63 probe)=continuation이 first와 byte-identical(width/sign/type 상속·`[3:0]`/`signed[7:0]`/int/byte/…·inter-param ref)·全 scope(module·package scoped `pk::B`+unqualified·interface·program·generate for/if BEGIN+single-branch)·queue leak 0(인접 submodule/instance/sibling gen block 미오염)·회귀 0(single param·header comma-list §4.5.159). soundness=producer 단일 site·`parse_module_item`(2 caller)·`parse_gen_item`(2 caller) **全 4 drain**·class body는 producer 미도달(별도 `parse_class_param_list`)·cross-scope leak 없음(body loop/`parse_gen_items_until`에 internal break/return 無→pending 잔류 exit=at_eof only=terminal)·FIFO·error-recovery panic/hang 0·const_locals 全 item·gen if/for/case single-item branch-scoped. `debug_assert!(at_eof || pending.is_empty())`(container 종료 후·future-proof).

**발굴 pre-existing(§2 기록·이 기능과 무관·comma-list 없이 재현)**: #1 untyped param↔param signed compare(`localparam A=-1,B=2; A<B`=vita `ge`/0 vs iverilog `less`/1·param↔literal은 정상) · #2 untyped param sum `%b` width(33 vs 32-bit) — **동근**(untyped param IEEE §6.20.2 값-결정 타입 미적용·sign+width **multipart**·§S⑤ defer). §4.5.159/이 슬라이스 무관(코드추론+param↔literal MATCH로 확정).

**검증**: `array_param.rs` canary flip(옛 "comma-list stays loud" 인코딩 → 새 정답 `1 2`·§5 룰=옛 버그동작 갱신) + 신규 `param_body_comma_list.rs`×10(untyped/typed/width·sign 상속/parameter/inter-ref/last-item/package-scoped/generate/single). **3653 green**(+10). clippy/fmt clean. format_version 22 불변.

#### 4.5.159 ANSI `#(…)` param-port comma-list — continuation이 type prefix 상속 (2026-07-19, branch feat-param-header-comma-inherit) ✅

**발굴 경위**: §2 中형 recorded "narrow-typed param init width"(§4.5.146) 재현 probe → `localparam logic [3:0] B = (2==2)` 등 **전부 MATCH**(후속 param_meta 작업이 해소 → §4.5.146 **non-reproducing** 판정). 이어 적대 comma-list probe로 신규 silent-wrong 발굴: `#(parameter [3:0] A=20, B=20)`=vita `A=4 B=20` vs iverilog `A=4 B=4`·`#(parameter signed [7:0] A=-1, B=200)`=vita `B=200`(unsigned 32-bit) vs iverilog `B=-56`(signed 8-bit wrap).

**silent-wrong — 헤더 comma-list continuation이 leading type 유실(parser·iverilog-diff)**: ANSI `#(…)` 헤더 루프가 각 comma 항목을 `parse_param_decl`로 **매번 새로 파싱** → 무장식 continuation(`, B=20`)은 kind=Parameter·ty=Implicit로 재-디폴트 → `B`가 값-사이즈 IMPLICIT 32-bit param이 됨(선언 `[3:0]`/`signed [7:0]` 미상속·width truncation·signedness 유실). first 항목(A)만 올바른 타입. **fix**(parser-only): `parse_param_decl`을 **`parse_param_prefix`**(`[param|localparam] [signing] [type] [packed_range]`를 1회 파싱→`ParamPrefix`) + **`finish_param_assignment`**(name+array/scalar+value·prefix 인자)로 분리. 헤더 루프는 그룹당 prefix 1회 파싱 후 무장식 continuation에 **동일 prefix 적용**(IEEE §6.20.1)·comma 뒤 fresh prefix 키워드(`, parameter …`)면 **새 그룹**(`starts_param_prefix`). AST/schema/sim-ir/golden/format 全 불변(ParamDecl 재사용·같은 노드 多 생성·format_version 22 유지).

**적대 2렌즈 CLEAN**: differential(iverilog 라이브 ~40 케이스)=`[3:0]`/`[7:0]`/`signed [7:0]`/multi-group/3-name/header-localparam/inter-param-ref(`B=A+1`) 全 상속 일치·회귀 0(untyped `A=1,B=2`·`int A,B`·body single `localparam [3:0] X`·body A2a array param 불변). soundness=`parse_module_like`의 **단일 헤더 루프**가 module/interface/program/package 4종 헤더를 uniform 커버(2258/2275/2288/2534 全 경유)·body single-param 경로(`parse_param_decl` wrapper) byte-identical·**격리 확정**: class value-param은 별도 typeless 경로(`parse_class_param_list`·`ClassParam{name,default}`가 type 폐기·no-oracle: iverilog가 typed class value-param 미지원)→**§2 기록**·body comma-list(`localparam A=1,B=2;`)는 여전히 honest-loud E2002(multi-emit 필요·**§3 기록**)·real-param `#(parameter real …)` E3009 DIFF는 **pre-existing**(stash-rebuild로 main도 동일 확인·ROADMAP §3 "real const-fold 전면 미지원" 기수록).

**검증**: 신규 `param_header_comma_inherit.rs`×7(narrow-width 상속·signedness+width 상속·3-name 공유·fresh-prefix 새 그룹·header localparam·untyped 불변·int 불변). **3643 green**(+7). clippy/fmt clean. format_version 22 불변(순수 parser 리팩터).

#### 4.5.158 enum label operand signedness — 선언 sign 상속 (2026-07-19, branch feat-enum-label-sign) ✅ [.vu re-pin]

**발굴 경위**: fresh-area 광역 sweep(interface/generate/hierarchical/casez/recursive-fn/foreach/param-override/string 전부 clean or no-oracle→코어 견고) 후 §2 中형 recorded(§4.5.154 differential 발굴 "enum-label 부호비교") 착수. `enum byte {A=-1,B=2} v=A; v>B`=vita **1**(255>2) vs iverilog **0**(−1>2 signed)·`B>C`(C=−3)=0 vs 1.

**silent-wrong — enum label sign이 per-value(elaborate·iverilog-diff·§4.5.153-154 계열)**: 라벨 sign을 `param_meta`에 `v < 0`(값별)로 등록 → **signed enum의 POSITIVE 라벨**(B=2)이 unsigned→IEEE §11.8.1 collective로 비교 전체 unsigned화(오답). 변수 whole-value sign(§4.5.153)·width(§4.5.154)는 fixed였으나 라벨은 AST enum 노드가 sign 미보유라 per-value. **fix**: AST `TypedefKind::Enum`에 `signed: bool` 추가(base 선언 sign=`TypeInfo.signed` §4.5.153/154 resolve값·파서서 `enum_signed=info.signed` 배선·**`.vu` schema re-pin**·sim-ir 불변·format 22)·라벨 sign=**`enum_signed || (v<0)`**(signed enum=全 라벨 signed·unsigned enum의 negative[illegal]는 graceful signed). base-less `enum {…}`=int→base_w=`Some(32)`로 라벨도 도달.

**적대 2렌즈 CLEAN(+발굴 즉수정)**: differential(110+ 케이스)=全 signed enum(byte/shortint/int/longint/logic-signed/base-less) 비교 iverilog SIGNED 일치·byte-identity(unsigned enum·label VALUE/`%0d`/`%b`·arith·`==`/`!=` sign-agnostic·case·method·concat self-width·`$bits`·array-idx 全 불변)·collective §11.8.1 정확(positive signed label이 unsigned 상대 잘못 signed화 안 함)·신규 divergence 0. soundness=sign-consumption 경로 확정(`walk_scopes(param_meta)`→`make_const_i64(v,w,signed)`)·`||v<0` 정확·base_w=32(unfoldable-range는 None 유지)·golden(sim-ir 무참조·re-pin만)·**KEY GAP 발굴=body-local enum**(`push_body_enum_labels`이 `self.params`만·param_meta 미설정→함수 내 라벨 sign/width value-inferred silent-wrong)→**§3 全 경로로 즉수정**(param_meta scoped save/restore[`restore_param_meta` 헬퍼·4 caller]·`push_pkg_consts_scoped` 선례 미러·pollution 없음 spot-check). 잔여 minor=unfoldable-range signed enum(base_w=None→value-inferred·극한 엣지)·generate-scope enum label=loud E3010(honest).

**검증**: 신규 회귀 테스트 `enum_label_sign.rs`×6(signed 라벨 relational·base-less int·unsigned 불변·value/arith 불변·**body-local**[hand-IEEE·iverilog가 body-local 라벨 bind 불가]·vita-내부 등가[enum 라벨≡plain signed const]). `schema_hash.rs` re-pin(+이유 doc). **3636 green**(+6). clippy/fmt clean. format_version 22 불변(`.vu` re-pin·sim-ir·golden 불변).

#### 4.5.157 atom+packed-dims loud-reject — 全 decl-site 완결 (§4.5.156 follow-through) (2026-07-19, branch feat-atom-dims-reject-allsites) ✅ [§3 全 경로 커버]

**발굴 경위**: §4.5.156이 `parse_net_var`만 loud-reject해 **divergence 생성**(§3 "한 경로만 고치면 divergence>uniform-wrong"): sibling decl-site(typedef/port/tf-port/struct-member/param/for-init)는 여전히 `byte [7:0] x` 등 atom+packed-dims illegal decl을 lenient 수용 vs iverilog reject. 6 sibling site 전부 vita ACCEPT/iverilog REJECT 확정.

**loud→supported — 全 decl-site 배선(파서·iverilog-diff·§3 全 경로)**: §4.5.156 inline check를 **공유 헬퍼 `reject_packed_dims_on_nonvector(kind, has_dims)`**로 추출(allow-list=`is_net()`[trireg/uwire 포함]+logic/reg/bit, IEEE §6.11 정확)하고 **9 decl-site + 3 type-spec-site**에 배선: `parse_net_var`·typedef-alias·struct-member·for-typed-init·ANSI-port·non-ANSI-port·ANSI-tf-port·non-ANSI-tf-port·typed-param(+**적대 soundness가 발굴한 3 잔여**: enum-base `enum byte [3:0]`·class-value-param `#(int [3:0])`·function-return `function int [7:0]`). per-site has_dims: struct-member=range만·param=`explicit_range`만(atom `forced_range`[byte→[7:0]] 제외)·func-return=Int-only(byte/shortint/longint는 forced range→leftover-`[` parse-error로 이미 loud). port/tf-port=typedef 해소 후 `net_or_var`(atom typedef는 range=None→no-op·vector typedef 허용).

**적대 2렌즈 CLEAN**: differential(~90 케이스 全 site)=**over-rejection 0**(legal typedef/port/tf-port/struct/param/for-init/enum/class/func-return 전부 accept·vita reject는 전부 pre-existing[unpacked-dims parser gap·multi-dim-packed·`trireg` E3009 등]으로 committed 바이너리와 byte-identical)·correct-reject 全 site iverilog parity. soundness=9 site 개별 SOUND(predicate·allow-set·single-error recovery·format 무영향)+**3 type-spec 잔여 발굴**(→즉시 배선)+주석 2건 정정(ANSI-port soundness 근거=atom-typedef range=None·param int-vs-byte reject 메커니즘). 잔여 pre-existing(orthogonal·非-atom-dims)=`class #(int X)` typed value-param을 iverilog 미지원(vita 수용·범위 밖).

**검증**: `atom_dims_reject.rs`에 all-sites 테스트 추가(10 site×illegal reject: var/typedef/port-ANSI/port-nonANSI/tf-ANSI/tf-nonANSI/struct/param/for-init/enum/class/func-return). **3630 green**. clippy/fmt clean. format_version 22 불변(파서 reject·IR-0).

#### 4.5.156 loud-reject a packed range/dimension on a non-vector type (byte/int/…/real/string/event) (2026-07-19, branch feat-atom-dims-reject) ✅ [loud→supported·correct-or-loud]

**발굴 경위**: §4.5.155 soundness 리뷰어 발굴 R1 재그라운딩. fresh-area probe(queue/dyn-array/assoc/streaming/real-math/`$sformatf`/part-select/mixed-sign/shift 전부 clean or no-oracle→코어 견고 확인) 후 loud→supported로 pivot. `byte [7:0] x`/`int [3:0] x` 등 fixed-width atom에 packed range/dims를 붙인 **illegal decl**(IEEE §6.11: `integer_atom_type`·real/string/event는 packed_dimension 불가)을 vita가 lenient 수용 vs iverilog compile-reject.

**loud→supported — atom+packed-dims parse-reject (파서·iverilog-diff)**: `parse_net_var`가 `opt_range()`+`opt_packed_dims()` 후 무검증 → single-range는 `range_to_dims`가 range 드롭(net 8·self-consistent but non-conformant)·**second packed dim은 genuine 발산**(`byte [7:0][1:0]`=`packed_extents` 8×2=16 vs `range_to_dims`/`$bits` 8). **fix**=`parse_net_var`에 allow-list 가드: `range.is_some()||!packed.is_empty()`인데 `!(kind.is_net() || Logic|Reg|Bit)`면 loud `E2002`(emit+keep-decl→나머지 파스 정상·elaborate는 errors>0로 skip). ALLOW-LIST(§3 원칙)=vector-typed(nets+logic/reg/bit)만 허용·나머지 전부 reject=construction상 완전.

**적대 2렌즈 CLEAN**: differential(110+ 케이스)=**over-rejection 0**(legal 60여종[scalar atom·vector+range·全 net+range·unpacked array·multi-packed vector·param bit/logic] 전부 accept)·correct-reject 15+종 iverilog 완전 parity(byte/shortint/int/longint/integer/time/real/realtime/event/string+range·multi-packed·signed). soundness=allow-list 정확(`is_net`이 trireg/uwire 포함 확인·reject-set≡IEEE atom+dimensionless)·ClassHandle/VirtualIface는 `parse_net_var` 미도달(defensive)·recovery clean(parse-error→elaborate skip·no panic·no cascade)·format 무영향. **under-rejection(sibling decl 경로)=전부 pre-existing**(fix는 error 추가만·새 accept 불가): 리뷰어가 정확화한 nit=single-range는 net 8 self-consistent(원 커밋 "net 16" 부정확→주석/doc 정정)·genuine 16-vs-8은 multi-packed만.

**검증**: 신규 회귀 테스트 `atom_dims_reject.rs`×3(atom+range reject[byte/shortint/int/longint/integer/real/multi-packed/event]·legal 11종 accept·unpacked array 불변). `legality_semantics.rs` 갱신(`event [3:0]`=elaborate-reject→parse-reject 이동·이유 주석·커버리지 `cli::atom_dims_reject`로 이관). 신규 §3 등재=sibling decl 경로(ports/typedef/tf-ports/struct-member/for-init/param) under-rejection(§4.5.156 soundness map). **3629 green**(+3). clippy/fmt clean. format_version 22 불변(파서 reject·IR-0).

#### 4.5.155 `$bits` of a fixed-width atom array element (byte/shortint/int/longint) (2026-07-19, branch feat-bits-atom-array-elem) ✅

**발굴 경위**: §4.5.154 review가 기록한 follow-on(`$bits(unpacked-array-elem)` of atom=1) 재그라운딩. `byte a[2]; $bits(a[0])`=vita **1** vs iverilog **8**(shortint/int/longint=1 vs 16/32/64). **storage/`%b`(10100101)/arith는 이미 8-bit 정확** — `$bits` reporting만 오류.

**silent-wrong — `$bits` prescan atom width 누락 (elaborate·iverilog-diff)**: `$bits`는 `bits_of_view`가 `from_prescan`(정적 `bits_prescan`) 우선 소비(const site `prescan_first=true`·runtime는 1-D 완전인덱스서 `from_table` miss→prescan fallback). `prescan_net_bits`가 `Integer=32`·`Real/Time/Event=64`만 특별처리하고 **byte/shortint/int/longint를 `_ => {range None => 1}` 폴백**으로 보냄(`range_to_dims`는 8/16/32/64 처리하는데 prescan이 미러 안 함·§4.5.154 scalar fix는 net 경로 `range_to_dims`만 탐). **fix**=prescan에 atom arm 추가(Byte=8·Shortint=16·Int|Integer=32·Longint=64·`range_to_dims` 미러). **보너스**: const-context scalar `$bits(byte)`(=prescan 우선)도 1→8 교정(`localparam W=$bits(b)`·`logic [$bits(b)-1:0]`). 

**적대 2렌즈 CLEAN**: soundness=**`from_table`는 fix 불요 확정**(unpacked byte-array net은 `range_to_dims`로 `nv.width=8` 이미 저장·1-D 완전인덱스는 `array_dims` absent라 from_table miss→prescan이 유일 authority)·`bits_prescan` 유일 populator·legal SV서 atom은 range/packed 미보유라 prescan≡range_to_dims≡net·value-only(elaborate-local BTreeMap·non-serialized). differential=3-way(pre-fix 바이너리 `1 1 1 1` 재현→post `8 16 32 64`)·byte-identity(logic-range/integer/bit/whole-array/packed/real·scalar 전부 pre==post==iverilog)·신규 divergence 0·51 케이스. 잔여(pre-existing·별개)=md-array **row** `$bits(m[0])`(partial-index sub-array=next-pow2 오류·logic도 동일·atom무관)·illegal `byte [7:0] x`(atom+dims lenient accept·loud 대상)→ROADMAP §2/§3.

**검증**: 신규 회귀 테스트 `bits_atom_array_elem.rs`×6(atom elem 8/16/32/64·whole+multidim elem·forward-ref·ranged/integer/bit 불변·**const-scalar 보너스**·vita-내부 등가(elem≡scalar≡`%b` 폭)). **3626 green**(+6). clippy/fmt clean. format_version 22 불변(IR-0).

#### 4.5.154 enum built-in base kind preservation — width + 2-state-ness (byte/shortint/int/longint/time/logic/bit) (2026-07-19, branch feat-enum-atom-base-width) ✅

**발굴 경위**: §4.5.153 발굴한 recorded §2 follow-on("enum atom-base width")을 mechanism-level 재그라운딩. iverilog 차분 sweep: `$bits(enum byte)`=vita **32** vs iverilog **8**·`enum logic`(bare)=32 vs 1·`enum time`=32 vs 64. 근본=파서 `parse_typedef`가 range 없는 built-in base(atom `byte`/`shortint`/`int`/`longint` + bare `logic`/`bit`/`reg` + base-less)를 전부 4-state 32-bit `Integer` `None` arm으로 뭉갬 → **width(변수+라벨 양 경로)·2-state-ness 소실**.

**silent-wrong ① — enum base width (파서·iverilog-diff·recorded §2)**: `$bits`/`%b`/concat/replication/struct-member/port/func-return 폭 전반 오류(`enum byte`=32 not 8). **fix**=파서가 **실제 base kind를 `TypeInfo`에 보존**(range=None·plain `byte`/`int`/`logic` decl과 byte-identical) + label-width 경로(`enum_base_width`·AST `base`)엔 kind 폭의 range를 합성. width machinery(elaborate `range_to_dims`)·2-state(`net_kind_is_two_state`)·sign(`atom_default_signed`)이 전부 plain-atom 경로 재사용. per-kind: byte/shortint/int/longint=2-state signed(8/16/32/64)·integer=4-state signed 32·time=4-state unsigned 64·logic/reg=4-state unsigned 1·bit=2-state unsigned 1·base-less=int(2-state 32).

**적대 Rev1 회귀 2건(soundness 발굴)→redesign**: 초기 시도는 atom을 `kind:Logic,range:Some`으로 라우팅 → (a) `simple_typedef_cast`(`kind∈{Logic,Reg}` 게이트)가 atom enum cast를 admit→2-state coercion 없이 desugar(`enum byte'(4'b1x01)`=`00001x01` X누수 vs 이전 E3009 loud) (b) enum-typedef-base 가드(`info.range.is_none()`)가 range=Some로 새어 loud→accept. **redesign=실제 kind 보존**(위)이 두 회귀 근본 해소: Byte/Bit∉{Logic,Reg}·range=None이라 cast는 E3009 loud 복원·guard도 유지. 리뷰어 확인 CLEAN(main과 동일 loud terminus).

**silent-wrong ② — int/base-less 2-state (differential 발굴·pre-existing·①이 부각)**: `enum int`·base-less `enum {…}`(암묵 base=int)가 4-state `Integer`로 모델링→uninit=X(state-machine `enum{IDLE,RUN}` read-before-assign이 X vs iverilog 0). ①이 byte/shortint/longint를 2-state로 만들며 이 pre-existing 불일치 노출. soundness 리뷰어가 정확한 remedy 제안("map int→Int, base-less→Int"). **fix**=int-family를 kind 보존에 균일 확장: int→`Int`(2-state)·integer→`Integer`(4-state 유지)·time→`Time`(64 width 보너스)·base-less None-arm=`Integer`→`Int`. plain int/integer/time와 byte-identical.

**적대 2렌즈×2라운드 CLEAN**: Rev1 리뷰(differential 70+·soundness)가 회귀 2건 발굴→redesign 후 Rev2 리뷰=**both findings CLOSED**(정확한 predicate citation)·신규 loud→silent gap 0·kind-preservation 全 consumer(cast/`$bits`/2-state-init/VCD var-type/struct-member/port/func-return/label) SOUND·differential 2-state/width/sign 정확·byte-identity(int/integer/vector-range/plain var). 잔여 발굴(pre-existing·별개축)=enum-label 부호비교·`$bits(unpacked-elem)` atom→ROADMAP §2.

**검증**: 신규 회귀 테스트 `enum_atom_base_width.rs`×9(atom width/bare-vector 1-bit/int-integer-baseless 32/concat/width×sign/int-baseless 2-state/integer 4-state 유지/time 64/**vita-내부 등가**(enum byte ≡ plain byte)). **3617 green**(+9·회귀 0). clippy/fmt clean. format_version 22 불변(IR-0·`TypeInfo` parser-internal·AST `base`는 Option<Range> 형상 불변·value-only).

#### 4.5.153 enum base-type signedness — vector-base `signed` + atom-base `unsigned` whole-value (2026-07-19, branch feat-enum-signed-base) ✅

**발굴 경위**: fresh-area probe(§4.5.152 signedness 계열의 sibling·type-qualifier whole-value 후보)로 신규 silent-wrong 사냥 중 **`typedef enum logic signed [N] {…}`** 발견 — struct/union(§4.5.152)의 정확한 enum-base sibling. 파서 주석이 이미 이 한계를 자인("the built-in `enum logic signed[N]` path also drops signedness — a separate pre-existing limit").

**silent-wrong ① — enum vector-base `signed` whole-value = unsigned (파서·iverilog-diff 확정)**: `parse_typedef`가 built-in enum base의 `signed`를 `let _ = self.opt_signed()`로 **폐기**하고 enum typedef `TypeInfo.signed`를 `false` 하드코딩 → enum을 한 값으로 읽을 때 unsigned(`enum logic signed[3:0]{B=-1} et; et e=B; %0d`=vita **15** vs iverilog **−1**). compare(`e<0`=0 vs 1)·arith(`e+1`=16 vs 0)·wider 대입 sign-extend(`logic signed[7:0] w=e`=15/00001111 vs −1/11111111) 발산. bit 패턴(`%b`=1111)·unsigned base·default-int base(no explicit base·이미 signed)는 정상. **fix**: `signed`를 `TypeInfo.signed`로 캡처(§4.5.152 struct/union과 **동일 퍼널** — enum도 `self.typedefs`에 저장돼 plain `typedef logic signed[N] alias`와 동일 소비).

**silent-wrong ② — enum atom-base 명시 `unsigned` 폐기 (파서·pre-existing·soundness 리뷰어 발굴·同 메커니즘 atom-arm)**: `TypeInfo` 빌드의 `None`(atom/base-less) arm이 `signed:true` 하드코딩 → atom base(`int`/`integer`)의 명시 `unsigned`도 폐기(`enum int unsigned{A=32'hFFFFFFFF} e=A; %0d`=vita **−1** cmp=1 vs iverilog **4294967295** cmp=0). §2 "全 경로 커버"상 ①의 vector-arm만 고치면 divergence>uniform-wrong이므로 **동일 커밋에 양 arm 완결**: `base_signed`를 `bool`→`Option<bool>`로 확장, vector arm=`unwrap_or(false)`(default unsigned·§7.2.1), atom/base-less arm=`unwrap_or(true)`(default signed·`int`은 32-bit signed). qualifier-less enum은 양 arm 모두 old default와 동일→**byte-identical**. (`integer unsigned`는 iverilog 자체 self-contradictory[−1 표시/cmp=0 unsigned·§4.5.91 기록]→vita 4294967295/cmp=0가 spec-correct.)

**적대 리뷰 2렌즈 CLEAN**: differential(라이브 iverilog 70+ 케이스)=widths 4~65bit·sext(signed/unsigned target)·arith/compare/shift/concat(unsigned로 소실)·ternary(mixed=unsigned)·cast·**case collective-signedness(§4.5.152 trap 포함)**·func arg/return·struct/union member·port·array·ascending base·2-state base 전부 iverilog 일치·비-signed enum(default/unsigned/int/byte·enum method `.name/.next`·VCD) **byte-identical**·발견 divergence는 전부 iverilog-reject(out-of-range label lenience 등 pre-existing). soundness(정적)=`parse_typedef` 단일 enum-base site 확정(typedef-name signed base·inline enum decl은 이미 honest-loud)·`base_signed` arm별 default 정확·enum var는 generic `NetVarDecl.signed` 경유(enum-ness는 `var_enum` 별도·signedness 무관)·label 경로(`enum_base_width`/`TypedefKind::Enum`)는 AST range만 참조·per-value 부호 유지(불변)·IR-0(`TypeInfo` parser-internal non-Serialize).

**검증**: 신규 회귀 테스트 `enum_signed_base.rs`×8(vector whole-value/sext/unsigned-base 불변/default-int 불변/atom-`int unsigned`/atom-signed·base-less 불변/struct-member/**vita-내부 등가**(signed enum ≡ plain `logic signed[7:0]` char-identical)). stale 주석 2건 갱신(elaborate `enum_base_width`/label-lower: base signed는 이제 변수 whole-value로 캡처·라벨은 AST range만이라 per-value 유지). **3611 green**(+8). clippy `-D warnings`·fmt clean. format_version 22 불변(IR-0).

#### 4.5.152 signed packed struct/union whole-value signedness + case collective-signedness (2026-07-18, branch feat-signed-packed-struct) ✅

**발굴 경위**: fresh-area probe(streaming·signed-div·x-prop·real-fmt·casez/wand·NBA·packed-struct 등 배터리)로 신규 silent-wrong 사냥 중 **`typedef struct packed signed {…}`** 발견 — `signed` 키워드가 whole-value로 안 먹힘.

**silent-wrong ① — signed packed struct/union whole-value = unsigned (파서·iverilog-diff 확정)**: 파서 `parse_typedef_struct`/`parse_typedef_union`이 `struct packed signed`의 `signed`를 `let _ = self.opt_signed()`로 **폐기**("sign ignored for layout" 주석)하고 typedef `TypeInfo.signed`를 `false` 하드코딩 → 구조체를 한 값으로 읽을 때 unsigned(`struct packed signed{logic[7:0] v} s; s=8'hFF; %0d`=vita **255** vs iverilog **−1**). 렌더링뿐 아니라 compare(`s<0`=0 vs 1)·arith(`s+1`=256 vs 0)·arith-shift(`s>>>1`)·wider 대입 sign-extend(`logic signed[15:0] w=s`=255 vs −1) 전반 발산. **fix**: `signed` 키워드를 `TypeInfo.signed`로 캡처(struct·union 양쪽). 멤버 접근(`s.field`)·packed 레이아웃·member별 signedness는 **불변**(멤버는 `StructLayout.fields[i].signed` 독립 경유). `signed` 없으면 `opt_signed()→None→false`로 **byte-identical**(모든 기존 struct). IR-0(`NetVar.signed`는 기존 필드·VALUE만 변경·SchemaHash/format_version 불변·`TypeInfo`는 parser-internal non-Serialize).

**silent-wrong ② — case/casez/casex collective-signedness (elaborate·pre-existing·①이 노출)**: 적대 differential 리뷰어가 `case(signed_struct)`서 발견 — **plain `reg signed [3:0]`도 동일**(struct 무관 증명). §12.5/§11.8.1: case 비교는 scrutinee AND **모든** label이 signed일 때만 signed(하나라도 unsigned면 전체 zero-extend). vita 엔진은 `CaseEq(scrut,label)`를 **pair별**(`signed(l)&&signed(r)`)로 sizing → signed scrut이 unsigned sibling label 있어도 signed label(`-1`)엔 sign-extend해 오매치(`case(s=4'hF) -1: ; 4'hF: ;`=vita `neg1` vs iverilog `hF`). ①이 signed struct을 올바로 signed화하면서 이 패턴이 correct→wrong로 노출(regression). **fix**(`lower_case`): label을 한 번만 lower(arena 재사용)→collective signedness 계산→`scrut_signed && !collective`면 scrut을 `$unsigned`로 **1회 래핑**. 그러면 모든 pair가 `pair_signed=false`→scrut·label **양쪽 zero-extend**=collective-unsigned 규칙 정확 재현(narrow signed label widening 잔여도 자동 해소·`$unsigned`는 width·x/z 보존). all-signed set=signed 유지·unsigned scrut=no-op(byte-identical). `case_label_eq`→`lower_case_label`+`case_cmp` 분리. 엔진·CaseEq 시맨틱 불변. IR-0(signed-scrut+unsigned-label 설계만 `$unsigned` 노드 1개 추가·형상/골든 불변).

**적대 리뷰 2렌즈 CLEAN**: soundness(정적 code-path)=`TypeInfo.signed` 단일 퍼널 확정(module var/port/tf-port/func-return/array/package 全 site가 `info.signed` 경유·하드코딩 signed:false는 non-struct/unpacked뿐)·멤버 per-field·value-only 확정. differential(라이브 ~45 케이스)=signed struct/union display/compare/arith/shift/ternary/concat(unsigned로 소실)/psel(unsigned)/sext/x-z/cross-boundary 전부 iverilog 일치·plain/unsigned struct byte-identical — **단 case collective-signedness 1건 발견**(→②로 수정). VCD 자가검증=signed/unsigned struct 동일 `$var wire 8`·동일 비트벡터(VCD sign-agnostic).

**검증**: 신규 회귀 테스트 2파일(`signed_packed_struct.rs`×6 — whole-value/sext/member-unsigned/union/unsigned-불변/**vita-내부 등가**(signed struct ≡ plain `logic signed [7:0]` char-identical) · `case_collective_signedness.rs`×6 — plain/struct/all-signed/narrow-label/unsigned/casez). clippy `-D warnings`·fmt clean. format_version 22 불변.

#### 4.5.151 전면 감사(spec↔코드 정합 + 적대 버그헌트 + 문서 최신화) — silent-wrong 3·panic 2·게이트 갭 1 수정, format_version 21→22 (2026-07-17, branch main) ✅

**방법론**: Fagan 역할분리 리뷰어 8팀 병렬(spec-정합 4축=diag/artifact-schema/VCD-FST-timescale/OBS · 적대 버그헌트 2축=sim-engine/front-end · 코드위생 1 · 문서일관성 1) → 발견 전량을 **라이브 iverilog 차분으로 확정/반증** 후 수정. 리뷰어 발견 중 1건(psel negative-offset READ 의혹)은 **차분이 반증**(vita `100x`/`00xx` = iverilog byte-일치) — 정적 추론만으로 수정했으면 회귀였을 것.

**silent-wrong 3건 수정(전부 iverilog-diff 확정)**:
1. **`**` 지수 truncation** (`eval.rs` Pow): self-determined 지수를 결과폭으로 `resize`해 좁은 결과폭에서 지수 VALUE가 절단(`logic[3:0] r = 2**18`→vita 4 vs iverilog 0 — 18 mod 16=2로 읽음). fix=지수는 widen-only(`max(w, own)`), 결과만 `arith` 후 w로 절단. native 경로는 POW_MAX=16 초과 시 오라클-bound라 자동 수혜.
2. **>64-bit signed→real 0.0** (`value.rs::to_i128_signed`): 64비트 게이트라 `signed [99:0]` −5가 `to_f64`서 None→`unwrap_or(0.0)`(vita 0 vs iverilog −5). fix=128-bit lane으로 확장(65..=128 부호 재구성 + unsigned 65..127 양수·>128=None 계약 유지). arr_cmp sort key·real-math arg도 동시 수혜. 잔여 known-edge=width>128→real은 여전히 0.0(초희귀·ROADMAP §3 deep 기록).
3. **timescale 2단계 반올림 미구현** (**HIGH·format_version 21→22**): doc-08은 `#delay`를 ①모듈 자신의 precision으로 반올림→②global tick 스케일 2단계로 명세하나 구현은 global grain 1회 반올림 — 혼합-precision 설계서 발산(`1us/10ns` 모듈 `#3.453us`=vita 34530 vs iverilog **34500** ticks·`1ns/1ns` 모듈 `#2.5`=2500 vs **3000**). fix=`ResolvedTimescales.prec_exp`(per-module) 신설→elaborate `mod_prec_exp`/`cur_prec_mult`/`proc_prec_mults`(per-process S=10^(prec−global))→`SimOpts.proc_prec_mults`→엔진 `delay_ticks`/const `const_delay_ticks`가 `round(d×M/S)×S`. **staged 배선**: `.vu` timescale tail=(unit_exp, global_prec, **prec_exp**) triple·`.velab` trailer=(proc_multipliers, global_prec, **proc_prec_mults**) triple → **v22 bump**(sim-ir 불변·SimIr 골든 v19 핀 유지·v20/21 동급 trailer-only). S=1(단일 timescale/레거시 4-인자 entry)=byte-identical. one-shot+staged 양 경로 iverilog 일치 검증.

**panic→clean 2건**: ④ package enum 라벨 `i64::MAX`+1 auto-increment가 unchecked `v+1`(module/body-local 쌍둥이는 이미 `wrapping_add`) — debug SIGABRT→wrapping(iverilog `8000…0` 일치). ⑤ md-packed `x[-1 +: 2]` const offset이 `const_eval_u32` wrapping_neg로 0xFFFF_FFFF→`c+w` u32 overflow debug panic→u64 승격+`u32::try_from` 가드로 clean loud(iverilog도 loud 거부).

**게이트 갭 1건**: ⑥ `W-FLIST-OVERRIDE`가 raw `eprintln!`으로 GatedSink 우회 — doc-15가 약속한 `-Werror=W-FLIST-OVERRIDE` 승격 불가·counts epilogue 미포함. fix=파싱 중 record→sink 생성 직후 replay(`record_override`/`emit_flist_overrides`·5 진입점+`--dump-filelist`). 승격(exit≠0)/기본(warnings 카운트)/억제 3모드 검증.

**하드닝·정리**: ⑦ FST `extend_vector` fast-path 대문자 X/Z 정규화 누락(latent — vita writer는 소문자만 방출) 수정. ⑧ doc-03 명세의 `separate-bins` dev 피처 실구현(3-line shim bins `src/bin/{vcmp,velab,vrun}.rs`+`cli::driver_main()` 공유·multicall과 동일 코드 경로). ⑨ `hdl-builtins`에 `publish=false`(dev-stub 형제 일관). 코드위생 스윕 판정=**중복/미사용/디버그 잔재 0건**(유사 코드는 전부 의도된 이중 경로 — 정확도 우선 KEEP)·스파게티 top5(lower_stmt 등 mega-match)는 시맨틱-carrying이라 leave-as-is.

**stale 주석 5곳 정정**: elaborate `warn()` W3008→W3056 실태·obs.rs 모듈헤더(OBS-1a만→1a/1b/2/3 전부)·vita-artifact/cli "RULE-V Phase-2 예정"→구현 완료 실태·probe 베이스라인 주석(t0 drive는 첫 chg로 기록됨).

**신규 ROADMAP 등재(loud=안전·수정 비대상 판정)**: `64'hFFFF_…`(bit63 set unsigned 64-bit) param 리터럴=E3009 over-reject(iverilog 수용 — naive `v as i64` fix는 downstream const 부호 비교를 silent-wrong으로 만들 위험이라 보류·§3 큐) · partial-timescale 정책 진단(`--timescale-policy`)=doc-08서 future로 강등+§3 큐.

**문서 최신화(2-2: 코드가 옳은 곳 전부)**: format_version 8→22(doc-14/16/17)·MsgCode 산문 55→58(doc-02/15·본문 entry는 이미 58 bijection)·MSRV 1.82→1.85 약 20곳(README/CONTRIBUTING/doc-02/03/09/18/manual)·FST 언급(README/doc-00/01/04/05/manual-000)·doc-15 구조(8xxx 헤더 위치·reserved 목록·E3001 실트리거·E-ART-FORMAT-MISMATCH 런타임 재사용)·doc-19 필드 정합(trace=4-state binary·stage=%0d decimal·coverage.json=covergroup-only·run_id 미구현·`utc_unix_s`)·doc-04 hdl-builtins 실태·crate 수 15→17·doc-09 깨진 링크·doc-07 id-code 오기(`"!`→`!!`)·CHANGELOG v22 entry.

**검증**: **3591 green**(기존 3579 무회귀 + 신규 회귀 12: pow 1·pkg-enum 1·psel-loud 2·wide-real 1·two-stage 3·flist-gate 3·fst-case 1) · clippy `-D warnings` clean · fmt clean · obs run.json format_version 핀 21→22 갱신 1건이 유일한 기존 테스트 변경.

#### 4.5.150 FST: fst-writer 0.2.6→0.3.1 + MSRV 1.82→1.85 (Surfer 상호운용 수정) (2026-07-17, branch fix-fst-surfer-msrv185) ✅

§4.5.149 FST를 **Surfer 0.7.0(wellen 리더)로 시각 검증하다 발견**: vita FST가 Surfer에서 `FailedToLoad(Fst, "I/O operation failed")`로 거부됨(GTKWave `fst2vcd`·`fst-reader`는 관대해 통과했으나 wellen은 엄격). 근본원인 = **`fst-writer` 크레이트의 타임테이블 인코딩 버그**(vita 무관 — raw fst-writer만으로 재현).

- **수정**: `fst-writer` `=0.2.6`→`=0.3.1`(타임테이블 버그 대거 수정·`de342f15`/`d51aac45`/`9ddbf3b4` + wellen end-to-end 테스트 추가 `9cfe669b`). 0.3.x=edition 2024 → **워크스페이스 MSRV 1.82→1.85**(rust-toolchain.toml·`Cargo.toml`·vita 코드는 edition 2021 유지, floor만 상향; 사용자 사전 승인). 0.2.x용 트랜지티브 edition2024 회피 핀(proc-macro-crate·indexmap) 제거(1.85는 edition2024 지원). 트랜스코더 코드 불변(0.3.1 API 동일).
- **검증**: 0.3.1로 현실적 설계(clk/count/nibble/real/wire·수백~수천 스텝) 전부 Surfer 로드 OK·값 canonical 동치. 기존 6 FST 테스트 green 유지.
- **잔여 known-edge(honest-loud·silent-wrong 아님)**: fst-writer 0.3.1도 **upstream 미해결 [issue #4](https://github.com/ekiwi/fst-writer/issues/4)**(타임테이블 LZ4)로 **특정 소형 크기**(재현 최소 = 값-변화 시각 11개)서 wellen/Surfer `I/O operation failed` loud 거부. n=1..1000 스윕서 11만 실패(50~1000 전부 로드)→대형 덤프 안전·소형은 VCD 권장. 로드되는 FST 값은 항상 정확(loud 거부일 뿐). SPEC preview/07 §FST known limitation에 기록·상류 리포트 대상. IR-0.

#### 4.5.149 FST waveform output (`$dumpfile("x.fst")` / `-o x.fst`) — G2 breadth (2026-07-17, branch feat-fst-waveform) ✅

GTKWave/Surfer 네이티브 **FST** 파형을 VCD와 **동일한 in-code 인터페이스**로 지원(신규 기능). `$dumpfile` 인자(또는 CLI `-o`)의 확장자가 `.fst`(대소문자 무관)면 FST, 그 외 VCD. `$dumpvars`·`$dumpoff`·`$dumpon`·`$dumpall`·`$dumpflush`·`$dumplimit` 전부 FST 경로서 그대로 동작. SPEC=[preview/07 §FST 파형 출력](preview/07-vcd-format.md).

- **설계(VCD→FST 트랜스코드, `vcd-writer/src/fst.rs::transcode_vcd_to_fst`)**: 시뮬 코어는 검증된 VCD를 사이드카(`<경로>.fst.vcdtmp`)로 쓰고 `simulate` 종료 시 FST로 변환·사이드카 삭제. 모든 dump 의미론(`$dumpoff`→전-x 등)이 이미 VCD 값-변화 스트림에 반영 → 그 스트림을 재생하면 **FST≡VCD 구성적 성립**(값-변화 경로마다 2nd 바이너리 싱크 배선 안 함 → silent-wrong 분기 여지 無). 압축 블록(geometry/hierarchy/value-change)은 순수-Rust `fst-writer`=0.2.6(BSD-3, Cornell/Laeufer) 위임 — vita가 바이너리 직접 안 만듦.
- **의존성/MSRV(툴체인·MSRV 상향 불요)**: fst-writer 0.2.x(MSRV 1.73 < 워크스페이스 1.82 핀; 0.3.x는 Rust 1.85 요구) `=0.2.6` 핀 + edition2024 회피용 트랜지티브 핀(`proc-macro-crate`=3.3.0 → toml_edit<0.23 → toml_datetime 0.6.x·`indexmap`=2.7.1)을 `vcd-writer/Cargo.toml`에 직접 선언(--locked 정책상 커밋 lock이 durable pin).
- **적대 리뷰(differential+soundness)서 self-introduced silent-wrong 1건 발굴·즉수정**: `$dumpoff` 중 real은 VCD서 전-x 벡터(`bxxx…x`)로 방출되는데(vita 특성) 이를 FST real(f64) 시그널에 벡터 바이트로 먹이면 **garbage float**(206842847014058100…)로 되읽힘=silent-wrong. fix=real 대상 벡터값 → **NaN**(iverilog `rNaN` 동치·real-domain unknown). 회귀 `real_dumpoff_maps_to_nan`.
- **검증(correct-or-loud)**: 독립 리더 `fst-reader`=0.10.2로 되읽어 파형 동치 확인. 커버=scalar·vec·real·wide-64b·x/z·`$dumpoff`/`$dumpon`·중첩 scope·>94 시그널(멀티문자 idcode)·음수/지수 real, iverilog-13.0 파형 핀. 경로=one-shot `$dumpfile`·`-o`·staged `vrun -o`·`.FST`. 단위(`vcd-writer` `fst::tests`)+엔드투엔드(`cli/tests/fst_waveform.rs`). IR-0(sim-ir 형상·format_version 불변).

#### 4.5.148 explicit single-symbol TYPE import (`import p::t;`) loud→supported (2026-07-17, branch feat-explicit-import-type) ✅

`import p::my_t;`(단일-심볼 TYPE 임포트)=vita `error[VITA-E3009]: package `p` has no symbol `my_t`` loud(over-reject) vs iverilog 정상. wildcard `import p::*`·bare `p::my_t` read는 이미 지원 — 명시적 단일 TYPE 임포트만 누락(③ loud→supported).

- **원인**: `apply_import_consts`(elaborate)의 "no symbol" E3009 가드가 consts/vars/funcs/tasks만 조회·TYPE 미조회. 타입은 파서가 parse-time에 해석(스코프 twin `p::t`→bare `t` 복사, `hdl-parser/src/lib.rs:2056-2077` 단일-임포트 arm — scalar/packed-struct·enum·unpacked-struct·union 5종 맵 全 복사)하므로 elaborate엔 BIND할 게 없으나, 임포트문 자체가 loud reject됨.
- **fix(IR-0)**: `pkg_types: BTreeMap<String, BTreeSet<String>>`(패키지→전 typedef명) 신설·`elaborate_package`서 모든 `ast::ModuleItem::Typedef` 수집(enum 특수분기 앞에 삽입→全 kind 커버)·"no symbol" 가드에 `&& !pkg_types[pkg].contains(sym)` 추가. 타입 임포트=nothing-bind(파서가 이미 twin 복사 완료).
- **적대 2렌즈 CONVERGE(high-confidence)**: soundness=타입 해석은 100% 파서-측·불변, fix는 elaborate 임포트문 에러 게이트만 건드림. `pkg_types`와 파서 twin 모두 실 typedef서 유래→대칭·이론적 비대칭(pkg_types엔 있으나 twin 無)도 `t x;` **parse**서 loud(silent 경로 無). differential=scalar/struct/enum/union/cross-pkg 全 iverilog 13.0 byte-일치·unknown symbol 여전히 loud(over-accept 無). regression=`package_import.rs` +3(explicit type·enum type·unknown-still-loud). IR-0(sim-ir 형상·format_version 불변)·3573 green.

#### 4.5.147 width-63 param `coerce_i64_to_width` overflow-panic fix (robustness) (2026-07-17, branch feat-width63-param-panic) ✅

§4.5.146 soundness 리뷰 발굴(F2). `localparam [62:0] P = …`(width 63)=vita debug-build **panic**("attempt to subtract with overflow" `lib.rs:coerce_i64_to_width`) vs iverilog 정상(`P=4611686018427387914`). vita가 합법 param에 crash(loud보다 나쁨).

- **원인**: `let mask = (1i64 << w) - 1` — w==63서 `1i64 << 63`=`i64::MIN`·`- 1`=subtract-overflow panic(release=wrap→우연히 correct). sign-extend `trunc - (1i64 << w)`도 w==63서 overflow. `w==0||w>=64` early-return이 w==63 미포함.
- **fix(IR-0)**: mask=`((1u64 << w) - 1) as i64`(u64는 w≤63 edge 無·w=63→i64::MAX)·sign-extend=`trunc.wrapping_sub(1i64.wrapping_shl(w))`(2's-comp 정확). w∈1..=62=byte-identical(u64 mask≡i64 mask·wrapping_sub≡sub)·w==63만 panic→정확값.
- **ALL-sites family probe**: 형제 `(1i64 << X) - 1` 5곳(RangeEnd::TypeExtreme·range bound) 全 이미 guarded(`w.min(63)`/`if width>=63 return`)·`coerce_i64_to_width`만 미가드. 
- **적대 differential**: w63 값 sweep(0·2^62·2^63-1·signed ±·bit62-set)+adjacent width(w62/w32/w8/w1) regression+array-elem/arith/array-bound 컨텍스트 live iverilog 일치·panic 無. byte-id: unsigned max=2^63-1·signed -10/-1.
- **out-of-scope**: `[63:0]` full 2^64-1 param=const_eval i64 초과→loud(pre-existing·별개). format_version 21.

#### 4.5.146 compound-const `==?`/`!=?` wildcard-literal fold (loud→supported) (2026-07-17, branch feat-const-wildeq-fold) ✅

recorded §3(compound-const `==?` fold) 착수. fresh-area sweep서 나머지 no-oracle 확정(iverilog 13.0: array reduction method·`inside` operator·`case inside` 全 거부)·casez/casex robust·`inside` operator는 지원(== 시맨틱). `localparam bit M=(P ==? 4'b1x1x)`=vita E3009 "not foldable" vs iverilog folds. const_eval_in_scope WildEq collapse가 "folded const has no x/z" 전제→x/z 패턴 리터럴 RHS서 `const_eval(rhs)`=None→loud.

- **룰(iverilog 13.0)**: `==?`/`!=?` 패턴(RHS) x/z=don't-care(§11.4.6). **SIZED** 패턴은 폭 위 bit=zero-extend·**UNSIZED** x/z 패턴(`'hx`)은 context-width로 x-fill.
- **구현(IR-0, `const_eval_in_scope` Binary arm)**: WildEq/WildNe+RHS=**Sized** IntLit면 generic `const_eval(rhs)` 前 `parse_int_literal`로 (val, mask) 추출→`(a & !mask)==(pat & !mask)`(WildNe negate). fail-closed: LHS const `a>=0`·single-word·bit-63-clear·**Sized only** 아니면 None(loud). 런타임 `lower_wildcard_eq`와 동일 수식·runtime 무관.
- **적대 2렌즈**: differential이 **자가 유발 silent-wrong 발굴**=UNSIZED x/z 패턴+LHS>32bit(`P[39:0] ==? 'hx`)서 `parse_int_literal`이 32-bit self-width로 sizing→bit[39:32]가 mask 밖=required-0→iverilog(x-fill match=1) vs vita 0. 前=loud(안전)를 fold가 silent化. **Sized-only 가드로 수정**(unsized x/z=loud). soundness(masked-compare §11.4.6·runtime 미러·guard·IR-0 CLEAN). byte-id: sized A1B0C1D0E0F1G1·40-bit sized wild·hex-xz.
- **out-of-scope(fail-closed loud)**: unsized x/z 패턴·negative-signed LHS·non-literal RHS. **pre-existing 발굴(§2)**: x/z-fill const param LHS→0(upstream·全 const op 상속)·`coerce_i64_to_width` w==63 debug-panic·narrow-typed param from comparison=`%b` wide. format_version 21.

#### 4.5.145 md-packed nested part-select WRITE `x[j][m:l]`/`arr[i][j][m:l]` (loud→supported) (2026-07-17, branch feat-mdpacked-nested-write) ✅

recorded §3(§4.5.117 write 잔여) 착수. READ는 이미 element-select+packed-flatten 조합으로 정답·WRITE만 E3009 2-path 거부(bare=`nested lvalue select`·unpacked-array=`bit-select then part-select`). iverilog 지원=②.

- **룰(iverilog 13.0)**: `x[j][m:l]=v`(bare `[1:0][7:0]`)·`arr[i][j][m:l]=v`(unpacked-array-of-packed). unpacked idx→element word·**const** packed idx→leaf base bit(`flatten_word` over packed extents)·`[m:l]`→leaf 내 bit. variable PACKED idx=iverilog 거부→vita loud(무오라클). variable UNPACKED idx=iverilog 허용→vita 계산.
- **구현(IR-0)**: `try_nested_packed_part_lval`(`elaborate`) — PartSelect lval arm서 `try_packed_part_select_lval`(outer)+hier 후·generic `lval_part_base` 前. READ path 미러: `base_off=flatten_word(pext,packed_idxs)`(prefix→leaf.size가 stride=bit offset)+`l`, width=`m-l+1`, kind=PartIdxUp, word=unpacked flatten. 엔진 `write_chunk`가 `base=word*net_w`+offset로 word+PartIdxUp 정확 소비(비-flat). fail-closed: leaf=descending zero-lsb·const in-range `[m:l]`·const packed idx·leaf 1개 남음. non-대상 disjoint(scalar `r[m:l]`·whole-element·outer part-select 불변).
- **적대 2렌즈 全 CLEAN**: differential(~45 case: shape/width/3D/4D/2D-unpacked/non-zero-base/asc-outer/edge-partsel/NBA/task/CA byte-id·**ascending OUTER dim도 정확 계산**·leaf만 gated)+soundness(ud partition·flatten prefix bit-offset·word RMW self-consistent·PartIdxUp+word=Some 엔진 확인·guard 완전성·IR-0). 회귀 테스트 4(bare·array+var-unpacked·var-packed-loud·3D).
- **fail-closed 잔여(전부 loud/no-op·§3 기록)**: ascending/non-zero-lsb leaf·genvar-index(`x[g][m:l]`=const-fold 안 됨·over-reject)·OOB `[m:l]`(iverilog truncate·was-loud pre-fix)·const-OOB packed idx=silent no-op(read path 공유·값 무손상). 3565 green·format_version 21.

#### 4.5.144 `%s` 출력 문자열 fidelity: 숫자-const NUL→space + bare string-var template (2026-07-17, branch feat-pct-s-nul) ✅

recorded §2(§4.5.119) 재그라운딩 → format 엔진 문자열 렌더 silent-wrong 2건(별도 커밋). **hexdump 필수**(NUL/space=grep/터미널이 mangle → 최초 "vita empty" 오진단은 harness 아티팩트·실제는 0x00 리터럴 방출).

- **A. `%s` of 숫자 const = NUL byte 오방출(silent-wrong)**: `$write("%s",16'h0041)`=vita `[00 41]`(NUL 리터럴) vs iverilog `[20 41]`(space)·`8'h00`=vita 빈문자열(trailing-NUL strip) vs iverilog `" "`. iverilog 룰=**모든 0x00→space·full reg width·no strip**. 원인=숫자 const literal이 `render_template` `%s` const-arm(`Expr::Const`)서 `const_string`(trailing strip+NUL 리터럴) 경유. **런타임 var 경로(`fmt_packed_chars`)는 이미 정확**(baseline byte-id). fix=const-arm 가드를 `ConstRepr::StrUtf8`로 좁힘→숫자 const fall-through=런타임과 동일 `fmt_packed_chars`(0x00→space·x/z mask→space). byte-id: NUL 각 위치·`%0s`/`%5s`/`%-5s`·80-bit wide·x/z·concat/repl/enum/param·全 sink. IR-0.
- **B. bare string VAR arg = 숫자 렌더(silent-wrong)**: `string s="hello"; $write(s)`=vita `448378203247`(byte를 %d) vs iverilog `hello`. iverilog 룰(IEEE 1364 §17.1)=**string-typed arg(literal OR var)=format template**(`%`spec가 후속 arg 소비·multi-arg·`%%`→lit·empty→무). 원인=`format_args_str` bare-arg 루프가 string LITERAL(`str_const_of_expr`)만 template 처리·string VAR은 `push_default_radix`(%d). fix=eval후 `is_str`면 `to_str_bytes`→`render_template`(§2602 is_str 경로 미러). 全 sink 공유.
- **적대 4-agent(A·B 각 2렌즈, 全 CLEAN 수렴)**: A-differential(exotic const 60+: real/enum/struct/concat/wide/param·trailing-NUL이 fix 증명)·A-soundness(ConstRepr 3변종·StrUtf8 유일 discriminator·전 sink·bounds-safe). B-differential(is_str source 全 match·**non-string wrongly-is_str 트랩 부재**=enum/bit-select/struct/`.len()` 全 숫자 유지·is_str는 `from_str_bytes` 단일소스)·soundness self(sink 전수·is_str 1곳). teeth=const≡runtime var byte-id.
- **out-of-scope(전부 pre-existing·§2 기록)**: real-const `%s`(vita packed f64 vs iverilog warn+`<%s>`)·string-LITERAL embedded-NUL(const_string NUL-리터럴+lexer octal-escape)·render_template malformed-spec(missing-arg·`%`+non-spec=vita `x`/lit vs iverilog `<…>`+warn·literal+var 공통)·high-byte 128-255 UTF-8 remangle(A/B 모두 런타임과 동일 공유·미변경). 3561 green·format_version 21.

#### 4.5.143 runtime `$clog2(real)` round-then-count (silent-63 → correct) (2026-07-17, branch feat-clog2-real) ✅

recorded §2(§4.5.122/124) 재그라운딩. `real r=100.0; $clog2(r)`=vita **63**(IEEE-754 비트패턴을 정수로 오독) vs iverilog **7**. 정수 `$clog2` 정상.

- **룰 핀(iverilog 13.0 라이브 20+ 케이스 byte-id)**: `$clog2(real)` = real→최근접 정수 **반올림(ties away from zero)**→clog2. 100.0→7·7.0→3·1.5→1·2.5→2·4.5→3·8.5→4·0.5→0·128.0→7·129.0→8·5e9→33. iverilog는 `$clog2`를 산술로 수용(형제 bit-query `$countones`/`$onehot`/`$onehot0`/`$isunknown`은 real 거부→vita 이미 elaborate E3009 loud). Rust `f64::round()`=ties-away 일치.
- **구현(IR-0, `eval.rs` `SysFuncId::Clog2`)**: `a.is_real`이면 `to_f64().round()`가 `finite && ≥0 && <2^64`면 `from_i128(r,64,false)`로 정수 Value 교체 후 기존 word-logic 통과, 아니면 `Value::xs(32)`. 비-real 경로 verbatim.
- **correct-or-loud 경계**: 음수 real은 iverilog=32(32-bit-wrap 아티팩트)이나 양수 5e9는 64-bit magnitude→단일 변환으로 양쪽 불일치. 음수/비유한/≥2^64 = **X**(honest-undefined·never confident-wrong).
- **적대 병렬 2렌즈(수렴)**: differential(양수 60+ live diff = 0 divergence·ties-away 확인)+soundness(ALL-sites·is_real 전파·경계) 둘 다 **동일 단일 silent-wrong** 지적: `u64::MAX as f64`가 2^64로 올림→`<=` 가드가 정확히 `r==2^64` 통과→`from_i128` mask→`clog2(0)=0`. `<2.0_f64.powi(64)` strict化로 fail-closed(2^64→X, 2^64−2048→64). soundness가 4번째 evaluator `native_eval.rs`(VM fast-path)도 클리어(`ConstRepr::Real` decline→커널 eval bail=funnel 단일소스).
- **ALL-sites(4)**: runtime eval(수정)·`width.rs` const-width-fold(non-Numeric decline·base==fix)·elaborate `const_eval_in_scope`(i64-only·real→None loud)·`native_eval.rs` VM(decline→bail). 전부 커버.
- **out-of-scope(loud 유지)**: const-fold `localparam=$clog2(real-lit)`=const_eval i64-only→E3009(real const-fold 전면 미지원과 동근·§3 기록). 숫자 literal `%s` leading-NUL(§4.5.119)=별개 thread(재확인). 3559 green·clippy/fmt clean·format_version 21 불변.

#### 4.5.142 `%d`-of-real 기본 필드폭 제거 + fresh-area sweep (2026-07-17, branch feat-pct-d-real-width) ✅

fresh-area 스윕(generate for/if/case·macro w/args·signedness 전파[self-det operand·concat-unsign·sign-ext·signed shift]·`$sscanf`·`$sformat`·real-math sysfunc[NaN/-inf edge 포함]) 全 iverilog 일치=코어 견고. 이후 recorded §2 中형 2건 재그라운딩→1건 shallow fix.

- **fix = `%d`-of-real width**(recorded §4.5.120): bare `%d`의 real 피연산자가 20폭(`dec_field_width(64)`)으로 pad→iverilog는 rounded 값 **무pad**(`%d` of 2.5="3"). `builtins.rs` `%d` formatter서 real+width없음=fw 0(`%0d`처럼). 정수 `%d`·명시 `%Nd`/`%0Nd`/`%-Nd` 불변(byte-id). `$display`/`$sformatf`/`$fdisplay`/`$fwrite` 공유 경로라 일괄. IR-0.
- **적대(proportionate)**: 유일 edge=`%d` of non-finite real(inf/nan)→vita `i64::MAX`(`fmt_dec` 포화) vs iverilog `inf`—**pre-existing value 동작**(width fix가 value 미변경). ROADMAP §2 기록.
- **재그라운딩 결과**: `$signed`-in-wider-sum(§4.5.111)=현재 MATCH(후속 슬라이스가 해결한 듯). mixed-sign enum(§4.5.109/110)=confirmed **multi-part**: AST `TypedefKind::Enum.base`=`Option<Range>`만(base type/signedness 미포함)·parser가 `signed` drop→respect엔 AST enrichment+parser+elaborate 필요→stays deferred(§2).
- 기존 golden(`float_format_determinism_golden`) 옛 20폭 핀→무pad 정정·+1 회귀. 3558 green·clippy/fmt clean.

#### 4.5.141 real `**` 지원(loud→supported·$pow desugar) (2026-07-17, branch feat-real-power) ✅

fresh-area probe(power operator)로 발굴한 loud→supported: vita가 real 피연산자 `**`를 E3009 "power (**) not defined on real operand in MVP"로 거부·iverilog는 지원(IEEE 1800 §11.4.9: real 피연산자→real 결과=pow(base,exp)). vita는 이미 `libm::pow`를 `$pow` sysfunc로 보유→순수 갭.

- **fix**(`elaborate` binary lowering real-operand branch): `**`를 error 대신 `SysFunc{Pow,[lhs,rhs]}`로 desugar. `$pow` eval이 양 피연산자를 real 변환(`real_arg`=integral→f64)→`2.0**3`·`2**3.0`·`r**e`·`(-2.0)**3`·`9.0**0.5`·`2.0**-2`·nested·comparison/chain 全 iverilog byte-match. 정수 `**` 불변(byte-id). **IR-0·format 불변**.
- **적대(proportionate·additive)**: 유일 divergence 2건=**전부 pre-existing**($pow/$sqrt/$itor 직접 probe로 확인): ① X-bearing integral→real: vita whole→0.0(`real_arg`) vs iverilog per-bit X→0(공통·`**` 특정 아님) ② const context `localparam real P=2.0**3`=E3009 "not foldable"—but real const 산술 전부(+*/-)uniformly loud→런타임 `**`가 다른 런타임 real op과 일치·const는 broad gap. 둘 다 ROADMAP §3.
- 기존 reject 테스트(`real_power_rejected_at_elaborate`)→`real_power_supported_via_pow`로 재목적(value 핀). 3557 green·clippy/fmt clean.

#### 4.5.140 지연 게이트/cont-assign 출력=초기 X (Z 아님) (2026-07-16, branch feat-delayed-driver-init-x) ✅

fresh-area probe(gate-primitive)로 발굴한 silent-wrong: **지연** 게이트/연속대입(`and #3(o,a,b)`·`assign #3 o=a&b`) 출력 net이 첫 지연출력 landing 전 `[0,d)` 창에서 **undriven-Z**(net default)로 읽힘 — iverilog=**X**(driven-unknown). 드라이버는 t=0부터 연결·출력 레지스터는 delay 경과까지 X 유지=구동중이지 floating 아님→Z 오류. 무지연 드라이버는 이미 정확(fixpoint가 t=0에 구동)·지연 경로만 Z.

- **fix**(`sched.rs settle_cont_assigns`): zero-delay settle **fixpoint 내부**서, 첫 출력 미landing(`last_ca_drv` None)·**net 단독 드라이버**(`delayed_sole`)인 지연 CA가 all-X 구동. fixpoint **내부** 구동이라야 downstream 무지연 CA로 X 전파(검증 `and #4(g,a,1);assign d=g`→`d==x` on [0,4)). 지연 계산값은 now+d 스케줄 불변·landing 후(`last_ca_drv` Some) X-drive 영구 skip(None→Some 단방향).
- **적대 soundness가 실회귀 발굴→수정**: 처음 `!ca_md[ci]` 가드는 **지연 드라이버에 vacuous**(`ca_md` 멤버십=`delay.is_none()` 필요)·E3001은 dynamic/array-elem select **blind spot**→지연 whole + 무지연 `assign y[i]=b` 겹침 시 X-drive가 매 delta 충돌→1M-delta spin+가짜 "oscillation" fatal(내가 낸 회귀). **`delayed_sole`**(per-net 드라이버 카운트, net 공유 시 pre-fix Z 유지=byte-id)로 robust 수정. differential=~20 variant(width·expr·gate·tristate Z-out·rise/fall·inertial supersede·partial-x·전파) 全 iverilog 일치·negative control 불변.
- 교훈: **가드가 맞아 보여도 대상 subset(지연)에 실제 적용되는지 검증**(vacuous guard)·기존 "보호"(E3001)도 blind spot 가능→robust는 per-net 카운트. IR-0·format 불변. 기존 테스트 1건(Z를 "impl-defined"로 핀했던 것) X로 정정+회귀 1. 3557 green.

#### 4.5.139 VCD VALUE 검증(정확)+회귀 가드·fresh-area sweep (2026-07-16, branch feat-vcd-value-verify) ✅

§4.5.138($var range) 후속 VCD probe iteration. **VCD value-change 덤프가 정확함을 검증**: left-extend + last-write-wins decoder로 vita decoded waveform이 iverilog 13.0과 일치(x/z·real·wide·`$readmemh`/`$readmemb`). 발굴한 vita↔iverilog VCD 차이는 **전부 encoding/cosmetic·decode 동일→silent-wrong 아님**:
- value 미압축(vita `b10xz01xz`/`bzzzz` full vs iverilog leading-redundant strip `bx`/`bz`/`b0`)·decode 동일·golden churn 큼→defer.
- t=0 초기덤프 구조(vita `$dumpvars`에 pre-assign X + `#0` change vs iverilog settled)·final 동일.
- var-type keyword(logic 절차구동=vita `wire` vs iverilog `reg`·usage 의존 non-trivial·`int`=`reg` vs `integer`)·real size `64`vs`1`. 전부 ROADMAP §3.
- 함께 probe·iverilog 일치(코어 견고): format spec(%e/%g/%f/%o/%h/%d/%b·%c high-byte=기존 DEEP-defer)·casez/casex·reduction on x/z·`$readmemh`/`$readmemb`(comment·@addr·range·multi-value·X-fill).
- ship: 회귀 가드 `vcd_dumps_xz_and_real_values`(x/z·real value-line 핀·향후 compression 채택=의식적 golden update). 3556 green·clippy/fmt clean·format 21 불변. **코드 무변**(검증+가드+기록 iteration·§1 valid).

#### 4.5.138 VCD `$var` `[msb:lsb]` bit-range reference (2026-07-16, branch feat-vcd-varrange) ✅

fresh-area VCD probe(NEXT §1)로 발굴한 silent-wrong: vita가 벡터 `$var`에 bit-range 미방출(`$var reg 4 ! cnt $end` vs iverilog `cnt [3:0]`). IEEE 1364-2005 §18.2.3.2 `<reference> ::= id [ [msb:lsb] ]` — range 없으면 뷰어가 ascending `[0:3]`↔descending `[3:0]`을 구분 못하고(`reg 4` 동일) non-zero-base `[7:4]` base 상실 → **비트 라벨 오표시(silent)**. 값은 정확·메타데이터만 결손.

- **fix**(`state.rs` `vcd_var_reference` + `builtins.rs` 4 declare_var site): 벡터(width>1 **OR** 1-bit non-zero index `[5:5]`)=declared `[msb:lsb]`·true scalar/`[0:0]`/real·realtime=range 無. **Pure IR-0**(엔진-local·NetVar.msb/lsb는 frozen SimIr서 read만·format 불변). 배열 요소도 일괄(hand-IEEE: iverilog은 memory 미덤프).
- **적대 2-lens 둘 다 실결함 발굴→수정**: (a) differential=1-bit non-zero-index(`[5:5]`) 초기 `width>1` gate서 누락→gate 보정 (b) soundness=packed-multi-dim(`[0:3][7:0]`)서 elaborate가 msb=width-1만 갱신·lsb stale→초기 fix가 inconsistent `[31:3]`(span≠size) 방출=**내가 낸 회귀**→span==width일 때만 declared 사용·아니면 flat `[width-1:0]` fallback(iverilog은 packed 전부 `[31:0]`). 최종 differential CLEAN(desc/asc/non-zero/single-bit/[0:0]/integer/time/packed struct·2D 전부 iverilog 일치).
- golden VCD 테스트(rangeless 인코딩분) 갱신: `vcd_golden_byte_exact`·`vcd_dumpvars_declares_memory_array`·`vcd_array.rs` 4. +1 회귀. 게이트 3555 green·clippy/fmt clean.
- **follow-on(cosmetic·ROADMAP §3)**: real size `64`vs`1`·packed=`wire`vs`reg`·`parameter` 미덤프·elaborate NetVar.lsb stale(VCD서 우회·근본 미수정).

#### 4.5.137 `$fflush` = 인식된 no-op (오진단 warning 제거) (2026-07-16, branch feat-fflush-noop) ✅

NEXT §2 loud→supported. vita가 `$fflush`를 "unsupported system task skipped (v2)" W3056로 경고+drop했으나 iverilog는 무경고 실행. **`$fflush`=vita서 provable no-op**: 열린 파일=raw UNBUFFERED `std::fs::File`(각 `$fwrite`가 `write_all`로 OS 직행)·`$display`/STDOUT=결정성 sink capture → flush할 대상 없음. write fd를 닫지 않은 same-sim reopen-read가 직전 write 전부 봄(검증 `rb=[data12]`)=drop해도 무손실·출력이 이미 iverilog 일치. 경고가 오진단(없는 degradation 암시).

- **fix**(`elaborate::lower_systask`): `map_systask` fallback 前 `"$fflush"` 인식→silent accept-and-drop(Stmt·경고 無). **Pure IR-0**(Stmt 미방출·SysTaskId 변종 無·엔진 무변·format 불변). 全 form(bare·`()`·`(fd)`·`(0)`=flush-all·MCD·preopened STDOUT fd) no-op.
- **적대(proportionate·behavior-preserving)**: 全 form differential + `$write`-interleaving + read-back 모두 byte-match·full suite byte-identical(stderr 경고만 소멸·stdout 무변 3554 green). 유일 edge=side-effecting arg `$fflush($fopen())` drop=기존 skip 경로와 동일(무회귀·pathological).
- **defer(ROADMAP §3)**: `$fstrobe`/`$fmonitor`=W3056 skip으로 **파일출력 silent drop**·지원=직렬화 마커(SysTaskId 변종 or staged 사이드카)→**format bump**=전용 슬라이스. **교훈**: 부작용 없는 no-op systask=elaborate서 return None(Stmt/사이드카/SysTaskId 변종 불필요·format 불변)·엔진효과 있는 건 format bump.
- +1 회귀(`file_io.rs`). clippy/fmt clean·format 21 불변.

#### 4.5.136 neg-range-bound 진단 정직화 + grounding sweep (2026-07-16, branch feat-negbound-diag) ✅

fresh-area probe(NEXT §1) iteration: classic-Verilog 스윕(bit-vector fn·signed div/mod/shift·indexed part-select·div-by-0·zero-replication·signed concat·ternary sign·`$sformatf`·array-query fn `$left/$right/$high/$low/$size/$increment/$bits/$dimensions`)에서 **코어 견고 재확인**(전부 iverilog byte-match). `%m`은 내용 일치·모듈간 initial-block 순서만 상이(impl-defined t0).

- **유일 발굴 = 음수 range bound**(`logic [3:-2]`=6bit per iverilog `|msb-lsb|+1`): net/multi-packed inner = W3056 warn+clamp-1(**whole-value 손상**)·unpacked `[-1:2]` = E4002. 전부 **non-silent**(§2: W-degrade=비-silent)→ correct-or-loud상 loud→supported gap, cardinal-sin 아님. **DEEP**: u32 dbase/offset + frozen `ir::NetVar.msb/lsb`가 음수 base 불가 → format bump 또는 signed 사이드카. packed-struct-member 경로는 이미 whole-correct(flat offset)+sub-select-loud 처리(`struct_field_select.rs` 선례)—plain-net이 이를 미러해야. ROADMAP §3 defer.
- **이번 수정(behavior 불변)**: W3056 message가 리터럴 `[3:-2]`를 "parameterized range underflowed (param value 0?)"로 **오진단**. lsb<0(음수 low bound)↔ msb<0·lsb≥0(`[W-1:0]`-with-W==0 param underflow) 2-shape 분리 진단. 두 shape 모두 warn+clamp-1 유지 → 非음수 디자인 byte-identical(full suite 불변). param-underflow graceful(elaborate `v3_12`) 보존.
- **교훈**: 한 경로(plain-net) 버그가 형제 경로(struct-member)엔 이미 정답 처리일 수 있음 → defer 前 형제 grep·미러(LOOPROMPT §1 병합). +2 회귀(`nonconst_width_p0ncw.rs`). 게이트 3553 green·clippy/fmt clean·format 21 불변(IR-0).

#### 4.5.135 `$time`/`$stime` 반올림 + `$stime`-in-`$monitor` 제외 (2026-07-16, branch feat-time-round) ✅

fresh-area probe(NEXT §1)로 발굴한 신규 silent-wrong 2건을 같은 반복 내 수정(§S④). 오라클 = iverilog 13.0 라이브.

- **① `$time`/`$stime` 절사→반올림** (`crates/sim-engine/src/eval.rs`): 두 시스템함수가 모듈 단위 시각을 `self.now / m`(floor)로 절사 — IEEE 1800-2017 §20.3.1 "scaled to the time unit … and rounded to an integer value" 위반. `#1.5`@1ns/1ps → vita `$time`=1, iverilog=2. `$realtime`는 원래 정확(소수 유지). fix = `self.now.saturating_add(m/2)/m`(round-half-up = time≥0이라 round-half-away-from-zero, iverilog 일치: 1.5→2·2.5→3·4.6→5). `$stime` = 반올림 후 low-32 mask. VCD·`#delay`는 raw tick 유지(무영향). all-sites: compute는 이 2곳뿐(native_eval는 인터프리터 bail·sched.rs는 분류만). m은 항상 10^k≥1(m/2 정확·m=1→0 무회귀). 옛 절사를 인코딩한 기존 테스트 3건(end_to_end.rs) + SPEC doc-08(절사 서술·stale "$stime 미구현" 표기) iverilog 값으로 정정.
- **② `$stime`-in-`$monitor` 매-스텝 re-fire** (`crates/sim-engine/src/sched.rs`, ①의 적대 differential 리뷰서 발굴): monitor change-detection의 `is_direct_time` 제외집합이 `$time`/`$realtime`만 필터(IEEE §21.2.3: 시간함수는 렌더하되 change-trigger 제외)·`$stime` 누락 → `$monitor("s=%0d x=%b",$stime,x)`가 매 1ns 스텝 재출력(iverilog 2줄 vs vita 16줄). fix = `matches!`에 `Stime` 추가(IEEE 3종 {time,stime,realtime} 완성). ①과 독립(정수-ns repro서 반올림=identity로 재현). `$stime` 값 렌더는 정상 유지.
- **적대 2-lens**: differential(~140 관측·9 timescale/precision·half-boundary·32-bit wrap·2^53/2^60 magnitude 전부 byte-match → half-up 확정) + soundness(all-sites·VCD/scheduler raw-tick·M=10^k·saturating_add·round-then-mask 전부 SOUND). teeth = iverilog 라이브 + 신규 회귀 3(round-half-up boundary·wide-ratio 1s/1ps M=10^12·`$stime`-monitor 제외). 게이트 3551 green·clippy/fmt clean·format_version 21 불변(IR-0).
- **follow-on(no-oracle)**: `$random`/`$urandom` 직접 `$monitor` 인자 re-fire — iverilog가 non-simple 인자 거부("SORRY")라 오라클 부재. ROADMAP §2 기록.

*(2026-07-16 이관 이후 완료분이 여기에 추가된다. §4.5.134까지는 아래 스냅샷에 있음.)*

---

---

## 이관 스냅샷 (구 §0~§7) → 별도 파일

2026-07-16 이관 시점의 **구 ROADMAP 원문**(§0 핵심 발견 · §1 트랙별 과제 · §2 우선순위 ·
§3 교훈 · §4 Phase-2+ 퓨처 플랜 · §5 하드닝 백로그 · §6 외부 호환성 리포트 · §7 G2 OBS
트랙)은 **[ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md)** 로 옮겼다
(2026-07-28 — 이 파일이 995 KB 였고 그 3분의 2가 갱신되지 않는 동결 스냅샷이었다).
**§번호는 보존**되어 있고 내용은 무삭제다.

## §2 「다음 착수 순서」 표 — 해소된 행 (2026-09-02 이관)

> ROADMAP §2 의 착수표에서 해소된 행 **21개**를 원문 그대로 옮긴 것이다. ROADMAP 의 운용 규칙은
> *"완료 항목은 즉시 제거하고 ARCHIVE 로"* 인데 취소선 행이 21,824 바이트 남아 있었다 — 그 잔류가
> 예전에 그 파일을 106 KB 로 불린 원인이고, 같은 일이 다시 벌어지고 있었다.
> **행 번호는 이관 당시 그대로 보존한다** — 다른 문서·테스트 주석의 "§2 표 N" 참조는 여기서 찾는다.

> | ~~**1**~~ | ✅ **RESOLVED §4.5.355 — 계층 타깃의 fill 폭**(2026-08-21) | 큐엔 **셋**으로 적혀 있었고 census 가 **일곱**(+ 적대 프로브가 `'x`/`'z`·task 스코프까지 **열**)임을 보였다. ⭐ 고침은 새 폭 규칙이 아니라 **같은 `ir_lvalue_width` 를 chunk 가 진짜가 된 뒤 한 번 더 묻는 것**. 상세=ARCHIVE §4.5.355 |
> | ~~**1e**~~ | ✅ **RESOLVED §4.5.360 — 선행조건이 "스코프 스냅샷"이 아니라 문자열 하나였다**(2026-08-22) | §4.5.357 이 남긴 4칸(`u.a = c ? '1 : 1'b0`)의 선행조건은 *"resolve 에서 쓸 수 있는 lowering 스코프"* 였는데, 재보니 vita 의 이름 해석은 **`cur_prefix` 에서 바깥으로 걷는 walk + elaboration 내내 살아 있는 FQ 테이블**이다 ⇒ 스냅샷 = **prefix 문자열 하나** · ⚠️ 재-lower 는 **읽기 전용 검증 뒤에만**(미해결 이름은 진단을 이미 뱉어서 되돌릴 수 없다 — 틀린 값이 거짓 loud 로 바뀌면 회귀) · 70칸 residue **4 → 0** · 상세=ARCHIVE §4.5.360 |
> | ~~**1d**~~ | ✅ **CLOSED §4.5.361 — 셋 다 no-op 이고, 그게 측정된 결론이다**(2026-08-22) | ⭐⭐ **분열은 "판정 대기"가 아니라 "어느 오라클이 그 자리에서 오라클이 아닌지"의 문제였다.** ⓐ `s <= 1'b1` — iverilog 가 **여기서 오라클이 아니다**: `s<"ab"`·`s<"aa"`·`s<"zz"` 를 **전부 1** 이라 답한다(폭·부호·변환이 하나도 없는 string↔string 비교인데 셋이 동시에 참일 수 없다) ⇒ 그 **1** 은 정보가 없다. vita=verilator, 전 칸 일치 ⇒ **고칠 것 없음** · ⓑ `'1 ** r` — ⚠️⚠️ **고쳤다가 측정이 기각했다**: 근거는 vita 자신의 불일치(`'1 ** r`=480 인데 같은 all-ones 를 fill 없이 쓴 `(4'd15+4'd1) ** r`=0)였고 그 불일치는 진짜지만, 288칸 PRE-3-way 가 iverilog 일치 **267 → 247**(옮겨간 35칸이 **전부 멀어짐**)을 냈다 ⇒ **되돌림**(differential > soundness 규칙이 작동한 자리) · ⭐ 그리고 진단이 뒤집혔다 — iverilog 는 `(4'd15+4'd1) ** r` 을 **1024** 로 읽는다 = 대입 문맥이 **밑수에 닿는다** ⇒ vita 의 불일치는 fill 경로가 문맥을 **더 줘서**가 아니라 non-fill 경로가 **덜 줘서**다 ⇒ **§2-1f 로 이관** · ⓒ `%` on real — 두 툴이 실행하되 **서로 다른 답**을 낸다(iverilog fmod 1.5 / verilator 0) = 정의된 의미가 아니라 미정의 구석의 서명. 불법 코드엔 loud 가 최상단 ⇒ **§3 갭이 아니다** · 판정 4건 전부 `oracle_split_rulings.rs` 로 핀 고정(미래 스윕이 "고쳐서" silent-wrong 을 만들지 못하게) |
> | ~~**1f**~~ | ✅ **RESOLVED §4.5.362 — real 지수는 밑수의 문맥을 회수한다**(2026-08-22) | ⚠️⚠️ **§4.5.361 이 이 행에 적어 둔 진단이 거꾸로였다**(그 슬라이스가 iverilog 한 칸만 보고 뒤집었다) — 결함은 non-fill 경로가 덜 주는 게 아니라 **fill 경로가 대입 폭을 준다**는 것 · ⭐⭐ 실격 질문은 하나였다: **대입 폭이 없는 곳에 같은 식을 보내라** — `real x = ('1+4'h0) ** r` 는 **셋 다 871.4213**, §11.4.9 가 연산자의 뜻으로 정의한 `$pow(('1+4'h0), r)` 도 **셋 다 871.4213** ⇒ iverilog 의 480 은 **목적지가 밑수로 새어 든 것**이고, 자기가 15 라고 답한 밑수의 기준이 될 수 없다 · ⭐ 규칙은 이미 `+`·`-`·`*`·`/` 가 지키고 있었다(같은 네 밑수가 `+ r` 에서 **4·18·2·18** = 형제 폭 반영) — `**` 만 한 값(480)으로 뭉갰다 · 게이트 = **연산자형 vs `$pow` 형** 192쌍 **115 → 192** · ⚠️ 순수-정수 `**` 384칸 **바이트 동일**(Table 11-21 의 나머지 절반: 정수 지수면 밑수는 여전히 문맥-결정) · ⚠️ 적대 리뷰가 **이웃 가드를 그대로 베낀 것**을 잡았다(`!expr_contains_fill(rhs)` — fill 은 real 이 못 되지만 **fill 을 품은 식은 real 이 된다**: `('1+4'h0) ** (r + '1)`) · 상세=ARCHIVE §4.5.362 |
> | ~~**🆕 A**~~ | ✅ **RESOLVED 2026-08-29 — 그리고 그게 더 큰 규칙 하나를 드러냈다** | §11.8.1 상 **한쪽이 unsigned 면 비교 전체가 unsigned** 이고 그 부호가 양쪽 피연산자로 **내려간다** ⇒ 서명된 피연산자는 zero-extend 되고 그 안의 `>>>` 는 **논리 시프트로 강등**된다. vita 는 부호를 안 내려보낸다 · **최소 재현(2 연산자)**: `reg signed [7:0] b = 8'shB3; (b >>> 4) > 8'd100` ⇒ **vita 1 / iverilog 0 / verilator 0**(exit 0 · 진단 0) · ⭐ **분리됐다**: 같은 식의 `> 8'sd100`(양쪽 signed) 은 **셋 다 0** 이고 `$signed(b >>> 4)` 는 **셋 다 −5** ⇒ 시프트도 부호도 아니라 **비교가 부호를 전파하는 방향** 하나 · 발견 = §4.5.391 의 differential 렌즈 fuzz(깊은 트리 1,130설계 중 5건이 같은 가족) · ⚠️ **wprog 와 무관**(native·vm·interp 가 전부 같은 답) ⇒ 표적은 `eval_binary_ctx` 의 비교 arm 이 `pair_signed` 를 **읽기만 하고 아래로 안 주는** 것 · ⚠️ 착수 전 census 필수 — `<`·`<=`·`>`·`>=`·`==`·`!=`·`===`·`!==` 여덟 연산자 × 부호 조합 넷 × `>>>`·`$signed`·단항 `-` 같은 부호민감 피연산자 | · ✅ **고침 = `AShr` arm 이 `eff_signed`(= self && ctx)를 쓴다**(`eval_core` + tier-2 `native_eval` **복사본 둘** · `wprog` 는 균일-부호 게이트가 이 가족을 먼저 거절해 무관) · 303칸 census **70 → 0 · 오라클 분열 0** · ⚠️⚠️ **그런데 이 수정이 만든 correct→silent-wrong 을 적대 리뷰가 BLOCKING 으로 잡았다**: 프레임 인자 copy-in 이 **formal 의 부호**를 문맥 부호로 넘긴다(§13.5.1 은 bind 를 대입으로 만들고 §11.8.3 은 **폭만** 빌려주며 §11.8.1 은 부호가 좌변에 의존하지 않는다고 한다 — 실측: iverilog 에서 formal 의 선언 부호는 **효과 0**, `fu16(8'shf7)`==`fs16(8'shf7)`==`fff7`). 옛 `AShr` 이 `ctx_signed` 를 **무시해서** 우연히 면역이었고, 고치자 `au(b>>>2)` 가 **4294967276 → 44** · ⭐⭐ **퍼널이 셋이고 리뷰 라운드마다 하나씩 나왔다** — 둘을 고친 뒤 soundness 가 12자리 census 로 *"둘이면 충분"* 이라 했는데 **differential 이 세 번째를 측정으로 찾았다**(`task_frames.rs` 의 `Terminator::Call` — **프레임 본문 안에서** 부른 **비-suspendable** 태스크. 콜리에 `$display` 하나만 있어도 이미 고친 경로로 라우팅된다) ⇒ **자리를 세는 census 는 콜리의 성질이 고르는 자리를 놓친다** · 라운드 3 재측정 **~9,200칸 · 회귀 0 · 백엔드 분열 0 · 코퍼스 바이트 동일** · 곁수확 = `inline_formal_bind.rs` 의 문서화된 갭(`ff(8'shf7)` `000000f7` → **`0000fff7`**)과 그 옆 pre-existing 셋도 닫혔다 · ⭐ 부산물 판정: signed **default 인자**에서 iverilog 가 **자기모순**(같은 default·같은 formal 인데 함수 `000000b3` / 태스크 `ffffffb3` · 자기 plain-assign 쌍둥이는 태스크 쪽과 일치) ⇒ 여기서 iverilog 는 오라클이 아니다 · vita 는 verilator 와 일치하고 **내부적으로 균일**해졌다 ⇒ `oracle_split_rulings.rs` 에 핀 |
> | ~~**2**~~ | ✅ **RESOLVED §4.5.358 — 다만 큐에 적힌 셋이 아니었다**(2026-08-22) | ⭐ 재-census 결과 §4.5.349 가 남겼다던 **셋은 이미 전부 두 오라클과 일치**했다(이후 슬라이스가 닫았다). 대신 33칸 census 가 **새 칸 하나**를 냈고 그게 60칸 → 12칸 클래스로 자랐다: **real 타깃이 폭을 빌려줘 unsigned 피연산자가 64비트에서 평가**된다(`byte unsigned b=8; real r; r=-b;` 두 오라클 **248** / vita **1.84467e+19**) · 상세=ARCHIVE §4.5.358 |
> | ~~**2b**~~ | ✅ **CLOSED §4.5.361 — 분열이 부호가 아니라 "컨테이너 절단"이었다**(2026-08-22) | ⭐⭐ 판정을 **부호 축에서 물으면 영영 안 풀린다**: iverilog 는 `$itor(64'h1_0000_0008)` 을 **unsigned 든 signed `longint` 든 똑같이 8** 로 읽는다 ⇒ 부호 해석이 아니라 **32비트 컨테이너로 절단**이다. 채택하면 vita 가 지금 맞는 **2³¹ 이상 전 구간**이 correct-support → silent-wrong 으로 **내려간다** ⇒ 사다리 규칙상 금지 ⇒ **고칠 것 없음** · ⚠️ 곁가지로 **iverilog 결함 ⑥** 확정(오라클 주의 목록에 추가) · ⚠️ 구조적 봉인도 같이 기록: `real'(x)` 가 **같은 `SysFuncId::Itor` 로 lowering** 되고 그건 동결 sim-ir 타입이라 이 축을 건드리면 **format_version 29→30** 이다 · anti-절단 핀 = `oracle_split_rulings.rs::itor_keeps_the_value_instead_of_truncating_to_thirty_two_bits` |
> | ~~**3**~~ | ✅ **RESOLVED §4.5.359 — 음수 bound 의 남은 선언 스코프**(2026-08-22) | 큐엔 **셋**(포트·서브프로그램 지역·클래스 속성)인데 census 는 **여섯**이었다(+함수 반환 타입 · 함수 형식인자 · **static task 지역**) · ⭐ 고침은 여섯 복사가 아니라 **한 함수**(`record_declared_bounds_for` = 모듈 스코프에 인라인돼 있던 20줄) · 78칸(13 스코프 × 6 range) **전부 iverilog 일치** · 상세=ARCHIVE §4.5.359 |
> | ~~**4**~~ | ✅ **RESOLVED 2026-09-01** | `$itor(3.9)` was the double's bit pattern (4.615964438073390e18) and is now 4.0 = iverilog's answer: §20.5 makes the argument integral, so it is rounded first (§6.24.1, half away from zero). ⚠️ Fixed on the VALUE in the ONE evaluator, not by inserting a cast at elaborate — review measured that an AST-level real-ness gate is blind to a real-returning FRAME function and to a hierarchical real net, that a fixed-width integer intermediate FLIPS THE SIGN past 2^63 (iverilog keeps `$itor(1e30)` as 1e30, so its model is not a round-trip through any width), and that the cast desugars into `$floor`/`$rtoi`/`$signed` nodes `--obs-procs` then reports as calls the source never wrote. `real'()` shares the id and is unmoved in all eight spellings.
> | ~~**6**~~ | ✅ **RESOLVED 2026-09-01 (ladder move: silent-wrong → loud)** | The inline fold decided "return value or local" from the LHS spelling alone, so a write to a module net became a substitution binding and vanished. It now carries the set of names the body OWNS (formals + all decls + the function name) and refuses anything else, naming the target and the reason — the same answer the `automatic` spelling already gave. ⚠️ **Not the destination**: performing the write means emitting a statement from an expression-position fold (the hoisting machinery), and the measured cost of the move is that a design whose foreign write is never OBSERVED was accidentally correct and is now loud. 🆕 That follow-on is the remaining half of this row.
> | ~~**8**~~ | ✅ **RESOLVED 2026-09-01** | §14.3 makes a clocking input read-only and the LVALUE funnel enforced it (`cb.sig = …` is `E3009`), but a `$readmem*` memory ARGUMENT is resolved through the ordinary READ path, so it wrote the holding net at exit 0 while the real variable kept its contents. One rule, two funnels, two answers. `deny_clocking_input_arg` now asks from the PATH, like the lvalue twin, at the `$readmem*` memory position. ⚠️ It must ask from the path and NOT from the resolved net: a clocking input of an unpacked ARRAY has only a scalar holding net, so a net-keyed first version fired for the scalar spelling and missed the array one that motivated the row. ⚠️ **No oracle** (iverilog cannot parse clocking, verilator rejects the shape) ⇒ hand-IEEE, and the CONTROLS carry the weight: an ordinary array still loads and `$writemem*`, which only READS, still takes a clocking input.
> | ~~**8b**~~ | ✅ **RESOLVED 2026-09-01 — one funnel, and it turned out to be nine tasks and two lowering positions** | `deny_const_param_write` is now `deny_readonly_write` and asks the clocking question first, so a parameter and a clocking INPUT are refused in ONE place at all 21 write positions; `$sformat`/`$swrite` did not go through it at all and now do. `$readmem*`'s memory argument keeps a PATH-keyed twin, because a clocking input of an unpacked ARRAY has only a scalar holding net and never produces a net id to test. ⚠️ **Review measured that the census was short by three**: `$cast`'s destination — the guard `systask.rs` cites as the model its own check *mirrors* — and the SEED of `$random` and `$dist_*`, which the engine advances, reached no funnel at all. ⚠️⚠️ **And the first closure was defeated by one keyword**: a destination inside an `automatic` task, or written hierarchically, is lowered before its name exists, so the funnel saw `POISON_NET` and answered nothing — `$readmem*` was loud there only because `hier_mem_args` happens to carry the same fact for another reason. `hier_write_args` + `deny_readonly_write_at` record the eid and `resolve_deferred_hier` asks once the net exists. **Fifteen spellings loud, PRE == POST on 240 no-clocking cells and 26 parameter cells, examples byte-identical.** ⚠️ Residues below: rows 22, 23, 24.
> | ~~9~~ | ✅ **RESOLVED §4.5.384 — a constant-context select of an ENUM LABEL** (2026-08-25 · 59칸 **FIXED 39 · REGRESSION 0 · MOVED 0**) | ⭐ 라벨의 폭도 **선언된 사실**이다(enum 의 base 타입) — 없던 건 `param_range` 항목뿐이고, `enum_base_range` 를 라벨을 바인딩하는 **세 자리**(모듈 본문·패키지·함수/태스크 본문)가 `param_meta` 옆에 기록한다 · ⭐⭐ §4.5.373 의 두 번째 선행조건(*그 폭에서 값이 canonical*)은 **다른 이유로 쓰인 게이트가 이미 세우고 있었다** — `enum_label_range.rs`(§6.19 위반 = loud E2002) · ⚠️⚠️ **비-0 LSB base 는 오라클이 갈린다**(`enum logic [39:8]` 에서 `EA[15:8]` = iverilog **171** / verilator **52**)고 ascending 은 iverilog 가 선언 자체를 거부 ⇒ **둘 다 declines**(PRE 그대로) · 상세=ARCHIVE §4.5.384 |
> | ~~**11**~~ | ✅ **RESOLVED 2026-09-01 — a label is a const declaration, so it binds in DECLARATION ORDER** | The (3c) block is now `bind_enum_labels_of`, called from the (3b) decl-order walk as a quiet pre-pass and again afterwards. ⭐ The row was narrower than the defect: a param→label→param CHAIN (`localparam A = 3; typedef enum {L = A + 1} e_t; localparam B = L + 1;`) is `A=3 L=4 B=5` in both oracles and was loud on `B`, and a label was equally invisible to a later WIDTH, a later `generate if`, and a later typedef's label. The forward reference the queue line warned about is untouched — the after-the-walk pass still folds it, and it stays an ORACLE SPLIT. ⚠️⚠️ **Two bindings are sound only if they AGREE, and the first version shipped without checking**: two review lenses each measured the same two mechanisms independently and a third only one saw, all with one shape — a consumer that folds BETWEEN the passes keeps pass 1's answer while everything after keeps pass 2's, so one label read two values in one run at exit 0. The gate declines the WHOLE typedef when any label's value or the base's width is not yet a fact; `enum_label_prepass` makes pass 2 VERIFY rather than overwrite, which covers the third (a name resolving to a different declaration in each pass) and whatever the fourth is. Details = CHANGELOG.
> | ~~**12**~~ | ✅ **RESOLVED 2026-09-01** | The parser's §6.19 check folds base bounds with a literal-only fold, so `enum logic [W-1:0]` was fail-open and vita exited 0 on designs both oracles reject. It now has an ELABORATE twin, where the bound is folded. ⚠️ Three things review corrected in it, all closed: the bounds arithmetic used `1i128 << w`, which WRAPS at 128 (a legal 128-bit parameterised base was rejected, and a debug build PANICKED from w=127) — capped at the widths where an `i64` label can actually overflow; a hand-mirror of the parser's fold used to decide what to skip, and it drifted at once, leaving `[(3):0]` and `[8'd7:0]` unchecked by BOTH — deleted, because a parser error halts the pipeline so there is nothing to skip; and the range reported is the UNION of the signed and unsigned readings, because the two folds disagree about a based literal (`8'shFF` is 255 to one and −1 to the other). ⚠️ **Residual**: a negative label on an UNSIGNED base is loud when the bounds are literals and accepted when they are not — closing that needs the sign PROVENANCE row 14 stops at. Also: the earlier symptom (*"one label, two answers, `$bits` 43828 vs 52"*) was already gone before this — §4.5.384 closed it.
> | ~~**13**~~ | ✅ **RESOLVED 2026-09-01 — a label carries its base's DECLARED sign, and its value is canonical at the declared width** | The `|| v < 0` clause (§4.5.154) is gone from all THREE label binders and the value is stored `const_mask`ed. A 64-cell census (8 declaration forms × 9 consumers, module + package) moved exactly the wrong cells; review measured three more fixes the change did not claim (a net width 4→254, `$size` 3→255, a `generate if` arm). ⚠️ Two BLOCKING defects came out of it and are closed: the package binder took its auto-increment from the RAW value so the same enum was loud at module scope and silent in a package, and the mask newly routed a negative label into a package-internal concat that folds at the wrong width — the root being that a package makes its PARAMETERS' widths live for its own body and never did the same for its LABELS. Details = CHANGELOG.
> | ~~**18**~~ | ✅ **RESOLVED 2026-09-01** | The collector now computes the override EXPRESSION's signedness with the SAME helper the `#()` and `-G` channels use, and threads it through `DefparamOverride` into `ResolvedOverride::signed`, which was `None` — read by `bind_one_param` as *"stay on the route you took before"*, i.e. zero-extension. `defparam u.K = -32'sd7;` onto a `parameter logic [127:0]` was `0000000000000000fffffffffffffff9` and is now all ones, matching both oracles; `32'd7`, `64'hFFFF_FFFF_FFFF_FFFF` and `-(64'sd1)` are unchanged and match. ⚠️ `32'd0 - 32'd1` is row 17's oracle split and is deliberately untouched.
> | ~~**21**~~ | ✅ **RESOLVED 2026-09-01 — the declared width is an INPUT to the wide fold, not a resize after it** | `fold_self_bits` is a wrapper over `fold_bits_at(e, ctx, name)`: a context-determined arm computes at `max(operand widths, ctx)` and recurses with `ctx`, a self-determined one recurses with 0, and the two positions the LRM carves out — a shift's RIGHT operand (§11.4.10) and `**`'s exponent (Table 11-21) — stay self-determined, which is the rule row 27 established for the ≤64-bit walk. A size cast is its OWN context, so `ctx` stops there. **312 cells across two censuses: 159 FIXED · 0 REGRESSION · 0 silent-swap · 0 still disagreeing with iverilog.** `localparam logic [127:0] C = ~32'd0;` was `00…00ffffffffffffffff` at exit 0 and is 128 ones; `3 ** 41` at 128 bits is the `…1fa2a1cf67b5fb863` both oracles print. ⭐⭐ **Four separate pinned tests went loud → correct, and each had named THIS prerequisite in its own docstring** (*"until the walk can carry a context width"* · *"the wide fold cannot widen a context"* ×2 · *"this walk cannot carry a context width, so it declines"*) — `const_pow_exponent_domain.rs` no longer has a `loud` helper at all. ⚠️⚠️ **BLOCKING three, all mine, 246 cells, and both lenses found the first two independently**: ⓐ a leading `(` routed past the whole slice — `wide_param_const_in_scope` matched `Paren` at the TOP and sent it to `fold_init`, which folds at ctx 0 and resizes; PRE never reached it because the backstop declined, and threading `ctx` INTO the backstop stopped the decline firing, so the stale narrow route won (`(8'hFF + 8'hFF)` → `…00fe` vs `…01fe`, and the slice's own headline cell got WORSE through a paren) ⇒ peel parens before choosing the route ⓑ `widen_to`/`bp_operands_at` extended each operand in its OWN sign where §11.8.2 decides the EXPRESSION's sign first and coerces the operand to it — and **vita's own runtime already did it right**, so the constant domain was contradicting the runtime ⓒ widening an x/z value replicated the unknown MSB, turning a loud decline into a silent 120-bit `x` ⇒ decline instead (the underlying `resize_bits` x-extension is pre-existing). ⭐ Review also showed the docstring's *"the classification is not written here"* was FALSE — the arms hand-match, so this is a sixth reading of `binop_result_is_context_determined`, and the drift was already real: `BitXnor` was missing, so `96'hF0 ~^ 96'h0F` was wrong in PRE and POST. Added, and the docstring now says what the code does. ⚠️ `resize_bits` gained an identity early-return: without it a 300-deep `+` chain at `MAX_NET_WIDTH` cost 3.0 s of elaborate. ⚠️ The OVERRIDE channel is deliberately untouched — measured unmoved across five spellings — because most of row 16's cells are row 17's oracle split.
> | ~~22~~ | 🔄 **RE-GROUNDED 2026-09-01 — MOVED TO §3 (missing-loud, not silent-wrong) and it is 126 cells, not 3 spellings** | Every cell is illegal SV (§3.13: module items share one name space), so nothing correct is being got wrong — the ladder move is silent-ACCEPT → loud, which belongs in §3. **Full 12×12 ordered matrix at module scope = 132 cells: vita accepts 126, iverilog and verilator reject 132/132**, and a 132-cell CONTROL set differing only in the name is accepted 132/132 by all three ⇒ the name is the only variable and there is **no oracle split**. vita's 6 existing rejections are exactly `{wire,var,port}×{wire,var}`. ⚠️ The recorded line names params/labels/nets/genvars; **typedef names, functions, tasks, instance names, generate-block labels and ports are equally unchecked**, and the same hole is in generate, interface and package scope. ⭐ **Which declaration wins is per-CONSUMER, and one design gives two answers**: an enum label beats an explicit `localparam` in BOTH orders; a param beats a net for reads while the net's write and its continuous assign are silently dropped; `typedef logic[7:0] A; logic[3:0] A;` gives a 4-bit value and `$bits`=8; `function A` × `logic A` answers 9 in a value position and 5 in a call position. ⭐ Site = a pre-pass over `m.body` in `instance.rs::elaborate_instance` (which already visits every kind), **not** `net_util.rs::add_net` (50 callers). Twins to mirror: `iface_inst.rs`, `generate.rs`, `package.rs::elaborate_pkg_netvar` (§4.5.377's check is the right SHAPE and the wrong hook — package-local, parameters only). ⚠️ `elaborate_instance` runs once per INSTANCE, so dedupe per module definition. ⭐ **Regression exposure measured at ZERO**: 1,501 bench + 4 example files scanned (1 candidate, refuted — mutually exclusive `` `ifdef `` branches), and **15 legal-shadowing shapes** a naive check would false-reject are ACC in all three tools (block/task/loop locals are keyed `$blk$…`, not bare). Cheapest clean loud gap on the queue — but it buys nothing for any existing design, so do not rank it above a real §2 silent-wrong.
> | ~~**27**~~ | ✅ **RESOLVED 2026-09-01 — §11.4.10 in one helper, consumed by all three constant folds** | A shift's RIGHT operand is self-determined AND *"always treated as an unsigned number"*; the constant domain gave it the operand's OWN signedness (right for a ternary condition and a select index) so a narrow SIGNED count arrived negative and every shift by it collapsed to 0. `eval_const_shift_count` evaluates as before and then reinterprets the RESULT unsigned at the count's self width. **168-cell census: 14 FIXED · 0 REGRESSION**, and the six module-scope positions the re-grounding found are all fixed — a net's declared width (`$bits` 2 → 8), an unpacked dimension, a `repeat`, a `-:` width, a parameter bound, and a **`generate if` that had been taking the wrong branch and deleting its body at exit 0**. ⭐ Review's soundness lens found the third constant fold (`eval_const_env`) would have been left untaught — the `Pow` arm two lines above it already routes through a shared helper for exactly that reason — so all three now call one helper. ⚠️⚠️ **BLOCKING, mine, and it is [[a-default-is-not-a-fact]] for the third slice running**: `const_self_width`'s name arm reads `param_meta`, and for an untyped `parameter C = 3'sd1;` that is the DEFAULT literal's 3 bits, which §6.20.2 replaces the moment `#(.C(-3))` arrives. Reading the count SIGNED was **accidentally immune** (−3 is out of `0..64` at any width ⇒ 0, the right answer for a 32-bit count); reading it unsigned at a stale 3 bits makes it **5** and the shift really happens — 21 cells correct→silent-wrong across all four override channels, one into a declared net width. ⇒ the mask is applied only where the width is a FACT (a literal, an operator tree over literals, or a name `envw` records). ⭐ `param_range` was tried as a second admission and **measured** to carry an entry for that same overridden parameter ⇒ **residue: the module-scope NAMED count spelling keeps the pre-slice answer** (16 of 168 cells), and widening it needs the declared-vs-inferred provenance row 14 stopped at. ⚠️ Further residue review measured, PRE == POST: `-<sized signed literal>` as a count is still wrong for a signed LHS at width 63/64 and for any LHS ≥65 (`const_wide::fold_shift_count` does not fold a unary minus). ⭐ vita's own const-vs-runtime self-contradiction count over the review's sweep went **346 → 36**.
> | ~~**R30-2** unpacked-array element LHS~~ | ✅ **RESOLVED 2026-08-26 — and the diagnosis was refuted twice.** The axis is a SIGNED CONSTANT operand in the RHS, not the LHS storage class: over 34 cells native was ahead on every 1-D array-element-LHS cell and 3.9× ahead on the report's own headline shape. See §5. |
