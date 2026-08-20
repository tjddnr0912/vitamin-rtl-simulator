# vitamin — 잔여 작업 트래커 (Remaining Work)

> **"goal까지 남은 것"의 상위 스냅샷.** 재계획 시점마다 통째로 갱신한다(과거 판본은 git 이력이 보존).
>
> - **기준(2026-08-20)**: format_version **29** · **5,679 tests green**(+ 제품 형태 lib green · **`VITA_JIT=1` 로도 전 스위트 green**) · CI **3-OS + `build-no-oracle`** green · MsgCode **68** · **MSRV 1.85** · **기본 백엔드 = `native`**.
> - **최신 완료**: **§4.5.351**(NBA 적용이 이미 지어 둔 flat store 를 한 번도 안 불렀다 — picorv32 **−3.9%** · 바이트 동일 · ⭐ 증명은 스케줄 시점에 이미 있었고 apply 시점에 버려지고 있었다) · 직전 = **§4.5.350**(§2 음수 상수를 먹는 소비자 셋 — replication count·오름차순 음수 range bound·unpacked 배열 크기 · ⭐ 부호는 **자기결정 폭에서만** 존재한다 · 적대 리뷰 BLOCKING = dyn/queue 원소가 넓은 폭만 받고 사이드맵을 건너뛰던 하강) · 직전 = **§4.5.349**(런타임 mixed-real 변환 경계) · **§4.5.348**(지수의 부호). 그 이전 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md) 인덱스.
> - (이전 세대 슬라이스의 상세 서사는 전부 ARCHIVE 로 이관 — 이 문서는 상위 스냅샷만 둔다.)
>
> - 잔여 상세 목록(정본) = [ROADMAP.md](ROADMAP.md) · 완료 상세 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존) · 이력 = [DEVLOG.md](DEVLOG.md) · 실행 큐 = `LOOPROMPT.md` NEXT.
> - **최종 목표**: **G1** = icarus·verilator·xcelium·vcs급 *정확한* RTL 시뮬레이터(correct-or-loud) · **G2** = AI-Agent 친화 simulator(SPEC=[preview/19](preview/19-ai-agent-observability.md)).

## A. 현재 상태 한 줄 요약

> **★★★ 2026-08-17 — Phase A~D 가 전부 끝났다. 다음에 무엇을 할지는 [ROADMAP §5.2 재개 지점](ROADMAP.md) 이 정본이다.** 요약: **A** 커버리지 100.00% · **B** 제품 표면 native 하나 · **C** interp = 테스트 도구 · **D** 성능(벤치 **8/8 에서 native < vm** · 착수 때 최악 2.52×). ⭐⭐ **코드젠은 지어서·배선해서·재서 기각**(런의 ~38% 가 shim · 천장 8.9~11.3% · §5.1-be) ⇒ **성능 축은 수확 체감**이고 **다음 우선순위는 정확성 큐(§2) → loud 승격(§3) → OBS(§6)** 다. ⚠️ 성능을 다시 본다면 **미측정 축은 스케줄러**다(picorv32 비율이 안 움직인 이유는 아직 안 쟀다).
>
- **열린 silent-wrong 의 정본 목록·개수 = ROADMAP §2**(여기 개수를 복제하지 않는다 — 두 번 적으면 하나는 썩는다).
- 외부 리포트 1·2차(EXT2)·round 3~19 = **사실상 완결**(잔여 3건=A2c·NAP·DOC, 전부 no-oracle/docs).
- 나머지 잔여는 **honest-loud=안전**(ROADMAP §3~§5) + **G2 OBS 트랙**(ROADMAP §6).

## B. 다음 착수 후보 (⚠️ **정본은 [ROADMAP §5.2 재개 지점](ROADMAP.md)** — 아래는 그 요약이다)

> ⚠️⚠️ **2026-08-18 정정**: 이 표는 예전에 *"정본 순서·상세 = ROADMAP §1"* 이라고 적혀 있었는데
> **§A 는 §5.2 를 정본이라 적고 있었다** — 포인터가 둘이고 §1 쪽이 썩어 있었다(그 NEXT 0번은
> 2026-08-03 에 완료된 `③층 S1d-4a` 였다). **§1 은 이제 시간 불변 원칙만 갖고, 현재 큐는 §5.2 하나다.**

