# vitamin — 잔여 작업 트래커 (Remaining Work)

> **"goal까지 남은 것"의 상위 스냅샷.** 재계획 시점마다 통째로 갱신한다(과거 판본은 git 이력이 보존).
>
> - **기준(2026-08-08)**: format_version **26** · **5280 tests green** · 3-OS CI green · MsgCode **64** · **MSRV 1.85**.
> - **최신 완료**: **§4.5.314**(IEEE 1364-2005 §12.2 암시적 파라미터 포트 리스트 + staged `-G` + `'x`/`'z` override 차단 — 적대 3라운드 결함 20건, 넷은 내 앞 라운드 수정이 만든 하강). 직전 = §4.5.313(외부 리포트 aes_top 16항목 + 자체 리뷰 17결함) · §4.5.311/312(③층 S3a·S2 슬라이스 4). **완료 슬라이스 전체 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)**(인덱스 = 파일 상단, `#### 4.5.<N>` 검색) · 구 §0~§7 원문 = [ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md).
> - (이전 세대 슬라이스의 상세 서사는 전부 ARCHIVE 로 이관 — 이 문서는 상위 스냅샷만 둔다.)
>
> - 잔여 상세 목록(정본) = [ROADMAP.md](ROADMAP.md) · 완료 상세 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존) · 이력 = [DEVLOG.md](DEVLOG.md) · 실행 큐 = `LOOPROMPT.md` NEXT.
> - **최종 목표**: **G1** = icarus·verilator·xcelium·vcs급 *정확한* RTL 시뮬레이터(correct-or-loud) · **G2** = AI-Agent 친화 simulator(SPEC=[preview/19](preview/19-ai-agent-observability.md)).

## A. 현재 상태 한 줄 요약

> **★★ 2026-08-08 최우선 = ③층 S3 — 바디 코드 생성(진짜 코드젠·아직 0%)** (정본 = [ROADMAP §5.0](ROADMAP.md) / [preview/21 §5 S3+§7](preview/21-tier3-native-backend.md)). T0·S0·S1 전부·S2 4슬라이스·**S3a(호출 흡수)** 완료(§4.5.285~312). ⚠️ 지금의 `--backend native` 는 기계어를 안 만든다(전용 저장 + 폭 특수화 평가기 = 세 번째 인터프리터): native/vm = 1.41×(flat)·1.14×(호출형)·**0.97×(picorv32)**, verilator 대비 54~722×. **flat↔호출형 13× 가 S3 본체의 표적**이고, picorv32 가 1.0× 근처라는 것은 **스케줄러 몫을 먼저 프로파일로 재라**는 뜻이다. 아래 정확성 큐는 사라지지 않고 **그 뒤로 밀린다**.
>
> **직전 = §4.5.283 (외부 round-27, 최고 심각도)** — `@(*)` 가 attribute instance 로 렉싱되어 **주석이 실행 코드로 승격**되고 `errors=0` 으로 틀린 값이 나왔다. 원문 정규식 스캔이 주석·문자열을 뚫고, 짝을 못 찾으면 조용히 폴백해 발현이 **컴파일 단위 전체의 `(*`/`*)` 개수**에 달렸다(파일 경계도 넘었다). attribute 인식을 **토큰 스트림**으로 옮기고, `@` 직후는 event control 로 두고, 안 닫힌 opener 를 loud 로 만들어 닫았다 — 3-way 16형 **회귀 0 · 수정 8**.

- **오라클 있는 열린 silent-wrong = 6건**(ROADMAP §2 상단) — §4.5.228 이 그중 2건(fork-arm 재개 · 음수 하한 unpacked)을 닫았고, 같은 라운드에서 **multi-packed 음수 inner bound 가 silent 였다는 것**을 새로 측정해 함께 닫았다(ROADMAP 이 "warn+clamp"로 적어둔 것은 틀렸다 — 경고는 형제 선언에서 나오고 있었다). 2026-07-23 판이 "소진"이라 적은 것은 그 시점 기준이며, §4.5.217~228 의 적대 리뷰·PRE 3-way 측정이 pre-existing 결함을 계속 **발굴**했다(악화가 아니라 가시화). 그중 **1건은 pre-existing이 아니다** — §4.5.221이 도입한 좁은 loud→silent 하강(계층 real param 바운드).
- 외부 리포트 1·2차(EXT2)·round 3~19 = **사실상 완결**(잔여 3건=A2c·NAP·DOC, 전부 no-oracle/docs).
- 나머지 잔여는 **honest-loud=안전**(ROADMAP §3~§5) + **G2 OBS 트랙**(ROADMAP §6).

