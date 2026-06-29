# 09 · VHDL 합성 가능성 (Synthesizability)

IEEE 1076-2008 기준. 합성 도구(Vivado, Quartus 등)가 VHDL 소스를 실제 게이트 넷리스트로 변환할 수 있는지 여부를 정리한다.

**범례:**
- ✅ 합성 가능 (Universal) — 주요 툴 모두 지원
- ⚠️ 조건부 — 툴/버전/사용 방식에 따라 지원 여부 다름
- ❌ 비합성 — RTL 설계에서 사용 불가 (시뮬레이션·testbench 전용)

---

## ✅ 합성 가능 (Universal)

### 신호 타입

```vhdl
signal a : std_logic;                        -- 1비트 로직
signal b : std_logic_vector(7 downto 0);     -- 벡터
signal c : std_ulogic;                       -- 비결선형 (합성 동일)
signal d : unsigned(7 downto 0);            -- numeric_std 타입
signal e : signed(7 downto 0);
```

```vhdl
-- integer: 반드시 range 제약 명시 (무한 범위는 비합성)
signal cnt : integer range 0 to 255;         -- ✅
signal bad : integer;                        -- ⚠️ 도구에 따라 경고/오류
```

### 열거형 / boolean

```vhdl
type state_t is (IDLE, FETCH, DECODE, EXECUTE, WRITEBACK);
signal state : state_t;         -- 자동으로 바이너리 또는 one-hot 인코딩

signal flag : boolean;          -- false/true → 0/1
```

### Process (완전 감도 목록)

```vhdl
-- 동기 (클록 + 비동기 리셋)
process(clk, rst_n)
begin
    if rst_n = '0' then
        q <= (others => '0');
    elsif rising_edge(clk) then
        q <= d;
    end if;
end process;

-- 조합 (VHDL-2008: process(all))
process(all)                     -- all = 모든 읽기 신호 자동 포함
begin
    y <= a and b;
end process;
```

### if / case

```vhdl
-- if: else 없으면 래치 추론 → 조합 process에서는 else 필수
if sel = '1' then
    y <= a;
else
    y <= b;                      -- else 있어야 래치 회피
end if;

-- case: when others 포함 권장 (std_logic_vector는 'U','X' 등 추가 값)
case opcode is
    when "00" => exec_add;
    when "01" => exec_sub;
    when others => exec_nop;     -- 나머지 모두 커버
end case;
```

### for-loop (정적 범위)

```vhdl
-- 합성 시 자동 unroll → N개 병렬 하드웨어
for i in 0 to 7 loop
    result(i) <= a(i) xor b(i);
end loop;

-- generic으로 크기 파라미터화
for i in 0 to WIDTH-1 loop      -- WIDTH가 elaboration 시 상수이면 OK
    sum := sum + to_integer(unsigned'(0 => vec(i)));
end loop;
```

### generate

```vhdl
-- for-generate: N개 인스턴스 자동 생성
gen_ff : for i in 0 to N-1 generate
    dff_i : dff port map(clk => clk, d => d(i), q => q(i));
end generate;

-- if-generate: 파라미터 조건부 구조
gen_rst : if HAS_RESET generate
    reset_logic : process(clk, rst_n) ...
end generate;

-- case-generate (VHDL-2008): ⚠️ 부분 지원 (아래 조건부 섹션 참고)
gen_sel : case ARCH_TYPE generate
    when "fast" => fast_inst : fast_module port map(...);
    when others => slow_inst : slow_module port map(...);
end generate;
```

### Function / Procedure (제약 만족 시)

```vhdl
-- pure function → 조합 논리로 매핑
function parity(v : std_logic_vector) return std_logic is
    variable p : std_logic := '0';
begin
    for i in v'range loop p := p xor v(i); end loop;
    return p;
end function;

-- wait·file·외부 상태 없는 procedure
procedure gray_encode(
    bin  : in  std_logic_vector;
    gray : out std_logic_vector
) is begin
    gray := bin xor ('0' & bin(bin'high downto 1));
end procedure;
```

### 패키지 연산자

```vhdl
-- ieee.std_logic_1164: and/or/xor 등 std_logic 연산
y <= a and b;

-- ieee.numeric_std: signed/unsigned 산술
sum <= a + b;
diff <= a - b;
```

