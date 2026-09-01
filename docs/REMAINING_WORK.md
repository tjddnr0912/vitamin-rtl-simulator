# vitamin — 잔여 작업 트래커 (Remaining Work)

> **"goal까지 남은 것"의 상위 스냅샷.** 재계획 시점마다 통째로 갱신한다(과거 판본은 git 이력이 보존).
>
> - **기준(2026-09-01)**: format_version **29** · **6,394 tests green** · 워크로드 코퍼스 **8/10** · CI **3-OS + `build-no-oracle`** green · MsgCode **68** · **MSRV 1.85** · **기본 백엔드 = `native`**.
> - **최신 완료**: **§4.5.399**(§2 의 남은 후보 전부를 재측정 · 실은 것 = 표 **18**(defparam 부호)·**8**(clocking INPUT 이 `$readmem*` 로 쓰기 가능) · ⭐⭐ **코드 없이 닫은 셋** = `buf`(반박 — `~(~i)` 는 §7.3 z→x 강제다)·FST(진단 오류 · 지어서 되돌림)·표 10(넷 중 둘 이미 닫힘) · ⚠️⚠️ BLOCKING 둘 다 내 수정: 게이트를 **모든 인자**에 걸어 파일 이름을 correct→loud 로 거절 · `param_meta` 의 `signed` 는 **기본값이지 사실이 아니다** ⇒ 부호가 **구문에서 evident 할 때만** 기록) · 직전 = **§4.5.398**(§2 표 **13·6·4·12** 를 큐 순서대로 · ⭐ 착수 전 재측정이 둘의 모양을 바꿨다 — 표 **14** 는 fold 의 부호 규칙이 맞고 빠진 것이 **이름의 선언 폭 provenance** 라 그 벽의 **네 번째 도착**이므로 착수 금지, 표 **12** 의 기록된 증상은 §4.5.384 가 이미 닫아 사다리 한 칸 위였다 · ⚠️⚠️ **적대 리뷰 BLOCKING 다섯이 전부 내 수정의 산물이고 넷이 같은 모양** = *다른 PHASE 에서 빌려온 술어는 거울이고 거울은 닿는 순간 어긋난다*) · 직전 = **§4.5.397**(**포트 연결이 없는 전이를 만들었다** — ROADMAP §2-N 의 남은 절반 · `assign n = m` 같은 «비트를 옮기는» 구동자는 t0 전이를 만들 수 없다 · 117칸 중 **47 FIXED · 0 REGRESSION** · ⚠️⚠️ 2라운드 리뷰가 서로 **반대 방향**의 수정을 요구했다: 미러는 이벤트를 **지어내고** 즉시 억제는 이벤트를 **잃는다** ⇒ 살아남는 성질은 **전이(transitive)** · ⚠️ verilog-axi 는 여전히 미승격이고 잔여는 **오라클 자기모순**(`a|b` 발화 · `a&b` 무발화)).
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
| **1** | **§2 오라클-有 silent-wrong** — ⚠️ **§2 를 위에서부터 읽지 마라**(맨 위 뭉치는 *AST self-폭 패스* 선행조건에 막혀 있다). 착수표 = §2 머리말의 「다음 착수 순서」. **2026-09-01 기준 = 표 13·6·4·12·18·8 ✅ 해소 · 2-N ✅ 해소** ⇒ 다음은 **표 7**(부모 `initial` 이 t0 에 자식 net 을 읽으면 X · 2-오라클 · ⚠️ **코퍼스 수요 0** 이고 `format_version` 30 을 요구한다) · **표 3b·8·10·11·15~21** · ⛔ **표 14 는 착수 금지**(fold 의 부호 규칙은 맞고 빠진 것은 **이름의 선언 폭 provenance** — §4.5.371/373/382 가 멈춘 그 벽의 네 번째 도착 · **한 인프라 항목이지 네 줄이 아니다**) | iverilog + verilator 라이브 차분 |
| **1b** | 🆕 **§2-N 이 남긴 신규 두 줄** — FST 가 **시간표 엔트리 하나짜리 덤프에서 값을 전부 잃는다**(pre-existing · iverilog FST 로 확인 · 3 백엔드 동일) · `buf` 가 `~(~i)` 로 낮아져 **순수 비트 이동인데 copy 로 안 보인다** | iverilog(FST) · hand-IEEE |
| **2** | **§3 loud→supported** — ⭐⭐ 착수 순서는 **워크로드 코퍼스**가 정한다(§4.5.369). 남은 거절 하나 = **⑧ verilog-ethernet**(frame-call subset 밖) · ⚠️ **verilog-axi 는 돌지만 미승격**이고 그 잔여는 **오라클 자기모순**이라 제2 오라클 없이 착수 금지 | 2-오라클 · hand-IEEE |
| **2a** | ⭐ **워크로드 코퍼스 확장** — 지금 **10 중 1 거절 · 1 미승격**. **ibex** 가 열리면 코퍼스 첫 SystemVerilog 워크로드 | `corpus-runner list` · [study/03](study/03-workload-corpus.md) |
| **2b** | **§0 승격 큐 T2 잔여** — `real` const-fold(= `int'(<real param>)` 바운드의 선행) | iverilog ✓ |
| 3 | **§2 DEEP / 인프라** — ⭐ **선언 폭·부호 provenance**(§2 표 14 의 선행조건이자 §4.5.371/373/382 의 공통 벽) · 인라인 fold 가 **식 위치에서 문장을 낼 수 있게**(§2 표 6 의 남은 절반) · UTF-8 pipeline · `$unit` typedef | 전용 인프라 슬라이스 |
| 4 | **§6 OBS** — OBS-2 sva.jsonl(R-L6) 또는 OBS-1 잔여(staged obs·`--seed`) | 3-way 내부 차분 |
| 5 | **성능 — 4위, 닫힌 게 아니다**(사다리 아래) — 표적은 실측치로 **ROADMAP §5.2 표 4·4b + §4 프로파일 절**에 · ⚠️ A/B 는 **양방향 인터리브**, 스냅샷은 **release** | 프로파일 실측 |
| **0** | **✅✅✅ Phase A~D 전부 완료 (2026-08-17)** — **A** 커버리지 **100.00%** · **B** 제품 표면이 **native 하나** · **C** interp = 테스트 도구 · **D** 벤치 **10/10 에서 native < vm** + **코드젠 기각**. 실행 기록 = [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md) · 해설 = [study/02](study/02-v1-native-coverage.md) | clippy 3 구성 0 |

