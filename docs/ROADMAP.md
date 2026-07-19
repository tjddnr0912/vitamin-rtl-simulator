# ROADMAP — 잔여 과제 (vitamin)

> **이 문서 = 전방(남은 것)-전용.** 완료 항목의 상세 로그·옛 §번호(§0~§7·§4.5.x) 원문은 전부 [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존)로 이관했다. 이력 내러티브 = [DEVLOG.md](DEVLOG.md), 상위 스냅샷 = [REMAINING_WORK.md](REMAINING_WORK.md), 실행 큐 = `LOOPROMPT.md` NEXT(로컬 dev-meta), SPEC 정본 = `docs/preview/`.
>
> **기준선(2026-07-19)**: format_version **22** · **3636 tests green** · 3-OS CI green · MsgCode 58 · **MSRV 1.85**(§4.5.150). 최신 완료 = §4.5.158(enum label operand signedness — 선언 sign 상속, silent-wrong 수정·全 3 label-site). 최종 목표 = **G1**(icarus·verilator·xcelium·vcs급 정확성·correct-or-loud) + **G2**(AI-Agent 친화·SPEC=[preview/19](preview/19-ai-agent-observability.md)).
>
> **운용 규칙**: 슬라이스 완료 시 → 상세 로그를 ARCHIVE "완료 슬라이스 로그"에 append(§4.5.x 양식·최신이 위), 이 문서의 해당 잔여 항목 삭제. 신규 발굴은 아래 해당 섹션에 1줄로 추가.

## 1. 착수 우선순위

1. **오라클 있는 CRITICAL silent-wrong** (§2에서 선정) — 항상 최우선.
2. **오라클 있는 loud→supported** (§3 · additive=저위험).
3. **전제조건 충족된 honest-loud 승격** (§4~§5).
4. **G2 OBS 슬라이스** (§6).

현재 NEXT 큐(상세=LOOPROMPT): 소형 loud→supported 큐 소진 → OBS-2 sva.jsonl 또는 deep-defer 재개.

## 2. Silent-wrong 잔여 (전부 pre-existing·baseline 동일 — deep defer 또는 기록됨)

> 발굴 경위·재현·범위 상세는 ARCHIVE의 해당 §4.5.x 참조.

**🔴 DEEP-defer (전용 인프라 필요):**

- `%c`/`%s` high-byte(128-255) UTF-8 remangle — output-pipeline 전체 byte-clean 필요(diag `RtlText`→`Vec<u8>`·~8 test sink·CLI·OBS). §4.5.119/128 발굴.
- derived-localparam self-width (`localparam P=A+B`·`$bits(Q)`=32 등 expression-init 폭) — `param_meta` pollution+allow-list 인프라. §4.5.124 blocker.
- top-level(`$unit`) typedef ② — flat map이 §26.3 scope-precedence 미모델(wildcard-import가 same-name `$unit` typedef 미shadow). 2회 revert. §4.5.104.
- enclosing-const-over-inner-block-local resolution(§6.21 역행) · static-coalesce-onto-automatic. §4.5.108 발굴.
- packed-WIDTH sibling-block 멤버 coalesce(SW2) — read-gate 불가(SOMETIMES-correct)·per-block-scope width 필요. §4.5.98 재분류.

**중형 (오라클 확보 시 착수 후보):**

