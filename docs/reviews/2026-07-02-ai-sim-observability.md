# [외부 리뷰 원문] AI_SIM_OBSERVABILITY.md — LLM 친화적 시뮬레이션 관찰(로그/DB) 설계

> **보존 사유(2026-07-02)**: 외부 리뷰어(crypto 해시 IP 검증팀, 2026-06-29 호환성 리포트=ROADMAP §6과 동일 원천)가 제공한 "시뮬레이터 제작자 전달용" 설계서 원문. vitamin의 신규 최종목표 **G2(AI-Agent 친화 simulator)**의 요구 원천이다. vitamin 측 대응 SPEC = [`../preview/19-ai-agent-observability.md`](../preview/19-ai-agent-observability.md), 트랙 = ROADMAP §7(OBS). 원문 내 상대링크(VERIFICATION_RULES.md·../CLAUDE.md 등)는 리뷰어 저장소 기준이라 여기선 dead — 문맥용으로만 남긴다.

---

# AI_SIM_OBSERVABILITY.md — LLM 친화적 시뮬레이션 관찰(로그/DB) 설계

> **Base: v0.14.**  **★ Be brief.**  목적: emulator 가 아닌 **실 시뮬레이터(xrun/vcs/verilator)**로
> RTL 을 돌릴 때, **LLM 이 실패를 라운드트립 없이 진단**하고 커버리지를 파악할 수 있도록 sim 이
> 어떤 데이터를 · 어떤 형태로 · 어떤 구조로 남겨야 하는지 규정.  시뮬레이터 개발 참고용 설계서.
> 근거 = 본 repo 의 emulator-first 방법론([`VERIFICATION_RULES.md`](VERIFICATION_RULES.md)) + race-class 교훈(Bug 27-33) + SV signedness trap.
>
> **★ §2-8 = "sim 이 뭘·어떻게 emit 하나"(legacy 도 손계측하면 대부분 가능).  §9 = "legacy 가 LLM 에게 구조적으로 못 주는 것" wishlist — 시뮬레이터 제작자 전달용 핵심 산출물.**

---

## 1. 설계 원칙 (이 프로젝트가 실전에서 얻은 5가지 — 일반론 아님)

1. **★ Golden 정렬이 전부다.**  이 repo 는 이미 cycle/byte-accurate **emulator 5종**(`ref/*_xcheck.py`,
   `*_emul.py`)이 stage 별 ground-truth 를 낸다.  sim 의 1순위 임무 = **emulator 와 동일한 스키마로
   stage trace 를 emit** → RTL trace vs emulator trace 를 기계적으로 diff (§3).  이게 최대 차별점.
2. **★ Avalanche → 최종 digest byte-diff 금지, "stage/block/round/lane" 단위로 발산점 국소화.**
   최종 해시 1비트 틀리면 전 byte 무의미(규칙 16).  sim 은 **내부 state checkpoint**(absorb 후 state[],
   permute 후 state[], squeeze lane)을 노출해야 FAIL 이 actionable 해진다.  → 계층 3.
3. **★ Race-class 는 sim 만이 최종 판정** (emulator 는 안전성만).  handshake(valid/ready, chunked
   `o_full`/`i_clear`) 를 **전수 event 로 기록** + 가능하면 SV region/delta 주석 → LLM 이 프로토콜 replay.
4. **Signedness/width 를 로그 자체에서 제거.**  SV `byte` signed trap([memory](../CLAUDE.md)) →
   값은 **명시적 폭 hex**(`0x86`, `64'h...`)로, enum 은 **문자열**(`S_ABSORB`, `HASH_SHAKE256`)로. magic number 금지.
5. **Token 경제 = 계층적 요약.**  PASS = 1줄 terse.  FAIL = rich object + trace window.  전 cycle dump 금지
   (transition/event 만).  waveform 원본(VCD/FSDB)은 LLM 입력 아님 — semantic 로그를 **따로** emit.

---

## 2. 관찰 계층 & 스키마 (Layer 0–6)

