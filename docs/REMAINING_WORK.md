# vitamin — 잔여 작업 트래커 (Remaining Work)

> **리뉴얼: 2026-06-10** · 기준 HEAD `b3651fa` → **동일자 전 항목 소진**: P0 9건 → P1 9건 → P2 12/12건 → P3 4건+양호판정 → P4 T0a/T0b/T1 → native-eval follow-on → P5 문서부채 전체(571 green, golden unflipped fmt_ver 3) → **2탄: perf 축 2건 + Phase-1.x 전부**(스케줄러축 ≈1.85x · 구조 native lane ≈2.8x · vita-log 게이트+exit 2 · filelist · explain · **format_version 4**(런타임 delay·dump flush/limit·Force/Release) — HEAD `8664627`, **611 tests green**) · clippy/fmt clean · MsgCode **50**. **이 트래커는 완결 — 신규 작업은 §권장 순서(아래 갱신본) 또는 ROADMAP §2.**
> **⚠️ 이 헤더는 2026-06-10 Phase-2 이전 스냅샷이다.** 2026-06-16 현재 = **format_version 8** · 약 **1088 tests green** · MsgCode **55**. Phase-2/3 진행(worklib·v7 bump·Phase-3 SVA 트랙 S4–S15+A1–A4·wait fork)은 **[ROADMAP](ROADMAP.md) §4가 단일 라이브 트래커**다 — 이 파일은 더 갱신하지 않는다.
> 출처: 7축 감사 — ①Gemini-fix 검토 ②spec-gap ③sim-engine ④front-end ⑤메모리/자원 ⑥운용성 ⑦병렬화. 핵심 항목은 라이브 재현(+iverilog 차분)으로 확정, 각 항목에 `재현:` 표기.
> 이전 트래커(2026-06-05 생성: 감사52 + Stage A/B/C 이력)는 **전항목 완결로 아카이브** — 이 파일의 git 이력(`b3651fa` 시점 버전) · perf 시계열 = [doc-18 §실측](preview/18-acceleration-analysis.md) · 전략 = [ROADMAP](ROADMAP.md). 요약은 맨 아래 §아카이브.
> 미해결 `- [ ]` / 해결 `- [x]` + 커밋·날짜. 우선순위: **P0**(silent-wrong 정확성) > **P1**(시뮬 의미론: warn-후-오동작) > **P2**(운용/CLI/진단) > **P3**(메모리/장기 안정) > **P4**(병렬화·신규 트랙) > **P5**(문서부채).

## 2026-06-23 Phase B — N7-REST 검증 플랫폼 착수(constrained-random verification B1)

> 사용자 결정 "B는 검증 플랫폼으로 키워. N7-REST 진행". **B1 완료**(1707 green·clippy/fmt clean·**format_version 9→10 bump**). vitamin이 "RTL 시뮬"에서 "CRV 가능 검증 플랫폼"으로 진입.
>
> **구현(B1):**
> - **파서**: `rand`/`randc` 데이터 멤버 + `constraint NAME { expr; … }` 블록(`parse_class_item`/`parse_constraint`, AST `ClassItem::{RandProperty,Constraint}`+`ConstraintDecl` → `.vu` AST hash 재핀).
> - **elaborate**: 제약 폴딩 `apply_constraint_expr`(`FIELD </<=/>/>=/== CONST`·`&&` 결합·`const OP field` flip) → per-field `[lo,hi]` 바운드(상속 체인 union·모순=loud). `randomize()` 인터셉트(`try_emit_randomize`: 문장·`r=…` 대입 양형) → **`class_rand` 사이드카(IR-0)**.
> - **sim-ir/engine**: 유일 IR 추가 `SysTaskId::ClassRandomize`(args=[obj_handle]) → **format_version 10**. 엔진 `class_randomize`가 obj→class_id→`class_rand` 조회 후 각 rand 필드를 **결정적 seeded `dist_uniform`**(iverilog-pinned·순수 f64·3-OS byte-identical)로 [lo,hi] 균일추출(≤i32=fast path·광폭/대경계=i64 modulo lane). 전용 `randomize_seed` 스트림(=$random/$urandom와 격리).
> - **staged**: `class_rand`를 14번째 `StagedExtraSidecars` trailer에 추가(STAGED-DROP 회피)·trailer-pin 재생성.
>
> **⚠️ 적대 silent-wrong hunt(4-에이전트·무iverilog 오라클=IEEE§18+통계 invariant)가 1건 발굴→즉수정**: 폭>32비트 또는 경계>i32인 제약 필드가 `[lo,hi]`를 무시하고 full-width 추출(`ranged=fits-i32` 게이트가 제약을 silently drop). ✅ 판정을 `constrained=(lo,hi)≠type_range`로 교체 + i64 draw lane(`draw_in_range`/`draw_u64`)으로 임의 폭에서 바운드 honor. ≤32비트 경계 의미론은 14프로브로 전부 정확 확인(strict 배제·inclusive 포함·`==` pin·역형 flip·음수/zero-straddle). 회귀: `class_crv.rs` 13종(폭40/63·상한 unsigned·longint 단측·상속·다필드·conjunction·staged·randc/모순 reject).
>
> **✅ B2 코어 완료(2026-06-23, rejection-sampling 솔버, format_version 11)**: inter-variable(`x<y`·`a+b==50`)·`x inside {set/[lo:hi]}`·implication `a->b`·`soft` constraint — `COp` 포스트픽스 술어 바이트코드(non-frozen, 골든 무영향). B2 적대 hunt 8 confirmed 수정(randomize() §18.11 반환값·signed>64bit·wide-술어 i64-unsafe loud). `soft` constraint. **✅ pre-existing `unsigned` 버그 수정(2026-06-23, 사용자 우선순위)**: `int/byte/shortint/longint/integer unsigned`가 signed로 비교/출력되던 broad 타입-시스템 silent-wrong — `opt_signed`→tri-state+`signed_eff(kind)`+`range_to_dims` flag 존중(iverilog parity, 1748 green). **✅ `dist`/`randc` 완료(2026-06-24)**: dist(가중 sampling `:=`/`:/`·`class_dist`·format 12)·randc(cyclic 순열·per-instance 상태·full-range v1·constrained-randc/>16bit loud·`class_randc`·format 13). **B2 잔여(마지막 1종)**: inline `randomize() with {…}`(per-call constraint·UserTaskCall AST flip)·`rand` real/string/array 멤버.

## 2026-06-23 Phase A — Tier ⓐ honest-loud 갭 4종 닫기 + 적대 hunt silent-wrong 3종 수정

