# Verilog Modules / Behavioral / Procedural Statements Research Log
**Date**: 2026-05-28
**Scope**: IEEE 1364-2001/2005 + IEEE 1800-2017 §23/§9/§12 Verilog-compat 부분집합
**Topics**: (A) 모듈·포트·파라미터·인스턴스·generate·계층명, (B) initial/always·blocking/NBA 타이밍, (C) 절차 제어문 if/case/loop/fork-join

---

## 조사 방법론

2라운드 WebSearch + WebFetch 검증 방식. Round 1에서 모듈 선언·동작 구문·절차문
3개 주제를 동시 브로드 스윕하고, Phase 1.5에서 핵심 출처 8곳을 WebFetch로 직접
검증. Round 2에서 defparam deprecated 근거, generate block 계층명 접근, fork-join
SV 확장(join_any/join_none) 3개 gap을 fill. 5차원 체크리스트로 통과 확인 후 종합.

---

## Round 1 — Broad Sweep

### 검색 쿼리 4개
1. `Verilog module declaration ANSI non-ANSI port list syntax IEEE 1364-2001 parameter defparam instantiation named positional`
2. `Verilog initial always block sensitivity list blocking non-blocking assignment NBA region timing semantics IEEE 1800-2017`
3. `Verilog case casex casez differences pitfalls full_case parallel_case pragma synthesis generate block`
4. `Verilog fork join disable repeat forever while loop procedural timing control delay event control IEEE 1364`

### Phase 1.5 — WebFetch 1차 Source 검증

| URL | 확인 질문 | 결과 |
|-----|---------|------|
| sigasi.com/tech/ansi-vs-non-ansi/ | ANSI/non-ANSI 문법 차이, 2001 추가 | ✅ |
| verilogpro.com/verilog-case-casez-casex/ | casex X 위험, casez+? 권장 | ✅ |
| chipverify.com/verilog/verilog-blocking-non-blocking-statements | NBA 메커니즘 | ✅ |
| vlsiverify.com/verilog/procedural-timing-control/ | #delay, @event, wait | ✅ |
| chipverify.com/verilog/verilog-always-block | sensitivity list 종류 | ✅ |
| chipverify.com/verilog/verilog-generate-block | generate for/if/case | ✅ |
| chipverify.com/verilog/verilog-parameters | parameter vs defparam | ✅ |
| chipverify.com/systemverilog/systemverilog-fork-join | fork-join 3종 | ✅ |

### Round 1 주요 발견
- ANSI 스타일은 IEEE 1364-2001에서 도입 — 포트 목록에 방향·타입 일괄 선언
- defparam은 IEEE 1800-2017 §23.10에서 명시적 deprecated; Verilog-2001의 named parameter override `#(.P(v))` 로 대체
- casex는 data 측의 X도 don't-care로 처리 → 시뮬레이션에서 전파된 X가 의도치 않은 분기 활성화 → RTL 금지
- `full_case`/`parallel_case` pragma는 시뮬레이터가 완전히 무시(주석) → 합성과 시뮬레이션 불일치 유발
- non-blocking `<=`의 LHS 갱신은 NBA region에서 일괄 수행 → shift register 결정론 보장

---

## Round 2 — Gap Fill

### 추가 검색 쿼리
1. `defparam deprecated Verilog IEEE 1364-2001 module parameter override alternative named parameter syntax`
   → Accellera deprecation proposal 확인: "all designs should now be using #(...) format"
2. `SystemVerilog fork join_any join_none disable fork examples IEEE 1800-2017 section 9`
   → chipverify SV fork-join 3종 차이 확인
3. `Verilog generate block for if case syntax examples genvar hierarchical name access`
   → vlsiverify generate block: genvar elaboration-only, 계층명 `label[i].signal`

### Phase 1.5 (Round 2) — WebFetch 추가 검증

| URL | 확인 내용 | 결과 |
|-----|---------|------|
| chipverify.com/systemverilog/systemverilog-fork-join | fork-join_any/none 차이 | ✅ |
| vlsiverify.com/verilog/generate-blocks-in-verilog/ | genvar, 계층명 pattern | ✅ |
| chipverify.com/verilog/verilog-parameters | defparam 문제점 추가 확인 | ✅ |

### Round 2 주요 발견
- defparam 폐지 맥락: 인스턴스와 분리된 위치에서 파라미터 덮어쓰기 가능 → 가독성·도구 복잡도 문제
- fork-join_any: 첫 번째 완료 시 main thread 재개, 나머지 스레드는 백그라운드 계속 실행
- fork-join_none: spawn 즉시 main thread 재개 (fire-and-forget)
- `disable fork;`: 현재 scope에서 spawn된 모든 활성 스레드 즉시 종료
- genvar는 elaboration time 전용 — simulation에는 존재하지 않음
- generate block 내 금지 항목: port declaration, specify block, parameter declaration

---

## 5차원 Gap Check

| 차원 | 상태 | 근거 |
|------|------|------|
| 정의 | ✅ | 모든 구조(ANSI/non-ANSI, generate, fork-join 종류) 정의 확보 |
| 현황 | ✅ | 구체 예시 + 표준 섹션 정보, deprecated 상태 명시 |
| 근거 | ✅ | 8곳 WebFetch 검증, Accellera proposal + chipverify/sigasi/vlsiverify 교차 확인 |
| 반론/한계 | ✅ | casex 위험, full_case/parallel_case 함정, forever 무한 루프 경고 명시 |
| 적용 | ✅ | 문서 작성에 충분한 데이터 확보 |

---

## 핵심 요약

### A. 모듈 (IEEE 1364-2001/2005, 1800-2017 §23)

