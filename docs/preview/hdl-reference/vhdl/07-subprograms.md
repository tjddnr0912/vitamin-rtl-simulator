# 07 · VHDL Subprograms — Function & Procedure

IEEE 1076-2008 §4 기준. Subprogram(하위 프로그램)은 **function**과 **procedure** 두 종류다. 반복 로직의 추상화, 연산자 오버로딩, 테스트벤치 유틸리티 작성에 핵심적으로 사용한다.

---

## Function vs Procedure 비교

| 항목 | function | procedure |
|------|---------|-----------|
| 반환값 | 정확히 1개 (`return TYPE`) | 없음 (`return;`으로 조기 탈출만) |
| 호출 문맥 | **expression** (값이 필요한 곳) | **statement** (문장 위치) |
| 파라미터 모드 | `in`만 허용 | `in` / `out` / `inout` 모두 허용 |
| 파라미터 클래스 | constant, signal, file (`in`) | constant, signal, variable, file |
| `wait` 문 | 포함 불가 | 포함 가능 (시뮬레이션) |
| 순수성(purity) | `pure`(기본) / `impure` 선택 | 본질적으로 impure |
| concurrent 호출 | 불가 (expression이므로) | 가능 (concurrent procedure call) |

---

## Function

### 문법

```vhdl
[pure|impure] function FUNC_NAME (
    param1 : [constant|signal|file] in type_name [:= default_val];
    param2 : in type_name [:= default_val]
) return return_type is
    -- 선언부: 변수, 상수, 중첩 subprogram 등
begin
    -- 순차 문장
    return expression;
end [function] [FUNC_NAME];
```

- `pure`가 기본값이므로 생략 가능. 불순(impure)임을 명시할 때만 `impure` 키워드를 쓴다.
- **함수 선언(declaration)** 은 별도 작성 선택적 — body만으로 충분.
- `return` 문은 반드시 포함해야 한다.

### pure vs impure

| | `pure` (기본) | `impure` |
|-|--------------|---------|
| 동일 인자 → 동일 반환 | **보장** | 보장 안 됨 |
| 스코프 외부 객체 접근 | **불가** (shared variable, file 등) | 가능 |
| impure function 호출 | **불가** | 가능 |
| 합성 | 제한 없음 | 외부 상태 의존 시 비합성 |
| 대표 용례 | 조합 논리, 형 변환, 수학 연산 | 난수 생성, 파일 stimulus 읽기 |

```vhdl
-- pure function: 조합 로직 추상화
pure function parity(v : std_logic_vector) return std_logic is
    variable p : std_logic := '0';
begin
    for i in v'range loop
        p := p xor v(i);
    end loop;
    return p;
end function parity;

-- impure function: 공유 상태 접근 (testbench 용)
shared variable seed1 : integer := 42;
shared variable seed2 : integer := 17;

impure function rand_int(lo, hi : integer) return integer is
    variable r : real;
begin
    uniform(seed1, seed2, r);               -- math_real uniform()
    return lo + integer(r * real(hi - lo));
end function rand_int;
```

### 파라미터 기본값

```vhdl
-- 기본값이 있는 파라미터는 호출 시 생략 가능
function to_slv(
    val   : integer;
    width : natural := 8         -- 기본값: 8
) return std_logic_vector is
begin
    return std_logic_vector(to_unsigned(val, width));
end function;

-- 호출 예시
signal a : std_logic_vector(7 downto 0) := to_slv(42);       -- width=8 생략
signal b : std_logic_vector(15 downto 0) := to_slv(42, 16);  -- 명시
```

### 재귀 함수

```vhdl
-- 컴파일 시간에 깊이가 결정돼야 합성 가능
function clog2(n : positive) return natural is
begin
    if n <= 1 then
        return 0;
    else
        return 1 + clog2((n + 1) / 2);   -- 재귀 호출
    end if;
end function clog2;

-- 용례: 주소 버스 폭 계산 (elaboration time 상수로만 사용)
constant ADDR_WIDTH : natural := clog2(MEM_DEPTH);
```