> **순서 주의**: 원칙은 `① 오라클 있는 CRITICAL silent-wrong > ② loud→supported > ③ 전제조건 충족 honest-loud > ④ OBS`([ROADMAP §1](ROADMAP.md)).
> **성능은 이 사다리 위에 올라오지 않는다**(2026-08-17 이후 · Phase D 종료로 옛 "T 단계 최우선" 오너 지시는 소멸).

## C. 잔여 분류 (요약 — 상세=ROADMAP 해당 §)

| 분류 | 항목 수 | 내용 | 정본 |
|---|---:|---|---|
| correct-support 승격 큐 | 6 | **T1 전부 완료** · T2 독립 4 · T3 전제조건 2 | ROADMAP §0 |
| 🔴 silent-wrong 잔여 | ~24 | **2026-09-01 기준 해소**: §2-N(포트 연결이 만든 가짜 전이) · 표 13(음수 enum 라벨의 부호) · 표 6(static function 의 모듈 net 쓰기 → loud) · 표 4(`$itor` 의 real 인자) · 표 12(§6.19 fail-open) · **잔여의 큰 덩어리 하나 = 이름의 선언 폭·부호 provenance**(표 14 + §4.5.371/373/382 · **한 인프라 항목**) + 표 7(t0 프로세스 순서 · format 30 필요) + DEEP 5 + 무오라클 3 | ROADMAP §2 |
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
