# Research Log — VHDL Subprograms · Packages · Synthesizability
**Date:** 2026-05-28
**Standard:** IEEE 1076-2008
**Coverage:** (A) Subprograms, (B) Standard Packages, (C) Synthesizable Subset

---

## 조사 방법

WebSearch 4라운드 + WebFetch 5회 1차 source 검증.
주요 참조: IEEE 1076-2008 LRM §4/§15/§16, Doulos VHDL-2008 guide, AMD Vivado UG901, Sigasi deprecated libraries, HDL Factory VHDL IEEE reference, VHDL-Online tutorial.

---

## A. Subprograms (IEEE 1076-2008 §4)

### function

```vhdl
[pure|impure] function FUNC_NAME (
    param1 : [class] in  type_name [:= default];
    ...
) return return_type is
    -- 선언부 (변수, 상수, 중첩 subprogram 등)
begin
    -- 순차 문장
    return expression;
end [function] [FUNC_NAME];
```

- **기본값: pure.** `impure` 키워드를 명시해야 impure가 된다.
- **pure function**: 동일 인자 → 동일 반환값 보장. 스코프 외부 객체(shared variable 등) 접근 불가. 다른 impure function 호출 불가.
- **impure function**: 동일 인자라도 다른 값 반환 가능. shared variable·file 등 외부 상태 읽기/쓰기 가능. 대표 용례: 난수 생성기, 파일에서 stimulus 읽기.
- **파라미터 모드**: 오직 `in`만 허용 (out/inout 불가). 클래스: constant(기본), signal, file.
- **기본값**: `:= expression` 형태로 선언 가능.
- **wait 문 포함 불가.** return 문 필수.
- **재귀 호출** 지원 (synthesis 시 깊이가 정적으로 bound돼야 함).
- 함수 선언(declaration)은 선택적; body만으로 충분.

[출처: Azimuth, 2025-02-23 — LRM §4.2.2.2/§4.2.2.3; HDLworks Function ref]

### procedure

```vhdl
procedure PROC_NAME (
    signal   clk   : in  std_logic;
    variable data  : out integer;
    signal   bus   : inout std_logic_vector(7 downto 0)
) is
begin
    -- 순차 문장 (wait 포함 가능)
    [return;]  -- 선택적, 조기 종료
end [procedure] [PROC_NAME];
```

- **반환값 없음.** 필요 시 `return;`으로 조기 종료 가능.
- **파라미터 모드**: in / out / inout 모두 허용.
- **파라미터 클래스**: in → constant/signal, out/inout → variable/signal.
- 기본 모드: in. 기본 클래스: in → constant, out/inout → variable.
- **concurrent procedure call** 가능 (concurrent 영역에서 호출 시 자동으로 process 래핑).
- LRM 관점에서 모든 procedure는 impure로 취급 가능.

[출처: VHDL-Online subprograms tutorial; Azimuth 2025-02-23 — LRM §6.4.2.1]

### Overloading

같은 이름, 다른 타입 시그니처를 가진 subprogram을 여러 개 정의.

```vhdl
-- 일반 오버로딩
function to_slv(val : integer; width : natural) return std_logic_vector;
function to_slv(val : boolean) return std_logic_vector;

-- 연산자 오버로딩
function "+" (L, R : my_fixed_t) return my_fixed_t is
begin
    return my_fixed_t(to_integer(L) + to_integer(R));
end "+";
```

- 오버로딩 가능한 연산자: `+` `-` `*` `/` `**` `mod` `rem` `&` `=` `/=` `<` `<=` `>` `>=` `not` `and` `or` `nand` `nor` `xor` `xnor` `sll` `srl` `sla` `sra` `rol` `ror` `abs`
- 연산자 오버로딩은 보통 package body에서 정의 (ieee.std_logic_1164, ieee.numeric_std가 내부적으로 이 방식 사용).

[출처: HDLworks Operator Overloading ref; VHDL-Online]

---

## B. Standard Packages (IEEE 1076-2008 §15/§16)

### std.standard — 묵시적(implicit)

- **use 절 불필요.** 모든 design unit에 자동 포함.
- 정의된 타입: `boolean`, `bit`, `bit_vector`, `character`, `string`, `integer`, `natural`, `positive`, `real`, `time`, `delay_length`, `severity_level`, `file_open_kind`, `file_open_status`
- `natural`: `integer range 0 to integer'high`
- `positive`: `integer range 1 to integer'high`
- 모든 타입에 대한 predefined operator 포함.

[출처: HDLworks Standard Package ref; UMBC stdpkg.html]

