# ROADMAP — 잔여 과제 (vitamin)

> **이 문서 = 전방(남은 것)-전용.** 완료 항목의 상세 로그·옛 §번호(§0~§7·§4.5.x) 원문은 전부 [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존)로 이관했다. 이력 내러티브 = [DEVLOG.md](DEVLOG.md), 상위 스냅샷 = [REMAINING_WORK.md](REMAINING_WORK.md), 실행 큐 = `LOOPROMPT.md` NEXT(로컬 dev-meta), SPEC 정본 = `docs/preview/`.
>
> **기준선(2026-07-16)**: format_version **21** · **3548 tests green** · 3-OS CI green · MsgCode 58. 최신 완료 = §4.5.134(N3 이종 힙). 최종 목표 = **G1**(icarus·verilator·xcelium·vcs급 정확성·correct-or-loud) + **G2**(AI-Agent 친화·SPEC=[preview/19](preview/19-ai-agent-observability.md)).
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
- array-of-packed residual sub-select `tm[i][j][m:l]` · residual-2D. §4.5.117 잔여.
- inline-local/generate/task-local param sub-select(`param_range`=module-scope only) · net-vs-param VALUE precedence · gen-scope param value truncation. §4.5.118 잔여.
- 2D-packed iface member element/outer part-select · scalar iface member over-acceptance. §4.5.116 잔여.
- non-uniform `$dist_*` libm draw 발산 · chi_square/t seed LCG. §4.5.126.
- string-array-elem 전원 concat `{s[0],"-",s[1]}` truncate — native-eval static string-width. §4.5.134 발굴.
- `$monitor`에 직접 `$random`/`$urandom` 인자=매 스텝 spurious re-fire(no-oracle: iverilog는 non-simple `$monitor` 인자 자체 거부 "SORRY"·시간함수 3종만 예외). 값 렌더는 정상. §4.5.135 발굴.
- leading-NUL frame string · frame-body 内 SYS-READ(assignment form도). §C/§4.5.122/124. (runtime `$clog2(real)` f64 misread = §4.5.143서 해결)
- 숫자 literal `%s` NUL-byte(§4.5.119). `%d`-of-real **width** 해결(§4.5.142); 잔여=`%d`/`%0d`-of-**non-finite** real(inf/nan)=vita `i64::MAX`(`fmt_dec` 포화) vs iverilog `inf`/`nan`(pre-existing·value edge).
- inline 함수 잔여 4종: global-reader widening 미수혜 · size-cast `16'(a*b)` context · signed `>>>` unsigned-context · inline-call return 미truncate. §4.5.80 잔여.
- hier `@(*)` sensitivity · hier md-packed-nested part-select. §4.5.115/103 잔여.

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
- 음수-LSB 멤버 sub-select 정식 지원(§4.5.114) · md-packed element WRITE nested-lval(§4.5.117).
- method/ctor NAME-default class-scope 해석(§4.5.90) · G4 string-return frame call(§4.5.129).

**소형 큐:**

- gen/iface queue/dyn/string decl-init · generate-case 스코프 이름 `gcase[0].x` · 계층 함수호출 `u1.f(x)`.
- SYS-READ hier-element dest · hier-write sentinel panic→loud · generate-내 `import` · package 자기-func init(㉽) · explicit `import p::t`(TYPE).
- `$fmonitor`/`$fstrobe`(파일 strobe/monitor) — 현재 W3056 skip=**파일출력 silent drop**(non-silent·warned). 지원=**format bump 필요**(`SysTaskId` 변종 ① or 직렬화 사이드카 ②·staged 파리티): `FmtCapture`에 `fd:Option<u32>` 추가(engine-local)+strobe drain을 `file_write` 라우팅·전용 슬라이스. STDIN read(결정성 설계 필요).
- compound-const `==?` fold · param override 비상수(W3056→error) · longint MIN fold(package) · loud-message 품질 2건(`[bit]` 캐스케이드·typedef-키 메시지).

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

> 완료: OBS-0 스펙 · OBS-1a run.json+results.jsonl(§4.5.73) · OBS-1b coverage.json(§4.5.99) · OBS-2 v1 trace.jsonl(§4.5.100) · OBS-3 stage.jsonl(§4.5.101). teeth=3-way 내부 차분(JSONL≡VCD≡`$display`)+결정성 골든. 틀린 로그=silent-wrong과 동급.

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
| VCD-EXT | `$dumpports*` · FST | 파형 툴 수요 |
| MVP-CUT | string concat-nonassign · wildcard assoc `[*]` · package internal-import/scoped-call 잔여 · cross-frame disable | 개별 수요 시 |

## 8. 비계획 (영구 비목표 · gap 아님)

- **DEFPARAM**(IEEE deprecated·`#(.param())`로 충분) · **IMPLICIT-NET**(정책=E3010 명시 에러) · **OOS**(synthesis·waveform GUI·UPF/SDF/DPI-C·shortreal·trireg·UVM 생태계·unique/priority 다중-match 검사).

## 9. 완료 이력 포인터

- 완료 슬라이스 상세 로그(§4.5.3~§4.5.134)·구 §0~§7 원문 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존).
- 탄 단위 내러티브·방법론 교훈 = [DEVLOG.md](DEVLOG.md)·ARCHIVE §3.
- 외부 호환성 리포트 1·2차 전말(A1~C1·EXT2 체인) = ARCHIVE §6·§6-2 — **잔여는 위 §3 "외부 리포트 잔여" 3건뿐**.
