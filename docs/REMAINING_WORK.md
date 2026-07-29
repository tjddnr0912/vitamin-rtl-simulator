# vitamin — 잔여 작업 트래커 (Remaining Work)

> **"goal까지 남은 것"의 상위 스냅샷.** 재계획 시점마다 통째로 갱신한다(과거 판본은 git 이력이 보존).
>
> - **기준(2026-07-29)**: format_version **26** · **4890 tests green** · 3-OS CI green · MsgCode **59**(W3057) · **MSRV 1.85**.
> - **최신 완료 3건(외부 round-17 리포트 응답 — 진단 34건 전부 RESOLVED + silent-wrong 3건)**: **§4.5.271** 오라클을 만들다 나온 silent-wrong 2건 — ①`expr_reads_ident` 가 콜 경로의 **머리(리시버)** 를 안 봐서 `s.atoi()` 가 `s` 의 읽기로 안 잡혔고, 그 워커 위의 scope-leak 검출기가 블록 밖 참조를 놓쳐 **블록 밖 읽기가 블록 안 값을 돌려줬다**(vita `B 1234` / iverilog `B 9999`) ②`atoi`/`atohex`/`atooct`/`atobin` 이 IEEE §6.16.9 가 아니라 **`strtol`** 이었다(공백 skip·부호·`_` 에서 중단; LRM 은 *leading digits and underscores* 만) — 코드에는 "iverilog 가 버그"라는 주석과 그걸 고정한 핀이 있었고 **둘 다 틀렸다**. + scope-leak/deferred-hier 진단에 위치 부여 · `block_local.rs` 1248줄 분할 · **§4.5.270** 안 쓴 로컬은 per-entry 저장과 **바이트 동일**(§3.2) — 근거는 경로 추론이 아니라 "아무도 안 바꾸므로 매 진입 기본값" 한 줄, `CallEffect::Reads` 신설 + writer 검출기에 리졸버를 opt-in 으로. 그 게이트를 열자 **IEEE §23.9 구멍**(automatic 로컬로의 계층 참조)이 드러나 함께 loud 화(pre-existing 실측, 그러나 도달 범위를 넓힌 건 이 슬라이스) · **§4.5.269** 외부 round-17 §3.1/§3.1b/§3.3 — `expr_no_ref` 에 **`MethodCall` arm 이 없어서** 체인 하나가 그 블록의 뒤쪽 로컬을 전부 거부(12건), DA catch-all 이 **이미 확실히 대입된 상태를 무시**해서 축소 불가 21건, 타이밍 붙은 첫 쓰기 5형태 + 포기 지점을 말하는 `note:`. 직전(외부 round-16 응답 — 뿌리 7 + 진단 품질 4 전부 RESOLVED): **§4.5.268** §3.4~§3.7+§4 — Nets 단계 hoist 는 평평하게, Logic 단계는 `with_scope` 로 중첩하고 있어 두 레벨이 다 스코프면 넷이 있는 경로와 찾는 경로가 갈렸다(분류기의 "중첩 후보 drop"이 그 우회였고 곧 기능 제한이었다) · declarator 단위가 아닌 선언 단위 거부(8→1 진단) · SoA record queue 의 discarding pop · `return f(dyn)`/concat · 진단 위치·거짓 주장·stale 문구·캐스케이드 · **§4.5.267** 고정 크기 `automatic` unpacked 배열 — **per-entry 리셋을 구현했다가 측정으로 기각**(automatic 저장은 블록 진입이 아니라 activation 단위: iverilog 루프 3회 `xx,10,11` vs 호출 3회 `xx,xx,xx`), 대신 커버리지를 증명 · **§4.5.266** definite-assignment 이 제어 흐름과 callee 본문을 본다(리포트 84건 중 **53건**) — bool 격자가 "흘러간다"와 "뛰어나간다"를 뭉개고 있었고, 문장 위치 호출의 F5 위험은 **실재해서**(계층 `t.a=99` 로도 닿는다) 제약을 걷는 대신 **못 건드림을 증명**. 직전 = **§4.5.253** §4.5.251 적대 리뷰 하강 4건 · **§4.5.252** `$sformatf` 근인 재배치(degenerate `eval` arm) · **§4.5.251** `$blk$` decl-init 수집(제외 3개 동시 해소) · **§4.5.250** §4.5.248/249 적대 리뷰(2 렌즈)가 잡은 **사다리 하강 6건** 수정 — 평가 이동 5(`$monitor`/`$strobe` 동결 · 단락 우변 · replication · 인자 순서 역전 · self-ref 큐) + **게이트 극성** 1 · **§4.5.249** 외부 round-20 §6(elaborate 진단 `file:line:col` — `diag::SpanResolver`) + §4.11 일부(같은 이름 **동적** 로컬을 lifetime 무관 분리) · **§4.5.248** 외부 round-20 **8 가족**(fork-arm 블록 로컬 🔴 · `automatic string` decl-init · queue `'{…}`/`void'(pop)` · DA 워크 오진 🔴 · dyn-array formal `'{}` actual · 문장 `'{…}` 대입 · task named arg · `new[N]` decl-init · `$sformatf` 위치) · **§4.5.247** §4.5.246 회귀 수정(flatten 블록 로컬이 generate 스코프 shadow). **완료 슬라이스 223건 전체 목록·상세 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)**(인덱스 = 파일 상단, `#### 4.5.<N>` 로 검색) · 구 §0~§7 원문 = [ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md).
>
> - 잔여 상세 목록(정본) = [ROADMAP.md](ROADMAP.md) · 완료 상세 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존) · 이력 = [DEVLOG.md](DEVLOG.md) · 실행 큐 = `LOOPROMPT.md` NEXT.
> - **최종 목표**: **G1** = icarus·verilator·xcelium·vcs급 *정확한* RTL 시뮬레이터(correct-or-loud) · **G2** = AI-Agent 친화 simulator(SPEC=[preview/19](preview/19-ai-agent-observability.md)).