> 사용자 결정 "A: 3개 닫기 + 잔여 2개 권장반영". **닫기 완료(전부 IR-0·format_version 9 불변·1694 green)**: ① `function void`(모듈/free=내부 TaskDef 변환·class=discard-at-call) + typed `parameter int/byte/shortint/longint/logic[W]`(lexer `void` kw + `parse_param_decl`/`parse_function_def` type-kw 분기) ② 고정크기 unpacked `foreach`(`lower_fixed_foreach_step` plain 인덱스 walk, **선언방향 존중**=descending는 hi→lo, signed 비교) ③ leading-`##` SVA consequent(`parse_seq_concat`이 암묵 `1` leaf 합성) ④ **`return` 키워드 = IR-0로 판명**(투자 전 read-only 검증이 doc의 "format_version bump 동반" 주장 **반증**: frame-func가 class-method와 동일 exit-block+`cur_return` 기구 재사용 가능 → 모듈/free 함수·태스크 `return` 지원, `body_has_return` 게이트로 return-free 본문은 byte-identical).
>
> **⚠️ 닫기 직후 적대 silent-wrong hunt(5-에이전트, 라이브 iverilog 차분)가 3종 발굴→즉수정**(전부 iverilog parity 재검):
> - [x] **typed/ranged param 값 미coercion** — `parameter byte B=200`→200(틀림, -56이어야)·`signed [7:0]=8'hA5`→165(틀림, -91). 근본=param이 bare i64로 저장돼 선언폭/부호 미적용(**pre-existing**: `signed [7:0]`도 동일). ✅ `coerce_param_value`(3 bind site)가 선언폭 truncate+signed sign-extend(unsized/`time`은 full 유지·`int`/`integer`은 암묵 signed). 1694 green=기존 param 테스트 0 회귀.
> - [x] **foreach 하강범위 반복순서 역전** — `int b[3:0]`을 0,1,2,3로 순회(틀림; IEEE §12.7.3=선언순 3,2,1,0). ✅ `array_dim_desc`로 방향 판정. (위 ②에 포함된 수정)
> - [x] **frame-func 2-state 로컬/return-slot 기본값 X** — `int z`(미대입)을 X로 초기화(틀림; IEEE §6.4 2-state=0), `case(z)`가 wrong arm(제어흐름 오염). 근본=`run_frame_call`/`run_task_call`이 `Value::xs`로 전 슬롯 X-fill(net의 `init` 무시). ✅ `Value::from_packed(&nv.init,…)`로 교체(4-state=X·2-state=0 보존).
>
> **✅ Tier0(2026-06-23) — 적대 hunt 발굴 silent-wrong 전량 수정**(사용자 결정 "의사결정 없이 모조리 bugfix", 1733 green·전부 IR-0·format_version 10 불변): (a) **task output formal copy-out 위반** ✅ — inline-task 직접 aliasing(`out_subst`)을 **formal-폭 local-net copy-in/copy-out**으로 전면 교체(IEEE §13.5.1/§13.5.3): width/sign coercion·§13.4.1 static 단일인스턴스 retention(task당 공유 local)·intermediate-write/glitch 제거·input↔output aliasing 해소·narrow-input 절단·nested output threading·2-state X→0 coercion. (b) **SVA 시퀀스/property X/Z 불리언 = NON-match** ✅ — `sva_match(e)=(\|e===1'b1)` X-strict 래퍼를 전 consequent 사이트(boolean·sequence·multiclock·crossclock·prop-expr·liveness)+`disable iff(X)`에 적용(antecedent는 `LogAnd(ante,!match)`로 자연 vacuous). **동반 수정(2-state)**: `int x=5` continuous-driver→1회성 init(`is_var` 추가)·`int x=P`/`reg x=A+B` param/expr 상수 init 폴딩(const-eval). **적대 hunt 2회(24+5 confirmed) 전량 수정**(라이브 iverilog 차분; SVA=hand-IEEE). **잔여 known-limit(별개·pre-existing·희소)**: 비상수 var-ref init `int x=다른변수`(전 var-type 공통, init-phase ordering 필요)·output/inout actual part-select(loud).

## 2026-06-22 적대적 스펙-감사 — silent-wrong 4종 수정 + 잔여 loud 갭

> 6영역 스펙↔구현 적대 감사(라이브 vita-vs-iverilog 프로브). **confirmed silent-wrong 4종 = 전부 수정 완료**(TDD, 1613 green, golden/clippy/fmt clean, format_version 9 불변 — 전부 사이드카/엔진/포맷-렌더러라 IR-0). 나머지는 LOUD(안전)이라 추적만.

**✅ 수정 완료 (silent-wrong → 정정):**
- [x] **SW1** class 필드 선언 초기화자 `int x=42` 무시(읽기 0) — ✅ `collect_class_fields`가 `fold_init`으로 상수 폴딩 → `class_field_inits` 사이드카(SimOpts) → 엔진 `class_alloc` 기본값 override(IEEE §8.8). 비상수 init=loud. 회귀 `class_oop::field_declaration_initializer_applied`.
- [x] **SW2** derived ctor가 `super.new()` 생략 시 base ctor 미실행(필드 0) — ✅ `lower_class_method_body`가 `new` 본문 선두에 `super.new()` 자동 주입(IEEE §8.13, `body_calls_super_new`로 중복 방지, static 디스패치). 회귀 `auto_super_new_runs_base_ctor`+`explicit_super_new_still_works`. iverilog 오라클 일치(`d.x=5`).
- [x] **SW3** `%0N` 제로패딩(`%06d`→`42`, `%06h`→`a`) — ✅ `render_template`/`fmt_radix`가 `min_zero`를 width 유무와 분리: `%0d`=minimal·`%0Nd`=zero-pad(부호인지 `-00042`)·`%Nd`=space-pad·`%h`=full width. iverilog byte 일치. 회귀 `display_semantics::zero_pad_format_specifiers`.
- [x] **SW4** VCD `real` 신호가 `r<%.16g>` 대신 64-bit 바이너리 벡터 방출(GTKWave 오표시) — ✅ vcd-writer가 `VarType::Real` id를 기억(`VarMeta.is_real`)해 `r<value> <id>` 자동 포맷(`encode_real`+`fmt_g16`, 결정성 Ryū 기반). spec 07:164 일치. 회귀 `vcd-writer::real_var_emits_r_format`.

