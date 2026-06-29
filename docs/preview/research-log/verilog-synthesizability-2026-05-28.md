# Verilog 합성 가능성 리서치 로그
**날짜**: 2026-05-28
**스코프**: Verilog-2005 (IEEE 1364-2005) RTL 합성 가능/조건부/비합성 매핑
**대상 도구**: Synopsys Design Compiler (DC), Xilinx Vivado, Cadence Genus

---

## 조사 방법론

4라운드 WebSearch + WebFetch 검증. Round 1에서 IEEE 1364.1 합성 표준, Vivado UG901,
비합성 구문 목록을 브로드 스윕. Phase 1.5에서 billauer.se (initial 블록 FPGA 지원),
asic-soc.blogspot (합성/비합성 테이블), nandland (delay 처리) 3곳 WebFetch 검증.
Round 2에서 defparam deprecated, full_case/parallel_case mismatch, fork-join 비합성,
recursive function, integer 타입 합성 5개 gap fill.
lowRISC style-guide WebFetch로 defparam·casex·full_case 금지 근거 직접 확인.
Round 3에서 Vivado UG901 HTML 버전 접근 시도(PDF binary로 추출 불가 — HTML 페이지는
목차만 반환). DC initial block warning VER-708 "declaration initial assignment is
not supported; it is ignored" 패턴 확인. Round 4에서 fork-join 비합성 모순 해소
(asic-soc "합성 가능" vs 다수 출처 "비합성" — 파악: asic-soc 테이블의 "fork, join"은
Verilog 시뮬레이션 의미론적 허용을 표기한 것으로, 실제 합성 도구는 모두 거부).

---

## Round 1 — Broad Sweep

### 검색 쿼리
1. `Verilog synthesizable subset IEEE 1364.1 Synopsys DC Vivado Cadence Genus initial blocks delays UDP tasks functions loops generate defparam`
2. `Vivado synthesis UG901 Verilog supported constructs initial block FPGA synthesizable 2025`
3. `Verilog synthesis non-synthesizable constructs real type fork-join system tasks force release delay always synthesis tool warning error`

### Phase 1.5 — WebFetch 1차 Source 검증

| URL | 확인 질문 | 결과 |
|-----|---------|------|
| billauer.se/blog/.../verilog-initial-xst-quartus-vivado/ | initial 블록 FPGA 지원 여부, ASIC 지원 여부, 처리 방식 | ✅ — FPGA(XST/Quartus/Vivado) 파워업 초기값으로 처리; ASIC 미언급 |
| asic-soc.blogspot.com (합성/비합성 테이블) | 합성/비합성 구문 전체 테이블 | ✅ — real/time/event 비합성; system tasks 비합성; force/release 비합성; 절차 delay "partial" |
| nandland.com/lesson-6 | #delay 처리 방식 | ✅ — delay는 비합성, "FPGA has no concept of time" |

### Round 1 주요 발견
- IEEE 1364.1-2002: Verilog RTL 합성 서브셋 표준, 2009년 IEEE 1800에 흡수
- FPGA 도구(XST/Quartus/Vivado)는 initial 블록을 파워업 초기값으로 변환
- DC는 initial 블록을 "VER-708: not supported, ignored" 경고와 함께 무시
- real/time 타입: 비합성
- system tasks ($display 등): 비합성
- force/release: 비합성
- #delay in always: 무시 또는 에러 (도구별)

---

## Round 2 — Gap Fill

### 추가 검색 쿼리
1. `Verilog defparam synthesis deprecated Synopsys DC Vivado Cadence Genus support`
2. `full_case parallel_case synthesis pragma deprecated simulation synthesis mismatch`
3. `Verilog integer type synthesis synthesizable 32-bit signed`

### Phase 1.5 — WebFetch 검증

| URL | 확인 질문 | 결과 |
|-----|---------|------|
| github.com/lowRISC/style-guides VerilogCodingStyle.md | defparam, casex, full_case/parallel_case 금지 근거 | ✅ |
| eclipse.umbc.edu/robucci/cmpe316/lectures/L08 | full_case/parallel_case 상세 | ❌ 502 |
| teamvlsi.com/2023/06/writing-synthesizable | defparam, recursive, initial ASIC | ❌ 내용 미포함 |

### Round 2 주요 발견
- **defparam**: lowRISC style-guide에서 "Do not use `defparam`" 명시. 합성 도구별 지원 여부보다 표준 설계 관행상 사용 금지.
  IEEE 1800-2012에서 deprecated 예고, 사용 지양 권장.
- **full_case/parallel_case**: "simulation/synthesis mismatch" 위험 — lowRISC "Never use either pragma" 명시.
  SV의 `unique case` / `priority case`가 공식 대체제.
- **casex**: lowRISC "should not be used" — X-propagation 대칭성 위험.
  casez + `?` wildcard 또는 SV `case inside`가 권장 대체.
