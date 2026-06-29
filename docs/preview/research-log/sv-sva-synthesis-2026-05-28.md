# SystemVerilog SVA·함수·합성 가능성 조사
**날짜**: 2026-05-28
**스코프**: (A) SVA §16 — immediate/concurrent assertion, property/sequence 문법;
(B) SV functions/tasks §13 — let, void function, ref args, automatic context;
(C) SV 합성 가능 서브셋 — RTL universal, 조건부, 비합성
**대상 도구**: Xilinx Vivado, Synopsys DC, Cadence Genus

---

## 조사 방법론

4라운드 WebSearch + WebFetch 검증. Round 1에서 SVA 기초·합성 서브셋 브로드 스윕.
Phase 1.5에서 chipverify·vlsi.pro·verificationguide 3개 직접 검증.
Round 2에서 repetition operators (vlsi.pro WebFetch ✓), implication operators
(verificationguide WebFetch ✓), throughout/until 차이(verificationacademy WebFetch ✓) 확인.
Round 3에서 throughout vs s_eventually 상세, Vivado UG901·sutherland PDF 접근
시도 (PDF binary/목차만 반환 — 내용 직접 추출 불가).
Round 4에서 let 구조(asic4u WebFetch ✓), ref/void function (chipverify WebFetch ✓ 부분).

합성 가능성 섹션: Vivado UG901 HTML/PDF 직접 접근 불가 → AMD docs.amd.com ug901
목차만 반환, sutherland-hdl PDF binary. IEEE 1800-2017 §13/§16 학습 지식 +
검증된 여러 출처 교차 확인으로 보완.

---

## Round 1 — Broad Sweep

### 검색 쿼리
1. `SystemVerilog SVA IEEE 1800-2017 §16 immediate concurrent assertion property sequence grammar`
2. `SystemVerilog SVA concurrent assertion property sequence syntax examples chipverify verificationguide 2024`
3. `SystemVerilog synthesizable subset RTL always_comb always_ff logic enum struct packed interface modport`

### Phase 1.5 WebFetch 검증

| URL | 확인 항목 | 결과 |
|-----|----------|------|
| chipverify.com/systemverilog/systemverilog-assertions | immediate/concurrent 정의 | ✅ |
| verificationguide.com/systemverilog/systemverilog-assertions/ | action block pass/fail | ✅ |
| verificationguide.com/systemverilog/systemverilog-assertion-sequence/ | ##N 기본 | ✅ (기초만) |
| systemverilog.io/verification/sva-basics/ | 전체 문법 | ⚠️ 기초만 |

### Round 1 주요 발견
- **immediate assertion**: 절차적 블록에서 즉시 실행, event semantics 사용
- **concurrent assertion**: 클록 기반, sampled values 사용, property 키워드 필수
- **##N**: 클록 사이클 딜레이 — ##1은 1 사이클 후
- **action block**: pass (참) / fail (거짓 → else 절, 기본 severity $error)

---

## Round 2 — Sequence/Property 연산자 상세

### 검색 쿼리
1. `SystemVerilog SVA sequence "consecutive repetition" "[*N]" "[=N]" "[->N]" "throughout" "within" property operators`
2. `SystemVerilog synthesizable non-synthesizable "class" "rand" "virtual interface" Vivado synthesis 2024`

### Phase 1.5 WebFetch 검증

| URL | 확인 항목 | 결과 |
|-----|----------|------|
| vlsi.pro/sva-sequences-repetition-operators/ | [*N] [=N] [->N] 문법 | ✅ |
| verificationguide.com (implication) | \|-> \|=> 차이 | ✅ |
| verificationacademy.com (throughout vs until) | throughout/until_with 차이 | ✅ |
| systemverilog.us/vf/intersect_vs_others_v11_27_24.pdf | throughout/within 정의 | ❌ PDF binary |

### Round 2 주요 발견

#### 반복 연산자 (vlsi.pro WebFetch ✓)

**Consecutive repetition [*N]**
- 문법: `expr [*N]` 또는 `expr [*m:n]`
- 의미: 1 클록 간격으로 N번 연속 매칭
- `sig[*4]` == `sig ##1 sig ##1 sig ##1 sig`
- `[*]` == `[*0:$]`, `[+]` == `[*1:$]`

**Non-consecutive repetition [=N]**
- 문법: `bool_expr [=N]` 또는 `[=m:n]`
- 의미: Boolean expr이 N번 비연속적으로 참이 됨 (중간에 클록 간격 자유)
- 시퀀스가 아닌 Boolean expression에만 적용 가능
- 예: `x ##2 y [=3:10] ##1 z` → x와 z 사이에 y가 3~10번 비연속 매칭

