# vitamin — 잔여 작업 트래커 (Remaining Work)

> **리뉴얼: 2026-07-02** (사용자 지시 재계획) · 기준 갱신 2026-07-03 = **format_version 19 · 2921 tests green · 3-OS CI green · MsgCode 58** · known **silent-wrong = §C 잔여(string 함수 formal·㊁ inline signed+unsigned·㊂ md-packed frame-local — 전부 INLINE/frame 경로 deep·다음 우선순위 ①=전용 슬라이스)**, 나머지 잔여는 honest-loud=안전. 최신 = **2026-07-03 EXT2-F gen-for step `g++`/`op=` ✅**(§4.5.79·desugar 재사용·byte-identical·적대 R1 CLEAN) · ㊀ narrow-actual INLINE 인자 width-extend ✅(§4.5.78) · EXT2-C tf-port ✅(§4.5.77) · EXT2-E/A/0(§4.5.76/75/74) · 외부 리포트 2차 EXT2 체인(§A′·ROADMAP §6-2). **G2 OBS 트랙**: OBS-1a ✅ · OBS-1b는 EXT2 체인 다음.
> 이 파일 = **"goal까지 남은 것" 상위 스냅샷** — 재계획 시점마다 통째로 갱신한다. 슬라이스 단위 라이브 기록 = [ROADMAP](ROADMAP.md) §2(착수 순서)·§4.5.x(슬라이스 로그)·§6(외부 리포트)·§7(OBS), 실행 큐 = `LOOPROMPT.md` NEXT. 2026-06-16 이전 P0~P5/Phase-A/B 상세 이력은 이 파일의 **git 이력**과 [DEVLOG](DEVLOG.md)가 보존(여기서 삭제).
>
> **최종 목표**: **G1** = icarus·verilator·xcelium·vcs급 *정확한* 오픈소스 RTL 시뮬레이터(correct-or-loud) · **G2**(2026-07-02 추가) = **AI-Agent 친화 simulator**(SPEC=[preview/19](preview/19-ai-agent-observability.md)).

## A. ~~최우선 — A2 체인~~ — 🏁 **전 체인 완료(2026-07-03) · §6 외부 리포트 완전 종결**

| # | 항목 | 내용 | 오라클 |
|---|---|---|---|
| A2a ✅ | module-body array parameter | **완료(2026-07-02, ROADMAP §4.5.69, 2799 green)** — 파서 desugar→const 변수-배열·deny 13사이트·scope-gate(generate/interface/port)·적대 4-round CLEAN | hand-IEEE + 내부 차분 ✓ |
| A2b-prereq ✅ | package-level 변수 저장 | **완료(2026-07-03, ROADMAP §4.5.70, 2823 green)** — `$pkg$` 예약 스코프 단일 인스턴스·const-init(비상수=loud v1)·alias 접근(`p::x`+import)·VCD 제외+W4026·MVP-CUT 해제·적대 2-round(F1/F2·S1~S4 수정→R2 CLEAN) | iverilog ✓ 라이브 차분 |
| A2b ✅ | package-level array parameter | **완료(2026-07-03, ROADMAP §4.5.71, 2835 green)** — package §6.8 pre-sweep(collect+flush·ProcId 최선두)·const_param 해제·비상수 init(㉺) 동시 해소·**RC_TABLE[0:23] acceptance PASS → §6 CLOSE**·적대 2-round(hier-ref 가드 수정) | twin 내부차분 + var형 iverilog ✓ |

## A′. 최우선 — 외부 리포트 2차 EXT2 체인 (2026-07-03 수신 · 정본=ROADMAP §6-2)

> 동일 외부 사용자의 **실-RTL 18파일 재평가**(1차 5갭 FIX 확인 후). 전건 HEAD `0ef0eec` 라이브 재현 완료 + 리포트 §5/§7 stale 주장 11건 판별(string/queue/class/SVA/`$urandom`/`$sformatf`/`$fscanf`/plusargs-in-if/sigpipe 등 = 전부 동작, §6-2 정정표). 순서 = silent-wrong 먼저, 이후 리포트 영향순.

