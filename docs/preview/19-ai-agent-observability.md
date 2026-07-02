# 19 — AI-Agent Observability (OBS) — G2 "AI-Agent 친화 simulator" 스펙

> **신설: 2026-07-02.** vitamin 최종목표에 **G2**를 추가한다: 기존 **G1**(icarus·verilator·xcelium·vcs급 *정확한* 오픈소스 RTL 시뮬레이터, correct-or-loud)에 더해, **AI Agent(LLM 하네스)가 라운드트립 없이 실패를 진단·국소화하고, 커버리지를 즉답받고, TB 재빌드 없이 시뮬레이션을 프로그램 제어**할 수 있는 시뮬레이터.
> 요구 원천 = 외부 리뷰어 설계서 **[AI_SIM_OBSERVABILITY.md](../reviews/2026-07-02-ai-sim-observability.md)**(2026-07-01, ROADMAP §6 리포트와 동일 사용자 그룹). 이 문서가 vitamin 측 단일 정본 SPEC이며, 트랙 관리 = ROADMAP §7(OBS-0~6).

---

## 0. 요약 (결론 먼저)

- **왜 vitamin인가**: 리뷰어 §9("legacy가 구조적으로 못 주는 것")의 전제조건 — **결정성(동일 seed→byte-identical 출력·이벤트 순서 동일)** — 을 vitamin은 이미 코어 불변식으로 보유(3-OS byte-identical·BTree-only·seeded RNG·SchemaHash 게이트). legacy sim이 사후에 못 붙이는 계약을 우리는 무료로 상속한다. 관찰(JSONL rail)·제어(JSON-RPC)는 그 위의 직렬화기/REPL이다.
- **correct-or-loud의 확장**: **틀린 로그 = silent-wrong과 동급**(LLM을 오도해 잘못된 디버깅으로 유인). 관찰값은 엔진 단일 소스에서만 파생(이중 계산 경로 금지), 미해석 probe 경로·미지원 질의는 조용히 스킵하지 않고 **loud**(E-code). 표준 teeth = **3-way 내부 차분(JSONL ≡ VCD ≡ `$display`)**.
- **우선순위**: correctness(A2 체인·silent-wrong 사냥)가 항상 선순위. OBS는 그 다음 슬롯에서 OBS-1(MVP)부터 단계 순.

## 1. 자료수집 — 요구사항 인벤토리 (리뷰어 문서 → REQ ID)