## B. 다음 착수 후보 (현재 큐 — 정본 순서·상세=ROADMAP §1)

| # | 항목 | 근거/오라클 |
|---|---|---|
| 1 | **§0 승격 큐 T2 잔여 2건** — `real` const-fold · sized-literal enum label | iverilog ✓ 2/2 |
| 1b | ~~**§0 T2-14 elaborate 단계 파라미터 override**~~ **완료** — `-G`/`--param` 이 `vita`(§4.5.313)와 `velab`(§4.5.314·`-L` 포함)에 적용되고 `vcmp`/`vrun` 은 wrong-stage loud. **잔여 3건** = `-pvalue+` 별칭 · `-P<path>=` 계층 · `.velab` 합성-해시 헤더 필드(format bump) — 상세 = ROADMAP §0-14 | iverilog `-P` 차분 ✓ |
| 2 | ~~fork arm의 suspendable task 재개~~ **완료**(§4.5.228) — 동시 활성화 dyn 배열까지 같이 열렸다 | — |
| 3 | **§2 오라클-有 silent-wrong** — part-select 바운드 silent-0 + replication count silent-0(동근: `const_eval_in_scope` `Cast`/`Call` arm) · package-scope real · 구조적 지연 · real→`input int` formal | iverilog 라이브 차분 |
| 4 | **§2 DEEP** — inner NET vs outer PARAM shadow(선행 = order-INDEPENDENT AST-gathered per-scope name set) | iverilog ✓ |
| 5 | OBS-2 sva.jsonl(R-L6) 또는 OBS-1 잔여(staged obs·`--seed`) | 3-way 내부 차분 |
| 6 | DEEP-defer 재개(%c/%s UTF-8 pipeline·derived-localparam self-width·`$unit` typedef ②) | 전용 인프라 슬라이스 |
| **0** | **★★ ③층 S1d-4c-2d — in-body 웨이터** (…4c-2c ✅ §4.5.285~298 — 런 루프가 돌고 코퍼스 65/72 가 바이트 동일). ⚠️ 코퍼스 커버리지 **0**(정지 터미네이터 138 이 전부 `Delay`)이라 전용 설계로 · 판별 설계는 **순서를 뒤집는 것**이어야 한다 · 게이트에 **진단 스트림**을 반드시 넣어라. 다음 = **4d** settle·wired·VCD → corpus **stdout+VCD 바이트 동일** | 엔진 오라클 in-crate 차분 |

> **순서 주의**: 정본 우선순위는 `① 오라클 있는 CRITICAL silent-wrong > ② loud→supported`인데 1·2위(§0=②)가 3위(§2=①) 앞에 있다 — **오너 지시**. §0를 먼저 해도 §2의 ①-급이 사라진 것은 아니다.

## C. 잔여 분류 (요약 — 상세=ROADMAP 해당 §)

| 분류 | 항목 수 | 내용 | 정본 |
|---|---:|---|---|
| correct-support 승격 큐 | 6 | **T1 전부 완료** · T2 독립 4 · T3 전제조건 2 | ROADMAP §0 |
| 🔴 silent-wrong 잔여 | 38 | **오라클-有 7**(part-select 바운드·replication count·package real·구조적 지연·real→int formal·inner-NET shadow·block-local package clobber) + DEEP 5(UTF-8 pipeline·derived-param width·`$unit` typedef·enclosing-const·packed-WIDTH sibling) + 중형 ~20 + 무오라클 3 | ROADMAP §2 |
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