| # | ID | 항목 | 단계/오라클 | 공수 |
|---|---|---|---|---|
| 1 ✅ | EXT2-0 | `$fgets` `string` dest silent no-op(0 반환·빈 문자열 — **유일 silent-wrong**) — **완료 2026-07-03(ROADMAP §4.5.74, 2867 green)**: `NetKind::String` 브랜치+C-string NUL 절단·적대 2-round CONVERGE(embedded-NUL)→재검 CLEAN | engine·IR-0 / iverilog ✓ | S |
| 2 ✅ | EXT2-A | labeled end `endmodule : m` 계열 전수(외부 14/18 파일 차단) — **완료 2026-07-03(§4.5.75, 2874 green)**: 컨테이너 4종+UDP `opt_block_label` 1줄씩·AST 무변경·적대 2 CLEAN. follow-on=mismatch 통일 loud 진단 | parser / iverilog ✓ | S |
| 3 ✅ | EXT2-E | scoped type `pkg::t` in type positions(port·body·fn·struct member·chained) — **완료 2026-07-03(§4.5.76, 2893 green)**: 퍼널 scope-aware+`"pkg::t"` twin(cross-pkg collision-safe·kind+freshness gating)·적대 R2 CONVERGE(mixed-kind→enum-arm)→CLEAN. follow-on=scoped cast/`$bits`/param loud | parser / iverilog ✓ | S-M |
| 4 ✅ | EXT2-C | packed struct/union typedef tf-port — **완료 2026-07-03(§4.5.77, 2902 green)**: `try_tf_port_typedef` struct 수용+`bind_tf_port_struct`+snapshot/restore 스코프·적대 R1 CLEAN. md-packed tf-port=defer(md frame-local 선행)·발굴 3 silent-wrong=§C | parser+bind / iverilog ✓ | M |
| 5 | EXT2-D | block-local `automatic` 선언(§D MVP-CUT form2서 승격) | parser+lifetime / hand-IEEE(iverilog sorry) | M |
| 6 | EXT2-B | function-body `typedef enum`(v1-cut 해제) | parser / iverilog 확인 | S-M |
| 7 ✅ | EXT2-F | generate-for `++/--`/`op=` step(+prefix) — **완료 2026-07-03(§4.5.79, 2921 green)**: `parse_gen_assign(is_step)` desugar 재사용·byte-identical·적대 R1 CLEAN | parser / iverilog ✓ | S |
| 8 | EXT2-H | frame fn part-select/array-element 대입(frame-call subset 확장) | elaborate / iverilog ✓ | M |
| 9 | EXT2-A2c | packed multi-dim param(리포트 R2b·외부 미사용) | parser / hand-IEEE+내부차분 | S-M |
| 10 | EXT2-DOC | 문서 stale(CLI-ref·lang-ref·system-tasks·explain — 외부 2회 보고) | docs | S |
| 11 | EXT2-NAP | named AP `'{k:v}`(외부 0회·저우선) | parser / iverilog 확인 | S-M |

## B. G2 — OBS 트랙 (AI-Agent 친화 · ROADMAP §7 / SPEC preview/19)

| 단계 | 산출물 | 공수 |
|---|---|---|
| OBS-0 ✅ | 스펙/계약(envelope·명시폭 hex·enum 문자열·결정성 게이트·loud 관찰) | 완료 2026-07-02 |
| OBS-1 | `--obs-dir` → run.json+results.jsonl+coverage.json (MVP) | S-M |
| OBS-2 | `--probe` → trace.jsonl(transition-only) + sva.jsonl(support-cone v0) | M |
| OBS-3 | `$vita_stage` → stage.jsonl (golden stage-diff 훅) | M |
| OBS-4 | `--control stdio` JSON-RPC(peek/poke/step/run_until)+poke 저널 replay | L |
| OBS-5 | snapshot/restore/rewind | L-XL |
| OBS-6 | X-origin·region-annotated events·정적 backward cone | L+ |

## C. In-scope SV 잔여 (대부분 honest-loud=안전 · ROADMAP §4.5.2)