---

## ⚠️ 조건부 합성 (Tool-dependent)

### record 타입

```vhdl
type pixel_t is record
    r, g, b : unsigned(7 downto 0);
end record;

signal px : pixel_t;          -- 신호·변수: 대부분 도구 OK
```

| 사용 위치 | 지원 여부 |
|----------|----------|
| signal, variable | ✅ 대부분 도구 OK |
| entity port | ⚠️ Vivado 2019+: OK; 일부 도구: 미지원 |
| generic | ⚠️ VHDL-2008+, 부분 지원 |

Vivado에서 record port를 사용할 경우 `-2008` 컴파일 옵션 및 프로젝트 언어 버전 설정 필요.

### variable (공유 변수)

```vhdl
-- process-local variable: ✅ 합성 가능
process(clk)
    variable cnt : integer range 0 to 15 := 0;
begin
    if rising_edge(clk) then
        cnt := cnt + 1;
    end if;
end process;

-- shared variable: ⚠️ 합성 도구마다 다름
shared variable global_cnt : integer := 0;   -- 주의: 대부분 합성 경고
```

### protected 타입 (VHDL-2008+)

```vhdl
type counter_t is protected
    procedure increment;
    impure function get return integer;
end protected;
```

시뮬레이션에서 thread-safe 카운터로 유용. 합성: **매우 제한적** — 주요 툴 미지원.

### generic 타입 (VHDL-2008+)

```vhdl
-- 타입 파라미터 entity
entity sorter is
    generic (type T; N : natural; function "<"(a, b : T) return boolean is <>);
    port (data : in my_array_t; sorted : out my_array_t);
end entity;
```

**부분 지원** — Vivado에서 제한적으로 지원. Quartus 지원 수준 확인 필요.

### 재귀 (정적 깊이)

```vhdl
-- elaboration 시 완전히 unroll 가능한 재귀: ✅ (도구 지원 확인)
function clog2(n : positive) return natural is
begin
    if n <= 1 then return 0;
    else return 1 + clog2((n + 1) / 2);
    end if;
end function;

constant ADDR_W : natural := clog2(MEM_SIZE);  -- 상수 계산에만 사용
```

Vivado: 정적 깊이 재귀 OK. 런타임 깊이 재귀: ❌.

### case-generate (VHDL-2008)

```vhdl
gen_arch : case IMPLEMENTATION generate
    when 0 => ...    -- LUT 기반
    when 1 => ...    -- DSP 기반
    when others => ...
end generate;
```

**부분 지원** — Vivado 2019+: OK. 구버전 및 일부 툴: 미지원.

### fixed_pkg / float_pkg

- **합성 가능하나 면적 비용 큼.**
- `fixed_pkg`: Vivado OK. 구형 도구는 manual expansion 필요.
- `float_pkg`: Vivado 합성 지원. 단, 단정밀도 FP 연산 하나가 수백 LUT.
  → 복잡한 FP 연산은 Vivado Floating Point IP 코어 사용 권장.

---

## ❌ 비합성 (Simulation/Testbench only)

### file 타입 / textio

```vhdl
-- 하드웨어에 파일 시스템 없음 → 비합성
file stim_file : text open READ_MODE is "input.txt";
use std.textio.all;
```

### access 타입 (포인터 / 동적 메모리)

```vhdl
-- access = VHDL 포인터. 동적 할당 → 하드웨어 매핑 불가
type int_ptr is access integer;
variable ptr : int_ptr;
ptr := new integer'(42);            -- ❌ new 키워드 자체 비합성
deallocate(ptr);
```

### wait for (시간 지정)

```vhdl
wait for 10 ns;                     -- ❌ 물리 시간 = 시뮬레이터 개념
wait for CLK_PERIOD / 2;            -- ❌ testbench 전용

-- Quartus: 단일 wait until은 허용; 복수 또는 wait for는 거부
wait until rising_edge(clk);        -- ✅ (단일 wait until, 일부 툴)
```

### after 지연 (신호 할당)

