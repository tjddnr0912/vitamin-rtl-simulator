# vitamin — 잔여 작업 트래커 (Remaining Work)

> **리뉴얼: 2026-07-02** (사용자 지시 재계획) · 기준 = **format_version 19 · 2794 tests green · 3-OS CI green · MsgCode 57** · known **silent-wrong 0**(잔여는 전부 honest-loud=안전 또는 신규 기능).
> 이 파일 = **"goal까지 남은 것" 상위 스냅샷** — 재계획 시점마다 통째로 갱신한다. 슬라이스 단위 라이브 기록 = [ROADMAP](ROADMAP.md) §2(착수 순서)·§4.5.x(슬라이스 로그)·§6(외부 리포트)·§7(OBS), 실행 큐 = `LOOPROMPT.md` NEXT. 2026-06-16 이전 P0~P5/Phase-A/B 상세 이력은 이 파일의 **git 이력**과 [DEVLOG](DEVLOG.md)가 보존(여기서 삭제).
>
> **최종 목표**: **G1** = icarus·verilator·xcelium·vcs급 *정확한* 오픈소스 RTL 시뮬레이터(correct-or-loud) · **G2**(2026-07-02 추가) = **AI-Agent 친화 simulator**(SPEC=[preview/19](preview/19-ai-agent-observability.md)).

## A. 최우선 — A2 체인 (외부 xcelium 리포트 §6 잔여 1건 · 2026-07-02 승격)

| # | 항목 | 내용 | 오라클 |
|---|---|---|---|
| A2a ✅ | module-body array parameter | **완료(2026-07-02, ROADMAP §4.5.69, 2799 green)** — 파서 desugar→const 변수-배열·deny 13사이트·scope-gate(generate/interface/port)·적대 4-round CLEAN | hand-IEEE + 내부 차분 ✓ |
| A2b-prereq | package-level 변수/집합-상수 저장 | 현재 E3009 "(v7)" loud → 단일 인스턴스 lowering(예약 scope NetVar·format 불변)·t0-이전 init·`pkg::x`+import 해소·MVP-CUT package-var 동시 해제 | **iverilog ✓**(2026-07-02 그라운딩: package var 지원 확인=라이브 차분) |
| A2b | package-level array parameter | A2a+prereq 결합 · acceptance=sha3_pkg `RC_TABLE[0:23]` repro → **§6 리포트 CLOSE** | hand-IEEE + 내부 차분 |

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

## C. In-scope SV 잔여 (전부 honest-loud=안전 · ROADMAP §4.5.2)

| 항목 | 잔여 내용 |
|---|---|
| cast | class **down-cast** `Derived'(base)`(=`$cast` 런타임 타입가드 선행 필요)·real→longint |
| SVA | empty-match `##0`/unbounded `##[m:$]` 융합(§16.9.2.1·오라클 부재)·N2c full(중첩 attempt=L급)·later-antecedent read·advanced prop-ref skew(2-cycle/중첩/cross-clock)·SVA-QUAD default-flip(full-VCD audit 선행) |
| N4 clocking 잔여 | non-`#1step` skew·INOUT·multi-event-list clock·non-net bind·hier input drive·cross-hier `@(inst.cb)` |
| file-I/O 소형 | `$fflush` accept·`$fmonitor`/`$fstrobe`·STDIN read(결정성 설계 필요) |
| 소형 슬라이스 큐 | 계단식 CA 체인 t0 전파 그라운딩 · 계층 함수호출 `u1.f(x)` · compound-const `==?` fold · `%-` 좌측정렬 family · loud-message 품질 2건(`[bit]` 캐스케이드·typedef-키 메시지) |
| **A2a 발굴 pre-existing**(§4.5.69 ㉮~㉵) | **generate/interface 스코프 배열 decl-init 영구 silent-drop**(iverilog ✓·①급) · 크로스모듈 t0 decl-init race(iverilog ✓·ProcId 순서=golden 리스크 M~L) · SYS-READ hier-element dest 실지원(iverilog ✓·현 honest-loud) · hier-write sentinel cont_assigns/out_binds 미패치 panic · scalar `int unsigned` param 부호(iverilog ✓) · repl-count 변수→0 · assoc 배열-key/clocking 배열-output word0 · typedef-요소 param 진단 |
| deep 잔여(저우선) | inline body NON-fill context-width·runtime `==?` pattern·string queue·block-local queue decl·modport 방향 강제·force part-select |

## D. 별도 관리 — 재진입 트리거 충족 시에만 승격 (정확성과 직교 · ROADMAP §조건부 13~17)

| id | 항목 | 트리거 |
|---|---|---|
| BACKEND | ① cycle-based 컴파일드(Verilator급 throughput) ② PDES BSP 병렬(Amdahl 상한 T4≈2.5x) ③ native-eval 잔여 lane | ①대형 RTL 실수요 ②지속 W≥64+grain≥200ns ③저-ROI 상시 defer |
| VHDL | VHDL 프론트엔드(9-value std_logic 매핑·별도 파서·GHDL 오라클·E7xxx) | SV plateau + 값도메인 결정 + GHDL 셋업 |
| VCD-EXT | `$dumpports*`·FST | 파형 툴 수요 |
| MVP-CUT | string concat-nonassign·wildcard assoc `[*]`·package var/import/scoped-call(→**A2b-prereq가 package-var 해제 예정**)·cross-frame disable·block-local `automatic` form2 | 개별 수요 시 |

## E. 비계획 — 영구 비목표 (gap 아님 · ROADMAP §비계획 18~20)

- **DEFPARAM**(IEEE deprecated·`#(.param())`로 충분) · **IMPLICIT-NET**(정책=E3010 명시 에러) · **OOS**(synthesis·waveform GUI·UPF/SDF/DPI-C·shortreal·trireg·UVM 생태계·unique/priority 다중-match 검사).
