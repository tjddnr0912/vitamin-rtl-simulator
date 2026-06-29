# 04 · VHDL 설계 단위 (Design Units)

IEEE 1076-2008 §3 기준. entity / architecture / package / library·use / configuration.

---

## 설계 단위 종류 요약

| 단위 | 역할 | 합성 |
|------|------|------|
| entity | 외부 인터페이스 선언 (generics + ports) | ✅ |
| architecture | 내부 구현 (entity에 종속) | ✅ |
| package declaration | 타입·상수·컴포넌트·함수 선언 공유 | ✅ |
| package body | 함수·프로시저 구현, 지연 상수 값 | ✅ (함수 내용) |
| configuration | 컴포넌트를 entity-arch 쌍에 바인딩 | ❌ (대부분 툴 미지원) |

컨텍스트 절(`library` + `use`)은 독립 설계 단위가 아니라 설계 단위 앞에 붙는 헤더다.

---

## entity

하드웨어 모듈의 **외부 인터페이스**를 선언한다. 동작은 담지 않는다.

### 문법

```vhdl
entity entity_name is
  generic (
    generic_name : type [:= default_value];
    ...
  );
  port (
    port_name : mode type;
    ...
  );
end entity entity_name;
```

- `generic` 절과 `port` 절 모두 선택적.
- `end entity entity_name;` — `entity` 키워드와 이름은 생략 가능 (하지만 명시 권장).

### generic

인스턴스화 시 외부에서 주입되는 파라미터. `constant`와 유사하나 설계 단위 경계에서 값을 받는다.

```vhdl
entity adder is
  generic (
    WIDTH     : integer := 8;        -- 기본값 있음
    SIGNED_OP : boolean := false
  );
  port (
    a, b : in  std_logic_vector(WIDTH-1 downto 0);
    sum  : out std_logic_vector(WIDTH downto 0)
  );
end entity adder;
```

### port 모드

| 모드 | 내부 읽기 | 내부 쓰기 | 다중 드라이버 |
|------|-----------|-----------|--------------|
| `in` | ✅ | ❌ | N/A |
| `out` | ❌ (93) / ✅ (2008+) | ✅ | ❌ |
| `inout` | ✅ | ✅ | ✅ |
| `buffer` | ✅ | ✅ | ❌ |
| `linkage` | 제한 | 제한 | — |

VHDL-2008에서 `out` 포트를 아키텍처 내부에서 읽을 수 있게 되어 `buffer`의 필요성이 크게 줄었다.

---

## architecture

entity의 **내부 구현**을 담는다. 하나의 entity에 여러 architecture를 붙일 수 있다.

### 문법

```vhdl
architecture arch_name of entity_name is
  -- 선언 영역: signal, constant, component, type, subtype, function, procedure, ...
begin
  -- 동시문(concurrent statements) 영역
end architecture arch_name;
```

### 1개 entity — 여러 architecture

```vhdl
-- RTL 구현
architecture rtl of adder is
begin
  sum <= std_logic_vector(
    ('0' & unsigned(a)) + ('0' & unsigned(b))
  );
end architecture rtl;

-- 동작 모델 (시뮬레이션용)
architecture behavioral of adder is
begin
  process(a, b)
    variable s : integer;
  begin
    s   := to_integer(unsigned(a)) + to_integer(unsigned(b));
    sum <= std_logic_vector(to_unsigned(s, sum'length));
  end process;
end architecture behavioral;
```

툴은 기본적으로 **마지막 컴파일된** 아키텍처를 선택한다. 명시적 선택이 필요하면 configuration을 사용한다.

### 선언 영역

```vhdl
architecture rtl of top is
  signal   s1     : std_logic;
  signal   bus8   : std_logic_vector(7 downto 0) := X"00";
  constant C_SIZE : integer := 16;
  component mux2
    port (sel : in std_logic; a, b : in std_logic; y : out std_logic);
  end component;
begin
  ...
end architecture;
```

---

## package

타입·상수·컴포넌트·서브프로그램 선언을 여러 설계 단위에 **공유**한다.

### package declaration (선언부)

```vhdl
library IEEE;
use IEEE.std_logic_1164.all;

package my_pkg is
  -- 상수 (즉시 값)
  constant DATA_WIDTH : integer := 8;

  -- 지연 상수 (deferred constant) — 값은 바디에서
  constant MAX_COUNT  : integer;

  -- 타입·서브타입
  subtype byte_t  is std_logic_vector(7 downto 0);
  type state_t    is (IDLE, ACTIVE, DONE);

  -- 컴포넌트 선언
  component fifo
    generic (DEPTH : integer := 16);
    port (clk, rst, wr_en, rd_en : in  std_logic;
          wr_data                 : in  byte_t;
          rd_data                 : out byte_t;
          full, empty             : out std_logic);
  end component;

  -- 함수·프로시저 선언 (바디 없음)
  function parity(v : byte_t) return std_logic;
  procedure swap(a, b : inout integer);
end package my_pkg;
```

- **선언부만 외부에서 보인다.** 바디 내용은 외부 불가.
- 서브프로그램 바디가 없으면 패키지 바디 자체를 생략할 수 있다.

### package body (바디)

```vhdl
package body my_pkg is
  -- 지연 상수 값 부여
  constant MAX_COUNT : integer := 255;

  -- 함수 구현
  function parity(v : byte_t) return std_logic is
    variable p : std_logic := '0';
  begin
    for i in v'range loop
      p := p xor v(i);
    end loop;
    return p;
  end function parity;

  -- 프로시저 구현
  procedure swap(a, b : inout integer) is
    variable tmp : integer;
  begin
    tmp := a;  a := b;  b := tmp;
  end procedure swap;
end package body my_pkg;
```