> 형식 = **JSONL**(1 record = 1 line, self-contained).  모든 record 는 `run_id`+`test_id` 로 상호참조.

### L0 — Run manifest (`run.json`, 1개)
재현성.  `{run_id, utc, git_sha(rtl+tb), sim{tool,version}, seed, plusargs[], tb, filelist_hash, pass, fail, skip, wall_s}`.

### L1 — Test-case ledger (`results.jsonl`, case 당 1줄) — 최상위 triage

```json
{"test_id":"cavp/SHAKE256VarOut#0931","mode":"HASH_SHAKE256","msg_bits":0,
 "xof_bytes":250,"squeeze":false,"status":"FAIL","result_cycle":41230,
 "expected_sha":"<sha256 of expected>","got_sha":"<sha256 of got>",
 "first_diverge":{"stage":"squeeze_beat","index":9,"module":"sha3_core"},
 "detail_ref":"fail/cavp_SHAKE256VarOut_0931.json"}
```

PASS record 는 `status/result_cycle` 만 + detail_ref 생략 (terse).

### L2 — Failure detail (`fail/<test_id>.json`, FAIL 당 1개) — rich
`{test_id, latched_config{mode,xof_bytes,squeeze,total_bit_len,keylen,cs_str_len,...@i_start/@len_commit},
 expected_hex, got_hex, first_diverge{stage,index,module,expected_lane,got_lane},
 trace_window{cycle_lo,cycle_hi}, notes}`.  ★ expected/got 는 **stage 발산점의 값**을 우선(최종 digest 아님).

### L3 — FSM/state trace (`trace/<test_id>.jsonl`, **transition 만**)

```json
{"cyc":41210,"blk":"sha3_core","fsm":"S_SQUEEZE","from":"S_PERM",
 "abs_lane":0,"sqz_lane":9,"round":0,"bytes_left":58,"squeeze_mode":false}
```

매 cycle 아님 — `fsm` 전이 + 주요 datapath reg 변화 시점만.  hang 진단용 `stuck_in{fsm,cycles}` 도.

### L4 — Handshake/protocol event (`hs/<test_id>.jsonl`) — race 진단 핵심

```json
{"cyc":100,"ch":"msg","fire":true,"data":"128'h...","strb":16,"last":false}
{"cyc":40990,"ch":"chunk_full","o_full":true}          // result_buf 가득
{"cyc":41005,"ch":"clear","i_clear_pulse":true}        // host ack
{"cyc":41230,"ch":"result","o_done":true,"o_error":false}
```

msg beat / digest beat / `o_full`↑ / `i_clear` pulse / `o_done`·`o_error`.  Bug 27-33 류 재현의 유일 소스.

### L5 — Coverage summary (`coverage.json`)
`{modes_hit[], fsm_states_hit[], absorb_case_hit{A..I:count}, cavp_records{ran,skip}, xof_lengths{min,max,>64B},
 chunked_ops, squeeze_ops, hmac, kmac_keylen_range, assertions{pass,fail}}`.  → "무엇이 안 돌았나" 즉답.

### L6 — Assertion/SVA log (`sva.jsonl`)
`{cyc, prop:"p_msg_ready_low_during_precompress", status:"FAIL", signals{...}}`.  property **이름**(번호 아님)+연루 신호값.

---

## 3. ★ emulator ↔ sim 공통 stage-trace 스키마 (킬러 기능)

이 프로젝트만의 결정적 이점: **양쪽이 같은 스키마로 stage 값을 내면 diff 가 자동**.  LLM 은
`(test_id, stage, index)` 로 정렬 → **첫 mismatch 의 module 을 즉시 지목**(계층 2 의 `first_diverge`).

SHA-3 파이프라인 stage 정의(=`ref/sha3_pipeline_xcheck.py` 단계와 1:1):