### std.textio — 비합성, testbench 전용

```vhdl
use std.textio.all;
```

- `text`, `line` 타입 + `read`, `write`, `readline`, `writeline` 절차 제공.
- **비합성.** 시뮬레이션·testbench 전용.
- VHDL-2008: `std_logic_textio`의 기능이 `ieee.std_logic_1164`로 통합됨 (std_logic_textio는 stub 패키지가 됨).

### ieee.std_logic_1164 — 9값 로직, 합성 가능

```vhdl
library ieee;
use ieee.std_logic_1164.all;
```

| 타입 | 설명 |
|------|------|
| `std_ulogic` | 9값 비결선형 로직: U X 0 1 Z W L H - |
| `std_logic` | std_ulogic + resolution function (다중 드라이버 허용) |
| `std_logic_vector` | std_logic의 배열 |

**Resolution**: 여러 드라이버가 충돌할 때 결선 테이블로 값 결정 (std_ulogic는 단일 드라이버만).

**변환 함수:**

| 함수 | 방향 |
|------|------|
| `to_bit(sl)` | std_logic → bit |
| `to_bitvector(slv)` | std_logic_vector → bit_vector |
| `to_stdulogic(b)` | bit → std_ulogic |
| `to_stdlogicvector(bv)` | bit_vector → std_logic_vector |

VHDL-2008: `std_logic_textio` 기능 흡수 (std_logic에 대한 read/write 절차 내장).
**합성 가능: YES.**

[출처: HDL Factory 2025; HDLworks Std_Logic_1164 ref]

### ieee.numeric_std — 산술 연산 표준, 합성 가능

```vhdl
use ieee.numeric_std.all;
```

- `unsigned`, `signed` 타입 정의 (내부는 std_logic 배열 기반).
- 산술: `+`, `-`, `*`, `/`, `mod`, `rem`, `abs`, `**`
- 비교: `=`, `/=`, `<`, `<=`, `>`, `>=` (숫자적 해석)

**변환 함수:**

| 함수 | 설명 |
|------|------|
| `to_integer(v)` | signed/unsigned → integer |
| `to_signed(i, n)` | integer → signed(n-1 downto 0) |
| `to_unsigned(i, n)` | integer → unsigned(n-1 downto 0) |
| `resize(v, n)` | signed: 부호 확장 / unsigned: 0 확장 |

**시프트/회전 함수:**

| 함수 | 설명 |
|------|------|
| `shift_left(v, n)` | 논리 좌시프트 (zero-fill) |
| `shift_right(v, n)` | unsigned → 논리 / signed → 산술 (MSB 복제) |
| `rotate_left(v, n)` | 순환 좌시프트 |
| `rotate_right(v, n)` | 순환 우시프트 |

**합성 가능: YES.** 모든 신규 설계에서 std_logic_arith 대신 이 패키지 사용 권장.

[출처: HDL Factory 2025 (WebFetch 검증); Sigasi deprecated libs]

### ieee.std_logic_arith — ⚠️ DEPRECATED (사용 금지)

- Synopsys가 제작한 비표준 패키지. IEEE가 공식 표준화한 적 없음.
- Synopsys/Cadence/Mentor 각사의 구현이 서로 달라 이식성 파괴.
- `SIGNED`/`UNSIGNED` 타입 정의가 numeric_std와 충돌.
- **모든 신규 설계에서 사용 금지.** `ieee.numeric_std`로 대체.

[출처: Sigasi deprecated-ieee-libraries (WebFetch 검증)]

### ieee.std_logic_signed / ieee.std_logic_unsigned — ⚠️ DEPRECATED

- std_logic_vector에 부호/무부호 해석을 암묵적으로 부여.
- 두 패키지를 동시에 사용하면 연산자 충돌 → **상호 배타적.**
- `ieee.numeric_std` 또는 `ieee.numeric_std_unsigned`(2008+)로 대체.

### ieee.numeric_std_unsigned — VHDL-2008+, 합성 가능

```vhdl
use ieee.numeric_std_unsigned.all;
```

- std_logic_vector를 unsigned로 캐스팅 없이 직접 산술 연산 가능.
- std_logic_unsigned의 표준 대체제 (2008 이후).
- **합성 가능: YES.**

[출처: HDL Factory 2025]

### ieee.math_real — 비합성, testbench 전용

```vhdl
use ieee.math_real.all;
```

- 상수: `MATH_PI`, `MATH_E`, `MATH_SQRT2` 등
- 함수: `sqrt`, `log`, `log2`, `log10`, `sin`, `cos`, `tan`, `arctan`, `exp`, `ceil`, `floor`, `round`, `uniform`(난수)
- **비합성.** 주로 testbench에서 stimulus 생성, constraint 계산 등에 사용.