> **합성 주의:** 재귀 함수는 elaboration 단계에서 정적으로 unroll돼야 한다. 런타임에 깊이가 결정되는 재귀는 비합성.

---

## Procedure

### 문법

```vhdl
procedure PROC_NAME (
    signal   clk   : in    std_logic;
    variable data  : out   integer;
    signal   bus   : inout std_logic_vector(7 downto 0);
    constant LIMIT : in    integer := 255    -- 기본값 지원
) is
    -- 선언부
begin
    -- 순차 문장 (wait 포함 가능)
    [return;]   -- 선택적, 조기 종료
end [procedure] [PROC_NAME];
```

- **모드별 기본 클래스**: `in` → constant, `out`/`inout` → variable (signal 명시하면 signal).
- `out` / `inout` 파라미터를 통해 여러 값을 반환하는 것처럼 사용.

### 파라미터 모드

```vhdl
procedure add_with_carry (
    a, b   : in  unsigned(7 downto 0);
    result : out unsigned(8 downto 0)   -- 9비트로 올림수 포함
) is
begin
    result := ('0' & a) + ('0' & b);
end procedure;

-- 호출
procedure_result : process(a, b)
    variable sum9 : unsigned(8 downto 0);
begin
    add_with_carry(a, b, sum9);
    carry  <= sum9(8);
    result <= sum9(7 downto 0);
end process;
```

### signal 파라미터 + wait (testbench 용)

```vhdl
-- testbench에서 SPI write 시퀀스 캡슐화
procedure spi_write (
    signal sck   : out std_logic;
    signal mosi  : out std_logic;
    signal cs_n  : out std_logic;
    data         : in  std_logic_vector(7 downto 0);
    constant T   : in  time := 10 ns
) is
begin
    cs_n <= '0';
    for i in 7 downto 0 loop
        sck  <= '0';
        mosi <= data(i);
        wait for T;
        sck  <= '1';
        wait for T;
    end loop;
    sck  <= '0';
    cs_n <= '1';
    wait for T;
end procedure;

-- testbench process에서 호출
spi_write(sck, mosi, cs_n, X"A5");
```

### Concurrent Procedure Call

```vhdl
-- 아키텍처 body 레벨(concurrent 영역)에서 직접 호출 가능
-- 내부적으로 자동으로 process로 래핑됨
LABEL : PROC_NAME(port_or_signal_list);

-- 예시: 동일 아키텍처에서 버스 모니터 연속 실행
bus_mon : monitor_bus(clk, addr, data, wr_en);
```

---

## Subprogram Overloading

같은 이름으로 다른 타입 시그니처를 가진 subprogram을 여러 개 정의할 수 있다. 컴파일러가 인자 타입을 보고 올바른 버전을 선택한다.

```vhdl
-- 타입별 오버로딩
function to_slv(val : integer;  width : natural) return std_logic_vector;
function to_slv(val : unsigned)                  return std_logic_vector;
function to_slv(val : boolean)                   return std_logic_vector;

-- 호출 시 자동 선택
signal a : std_logic_vector(7 downto 0) := to_slv(42, 8);   -- 첫 번째
signal b : std_logic_vector(7 downto 0) := to_slv(u_val);   -- 두 번째
signal c : std_logic_vector(0 downto 0) := to_slv(true);    -- 세 번째
```

---

## 연산자 오버로딩 (Operator Overloading)

함수 이름을 **연산자 문자열**로 지정하면 해당 연산자를 새 타입에 대해 정의할 수 있다.

### 오버로딩 가능 연산자

| 분류 | 연산자 |
|------|--------|
| 산술 | `+` `-` `*` `/` `**` `mod` `rem` `abs` |
| 비교 | `=` `/=` `<` `<=` `>` `>=` |
| 논리 | `and` `or` `nand` `nor` `xor` `xnor` `not` |
| 시프트 | `sll` `srl` `sla` `sra` `rol` `ror` |
| 연결 | `&` |

