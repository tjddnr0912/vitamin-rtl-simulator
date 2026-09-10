---
title: "SystemVerilog System Functions 조사 — Conversion · Bit-Vector · Math (Phase 2)"
date: 2026-05-28
author: research-skill (Claude Sonnet 4.6)
scope:
  - IEEE 1800-2017 §20.8 / §20.9 conversion + bit-vector + math system functions
  - $signed, $unsigned, $rtoi, $itor, $bitstoreal, $realtobits
  - $bits, $clog2, $countones, $countbits, $onehot, $onehot0, $isunknown
  - $pow, $ln, $log10, $exp, $sqrt, trig + hyperbolic, $floor, $ceil
  - Icarus Verilog / Verilator 동작 차이
rounds: 2
status: complete
---

# SystemVerilog System Functions 조사 — Conversion · Bit-Vector · Math

## 조사 배경

Vitamin RTL 시뮬레이터 `hdl-builtins` 크레이트의 Phase 2 구현을 위해
변환 함수(§20.9), 비트벡터 쿼리(§20.9), 수학 함수(§20.8) 세 카테고리의
표준 의미론과 주요 시뮬레이터 동작 차이를 확인한다.
참조 표준: IEEE 1800-2017 §20.8 (수학), §20.9 (변환 + 비트벡터).

---

## 변환 함수 — $signed / $unsigned / $rtoi / $itor / $bitstoreal / $realtobits

### 부호 캐스트: $signed와 $unsigned

두 함수의 핵심은 **비트 폭을 바꾸지 않고 해석만 변경**한다는 점이다.
`$signed(expr)`는 동일한 비트 패턴을 signed(2의 보수)로 재해석하고,
`$unsigned(expr)`는 unsigned(비음수)로 재해석한다.

부호 확장(sign extension)이나 영 확장(zero extension)은 함수 자체가 수행하는 것이 아니라
대입 대상이 더 넓은 경우 SystemVerilog의 일반 대입 규칙에 따라 발생한다.
즉, `$signed(a)`를 더 넓은 와이어에 대입할 때 MSB가 sign extension에 쓰이고,
`$unsigned(a)` 결과를 더 넓은 타겟에 대입할 때 zero extension이 적용된다.

MSB가 x 또는 z인 경우, signed 캐스트 후 더 넓은 타겟에 대입하면 x 또는 z로 sign extension된다.

[출처: circuitcove.com/system-tasks-conversion/ WebFetch ✓;
01signal.com signed-wire-reg WebFetch ✓]

### 정수 ↔ 실수 변환: $rtoi / $itor

`$rtoi`는 실수를 정수로 변환할 때 **향0 방향 절사(truncation toward zero)**를 사용한다.
192.15 → 192, 7.5 → 7, -3.9 → -3. 반올림이 아님.
반환 타입은 `integer`(32-bit signed).

`$itor`는 반대 방향이다. 정수를 IEEE 754 double precision `real`로 변환한다.
32-bit 정수는 double의 유효 자릿수(약 15~16자리) 안에 완전히 표현 가능하므로
정밀도 손실이 없다.

[출처: hdlworks.com/hdl_corner/verilog_ref/items/SystemRealConversionFuncs.htm WebFetch ✓;
chipverify.com/verilog/verilog-conversion-functions WebFetch ✓]

### IEEE 754 비트 레벨 캐스트: $realtobits / $bitstoreal

이 두 함수는 숫자 변환이 아니라 **비트 패턴 그대로 추출/주입**하는 연산이다.

`$realtobits(r)`는 IEEE 754 double precision 인코딩을 64-bit logic vector `[63:0]`로 반환한다.
모듈 포트 경계를 통해 real 값을 전달할 때 사용한다 — Verilog/SV 포트는 real 타입을 직접 지원하지 않으므로.

`$bitstoreal(v)`는 64-bit vector를 IEEE 754 double로 역변환한다. $realtobits의 역.
입력이 정확히 64비트가 아니면 동작 미정의.

SV에는 `$shortrealtobits`(32-bit single precision ↔ 32-bit vector) 쌍도 존재한다.

[출처: hdlworks.com WebFetch ✓; circuitcove.com/system-tasks-conversion/ WebFetch ✓;
groups.google.com/g/comp.lang.verilog/c/JYcSGiNJn5M — bit-level cast 개념 교차 확인]

---

## 비트벡터 쿼리 함수

### $bits — 타입/표현식 비트 폭 조회

`$bits(type_or_expression)`은 C의 `sizeof()`의 비트 단위 버전이다.
type name을 직접 인자로 받는다는 점이 특이하다:
`$bits(logic [7:0])` = 8, `$bits(MyStruct)` = 구조체의 총 비트 수.
expression도 받는다: `$bits(my_signal)` = 신호의 선언 비트 폭.

elaboration-time constant로 평가되므로 파라미터 선언이나 다른 타입 크기 정의에 사용 가능하다.
반환 타입: integer.