| REQ | 내용 | 원문 | 분류 |
|---|---|---|---|
| R-L0 | run manifest(run.json: run_id·utc·tool/version·seed·plusargs·소스 해시·pass/fail 카운트·wall_s) | §2 L0 | 로그 rail |
| R-L1 | test-case ledger(results.jsonl, PASS=1줄 terse·FAIL=detail_ref) | §2 L1 | 로그 rail |
| R-L2 | failure detail(fail/*.json, 발산점 값 우선) | §2 L2 | 로그 rail |
| R-L3 | FSM/state trace(**transition만**, hang용 stuck_in) | §2 L3 | 로그 rail |
| R-L4 | handshake/protocol event(채널 fire 전수) | §2 L4 | 로그 rail |
| R-L5 | coverage summary(coverage.json — "무엇이 안 돌았나" 즉답) | §2 L5 | 로그 rail |
| R-L6 | SVA log(property **이름**+연루 신호값) | §2 L6 | 로그 rail |
| R-S3 | emulator↔sim 공통 **stage-trace** 스키마 + golden stage hook(정렬 diff→모듈 지목) | §3·§9.2 | 로그 rail(훅) |
| R-C1 | 프로그램 제어 API: `poke/peek/step/run_until` (stdio JSON-RPC, TB 없이 루프 폐합) | §9.1 | 제어 capability |
| R-C2 | 결정적 checkpoint/replay/time-travel(`snapshot/restore/rewind_to`) | §9.1 | 제어 capability |
| R-C3 | delta-cycle/region 순서 event(NBA vs active, glitch↔settled 구분) | §9.1 | 관찰 capability |
| R-C4 | X-전파 origin(`cause: uninit\|multi-drv\|arith-X`, first-X 우선) | §9.1 | 관찰 capability |
| R-C5 | dataflow backward slice(값을 결정한 구동 cone) | §9.1 | 관찰 capability(stretch) |
| R-I1 | config-driven signal introspection(hand-`bind` 없이 named JSONL 자동 dump) | §9.2 | 자동화 |
| R-I2 | semantic transaction log(채널 1회 기술→L4 자동 emit) | §9.2 | 자동화 |
| R-F1 | 형식 계약: JSONL+`schema_ver`·안정 계층경로 문자열·**명시폭 hex+enum 문자열**·windowed 질의·**동일 seed→byte-identical** | §9.3 | 계약 |
| R-A | anti-pattern 계약: VCD를 LLM 입력으로 금지·최종 digest-diff 금지·magic number 금지·전 cycle dump 금지·PASS terse·seed/version 필수 | §6 | 계약 |

리뷰어 자체 우선순위: 로그 측(§8) = L0+L1+L5 MVP → stage trace → L4+L2 → L3+L6. capability 측(§9.4) = ① 제어 API+replay ② config introspection ③ delta/region event ④ X-origin·(stretch) slice.

## 2. 타당성검토 — vitamin 현재 자산 대비 fit/gap

| REQ | 현재 자산 (fit) | gap (해야 할 일) | 공수 |
|---|---|---|---|
| R-F1 결정성 | **이미 코어 불변식**(3-OS byte-identical·seeded RNG·BTree-only) — legacy 대비 구조 우위 그대로 상속 | JSONL sink가 같은 게이트(골든 byte-diff 테스트)를 통과하게 작성 | S |
| R-F1 경로/값 | VCD writer가 계층 `$scope` 안정 경로 보유·enum 이름은 IR/사이드카에 존재·4-state 값 모델 | 경로 문자열 규칙(VCD scope 규칙 재사용)·값 포맷터(`W'h..`, x/z 문자 유지)·envelope(`v`,`t`,`kind`) 확정=**본 스펙 §3** | S |
| R-L0/L1/L5 | exit-code 분류·`$fatal`/assertion 카운트·**N5 functional coverage + SVA/cover 카운터 이미 구현**·plusargs 파서 존재 | `--obs-dir` CLI + 직렬화기. 주의: vitamin은 현재 1 run=1 test 모델 → v1 test_id=run 단위(TB-루프 분절은 `$vita_test_begin/end` v2) | S-M |
| R-L3/R-I1 | 엔진에 net 변경 감지 스트림 존재(VCD가 그 소비자)·net_names 사이드카 | **JSONL probe sink** = 같은 변경 스트림의 2번째 소비자(`--probe <path>`/`--probe-file`). 미해석 경로=loud | M |
| R-L4/R-I2 | — (채널 추상화는 설계 지식이라 sim이 강제 불가) | config로 채널 기술(valid/ready/data 튜플)→fire-event 자동 emit. probe sink 위의 얇은 층 | M |
| R-L6 | SVA 체커가 property 이름·실패 시각을 내부 보유(§4.5.x SVA 트랙) | sva.jsonl emit + support-cone v0(property expr의 leaf 신호 값 dump) | M |
| R-S3 | `$display`/`$fwrite` 이미 지원(리뷰어 §7의 수동 방식은 오늘도 가능) | 전용 벤더 태스크 **`$vita_stage("label", vals…)`** → stage.jsonl(구조화·escape 불필요). iverilog에 없는 태스크=`` `ifdef VITA `` 가드 문서화 | M |
| R-C1 제어 API | vrun이 **단일 스레드 이벤트 루프를 소유** — time-step 경계 REPL 삽입이 구조적으로 깨끗. `run_until(time/cond)`은 기존 워치독/이벤트 기구 재사용 | stdio JSON-RPC(`peek/poke/step/run_until/finish`)·poke=스케줄된 주입 이벤트(저널에 기록→재현성 유지) | L |
| R-C2 checkpoint | 엔진 상태가 serde-문화(postcard 단일 인코더)·힙 컬렉션 전부 BTree/Vec | 스냅샷 경계 정의(NetVar 값·스케줄 큐·frame 스택·class heap·RNG 상태·시각·VCD append 오프셋)=신중 설계 필요. OBS-4 이후 | L-XL |
| R-C3 region event | 스케줄러가 IEEE stratified region 큐를 **명시 보유** — region 주석은 내부 구조의 노출이지 신규 기계 아님 | probe-set 한정 + windowed로 이벤트 폭발 방지 | M-L |
| R-C4 X-origin | 4-state 코어 — X 생성 지점(uninit/multi-driver/arith)이 코드 상 식별 가능 | per-net first-X 태깅 v1(전수 이력은 비목표) | L |
| R-C5 slice | sim-ir가 CA 의존 에지 보유 | 정적 backward cone v1(질의 시)·동적 slice=stretch | L/XL |

**제약(전 단계 공통)**:
- **골든 무영향**: obs 설정은 CLI/SimOpts 사이드카(out-of-band) — frozen SimIr 형상 불변=**전부 IR-0**, format_version 불변. obs rail 자체 버전은 별도 `schema_ver`(초기 1).
- **결정성 게이트**: 동일 입력+seed+obs 설정 → JSONL **byte-identical**(이벤트 순서 포함)을 골든 테스트로 상시 게이트. wall-clock(`utc`,`wall_s`)은 manifest의 명시 필드 2개에만 격리(비교 시 제외 규칙을 스펙에 핀).
- **loud 관찰**: probe 경로 미해석·미지원 kind·windowed 질의 범위 오류 = E-code loud. 값은 엔진 값에서만 파생(3-way 차분이 teeth).
- **token 경제**: PASS=1줄 terse·FAIL=rich·transition/event만(전 cycle 덤프 금지)·VCD는 사람용으로 유지(JSONL은 별도 rail).

## 3. 방향성 — 설계 결정 5핀

1. **rail 분리**: VCD(사람용)와 semantic JSONL(LLM용)은 별개 산출물. 리뷰어 anti-pattern 계약(R-A) 전문 채택.
2. **결정성 계약의 승격**: R-F1의 "동일 seed→byte-identical 로그"를 vitamin 골든 게이트로 승격(기존 3-OS 결정성 인프라 재사용). 이것이 legacy 대비 1호 차별점이자 checkpoint/bisect/stage-diff의 전제.
3. **로그도 correct-or-loud**: 틀린 로그=silent-wrong. 이중 계산 금지(엔진 단일 소스)·미해석=loud·**3-way 내부 차분(JSONL≡VCD≡`$display`)을 OBS 슬라이스의 표준 teeth**로. 적대 2-sub 리뷰 프로토콜(LOOPROMPT §4) 동일 적용.
4. **record envelope(스키마 계약)**: 매 record = self-contained 1줄 JSON, 최소 필드 `{"v":1,"t":<time u64>,"kind":"..."}` + kind별 payload. 값=**명시폭 hex, 4-state 문자 유지**(`"8'h1x"`) · enum=**이름 문자열**(미명명 값은 `"E:8'h03"` 폴백) · 경로=VCD scope 규칙의 full-hier 문자열(`"top.u0.state_q"`). 키 순서 고정(직렬화기가 결정)=byte-identity.
5. **단계 순서 = 리뷰어 §8(로그 먼저) → §9.4(capability)**: MVP가 즉시 유용하고 저위험(S-M)·제어 API는 L급. OBS-1→2→3→4→5→6.

## 4. 단계별 계획 (step-by-step)

> 각 단계 = LOOPROMPT 표준 루프(그라운딩→구현→적대 2-sub→게이트→문서→커밋) 1~2 슬라이스. **전부 IR-0.**

| 단계 | 산출물 | 구현 스케치 | 검증(teeth) | 공수 |
|---|---|---|---|---|
| **OBS-0 ✅** | 본 스펙(계약·스키마·우선순위) | — | — | — |
| **OBS-1 (MVP)** | `--obs-dir D` → `run.json`(R-L0: tool/version/format_version/seed/plusargs/소스 blake3/exit 분류/카운트) + `results.jsonl`(R-L1: v1=run당 1줄, status=PASS/FAIL(exit·`$fatal`·assertion fail)·finish time) + `coverage.json`(R-L5: covergroup/coverpoint/bins hit·assertion pass/fail·cover property 카운트 직렬화) | CLI 플래그+직렬화기(vita-log 인접 신규 모듈). 엔진의 기존 카운터를 종료 시 flush | 골든 byte-diff(같은 입력 2-run 동일)·수치는 기존 `$display`/exit와 3-way 대조 | S-M |
| **OBS-2** | `--probe <path>`(반복)/`--probe-file F` → `trace.jsonl`(R-L3/R-I1: **변경 시만** `{v,t,kind:"chg",path,old,new}`) + `sva.jsonl`(R-L6: property명·시각·verdict·leaf 신호값=support-cone v0) | VCD 변경 스트림의 2번째 소비자로 probe sink 연결·경로 해소는 elaborate 심볼 테이블(미해석=loud E-code) | 3-way 차분(trace.jsonl ≡ VCD 동일 net 타임라인 ≡ `$monitor`)·probe 오타=loud 테스트 | M |
| **OBS-3** | `$vita_stage("label", v0, v1, …)` → `stage.jsonl`(R-S3: `{v,t,kind:"stage",label,idx,vals[]}`) — 사용자 TB가 emulator와 동일 스키마로 정렬 diff 가능 | 벤더 시스템 태스크(no-op Display+StmtId 사이드테이블 선례=§4.5.x `$timeformat` 패턴, IR-0·bump 회피)·`+STAGE_TRACE` plusarg 게이트 | 라벨 순서·값을 `$display` 병행 emit과 바이트 대조·iverilog 호환은 `` `ifdef VITA `` 가드 문서화 | M |
| **OBS-4** | `vrun --control stdio` JSON-RPC(R-C1): `peek(path)`·`poke(path,val)`·`step(n)`·`run_until(time)`·`finish` + 에러 계약(unknown path/bad val=구조화 에러) | time-step 경계에 REPL(단일 스레드 유지)·poke=스케줄 주입 이벤트·**전 명령을 run.json에 저널**(→동일 세션 재생=replay v0) | 제어 세션 기록→비대화식 재실행이 byte-identical·poke≡`force/release`-등가 케이스 내부 차분 | L |
| **OBS-5** | `snapshot()→file`·`restore(file)`·`rewind_to(t)`(R-C2) | 엔진 상태 postcard 직렬화(스냅샷 경계=OBS-4 저널과 결합해 재현). VCD는 restore 후 신규 파일(append 이어쓰기 비목표) | `snapshot→restore→계속` ≡ 무중단 실행 전 구간 byte-diff | L-XL |
| **OBS-6** | X-origin(R-C4: per-net first-X `{t,path,cause}`)·region-annotated events(R-C3: `region:"active\|nba\|…"`,probe-set 한정)·정적 backward cone(R-C5 v1) | X 생성 3지점(uninit/multi-drv/arith) 태깅·스케줄러 region 큐 노출·sim-ir 에지 역추적 | X-cause를 수작업 유도 케이스로 핀·region 순서는 스케줄러 스펙(doc-06)과 대조 | L+ |

**비목표(rail 밖)**: FSDB/UCDB·SQLite 내장(외부 로더 스크립트 1개로 충분 — 리뷰어도 optional)·waveform GUI·UVM 연동·L4 채널 자동 추론(R-I2는 config 기술 기반만). VCD는 사람용으로 유지.

## 5. 트래킹

- 단계별 상태·착수 순서 = **ROADMAP §7**(이 표의 요약본). 실행 큐 = LOOPROMPT NEXT(correctness A2 체인 후 OBS-1부터).
- 스키마 변경은 이 문서 + `schema_ver` bump로만(record envelope는 §3-4핀이 동결 기준).