| stage | index | payload | 담당 module |
|---|---|---|---|
| `prefix_bytes` | — | prefix byte stream (cSHAKE/KMAC) | sha3_prefix_builder |
| `absorb_block` | block# | XOR 직후 state[0..24] (25×64 hex) | sha3_core (absorb) |
| `permute` | perm# | permute 직후 state[0..24] | sha3_keccak_f |
| `squeeze_lane` | beat# | 방출 64b lane | sha3_core (squeeze) |
| `result_pack` | — | o_result_0..7 | hash_result_buf |
| `final_digest` | — | decoded output bytes | TB decode |

- emulator 는 `--trace` 로 이미 이 값들을 낸다.  **sim TB 도 동일 JSONL 을 `+STAGE_TRACE` plusarg 로 dump**.
- 정렬 diff → "prefix 는 일치, absorb_block#2 부터 발산" = **prefix_builder OK, absorb 버그** 로 자동 국소화.
- SHA-2 도 동형(pad_unit→compress round→h_reg pack).  단 SHA-2 RTL 은 owner — 관찰 포트만 요청.

> 이게 avalanche 문제(원칙 2)의 해법: 최종 digest 만 보면 "틀림"뿐이지만, stage 정렬은 **모듈 지목**까지 준다.

---

## 4. LLM 친화적 DB 형태

**Primary = per-run 디렉토리 + JSONL** (LLM 이 직접 read/grep, 각 줄 self-contained):

```
sim_runs/<run_id>/
  run.json            # L0 manifest
  results.jsonl       # L1 ledger (grep status=FAIL 로 즉시 triage)
  coverage.json       # L5
  sva.jsonl           # L6
  fail/<test_id>.json # L2 (FAIL 만)
  trace/<test_id>.jsonl  # L3 (FAIL 또는 +TRACE_ALL)
  hs/<test_id>.jsonl     # L4 (FAIL 또는 +HS_ALL)
  stage/<test_id>.jsonl  # §3 공통 stage trace
```

**왜 JSONL**: (a) 각 줄 독립 → 부분 read 로도 이해, (b) `grep`/`jq` 로 LLM 이 필터, (c) append-only 스트리밍,
(d) 스키마가 named field.

**Optional = SQLite view** (`run.db`): 집계 질의용(`SELECT mode,count(*) FROM results WHERE status='FAIL' GROUP BY mode`).
JSONL → SQLite 로더 1개.  LLM 이 query 도구 있을 때 유용, 없으면 JSONL 로 충분.

**계층적 drill-down**: LLM 은 `results.jsonl`(요약) → FAIL 의 `detail_ref` → `stage/` diff 순으로 **필요한 것만** 로드
(token 절약, 원칙 5).

## 5. Worked example — SHA3-256 chunked squeeze mismatch

`results.jsonl` 한 줄이 `status:"FAIL", first_diverge:{stage:"squeeze_lane",index:9,module:"sha3_core"}` →
LLM 판단 흐름:
1. `stage/` diff: `absorb_block#*`·`permute#*` 전부 일치, `squeeze_lane#0..8` 일치, **#9 부터 발산** → absorb/permute 무결, **squeeze 또는 chunk 경계 버그**.
2. `hs/`: `cyc 40990 o_full↑`, `cyc 41005 i_clear`, `squeeze_lane#8` 이 chunk#0 마지막(64B=8 lane) → #9 는 chunk#1 첫 lane.  발산이 **chunk 경계와 정확히 일치** → chunked handshake re-arm 버그 의심(Bug 27-33 계열).
3. `trace/`: `S_SQUEEZE→S_PERM→S_SQUEEZE` 전이에서 `sqz_lane` 리셋 타이밍 확인.
4. 결론 후보 + `VERIFICATION_RULES` 절차 진입.  **byte-diff 없이 모듈·cycle·원인가설**까지 도달.

→ 최종 digest hex 만 있었으면 불가능.  stage+hs 정렬이 이걸 가능케 함.

## 6. Anti-pattern (하지 말 것)

