# 08 · VHDL 패키지 & 표준 라이브러리

IEEE 1076-2008 §4 (package 문법), §16 (standard packages) 기준.

---

## Package 문법

패키지(package)는 타입, 상수, subprogram, 컴포넌트 선언을 하나의 네임스페이스로 묶는다.

```vhdl
-- 패키지 선언 (spec)
package PKG_NAME is
    -- 타입 선언
    type my_state_t is (IDLE, RUN, DONE);

    -- 상수
    constant CLK_FREQ : integer := 100_000_000;

    -- subprogram 선언 (시그니처만)
    function to_slv(val : my_state_t) return std_logic_vector;

    -- 컴포넌트 선언 (선택적 — VHDL-2008에서는 direct instantiation 권장)
    component uart_rx
        port (clk, rx : in std_logic; data : out std_logic_vector(7 downto 0));
    end component;
end package PKG_NAME;

-- 패키지 본체 (body) — subprogram 구현 필요 시
package body PKG_NAME is
    function to_slv(val : my_state_t) return std_logic_vector is
    begin
        return std_logic_vector(to_unsigned(my_state_t'pos(val), 2));
    end function;
end package body PKG_NAME;
```

### Library + Use 절

```vhdl
library WORK;                   -- 현재 작업 라이브러리 (기본 포함됨)
use work.PKG_NAME.all;          -- 패키지 전체 가시화

library IEEE;
use ieee.std_logic_1164.all;    -- IEEE 패키지
use ieee.numeric_std.all;
```

---

## 표준 패키지 전체 목록

| 패키지 | 합성 | 용도 요약 |
|--------|------|----------|
| `std.standard` | ✅ | 기본 타입 전체 (묵시적 — use 불필요) |
| `std.textio` | ❌ | 파일 I/O (testbench) |
| `ieee.std_logic_1164` | ✅ | 9값 로직 · std_logic 계열 |
| `ieee.numeric_std` | ✅ | signed/unsigned 산술 **[권장]** |
| `ieee.std_logic_arith` | ⚠️ | **DEPRECATED — 사용 금지** |
| `ieee.std_logic_unsigned` | ⚠️ | **DEPRECATED — 사용 금지** |
| `ieee.std_logic_signed` | ⚠️ | **DEPRECATED — 사용 금지** |
| `ieee.numeric_std_unsigned` | ✅ | std_logic_vector 직접 산술 (2008+) |
| `ieee.math_real` | ❌ | 수학 함수 (testbench/elaboration) |
| `ieee.fixed_pkg` | ✅⚠️ | 고정소수점 (2008+, 도구 지원 확인) |
| `ieee.float_pkg` | ✅⚠️ | 부동소수점 IEEE 754 (2008+, 면적 주의) |

---

## std.standard — 묵시적, use 불필요

모든 VHDL design unit에 자동으로 포함된다. 명시적 `use` 절 없이도 아래 타입을 사용할 수 있다.

```vhdl
-- use 절 없이 사용 가능
signal flag  : boolean;                         -- false / true
signal bit0  : bit;                             -- '0' / '1'
signal vec   : bit_vector(7 downto 0);
signal ch    : character;
signal str   : string(1 to 8);
signal n     : integer range 0 to 2**16-1;
signal nat   : natural;                         -- 0 to integer'high
signal pos   : positive;                        -- 1 to integer'high
signal r     : real;                            -- (합성 불가)
signal t     : time;                            -- (합성 불가)
```

**predefined subtype:**

| 이름 | 정의 |
|------|------|
| `natural` | `integer range 0 to integer'high` |
| `positive` | `integer range 1 to integer'high` |

---

## std.textio — 파일 I/O, testbench 전용

```vhdl
use std.textio.all;
```

```vhdl
-- 파일 읽기 예시 (testbench)
file input_file : text open READ_MODE is "stimulus.txt";
variable line_buf : line;
variable val      : integer;

process
begin
    while not endfile(input_file) loop
        readline(input_file, line_buf);     -- 한 줄 읽기
        read(line_buf, val);                -- 정수로 파싱
        data_in <= std_logic_vector(to_signed(val, 8));
        wait until rising_edge(clk);
    end loop;
    wait;
end process;
```

> **VHDL-2008:** `std_logic_textio`(std_logic read/write 절차)의 기능이 `ieee.std_logic_1164`로 통합됐다. `std_logic_textio`는 stub이 됐으나 하위 호환을 위해 여전히 선언할 수 있다.

---

## ieee.std_logic_1164 — 9값 로직 표준

```vhdl
library ieee;
use ieee.std_logic_1164.all;
```

### 9값 로직 (std_ulogic)

| 값 | 의미 |
|----|------|
| `'U'` | Uninitialized |
| `'X'` | Unknown (강함) |
| `'0'` | Logic 0 (강함) |
| `'1'` | Logic 1 (강함) |
| `'Z'` | High impedance |
| `'W'` | Unknown (약함) |
| `'L'` | Logic 0 (약함) |
| `'H'` | Logic 1 (약함) |
| `'-'` | Don't care |

### std_ulogic vs std_logic

