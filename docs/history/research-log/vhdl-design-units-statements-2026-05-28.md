# VHDL (IEEE 1076-2008) Design Units · Concurrent Statements · Sequential Statements Research Log

**조사일**: 2026-05-28
**대상**: IEEE 1076-2008 §3/§10/§11 — 설계 단위, 동시문, 순차문
**목적**: 04-design-units.md / 05-concurrent.md / 06-sequential.md 작성 근거 확보

---

## 조사 방법

- Round 1: WebSearch 2쿼리 (design units 전반, concurrent/sequential 전반)
- Phase 1.5: WebFetch 1차 source 검증 (doulos 403 → vhdlwhiz ✓, vhdl-online ✓)
- Round 2: WebSearch 2쿼리 (configuration 상세, 컴포넌트 인스턴스화 상세 / generate)
- Round 2 verify: WebFetch 4건 (doulos configurations 403 → peterfab config ✓, vhdlwhiz entity/component instantiation ✓, fpgaer if-generate ✓, fpgaer case-generate ✓)
- Round 3: WebSearch 2쿼리 (case? matching case / package 상세)
- Round 3 verify: WebFetch 3건 (doulos small-changes 403 → fpgatutorial generate ✓, hdlworks package ✓, kindatechnical package body ✓)
- Round 4: WebSearch 1쿼리 (block statement), WebFetch 2건 (peterfab configuration ✓, portal.cs.umbc sequential ✓)

---

## A. 설계 단위 (Design Units)

### Entity

```vhdl
entity entity_name is
  generic (
    WIDTH : integer := 8;
    SIGNED_OP : boolean := false
  );
  port (
    clk     : in  std_logic;
    data_in : in  std_logic_vector(WIDTH-1 downto 0);
    result  : out std_logic_vector(WIDTH downto 0)
  );
end entity entity_name;
```

- 하드웨어 모듈의 외부 인터페이스 선언. 동작은 없음.
- `generic`은 파라미터(인스턴스화 시 값 결정).
- `port`는 입출력 핀. 모드: `in`, `out`, `inout`, `buffer`, `linkage`.
- `end entity entity_name;` — `entity` 키워드와 이름은 VHDL-2008에서 선택적.

### Architecture

```vhdl
architecture arch_name of entity_name is
  -- declarative region: signal, constant, component, type...
begin
  -- concurrent statements
end architecture arch_name;
```

- 1개 entity에 여러 architecture 가능.
- 컴파일러는 마지막 컴파일된 아키텍처를 기본 사용.
- `rtl`, `behavioral`, `structural`, `testbench` 등 이름 관례.

출처: IEEE 1076-2008 §3.4 (design entity), Doulos an-example-design-entity ✓ (403)

### Package Declaration + Body

```vhdl
-- 패키지 선언 (공개 인터페이스)
library IEEE;
use IEEE.std_logic_1164.all;

package my_pkg is
  constant DATA_WIDTH : integer := 8;         -- 즉시 값 부여
  constant MAX_COUNT  : integer;              -- 지연 상수 (deferred)
  subtype byte_t is std_logic_vector(7 downto 0);
  type state_t is (IDLE, ACTIVE, DONE);
  component mux2
    port (sel : in std_logic; a, b : in std_logic; y : out std_logic);
  end component;
  function parity(v : byte_t) return std_logic;  -- 선언만
end package my_pkg;

-- 패키지 바디 (구현부)
package body my_pkg is
  constant MAX_COUNT : integer := 255;        -- 지연 상수 값 부여

  function parity(v : byte_t) return std_logic is
    variable p : std_logic := '0';
  begin
    for i in v'range loop
      p := p xor v(i);
    end loop;
    return p;
  end function;
end package body my_pkg;
```

- 패키지 선언부만 외부에서 보인다.
- 바디 내 상수는 외부 불가 (흔한 혼동 주의).
- 바디는 서브프로그램 바디가 없으면 생략 가능.

출처: hdlworks.com/Package.htm ✓, kindatechnical.com/package-body-and-declaration ✓

### Library · Use Clause

```vhdl
library IEEE;
use IEEE.std_logic_1164.all;
use IEEE.numeric_std.all;

library work;
use work.my_pkg.all;
use work.my_pkg.parity;   -- 선택적 임포트
```

