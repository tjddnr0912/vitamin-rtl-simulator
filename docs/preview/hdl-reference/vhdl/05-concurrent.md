# 05 · VHDL 동시문 (Concurrent Statements)

IEEE 1076-2008 §11 기준. 아키텍처 바디 내 모든 문장은 동시에 활성화된다 — 순서는 의미 없다.

---

## 동시문 종류 요약

| 문장 | 키워드 | 주요 용도 |
|------|--------|----------|
| 프로세스 | `process` | 순차 실행 캡슐화, 레지스터·상태기계 |
| 단순 신호 할당 | `<=` | 조합 논리 기술 |
| 조건부 신호 할당 | `when/else` | 우선순위 MUX |
| 선택 신호 할당 | `with/select` | 병렬 MUX (case 등가) |
| 컴포넌트 인스턴스화 | `entity`/컴포넌트 레이블 | 구조적 계층 |
| generate | `generate` | 반복·조건부 하드웨어 생성 |
| block | `block` | 동시문 그루핑 (계층적 가독성) |
| 동시 프로시저 호출 | 프로시저 이름 | 프로세스 없이 프로시저 호출 |
| 동시 어시션 | `assert` | 설계 제약 검사 (시뮬레이션) |

---

## 프로세스 (process)

동시문 중 유일하게 **순차 실행**을 담는다. 프로세스 내부는 §06-sequential 참고.

### 감도 목록(Sensitivity List) 방식

```vhdl
-- VHDL-93: 명시적 나열
process(clk, rst_n)
begin
  if rst_n = '0' then
    q <= '0';
  elsif rising_edge(clk) then
    q <= d;
  end if;
end process;

-- VHDL-2008: process(all) — 읽힌 모든 신호 자동 포함
process(all)
begin
  case sel is
    when "00" => y <= a;
    when "01" => y <= b;
    when others => y <= c;
  end case;
end process;
```

`process(all)`: 감도 목록 누락 버그 방지. 일부 합성 툴에서 미지원 — 프로젝트 툴체인 확인 필요.

### wait 방식

```vhdl
-- testbench 클록 생성 패턴
process
begin
  clk <= '0';
  wait for CLK_PERIOD / 2;
  clk <= '1';
  wait for CLK_PERIOD / 2;
end process;
```

감도 목록과 `wait`은 **하나의 프로세스에 같이 사용 불가**.

### 감도 목록 ↔ wait 동치

```vhdl
-- 아래 두 프로세스는 동치
process(a, b)
begin
  y <= a and b;
end process;

process
begin
  y <= a and b;
  wait on a, b;      -- 프로세스 끝에 wait on 감도 신호들
end process;
```

### 규칙 요약

| 구분 | 감도 목록 | wait 방식 |
|------|-----------|-----------|
| 합성 | ✅ (RTL 표준) | ⚠️ (제한적) |
| 시뮬레이션 | ✅ | ✅ |
| 용도 | RTL 코드 | testbench, 시뮬레이션 모델 |

---

## 동시 신호 할당 (Concurrent Signal Assignment)

### 단순 할당 (Simple)

```vhdl
y    <= a and b;
z    <= not (a or b) after 2 ns;  -- 지연 포함 가능
flag <= '1';                       -- 상수 드라이브
```

오른쪽 피연산자 신호 중 하나라도 이벤트가 발생하면 재평가한다.

### 조건부 할당 (Conditional Signal Assignment)

```vhdl
-- 4-to-1 MUX
mux_out <= a when sel = "00" else
           b when sel = "01" else
           c when sel = "10" else
           d;                -- 마지막 무조건 else 필수 (없으면 래치 추론)
```

- **우선순위**: 위에서 아래 순 (첫 번째 true 조건 적용).
- 조건 중복 허용 — 상위 조건이 우선.
- 합성에서 마지막 `else` 없으면 래치 생성 경고.

3상태 버스 예시:

```vhdl
bus_out <= data_out when oe = '1' else (others => 'Z');
```

### 선택 할당 (Selected Signal Assignment)

```vhdl
with sel select
  mux_out <= a when "00",
             b when "01",
             c when "10",
             d when others;  -- 완전 커버 필수

-- 여러 값 묶기
with opcode select
  alu_op <= OP_ADD  when X"00" | X"01",
            OP_SUB  when X"02",
            OP_AND  when X"10" to X"13",   -- 범위
            OP_NOP  when others;
```

