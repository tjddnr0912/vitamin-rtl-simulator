# 08 · Math Functions

## 개요

IEEE 754 double precision 실수 연산을 제공하는 수학 함수 카테고리다.
전체 함수가 `real` 타입 입출력을 사용하며, C 표준 수학 라이브러리(`<math.h>`)와
1:1 대응된다. 합성 불가 시뮬레이션 전용이다.

## 지원 Phase

- ⏳ **전 함수 미구현 (loud-reject E3009, N6 트랙)**: `$ln`, `$log10`, `$exp`, `$sqrt`, `$pow`, `$floor`, `$ceil`, `$sin`, `$cos`, `$tan`, `$asin`, `$acos`, `$atan`, `$atan2`, `$sinh`, `$cosh`, `$tanh`, `$asinh`, `$acosh`, `$atanh`, `$hypot`.
  수학 transcendental은 **순수-Rust libm 핀 결정에 블록**되어 있다 — Rust `std` `f64` 메서드는 플랫폼별 libm으로 위임되어 3-OS 바이트 동일성(결정성 골든 게이트)을 깰 위험이 있어, 결정론적 구현 전략이 확정될 때까지 elaborate가 `E3009 "unsupported system function"`으로 거부한다.
  (`$clog2`만 §20.8.1 계열에서 예외로 구현됨 — [07-bit-vector.md](07-bit-vector.md) 참조.)

---

## 항목 상세

### 전체 함수 목록

모든 함수의 반환 타입은 `real`(IEEE 754 double precision)이다.
예외: `$clog2`는 §20.8.1이지만 integer 반환으로 [07-bit-vector.md](07-bit-vector.md)에서 다룬다.

| 함수 | 시그니처 | C 동등 | 분류 |
|------|---------|--------|------|
| `$ln` | `$ln(x: real)` → real | `log(x)` | 로그 |
| `$log10` | `$log10(x: real)` → real | `log10(x)` | 로그 |
| `$exp` | `$exp(x: real)` → real | `exp(x)` | 지수 |
| `$sqrt` | `$sqrt(x: real)` → real | `sqrt(x)` | 제곱근 |
| `$pow` | `$pow(x: real, y: real)` → real | `pow(x, y)` | 거듭제곱 |
| `$floor` | `$floor(x: real)` → real | `floor(x)` | 내림 |
| `$ceil` | `$ceil(x: real)` → real | `ceil(x)` | 올림 |
| `$sin` | `$sin(x: real)` → real | `sin(x)` | 삼각함수 |
| `$cos` | `$cos(x: real)` → real | `cos(x)` | 삼각함수 |
| `$tan` | `$tan(x: real)` → real | `tan(x)` | 삼각함수 |
| `$asin` | `$asin(x: real)` → real | `asin(x)` | 역삼각 |
| `$acos` | `$acos(x: real)` → real | `acos(x)` | 역삼각 |
| `$atan` | `$atan(x: real)` → real | `atan(x)` | 역삼각 |
| `$atan2` | `$atan2(y: real, x: real)` → real | `atan2(y, x)` | 역삼각 (사분면 인식) |
| `$sinh` | `$sinh(x: real)` → real | `sinh(x)` | 쌍곡선 |
| `$cosh` | `$cosh(x: real)` → real | `cosh(x)` | 쌍곡선 |
| `$tanh` | `$tanh(x: real)` → real | `tanh(x)` | 쌍곡선 |
| `$hypot` | `$hypot(x: real, y: real)` → real | `hypot(x, y)` | √(x²+y²) |
| `$asinh` | `$asinh(x: real)` → real | `asinh(x)` | 역쌍곡선 |
| `$acosh` | `$acosh(x: real)` → real | `acosh(x)` | 역쌍곡선 |
| `$atanh` | `$atanh(x: real)` → real | `atanh(x)` | 역쌍곡선 |

---

### 로그·지수 함수 (`$ln`, `$log10`, `$exp`)

```sv
real x = 2.71828182845904523536;  // e

$ln(x)         // ≈ 1.0
$ln(1.0)       // 0.0
$log10(100.0)  // 2.0
$log10(1.0)    // 0.0
$exp(1.0)      // ≈ 2.71828...
$exp(0.0)      // 1.0
```

**도메인 오류**: IEEE 1800-2017은 도메인 오류 시 동작을 **implementation defined**로 남겼다.
C runtime IEEE 754 행동을 위임하는 시뮬레이터에서 실제로 나타나는 결과:

| 호출 | 예상 결과 | 원인 |
|------|----------|------|
| `$ln(0.0)` | −∞ | C `log(0)` = -INFINITY |
| `$ln(-1.0)` | NaN | 음수의 로그는 실수 범위 밖 |
| `$log10(0.0)` | −∞ | 동일 |
| `$exp(1000.0)` | +∞ | double overflow |