**✅ 잔여 LOUD 갭 4종 = 2026-06-23 Phase A에서 전부 닫힘(위 섹션):**
- [x] **고정크기 unpacked `foreach`** — ✅ `lower_fixed_foreach_step`(선언방향 존중). 잔여 loud=`break`(키워드 미지원·E3010), multi-dim `foreach(m[i,j])`(E2002) — 둘 다 honest.
- [x] **자유/모듈/패키지 함수 `return` 키워드** — ✅ IR-0(format-bump 주장 반증)·`body_has_return` 게이트. 잔여 loud=함수/태스크 본문 내 `$display`/NBA/force(frame-call subset, E3009).
- [x] **`function void` 반환형 · typed `parameter int W`** — ✅ lexer `void` + parser type-kw 분기. (값 coercion=위 hunt 수정에 포함). 잔여 loud=`longint L=64'h…`(64-bit 사이즈드 리터럴 폴딩 불가, E3009).
- [x] **leading-`##` SVA consequent** (`a |-> ##1 b`) — ✅ 암묵 `1` leaf 합성(golden-neutral).

## Gemini shift fix 검토 결과 (2026-06-10 · 채택)

`const_eval_in_scope`의 `wrapping_shl/shr` → `checked_shl/shr().unwrap_or(0)` (elaborate/src/lib.rs:1379-1382):

- ✅ **채택** — Rust `wrapping_shl`이 shift량을 mod 32로 마스킹해 `1<<32`→1이 되던 함정 제거는 옳다. 460 green·clippy 무영향·골든 무영향, 4개 arm 통합도 무해.
- ⚠️ 단, gemini_debug.md의 근거 서술 2건은 부정확: ①"중복 매치 암 제거" = **오진**(제거된 것은 별개 variant `AShl/AShr`; 진짜 중복이면 clippy `-D warnings` 게이트가 이미 실패했음) ②"0이 정답" = 32bit-exact 해석에서만 참 — **차분 오라클 iverilog는 `1<<32`=4294967296**(unsized 상수를 >32bit로 폴딩)이고, `parameter [63:0] F = 1<<32`는 IEEE 컨텍스트 확장상 2^32가 명백 정답(vita 0, 수정 전 1 — 둘 다 오답). 근본 해소 = P0-6.

## P0 — 정확성: silent-wrong-value (최우선)

**런타임 >64bit 절단 클러스터** — 공통 근원: `Value::to_u64`(value.rs:313-320)가 width>64에서 None 대신 word0 절단값을 반환.

- [x] **[P0-1]** >64bit relational 비교 절단 — ✅ 2026-06-10 `7bfd8c3`. 임의 폭 word-wise 정확 비교(부호 인지)로 교체, 64/128 lane 의존 제거. 회귀 `wide_value_semantics.rs` + iverilog 차분 `diff_wide_value_truncation_cluster`.
- [x] **[P0-2]** shift-amount 절단 — ✅ `7bfd8c3`. over-u64 amount는 saturate(전부 shift-out: 논리 0/산술 sign-fill). x/z는 기존대로 X.
- [x] **[P0-3]** unary minus(negate) 단일워드 — ✅ `7bfd8c3`. 전 폭 two's complement(word carry).
- [x] **[P0-4]** **`to_u64`/`to_u128` 계약 수정**(overflow→None) + 호출부 전수 — ✅ `7bfd8c3`. array word index/lvalue offset의 `as u32` wrap(2^32+k→k, 읽기·쓰기 모두)도 OOR sentinel로; part-select offset·$clog2(임의 폭 정확)·%c(low byte 유지)·unsigned→real(u128 lane). arith()는 기존 width 게이트 뒤라 unwrap 안전. 워크스페이스 470 green.

**elaborate 상수 도메인 클러스터:**

- [x] **[P0-5]** 폴딩 불가 param/localparam/enum-label → silent 0 — ✅ 2026-06-10 `b30881a`. ternary `?:`+`$clog2` 폴딩 추가, 미폴딩 param/enum-label은 **ElabUnsupported Error**(0은 post-error recovery 값일 뿐). concat/함수호출 폴딩은 필요 시 후속(현재는 loud).
- [x] **[P0-6]** const 도메인 u32 → 부호 있는 i64 — ✅ `b30881a`. `1<<32`=4294967296(iverilog parity), checked 산술(overflow=loud), signed AShr sign-extend, 음수 param은 32bit signed const로 바인딩(`%0d`→`-4`), `0..=u32::MAX`는 기존 const 형상 그대로 → **기존 디자인 골든 byte 불변**(482 green). 잔여: >64bit 리터럴/i64 초과값은 도메인 밖 → None(loud).
- [x] **[P0-7]** 하강 generate-for 폭주 — ✅ `b30881a`(P0-6의 signed 비교로 해결). `for(i=3;i>=0;i=i-1)` 정상 4회 unroll + zero-trip(-1 시작) 무진단 통과. 회귀 `const_domain_semantics.rs` + iverilog 차분 `diff_const_domain_cluster`.

**display/monitor 의미론:**