- **`when others` 필수** — 모든 경우를 커버해야 함.
- choices 중복 불허 (case 문과 동일 제약).
- 우선순위 없음 — 각 choice는 독립(mutually exclusive).

### CSA vs SSA 비교

| | 조건부 (when/else) | 선택 (with/select) |
|--|-------------------|-------------------|
| 우선순위 | 있음 (상위 우선) | 없음 (독립 branches) |
| 조건 형태 | 임의 boolean 조건 | 단일 표현식의 값/범위 |
| 중복 조건 | 허용 | 불허 |
| 합성 결과 | 우선순위 MUX 체인 | 병렬 MUX |

---

## 컴포넌트 인스턴스화

### 직접 entity 인스턴스화 (권장, VHDL-93+)

```vhdl
-- 명명된 연관 (Named Association) — 권장
u_adder : entity work.adder(rtl)
  generic map (
    WIDTH     => 16,
    SIGNED_OP => false
  )
  port map (
    clk => clk,
    a   => op_a,
    b   => op_b,
    sum => result
  );

-- 아키텍처 이름 생략 → 마지막 컴파일된 아키텍처 사용
u_adder2 : entity work.adder
  port map (clk => clk, a => op_a, b => op_b, sum => result);
```

```vhdl
-- 위치 연관 (Positional Association) — 비권장
u_adder3 : entity work.adder(rtl)
  generic map (16, false)
  port map (clk, op_a, op_b, result);
```

위치 연관은 포트 순서 변경에 취약하므로 명명된 연관 사용 권장.

### 컴포넌트 인스턴스화 (Verilog 통합 / netlist / configuration 용도)

```vhdl
architecture rtl of top is
  -- 1단계: 아키텍처 선언 영역에 컴포넌트 선언
  component adder
    generic (
      WIDTH     : integer := 8;
      SIGNED_OP : boolean := false
    );
    port (
      clk : in  std_logic;
      a   : in  std_logic_vector(WIDTH-1 downto 0);
      b   : in  std_logic_vector(WIDTH-1 downto 0);
      sum : out std_logic_vector(WIDTH downto 0)
    );
  end component adder;

begin
  -- 2단계: 인스턴스화
  u_adder : adder
    generic map (WIDTH => 16, SIGNED_OP => false)
    port map (clk => clk, a => op_a, b => op_b, sum => result);
end architecture;
```

### open 키워드

선택적 포트를 연결하지 않을 때 사용:

```vhdl
u_ff : entity work.dff
  port map (
    clk   => clk,
    d     => data_in,
    q     => data_out,
    q_bar => open      -- 사용 안 함
  );
```

### 선택 가이드

```
직접 entity 인스턴스화 사용:
  → 일반 VHDL 모듈 연결 (대부분의 경우)

컴포넌트 인스턴스화 사용:
  → Vivado에서 Verilog 모듈 통합
  → 넷리스트, 하드 매크로
  → configuration으로 구현체를 동적 교체해야 할 때
```

---

## generate 문

반복 또는 조건부로 하드웨어를 생성한다. generate 내부에는 모든 동시문이 올 수 있다(process, 신호 할당, 중첩 generate 포함).

### for-generate

```vhdl
-- N개 RAM 뱅크 생성
gen_ram : for i in 0 to N_BANKS-1 generate
  u_ram : entity work.ram_sp
    port map (
      clk     => clk,
      en      => enable(i),
      we      => we,
      addr    => addr,
      wr_data => wr_data,
      rd_data => rd_data(i)
    );
end generate gen_ram;

-- 내부에 process 포함 가능
gen_regs : for i in 0 to 7 generate
  process(clk)
  begin
    if rising_edge(clk) then
      reg(i) <= data_in(i);
    end if;
  end process;
end generate gen_regs;
```

### if-generate

**VHDL-93**: `elsif/else` 없음.

```vhdl
-- VHDL-93
gen_dbg_93 : if DEBUG generate
  u_probe : entity work.ila port map (...);
end generate gen_dbg_93;
```

**VHDL-2008**: `elsif/else` 추가.

