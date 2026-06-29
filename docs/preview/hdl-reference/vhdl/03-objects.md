# 03 · VHDL 오브젝트

IEEE 1076-2008 §6 기준. signal / variable / constant / generic / file 및 포트 모드.

---

## 오브젝트 종류 요약

| 오브젝트 | 키워드 | 갱신 타이밍 | 유효 스코프 | 합성 |
|----------|--------|------------|------------|------|
| signal | `signal` | delta cycle 후 | 아키텍처·패키지·포트 | ✅ |
| variable | `variable` | 즉시 | process / subprogram 내부 | ✅ |
| shared variable | `shared variable` | 즉시 | 아키텍처·패키지 | ⚠️ |
| constant | `constant` | 불변 (설계 시간) | 모든 선언 영역 | ✅ |
| generic | `generic` | 불변 (인스턴스화 시) | entity / component | ✅ |
| file | `file` | N/A | process / subprogram | ❌ |

---

## signal

하드웨어 넷(net)을 모델링한다. 값은 드라이버(driver)가 구동하며,
할당 후 실제 값이 반영되는 시점은 **delta cycle**이다.

### 선언

```vhdl
signal clk      : std_logic := '0';
signal data_bus : std_logic_vector(7 downto 0);
signal count    : integer range 0 to 255 := 0;
```

### 할당 — delta cycle 지연

```vhdl
process(clk)
begin
  if rising_edge(clk) then
    s <= '1';          -- 스케줄: 다음 delta에서 반영
    -- 이 줄에서 s를 읽으면 아직 이전 값
    out_val <= s;      -- 이전 s 값이 전달됨
  end if;
end process;
```

프로세스가 `wait` 또는 감도 목록(sensitivity list)에서 정지할 때
누적된 신호 할당들이 한꺼번에 delta cycle 1에서 반영된다.
신호 값의 변화가 또 다른 프로세스를 깨우면 delta cycle 2가 시작되고,
물리 시간이 진전 없이 이 과정이 수렴할 때까지 반복된다.

### delta cycle 직관

```
물리 시간 T=10ns:
  delta 1: process A 실행 → signal a <= '1' 스케줄
  delta 2: a = '1' 반영 → process B(감도: a) 깨어남
  delta 3: process B 실행 → b <= a 스케줄
  delta 4: b = '1' 반영 → 더 이상 이벤트 없음 → T=10ns 완료
```

### 다중 드라이버

`resolved` 타입(`std_logic`)은 다중 드라이버 허용.
`std_ulogic`나 `bit`는 단일 드라이버만 허용 — 위반 시 시뮬레이션 오류.

### 속성 (Attributes)

```vhdl
s'event       -- 이번 delta에서 값이 바뀌었는가 (boolean)
s'active      -- 이번 delta에서 할당이 발생했는가
s'last_value  -- 직전 값
s'last_event  -- 마지막 이벤트로부터 경과 시간
s'stable(t)   -- t 시간 동안 변화 없었는가 (boolean)
s'quiet(t)    -- t 시간 동안 할당이 없었는가

-- 클록 에지 감지 관용 표현
rising_edge(clk)   -- clk'event and clk = '1'
falling_edge(clk)  -- clk'event and clk = '0'
```

---

## variable

프로세스 또는 서브프로그램 내부 지역 저장소.
할당 즉시 값이 반영된다 — 하드웨어 레지스터가 아닌 소프트웨어 변수 의미.

### 선언

```vhdl
process
  variable v     : integer := 0;
  variable temp  : std_logic_vector(7 downto 0);
begin
  v := v + 1;        -- 즉시 반영
  temp := X"FF";
  out_sig <= temp;   -- temp 현재 값이 신호 할당에 반영
  wait for CLK_PERIOD;
end process;
```

### signal vs variable 핵심 차이

```vhdl
-- signal: 이전 값 읽힘
process
  signal s : std_logic := '0';
begin
  s <= '1';
  out1 <= s;   -- '0' (아직 반영 안 됨)
  wait;
end process;

-- variable: 즉시 반영
process
  variable v : std_logic := '0';
begin
  v := '1';
  out2 <= v;   -- '1' (즉시 반영)
  wait;
end process;
```

### 합성에서 variable

```vhdl
-- 조합 논리: 프로세스 내 variable은 와이어로 합성
process(all)
  variable temp : integer;
begin
  temp := a + b;
  result <= temp * c;   -- temp는 와이어
end process;

-- 레지스터로 합성: 클록 에지 프로세스에서 변수를 다음 클록까지 유지
process(clk)
  variable acc : integer := 0;
begin
  if rising_edge(clk) then
    acc := acc + input;   -- acc가 FF로 합성
    output <= acc;
  end if;
end process;
```

### shared variable (공유 변수)

여러 프로세스에서 접근 가능. VHDL-2008에서 `protected type`만 shared variable로 사용하도록 권장.

```vhdl
shared variable counter : integer := 0;
```

경쟁 조건(race condition) 주의 — 동시 접근 시 결과 미정.

---

## constant

설계 시간에 값이 결정되는 불변 오브젝트.

### 선언

```vhdl
constant CLK_PERIOD : time    := 10 ns;
constant DATA_WIDTH : integer := 8;
constant RESET_VAL  : std_logic_vector(7 downto 0) := X"00";
```

### 지연 상수 (Deferred Constant)

패키지 선언부에서 타입만 선언하고 패키지 바디에서 값 지정.