주의: 바디 내에서 새로 선언한 상수·타입은 외부에서 보이지 않는다 — 흔한 혼동 지점.

---

## library · use 절

### 문법

```vhdl
library library_name;
use library_name.package_name.item_or_all;
```

### 관용 패턴

```vhdl
-- IEEE 표준 패키지
library IEEE;
use IEEE.std_logic_1164.all;   -- std_logic, std_logic_vector
use IEEE.numeric_std.all;      -- unsigned, signed

-- 현재 프로젝트 패키지
library work;                  -- 암시적으로 항상 적용 (선언 생략 가능)
use work.my_pkg.all;           -- 패키지 전체
use work.my_pkg.parity;        -- 선택적 임포트
```

### 동작 규칙

- `library` 절은 라이브러리를 현재 컨텍스트에 추가.
- `work`는 현재 프로젝트의 기본 컴파일 목적지 — `library work;` 생략해도 항상 유효.
- 컨텍스트 절은 설계 단위 **앞**에 위치하며 그 설계 단위에만 적용된다.
- 패키지를 변경하면 그것을 `use`한 모든 설계 단위를 재컴파일해야 한다.

### 표준 라이브러리

| 라이브러리 | 패키지 | 주요 내용 |
|-----------|--------|----------|
| `IEEE` | `std_logic_1164` | `std_logic`, `std_logic_vector`, 변환 함수 |
| `IEEE` | `numeric_std` | `unsigned`, `signed`, 산술 연산 |
| `IEEE` | `math_real` | `sqrt`, `log`, 삼각함수 (시뮬레이션 전용) |
| `STD` | `standard` | `integer`, `boolean`, `bit` 등 기본 타입 (항상 내포) |
| `STD` | `textio` | 파일 I/O (시뮬레이션 전용) |

---

## configuration

아키텍처 내 컴포넌트 인스턴스를 **특정 entity-architecture 쌍에 바인딩**한다.

### configuration declaration

```vhdl
configuration cfg_name of entity_name is
  for architecture_name
    -- 컴포넌트 인스턴스 바인딩
    for instance_label : component_name
      use entity lib_name.entity_name(arch_name);
      generic map (generic_name => value);
      port map    (comp_port    => entity_port);
    end for;
  end for;
end configuration cfg_name;
```

### 실제 예시: 두 가지 구현체 교체

```vhdl
-- 빠른 구현 선택
configuration cfg_fast of top is
  for rtl
    for u_alu : alu_comp
      use entity work.alu(fast_rtl);
    end for;
  end for;
end configuration cfg_fast;

-- 검증용 행동 모델 선택
configuration cfg_behav of top is
  for rtl
    for u_alu : alu_comp
      use entity work.alu(behavioral);
    end for;
  end for;
end configuration cfg_behav;
```

### 계층 구조 바인딩

```vhdl
configuration cfg_full of system is
  for struct
    for u_cpu : cpu_comp
      use entity work.cpu(rtl);
      for rtl
        for u_alu : alu_comp
          use entity work.alu(fast_rtl);
        end for;
      end for;
    end for;
  end for;
end configuration cfg_full;
```

### configuration specification (아키텍처 내 인라인 바인딩)

```vhdl
architecture rtl of top is
  component alu_comp
    port (a, b : in std_logic_vector(7 downto 0); result : out std_logic_vector(7 downto 0));
  end component;

  -- 선언 영역에서 바인딩 (configuration specification)
  for u_alu : alu_comp
    use entity work.alu(rtl);
  end for;
begin
  u_alu : alu_comp port map (a => op_a, b => op_b, result => res);
end architecture;
```

### 합성 제한

대부분의 합성 툴(Vivado, Quartus 등)은 configuration을 지원하지 않는다. 실무에서는:
- 시뮬레이션/검증 환경에서 구현체 교체에 사용
- RTL 합성에서는 직접 entity 인스턴스화 + architecture 이름 명시가 대안

---

## 설계 단위 파일 구조 관례

```vhdl
-- my_module.vhd

-- 1. 컨텍스트 절 (모든 설계 단위 앞에)
library IEEE;
use IEEE.std_logic_1164.all;
use IEEE.numeric_std.all;

-- 2. entity
entity my_module is
  generic (WIDTH : integer := 8);
  port (
    clk   : in  std_logic;
    rst_n : in  std_logic;
    data  : in  std_logic_vector(WIDTH-1 downto 0);
    valid : out std_logic
  );
end entity my_module;

-- 3. architecture (같은 파일 또는 별도 파일)
architecture rtl of my_module is
  signal count : unsigned(3 downto 0) := (others => '0');
begin
  process(clk, rst_n)
  begin
    if rst_n = '0' then
      count <= (others => '0');
      valid <= '0';
    elsif rising_edge(clk) then
      count <= count + 1;
      valid <= '1' when count = X"F" else '0';
    end if;
  end process;
end architecture rtl;
```

---

## Sources

- IEEE 1076-2008 §3 (Design entities and configurations)
- IEEE 1076-2008 §4 (Subprograms and packages)
- vhdlwhiz.com/entity-instantiation-and-component-instantiation/ ✓
- hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/Package.htm ✓
- kindatechnical.com/vhdl-guide/package-body-and-declaration.html ✓
- peterfab.com/ref/vhdl/vhdl_renerta/mobile/source/vhd00020.htm ✓ (configuration)
- Research log: vhdl-design-units-statements-2026-05-28.md