- [x] **[P0-8]** `$display` 인자 의미론 — ✅ 2026-06-10 `5b3c6d4`. IEEE §17.1 순차 처리로 엔진 통합: ①잔여 인자=기본 radix(padded %d/실수 %g) ②문자열 인자=inline 포맷 세그먼트(StrUtf8 검출, 후속 인자 소비) ③무포맷 branch=패딩 필드 연접(공백 join 제거) ④`%v`(St0/St1/StX/HiZ)·`%u/%z`(소비+무출력)·`%p`(값) 인자 소비. elaborate 무변경(엔진만), 회귀 `display_semantics.rs` 10 + iverilog 차분 `diff_display_arg_semantics`(패딩까지 byte 일치).
- [x] **[P0-9]** `$monitor` 트리거 과민 — ✅ `5b3c6d4`. ①직접 `$time/$realtime` 인자는 변화 비교에서 제외(IEEE §17.1.3 — 시간만 흘러도 매 스텝 재인쇄하던 버그) ②비교를 비트평면(width/val/unk)만으로(`vals_same_bits`).
- [x] **[P0-10]** unsized integer 리터럴 폭이 값을 안 담음 (≥2³¹ silent 절단/오부호) — ✅ 2026-06-16 `2233ccd`. `parse_int_literal`이 unsized 리터럴을 고정 32-bit로 pack해 **≥2³¹부터** 깨짐(2³¹→−2³¹·2³²→0, 좁은 폭이 self-determined expr context도 오염: `2³²>1`=0). IEEE §3.5.1 "최소 32, 값 담게 성장"으로 교체(iverilog `$bits` byte-exact): 평decimal=max(32,nbits+1)(부호비트)·`'d`=max(32,nbits)·`'sd`=max(32,nbits+1)·`'h/'b/'o`=max(32,digit-span)(leading-zero 계수). sized·SV fill 불변, <2³¹/≤32-digit-bit는 width-32 유지(골든 byte-identical). 로버스트: `lower_int_literal`에서 `MAX_NET_WIDTH`(1<<20) 캡(net과 동일, over-cap=loud E3009, `bits.len() as u32` 오버플로 무사화). 단위 t11b + e2e `literal_width.rs` 6 + 3렌즈 리뷰(의미론 byte-exact·골든 CLEAN·shift/unary-minus 폭 발산은 vita가 iverilog보다 IEEE-정합).
- [x] **[P-perf] `decimal_bits` O(n²) — 거대 십진 리터럴 parse DoS** — ✅ **2026-06-16 (1030 green)**. 옛 schoolbook base-10→binary 장제법(비트당 O(digits) 패스 + 비트당 `Vec` 재할당 = O(digits²), 40000자리=27s)을 **Horner base-10¹⁹ over base-2⁶⁴ limbs**(u128 중간, 청크당 O(limbs))로 교체 → 40000자리 **27s→25ms(~1100×)**, 최악 수용(315652 nines=1Mbit) 977ms(debug). 추가 ① **`MAX_DECIMAL_DIGITS` 시간-경계 가드**(=`floor(cap·log10 2)+4`≈315656, over-cap는 변환 전 O(n) digit-scan 거부 → 320000 nines **3.65s→16ms**; 옛 320000 슬랙은 폭-경계라 변환 후 거부=잔존 DoS였음) ② **`alloc_width` clamp**(거대 explicit-width `4294967295'h1`의 ~1GiB `pack_bits` 차단 = 적대 리뷰가 끌어낸 인접 hole; const-eval 경로 `localparam P=4294967295'h1` 30.9s/1GiB→0.01s 동반 수정) ③ malformed-echo 64자 절단(무한 stderr 방지). byte-identity는 멀티워드 characterization(`t11c`) + 51/51 python + iverilog(워크플로우 44 + spot)로 락, 골든/`fmt_ver 8` 무변경(elaborate-only). 적대 3렌즈 리뷰(286+ 라이브 프로브) = byte-identity·boundary·regression CLEAN, 유일 MEDIUM(O(n²)/가드=폭경계)을 base-10¹⁹+가드축소로 소진. 잔존 O(n²)는 가드로 바운드(수용 최악 1Mbit 리터럴 ≈ sub-second; 실 RTL ≤~100자리=µs).

## P1 — 시뮬 의미론: warn-후-오동작 (정지·계속 클래스)

- [x] **[P1-1]** `$fatal/$error/$warning/$info` — ✅ 2026-06-10 `8d6abec`. Display 스텀트 + **SeverityTable 사이드테이블**(StmtId→kind, SimOpts/.velab 5번째 trailer, frozen IR 0줄·골든 무영향)로 구현. `$fatal`=묵시 $finish+ExitClass::Fatal(exit 1, 선행 finish_number 리터럴 소비), `$error`=HadErrors+계속, `$warning/$info`=진단만. 출력=진단 스트림(F4004/E4003/W4007/I4005, sim_time 부착) — stdout 비오염. Kernel/Op/StmtEffect에 StmtId 배선(인터프리터=VM parity 테스트). 회귀 `severity_tasks.rs` 8 + `severity_exit.rs` 4(staged trailer 왕복 포함).
- [x] **[P1-2]** force/release·proc assign/deassign·`->event` — ✅ 2026-06-10 `522e76c`. ElabUnsupported 하드에러 승격(구문별 메시지). 회귀 `legality_semantics.rs`.
- [x] **[P1-3]** 비상수 `#delay` — ✅ `522e76c`. loud-reject(런타임 delay=Phase-2, frozen Delay{u32}).
- [x] **[P1-4]** in-body `@(*)`·멀티엣지 — ✅ 2026-06-10 `097f2c3`. `@(*)`=제어 문장 read-set 추론(IEEE 1800 §9.4.2.2, Wait cause 사후 패치로 comb 기계 재사용); 멀티엣지=loud-reject(frozen Edge 1-term). iverilog 차분 포함.
- [x] **[P1-5]** b/o/h 16종 — ✅ 2026-06-10 `4292eec`. RadixTable 사이드테이블(StmtId→2/8/16, 6번째 trailer)+FmtCapture.radix, `fmt_radix` 재사용(iverilog 무구분자 padded join까지 byte 일치, 라이브+차분 검증). **Sidecars 구조체 도입**(5→7테이블 번들; trailer는 세그먼트별 append-only 유지). doc-01 주장은 이제 참.
- [x] **[P1-6]** `$finish` 동일스텝 postponed 유실 — ✅ `097f2c3`. Finish/Stop/Fatal 전부 현 스텝 드레인 후 종료(IEEE §17, Icarus/VCS 정합). MVP-분기 박제 테스트는 IEEE-strict 계약으로 갱신.
- [x] **[P1-7]** `fork_mode()` panic — ✅ `522e76c`. t0 게이트+런타임 둘 다 Fatal 진단(E9001)+graceful 종료. trailer 외과 절단 회귀(staged_flow.rs).
- [x] **[P1-8]** part/bit-select 멀티드라이버 — ✅ `097f2c3`. per-bit 구간 계상(whole=[0,w), 정적 select=[off,off+w), `(msb-lsb)+1` 폭 엣지 폴딩). 중첩=E3001, 분할 구동=합법 유지. 동적 offset은 보수적 미계상.
- [x] **[P1-9]** net-vs-var 적법성 — ✅ `522e76c`. **E3018 `E-ELAB-LVALUE-KIND`**(부록A→본문 승격): user `assign`→Reg/Integer/Real 거부, 절차 대입→Wire 거부, SV logic 양방향 통과, 포트바인딩/decl-init 합성분 면제(IEEE 1800 var-port). 위법 픽스처 3건 교정 부수확.

## P2 — 운용/CLI/진단 견고성

**silent-failure 군집:**

- [x] **[P2-1]** VCD open 실패 침묵 — ✅ 2026-06-10. `W-RUN-VCD-OPEN-FAIL`(VITA-W4018, 경로+OS에러) 경고 후 시뮬 계속. 회귀 `run_diagnostics.rs`.
- [x] **[P2-2]** VCD flush 에러 침묵 — ✅ `finalize_vcd` flush 실패 → `W-RUN-VCD-WRITE-FAIL`(VITA-W4019). 단위테스트 state.rs(FailWriter 주입).
- [x] **[P2-3]** delta-limit 무진단 — ✅ `F-RUN-NO-CONVERGE`(VITA-F4016, 부록A→본문 승격) 단일샷 발행. 전 경로 funnel(settle/run-loop `fatal_delta_limit` + interp/VM in-body guard `mark_fatal`), VM parity 테스트. ⭐4-state에선 `assign a=~a`가 X-안정이라 발진 repro에 정의값 시드 필요.
- [x] **[P2-4]** `--help/-h`/`--version/-V` — ✅ 전 applet(vita/vcmp/velab/vrun) usage+버전 출력, exit 0. `cli_ux.rs` 4 테스트. MsgCode 45→**48**(bijection 게이트 동기화, doc-15 본문 3절 추가).