- **VCD/FSDB 원본을 LLM 입력으로** — binary·거대·semantic 없음.  semantic JSONL 을 따로 emit(파형은 사람용 보조).
- **최종 digest byte-diff 를 진단 근거로** — avalanche(규칙 16).  stage 발산점을 줘라.
- **magic number**(`fsm=3`, `mode=5'h11`) — 문자열 enum.
- **전 cycle dump** — token 폭발.  transition/event/stage checkpoint 만.
- **PASS 도 rich 하게** — PASS 는 1줄.  rich 는 FAIL 한정.
- **seed/version 누락** — 재현 불가 → 재-run 지시 못 함.

## 7. SV 구현 노트 (xrun TB 기준)

- emit 수단: `$fwrite(fd,"%s\n", json_line)` 로 JSONL 직접, 또는 구조화 `$display` prefix(`@@JSON@@ {...}`) + Python 후처리(`ref/` 에 파서).  후자가 기존 walker/`hash_test_pkg.sv` 와 결합 쉬움.
- 기존 자산 활용: `hash_test_pkg` 에 `emit_result()`/`emit_stage()`/`emit_hs()` task 추가 → walker·scenario 에서 호출.  BUILD-ID·PASS/FAIL 카운트도 L0/L1 로 흡수.
- 내부 state 접근: `bind` 로 `sha3_core`/`hash_result_buf` 의 `state_q`/`fsm_q`/`wptr_q` 관찰(합성 RTL 무변경).  §3 stage dump 는 bind module 에서.
- gating: `+STAGE_TRACE`/`+HS_ALL`/`+TRACE_ALL` plusarg 로 FAIL-only vs full 선택(대량 run 은 FAIL-only default).

## 8. 구현 우선순위

1. **MVP**: L0 manifest + L1 ledger(JSONL) + L5 coverage.  → PASS/FAIL triage 자동화 (현 8187/8187 카운트를 구조화).
2. **★ 진단 핵심**: §3 stage trace(bind) + emulator `--trace` 를 동일 스키마로 → 자동 stage-diff 스크립트(`ref/stage_diff.py`).
3. **race**: L4 handshake event + L2 detail.
4. **정밀**: L3 fsm trace + L6 SVA.  Optional SQLite loader.

## 9. ★ Legacy sim 이 LLM 에게 "못" 주는 것 → 신규 sim 의 차별 기회 (제작자 전달용)

**프레이밍**: xrun/vcs/verilator 도 TB 를 손으로 계측하면 §2-8 대부분을 낼 수 있다.  아래는 (a) 그 손계측조차 매우 고통스럽거나, (b) legacy 가 **구조적으로 못 주는** 것 — 신규 sim 이 *LLM-first* 로 설계될 때의 진짜 차별점.  이게 제작자에게 전달할 핵심.

### 9.1 구조적으로 legacy 가 못 주는 것 (최우선 — 신규 sim 의 존재이유)

| capability | legacy 한계 | LLM 이 원하는 정보 · 데이터 · 형식 | 이 repo 근거 |
|---|---|---|---|
| **프로그램 제어 API** (step/poke/peek) | 컴파일된 TB 필요, 입력 바꾸면 재빌드 | stdio/socket JSON-RPC: `poke(path,val)`·`peek(path)→val`·`step(n)`·`run_until(cond,timeout)→{cyc,hit}`.  → LLM 하네스가 **TB 없이** emulator↔RTL 루프 폐합 | emulator-first 방법론; 이번 XOF chunked-handshake 도 API 면 재빌드 없이 파라미터 sweep |
| **결정적 checkpoint / replay / time-travel** | 항상 cycle 0 부터 재-run (SHAKE N=65535 = 수만 cyc) | `snapshot(cyc)→handle`·`restore(handle)`·`rewind_to(first_diverge-100)`.  입력 살짝 바꿔 재개(bisect) | 8187 CAVP sweep·long-XOF 재-run 비용 |
| **delta-cycle / region 순서 event** | VCD 는 timestep 로 collapse — NBA vs blocking 순서 소실 | timestep 내 정렬 이벤트 `{cyc,delta,region:"NBA"\|"active",sig,old,new}` (최소 glitch↔settled 구분) | race 최종판정은 sim 뿐(원칙3); Bug27-33 `o_full`/`i_clear` 2-FF falling-edge 타이밍 |
| **X-전파 origin** | 파형에 X 는 보이나 **발원지·원인 없음** | `{cyc,sig,cause:"uninit"\|"multi-drv"\|"arith-X",driver_path}` (first-X 우선) | reset/init·merged bus(SHA2/3 mux) driver 충돌 |
| **dataflow backward slice** | "왜 Y=V @cyc?" 를 못 답 — 사람이 파형 역추적 | 질의 시 구동 cone = 값을 결정한 `{sig,val}` 목록(1-hop+) | LLM 최대 시간소모(root-cause) 제거 |