## A. 현재 상태 한 줄 요약

- **오라클 있는 열린 silent-wrong = 6건**(ROADMAP §2 상단) — §4.5.228 이 그중 2건(fork-arm 재개 · 음수 하한 unpacked)을 닫았고, 같은 라운드에서 **multi-packed 음수 inner bound 가 silent 였다는 것**을 새로 측정해 함께 닫았다(ROADMAP 이 "warn+clamp"로 적어둔 것은 틀렸다 — 경고는 형제 선언에서 나오고 있었다). 2026-07-23 판이 "소진"이라 적은 것은 그 시점 기준이며, §4.5.217~228 의 적대 리뷰·PRE 3-way 측정이 pre-existing 결함을 계속 **발굴**했다(악화가 아니라 가시화). 그중 **1건은 pre-existing이 아니다** — §4.5.221이 도입한 좁은 loud→silent 하강(계층 real param 바운드).
- 외부 리포트 1·2차(EXT2)·round 3~19 = **사실상 완결**(잔여 3건=A2c·NAP·DOC, 전부 no-oracle/docs).
- 나머지 잔여는 **honest-loud=안전**(ROADMAP §3~§5) + **G2 OBS 트랙**(ROADMAP §6).

## B. 다음 착수 후보 (현재 큐 — 정본 순서·상세=ROADMAP §1)

| # | 항목 | 근거/오라클 |
|---|---|---|
| 1 | **§0 승격 큐 T2 잔여 2건** — `real` const-fold · sized-literal enum label | iverilog ✓ 2/2 |
| 1b | **§0 T2-14 elaborate 단계 파라미터 override** — `-G<n>=<v>`/`-pvalue+`(+후속 `-P<path>=`). 상용 3단계 knob 중 **양 끝만 있다**(compile `+define+` ✓ · run `+plusarg` ✓ · elaborate ✗ = `unknown flag '-G'` 실측). 스펙은 doc-14 §RULE B 에 **이미 있고 코드만 없다**. 재컴파일 없이 top param 만 바꿔 재실행이 현재 불가 | iverilog `-P` 차분 ✓ |
| 2 | ~~fork arm의 suspendable task 재개~~ **완료**(§4.5.228) — 동시 활성화 dyn 배열까지 같이 열렸다 | — |
| 3 | **§2 오라클-有 silent-wrong** — part-select 바운드 silent-0 + replication count silent-0(동근: `const_eval_in_scope` `Cast`/`Call` arm) · package-scope real · 구조적 지연 · real→`input int` formal | iverilog 라이브 차분 |
| 4 | **§2 DEEP** — inner NET vs outer PARAM shadow(선행 = order-INDEPENDENT AST-gathered per-scope name set) | iverilog ✓ |
| 5 | OBS-2 sva.jsonl(R-L6) 또는 OBS-1 잔여(staged obs·`--seed`) | 3-way 내부 차분 |
| 6 | DEEP-defer 재개(%c/%s UTF-8 pipeline·derived-localparam self-width·`$unit` typedef ②) | 전용 인프라 슬라이스 |

> **순서 주의**: 정본 우선순위는 `① 오라클 있는 CRITICAL silent-wrong > ② loud→supported`인데 1·2위(§0=②)가 3위(§2=①) 앞에 있다 — **오너 지시**. §0를 먼저 해도 §2의 ①-급이 사라진 것은 아니다.

## C. 잔여 분류 (요약 — 상세=ROADMAP 해당 §)

| 분류 | 항목 수 | 내용 | 정본 |
|---|---:|---|---|
| correct-support 승격 큐 | 6 | **T1 전부 완료** · T2 독립 4 · T3 전제조건 2 | ROADMAP §0 |
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