```vhdl
-- package header
package my_pkg is
  constant MAX_COUNT : integer;
end package;

-- package body
package body my_pkg is
  constant MAX_COUNT : integer := 255;
end package body;
```

---

## generic

엔티티 또는 컴포넌트의 파라미터. 인스턴스화 시 값 전달.
`constant` 와 유사하지만 설계 단위 경계에서 값을 주입받는다.

### 선언 및 사용

```vhdl
entity adder is
  generic (
    WIDTH : integer := 8;          -- 기본값 있음
    SIGNED_OP : boolean := false
  );
  port (
    a, b : in  std_logic_vector(WIDTH-1 downto 0);
    sum  : out std_logic_vector(WIDTH downto 0)
  );
end entity;

architecture rtl of adder is
begin
  sum <= std_logic_vector(
    ('0' & unsigned(a)) + ('0' & unsigned(b))
  );
end architecture;
```

### 인스턴스화 시 값 전달

```vhdl
u_add16 : entity work.adder
  generic map (WIDTH => 16, SIGNED_OP => false)
  port map (a => a16, b => b16, sum => s17);
```

generic은 합성 후 파라미터가 인라인(inline)되어 실제 회로로 구체화된다.

### 패키지 generic (VHDL-2008+)

```vhdl
package generic_fifo is
  generic (type ITEM_TYPE; DEPTH : integer := 16);
  -- ...
end package;
```

---

## file 오브젝트

시뮬레이션 전용 파일 I/O. **합성 불가**.

### 선언

```vhdl
file input_file  : text open READ_MODE   is "stimulus.txt";
file output_file : text open WRITE_MODE  is "results.txt";
file log_file    : text open APPEND_MODE is "sim.log";
```

### 주요 프로시저 (textio 패키지)

```vhdl
use std.textio.all;

variable line_buf : line;
variable val      : integer;

readline(input_file, line_buf);    -- 한 줄 읽기
read(line_buf, val);               -- 값 파싱

write(line_buf, val);              -- 값 씌기
writeline(output_file, line_buf);  -- 출력
```

---

## 포트 모드 (Port Modes)

엔티티 포트 선언의 인터페이스 방향.

```vhdl
entity entity_name is
  port (
    clk      : in  std_logic;
    data_out : out std_logic_vector(7 downto 0);
    data_bus : inout std_logic_vector(7 downto 0);
    feedback : buffer std_logic
  );
end entity;
```

### 모드별 허용 동작

| 모드 | 아키텍처 내부에서 읽기 | 아키텍처 내부에서 쓰기 | 다중 드라이버 | 사용 시나리오 |
|------|----------------------|----------------------|--------------|--------------|
| `in` | ✅ | ❌ | N/A | 입력: 클록, 리셋, 데이터 수신 |
| `out` | ❌ (93) / ✅ (2008+) | ✅ | ❌ | 출력: 결과, 상태 |
| `inout` | ✅ | ✅ | ✅ | 양방향 버스, I/O 핀 |
| `buffer` | ✅ | ✅ | ❌ (단일) | 출력 피드백이 필요한 경우 |
| `linkage` | 제한 | 제한 | — | 거의 미사용 (연결 선언 전용) |

**VHDL-2008 `out` 개선**: 이전 표준에서는 `out` 포트를 아키텍처 내부에서 읽을 수 없어
`buffer`를 사용해야 했다. VHDL-2008부터 `out` 포트도 내부에서 읽을 수 있어
`buffer` 사용 필요성이 크게 줄었다.

```vhdl
-- VHDL-93: out 포트 피드백이 필요하면 buffer 사용
port (count : buffer integer range 0 to 255);

-- VHDL-2008: out으로 직접 읽기 가능
port (count : out integer range 0 to 255);
architecture rtl of ...
begin
  process(clk) begin
    if rising_edge(clk) then
      count <= count + 1;  -- 2008에서 out 포트 읽기 가능
    end if;
  end process;
end architecture;
```

**`inout` vs `buffer`**:
- `inout`: 다중 드라이버 허용. 실제 양방향 핀(예: I²C SDA)이나 버스에 사용.
- `buffer`: 단일 드라이버. 피드백만 필요하고 외부 다중 드라이버가 없을 때.

**`linkage`**: 포트 값이 외부의 `linkage` 모드 포트에만 연결될 수 있다.
표준에는 존재하지만 실용적 사용 예가 거의 없다.

---

## 오브젝트 선언 위치 요약

```vhdl
architecture rtl of my_entity is
  signal   s1 : std_logic;       -- 아키텍처 선언 영역
  constant C1 : integer := 8;    -- 아키텍처 선언 영역
  shared variable sv : integer;  -- 아키텍처 선언 영역

begin
  process
    variable v1 : integer := 0;  -- 프로세스 선언 영역
    file f1 : text open READ_MODE is "x.txt";
  begin
    -- ...
  end process;
end architecture;
```

generic은 entity 선언부에서만 선언한다.

---

## Sources

- IEEE 1076-2008 §6 (Objects, classes, and associated operations)
- IEEE 1076-2008 §9 (Concurrent statements) — process semantics
- vhdlwhiz.com/delta-cycles-explained/ ✓
- emlogic.no/2024/01/using-variables-as-registers-in-vhdl/ ✓
- piembsystech.com/ports-and-port-modes-in-vhdl/ ✓
- hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/Port.htm ✓
