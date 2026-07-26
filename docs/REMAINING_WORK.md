# vitamin — 잔여 작업 트래커 (Remaining Work)

> **"goal까지 남은 것"의 상위 스냅샷.** 재계획 시점마다 통째로 갱신한다(과거 판본은 git 이력이 보존).
>
> - **기준(2026-07-25 · main `1d2d7eb`)**: format_version **23** · **4474 tests green** · 3-OS CI green · MsgCode **59**(W3057) · **MSRV 1.85**.
> - **최신 완료 5건**: §4.5.221 real-valued parameters(`parameter real` — 적대 리뷰 6R·후속 브랜치서 잔여 해소·머지) · §4.5.220 DYN string-array element byte-select silent-0 수정 + write-twin loud화 · §4.5.219 FIXED string-array decl-init · §4.5.218 inner-scope local의 string-array side-map shadow · §4.5.217 string-ARRAY ELEMENT packed 누출. **완료 슬라이스 220건 전체 목록·상세 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)**(인덱스 = 파일 상단, `#### 4.5.<N>` 로 검색).
>
> - 잔여 상세 목록(정본) = [ROADMAP.md](ROADMAP.md) · 완료 상세 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존) · 이력 = [DEVLOG.md](DEVLOG.md) · 실행 큐 = `LOOPROMPT.md` NEXT.
> - **최종 목표**: **G1** = icarus·verilator·xcelium·vcs급 *정확한* RTL 시뮬레이터(correct-or-loud) · **G2** = AI-Agent 친화 simulator(SPEC=[preview/19](preview/19-ai-agent-observability.md)).

## A. 현재 상태 한 줄 요약

- **오라클 있는 열린 silent-wrong = 7건**(ROADMAP §2 상단). 2026-07-23 판이 "소진"이라고 적었던 것은 그 시점 기준이며, §4.5.217~221의 적대 리뷰·PRE 3-way 측정이 pre-existing 7건을 새로 **발굴**했다(악화가 아니라 가시화). 그중 **1건은 pre-existing이 아니다** — §4.5.221이 도입한 좁은 loud→silent 하강(계층 real param 바운드).
- 외부 리포트 1·2차(EXT2)·round 3~19 = **사실상 완결**(잔여 3건=A2c·NAP·DOC, 전부 no-oracle/docs).
- 나머지 잔여는 **honest-loud=안전**(ROADMAP §3~§5) + **G2 OBS 트랙**(ROADMAP §6).

## B. 다음 착수 후보 (현재 큐 — 정본 순서·상세=ROADMAP §1)

| # | 항목 | 근거/오라클 |
|---|---|---|
| 1 | **§0 승격 큐 T1** — string-array 가족 6종(runtime index·`foreach`·`string q[$]`·multi-dim·hier·frame-local) | iverilog ✓ 7/7 재현. **전제조건 해소됨**(§4.5.219+220 → dyn ⊇ fixed) |
| 2 | **§0 승격 큐 T2** — real const-fold · generate/iface string decl-init · sized-literal enum label · 음수 range bound | iverilog ✓ 4/4 |
| 3 | **§2 오라클-有 silent-wrong** — part-select 바운드 silent-0 + replication count silent-0(동근: `const_eval_in_scope` `Cast`/`Call` arm) · package-scope real · 구조적 지연 · real→`input int` formal | iverilog 라이브 차분 |
| 4 | **§2 DEEP** — inner NET vs outer PARAM shadow(선행 = order-INDEPENDENT AST-gathered per-scope name set) | iverilog ✓ |
| 5 | OBS-2 sva.jsonl(R-L6) 또는 OBS-1 잔여(staged obs·`--seed`) | 3-way 내부 차분 |
| 6 | DEEP-defer 재개(%c/%s UTF-8 pipeline·derived-localparam self-width·`$unit` typedef ②) | 전용 인프라 슬라이스 |

> **순서 주의**: 정본 우선순위는 `① 오라클 있는 CRITICAL silent-wrong > ② loud→supported`인데 1·2위(§0=②)가 3위(§2=①) 앞에 있다 — **오너 지시**. §0를 먼저 해도 §2의 ①-급이 사라진 것은 아니다.

## C. 잔여 분류 (요약 — 상세=ROADMAP 해당 §)

| 분류 | 항목 수 | 내용 | 정본 |
|---|---:|---|---|
| correct-support 승격 큐 | 16 | T1 string-array 6 · T2 독립 4 · T3 전제조건 2 · 정정(stale) 3 | ROADMAP §0 |
| 🔴 silent-wrong 잔여 | 39 | **오라클-有 7**(part-select 바운드·replication count·package real·구조적 지연·real→int formal·inner-NET shadow·block-local package clobber) + DEEP 5(UTF-8 pipeline·derived-param width·`$unit` typedef·enclosing-const·packed-WIDTH sibling) + 중형 ~20 + 무오라클 3 | ROADMAP §2 |
| honest-loud 잔여 | 35 | string/heap·함수/formal·소형 큐·EXT2 3건·deep 저우선(VCD fidelity·X→real·x/z-fill param) | ROADMAP §3 |
| SVA/검증 잔여 | 6 | empty-match 융합·N2c full·prop-ref skew 고급형·QUAD default-flip·N4 clocking 잔여·class down-cast | ROADMAP §4 |
| perf/하드닝 | 4 | 전부 보류 판정(SVA-QUAD flip·FMT-CACHE b·GEN-3X-STR a·QUEUE-MID) | ROADMAP §5 |
| G2 OBS | 6단계 | OBS-2 sva.jsonl(M) → OBS-1 잔여(S-M) → R-L4(M) → OBS-4 control API(L) → OBS-5 snapshot(L-XL) → OBS-6(L+) | ROADMAP §6 |

## D. 별도 관리 — 트리거 충족 시에만 승격 (정확성과 직교)

| id | 항목 | 트리거 |
|---|---|---|
| BACKEND | cycle-based 컴파일드 · PDES BSP(T4≈2.5x 상한) · native-eval 잔여 lane | 대형 RTL 실수요 · W≥64+grain≥200ns · 저ROI defer |
| VHDL | VHDL 프론트엔드(GHDL 오라클·E7xxx) | SV plateau + 값도메인 결정 + GHDL 셋업 |
| VCD-EXT | `$dumpports*`(포트 strength) | 파형 툴 수요 (FST=**§4.5.149·150 지원**; known-edge=소형 fst-writer issue #4) |
| MVP-CUT | string concat-nonassign·wildcard assoc `[*]`·cross-frame disable 등 | 개별 수요 시 |

## E. 비계획 — 영구 비목표 (gap 아님)

- **DEFPARAM**(deprecated) · **IMPLICIT-NET**(정책=E3010) · **OOS**(synthesis·waveform GUI·UPF/SDF/DPI-C·shortreal·trireg·UVM·unique/priority 다중-match).