### 정의 문법

```vhdl
-- 연산자 함수: 이름이 큰따옴표로 감싼 연산자 기호
function "+" (L, R : my_vec_t) return my_vec_t is
begin
    return my_vec_t(unsigned(L) + unsigned(R));
end "+";

function "<" (L, R : my_rec_t) return boolean is
begin
    return L.value < R.value;
end "<";
```

### Package에서 정의하는 패턴 (권장)

```vhdl
package my_types_pkg is
    type q16_t is array (15 downto 0) of std_logic;  -- 16비트 고정소수점

    function "+" (L, R : q16_t) return q16_t;
    function "-" (L, R : q16_t) return q16_t;
    function "*" (L, R : q16_t) return q16_t;
end package;

package body my_types_pkg is
    function "+" (L, R : q16_t) return q16_t is
    begin
        return q16_t(signed(L) + signed(R));
    end "+";
    -- ... 나머지 구현
end package body;
```

연산자 오버로딩은 `ieee.std_logic_1164`, `ieee.numeric_std` 패키지 내부에서도 이 방식으로 std_logic/unsigned/signed 연산을 정의한다.

---

## Subprogram Declarations vs Bodies

```vhdl
package util_pkg is
    -- 선언(declaration): 시그니처만
    function parity(v : std_logic_vector) return std_logic;
    procedure check_range(val, lo, hi : in integer);
end package;

package body util_pkg is
    -- 본체(body): 실제 구현
    function parity(v : std_logic_vector) return std_logic is
        variable p : std_logic := '0';
    begin
        for i in v'range loop p := p xor v(i); end loop;
        return p;
    end function;

    procedure check_range(val, lo, hi : in integer) is
    begin
        assert val >= lo and val <= hi
            report "Value " & integer'image(val) & " out of range"
            severity ERROR;
    end procedure;
end package body;
```

- **Package spec**에 선언, **package body**에 구현.
- 단독 entity/architecture 내에서는 body만으로 충분 (선언 선택적).
- 선언과 body가 분리돼 있으면 전방 참조(forward reference) 가능.

---

## 합성 체크리스트

| 패턴 | 합성 결과 | 비고 |
|------|----------|------|
| pure function, no wait | ✅ 합성 가능 | 조합 논리로 변환 |
| impure function (외부 상태 없음) | ✅ 합성 가능 | 실질적으로 pure |
| impure function (shared var 접근) | ❌ 비합성 | 글로벌 상태 = 비합성 |
| function with wait | ❌ 비합성 | wait = 비합성 |
| procedure (no wait, no signal param) | ✅ 합성 가능 | |
| procedure with wait | ❌ 비합성 | testbench 전용 |
| recursion (정적 깊이) | ✅ 합성 가능 | elaboration 시 unroll |
| recursion (동적 깊이) | ❌ 비합성 | 런타임 깊이 불가 |
| operator overloading | ✅ 합성 가능 | 구현 함수가 합성 가능이면 |
| overloaded operator with file I/O | ❌ 비합성 | file I/O 포함 시 |

---

## Sources

- IEEE 1076-2008 §4 (Subprograms and packages)
- Azimuth — VHDL Function vs Procedure (2025-02-23): https://azimuth.tech/2025/02/23/whats-the-difference-between-a-vhdl-function-or-procedure/ ✓ WebFetch 검증
- VHDL-Online — Subprograms: https://www.vhdl-online.de/courses/system_design/vhdl_language_and_syntax/subprograms ✓ WebFetch 검증
- HDLworks — Function reference: https://www.hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/Function.htm ✓ WebFetch 검증
- HDLworks — Operator Overloading: https://www.hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/OperatorOverloading.htm
- Research log: [vhdl-subprograms-pkg-synth-2026-05-28.md](../../research-log/vhdl-subprograms-pkg-synth-2026-05-28.md)