```vhdl
-- std_ulogic: 단일 드라이버만 허용 (비결선형)
signal u : std_ulogic;

-- std_logic: 결선 함수(resolution function) 포함
-- → 여러 드라이버가 연결될 때 버스/tri-state 구현 가능
signal s : std_logic;                  -- 포트 기본 타입
signal v : std_logic_vector(7 downto 0);
```

합성 관점에서는 std_logic과 std_ulogic이 동일하게 처리된다.

### 변환 함수

```vhdl
-- std_logic ↔ bit 변환 (레거시 bit 타입 연동)
b  := to_bit(sl);                      -- std_logic → bit ('H'→'1', 'L'→'0')
bv := to_bitvector(slv);               -- std_logic_vector → bit_vector
sl := to_stdulogic(b);                 -- bit → std_ulogic
sl := to_stdlogic(b);                  -- bit → std_logic (VHDL-2008+)
sv := to_stdlogicvector(bv);           -- bit_vector → std_logic_vector
```

### 주요 연산자 (오버로딩)

`and`, `or`, `nand`, `nor`, `xor`, `xnor`, `not` — std_logic/_vector에 정의됨.

```vhdl
-- 리듀션 연산자 (VHDL-2008+)
result <= and  slv;    -- 모든 비트 AND
result <= or   slv;    -- 모든 비트 OR
result <= xor  slv;    -- 모든 비트 XOR (홀수 패리티)
result <= nand slv;
result <= nor  slv;
result <= xnor slv;    -- 모든 비트 XNOR (짝수 패리티)
```

---

## ieee.numeric_std — 산술 표준 **[신규 설계 필수]**

```vhdl
library ieee;
use ieee.numeric_std.all;
```

`signed`와 `unsigned` 두 타입을 정의. 내부는 std_logic 배열 기반.

```vhdl
signal u : unsigned(7 downto 0) := to_unsigned(200, 8);  -- 무부호
signal s : signed(7 downto 0)  := to_signed(-100, 8);    -- 2의 보수
```

### 산술/비교 연산

```vhdl
-- 오버플로 주의: 결과 폭을 직접 관리
signal a, b : unsigned(7 downto 0);
signal sum9 : unsigned(8 downto 0);

sum9 <= ('0' & a) + ('0' & b);         -- 9비트로 올림수 포함

-- 비교 (숫자적 해석)
if unsigned(addr) < to_unsigned(BASE, 16) then ...
```

### 변환 함수

```vhdl
-- signed/unsigned → integer
n := to_integer(u_val);               -- unsigned → integer (항상 >= 0)
n := to_integer(s_val);               -- signed → integer (음수 포함)

-- integer → signed/unsigned (크기 명시 필수)
u := to_unsigned(42, 8);             -- 8비트 unsigned
s := to_signed(-5,  8);              -- 8비트 signed

-- 크기 변경
u2 := resize(u, 16);                 -- unsigned: 0 확장 (zero-extend)
s2 := resize(s, 16);                 -- signed: 부호 확장 (sign-extend)
```

### 시프트 / 회전

```vhdl
-- 시프트 (빈 자리: 0 채움)
u_shifted := shift_left (u, 3);       -- 논리 좌시프트 (*8)
u_shifted := shift_right(u, 3);       -- 논리 우시프트 unsigned: 0 채움
s_shifted := shift_right(s, 3);       -- 산술 우시프트 signed: MSB 복제

-- 회전
u_rot := rotate_left (u, 2);
u_rot := rotate_right(u, 2);
```

### 형 변환 패턴

```vhdl
-- std_logic_vector ↔ unsigned/signed 캐스팅
u_val := unsigned(slv);               -- 재해석 (비트 복사 없음)
s_val := signed(slv);
slv   := std_logic_vector(u_val);
```

---

## ieee.std_logic_arith — ⚠️ DEPRECATED, 사용 금지

```vhdl
-- 아래 패키지들은 절대 신규 설계에 사용하지 않는다
-- use ieee.std_logic_arith.all;      -- ❌ DEPRECATED
-- use ieee.std_logic_unsigned.all;   -- ❌ DEPRECATED
-- use ieee.std_logic_signed.all;     -- ❌ DEPRECATED
```

**왜 위험한가:**

1. **비표준** — Synopsys가 작성. IEEE가 공식 표준화한 적 없음.
2. **이식성 파괴** — Synopsys/Cadence/Mentor 구현이 서로 다름. 동일한 IEEE namespace에 호환 안 되는 타입 정의.
3. **충돌** — `std_logic_arith`의 `SIGNED`/`UNSIGNED`가 `numeric_std`의 것과 **다른 타입**. 두 패키지를 함께 사용하면 컴파일 오류.
4. **상호 배타** — `std_logic_signed`와 `std_logic_unsigned`를 같은 design unit에서 동시 사용 불가.

**대체:**

```vhdl
-- 신규 설계 표준 조합
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;              -- 모든 산술은 여기서
```

---

## ieee.numeric_std_unsigned — VHDL-2008+

```vhdl
use ieee.numeric_std_unsigned.all;
```