- `library` 절은 라이브러리를 스코프에 추가.
- `work`는 현재 프로젝트의 기본 컴파일 목적지 — `library work;` 암시적으로 항상 적용.
- `use ... .all;` 전체 임포트. `use ... .identifier;` 선택적 임포트.
- 라이브러리·use 절은 설계 단위 앞에 위치 (context clause).

### Configuration

```vhdl
-- 기본 구조
configuration cfg_name of entity_name is
  for architecture_name
    for instance_label : component_name
      use entity lib_name.entity_name(arch_name);
      generic map (PARAM => value);
      port map    (comp_port => entity_port);
    end for;
  end for;
end configuration cfg_name;
```

실제 예시:

```vhdl
configuration cfg_test_inv of test_inv is
  for struct_t
    for lh : inv_comp
      use entity work.inverter(struct_i)
        generic map (PropTime => TimeH)
        port map    (IN1 => IN_A, OUT1 => OUT_A);
    end for;
  end for;
end configuration cfg_test_inv;
```

- 컴포넌트 인스턴스를 특정 entity-architecture 쌍에 바인딩.
- **합성 툴 대부분 미지원** — 시뮬레이션 환경에서만 사용.
- 계층 구조에서 중첩 `for` 블록으로 하위 아키텍처까지 바인딩 가능.

출처: peterfab.com/vhd00020.htm ✓

---

## B. 동시문 (Concurrent Statements)

### Process

**감도 목록 방식**:
```vhdl
process(a, b, sel)      -- VHDL-93
begin
  -- sequential statements
end process;

process(all)            -- VHDL-2008: 읽힌 모든 신호 자동 포함
begin
  -- sequential statements
end process;
```

**wait 방식**:
```vhdl
process
begin
  -- sequential statements
  wait on a, b, sel;    -- 명시적 감도
end process;
```

감도 목록 방식 ↔ 프로세스 끝에 `wait on ...;` 동치. [vhdlwhiz.com ✓]

process(all) 주의: 일부 합성 툴에서 미지원. VHDL-2008 컴파일 옵션 필요.

### 동시 신호 할당 — 3가지 형태

**단순 할당(Simple Signal Assignment)**:
```vhdl
y <= a and b;
z <= not (a or b);
```
오른쪽 피연산자 신호에 이벤트 발생 시 재평가.

**조건부 할당(Conditional Signal Assignment, CSA)**:
```vhdl
-- when/else 체인
mux_out <= a when sel = "00" else
           b when sel = "01" else
           c when sel = "10" else
           d;               -- 마지막 무조건 else 필수 (없으면 래치)
```
우선순위: 위에서 아래 순. 중복 조건 허용 (첫 번째 만족 조건 적용).

**선택 할당(Selected Signal Assignment, SSA)**:
```vhdl
-- with/select: case 문과 동일한 제약
with sel select
  mux_out <= a when "00",
             b when "01",
             c when "10",
             d when others;  -- 모든 경우 커버 필수
```
choices 중복 불허, 완전 커버(또는 `when others`) 필수.

### 컴포넌트 인스턴스화

**직접 entity 인스턴스화 (권장, VHDL-93+)**:
```vhdl
-- 명명된 연관 (Named Association) — 권장
u_add : entity work.adder(rtl)
  generic map (
    WIDTH     => 16,
    SIGNED_OP => false
  )
  port map (
    clk => clk,
    a   => operand_a,
    b   => operand_b,
    sum => result
  );

-- 위치 연관 (Positional Association) — 비권장
u_add2 : entity work.adder(rtl)
  generic map (16, false)
  port map (clk, operand_a, operand_b, result);
```

**컴포넌트 인스턴스화 (Verilog 통합 / netlist / configuration 용도)**:
```vhdl
-- 1. 아키텍처 선언 영역에 컴포넌트 선언
architecture rtl of top is
  component adder
    generic (WIDTH : integer := 8);
    port (a, b : in std_logic_vector(WIDTH-1 downto 0);
          sum  : out std_logic_vector(WIDTH downto 0));
  end component adder;
begin
-- 2. 인스턴스화
  u_add : adder
    generic map (WIDTH => 16)
    port map (a => operand_a, b => operand_b, sum => result);
end architecture;
```

- 명명된 연관이 권장 — 포트 순서 변경 시 컴파일 오류로 잡힘.
- `open` 키워드: 연결하지 않는 선택적 포트에 사용.