| # | 항목 | 근거/오라클 |
|---|---|---|
| **1** | **§2 오라클-有 silent-wrong** — ⚠️ **§2 를 위에서부터 읽지 마라**(맨 위 뭉치는 *AST self-폭 패스* 선행조건에 막혀 있다). 착수표 = §2 머리말의 「다음 착수 순서」(2026-08-20 현재: ① net 선언 초기화의 fill 리터럴 폭 ② mixed-real 잔여 셋 ③ 오름차순 음수 bound 의 남은 스코프 집합) | iverilog + verilator 라이브 차분 |
| **2** | **§3 loud→supported** — ✅ **3판 클리어 라운드 완료**(1~10+P1 · ARCHIVE §4.5.342). 다음 후보 = 라운드가 남긴 잔여(§3.1(c) `always_comb` decl-init · §3.3 part-select fold · §3.7 output/inout · §3.11 은 선행조건 ⓐ/ⓑ 로 재편성 — ⭐ⓑ codegen 쪽이 사다리를 안 건드린다) + §3 소형 큐 | iverilog·verilator 실측 ✓ |
| **2b** | **§0 승격 큐 T2 잔여 2건** — `real` const-fold(= `int'(<real param>)` 바운드의 선행) · sized-literal enum label | iverilog ✓ 2/2 |
| 3 | ~~**§2 DEEP** — inner NET vs outer PARAM shadow~~ ✅ **P1 로 해소**(2026-08-18 · ARCHIVE §4.5.342 — 답은 name-set 이 아니라 선언 블록의 SPAN · `repeat (LP)` 는 이미 열려 있었다) — 남은 형제 = §4.5.276 후속 ①(`for` trip-count 식별자) · package 변수 clobber | iverilog ✓ |
| 4 | **§6 OBS** — OBS-2 sva.jsonl(R-L6) 또는 OBS-1 잔여(staged obs·`--seed`) | 3-way 내부 차분 |
| 5 | **성능 — 스케줄러 축**(사다리 아래) — 측정 완료: 스케줄러 **29.0%** self + 그 축의 할당 5.8% ≈ **35%** 미최적화 · 첫 표적은 `propagate` 의 델타마다 `Vec` 셋 | 프로파일 실측 2026-08-18 |
| 6 | DEEP-defer 재개(%c/%s UTF-8 pipeline·derived-localparam self-width·`$unit` typedef ②) | 전용 인프라 슬라이스 |
| **0** | **✅✅✅ Phase A~D 전부 완료 (2026-08-17)** — **A** 커버리지 **100.00%**(거부 0) · **B** 제품 표면이 **native 하나**(`oracle` feature · 삭제 0) · **C** interp = 테스트 도구(성능 최적화 **영구 제외**) · **D** 벤치 **10/10 에서 native < vm** + **코드젠 기각**. **실행 기록 = [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md)** · 해설 = [study/02](study/02-v1-native-coverage.md) | **5,596 green** · clippy 3 구성 0 |

> **순서 주의**: 원칙은 `① 오라클 있는 CRITICAL silent-wrong > ② loud→supported > ③ 전제조건 충족 honest-loud > ④ OBS`([ROADMAP §1](ROADMAP.md)).
> **성능은 이 사다리 위에 올라오지 않는다**(2026-08-17 이후 · Phase D 종료로 옛 "T 단계 최우선" 오너 지시는 소멸).

## C. 잔여 분류 (요약 — 상세=ROADMAP 해당 §)

| 분류 | 항목 수 | 내용 | 정본 |
|---|---:|---|---|
| correct-support 승격 큐 | 6 | **T1 전부 완료** · T2 독립 4 · T3 전제조건 2 | ROADMAP §0 |
| 🔴 silent-wrong 잔여 | 38 | **오라클-有 7**(part-select 바운드·**net 선언 초기화 fill 폭**·**오름차순 음수 bound 의 포트/서브프로그램/클래스 스코프**·package real·구조적 지연·real→int formal·block-local package clobber — ~~replication count~~ 는 §4.5.350 해소) + DEEP 5(UTF-8 pipeline·derived-param width·`$unit` typedef·enclosing-const·packed-WIDTH sibling) + 중형 ~20 + 무오라클 3 | ROADMAP §2 |
| honest-loud 잔여 | 35 + **round-28 4건** | string/heap·함수/formal·소형 큐·EXT2 3건·deep 저우선(VCD fidelity·X→real·x/z-fill param) + **§4.5.284 follow-on 4**(`specify` 블록·이벤트 컨트롤 계층참조 실지원·cross-process `disable` no-op·E3010/E3009 file:line 일관성 — 전부 실사용 ASIC 사이트, 오라클 ✓) | ROADMAP §3 |
| SVA/검증 잔여 | 6 | empty-match 융합·N2c full·prop-ref skew 고급형·QUAD default-flip·N4 clocking 잔여·class down-cast | ROADMAP §4 |
| perf/하드닝 | 5 + **T0~T4** | ⭐ **T0~T4 = 측정된 10.7× 청구**(doc-21 §7.3 · VM 커버리지 0% · 프레임 호출 650 ns vs iverilog 375 ns · 함수 지역 배열 원소 쓰기 514 ns vs 24 ns). 기존 5건은 전부 보류 판정(SVA-QUAD flip·FMT-CACHE b·GEN-3X-STR a·QUEUE-MID + **COMB-DEPTH**: 깊이 비용은 iverilog 도 같음이 실측 = vita 결함 아님. levelize 승격은 프로세스 실행 순서 이동을 감수해야 하고 이득 상한 ≈D/2) | ROADMAP §5 |
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
