# 02 · VHDL 타입 시스템

IEEE 1076-2008 §5 기준. `std` 및 `ieee` 패키지 포함.

---

## 타입 분류 개요

```
타입 (type)
├── 스칼라 (scalar)
│   ├── 정수 (integer)          — integer, natural, positive
│   ├── 부동소수 (real)
│   ├── 물리 (physical)         — time
│   └── 열거 (enumeration)     — bit, boolean, character, severity_level, file_open_kind
├── 복합 (composite)
│   ├── 배열 (array)            — bit_vector, string, ...
│   └── 레코드 (record)
├── 액세스 (access)             — 포인터, 시뮬레이션 전용
└── 파일 (file)                 — 시뮬레이션 전용
```

---

## 스칼라 타입

### 정수 타입

```vhdl
-- 표준 패키지 선언
type integer is range -(2**31-1) to 2**31-1;

subtype natural  is integer range 0 to integer'high;
subtype positive is integer range 1 to integer'high;
```

| 타입 | 범위 |
|------|------|
| `integer` | −(2³¹−1) ~ 2³¹−1 (구현에 따라 더 넓을 수 있음) |
| `natural` | 0 이상 |
| `positive` | 1 이상 |

### 부동소수 타입

```vhdl
type real is range -1.0E308 to 1.0E308;
```

시뮬레이션·검증 전용. 합성 불가. 연산 후 정밀도는 구현 의존.

### 물리 타입 (Physical Type)

단위(unit)가 있는 정수. 시간 모델링에 필수.

```vhdl
-- 표준 패키지 time 정의 (축약)
type time is range -(2**63-1) to 2**63-1
  units
    fs;                 -- femtosecond (기준 단위)
    ps  = 1000 fs;
    ns  = 1000 ps;
    us  = 1000 ns;
    ms  = 1000 us;
    sec = 1000 ms;
    min = 60 sec;
    hr  = 60 min;
  end units;
```

사용 예:

```vhdl
constant CLK_PERIOD : time := 10 ns;
wait for 5 ns;
signal #: time := now;
```

### 열거 타입 (Enumeration)

```vhdl
type boolean        is (FALSE, TRUE);
type bit            is ('0', '1');
type severity_level is (NOTE, WARNING, ERROR, FAILURE);
type file_open_kind is (READ_MODE, WRITE_MODE, APPEND_MODE);
type file_open_status is (OPEN_OK, STATUS_ERROR, NAME_ERROR, MODE_ERROR);
```

`character`는 ISO-8859-1 기준 256개 문자의 열거형 (VHDL-1993부터).

**사용자 정의 열거**:
```vhdl
type state_t is (IDLE, FETCH, DECODE, EXECUTE, WRITEBACK);
signal state : state_t := IDLE;
```

---

## 복합 타입

### 배열 (Array)

**제약 배열(constrained array)**: 선언 시 범위 고정.

```vhdl
type byte_t      is array(7 downto 0) of bit;
type word_t      is array(15 downto 0) of bit;
type rom_256x8_t is array(0 to 255) of byte_t;
```

**비제약 배열(unconstrained array)**: `range <>` — 범위를 포트/제네릭/서브타입에서 결정.

```vhdl
-- 표준 패키지 정의
type bit_vector  is array(natural range <>) of bit;
type string      is array(positive range <>) of character;

-- 포트 선언 시 범위 지정
port (data_in : in bit_vector(7 downto 0));

-- 서브타입으로 제약
subtype byte_vec is bit_vector(7 downto 0);
```

**다차원 배열**:
```vhdl
type matrix_t is array(0 to 3, 0 to 3) of integer;
variable m : matrix_t;
m(0, 0) := 1;
```

**배열 속성(attributes)**:
| 속성 | 의미 |
|------|------|
| `a'length` | 요소 개수 |
| `a'left` | 좌측 인덱스 |
| `a'right` | 우측 인덱스 |
| `a'high` | 최대 인덱스 |
| `a'low` | 최소 인덱스 |
| `a'range` | 범위 (`low to high` 또는 `high downto low`) |
| `a'reverse_range` | 역방향 범위 |

### 레코드 (Record)

이종 필드의 묶음. SystemVerilog `struct`에 해당.

```vhdl
type axi_t is record
  data    : std_logic_vector(31 downto 0);
  valid   : std_logic;
  ready   : std_logic;
  last    : std_logic;
end record;

signal axi_bus : axi_t;
axi_bus.valid <= '1';
```

---

## 액세스 타입 (Access Type)

동적 메모리 할당 포인터. **시뮬레이션 전용, 합성 불가**.

```vhdl
type node_t;
type link_ptr is access node_t;
type node_t is record
  val  : integer;
  next : link_ptr;
end record;

variable head : link_ptr;
head := new node_t'(val => 0, next => null);
```

`deallocate(ptr)` 로 해제.

---

## 파일 타입 (File Type)

시뮬레이션 I/O. **합성 불가**.

```vhdl
type text is file of string;   -- 표준 패키지 정의
file my_file : text open READ_MODE is "input.txt";
```

---

## std_logic_1164 패키지

`library ieee; use ieee.std_logic_1164.all;`

IEEE Std 1164. VHDL 설계의 표준 로직 타입.

### std_ulogic — 9값 열거형

```vhdl
type std_ulogic is ('U','X','0','1','Z','W','L','H','-');
```