```vhdl
-- after 키워드는 합성 시 무시되거나 오류 처리
y <= a after 5 ns;                  -- ❌ 합성 무시 (파형 모델링 전용)
clk <= not clk after 5 ns;         -- ❌ testbench 클록 생성 전용
```

### real 타입 산술 / math_real

```vhdl
signal r : real;                    -- ❌ 합성 도구 거부
r := 3.14 * 2.0;

use ieee.math_real.all;
x := sqrt(2.0);                     -- ❌ 합성 불가
```

> 예외: `real` 값이 **elaboration time 상수** 계산에만 쓰이는 경우 일부 도구 허용. 신호·변수로 선언하면 거부.

### 무한 루프 (정적 exit 없음)

```vhdl
-- testbench 클록 생성 패턴 (RTL 금지)
loop
    clk <= '0'; wait for 5 ns;
    clk <= '1'; wait for 5 ns;
end loop;

-- while true with no static exit: ❌
while true loop
    ...         -- 합성 도구가 종료 조건을 정적으로 판단 불가
end loop;
```

### 전역 상태 의존 impure function

```vhdl
shared variable global_state : integer := 0;

impure function get_state return integer is
begin
    return global_state;           -- 글로벌 mutable 상태 참조
end function;
```

합성 시 side-effect를 하드웨어로 표현할 수 없음.

---

## 합성 설계 체크리스트

```
신호 타입
  [ ] std_logic / std_logic_vector 사용
  [ ] integer에 range 제약 명시
  [ ] record port 사용 시 툴 VHDL-2008 모드 확인

Process
  [ ] 동기 process: clk + rst_n 만 감도 목록
  [ ] 조합 process: process(all) 또는 완전 감도 목록
  [ ] wait 문 포함 여부 확인 (합성 process에 wait 금지)

분기/루프
  [ ] if에 else 포함 (래치 방지)
  [ ] case에 when others 포함
  [ ] for-loop 범위가 elaboration 상수
  [ ] while/무한 루프 RTL에서 제거

패키지
  [ ] ieee.numeric_std 사용 (std_logic_arith 금지)
  [ ] math_real, textio를 synthesizable 파일에서 제거
  [ ] fixed_pkg/float_pkg 사용 시 면적 예산 확인

Subprogram
  [ ] 합성 대상 function에 wait / file I/O 없음
  [ ] 재귀 함수의 깊이가 elaboration 상수
  [ ] 합성 파일에 testbench 전용 procedure 혼입 금지
```

---

## 비합성 패턴 → 합성 대체 패턴

| 비합성 패턴 | 합성 대체 |
|------------|----------|
| `wait for N ns` | 카운터 + 클록 에지 기반 타이밍 |
| `after N ns` | 레지스터 파이프라인 딜레이 |
| `real` 변수 | `integer`(고정소수점) 또는 `sfixed`/`ufixed` |
| `access` 타입 | 정적 배열 + 포인터 인덱스 integer |
| `math_real.sqrt(x)` | 뉴턴-랩슨 반복 알고리즘, CORDIC, 또는 LUT |
| `file` stimulus | ROM (초기화된 배열 constant) |
| `shared variable` | FSM 또는 독립 신호로 리팩터링 |

---

## Sources

- AMD Vivado UG901 — VHDL Constructs Support Status: https://docs.amd.com/r/en-US/ug901-vivado-synthesis/VHDL-Constructs-Support-Status
- AMD Vivado UG901 — Supported/Unsupported VHDL Data Types: https://docs.amd.com/r/en-US/ug901-vivado-synthesis/Supported-and-Unsupported-VHDL-Data-Types
- Intel/Altera — Quartus wait constructs: https://www.intel.com/content/www/us/en/support/programmable/articles/000076012.html
- EDAboard — real data type synthesis: https://www.edaboard.com/threads/vhdl-real-data-type-error-10414.353096/
- HDL Factory — VHDL IEEE Libraries (2025): https://www.hdlfactory.com/post/2025/06/29/vhdl-ieee-libraries-and-numeric-type-conversion-a-definitive-reference/ ✓
- Research log: [vhdl-subprograms-pkg-synth-2026-05-28.md](../../research-log/vhdl-subprograms-pkg-synth-2026-05-28.md)
