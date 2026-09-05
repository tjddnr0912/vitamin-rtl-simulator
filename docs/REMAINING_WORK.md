# vitamin — 잔여 작업 트래커 (Remaining Work)

> **"goal까지 남은 것"의 상위 스냅샷.** 재계획 시점마다 통째로 갱신한다(과거 판본은 git 이력이 보존).
>
> - **기준(2026-09-06)**: format_version **30** · **7,098 tests green** · 워크로드 코퍼스 **10/10** · CI **3-OS + `build-no-oracle`** green · MsgCode **68** · **MSRV 1.85** · **기본 백엔드 = `native`**.
> - **최신 완료**: **§4.5.425~427**(2026-09-06 · 셋째 3-슬라이스 묶음 · format 29→30) — ⓐ typedef 뒤 packed dims(`pkg::t [pkg::C-1:0] p`) + struct 포트 배열 원소 멤버(ibex 50→20, 다음 페이지 = 모듈 항목 `$fatal`) · ⓑ `%m` 블록 라벨 체인(§2 🆕 N · 32칸) · ⓒ 안쪽 차원 part-select(§2 🆕 L ⓞ · 43칸). 이전: **§4.5.422~424**(2026-09-05 · 둘째 3-슬라이스 묶음 · 한 리뷰) — ⓐ 문장 라벨 `L: stmt`(`ASSERT_INIT` 단, ibex 세 파일 30→0 · 다음 페이지 = ibex_top_tracing 포트 dim 의 `pkg::C`) · ⓑ 파스 시점 상수표·선언 범위 바운드 폭 인식(§2 🆕 L (aa) · 324칸 · fixed-silent 48 · 회귀 0) · ⓒ 계층 파라미터 select/`$bits` · wide 비트 연산 override(§2 🆕 M ⓒⓓ · 198칸 · loud→correct 136) · 잔여 = 🆕 N(`%m` 블록 스코프). 이전: **§4.5.419~421**(2026-09-05 · 첫 3-슬라이스 묶음 · 한 리뷰) — ⓐ 괄호로 감싼 SVA property(`ASSERT` 매크로 본문) 파서 단, ibex 세 파일 50→0 · ⓑ sized 파라미터 초기화식 안의 fill 을 선언 폭으로(§2 🆕 E · 153칸 · 🆕 D 는 재측정으로 이미 정답) · ⓒ >64-bit 파라미터의 계층 읽기(§2 🆕 G · row 28 · 120칸). 잔여 = §2 🆕 M. 그 전 **§4.5.404**(§3 ⑧ + `ca_always` 성능 — **코퍼스 9/10 → 10/10 · 거절 0**). 큐엔 §3 ⑧ 한 줄이었고 그건 **절반**이었다: `$finish` 를 지워도 verilog-ethernet 은 **~38시간** 걸린다. 나머지 절반은 어느 줄에도 없었다 — `expr_is_pure_of_nets` 가 모든 `Expr::Call` 을 거절해 **함수를 부르는 연속대입이 영원히 매 settle 패스 재평가**된다(런타임의 99.99%). ⭐ 기준은 순수성이 아니라 **의존 집합의 완전성**(두 오라클은 감도 목록의 넷이 움직일 때 정확히 재평가한다). ⚠️⚠️ **적대 리뷰 2라운드에 BLOCKING 4** — 라운드 1 이 하나(정적 지역 카운터), **그 고침이 라운드 2 에 셋을 더 만들었다**(`release` 가 *"의존 없으면 한 번"* 전제를 깬다 · 인자 0개 함수의 반환 슬롯이 formal 로 세어졌다 · 부분 쓰기가 넷 전체의 definite assignment 를 세웠다). 곁수확 = 본문 `$display` 30회 → 1회(두 오라클).
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
| **1** | **§2 오라클-有 silent-wrong** — ⚠️ **§2 를 위에서부터 읽지 마라**(주제별 묶음이지 착수 순서가 아니다). **2026-09-02 재그라운딩이 정한 순서 = ~~row 29~~(RESOLVED §4.5.405 · row 25 는 그 안에서 지어서 3라운드 재고 되돌림 — 패치·엣지 셋은 행에) → ~~row 30~~(BUILT·REVERTED §4.5.406 · 선행조건 = §11.8.1 region sign) → ~~§3 ⑦~~(RESOLVED §4.5.407 · 잔여 §2 🆕 H) → ~~row 33~~(RESOLVED §4.5.408 · 잔여 §2 🆕 I) → ~~🆕 C~~(RESOLVED §4.5.409 · 잔여 §2 🆕 J) → ~~§3 ⑤ 파스 규칙~~(RESOLVED §4.5.410 · 182/227 · pre-existing 둘 = 🆕 K 함께 · 잔여 🆕 L) → ~~§3 ⑤ ⓑ~~(RESOLVED §4.5.411 · struct/enum typedef 배열 파라미터 · 114/230 loud→correct · 0 silent · 변수 쌍둥이 decl-init/전체대입도 함께 · ⓒ 의 패키지 절반은 §6.20.1 로 공짜 해결, 헤더 절반이 남았다 · 잔여 = §3 ⑤ ⓔ 상수 문맥 클래스) → ~~§3 ⑤ ⓐ~~(RESOLVED §4.5.412 · 멀티딤 packed 파라미터, typedef·키워드 철자 둘 다 · 141/195 loud→correct · 0 silent · ibex_pkg.sv 에러 0 · pre-existing 셋 = 🆕 L ⓝⓞⓟ · 새 단 ⓕ = 헤더 import 가 헤더 기본값에 안 보임) → ~~§3 ⑤ ⓒ 헤더 절반~~(RESOLVED §4.5.413 · 헤더 배열 파라미터 · 기본값 `= pkg::Rst`/import/형제 · override `'{…}`/`pkg::Arr`/부모 배열 전달/부모 원소 패턴/positional · 109/183 loud→correct · 0 silent · ibex_top→ibex_core 체인 verilator 일치 · pre-existing = 🆕 L ⓢⓣ + `{1'b1, p::X}` 상수 concat 무-fold-arm) → ~~§3 ⑤ ⓓ~~(RESOLVED §4.5.414 · 중첩 packed struct(체인 · 패턴 재귀 · 캐스트) + 상수 폭 멤버(§6.20.1 로 넓힌 `const_locals` · import/scoped/shadow) + struct 패턴의 fill 상수 fold + struct 타입 캐스트 상수 fold · 65/102 loud→correct · 0 silent · ibex_cheriot_pkg.sv 50→0 · 리뷰: soundness BLOCKING 셋(signing cast 문맥 폭 · fit 게이트 회귀 · genvar 그림자) + 델타 재채점 1(헤더 genvar 스코프) 전부 고침 · pre-existing = 🆕 L ⓤ) → ~~ⓕ · ⓔ · 전처리기 단 · 파서 단~~(RESOLVED §4.5.415~418 · 헤더 import · 배열 원소 상수 문맥 · `define 기본인자/본문 지시어/`__LINE__ · tf-port multi-packed formal + based 리터럴 상수 + 멤버 폭 §11.6 fold · ibex 전처리 24→0 · prim 두 파일 50→0) → 다음 = SVA 괄호 property 단(`(a |-> b)` in `assert property`) → lockstep 의 헤더 파라미터 폭 멤버.** ⚠️⚠️ 그 재그라운딩에서 **열린 여덟 줄이 전부 모양이 바뀌었고 셋은 severity 등급이 바뀌었다** — 하나는 원인이 **정반대**, 하나는 **RESOLVED 로 적힌 채 silent 12칸**, 하나는 **제 fix 형태가 silent 를 0칸 고친다** | iverilog + verilator 라이브 차분 |
| **1b** | ~~§2 row 33~~ ✅ **RESOLVED §4.5.408** — 프로시저 read-through(자기 프로세스의 blocking 쓰기 뒤 읽기만) · 59/91 · 저장 측 전파는 picorv32 digest·UDP·keccak 패리티를 깨서 기각 · 잔여 = §2 🆕 I | 2-오라클 |
| **1c** | 🆕 **§2-N 이 남긴 두 줄** — FST 가 **시간표 엔트리 하나짜리 덤프에서 값을 전부 잃는다** · `buf` 가 `~(~i)` 로 낮아져 순수 비트 이동인데 copy 로 안 보인다 | iverilog(FST) · hand-IEEE |
| **2** | **§3 loud→supported** — ⚠️⚠️ **2026-09-02: 이 트랙의 외부 동인이 사라졌다.** 착수 순서를 정하던 워크로드 코퍼스가 **10/10 · 거절 0**(§4.5.404 가 마지막 거절 ⑧ 을 닫았다) ⇒ **§3 은 이제 §2 뒤**다. 남은 줄 = ROADMAP §3 본문(② · ③ 잔여 · ⑤ · ⑦) · ⚠️ ②⑦ 은 **지어서 되돌린** 것이고 선행조건이 기록돼 있다 — 착수 전 **그 선행조건이 아직 사실인지 재라**(§4.5.376 선례: revert 사유가 stale 이었다) | 2-오라클 · hand-IEEE |
| **2a** | ⭐ **워크로드 코퍼스 확장** — 지금 **10 중 10 이 돈다**(거절 0 · 1 ruled-split). ⇒ 값을 더 뽑으려면 **행을 늘려야** 한다. **ibex** 가 열리면 코퍼스 첫 SystemVerilog 워크로드 — ⚠️ §4.5.410 실측: 파스 규칙 하나가 아니라 **블로커 넷**이 더 있다(ROADMAP §3 ⑤ ⓐ~ⓓ · §4.5.411/412/413 이 ⓐⓑⓒ 를 닫아 ibex_pkg.sv 는 에러 0 · §4.5.414 가 ⓓ 를 닫아 cheriot_pkg 도 에러 0 · 전체 설계는 전처리기 E1013 24건) | `corpus-runner list` · [study/03](study/03-workload-corpus.md) |
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