[출처: systemverilog.io/ten-utilities WebFetch ✓]

### $clog2 — ceiling log₂, edge cases

`$clog2(N)`의 정의: ceiling(log₂(N)). N개 항목을 binary 주소로 표현할 때 필요한 최소 비트 수다.

| 입력 N | $clog2(N) | 설명 |
|--------|-----------|------|
| 0 | 0 | IEEE 1800-2017 §20.8.1 명시: "argument value of 0 shall produce a result of 0" |
| 1 | 0 | log₂(1) = 0, ceiling(0) = 0 |
| 2 | 1 | |
| 3~4 | 2 | |
| 100 | 7 | 2^7 = 128 > 100 |
| 1024 | 10 | 2^10 = 1024 (exact power — ceiling = log) |

인자는 unsigned 값으로 처리된다. 음수 인자는 미정의 동작.

**반환 타입 논쟁**: IEEE Verilog 2005 §17.11.1은 "integer"(signed) 반환을 명시한다.
그러나 Yosys 등 일부 도구는 unsigned로 반환해 뺄셈 시 예기치 않은 underflow가 발생했다.
(Yosys issue #708, WebFetch ✓). `$clog2(N) - 1` 같은 표현은 signed context에서 사용해야 안전하다.

[출처: circuitcove.com/system-tasks-clog2/ WebFetch ✓;
edaboard.com/threads/clog2 snapshot; github.com/YosysHQ/yosys/issues/708 WebFetch ✓;
fpgacpu.ca/fpga/clog2_function.html WebFetch ✓]

### $countones — 1-bit 개수

`$countones(expression)`은 벡터 내 1-valued 비트의 개수를 반환한다.
`$countbits(expression, 1'b1)`의 shorthand.
x/z 비트는 카운트에 포함되지 않는다 (1이 아니므로).
반환 타입: int.

### $countbits — 4-state 비트 카운터

`$countbits(expression, control_bit [, ...])`는 가장 범용적인 비트 카운터다.
control_bit는 1-bit logic 값으로 0, 1, x, z 모두 지정 가능하다.
여러 control_bit를 나열하면 합산한다: `$countbits(v, 'x, 'z)` = x + z 비트 수.

4-state 완전 지원이라는 점에서 다른 HDL 비트 카운팅 함수들과 차별화된다.

[출처: systemverilog.io/ten-utilities WebFetch ✓; circuitcove.com/system-tasks-vector/ WebFetch ✓]

### $onehot / $onehot0 — one-hot 검사

`$onehot(v)`: 정확히 하나의 비트만 1이면 1(참) 반환. FSM one-hot 인코딩 검증에 사용.
`$onehot0(v)`: 최대 하나의 비트만 1이면 1(참) 반환. 모두 0인 경우도 참.

x/z 비트 포함 시: 해당 벡터가 명확하게 one-hot인지 판단할 수 없으므로 대부분 시뮬레이터가 0 또는 x를 반환한다. IEEE 1800-2017은 이 동작을 명시하지 않아 구현 정의 영역이다.

### $isunknown — 알 수 없는 비트 존재 여부

`$isunknown(expression)`은 표현식의 비트 중 x 또는 z가 하나라도 있으면 1을 반환한다.
등가식: `$countbits(e, 1'bx, 1'bz) != 0`.
4-state 신호의 유효성 검사에서 어서션 전에 흔히 사용한다.

[출처: systemverilog.io/ten-utilities WebFetch ✓]

---

## 수학 함수 (IEEE 1800-2017 §20.8.2 — 모두 real 입출력)

수학 함수 전체는 `real`(IEEE 754 double precision) 입력을 받고 `real`을 반환한다.
예외: `$clog2`만 integer 반환.

### 전체 목록

| 함수 | 시그니처 | C 동등 | 주요 특성 |
|------|---------|--------|----------|
| `$ln` | `$ln(x)` → real | `log()` | 자연로그 |
| `$log10` | `$log10(x)` → real | `log10()` | 상용로그(밑 10) |
| `$exp` | `$exp(x)` → real | `exp()` | e^x |
| `$sqrt` | `$sqrt(x)` → real | `sqrt()` | 제곱근 |
| `$pow` | `$pow(x,y)` → real | `pow()` | x^y |
| `$floor` | `$floor(x)` → real | `floor()` | 내림 (반환도 real) |
| `$ceil` | `$ceil(x)` → real | `ceil()` | 올림 (반환도 real) |
| `$sin` | `$sin(x)` → real | `sin()` | 라디안 단위 |
| `$cos` | `$cos(x)` → real | `cos()` | 라디안 단위 |
| `$tan` | `$tan(x)` → real | `tan()` | 라디안 단위 |
| `$asin` | `$asin(x)` → real | `asin()` | 역삼각 |
| `$acos` | `$acos(x)` → real | `acos()` | |
| `$atan` | `$atan(x)` → real | `atan()` | 단일 인자 |
| `$atan2` | `$atan2(y,x)` → real | `atan2()` | 두 인자, 사분면 인식 |
| `$sinh` | `$sinh(x)` → real | `sinh()` | 쌍곡선 |
| `$cosh` | `$cosh(x)` → real | `cosh()` | |
| `$tanh` | `$tanh(x)` → real | `tanh()` | |
| `$asinh` | `$asinh(x)` → real | `asinh()` | 역쌍곡선 |
| `$acosh` | `$acosh(x)` → real | `acosh()` | |
| `$atanh` | `$atanh(x)` → real | `atanh()` | |
| `$hypot` | `$hypot(x,y)` → real | `hypot()` | √(x²+y²) |

### 도메인 오류 처리

IEEE 1800-2017은 `$sqrt(-1)`, `$ln(0)`, `$pow(-2, 0.5)` 같은 도메인 오류의 동작을
**implementation defined**로 남겼다. 표준에 명시된 동작이 없다.

실제 시뮬레이터 대부분은 C runtime의 IEEE 754 행동을 그대로 위임한다:
- `$sqrt(-1)` → NaN (C `sqrt()` 규칙)
- `$ln(0)` → −∞ (C `log()` 규칙)
- `$pow(−2, 0.5)` → NaN (C `pow()` 규칙)

이 점은 VHDL의 `MATH_REAL` 패키지가 명시적 assertion 처리를 가진 것과 대조된다.

[출처: circuitcove.com/system-tasks-math/ WebFetch ✓; chipverify.com/verilog/verilog-math-functions WebFetch ✓]

### $floor / $ceil의 반환 타입 주의

C의 `floor()`/`ceil()`과 달리, SystemVerilog의 `$floor`/`$ceil`은 **real을 반환**한다.
정수가 필요하면 `$rtoi($floor(x))` 또는 `int'($floor(x))`로 추가 변환해야 한다.

### Icarus vs Verilator 지원

| 범주 | Icarus | Verilator |
|------|--------|-----------|
| $signed, $unsigned | 완전 지원 | Generally supported |
| $bits, $clog2 | 완전 지원 | Generally supported |
| $countones, $countbits, $onehot, $onehot0, $isunknown | 완전 지원 | Generally supported |
| $sqrt, $pow, $ln, $exp, trig/hyperbolic | 완전 지원 | **AMS 모드에서만** (--language VAMS 필요) |

Verilator 표준 SV 모드에서는 수학 함수가 지원되지 않는다는 점은
testbench에서 이들을 사용하는 경우 중요한 제약이다.

[출처: verilator.org/guide/latest/languages.html WebFetch ✓]

---

## Sources

1. **circuitcove.com — Conversion Functions** — https://circuitcove.com/system-tasks-conversion/ (WebFetch Round 1 ✓)
2. **circuitcove.com — Bit Vector Functions** — https://circuitcove.com/system-tasks-vector/ (WebFetch Round 1 ✓)
3. **circuitcove.com — Math Functions** — https://circuitcove.com/system-tasks-math/ (WebFetch Round 1 ✓)
4. **circuitcove.com — $clog2** — https://circuitcove.com/system-tasks-clog2/ (WebFetch Round 2 ✓)
5. **chipverify.com — Verilog Conversion Functions** — https://chipverify.com/verilog/verilog-conversion-functions (WebFetch Round 1 ✓)
6. **chipverify.com — Verilog Math Functions** — https://chipverify.com/verilog/verilog-math-functions (WebFetch Round 1 ✓)
7. **hdlworks.com — System Real Conversion Functions** — https://www.hdlworks.com/hdl_corner/verilog_ref/items/SystemRealConversionFuncs.htm (WebFetch Round 1 ✓)
8. **systemverilog.io — Ten Utilities** — https://www.systemverilog.io/verification/ten-utilities/ (WebFetch Round 2 ✓)
9. **YosysHQ/yosys issue #708** — https://github.com/YosysHQ/yosys/issues/708 (WebFetch Round 2 ✓, $clog2 return type 논쟁)
10. **verilator.org — Input Languages** — https://verilator.org/guide/latest/languages.html (WebFetch Round 1 ✓)
11. **peterfab.com — Verilog Renerta Conversion** — https://peterfab.com/ref/verilog/verilog_renerta/source/vrg00006.htm (WebFetch Round 1 ✓)
12. **01signal.com — Signed Arithmetic** — https://www.01signal.com/verilog-design/arithmetic/signed-wire-reg/ (WebFetch Round 2 ✓)
13. **fpgacpu.ca — $clog2 function** — https://fpgacpu.ca/fpga/clog2_function.html (WebFetch Round 2 ✓)
14. **IEEE 1800-2017 §20.8, §20.9** — 2차 소스 교차 확인 (직접 fetch 불가, MIT/RFSoC 호스팅 PDF는 크기 초과로 fetch 실패)