**안전 레일:**

- [x] **[P2-5]** parser 재귀 가드 — ✅ 2026-06-10 `41f5162`. expr 재귀 cap **256**(2MiB 디버그 테스트 스택에서 512도 초과 — 프레임 비대) → clean parse error. ⚠️깊은 **문장** 중첩(begin×N)은 별도(희귀, 잔여 소항목).
- [x] **[P2-6]** `MAX_ARRAY_LEN`(1<<24) — ✅ `41f5162`. 초과=ElabUnsupported.
- [x] **[P2-7]** 아티팩트 비원자적 쓰기 — ✅ 2026-06-10 `write_artifact_atomic`(`<out>.tmp.<pid>` → rename, 실패 시 tmp 정리). vcmp/velab 양쪽. 잔여물-부재 회귀 `staged_flow.rs`.
- [x] **[P2-8]** native_eval eid 방어 — ✅ `41f5162`. `exprs.get()?` bail→오라클 폴백.
- [x] **[P2-9]** `--timeout <ticks>` — ✅ `41f5162`. vita/vrun, SimOpts.time_limit 배선(도달=clean Quiescent exit 0). 기본은 무제한 유지.

**진단 taxonomy/계약:**

- [x] **[P2-10]** warn() 코드 오염 — ✅ `41f5162`. 범용 warn=**W3056 `W-ELAB-FEATURE-LIMIT`**(부록A→본문 승격). W3008은 실제 폭-절단 경고 구현 전까지 본문 예약(현재 emitter 0 — 의도된 dead).
- [x] **[P2-11]** ✅ `41f5162`(+P1-1로 RunUser\*/RunFatal 활성화). 중복 모듈=**E-DUP-UNIT Error**, `%m`=proc_scopes 사이드카(7번째 trailer)로 실 계층경로(strobe/monitor는 등록 스코프 복원; iverilog 내용 일치 라이브 검증). 잔여 dead codes(DupUnit→활성됨 제외: ParseImplicitNet·ElabUser\*·RunAssertFail·RunNoLocations·LintUnclosed·W3008)=예약 상태 명문화. exit class 2 문서 불일치=P5로.
- [x] **[P2-12]** 정책 소항목 묶음 — ✅ 2026-06-10 일괄 처리. ~~`$finish(n)` doc 주장~~(✅ doc-04) · `timescale_unit_string`=clamp([-15,+2] 포화, "-16→100s" 오렌더 제거) · **`time` 타입 수용**(64-bit unsigned 4-state 변수, NetKind::Reg 매핑 — frozen IR 무변경; unpacked 배열·`parameter time` 포함, iverilog 차분 `diff_time_type_semantics` 추가) · `` `pragma`` 수용-무시(IEEE §22.11, 줄 소비·무진단) · implicit-net 정책 명문화(doc-15 W2003: v1=사실상 `default_nettype none`, 미선언→E3010, W2003=예약) · `same_path`=이미 canonicalize 동작 확인(./x.sv vs x.sv 거부 회귀 박제). 잔여 없음.

## P3 — 메모리/장기 시뮬 안정성

- [x] **[P3-1]** fork 아레나 무한 성장 — ✅ 2026-06-10 `0945dfe`. free-list 슬롯 재활용(child=보고 직후, barrier=전 자식 보고 시 — 그 시점엔 살아있는 참조 0 ⇒ ABA-safe; 순서키=tie라 byte 불변). churn 회귀 2종(join_none×5000 정확 1회 실행, blocking join×2000).
- [x] **[P3-2]** monitor baseline Vec — ✅ `0945dfe`. eval→비교→in-place 덮어쓰기(모니터 수명당 1회 할당).
- [x] **[P3-3]** VCD sink BufWriter — ✅ 2026-06-10. `BufWriter::with_capacity(64KiB)` 래핑(finalize가 명시 flush → byte 불변). dump-heavy perf 측정(T0b 잔여)은 P4에서.
- [x] **[P3-4]** net_to_edge clone — ✅ `0945dfe`. 인덱스 루프(바디는 cur.active만 push).
- [x] **[P3-5]** native 스택 alloc — ✅ `0945dfe`. 고정 64슬롯 배열+sp(호출당 heap 0); try_compile이 post-order 깊이 검증(초과=오라클 bail).
- [x] **[P3-기록] 종료/메모리 위생 양호 판정 (2026-06-10 감사)** — `unsafe` 0건 · Rc 9곳(vm_cache 한정) 비순환 · `finalize_vcd` 전 종료경로(정상/$finish/$stop/delta-limit/error) 호출 · HashMap 3곳(vcd by_id·parser typedefs 등) lookup-only로 결정성 무해 · BTree-only 스케줄러 재확인. Ctrl-C 핸들러 없음 = 커널 fd flush로 마지막 완료 write까지 유효한 truncated VCD(문서화만 권장). CLI 종료 시 미해제 누수 없음(정상 Drop + OS 회수). 라이브러리 임베딩 시 재평가.

## P4 — 병렬화 트랙 (신규 · 2026-06-10)

**현황:** 프로덕션 코드 스레딩 0(std::thread/rayon/Arc/Mutex 부재), 기존 계획 0 — doc-18:19가 PDES를 "결정성(3-OS byte-identical)과 상충·장기"로 박제했을 뿐, `--threads`류 옵션 구상 부재. 엔진은 의도적 단일스레드(`!Send`인 `Box<dyn Write>`/`LogSink`/`Cell`)이나 Rc는 9곳(vm_cache)으로 얕고, `simulate(&SimIr)`은 불변 입력의 순수 함수 — **스레드/프로세스당 1 시뮬은 이미 자유**.

**옵션/UX 설계(확정안):**
- `--threads N`(alias `-j N`) — `vita`/`vrun`에 추가(vcmp/velab은 당장 대상 없음). 기본 `auto` = `min(available_parallelism, 8)`(std, MSRV 1.82 OK, 신규 dep 0). env `VITA_THREADS`(플래그 우선). `--threads 1` = 현행과 완전 동일 경로.
- **계약: 모든 N·모든 OS에서 VCD/stdout/아티팩트/exit code byte-identical** — thread 수는 wall-clock만 바꾼다. corpus를 `--threads 1` vs `4`로 byte-diff하는 P5식 차분 게이트로 강제. 구현은 `SimOpts` out-of-band(frozen IR·골든 무영향).

| 단계 | 내용 | 기대효과 | 결정성 리스크 | 공수 |
|---|---|---|---|---|
| ✅ **T0a** | ~~multi-run 병렬~~ — 2026-06-10 완료. `backend_equiv`가 interp·VM을 `thread::scope` 동시 실행(`SimIr`=Sync, sink/VCD 경로 스레드별 분리) | 차분 스위트 ~2x | 0 | — |
| ✅ **T0b** | ~~BufWriter+측정~~ — BufWriter(P3-3)+`perf_dump_share` 측정 케이스. **실측: dump-heavy VCD 비중 40.9%(BufWriter 적용 후), T1 이론상한 ≤1.69x** | 측정 완료 → T1 정당화 | 0 | — |
| ✅ **T1** | ~~`--threads ≥2` VCD writer 스레드~~ — 2026-06-10 완료. `vcd_thread::ThreadedWriter`(bounded FIFO 8×64KiB chunk, 순서보존, write에러는 flush에서 표면화→W4019 경로 유지, Drop=drain+join). CLI `--threads N`/`-j N`(vita·vrun)+`VITA_THREADS`+auto(min(cores,8)). **byte-identical 계약 게이트**: `tests/threads.rs`(엔진 1vs4) + `cli_ux.rs`(subprocess 1vs4 VCD byte-diff) | VCD I/O 은닉(상한 1.69x) | 0(게이트 강제) | — |
| ❎ **T2** | ~~front-end per-CU 병렬~~ — **2026-06-11 측정 폐기**: 400모듈/12k라인 전체 front-end(vcmp)=**~10ms**(예상대로 비병목) + 단일-CU concat 모델(파일 간 `` `define`` 가시성)이라 per-CU 분할=의미론 변경. dump-heavy 추가 실측: 64-net/20k-tick $dumpvars 0.78s, writer 스레드(--threads 2) −3%(BufWriter가 syscall 비용 기흡수) | 폐기(측정) | — | — |
| ✕ **T3** | parallel elaborate — **비추천**: 전역 arena ID 순서 자체가 골든 계약, byte-identical 재현 머지 비용 高 | 小 | **高** | — |
| ❎ **T4** | ~~엔진 내 PDES/정적 파티셔닝~~ — **2026-06-11 타당성 연구 종결(조건부 NO-GO)**. 실측: τ(상주 spin-pool) 0.3~0.5µs/delta(naive spawn 31~93µs=즉사) · g≈700ns/activation · BSP mock 디스패치 측 W=64↑에서 3~4x — 그러나 sample 분류상 직렬 잔류(apply_nba/propagate) ~20% → **Amdahl 상한 T4 ≈2.5x**, corpus 워크로드는 W=1~8이라 0~손해. **결정성은 차단 요인 아님**(NBA-pure 클래스 병렬+run-splitting+per-process 로그 머지로 byte-identical by construction — 설계 스케치 doc-18). 재진입 조건 핀: 지속 W≥64+grain≥200ns 실워크로드 → BSP v1. 프로브 3종(`perf_pdes_*`)은 영구 계기 | 종결 | — | 연구 완료 |