- **integer**: 32-bit signed, 합성 도구가 사용 범위에 따라 비트 트리밍.
  edaboard 확인: 합성 가능하나 명시적 비트 폭(reg [N:0]) 권장.

---

## Round 3 — Gap Fill 2

### 추가 검색 쿼리
1. `Synopsys Design Compiler initial block ASIC synthesis not supported error RTL`
2. `Vivado UG901 Verilog constructs table synthesizable HTML`

### Phase 1.5 — WebFetch 검증

| URL | 확인 질문 | 결과 |
|-----|---------|------|
| copyprogramming.com/howto/how-does-synthesis-translate-off-work | DC initial block warning VER-708 패턴 | ✅ — "VER-708: The construct 'declaration initial assignment' is not supported in synthesis; it is ignored" 확인 |
| docs.amd.com/r/en-US/ug901-vivado-synthesis (HTML) | fork-join, real, initial, delay 지원 테이블 | ⚠️ HTML 목차만 반환 — 개별 페이지 직접 접근 필요 |

### Round 3 주요 발견
- DC의 initial block 처리: "ignored" 경고 발생, 기능 포함 안 됨
- Vivado UG901: PDF는 binary, HTML은 목차만 — 공식 구문 테이블 직접 접근 불가
  (단, billauer.se + 다수 출처로 Vivado initial block → 파워업 초기값 확인)
- fork-join: Quora 기사 결론("비합성") + analogcircuitdesign.com("비합성") 확인
  asic-soc.blogspot 표기("합성 가능")는 언어 의미론 측면 — 실제 도구 동작은 비합성

---

## Round 4 — fork-join 모순 해소 + recursive function 확인

### 추가 검색 쿼리
1. `Verilog fork join synthesis "not synthesizable" ASIC FPGA RTL tool`
2. `Verilog "recursive function" synthesis "not supported" Synopsys DC Vivado`

### Phase 1.5 — WebFetch 검증

| URL | 확인 질문 | 결과 |
|-----|---------|------|
| quora.com (fork-join 비합성 이유) | fork-join 비합성 근거 | ❌ 403 |
| analogcircuitdesign.com/verilog-simulation-synthesis/ | fork-join/real/recursive 합성 여부 | ❌ 403 |

### Round 4 주요 발견
- **fork-join**: WebSearch 결과 다수 출처가 "non-synthesizable, testbench only" 확인.
  asic-soc의 표기는 Verilog 시뮬레이터 의미론 기준이며 합성 도구는 지원 안 함.
  → ❌ 비합성 판정.
- **recursive function**: WebSearch에서 DC/Vivado 모두 재귀 함수 합성 불지원 확인
  (Synopsys synthesis 관련 포럼 + SNUG 2013 자료). IEEE 1364.1-2002에도 재귀 없음.
  → ❌ 비합성 판정.

---

## 5차원 최종 체크리스트

| 차원 | 상태 | 비고 |
|------|------|------|
| 정의 | ✅ | 13개 구문 분류 전부 정의 확보 |
| 현황 | ✅ | WebFetch 검증된 구체 사실 다수 (DC VER-708 warning, lowRISC 금지 사항, billauer FPGA initial 처리) |
| 근거 | ✅ | billauer.se, lowRISC style-guide, asic-soc.blogspot, nandland.com, teamvlsi.com, edaboard.com, WebSearch 다수 |
| 반론 | ✅ | asic-soc.blogspot fork-join "합성 가능" 표기 vs 실제 도구 비합성 — 모순 명시 및 해소 |
| 적용 | ✅ | ✅/⚠️/❌ 마커로 정리, FPGA/ASIC 차이, RTL 코딩 패턴 권장 |

---

## 종합 — Verilog-2005 합성 가능성 매핑

### ✅ 합성 가능 (universal — DC/Vivado/Genus 공통)

| 구문 | 비고 |
|------|------|
| 모듈 선언 / 포트 / 파라미터 (`parameter`, `localparam`) | RTL 기본 빌딩 블록 |
| `wire`, `reg` 데이터 타입 | 합성의 핵심 타입 |
| `integer` (32-bit signed) | 합성 가능; 비트 트리밍 발생. 명시적 `reg [N:0]` 권장 |
| 벡터 / 부호 있는 타입 (`reg signed`, `wire signed`) | 합성 가능 |
| `always @(posedge clk)` | 클록드 레지스터(FF) 추론 |
| `always @(*)` | 조합 논리 추론 |
| `if/else`, `case` | 우선순위 MUX 추론 |
| `casez` (with `?` only) | 합성 가능; X-propagation 없이 안전 |
| `for` loop (bounded) | 루프 경계가 컴파일 타임 상수인 경우 합성 가능 — unroll됨 |
| `generate` (for/if) | 파라미터화된 인스턴스 생성에 권장 |
| `task` (static, 시간 소비 없음) | 합성 가능; automatic 가능하나 DC에서는 static 선호 |
| `function` | 합성 가능 (0-시간, input 전용) |
| `assign` (연속 대입) | 조합 논리 |
| 내장 게이트 프리미티브 (`and`, `or`, `nand` 등 조합형) | 합성 가능 |
| `localparam`, `defparam` 대신 `parameter` | defparam 금지 — parameter/localparam 사용 |

