# vitamin — 잔여 작업 트래커 (Remaining Work)

> **"goal까지 남은 것"의 상위 스냅샷.** 재계획 시점마다 통째로 갱신한다(과거 판본은 git 이력이 보존).
>
> - **기준(2026-07-19)**: format_version **22** · **3636 tests green** · 3-OS CI green · MsgCode 58 · **MSRV 1.85** · 최신 완료 §4.5.158(enum label operand signedness — 선언 sign 상속·全 3 label-site·`.vu` re-pin). 미착수 DEEP=size-cast `N'(expr)` context width 미전파(ROADMAP §2).
> - 잔여 상세 목록(정본) = [ROADMAP.md](ROADMAP.md) · 완료 상세 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존) · 이력 = [DEVLOG.md](DEVLOG.md) · 실행 큐 = `LOOPROMPT.md` NEXT.
> - **최종 목표**: **G1** = icarus·verilator·xcelium·vcs급 *정확한* RTL 시뮬레이터(correct-or-loud) · **G2** = AI-Agent 친화 simulator(SPEC=[preview/19](preview/19-ai-agent-observability.md)).

## A. 현재 상태 한 줄 요약

- **oracle-backed ① silent-wrong = 소진**(발굴 즉시 수정 체제). known silent-wrong 잔여는 전부 **pre-existing·baseline 동일·deep-defer 기록됨**(ROADMAP §2).
- 외부 리포트 1·2차(EXT2)·round 3~11 = **사실상 완결**(잔여 3건=A2c·NAP·DOC, 전부 no-oracle/docs).
- 나머지 잔여는 **honest-loud=안전**(ROADMAP §3~§5) + **G2 OBS 트랙**(ROADMAP §6).

## B. 다음 착수 후보 (우선순위순)

| # | 항목 | 근거/오라클 |
|---|---|---|
| 1 | 신규 CRITICAL silent-wrong 발굴분(fresh-area probe) | iverilog 라이브 차분 |
| 2 | 소형 loud→supported 큐(ROADMAP §3) | iverilog ✓ 대부분 |
| 3 | OBS-2 sva.jsonl(R-L6) 또는 OBS-1 잔여(staged obs·`--seed`) | 3-way 내부 차분 |
| 4 | DEEP-defer 재개(%c/%s UTF-8 pipeline·derived-localparam self-width·top-level typedef ②) | 전용 인프라 슬라이스 |

## C. 잔여 분류 (요약 — 상세=ROADMAP 해당 §)

| 분류 | 내용 | 정본 |
|---|---|---|
| 🔴 silent-wrong 잔여 | DEEP 5(UTF-8 pipeline·derived-param width·$unit typedef·enclosing-const·packed-WIDTH sibling) + 중형 ~15(enum·sign-loss·repl-count·residual sub-select·param-scope·iface-2D·$dist_*·string-concat-width 등) — 전부 pre-existing·기록됨 | ROADMAP §2 |
| honest-loud 잔여 | string/heap·함수/formal·소형 큐·EXT2 3건·deep 저우선 | ROADMAP §3 |
| SVA/검증 잔여 | empty-match 융합·N2c full·prop-ref skew 고급형·QUAD default-flip·N4 clocking 잔여·class down-cast | ROADMAP §4 |
| perf/하드닝 | 전부 보류 판정(SVA-QUAD flip·FMT-CACHE b·GEN-3X-STR a·QUEUE-MID) | ROADMAP §5 |
| G2 OBS | sva.jsonl·staged obs·R-L4·control API·snapshot·X-origin | ROADMAP §6 |

## D. 별도 관리 — 트리거 충족 시에만 승격 (정확성과 직교)

| id | 항목 | 트리거 |
|---|---|---|
| BACKEND | cycle-based 컴파일드 · PDES BSP(T4≈2.5x 상한) · native-eval 잔여 lane | 대형 RTL 실수요 · W≥64+grain≥200ns · 저ROI defer |
| VHDL | VHDL 프론트엔드(GHDL 오라클·E7xxx) | SV plateau + 값도메인 결정 + GHDL 셋업 |
| VCD-EXT | `$dumpports*`(포트 strength) | 파형 툴 수요 (FST=**§4.5.149·150 지원**; known-edge=소형 fst-writer issue #4) |
| MVP-CUT | string concat-nonassign·wildcard assoc `[*]`·cross-frame disable 등 | 개별 수요 시 |

## E. 비계획 — 영구 비목표 (gap 아님)

- **DEFPARAM**(deprecated) · **IMPLICIT-NET**(정책=E3010) · **OOS**(synthesis·waveform GUI·UPF/SDF/DPI-C·shortreal·trireg·UVM·unique/priority 다중-match).