- dual-wildcard import type ambiguity(둘 다 silent·loud화 후보). §4.5.104.
- mixed-sign enum 산술 · `enum bit` 2-state-base X-leak(일반 enum-base-kind 갭). §4.5.109/110.
- `$signed`-in-wider-sum sign-loss · size-cast fn-call operand sign-loss `4'(f(15))`. §4.5.111/112.
- packed bit/part-select replication count · hierarchical `{s.CNT[0]{v}}` count silent-0. §4.5.109/121 잔여.
- inline-local/generate/task-local param sub-select(`param_range`=module-scope only) · net-vs-param VALUE precedence · gen-scope param value truncation. §4.5.118 잔여.
- 2D-packed iface member element/outer part-select · scalar iface member over-acceptance. §4.5.116 잔여.
- non-uniform `$dist_*` libm draw 발산 · chi_square/t seed LCG. §4.5.126.
- string-array-elem 전원 concat `{s[0],"-",s[1]}` truncate — native-eval static string-width. §4.5.134 발굴.
- `$monitor`에 직접 `$random`/`$urandom` 인자=매 스텝 spurious re-fire(no-oracle: iverilog는 non-simple `$monitor` 인자 자체 거부 "SORRY"·시간함수 3종만 예외). 값 렌더는 정상. §4.5.135 발굴.
- leading-NUL frame string · frame-body 内 SYS-READ(assignment form도). §C/§4.5.122/124. (runtime `$clog2(real)` f64 misread = §4.5.143서 해결)
- format 출력 잔여(§4.5.144 후·전부 pre-existing): real-const `%s`=vita packed f64 bytes vs iverilog warn+`<%s>` · string-LITERAL embedded-NUL(const_string NUL-리터럴+lexer octal-escape `\000`) · render_template malformed spec(missing-arg·`%`+non-spec char)=vita `x`/literal vs iverilog `<…>`+warn(literal+var 공통) · `%d`/`%0d`-of-non-finite real(inf/nan)=vita `i64::MAX` vs `inf`/`nan`. (숫자-const `%s` NUL→space·`%d`-of-real width=§4.5.144/142 해결)
- inline 함수 잔여 4종: global-reader widening 미수혜 · size-cast `16'(a*b)` context · signed `>>>` unsigned-context · inline-call return 미truncate. §4.5.80 잔여.
- hier `@(*)` sensitivity · hier md-packed-nested part-select. §4.5.115/103 잔여.
- narrow-typed(`bit`/`logic`) param init from a 32-bit-self-width expr(comparison·산술)=선언폭 미적용→32-bit width→`%b` wide(value 정확·`%0d` 정상). pre-existing(plain `==`도)·§4.5.146 발굴. const_eval i64→param에 declared-narrow truncate 필요.
- param scalar bit/part-select const-fold **미지원**(silent-wrong·DEEP): `logic [P[5:0]-1:0] x`=range-bound 폭 1 vs iverilog 63(param 값 컨텍스트는 E3009 honest-loud). §4.5.148 naive fold `(v>>i)&mask` 시도→**적대 2렌즈 수렴 발굴**: `[N:0]` descending만 정답·**zero-LSB ascending `[0:N]`**(선언 범위 미정규화→wrong bit)+non-zero-LSB below-LSB index=loud→silent 회귀→revert. 근원=`param_range`가 non-zero-LSB만 추적("absent=descending zero-LSB" 불변식·zero-LSB ascending 미탐·`base_net_ascending` false). fix=全 param 범위(lo/msb/direction) 기록 or `[lo..hi]` membership(param_range 불변식 확장=broad).