이 값들은 IEEE 754 특수값(NaN, ±Inf)이므로 이후 연산에서 전파된다.
production testbench에서 도메인 오류가 의심되면 결과를 `$isnan()`으로 확인하되,
`$isnan`은 SV 표준 함수가 아니므로 Verilog-A 확장 또는 직접 비교(`r != r`가 NaN의 특성)를 사용한다.

---

### `$sqrt`

- **시그니처**: `$sqrt(x: real)` → `real`
- **표준**: IEEE 1800-2017 §20.8.2
- **의미**: 비음수 `x`의 제곱근. C `sqrt()` 동등.

```sv
$sqrt(4.0)     // 2.0
$sqrt(2.0)     // ≈ 1.41421356...
$sqrt(0.0)     // 0.0

// 도메인 오류
$sqrt(-1.0)    // NaN (C sqrt 규칙 — implementation defined)
```

**정밀도**: IEEE 754 double precision — 약 15~16 유효 자릿수. 디지털 설계 파라미터 계산에는 충분하다.

---

### `$pow`

- **시그니처**: `$pow(x: real, y: real)` → `real`
- **표준**: IEEE 1800-2017 §20.8.2
- **의미**: `x`의 `y`승. C `pow(x, y)` 동등.

```sv
$pow(2.0, 10.0)    // 1024.0
$pow(2.0, -1.0)    // 0.5
$pow(4.0, 0.5)     // 2.0 ($sqrt(4.0)와 동일)
$pow(0.0, 0.0)     // 1.0 (수학 관례: 0^0 = 1)

// 도메인 오류
$pow(-2.0, 0.5)    // NaN (음수의 비정수 거듭제곱 — C pow 규칙)
$pow(0.0, -1.0)    // +∞ (C pow 규칙)
```

**SV `**` 연산자**: SystemVerilog는 `x ** y` 연산자도 제공한다. 정수 타입에서 `a ** b`는 `$pow`와 다르게 동작할 수 있으므로(정수 산술) real 컨텍스트에서는 `$pow` 또는 명시적 `real'(a) ** real'(b)` 사용 권장.

---

### `$floor` / `$ceil`

- **시그니처**: `$floor(x: real)` → `real`, `$ceil(x: real)` → `real`
- **표준**: IEEE 1800-2017 §20.8.2

```sv
$floor(3.7)    // 3.0  (내림 — 가장 큰 정수 ≤ x)
$floor(-3.7)   // -4.0 (음수 방향 내림)
$ceil(3.2)     // 4.0  (올림 — 가장 작은 정수 ≥ x)
$ceil(-3.2)    // -3.0
```

**반환 타입 주의**: C `floor()`/`ceil()`은 `double`을 반환하지만 SystemVerilog `$floor`/`$ceil`도 동일하게 **`real`을 반환**한다.
정수가 필요하면 추가 변환이 필요하다:

```sv
// 정수 변환 패턴
integer i = $rtoi($floor(3.7));   // i = 3
integer j = $rtoi($ceil(3.2));    // j = 4
// 또는 SV 캐스트
int k = int'($floor(3.7));        // k = 3
```

---

### 삼각함수 (`$sin`, `$cos`, `$tan`)

- **표준**: IEEE 1800-2017 §20.8.2
- **단위**: **라디안(radian)** — degree가 아님

```sv
real pi = 3.14159265358979323846;

$sin(0.0)       // 0.0
$sin(pi/2.0)    // 1.0
$cos(0.0)       // 1.0
$cos(pi)        // -1.0 (≈ −1.0 + epsilon, IEEE 754 근사)
$tan(pi/4.0)    // ≈ 1.0

// degree → radian 변환
real deg = 45.0;
real rad = deg * pi / 180.0;
$sin(rad)       // ≈ 0.7071 (sin(45°))
```

**$tan 특이점**: `pi/2`에서 이론적으로 ±∞이지만 IEEE 754 double에서 `pi/2`는 정확히 표현되지 않으므로 큰 유한값이 나온다. implementation defined 범주.

---

### 역삼각함수 (`$asin`, `$acos`, `$atan`, `$atan2`)

```sv
$asin(1.0)      // pi/2 ≈ 1.5707963...
$acos(1.0)      // 0.0
$atan(1.0)      // pi/4 ≈ 0.7853981...
$atan(-1.0)     // -pi/4

// $atan2 — 두 인자 버전, 사분면(quadrant) 인식
$atan2(1.0, 1.0)   // pi/4  (1사분면, x=1 y=1)
$atan2(1.0, -1.0)  // 3*pi/4 (2사분면)
$atan2(-1.0, 0.0)  // -pi/2 (음의 y축)
```