| 항목 | 잔여 내용 |
|---|---|
| **🔴 silent-wrong 잔여**(EXT2-C 적대리뷰 발굴·pre-existing·main 재현·**다음 우선순위 ①**) | ~~㊀ narrow-actual INLINE 함수 인자 width-extend~~ **✅ 수정(§4.5.78)** · ㊁ **INLINE(static) 함수 body context-width propagation 부재**(그라운딩 2026-07-03·전용 슬라이스): `f=s+u; f(-1,1)`=`0`(iverilog `256`)·`f=c<<4; f(8'hAB)`=`00b0`(iverilog `0ab0`)=**widening context-sensitive op(signed+unsigned·shift·carry-add) 전부**·module scope와 **frame(automatic) 경로는 정상**(a1/a4/a5=256/0ab0). 근본=`reduce_function_body`/`fold_straight_line`이 assign RHS를 SELF-width(`lower_ctx_or_plain`=fill만 grow)로 lower 후 resize=context 확장 유실. **수정 계획**=miscomputing static fn을 **frame set에 라우팅**(build_frame_set에 `ret_two_state` 옆 `body_needs_context_width` 추가·frame path 재사용=검증됨). 필요: AST self-width 헬퍼 신설(없음) 후 widening(lhs폭>rhs self폭 & context-sensitive) 정밀 검출=golden churn/잔여 silent 방지 · **string 함수 formal 광범위 silent-wrong**(그라운딩 2026-07-03·전용 슬라이스): ⓐ INLINE(static)+string VAR 인자=정상(review F2) · ⓑ INLINE+string LITERAL 인자=**packed 비교 오답**(`strlt("ab","b")`=0 vs 1·`ir_expr_is_string`이 literal을 packed 취급·둘 다 literal일 때만·한쪽 var면 정상) · ⓒ **FRAME(automatic) 함수+string 인자(var/literal 무관)=오답**(a_var=0 vs 1·`build_frame_set`이 automatic 전부 frame화·frame body string formal 처리 미비=별도 경로). 수정=binding서 literal→string-net coercion + expr_is_string_ast formal-type 추적, 양 경로(inline/frame)·string 라우팅 delicate(F2 이력) · ㊂ **md-packed frame-LOCAL element-select**: 함수 body `logic[1:0][7:0] p; p[0]`이 bit-select 오계산→`01`(iverilog `cd`)·module scope 정상 |
| cast | class **down-cast** `Derived'(base)`(=`$cast` 런타임 타입가드 선행 필요)·real→longint |
| SVA | empty-match `##0`/unbounded `##[m:$]` 융합(§16.9.2.1·오라클 부재)·N2c full(중첩 attempt=L급)·later-antecedent read·advanced prop-ref skew(2-cycle/중첩/cross-clock)·SVA-QUAD default-flip(full-VCD audit 선행). **2차 외부리포트의 "SVA deferred" 주장=stale**(2026-07-03 라이브 검증: `assert property` 동작·§6-2 정정표) |
| N4 clocking 잔여 | non-`#1step` skew·INOUT·multi-event-list clock·non-net bind·hier input drive·cross-hier `@(inst.cb)` |
| file-I/O 소형 | `$fflush` accept·`$fmonitor`/`$fstrobe`·STDIN read(결정성 설계 필요) · ~~`$fgets` silent no-op~~ **✅ 종결(§4.5.74 EXT2-0: `string` dest 브랜치·NUL 절단)** |
| 소형 슬라이스 큐 | 계단식 CA 체인 t0 전파 그라운딩 · 계층 함수호출 `u1.f(x)` · compound-const `==?` fold · `%-` 좌측정렬 family · loud-message 품질 2건(`[bit]` 캐스케이드·typedef-키 메시지) |
| **package 발굴 pre-existing**(§4.5.70 ㉶~㉼·§4.5.71 ㉽) | `always @(*)` decl-init wake(iverilog 미발화·vita-wide) · param override 비상수=W3056 warn+default(iverilog=error) · `@(p::x)` event control(iverilog ✓) · longint MIN fold 갭(package만) · ~~비상수 package init~~ ✅(§4.5.71서 해소) · `p::arr[i]` scoped element read(iverilog ✓) · generate-block 내 `import`(iverilog ✓) · package 자기-func init 호출(㉽, iverilog ✓·현 loud+진단 부정확) |
| **A2a 발굴 pre-existing**(§4.5.69 ㉮~㉵) | ~~generate/interface 스코프 배열 decl-init 영구 silent-drop~~ **✅ 종결(§4.5.72)** · 크로스스코프 t0 decl-init race(㉯·iverilog와 값 divergent이나 **둘 다 §6.8 합법·self-consistent**·문서 핀=골든 리스크로 defer) · SYS-READ hier-element dest 실지원(iverilog ✓·현 honest-loud) · hier-write sentinel cont_assigns/out_binds 미패치 panic · scalar `int unsigned` param 부호(iverilog ✓) · repl-count 변수→0 · assoc 배열-key/clocking 배열-output word0 · typedef-요소 param 진단 · **queue/dyn/string decl-init in gen/iface**(iverilog 일부 ✓·현 loud follow-on) · generate-case 스코프 이름 `gcase[0].x` E3010(gen-if는 정상) |
| deep 잔여(저우선) | inline body NON-fill context-width·runtime `==?` pattern·string queue·block-local queue decl·modport 방향 강제·force part-select |

## D. 별도 관리 — 재진입 트리거 충족 시에만 승격 (정확성과 직교 · ROADMAP §조건부 13~17)

| id | 항목 | 트리거 |
|---|---|---|
| BACKEND | ① cycle-based 컴파일드(Verilator급 throughput) ② PDES BSP 병렬(Amdahl 상한 T4≈2.5x) ③ native-eval 잔여 lane | ①대형 RTL 실수요 ②지속 W≥64+grain≥200ns ③저-ROI 상시 defer |
| VHDL | VHDL 프론트엔드(9-value std_logic 매핑·별도 파서·GHDL 오라클·E7xxx) | SV plateau + 값도메인 결정 + GHDL 셋업 |
| VCD-EXT | `$dumpports*`·FST | 파형 툴 수요 |
| MVP-CUT | string concat-nonassign·wildcard assoc `[*]`·package var/import/scoped-call(→**A2b-prereq ✅ 해제됨**)·cross-frame disable·block-local `automatic` form2(→**EXT2-D로 승격**, §A′) | 개별 수요 시 |

## E. 비계획 — 영구 비목표 (gap 아님 · ROADMAP §비계획 18~20)

- **DEFPARAM**(IEEE deprecated·`#(.param())`로 충분) · **IMPLICIT-NET**(정책=E3010 명시 에러) · **OOS**(synthesis·waveform GUI·UPF/SDF/DPI-C·shortreal·trireg·UVM 생태계·unique/priority 다중-match 검사).