| 값 | 이름 | 시뮬레이션 의미 |
|----|------|----------------|
| `'U'` | Uninitialized | 초기화 전 상태. 시뮬레이션 시작 기본값 |
| `'X'` | Forcing Unknown | 강한 드라이버 충돌 또는 미결정 |
| `'0'` | Forcing 0 | 강한 로직 0 (VCC 접지) |
| `'1'` | Forcing 1 | 강한 로직 1 (VCC 연결) |
| `'Z'` | High Impedance | 트라이스테이트: 드라이버 비활성 |
| `'W'` | Weak Unknown | 약한 드라이버 충돌 |
| `'L'` | Weak 0 | 풀다운 저항 |
| `'H'` | Weak 1 | 풀업 저항 |
| `'-'` | Don't Care | 합성 최적화 힌트; 시뮬레이션에서는 `'X'` 동작 |

합성 도구가 실제로 인식하는 값: `'0'`, `'1'`, `'Z'`, `'-'`.

### std_logic — 해소 서브타입

```vhdl
function resolved(s : std_ulogic_vector) return std_ulogic;
subtype std_logic is resolved std_ulogic;
```

`std_ulogic`은 단일 드라이버만 허용. `std_logic`은 `resolved` 함수를 통해
다중 드라이버(버스 공유)를 지원한다. `resolved`는 9×9 결정 테이블로 대표값을 반환.

해소 테이블 주요 케이스:

| 드라이버 A | 드라이버 B | 결과 |
|-----------|-----------|------|
| `'0'` | `'0'` | `'0'` |
| `'1'` | `'1'` | `'1'` |
| `'0'` | `'1'` | `'X'` |
| `'0'` | `'Z'` | `'0'` |
| `'1'` | `'Z'` | `'1'` |
| `'Z'` | `'Z'` | `'Z'` |
| `'H'` | `'0'` | `'0'` |
| `'L'` | `'1'` | `'1'` |
| `'U'` | (any) | `'U'` |
| `'-'` | (any) | `'X'` |

### 배열 타입

```vhdl
type std_ulogic_vector is array (natural range <>) of std_ulogic;
-- VHDL-2008: std_logic_vector는 std_ulogic_vector 서브타입
subtype std_logic_vector is (resolved) std_ulogic_vector;
```

VHDL-2008 이전에는 `std_logic_vector`와 `std_ulogic_vector`가 별개 타입이어서
직접 대입 시 타입 변환(`std_logic_vector(...)`)이 필요했다.
2008부터는 서브타입 관계이므로 묵시적 변환이 가능하다.

### VHDL-2008 신규 기능

```vhdl
-- 리덕션 연산자
and_reduce(v)   -- 전체 AND
or_reduce(v)    -- 전체 OR
xor_reduce(v)   -- 홀수 패리티

-- 매칭 비교 (don't care '-' 처리)
a ?= b    -- matching equality
a ?/= b   -- matching inequality

-- 변환 함수
to_string(v)    -- "10110..."
to_hstring(v)   -- "FF"
to_ostring(v)   -- "377"
to_bstring(v)   -- = to_string
```

---

## numeric_std 패키지

`library ieee; use ieee.numeric_std.all;`

IEEE Std 1076.3. 합성 가능한 정수 산술.

### 타입 선언

```vhdl
type unsigned is array (natural range <>) of std_logic;
type signed   is array (natural range <>) of std_logic;
```

| 타입 | 해석 | 범위 (n비트) |
|------|------|-------------|
| `unsigned` | 비부호 정수 | 0 ~ 2ⁿ−1 |
| `signed` | 2의 보수 | −2ⁿ⁻¹ ~ 2ⁿ⁻¹−1 |

### 주요 연산

```vhdl
-- 산술
a + b     -- 같은 타입, 같은 길이
a - b
a * b     -- 결과 폭 = 입력 폭의 합
abs a     -- signed only

-- 비교 (std_logic 반환)
a < b   a <= b   a > b   a >= b   a = b   a /= b

-- 비트 조작
shift_left(a, n)    shift_right(a, n)
rotate_left(a, n)   rotate_right(a, n)

-- 폭 변환
resize(a, new_size)   -- signed: 부호 확장; unsigned: 제로 확장

-- 타입 변환
to_integer(u)              -- unsigned/signed → integer
to_unsigned(n, size)       -- integer → unsigned (size비트)
to_signed(n, size)         -- integer → signed (size비트)
std_logic_vector(u)        -- unsigned → slv (타입 변환)
unsigned(slv)              -- slv → unsigned
signed(slv)                -- slv → signed
```

### 혼합 타입 연산 금지

`unsigned`와 `signed`는 직접 연산 불가. 명시적 변환 필요.

```vhdl
-- 오류
result <= u_val + s_val;

-- 올바른 방법
result <= u_val + unsigned(resize(s_val, u_val'length));
```

---

## 서브타입 (Subtype)

기존 타입에 제약을 추가하거나 이름을 붙인다.

```vhdl
subtype byte_t     is integer range 0 to 255;
subtype nibble_vec is std_logic_vector(3 downto 0);
```

서브타입은 부모 타입과 동일 타입으로 취급 → 타입 변환 없이 대입 가능.

---

## 타입 변환 (Type Conversion)

관련 타입 간 명시적 변환:

```vhdl
integer(r)              -- real → integer (반올림)
real(i)                 -- integer → real
std_logic_vector(u)     -- unsigned → slv
unsigned(slv)           -- slv → unsigned
to_integer(u)           -- unsigned → integer (numeric_std)
to_unsigned(i, n)       -- integer → unsigned, n비트
```

---

## Sources

- IEEE 1076-2008 §5 (Types)
- IEEE Std 1164 (std_logic_1164) — hdlworks.com ✓, hdlfactory.com ✓
- IEEE Std 1076.3 (numeric_std) — hdlfactory.com ✓