[출처: HDL Factory 2025; VHDL-2008 Support Library docs]

### ieee.fixed_pkg — 고정소수점, 합성 가능 (VHDL-2008+)

```vhdl
use ieee.fixed_pkg.all;
```

- `ufixed`: unsigned 고정소수점, `sfixed`: signed 고정소수점.
- **인덱스가 음수 포함 가능**: `sfixed(7 downto -8)` = 8.8 포맷 (16비트, 정수부 8비트 + 소수부 8비트).
- 이진 소수점: 인덱스 0과 -1 사이.
- **합성 가능: YES** (Vivado OK; 일부 구형 도구 partial).

### ieee.float_pkg — 부동소수점, 합성 가능 (VHDL-2008+)

```vhdl
use ieee.float_pkg.all;
```

- IEEE 754 호환 부동소수점.
- **합성 가능: YES** (상당한 로직 면적 소모 — FPGA에서 신중히 사용).
- `float32`, `float64` 타입 제공.

[출처: HDL Factory 2025; VHDL-2008 Support Library docs (ReadTheDocs)]

---

## C. Synthesizable Subset 주요 포인트

| 구분 | 항목 | 비고 |
|------|------|------|
| ✅ 합성 가능 | std_logic/_vector, integer (range 필수), boolean, enum | 범용 |
| ✅ 합성 가능 | process (complete sensitivity), if/case (others 포함) | 범용 |
| ✅ 합성 가능 | for-loop (정적 bound), generate | 범용 |
| ✅ 합성 가능 | function/procedure (wait 없음, 정적 재귀) | 조건부 |
| ✅ 합성 가능 | numeric_std, std_logic_1164 연산자 | 범용 |
| ⚠️ 조건부 | record (신호/변수: OK; port: Vivado 2019+ OK, 일부 도구 미지원) | 도구 의존 |
| ⚠️ 조건부 | shared variable (process-local variable은 OK) | 도구 의존 |
| ⚠️ 조건부 | recursion (정적 깊이 bound 필수) | 도구 의존 |
| ⚠️ 조건부 | fixed_pkg/float_pkg (면적 비용) | 합성은 되나 비용 큼 |
| ⚠️ 조건부 | case-generate (VHDL-2008, 부분 지원) | 도구 의존 |
| ❌ 비합성 | file 타입, textio | 시뮬레이션 전용 |
| ❌ 비합성 | access 타입 (포인터/동적 메모리) | 시뮬레이션 전용 |
| ❌ 비합성 | wait for (시간 지정 wait) | testbench |
| ❌ 비합성 | after (지연 신호 할당: `sig <= val after 10 ns`) | testbench |
| ❌ 비합성 | real 타입 산술, math_real | 시뮬레이션 전용 |
| ❌ 비합성 | 무한 루프 (정적 exit 없음) | 합성 불가 |
| ❌ 비합성 | 전역 상태에 의존하는 impure function | 합성 불가 |

[출처: AMD Vivado UG901; Intel Quartus support article; EDAboard real type discussion]

---

## Sources

| 출처 | URL | 검증 |
|------|-----|------|
| Azimuth — VHDL Function vs Procedure | https://azimuth.tech/2025/02/23/whats-the-difference-between-a-vhdl-function-or-procedure/ | WebFetch ✓ |
| HDL Factory — VHDL IEEE Libraries reference | https://www.hdlfactory.com/post/2025/06/29/vhdl-ieee-libraries-and-numeric-type-conversion-a-definitive-reference/ | WebFetch ✓ |
| Sigasi — Deprecated IEEE Libraries | https://www.sigasi.com/tech/deprecated-ieee-libraries/ | WebFetch ✓ |
| VHDL-Online — Subprograms | https://www.vhdl-online.de/courses/system_design/vhdl_language_and_syntax/subprograms | WebFetch ✓ |
| HDLworks — Function reference | https://www.hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/Function.htm | WebFetch ✓ |
| AMD Vivado UG901 — VHDL Constructs Support | https://docs.amd.com/r/en-US/ug901-vivado-synthesis/VHDL-Constructs-Support-Status | Referenced |
| Doulos — VHDL-2008 Incorporates existing standards | https://www.doulos.com/knowhow/vhdl/vhdl-2008-incorporates-existing-standards/ | Referenced |
| VHDL-2008 Support Library (ReadTheDocs) | https://fphdl.readthedocs.io/en/docs/ | Referenced |