`$atan2(y, x)`는 벡터 `(x, y)`의 각도를 −π~+π 범위로 반환한다.
인자 순서에 주의: **y가 첫 번째, x가 두 번째** — C `atan2(y, x)`와 동일.

도메인 오류: `$asin(2.0)`, `$acos(-2.0)` (|x| > 1) → NaN.

---

### 쌍곡선 함수 (`$sinh`, `$cosh`, `$tanh`)

```sv
$sinh(0.0)    // 0.0
$cosh(0.0)    // 1.0
$tanh(0.0)    // 0.0
$tanh(100.0)  // ≈ 1.0 (saturate to 1)
```

---

## 도메인 오류 요약

IEEE 1800-2017은 도메인 오류 동작을 명시하지 않는다(implementation defined).
Icarus 등 C runtime 위임 시뮬레이터에서 나타나는 실제 동작:

| 함수 호출 | 실제 결과 | IEEE 754 특수값 |
|----------|----------|----------------|
| `$sqrt(-1.0)` | NaN | quiet NaN |
| `$ln(0.0)` | −∞ | -INFINITY |
| `$ln(-1.0)` | NaN | quiet NaN |
| `$pow(-2.0, 0.5)` | NaN | quiet NaN |
| `$pow(0.0, -1.0)` | +∞ | +INFINITY |
| `$asin(2.0)` | NaN | quiet NaN |
| `$acosh(0.5)` | NaN | x < 1은 도메인 밖 |

NaN이 산술에 전파되므로 도메인 오류가 발생할 수 있는 코드에서는 선행 범위 검사를 권장한다.

---

## Icarus / Verilator 지원

| 범주 | Icarus | Verilator |
|------|--------|-----------|
| $ln, $log10, $exp, $sqrt, $pow | 완전 지원 | **AMS 모드에서만** (`--language VAMS`) |
| $floor, $ceil | 완전 지원 | AMS 모드에서만 |
| $sin, $cos, $tan | 완전 지원 | AMS 모드에서만 |
| $asin, $acos, $atan, $atan2 | 완전 지원 | AMS 모드에서만 |
| $sinh, $cosh, $tanh | 완전 지원 | AMS 모드에서만 |

**Verilator 제약**: 표준 SV 모드(`--language 1800-2017`)에서는 수학 함수가 지원되지 않는다.
파라미터 계산에만 쓰이는 경우 elaboration-time constant로 평가되어 문제없을 수 있지만,
시뮬레이션 런타임에 수학 함수를 호출하는 testbench는 Icarus로 실행해야 한다.

---

## 합성 가능성

❌ 전 함수 비합성 — real 타입 연산은 합성 도구가 지원하지 않는다.
파라미터 선언의 elaboration-time 컨텍스트에서 `$clog2` 외의 수학 함수를 쓰는 경우
일부 도구는 허용하지만 표준 보장 없음.

---

## 본 프로젝트 구현 메모

> ⏳ **현재 미구현 (N6 트랙, loud-reject E3009)** — 아래는 향후 설계 메모다.
> 블로커: Rust `f64::ln()`/`sqrt()`/`sin()` 등은 플랫폼 libm으로 위임되어 OS마다 last-ulp가
> 달라질 수 있어 결정성 골든 게이트(3-OS 바이트 동일)와 충돌한다. 순수-Rust 결정론적
> transcendental 라이브러리 핀이 확정되어야 활성화된다.

- `hdl-builtins` 크레이트 `math` 카테고리 담당
- Rust `f64` 표준 메서드로 1:1 매핑: `f64::ln()`, `f64::sqrt()`, `f64::sin()` 등
- 도메인 오류: Rust `f64` IEEE 754 동작을 그대로 위임 — NaN/Inf 발생 시 시뮬 경고 발행 여부는 설정 가능하게 구현 검토
- `$floor`/`$ceil`: 반환 타입 `real` 유지 (C와 동일 — 정수 변환은 호출자 책임)
- `$atan2`: 인자 순서 `(y, x)` — Rust `f64::atan2(y, x)` = `y.atan2(x)`와 동일
- Phase 3 함수(`$asinh`, `$acosh`, `$atanh`, `$hypot`): Rust `f64` 메서드 모두 존재, 추후 추가 용이

## Sources

- IEEE 1800-2017 §20.8.2 (mathematical functions)
- research-log: [system-tasks-conversion-math-2026-05-28.md](../../research-log/system-tasks-conversion-math-2026-05-28.md)
- [circuitcove.com — Math Functions](https://circuitcove.com/system-tasks-math/) (WebFetch ✓)
- [chipverify.com — Verilog Math Functions](https://chipverify.com/verilog/verilog-math-functions) (WebFetch ✓)
- [verilator.org — Input Languages](https://verilator.org/guide/latest/languages.html) (WebFetch ✓)