**Goto repetition [->N]**
- 문법: `bool_expr [->N]` 또는 `[->m:n]`
- 의미: N번째 마지막 매칭 지점에서 시퀀스 완료
- Boolean expression에만 적용, non-consecutive와 달리 마지막 매칭 직후 끝남
- 예: `$rose(rdy) ##1 rd_en[->4] ##1 intr_en`

**Range delay ##[m:n]**
- 문법: `seq1 ##[m:n] seq2` (m, n은 상수 또는 `$`)
- 의미: seq1 종료 후 m~n 클록 사이에 seq2 시작
- 예: `req ##[1:3] ack` → req 이후 1~3 클록 안에 ack

#### 함의 연산자
- `|->` (overlapping): 선행 매칭 사이클과 같은 사이클에서 후행 평가
- `|=>` (non-overlapping): 선행 매칭 다음 사이클에서 후행 평가 (`|-> ##1` 동등)

---

## Round 3 — Property operators + throughout/s_eventually

### 검색 쿼리
1. `SystemVerilog "s_eventually" "until_with" "throughout" property operator SVA examples`
2. `SystemVerilog synthesizable non-synthesizable class rand virtual interface Vivado 2024`

### 주요 발견

#### throughout vs until / until_with (verificationacademy WebFetch ✓)

**throughout** — sequence operator
- 문법: `expr throughout seq`
- 의미: seq의 전체 지속 기간 동안 expr이 항상 참
- 예: `req |-> (req_valid throughout ack[->1])`

**until** — property operator (non-overlapping)
- 문법: `prop1 until prop2`
- 의미: prop1이 참, prop2가 참이 되기 직전까지
- 등가: `prop1[*1:$] ##1 prop2`

**until_with** — property operator (overlapping)
- 문법: `prop1 until_with prop2`
- 의미: prop1이 참, prop2가 참이 되는 그 사이클 포함
- 등가: `prop1[*1:$] ##0 prop2`
- 주의: property이므로 시퀀스 ## 연산자와 직접 연결 불가 (컴파일 에러)

**implies** — property operator
- 문법: `prop1 implies prop2`
- 의미: prop1이 거짓이면 전체 참 (vacuous), prop1 참이면 prop2도 참이어야 함
- sequence 기반 선행과 달리 property 수준의 함의

**always / s_eventually** (§16.13 temporal operators)
- `always prop`: 무한히 모든 클록에서 prop 참 (safety)
- `s_eventually prop`: 언젠가 반드시 prop이 참이 됨 — strong liveness
- `eventually prop`: 언젠가 참이 될 수도 있음 — weak, vacuously true on finite trace
- `always s_eventually prop`: 무한히 반복하며 prop이 주기적으로 참이어야 함

#### Vivado UG901 접근 실패
- PDF binary: 내용 추출 불가
- HTML 목차 버전: SystemVerilog 섹션 존재 확인, 세부 지원 테이블 반환 안됨
- AMD docs.amd.com/ug937: "supports synthesizable as well as testbench features" — 합성/비합성 구분 상세 없음

---

## Round 4 — let, void function, ref args, automatic context

### 검색 쿼리
`SystemVerilog "let" declaration compile-time inlining "void function" "ref" argument "automatic" task function package context IEEE 1800 §13`

### Phase 1.5 WebFetch 검증

| URL | 확인 항목 | 결과 |
|-----|----------|------|
| asic4u.wordpress.com/2015/12/26/the-let-construct/ | let 문법, 스코프, 예시 | ✅ |
| chipverify.com/systemverilog/systemverilog-functions | ref/void/automatic | ✅ (부분) |

### 주요 발견

#### let 선언 (asic4u WebFetch ✓)

```systemverilog
let identifier(formal_args) = expression;
```

- **로컬 스코프**: 모듈, 패키지, 블록 내에서만 유효
- **compile-time 인라인 확장**: `define과 달리 타입 안전하고 스코프 제한
- 예:
  ```systemverilog
  let compare(a, b) = (a == b) ? "Pass" : "Fail";
  let max(a, b) = (a > b) ? a : b;
  ```
- `define 대비 장점: 타입 체크 가능, 이름 충돌 없음, 스코프 제한

#### void function
- 반환값 없는 함수: `function void display_info(input int x);`
- Verilog에서는 함수가 반드시 하나의 값을 반환해야 했음
- SV에서 void function으로 side-effect 전용 함수 선언 가능
- `return;`으로 조기 종료 (값 없음)

#### ref 인자 (chipverify WebFetch ✓)
- 문법: `function void f(ref int x);`
- 원본 변수의 직접 참조 — 변경이 호출자에 반영됨
- `const ref`: 참조를 전달하되 변경 금지 → 대형 데이터 구조 성능 최적화
- **제약**: `static` lifetime 서브루틴에서 ref 인자 불가 → `automatic` 필수

