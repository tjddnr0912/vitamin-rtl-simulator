# vitamin — 잔여 작업 트래커 (Remaining Work)

> **"goal까지 남은 것"의 상위 스냅샷.** 재계획 시점마다 통째로 갱신한다(과거 판본은 git 이력이 보존).
>
> - **기준(2026-08-18)**: format_version **27** · **5,533 tests green**(+ 제품 형태 lib green · **`VITA_JIT=1` 로도 전 스위트 green**) · CI **3-OS + `build-no-oracle`** green · MsgCode **67** · **MSRV 1.85** · **기본 백엔드 = `native`**.
> - **최신 완료**: **§4.5.341**(외부 aes_top 3판 — 리포트 11항목 중 6 수정·5 로드맵 · ⭐⭐ IEEE Table 5-1 이 **정의하는** escape 다섯이 조용히 틀려 있었다[`\ddd`·`\xhh`·`\v`·`\f`·`\a`] · `\r` 은 오라클이 갈려 **W3059** 로 loud · 진단이 `file:line:col`+인스턴스 경로를 얻었다[picorv32 elaborate 0 → 58/58] · `run.json` 에 `elab_s`/`sim_s`). 직전 = **§4.5.340**(외부 round-29 · format 27) · **§4.5.314**(IEEE 1364-2005 §12.2 암시적 파라미터 포트 리스트 + staged `-G` + `'x`/`'z` override 차단 — 적대 3라운드 결함 20건, 넷은 내 앞 라운드 수정이 만든 하강). 직전 = §4.5.313(외부 리포트 aes_top 16항목 + 자체 리뷰 17결함) · §4.5.311/312(③층 S3a·S2 슬라이스 4). **완료 슬라이스 전체 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)**(인덱스 = 파일 상단, `#### 4.5.<N>` 검색) · 구 §0~§7 원문 = [ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md).
> - (이전 세대 슬라이스의 상세 서사는 전부 ARCHIVE 로 이관 — 이 문서는 상위 스냅샷만 둔다.)
>
> - 잔여 상세 목록(정본) = [ROADMAP.md](ROADMAP.md) · 완료 상세 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존) · 이력 = [DEVLOG.md](DEVLOG.md) · 실행 큐 = `LOOPROMPT.md` NEXT.
> - **최종 목표**: **G1** = icarus·verilator·xcelium·vcs급 *정확한* RTL 시뮬레이터(correct-or-loud) · **G2** = AI-Agent 친화 simulator(SPEC=[preview/19](preview/19-ai-agent-observability.md)).

## A. 현재 상태 한 줄 요약

> **★★★ 2026-08-17 — Phase A~D 가 전부 끝났다. 다음에 무엇을 할지는 [ROADMAP §5.2 재개 지점](ROADMAP.md) 이 정본이다.** 요약: **A** 커버리지 100.00% · **B** 제품 표면 native 하나 · **C** interp = 테스트 도구 · **D** 성능(벤치 **8/8 에서 native < vm** · 착수 때 최악 2.52×). ⭐⭐ **코드젠은 지어서·배선해서·재서 기각**(런의 ~38% 가 shim · 천장 8.9~11.3% · §5.1-be) ⇒ **성능 축은 수확 체감**이고 **다음 우선순위는 정확성 큐(§2) → loud 승격(§3) → OBS(§6)** 다. ⚠️ 성능을 다시 본다면 **미측정 축은 스케줄러**다(picorv32 비율이 안 움직인 이유는 아직 안 쟀다).
>
> **직전 = §4.5.283 (외부 round-27, 최고 심각도)** — `@(*)` 가 attribute instance 로 렉싱되어 **주석이 실행 코드로 승격**되고 `errors=0` 으로 틀린 값이 나왔다. 원문 정규식 스캔이 주석·문자열을 뚫고, 짝을 못 찾으면 조용히 폴백해 발현이 **컴파일 단위 전체의 `(*`/`*)` 개수**에 달렸다(파일 경계도 넘었다). attribute 인식을 **토큰 스트림**으로 옮기고, `@` 직후는 event control 로 두고, 안 닫힌 opener 를 loud 로 만들어 닫았다 — 3-way 16형 **회귀 0 · 수정 8**.