출처: vhdlwhiz.com/entity-instantiation-and-component-instantiation/ ✓

### Generate 문

**for-generate**:
```vhdl
gen_array : for i in 0 to N-1 generate
  u_ram : entity work.ram_model
    port map (
      clk    => clk,
      enable => enable(i),
      addr   => addr,
      data   => data_array(i)
    );
end generate gen_array;
```

**if-generate (VHDL-2008: elsif/else 추가)**:
```vhdl
-- VHDL-2008
gen_dbg : if DEBUG_BUILD generate
  u_counter : entity work.counter
    port map (clk => clk, q => test_count);
elsif LITE_BUILD generate
  test_count <= (others => '0');
else generate
  test_count <= (others => '1');
end generate gen_dbg;
```
VHDL-93: `elsif/else` 없음, 별도 `if generate` 구문 필요.

**case-generate (VHDL-2008 신규)**:
```vhdl
gen_sel : case IMPLEMENTATION generate
  when 0 =>
    u_impl : entity work.impl_a port map (...);
  when 1 =>
    u_impl : entity work.impl_b port map (...);
  when others =>
    u_impl : entity work.impl_c port map (...);
end generate gen_sel;
```

출처: fpgaer.wordpress.com if-generate ✓, case-generate ✓, fpgatutorial.com/vhdl-generic-generate/ ✓

### Block 문

```vhdl
-- 비가드 블록 (non-guarded): 가독성용 그루핑. 합성 툴 무시.
blk_stage1 : block
  signal local_sig : std_logic;
begin
  local_sig <= a and b;
  y <= local_sig or c;
end block blk_stage1;

-- 가드 블록 (guarded): guard 신호 생성. 합성 불가.
blk_guard : block (enable = '1')
begin
  z <= guarded '1';
end block blk_guard;
```

### 동시 프로시저 호출 · 동시 어시션

```vhdl
-- 동시 프로시저 호출 (내부적으로 프로세스와 동치)
check_parity(data, parity_ok);

-- 동시 어시션
assert not (a = '1' and b = '1')
  report "Invalid state: a and b both high"
  severity WARNING;
```

---

## C. 순차문 (Sequential Statements)

### if / elsif / else

```vhdl
if condition1 then
  statements;
elsif condition2 then
  statements;
else
  statements;
end if;
```

### case / case?

**일반 case**:
```vhdl
case sel is
  when "00"        => q <= a;
  when "01" | "10" => q <= b;    -- 여러 값 OR
  when others      => q <= d;    -- 완전 커버
end case;
```
선택 항목 완전 커버 필수, 중복 불허.

**case? (VHDL-2008 matching case)**:
```vhdl
case? opcode is
  when "1---" => execute_branch;    -- '-' = don't care
  when "01--" => execute_load;
  when "001-" => execute_store;
  when others => execute_nop;
end case?;
```
`?=` 연산자 사용. `'-'`를 진짜 don't care로 처리.
주의: 패턴이 중복되지 않도록 설계자가 보장해야 함.
툴 지원 제한: GHDL issue #1940 (구현 버그 존재). 툴 버전 확인 필수.

### 루프

**for 루프** (정수 범위, 인덱스 변수 읽기 전용):
```vhdl
for i in 0 to 7 loop
  result(i) := data(i) xor mask(i);
end loop;

for i in 7 downto 0 loop   -- 역순
  ...
end loop;
```

**while 루프**:
```vhdl
while count < 10 loop
  count := count + 1;
end loop;
```

**무한 루프**:
```vhdl
infinite_loop : loop
  ...
  exit infinite_loop when done = '1';
end loop infinite_loop;
```

**next / exit**:
```vhdl
next;                              -- 현재 반복 건너뜀
next outer_loop;                   -- 레이블 지정
next when condition;               -- 조건부
exit;                              -- 루프 탈출
exit outer_loop when condition;    -- 레이블 + 조건
```

출처: portal.cs.umbc.edu/help/VHDL/sequential.html ✓

### wait 문

**감도 목록 프로세스에는 사용 불가** (LRM §11).

```vhdl
wait;                         -- 무한 대기 (시뮬레이션 정지 또는 testbench 끝)
wait on sig1, sig2;           -- 이벤트 대기
wait until clk = '1';         -- 조건 대기
wait until rising_edge(clk);  -- 관용
wait for 10 ns;               -- 시간 대기
wait until clk = '1' for 100 ns;  -- 조건 + 타임아웃
```