- enum label **unfoldable-range signed** 잔여(§4.5.158 minor): `enum logic signed [X-1:0]`처럼 base range가 const-fold 안 되면 `enum_base_width`=None→`base_w` None→라벨 sign이 value-inferred(positive 라벨 unsigned)·§4.5.158은 fold되는 base·base-less만 커버. 극한 엣지(unfoldable enum base). fix=`param_range` 불변식 확장 or value-inferred에 enum sign 전파.
- **🔴 size-cast `N'(expr)` context width 미전파 (DEEP·broad·§4.5.155 발굴)**: `5'(a+4'h1)`(a=4'hF)=vita `00000` vs iverilog `10000`(16)·`8'(a*b)`=13 vs 45·`6'(a<<1)`=14 vs 30·`5'(0-1)`=15 vs 31 — size cast가 inner 산술을 self-width로 계산 후 result만 resize(carry 소실). **assignment context(`logic[4:0] x=a+1`=16)는 정상**(엔진 `eval_ctx(rhs, net_w)`가 statement서 ctx 공급). 근본=IR Expr에 expr-level ctx-width 노드 없음(width=structural)·`lower_expr_ctx`/`lower_ctx_or_plain`이 **fill-only** 전파(non-fill leaf=self-width). fix=①새 IR Resize 노드(format bump) ②fill-only ctx 머신을 non-fill leaf까지 확장(26 caller+fill 시맨틱 회귀 위험) ③엔진 width 모델 — 전부 multi-part. recorded "inline size-cast `16'(a*b)` context"(§43)의 module-level 일반화·전용 슬라이스.
- **`$bits(md-array ROW)` partial-index sub-array** = next-pow2 오류(`byte m[2][3]; $bits(m[0])`=vita 32 vs iverilog 24·`int`=128 vs 96) — logic 배열도 동일(atom-independent)·element `m[i][j]`는 정상(§4.5.155). runtime 경로만(const-context는 §4.5.155가 부수 교정). from_prescan partial-index 미처리→from_table next-pow2. pre-existing·§4.5.155 발굴.

**문서화된 divergence (수정 비대상·핀됨):**

- 크로스스코프 t0 decl-init race(양쪽 §6.8 합법·self-consistent) · 런타임 구성 `-0.0` 표시 · iverilog 자인 결함들(expression-force "evaluated once" 등).

## 3. Loud→supported 후보 (현재 전부 loud=안전 · additive)

**string/heap:**

- frame string LOCAL 대입(E3018→heap slot) · substr-actual `s[i]` · string part-select `s[i:j]`.
- dyn string 요소 method(`s[i].len()`) · whole-element read as value(`x=arr[i]`) · record array-of-record.
- queue/assoc의 string·real 요소 · string queue · block-local queue decl.

**함수/패키지/formal:**

- control-flow pkg fn(frame이 처리 가능→relax) · pkg `function string` · pkg TASK statement call. §4.5.111 잔여.
- array formal 재전달(nested/recursion) · non-zero-LSB 원소 · 2-D/non-zero-base/signed/task array formal. §4.5.110 slice 밖.
- 음수-LSB 멤버 sub-select 정식 지원(§4.5.114). md-packed nested part-select WRITE `x[j][m:l]`/`arr[i][j][m:l]`=**§4.5.145 지원**(descending zero-lsb leaf·const packed idx); fail-closed 잔여(전부 loud·honest)=ascending/non-zero-lsb leaf·**genvar-index**(`x[g][m:l]`=const-fold 안 됨·over-reject)·const-OOB packed idx=silent no-op(read path 공유·값 무손상).
- method/ctor NAME-default class-scope 해석(§4.5.90) · G4 string-return frame call(§4.5.129).

**소형 큐:**

- gen/iface queue/dyn/string decl-init · generate-case 스코프 이름 `gcase[0].x` · 계층 함수호출 `u1.f(x)`.
- SYS-READ hier-element dest · hier-write sentinel panic→loud · generate-내 `import` · package 자기-func init(㉽). explicit `import p::t`(TYPE)=**§4.5.148 지원**.
- `$fmonitor`/`$fstrobe`(파일 strobe/monitor) — 현재 W3056 skip=**파일출력 silent drop**(non-silent·warned). 지원=**format bump 필요**(`SysTaskId` 변종 ① or 직렬화 사이드카 ②·staged 파리티): `FmtCapture`에 `fd:Option<u32>` 추가(engine-local)+strobe drain을 `file_write` 라우팅·전용 슬라이스. STDIN read(결정성 설계 필요).
- compound-const `==?` fold=**§4.5.146 지원**(sized 패턴)·잔여 fail-closed loud=unsized x/z 패턴(`'hx` self-width truncation)·negative-signed LHS·non-literal RHS. param override 비상수(W3056→error) · longint MIN fold(package) · loud-message 품질 2건(`[bit]` 캐스케이드·typedef-키 메시지).
- `case (x) inside {…}`(§12.5.4 wildcard case)=vita E2002 parse-reject(loud)·③ 후보(no-oracle: iverilog 13.0 `case inside`/`inside` op/array reduction method 全 거부→hand-IEEE `==?`+내부차분). `inside` operator는 지원(== 시맨틱·§11.4.13). based-literal 내 whitespace(`64'sh FFFF`)=vita lexer reject(loud) vs iverilog 허용(minor·§4.5.147 발굴).
- **enum label 범위검증 부재**(honest-loud 추가 후보): vita가 base 폭/부호를 벗어난 enum label(`enum logic [3:0] {X=-1}`·`{Y=16}`·signed `{Z=8}`)을 **조용히 truncate 수용** vs iverilog는 compile-reject("value too large/negative"). 유효 프로그램엔 무영향(§4.5.153 differential은 이 lenience 확인)·invalid program을 loud화하면 correct-or-loud 강화. 부호축은 §4.5.153서 해결·범위검증은 별개. §4.5.153 발굴.
- **bit63-set unsigned 64-bit param 리터럴**(`parameter logic [63:0] A = 64'hFFFF_0000_0000_0000`)=E3009 over-reject(loud) vs iverilog 수용. §4.5.151 발굴·**보류 사유**=`const_eval_i64_lit`의 naive `v as i64` 수용은 i64 image가 음수로 읽혀 downstream const 부호 비교/산술을 silent-wrong으로 전환할 위험(explicit-signed 64-bit arm은 已수용·비대칭) — param 값 도메인에 부호/폭 메타 배선 후 착수.
- **partial-timescale 정책 진단**(`--timescale-policy`·`W-PARSE-TIMESCALE-PARTIAL`/`E-PP-TIMESCALE-PARTIAL`): 일부 모듈만 `` `timescale `` 선언 시 현재 무진단 1ns/1ns 디폴트(전무 케이스만 W1017). doc-08 §15 설계는 문서화됨·`rt.default_used` 신호 이미 존재 — 배선만 필요. §4.5.151 발굴.

**외부 리포트 잔여 (§6-2 → ARCHIVE · 전부 no-oracle 또는 docs):**

- EXT2-A2c: packed multi-dim param `localparam logic[1:0][7:0] PK=…`(외부 0회 사용·hand-IEEE+내부차분).
- EXT2-NAP: named assignment pattern `'{k:v}`(외부 0회).
- EXT2-DOC: 문서 stale(CLI-ref·lang-ref·system-tasks·explain — 외부 2회 보고).

**deep 잔여(저우선):**

- t0 race 그라운딩(계단식 CA 체인) · `@(*)` decl-init wake · runtime `==?` pattern.
- inline body NON-fill context-width · modport 방향 강제 · force part-select · assoc 배열-key/clocking 배열-output word0.
- **음수 range bound**(`logic [3:-2]`/`[-1:-8]` net·multi-packed inner `[1:0][3:-2]`·unpacked `[-1:2]`) — iverilog=`|msb-lsb|+1`(예: `[3:-2]`=6bit). 현재 net/multi-packed=W3056 warn+clamp-1(**whole-value 손상**)·unpacked=E4002. 전부 non-silent(§4.5.135 후 diag 정직화). 정식 지원=**packed-struct-member 선례 미러**(whole=flat offset 정확+sub-select loud; `struct_field_select.rs`): `range_to_dims` 정확 폭+정규화 base·neg-base 마커로 sub-select loud-guard(u32 dbase→signed 또는 사이드카). 단, `[W-1:0]`-with-W==0 param underflow(lsb≥0)는 graceful width-1 유지(test `v3_12`).
- **VCD 잔여 fidelity**(§4.5.138 range fix 후·전부 **cosmetic·decode 동일**·§4.5.139서 VALUE 검증 완료: x/z·real·wide·readmem·format 全 decoded waveform iverilog 일치). 남은 encoding 차이: ① value 미압축=vita full-width(`bxxxxxxxx`) vs iverilog leading-redundant strip(`bx`·`b0`)—decode 동일·큰 golden churn ② t=0 초기덤프 구조=vita `$dumpvars`에 pre-assign X + `#0` change vs iverilog settled값—final 동일 ③ var-type=logic 절차구동시 vita `wire` vs iverilog `reg`(연속구동=both wire·usage 의존이라 non-trivial)·`int`=`reg` vs `integer` ④ real size `64` vs `1` ⑤ `parameter` 미덤프. + 근본: elaborate packed-md NetVar.lsb stale(lib.rs 8435/7862·VCD helper서 flat fallback 우회).
- **real const-fold 전면 미지원**(§4.5.141 발굴): `localparam/parameter real` = `2.0+3.0`·`*`·`/`·`-`·`**` 全 E3009 "not foldable"(iverilog=folds). `localparam=$clog2(real-lit)`도 동근(const_eval_in_scope=i64-only·real arg→None loud·§4.5.143 런타임은 해결). 런타임 real 산술은 정상(§4.5.141서 `**`도 지원)·const 경로만 uniformly loud. const_eval_in_scope에 real f64 arithmetic 추가 필요(broad·non-silent).
- **X-bearing integral→real 변환 divergence**(§4.5.141 발굴): vita=whole X값→`0.0`(`real_arg`=`to_i128_signed().unwrap_or(0)`) vs iverilog=per-bit X→0(예 `4'bxx01`→1). `$itor`/`$sqrt`/`$pow`/real-`**` 공통·pre-existing. non-silent 아니지만 divergent(impl-defined X→real).
- **width>128 정수→real 변환**=여전히 `0.0`(§4.5.151서 `to_i128_signed`를 128-bit lane까지 확장 — 65..=128 signed/unsigned는 수정 완료·>128만 잔여). 초희귀(129-bit+ 값의 real 대입)·word-grid f64 근사 필요.
- **x/z-fill const param LHS→0**(§4.5.146 발굴): `localparam logic [W] P = 'x`=const_eval가 `fill_to_i64`/`fill_literal_const`로 0 bind(x 소실)→**全 const 연산자 상속**(`P==0`·`P+1`·`P ==? pat` 4-state 결과 divergent). upstream param binding 근원·contrived(all-x const 선언)·broad. §4.5.146 `==?` fold는 sized 패턴만이라 무영향(a=int).

## 4. SVA / 검증 honest-loud 잔여

- empty-match `##0`/unbounded `##[m:$]` 융합(§16.9.2.1 불연속·오라클 부재).
- N2c full sequence local var(중첩 attempt 각자 데이터=L급; 단일-capture ✅).
- later-antecedent read · outer-`|=>` prop-ref skew 고급형(2-cycle·중첩·cross-clock).
- SVA-QUAD collapse default-flip(`VITA_SVA_COLLAPSE` opt-in 상태 — full-VCD 골든 audit 선행).
- N4 clocking 잔여: non-`#1step` skew·INOUT·multi-event-list clock·non-net bind·hier input drive·cross-hier `@(inst.cb)`.
- class: down-cast `Derived'(base)`($cast 런타임 타입가드 선행) · real→longint cast · base-shadow 명시 접근 `Base'(d).v` · cast-as-receiver `(B'(d)).foo()`.

## 5. perf / 하드닝 잔여 (전부 보류 판정 — 트리거 시만)

- SVA-QUAD default-flip = §4 항목과 동일(full-VCD audit 선행).
- FMT-CACHE part b(render_template pre-segment) · GEN-3X-STR part a(unroll plan 캐시=byte-identity 위험>이득) — 저ROI 보류.
- QUEUE-MID-ON: 스펙 내재 O(n)(iverilog 동일) — 영구 비권장·monitor-only.
- 백로그 원문·완료 32건 = ARCHIVE §5.

## 6. G2 — AI-Agent 친화 OBS 트랙 (SPEC=[preview/19](preview/19-ai-agent-observability.md))

> 완료: OBS-0 스펙 · OBS-1a run.json+results.jsonl(§4.5.73) · OBS-1b coverage.json(§4.5.99) · OBS-2 v1 trace.jsonl(§4.5.100) · OBS-3 stage.jsonl(§4.5.101) · OBS-S0 설계구조 export `--hier-tree`/`--inst-paths`(2026-07-13). teeth=3-way 내부 차분(JSONL≡VCD≡`$display`)+결정성 골든. 틀린 로그=silent-wrong과 동급. 값 인코딩 실태(§4.5.151 문서 정합)=trace `old`/`new`=full-width 4-state binary·stage `vals[]`=`%0d` decimal — doc-19 §3 pin 4에 기록.

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
| BACKEND | ① cycle-based 컴파일드(Verilator급) ② PDES BSP 병렬(Amdahl 상한 T4≈2.5x) ③ native-eval 잔여 lane(signed>64·>128bit·sysfunc·real) | ① 대형 RTL 실수요 ② 지속 W≥64+grain≥200ns ③ 저ROI 상시 defer |
| VHDL | VHDL 프론트엔드(9-value std_logic 매핑·별도 파서·GHDL 오라클·E7xxx) | SV plateau + 값도메인 결정 + GHDL 셋업 |
| VCD-EXT | `$dumpports*`(포트 strength) | 파형 툴 수요 (FST=**§4.5.149·150 지원** — `$dumpfile("x.fst")`/`-o x.fst`; known-edge=소형 타임테이블 fst-writer [issue #4] loud 거부, preview/07 참조) |
| MVP-CUT | string concat-nonassign · wildcard assoc `[*]` · package internal-import/scoped-call 잔여 · cross-frame disable | 개별 수요 시 |

## 8. 비계획 (영구 비목표 · gap 아님)

- **DEFPARAM**(IEEE deprecated·`#(.param())`로 충분) · **IMPLICIT-NET**(정책=E3010 명시 에러) · **OOS**(synthesis·waveform GUI·UPF/SDF/DPI-C·shortreal·trireg·UVM 생태계·unique/priority 다중-match 검사).

## 9. 완료 이력 포인터

- 완료 슬라이스 상세 로그(§4.5.3~§4.5.134)·구 §0~§7 원문 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존).
- 탄 단위 내러티브·방법론 교훈 = [DEVLOG.md](DEVLOG.md)·ARCHIVE §3.
- 외부 호환성 리포트 1·2차 전말(A1~C1·EXT2 체인) = ARCHIVE §6·§6-2 — **잔여는 위 §3 "외부 리포트 잔여" 3건뿐**.