## P5 — 문서부채 (docs ↔ code 불일치)

- [x] 01-display-io.md b/o/h·예시 주장 — ✅ **구현으로 해소**(P0-8이 :46 예시를, P1-5가 :11/219 "16종" 주장을 참으로 만듦). 문서 수정 불요.
- [x] ROADMAP §D "의도적 deferral 전부 loud-reject 확인됨" → 거짓이었음 — ✅ 2026-06-10 리뉴얼에서 §D 정정 + P1-2/3으로 실제 loud-reject화 완료.
- [x] doc-13/15 동기화 잔여 — ✅ 2026-06-10. ~~`$fatal` abort·exit-1~~(✅ P1-1로 참) · `-Wno-*`/`-Werror=` 억제 플래그=Phase-1.x 미래형 명기(doc-15 거버넌스 + doc-13 suppression 절, `--help`=진실 공급원) · 예약 dead codes(ParseImplicitNet·ElabUser\*·RunAssertFail·RunNoLocations·LintUnclosed·W3008) 실태 명기(doc-15 거버넌스 불릿) · exit class 표 정정(doc-13: 현 구현=0/1/3+101, class 2=예약·현재는 1로 분류 명기).
- [x] 소항목 잔여 — ✅ 2026-06-10. ~~10-vcd "7종"~~(✅) · ~~04 "$finish severity"~~(✅) · hdl-parser:1119 주석(게이트 프리미티브=키워드-led, 이 arm 미도달·E2002 loud 명기) · doc-01:22-26 filelist `-f`/multi-lib/`vita explain`=**Phase-1.x 인라인 표기로 결정**(de-scope 아님, 목표 유지).
- [x] (구)트래커:290-292 doc-01 drift 3건 — 2026-06-07에 이미 교정 완료된 stale checkbox였음. 이번 리뉴얼로 해소.

## 권장 작업 순서 (다음 세션 — 2026-06-10 2탄 후 갱신)