### 변수 할당 `:=` vs 신호 할당 `<=`

```vhdl
-- 변수: 즉시 반영
process
  variable v : integer := 0;
begin
  v := v + 1;      -- 다음 줄에서 새 값 사용 가능
  out1 <= v;       -- v 현재 값 반영
  wait;
end process;

-- 신호: delta cycle 후 반영
process(all)
begin
  sig <= a + 1;    -- 이 줄 이후에도 sig는 아직 이전 값
  out2 <= sig;     -- 이전 sig 값
end process;
```

**신호 할당 지연 모델**:
```vhdl
y <= a after 5 ns;                         -- inertial (기본): 5ns 미만 펄스 필터
y <= transport a after 5 ns;               -- transport: 모든 전이 전달
y <= reject 2 ns inertial a after 5 ns;    -- 2ns 미만 펄스 필터
clk <= '1', '0' after 5 ns, '1' after 10 ns;  -- 파형 (waveform)
```

### assert / report / severity

```vhdl
-- assert: 조건 불만족 시 메시지
assert data_valid = '1'
  report "Data invalid at time " & time'image(now)
  severity WARNING;

-- assert without report (기본 메시지)
assert a /= b severity ERROR;

-- report: 조건 없는 메시지 출력
report "Simulation start: " & integer'image(count);
report "Fatal error" severity FAILURE;   -- 시뮬레이션 즉시 중단
```

severity 레벨:
| 레벨 | 기본 동작 |
|------|----------|
| `NOTE` | 메시지 출력 후 계속 |
| `WARNING` | 메시지 출력 후 계속 |
| `ERROR` | 메시지 출력 후 계속 (기본값) |
| `FAILURE` | 시뮬레이션 즉시 중단 |

### return

```vhdl
return;           -- procedure에서 조기 반환
return a + b;     -- function에서 값 반환
```

---

## Sources

| URL | 검증 방법 | 신뢰도 |
|-----|----------|--------|
| vhdlwhiz.com/sensitivity-list/ | WebFetch ✓ | 높음 |
| vhdlwhiz.com/entity-instantiation-and-component-instantiation/ | WebFetch ✓ | 높음 |
| hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/Package.htm | WebFetch ✓ | 높음 |
| kindatechnical.com/vhdl-guide/package-body-and-declaration.html | WebFetch ✓ | 중상 |
| peterfab.com/ref/vhdl/vhdl_renerta/mobile/source/vhd00020.htm | WebFetch ✓ | 중상 |
| fpgaer.wordpress.com/vhdl-2008-quick-reference-if-generate-statement/ | WebFetch ✓ | 중상 |
| fpgaer.wordpress.com/vhdl-2008-quick-reference-case-generate-statement/ | WebFetch ✓ | 중상 |
| fpgatutorial.com/vhdl-generic-generate/ | WebFetch ✓ | 중상 |
| portal.cs.umbc.edu/help/VHDL/sequential.html | WebFetch ✓ | 높음 (LRM §11 기반) |
| fpgatutorial.com/vhdl-for-while-loop-if-case-statement/ | WebFetch ✓ | 중상 |
| vhdl-online.de/concurrent_statements | WebFetch ✓ | 중상 |

403 반환 (Doulos): doulos.com/knowhow/vhdl/configurations-part-1/, doulos.com/knowhow/vhdl/an-example-design-entity/,
doulos.com/knowhow/vhdl/vhdl-2008-small-changes/ — 다른 출처로 대체 검증 완료.

---

## 불확실성 / 추가 확인 필요

1. **case? 툴 지원**: GHDL issue #1940 에서 구현 버그 보고. Vivado/Quartus 지원 범위 별도 확인 필요.
2. **configuration 합성 지원**: peterfab 문서 "synthesis tools do generally not support configurations" — 툴별 상이.
3. **process(all) 합성 지원**: vhdlwhiz "most synthesizing tools don't yet support this" — 2026년 현재 Vivado/Quartus 최신 버전 지원 여부 최신 문서 확인 필요.
4. **if-generate label 필수 여부**: VHDL-2008에서 label이 필수로 변경됨 — 툴별 엄격도 다름.