`std_logic_vector`를 캐스팅 없이 직접 산술 연산할 수 있다. `std_logic_unsigned`의 표준 대체제.

```vhdl
-- numeric_std_unsigned 사용 시
signal a, b, c : std_logic_vector(7 downto 0);
c <= a + b;                            -- unsigned 산술 직접 적용
c <= a + "00000001";                   -- 상수와도 가능
if a > b then ...                      -- 숫자 비교
```

> `ieee.numeric_std`와 동시에 `use`하면 `+`, `<` 등의 연산자가 중의적이 될 수 있다. 같은 design unit에서 혼용 주의.

---

## ieee.math_real — 수학 함수, testbench 전용

```vhdl
use ieee.math_real.all;
```

```vhdl
-- 상수
MATH_PI        -- 3.14159...
MATH_E         -- 2.71828...
MATH_SQRT2     -- 1.41421...
MATH_LOG2E     -- log2(e)

-- 함수
sqrt(x)        ceil(x)    floor(x)   round(x)
log(x)         log2(x)    log10(x)   exp(x)
sin(x)         cos(x)     tan(x)
arcsin(x)      arccos(x)  arctan(x)  arctan2(y, x)
uniform(s1, s2, r)  -- 균일 분포 난수 [0.0, 1.0)
```

**합성 불가.** testbench stimulus 생성, elaboration 시간 상수 계산에만 사용.

```vhdl
-- 용례: CLK_PERIOD에서 파라미터 계산 (elaboration constant)
constant SAMPLES : integer := integer(ceil(MATH_PI * real(N)));

-- 난수 기반 stimulus (testbench)
impure function rand_slv(len : natural) return std_logic_vector is
    variable r    : real;
    variable s1, s2 : integer := 47;
    variable result : std_logic_vector(len-1 downto 0);
begin
    for i in result'range loop
        uniform(s1, s2, r);
        result(i) := '1' when r > 0.5 else '0';
    end loop;
    return result;
end function;
```

---

## ieee.fixed_pkg — 고정소수점 (VHDL-2008+)

```vhdl
use ieee.fixed_pkg.all;
```

### 타입 및 인덱스 표기

```vhdl
-- sfixed(정수부_MSB downto 소수부_LSB)
-- 이진 소수점: 인덱스 0과 -1 사이
signal x : sfixed( 7 downto -8);   -- 8.8 포맷: 16비트, ±127.996
signal y : ufixed( 7 downto -8);   -- 8.8 포맷: 16비트, 0 ~ 255.996
signal z : sfixed(15 downto -16);  -- 16.16 포맷: 32비트
```

```vhdl
-- 연산 예시
signal a, b : sfixed(7 downto -8);
signal c    : sfixed(8 downto -8);   -- +1비트: 오버플로 방지

c <= a + b;                           -- 자동 소수점 정렬
c <= resize(a + b, c'high, c'low);    -- 명시적 크기 조정
```

합성 가능 (Vivado 지원). 일부 구형 도구는 부분 지원 — 사용 전 툴 확인 권장.

---

## ieee.float_pkg — 부동소수점 IEEE 754 (VHDL-2008+)

```vhdl
use ieee.float_pkg.all;
```

```vhdl
signal f32 : float32;                     -- IEEE 754 단정밀도
signal f64 : float64;                     -- IEEE 754 배정밀도

-- integer/real ↔ float 변환
f32 <= to_float(42, f32);                 -- integer → float32
f32 <= to_float(3.14, f32);              -- real → float32
n   := to_integer(f32);
r   := to_real(f32);                      -- (elaboration time only)
```

합성 가능하나 **상당한 LUT 면적**을 소비한다. FPGA 설계에서는 IP 코어(Vivado Floating Point IP 등) 사용을 먼저 검토.

---

## 권장 use 절 조합

```vhdl
-- RTL 설계 (합성 대상)
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

-- testbench
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
use ieee.math_real.all;
use std.textio.all;

-- 고정소수점 RTL (VHDL-2008+)
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
use ieee.fixed_pkg.all;
```

---

## Sources

- IEEE 1076-2008 §4 (Packages and package bodies), §16 (Predefined packages)
- HDL Factory — VHDL IEEE Libraries and Numeric Type Conversion (2025): https://www.hdlfactory.com/post/2025/06/29/vhdl-ieee-libraries-and-numeric-type-conversion-a-definitive-reference/ ✓ WebFetch 검증
- Sigasi — Deprecated IEEE Libraries: https://www.sigasi.com/tech/deprecated-ieee-libraries/ ✓ WebFetch 검증
- Doulos — VHDL-2008: Incorporates existing standards: https://www.doulos.com/knowhow/vhdl/vhdl-2008-incorporates-existing-standards/
- VHDL-2008 Support Library (fphdl ReadTheDocs): https://fphdl.readthedocs.io/en/docs/
- HDLworks — Std_Logic_1164: https://www.hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/StdLogic1164.htm
- Research log: [vhdl-subprograms-pkg-synth-2026-05-28.md](../../research-log/vhdl-subprograms-pkg-synth-2026-05-28.md)