### 9.2 Legacy 가능하나 고통 → 자동화하면 큰 이득

| capability | 자동화 형태 | legacy 대비 |
|---|---|---|
| **signal introspection (no hand-bind)** | config 에 `sha3_core.state_q` 나열 → sim 이 named JSONL 자동 dump | `bind`+`$fwrite` 수작업 제거 (§7) |
| **semantic transaction log** | 채널(msg/result/chunk) 1회 기술 → L4 event 자동 emit | `emit_hs()` 수코딩 제거 |
| **native JSON coverage** | L5 를 기본 출력 | UCDB + 독점 merge 툴 회피([imc merge 이슈](../CLAUDE.md)) |
| **SVA fail 시 support-cone** | property 연루 신호 전체 자동 dump | 이름만 주는 것보다 actionable |
| **golden stage hook** | module 경계에서 "stage 라벨 후 state emit" 내장 (§3) | 자동 stage-diff 의 전제 |

### 9.3 인터페이스 / 형식 계약 (전 항목 공통 — 없으면 위가 무의미)

- **전부 machine-readable JSON/JSONL** + `schema_ver` 필드 (스키마 진화 대비).
- **signal path = 안정적 hierarchical 문자열**(`u_sha3.u_core.state_q[3]`), 숫자 index/handle 아님.
- **값 = 명시 폭 hex, enum = 문자열** (원칙 4 — SV `byte` signedness trap 원천봉쇄).
- **on-demand windowed 질의**(cycle 범위 → 이벤트)만, full-cycle dump 금지 (원칙 5).
- **결정성 보장**: 동일 seed → **로그 byte-identical + 이벤트 순서 동일**.  안 그러면 checkpoint/bisect/stage-diff 전부 무의미 → L0 manifest 에 seed·version·ordering-policy 명시.

### 9.4 제작자 요청 우선순위 (capability 측 — §8 은 로그 측)

1. **프로그램 step/peek/poke JSON API + 결정적 replay** — emulator↔RTL 루프를 TB 없이 폐합.  단일 최대 임팩트.
2. **config-driven signal introspection → JSONL** — §3 stage-diff 의 전제(bind 없이 내부 state 노출).
3. **handshake 채널 delta/region 순서 event** — race(Bug27-33)·chunked handshake 진단.
4. **X-origin**, (stretch) **dataflow slice** — root-cause 자동화.

---

## 문서 변경 이력

| 날짜 | 변경 |
|---|---|
| 2026-07-01 | 신규.  LLM 친화적 sim 관찰(로그/DB) 설계 — 5원칙 + L0-6 계층/스키마 + emulator↔sim 공통 stage-trace(§3) + JSONL DB 형태 + worked example + SV 구현/우선순위.  시뮬레이터 개발 참고용. |
| 2026-07-01 | §9 추가 — "legacy sim 이 LLM 에게 구조적으로 못 주는 것" wishlist (프로그램 제어 API·checkpoint/replay·delta/region event·X-origin·dataflow slice) + legacy-가능하나-고통 자동화 + 형식 계약 + capability 요청 우선순위.  시뮬레이터 제작자 전달용 산출물. |