**ANSI vs non-ANSI 포트 선언**:
- Non-ANSI: 포트 이름 목록 + 본문에서 방향/타입 분리 선언 (Verilog-1995)
- ANSI: 포트 목록에 방향·타입·폭 일괄 선언 (Verilog-2001 추가) — 권장

**parameter / defparam**:
- `parameter W = 8;` — 모듈 외부에서 override 가능
- `localparam` — 외부 override 불가 (FSM 인코딩 등 내부 상수)
- `defparam` — 인스턴스 외부에서 파라미터 변경; 가독성/도구 문제로 deprecated
- IEEE 1800-2017 §23.10 명시적 deprecated → `#(.P(v))` named override 권장

**instantiation**:
- Positional: `mod inst(a, b, c);` — 포트 순서 의존, 비권장
- Named: `mod #(.W(16)) inst(.clk(clk), .d(din), .q(qout));` — 권장

**generate block (Verilog-2001)**:
- `genvar` 변수는 elaboration time 전용 정수 (simulation에 없음)
- for generate: `generate for(i=0; i<N; i=i+1) begin : lbl ... end endgenerate`
- if generate: `generate if(COND) begin : lbl_t ... end else begin : lbl_f ... end endgenerate`
- case generate: `generate case(P) 0: ...; 1: ...; default: ...; endcase endgenerate`
- 계층명 접근: `top.gen_label[i].sub.sig`

**계층명 (hierarchical name)**:
- dot-separated path: `tb.dut.core.alu.result`
- generate block label 포함: `top.gen_adder[2].u_fa.cout`

### B. 동작 구문 (IEEE 1800-2017 §9)

**initial vs always**:
- `initial`: 시뮬레이션 t=0에 한 번 실행, 자연 종료 → testbench 초기화·파형 생성
- `always`: 영구 루프, sensitivity list로 트리거 제어 → RTL 기술

**sensitivity list**:
- `always @(a or b)` — level-sensitive (조합 논리)
- `always @(*)` — 암시적 전체 (Verilog-2001, incomplete sensitivity 방지)
- `always @(posedge clk)` — edge-triggered (순서 논리)
- posedge: 0→1 전환; negedge: 1→0 전환 (x/z 포함)
- 조합 always에서 sensitivity 누락 → 시뮬레이션/합성 불일치 (래치 또는 오동작)

**blocking(=) vs non-blocking(<=)**:
- Blocking: Active region에서 즉시 LHS 갱신 → 이후 문장은 갱신된 값 사용
- Non-blocking: Active에서 RHS 샘플 → NBA region에서 LHS 일괄 갱신
- NBA 분리: 같은 always 내 `b<=a; c<=b;`에서 c는 b의 구 값을 샘플 → shift register 정확
- 교차 참조: [06-simulation-engine.md] NBA region 상세

### C. 절차문 (IEEE 1800-2017 §12)

**case/casez/casex**:
- `case`: 4-state 완전 비교
- `casez`: case item의 z/? → don't-care (data X는 그대로 비교)
- `casex`: case item과 data 양쪽의 x/z/? → don't-care → RTL 사용 금지
- `full_case`/`parallel_case` pragma: 시뮬레이터 무시 → 합성/시뮬레이션 불일치 → 사용 금지

**루프/fork-join**:
- `for/while/repeat/forever` — 기본 루프, forever는 반드시 delay 포함
- `disable label;` — 명명된 블록 탈출 (break 역할)
- Verilog `fork...join` — 모든 스레드 완료 대기
- SV `fork...join_any` — 첫 완료 시 재개, 나머지 계속
- SV `fork...join_none` — 즉시 재개 (fire-and-forget)
- `disable fork;` — spawn된 모든 활성 스레드 종료

**delay/event**:
- `#d stmt;` — d time-unit 지연
- `data = #5 rhs;` — intra-assignment: RHS 즉시 평가, 5 후 LHS 갱신
- `@(posedge clk)` — rising edge 대기
- `wait(cond)` — level-sensitive: cond 참까지 블록

---

## Sources

| 출처 | 주제 | 검증 방법 |
|------|------|----------|
| sigasi.com/tech/ansi-vs-non-ansi/ | ANSI/non-ANSI 포트 | WebFetch |
| chipverify.com/verilog/verilog-ports | 포트 방향/타입 | WebSearch snippet |
| chipverify.com/verilog/verilog-parameters | parameter, defparam | WebFetch |
| chipverify.com/verilog/verilog-generate-block | generate 구문 | WebFetch |
| vlsiverify.com/verilog/generate-blocks-in-verilog/ | genvar, 계층명 | WebFetch |
| chipverify.com/verilog/verilog-always-block | sensitivity list | WebFetch |
| chipverify.com/verilog/verilog-blocking-non-blocking-statements | blocking/NBA | WebFetch |
| verilogpro.com/verilog-case-casez-casex/ | case 종류, casex 위험 | WebFetch |
| eclipse.umbc.edu/robucci/cmpe316/lectures/L08__Full_and_Parallel_Case/ | pragma 위험성 | WebSearch snippet |
| vlsiverify.com/verilog/procedural-timing-control/ | delay/event control | WebFetch |
| chipverify.com/systemverilog/systemverilog-fork-join | fork-join 3종 | WebFetch |
| accellera.org/images/eda/vlog-pp/att-0523/01-Deprecate_proposal_20020412.pdf | defparam deprecated 근거 | WebSearch snippet |
| csg.csail.mit.edu/6.375/.../cummings-case-snug99.pdf | full/parallel_case evil twins | WebSearch snippet (PDF binary) |

**주의**: IEEE 1800-2017 원문 PDF는 binary-compressed 형식으로 직접 읽기 불가.
위 출처들은 해당 표준을 인용·참조한 2차 자료이나, 교차 검증으로 일관성 확인됨.