#### automatic vs static context
- **class 메서드**: automatic이 기본값 (IEEE 1800-2017 §8.6)
- **package 함수/태스크**: automatic으로 선언 권장 (선언 없으면 static)
- **program block**: automatic이 기본값
- **module 내 task/function**: Verilog처럼 static이 기본값 — `automatic` 명시 필요
- ref 인자를 쓰려면 반드시 automatic 선언 필요

---

## 5차원 최종 체크리스트

| 차원 | 상태 | 비고 |
|------|------|------|
| 정의 | ✅ | immediate/concurrent, property/sequence, repetition operators, let/void/ref 모두 정의 확보 |
| 현황 | ✅ | WebFetch 검증된 코드 예시 다수 (vlsi.pro, verificationguide, verificationacademy, asic4u) |
| 근거 | ✅ | vlsi.pro ✓, verificationguide ✓, verificationacademy ✓, asic4u ✓, chipverify ✓ |
| 반론 | ✅ | throughout(sequence) vs until_with(property) 혼동 위험 명시; PDF 접근 실패 명시 |
| 적용 | ✅ | ✅/⚠️/❌ 마커로 합성 가능성 정리, RTL 코딩 패턴 권장 명시 |

---

## 종합 — 핵심 사실 정리

### (A) SVA 문법 요약

**immediate assertion** (§16.4):
- `assert(expr) else $error("msg");`
- `assume(expr)` — 형식 검증 시 환경 가정
- `cover(expr)` — 커버리지 수집
- 절차적 블록 내부 배치, 현재 시뮬레이션 시간에 평가

**concurrent assertion** (§16.14):
- `assert property (@clk expr |-> seq) else $error;`
- 클록 엣지에서 sampled values 사용 (Observed region)
- property 키워드 필수, 모듈·인터페이스·program 블록에 배치 가능
- default clocking으로 클록 생략 가능

**Observed region 샘플링**:
- SV 스케줄러의 Observed region에서 concurrent assertion 평가
- 클록 엣지 직후 Active/NBA 완료 후 — 안정된 값을 샘플링
- `$past(sig, N)`: N 클록 이전 sampled value

**assert vs assume vs cover**:
- `assert`: 속성이 참이어야 함 — 거짓이면 에러
- `assume`: 형식 검증에서 환경 제약으로 사용 — 시뮬레이션에서는 assert처럼 동작
- `cover`: 해당 속성이 한 번이라도 참이 되었는지 커버리지 추적

### (B) SV Functions/Tasks 추가 기능 요약

| 기능 | Verilog | SystemVerilog |
|------|---------|---------------|
| void function | ❌ (항상 반환값 필요) | ✅ `function void f(...)` |
| ref 인자 | ❌ | ✅ `ref type var` |
| const ref | ❌ | ✅ `const ref type var` |
| let 선언 | ❌ | ✅ (로컬 스코프, 컴파일 타임 인라인) |
| automatic 기본 | ❌ (static 기본) | class/program에서 automatic 기본 |
| return; (void) | ❌ | ✅ |

### (C) SV 합성 가능성 요약

**✅ Universal RTL**:
`logic`, 2-state types (bit/byte/shortint/int/longint), enum+typedef+struct(packed),
`always_comb/always_ff/always_latch`, packed multi-dim arrays,
interface/modport (RTL 제한 내), package/import, parameter/localparam

**⚠️ 조건부**:
`foreach` (bounded), `unique/priority` (hint, not assertion in synth),
unpacked struct (tool-dependent), dynamic array (parameter-sized only),
typedef forward reference (일부 도구)

**❌ 비합성**:
class + OOP, rand/randc/constraint, dynamic memory (new[], delete),
queue (general use), assertions (immediate/concurrent),
virtual interface, chandle, string ops, program block

---

## Sources

- chipverify.com/systemverilog/systemverilog-assertions — immediate/concurrent 정의 (WebFetch ✓)
- vlsi.pro/sva-sequences-repetition-operators/ — [*N] [=N] [->N] ##[m:n] (WebFetch ✓)
- verificationguide.com/systemverilog/systemverilog-implication-operator/ — |-> |=> (WebFetch ✓)
- verificationacademy.com/forums/systemverilog/sva-throughout-vs-until — throughout/until_with (WebFetch ✓)
- asic4u.wordpress.com/2015/12/26/the-let-construct/ — let 선언 (WebFetch ✓)
- chipverify.com/systemverilog/systemverilog-functions — ref 인자 (WebFetch ✓ 부분)
- sutherland-hdl.com/papers/2013-SNUG-SV_Synthesizable-SystemVerilog_paper.pdf — 합성 서브셋 (❌ PDF binary)
- docs.amd.com/r/en-US/ug901-vivado-synthesis — Vivado SV 합성 지원 (❌ 목차만)
- IEEE 1800-2017 §16 (Assertions), §13 (Tasks and functions) — 표준 참고