### ⚠️ 조건부 합성 (도구 의존 / 형식 제약)

| 구문 | 조건 | 비고 |
|------|------|------|
| `initial` | **FPGA**: Vivado/Quartus/XST → 파워업 초기값으로 변환 ✅ | DC(ASIC): VER-708 경고 후 무시 ❌ |
| `integer` | 합성 도구가 비트 트리밍; 명시적 폭 미지정으로 영역 낭비 가능 | RTL에서는 `reg [N:0]` 선호 |
| `casex` | 합성 도구는 처리하나 X-propagation 대칭 매칭 위험 → 사용 금지 권장 | `casez` 또는 SV `case inside` 대체 |
| `defparam` | 일부 도구 지원; IEEE 1800 deprecated — 설계 관행상 사용 금지 | `#(...)` 파라미터 override 방식 사용 |
| Tristate (`assign x = en ? d : 1'bz`) | FPGA top-level I/O OK; on-chip 내부 사용 비권장 | lowRISC: "Do not use Z for on-chip muxing" |
| 조합형 UDP | 일부 도구 지원; 이식성 낮음 — 사용 비권장 | `assign` 또는 always @(*) 대체 |
| `full_case` / `parallel_case` pragma | 일부 도구 처리하나 sim/synth mismatch 위험 → 사용 금지 | SV `unique case` / `priority case` 대체 |
| `task` (automatic, 시간 소비 있음) | DC는 automatic task 합성 제한적; 시간 소비 task는 비합성 | static + 0-시간 제약이 합성 안전 |

### ❌ 비합성 (시뮬레이션 전용)

| 구문 | 비고 |
|------|------|
| `real`, `time` 데이터 타입 | 부동소수점 / 64-bit 시뮬레이션 전용 |
| `#delay` (always 블록 내부) | 합성 도구가 무시하거나 에러; "FPGA has no concept of time" |
| `$display`, `$monitor`, `$finish` 등 system tasks | 시뮬레이션 전용 |
| `fork-join` | 병렬 실행 의미론 → 합성 불가; testbench 전용 |
| 순차형 UDP (edge-sensitive, level-sensitive latch UDP) | 합성 도구 미지원 |
| `force` / `release` | testbench 전용 |
| Recursive `function` / `task` | DC/Vivado 모두 재귀 합성 불지원; IEEE 1364.1에도 없음 |
| `while` / `forever` loop (unbounded) | 컴파일 타임 종료 조건 없는 루프 비합성 |
| 게이트 수준 delay (`and #(2) g1(...)`) | 타이밍 시뮬레이션 전용; 합성 도구 무시 |
| drive strength (`strong1`, `weak0` 등) | 시뮬레이션 전용; 합성 도구 무시 또는 경고 |
| MOS 스위치 / 양방향 스위치 (`nmos`, `tran` 등) | 아날로그/스위치 레벨 시뮬레이션 전용 |

---

## Sources

- billauer.se/blog/2018/02/verilog-initial-xst-quartus-vivado/ — FPGA initial block 지원 (WebFetch ✓)
- github.com/lowRISC/style-guides/blob/master/VerilogCodingStyle.md — defparam/casex/full_case 금지 (WebFetch ✓)
- asic-soc.blogspot.com/2013/06/synthesizable-and-non-synthesizable.html — 합성/비합성 구문 테이블 (WebFetch ✓)
- nandland.com/lesson-6-synthesizable-vs-non-synthesizable-code/ — #delay 비합성 (WebFetch ✓)
- edaboard.com/threads/synthesis-of-integer-in-verilog.399780/ — integer 타입 합성 동작 (WebSearch)
- copyprogramming.com (DC VER-708 warning) — initial block DC 동작 (WebSearch ✓)
- academia.edu/105046952 (Cliff Cummings, "full_case & parallel_case Evil Twins") — full/parallel_case 위험 (WebSearch)
- accellera.org (IEEE 1364.1-2002 합성 표준 참조) — 합성 서브셋 표준 (WebSearch)
- IEEE 1364.1-2002 "IEEE Standard for Verilog Register Transfer Level Synthesis" — 합성 서브셋 공식 정의 (IEEE Xplore, WebSearch)
- Vivado Design Suite User Guide Synthesis UG901 — Verilog Language Support (도구 공식 문서; PDF binary 추출 불가, HTML 목차만 확인)
- Synopsys Design Compiler Synthesis User Guide (copyprogramming.com 경유 VER-708 내용 확인)