```vhdl
-- VHDL-2008
gen_impl : if IMPLEMENTATION = "FAST" generate
  u_fast : entity work.alu_fast port map (...);
elsif IMPLEMENTATION = "AREA" generate
  u_area : entity work.alu_area port map (...);
else generate
  u_default : entity work.alu_rtl port map (...);
end generate gen_impl;
```

### case-generate (VHDL-2008 신규)

여러 옵션 중 하나를 조건에 따라 선택. if-generate 체인보다 가독성이 좋다.

```vhdl
gen_bus : case BUS_WIDTH generate
  when 8 =>
    u_comp : entity work.comp_8b
      port map (clk => clk, data => data(7 downto 0));
  when 16 =>
    u_comp : entity work.comp_16b
      port map (clk => clk, data => data(15 downto 0));
  when others =>
    u_comp : entity work.comp_32b
      port map (clk => clk, data => data(31 downto 0));
end generate gen_bus;
```

### generate 제약

- generate 내부 변수는 generate 외부에서 사용 불가.
- 레이블(label)은 VHDL-2008에서 필수 — 툴 버전마다 엄격도 다름.
- generate 파라미터(`i`)는 상수 정수이며 루프 변수처럼 수정 불가.

---

## block 문

아키텍처 내 동시문을 **계층적으로 그루핑**한다.

```vhdl
-- 비가드 블록: 가독성용. 합성 툴은 일반적으로 무시.
blk_datapath : block
  signal pipe_reg : std_logic_vector(7 downto 0);
begin
  pipe_reg <= data_in when rising_edge(clk) else pipe_reg;
  data_out <= pipe_reg;
end block blk_datapath;
```

```vhdl
-- 가드 블록: guard 신호 자동 생성. 합성 불가.
blk_tri : block (oe = '1')
begin
  bus_pin <= guarded data_out;   -- oe='1'일 때만 드라이브
end block blk_tri;
```

| 종류 | 합성 | 용도 |
|------|------|------|
| 비가드 블록 | ✅ (툴이 투명하게 처리) | 대형 아키텍처 가독성 구조화 |
| 가드 블록 | ❌ | 시뮬레이션 전용, 트라이스테이트 모델링 |

---

## 동시 프로시저 호출

```vhdl
-- 아키텍처 바디에서 직접 프로시저 호출
-- 내부적으로 다음과 동치: process(all) begin check_parity(data, parity_ok); end process;
check_parity(data, parity_ok);

-- 레이블 붙이기 가능
chk : check_parity(data_in, err_flag);
```

호출 프로시저 내에서 읽은 신호들이 감도 목록이 된다.

---

## 동시 어시션

```vhdl
-- 레이블 없는 형태
assert not (wr_en = '1' and rd_en = '1')
  report "Simultaneous read and write"
  severity ERROR;

-- 레이블 있는 형태
chk_setup : assert setup_time >= T_SETUP
  report "Setup time violation"
  severity WARNING;
```

내부적으로 `process(all) begin assert ...; end process;`와 동치. 합성 툴은 무시하고 시뮬레이션에서만 동작.

---

## 동시문 간 상호작용

동시문들은 **신호(signal)**를 통해 통신한다. 모든 동시문은 동일한 시뮬레이션 delta cycle에서 평가된다.

```vhdl
architecture rtl of example is
  signal a, b, c : std_logic;
begin
  -- 세 문장이 동시에 활성 — 순서 의미 없음
  a <= in1 and in2;               -- 단순 할당
  b <= a or in3;                  -- a의 이전 값 참조 (delta cycle 지연)

  p1 : process(all)               -- a 변경 → 다음 delta에서 실행
  begin
    c <= not a;
  end process;
end architecture;
```

---

## Sources

- IEEE 1076-2008 §11 (Concurrent statements)
- vhdlwhiz.com/sensitivity-list/ ✓
- vhdlwhiz.com/entity-instantiation-and-component-instantiation/ ✓
- vhdl-online.de/concurrent_statements ✓
- fpgaer.wordpress.com VHDL-2008 quick reference if/case generate ✓
- fpgatutorial.com/vhdl-generic-generate/ ✓
- Research log: vhdl-design-units-statements-2026-05-28.md