> **🏁 2026-06-11 마라톤: 아래 권장 순서 전체 소진(728 green, 7커밋 push).**
> ⑥front-end 일괄(.vu 재핀 1회) → ⑨P4-T2 측정 폐기 → 운영 인프라 3종
> (--dump-filelist·filelist dedup·RULE-V composite 기록) → §A word化(STRUCT_HEAVY
> interp ≈1.44x) → foreach desugar(AST/IR 0) → 적대 리뷰 적발 2건(rename 워커
> silent-capture) 수정·teeth 회귀.
>
> **🏁 2026-06-11 2차 마라톤: 구현 트랙 ①~⑤ 전부 소진(765 green, 5커밋 push, 적대 리뷰 9불변식 PASS·결함 0).**
> ~~①차기 format-bump 묶음~~ ✅ **v6 bump**(`212f85f`) — queue `insert(i,v)`/`delete(i)`(iverilog 차분),
> assoc `first/next/last/prev`(QPop-가족 문장-인터셉트 `StmtEffect::AssocIter`, hand-IEEE §7.9.4,
> 좁은 ref var=truncate+−1+W4020), **foreach를 uniform first/next desugar로 재작업**(dyn/queue=
> dense walk, 합성 인덱스 게이트), string 키(`NetKind::AssocStr`+`Offsets::AssocStrKey`+사전식
> first/next; AST `AssocKey::Str` 재핀). ⭐부수 적발: **string 리터럴 LSB-first 패킹 = IEEE §5.9
> 위반 라이브 버그**("ab"=25185 vs iverilog 24930) — 패킹·`const_string` 대칭 플립+회귀 핀.
> ~~②(D) 후속~~ ✅(`5840f20`) — modport 방향 강제(listed-only 가시성+input=read-only,
> `walk_scopes_key` 단일 진실 리팩터로 name-레벨 정밀도), iface `#(parameter)`(부모-스코프 해소),
> ANSI 헤더 포트(LATE pass 배선=wire_ports 재사용), generate-내 iface 바인딩(외향 스코프 워크).
> ~~③bounded queue~~ ✅(`5f6a549`) — `QueueBoundTable` 사이드카(8번째 trailer)+엔진 tail-truncate
> 단일 규칙(iverilog 라이브: push_back-full=skip/push_front·insert-full=뒤 요소 탈락 전부 재현),
> staged 왕복 e2e. ~~④§A 미세~~ ✅(`68f545a`) — **wide 구조 트리오**(WSelect/WConcatPair/WRepl,
> 65..=128bit: WIDE_STRUCT_HEAVY **VM 0.44x ≈2.3x**) + real lane **측정-폐기**(REAL_HEAVY VM 0.90x,
> real은 합성 핫패스 부재 — 프로브는 영구 계기) + has_xz/to_u128 **검사-폐기**(inline-Value 때 이미
> word-parallel). ~~⑤예약 진단 arm~~ ✅(`bf7f146`) — **E8003 CONFLICT arm**(sticky ctx=상속
> `` `timescale``, dup 존재 시에만 컨텍스트 워크) + **E9003 `vrun --upstream`**(라이브 재해시 vs
> composite, exit class 2 — worklib은 발견 자동화만 추가).
>
> **현재 잔여:** 없음 — ~~⑥ PDES(연구 트랙)~~ ✅ **2026-06-11 타당성 연구로 종결(조건부 NO-GO,
> P4-T4 행·doc-18 §PDES 참조 — 재진입 조건 핀: 지속 W≥64+grain≥200ns 실워크로드)**. 이로써
> **Phase-1 작업 큐 완전 소진.** 이후는 작업 항목이 아니라 스코프 결정 사안 — **두 축(확장 트랙
> P2-A~F+조건부/장기 · MVP 컷 인벤토리→해제 매핑)을 전개한 퓨처 플랜 = [ROADMAP §4](ROADMAP.md)**
> (권장 진입 순서: ①worklib(bump 0) → ②v7 bump 묶음(system tasks+string+package+정밀화 소묶음)
> → ③절차 고급 → ④SVA 서브셋(Phase-3)).

1. ~~트래커 P0~P5 전체~~ ✅ · ~~perf 축(스케줄러 R1·구조 native lane)~~ ✅ · ~~Phase-1.x 전체(게이트/filelist/explain/v4 bump/force-release)~~ ✅ — **611 green, HEAD `8664627`.**
2. ~~①dirty-list 넷 스캔(R2)~~ ✅ 2026-06-10(NETS_HEAVY 305→15.5ms ≈19.7x — dirty 스윕+snapshot_prev 삭제) · ~~②filelist typed 버킷~~ ✅ 2026-06-10(-D/-I·+define+/+incdir+·WRONG-STAGE·OVERRIDE) · ~~③native-eval C6 lane~~ ✅ 2026-06-10(array-indexed `LoadIndexed` + 65..=128bit u128-pair wide 스택 — WIDE_HEAVY 0.59x·MEM_HEAVY 0.72x, narrow 무변경; 잔여 native = signed>64/wide 구조/real/sysfunc 저ROI 문서화, doc-18 §실측). ~~④vita-log 2단계~~ ✅ 2026-06-10(--log 단일-writer tee·-q/-v/--verbosity·counts epilogue `errors= warnings= notes=`·unopenable log=exit 3) · ~~⑤intra-assignment delay·force 재평가·implicit-net~~ ✅ 2026-06-10(blocking `= #d` 실semantics + NBA `<= #d`=loud E3009→차기 bump·force 연속 재평가·implicit-net=E3010 정책 확정으로 종결). ~~⑥Phase-2 관문~~ ✅ 2026-06-10 평가 완료(ROADMAP §F: bump-필수=NBA-delay·named-event·dynamic-storage는 v5 일괄, IR-무변경=immediate-assert·interface 스파이크·disable은 즉시 가능 — 진입 시퀀스 명문화) · ~~⑦3-OS CI~~ ✅ 2026-06-10(`.github/workflows/vitamin-ci.yml` — ubuntu/macos-14/windows 매트릭스, fmt/clippy/test --locked, ubuntu에 iverilog 설치로 차분 오라클 실가동, paths 필터로 vitamin 변경시만). **첫 가동이 Windows-전용 발산 3건을 연속 적발·수정 후 3-OS 전부 green**(run `27276108641`): ①테스트 include 셰임의 `/`-키 맵이 `Path::join`의 `\` 미스(`b34f67e`) ②autocrlf 체크아웃이 LF 골든을 CRLF화 → `.gitattributes` `-text` 핀(`b28ecb8`) ③ron `PrettyConfig::default()`가 플랫폼 newline(Windows `\r\n`)로 생성 → 명시 `\n` 핀(`3a52230`). 셋 다 프로덕션 코드 아닌 주변부의 byte-identity 누수. ~~⑧net_to_edge/waiter 자료구조~~ ❎ 2026-06-10 **측정으로 폐기** — `perf_nets_scaling` 프로브(512→2048→8192 idle nets)가 평탄(~15-17ms)을 보임: R2가 idle-net 세금을 전부 제거, waiter 워크는 부하 비례라 세금 아님. 프로브는 영구 회귀 계기로 잔류. ~~ROADMAP §F 진입 시퀀스~~ ✅ **2026-06-10/11 완주(674 green)**: (E) immediate assert(파서 If-desugar, AST 동결 유지, iverilog 차분 — `66c880b`) → (D) interface 스파이크(GO: 심볼 aliasing, SimIr 무변경 확정 — `3068be9`) → (F) disable 실동작(동봉 named block Goto, lazy exit-BB로 기존 CFG byte-불변)+proc-assign/deassign(force weak-rank 재사용, `assign_ranks` 사이드카+trailer 세그먼트 — `0ac0069`) → (C) dynamic-storage 설계 문서(`af5898a`, v5 형상 diff 전량 확정+B 재분류) → **v5 bump**(형상+REGEN, 기능 0 — `e7f08e8`) → (A) NBA transport delay(`delayed_nba` wheel, 차분 4레인 — `1617980`) → (B) named-event 카운터 desugar(sim-ir 0, 차분 3레인, `.vu` 재핀 — `0a39dec`). ⭐동시-틱 tie(Active $finish vs due-update/edge-wake)는 도구-발산 영역 — 차분 디자인은 tie를 회피하고 주석으로 핀. **다음 후보:** **(C) 엔진 증분 — ③dyn array 3a(heap/new/size/delete) ✅ 2026-06-11(`33e741a`, 677 green: `dyn_heap`+`DynObj`+`NetReader::dyn_size` 시임, W4020 warn-once, VCD 핸들 skip, hand-built SimIr 시임 테스트 — 문법은 ⑥) → 3b(인덱스 r/w 라우팅) ✅ 2026-06-11(`dyn_is_handle` 비트맵 깔때기 라우팅, OOB/X=원소폭 X·무시+W4020, 680 green) → ④queue ✅ 2026-06-11(691 green: `DynObj::Queue{VecDeque}` — push=SysTask 디스패치(원소형 §5.5 cast·cap warn+drop), 인덱스 r/w=3b 깔때기 공유+**`q[size()]`=push_back 동등 append(IEEE §7.10.1, iverilog 라이브로 설계문서 "무시" 가정 정정)**, **pop=`StmtEffect::QPop` 문장-레벨 인터셉트**(P7a read-phase 순수성 유지, Kernel 2메서드)+pop-rhs 바디 `is_codegen_able` 제외(VM=interp fallback, teeth는 parity 테스트로 입증), 비지원 배치 pop=eval arm X+W4020, pop SelfWidth=원소형(signed/unsigned 확장 차분), `q[$]`=DynSize-1 desugar 시임 테스트, empty pop=X+warn-once) → ⑤assoc ✅ 2026-06-11(701 green: `DynObj::Assoc{BTreeMap<i64,Value>}` — **키 도메인=signed i64 전역**(음수·>u32 합법)이라 u32 offsets 쌍에 못 실림 → `Offsets::AssocKey(Option<i64>)` variant 신설+`k_write_lvalue` ABI를 slice→`&Offsets`로(NBA·CA·VM·QPop 전 깔때기 공유=by-construction), READ는 eval Signal arm이 u32 coercion **전에** `is_assoc` 분기→`assoc_key`(64-bit 부호확장, X/Z·real=invalid)→`assoc_read`; exists/num=순수 eval arm(VM parity 자동)·delete(k)=SysTask 디스패치(미존재 키=무음 no-op §7.9, X키=W4020)·delete()=DynDelete 공용; X키 r/w=X/무시+W4020, 미존재 read=X+warn, write=원소 생성(§7.8.6), cap 1<<24 warn+drop; **native-eval LoadIndexed에 Assoc bail**(u32 도메인 발산 차단); concat-lvalue 내 assoc chunk=loud degrade. **iverilog 13.0이 assoc 선언 자체를 거부(라이브 확인)** → 유일하게 hand-IEEE 핀 레인(§7.8/§7.9, expression-force 선례)) → **⑥front-end 일괄 ✅ 2026-06-11(722 green, .vu 해시 재핀 1회)**: (C) 문법(lexer `$`·`Dim` 3변형·`ExprKind::{New,Dollar}`·메서드=기존 Call AST 재사용·핸들 decl/인덱스/메서드/특수형 elaborate 인터셉트·오용 전부 E3009 loud — dyn/queue=iverilog 차분 13 e2e, assoc=hand-IEEE) + (D) interface(스파이크 그대로: ModuleDecl 재사용+Modport+AnsiPort.iface, **nets 단계 4c 조기 평탄화**(부모 body `i.sig` 해소), 포트=심볼 aliasing(net/cont-assign 0), resolve_net 다중-세그 dot-join, modport 존재검증 — **iverilog가 interface port도 거부 → hand-IEEE 핀**, e2e 8). SimIr 무변경(format_version 5 유지). 잔여 = **(D) modport 방향 강제·interface 파라미터·generate-내 iface 포트 전달(후속 증분)** · ~~⑨P4-T2~~ ❎ 2026-06-11 측정 폐기(parse ~10ms 비병목+단일-CU concat 의미론 — §P4 표) · §A 잔여(저빈도 value-op word化 등 저ROI).
3. **Phase-1.x 기능** — ~~`-Wno-*`/`-Werror=` 게이트 + exit class 2~~ ✅ 2026-06-10 `791cca4`(vita-log GatePolicy/GatedSink; 승격 실패=class 1·산출물 미생성, 아티팩트 게이트=exit 2) · ~~filelist `-f`/`-F`~~ ✅ `eedd486`(argv-레벨 전개 v1 서브셋; 잔여=+incdir+/+define+ 버킷·WRONG-STAGE·OVERRIDE) · ~~`vita explain`~~ ✅ `2ca8949` · ~~런타임 delay~~ ✅ **format_version 4 bump**(Delay.amount=ExprId, 평가·×M·round는 엔진 suspension-time; X/Z→0 iverilog parity) · ~~`$dumpflush/$dumplimit`~~ ✅ (bump 무임승차, vcd-writer 기계는 기존재) · ~~force/release 실semantics~~ ✅ 2026-06-10 — per-net `forced` 플래그가 write_chunk 깔때기에서 전 일반 경로(절차/NBA/settle/delayed-ca) 차단, release=net settle-복원/var 값-유지 비대칭, whole-net 타깃만(bit-select=loud). **같은 날 후속: expression force는 IEEE §9.3.2 연속 재평가로 승급**(`active_forces` 레지스트리 — const-rhs는 iverilog 차분 유지, expression lane은 iverilog가 자인한 비준수 영역이라 hand-IEEE 핀). **Phase-1.x 전 항목 소진.**

## 아카이브 (완결 이력 요약)

2026-06-05 6축 감사 52항목(BLOCKER 3: timescale 전체 모델 · `**` const-eval · VCD 계층/실명 — 전부 해결) + 후속 큐 5 + Stage A 릴리스 문서 + **Stage B** 컴파일드 백엔드 선결 11/11 + **Stage C** C1·C2 바이트코드 VM(byte동일·P5 차분 게이트) + profile-driven perf 4R(eval-heavy 2781→461ms ≈ **6x**) + **C4-lite native-eval**(식-바운드 VM ≈2.3x) + C7 혼합-timescale postponed 버그(`fbb869c`) + 멀티-top 다중 root(`148116b`) — **전부 완결**. 상세 시계열: 이 파일 git 이력(HEAD `b3651fa` 시점) · perf = [doc-18 §실측](preview/18-acceleration-analysis.md) · 결정 근거 = [ROADMAP](ROADMAP.md) §0·§3.