- **오라클 있는 열린 silent-wrong = 6건**(ROADMAP §2 상단) — §4.5.228 이 그중 2건(fork-arm 재개 · 음수 하한 unpacked)을 닫았고, 같은 라운드에서 **multi-packed 음수 inner bound 가 silent 였다는 것**을 새로 측정해 함께 닫았다(ROADMAP 이 "warn+clamp"로 적어둔 것은 틀렸다 — 경고는 형제 선언에서 나오고 있었다). 2026-07-23 판이 "소진"이라 적은 것은 그 시점 기준이며, §4.5.217~228 의 적대 리뷰·PRE 3-way 측정이 pre-existing 결함을 계속 **발굴**했다(악화가 아니라 가시화). 그중 **1건은 pre-existing이 아니다** — §4.5.221이 도입한 좁은 loud→silent 하강(계층 real param 바운드).
- 외부 리포트 1·2차(EXT2)·round 3~19 = **사실상 완결**(잔여 3건=A2c·NAP·DOC, 전부 no-oracle/docs).
- 나머지 잔여는 **honest-loud=안전**(ROADMAP §3~§5) + **G2 OBS 트랙**(ROADMAP §6).

## B. 다음 착수 후보 (⚠️ **정본은 [ROADMAP §5.2 재개 지점](ROADMAP.md)** — 아래는 그 요약이다)

> ⚠️⚠️ **2026-08-18 정정**: 이 표는 예전에 *"정본 순서·상세 = ROADMAP §1"* 이라고 적혀 있었는데
> **§A 는 §5.2 를 정본이라 적고 있었다** — 포인터가 둘이고 §1 쪽이 썩어 있었다(그 NEXT 0번은
> 2026-08-03 에 완료된 `③층 S1d-4a` 였다). **§1 은 이제 시간 불변 원칙만 갖고, 현재 큐는 §5.2 하나다.**

| # | 항목 | 근거/오라클 |
|---|---|---|
| **1** | **§2 오라클-有 silent-wrong** — ⚠️ **§2 를 위에서부터 읽지 마라**(맨 위 뭉치는 *AST self-폭 패스* 선행조건에 막혀 있다). 착수표 = §2 머리말: ① module-scope `$clog2` 무제한 fold ② 캐스트 SIZE 식 ③ real `**` 지수 — **셋이 한 뿌리** | iverilog 라이브 차분 |
| **2** | **§3 loud→supported** — ⭐ **클리어 순서가 확정됐다**(ROADMAP §3 「3판 잔여 5건의 클리어 순서」 · 코드 사이트까지 실측): **1** §3.5-① `repeat` fold arm+문구 → **2** §3.6-① `default clocking` 배선 → **3** §3.3 wide fold arm → **4** §3.1 이식성 경고 → **5** §3.7 string formal. **다섯 다 선행조건 0.** 그 뒤는 P1(아래 3번 행)에 묶인다 | iverilog·verilator 실측 ✓ |
| **2b** | **§0 승격 큐 T2 잔여 2건** — `real` const-fold(= `int'(<real param>)` 바운드의 선행) · sized-literal enum label | iverilog ✓ 2/2 |
| 3 | **§2 DEEP** — inner NET vs outer PARAM shadow(선행 = order-INDEPENDENT AST-gathered per-scope name set) ⚠️ **이 선행조건은 §3.5 `repeat (LP)` 도 막고 있다**(같은 `walk_scopes_key_shadowed` 계약) | iverilog ✓ |
| 4 | **§6 OBS** — OBS-2 sva.jsonl(R-L6) 또는 OBS-1 잔여(staged obs·`--seed`) | 3-way 내부 차분 |
| 5 | **성능 — 스케줄러 축**(사다리 아래) — 측정 완료: 스케줄러 **29.0%** self + 그 축의 할당 5.8% ≈ **35%** 미최적화 · 첫 표적은 `propagate` 의 델타마다 `Vec` 셋 | 프로파일 실측 2026-08-18 |
| 6 | DEEP-defer 재개(%c/%s UTF-8 pipeline·derived-localparam self-width·`$unit` typedef ②) | 전용 인프라 슬라이스 |
| **0** | **✅✅✅ Phase A~D 전부 완료 (2026-08-17)** — **A** 커버리지 **100.00%**(거부 0) · **B** 제품 표면이 **native 하나**(`oracle` feature · 삭제 0) · **C** interp = 테스트 도구(성능 최적화 **영구 제외**) · **D** 벤치 **10/10 에서 native < vm** + **코드젠 기각**. **실행 기록 = [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md)** · 해설 = [study/02](study/02-v1-native-coverage.md) | **5,533 green** · clippy 3 구성 0 |

> **순서 주의**: 원칙은 `① 오라클 있는 CRITICAL silent-wrong > ② loud→supported > ③ 전제조건 충족 honest-loud > ④ OBS`([ROADMAP §1](ROADMAP.md)).
> **성능은 이 사다리 위에 올라오지 않는다**(2026-08-17 이후 · Phase D 종료로 옛 "T 단계 최우선" 오너 지시는 소멸).

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
