# 06 · VHDL 순차문 (Sequential Statements)

IEEE 1076-2008 §10 기준. `process`, `function`, `procedure` 내부에서만 사용. 위에서 아래 순서대로 실행된다.

---

## 순차문 종류 요약

| 문장 | 키워드 | 주요 용도 |
|------|--------|----------|
| if | `if/elsif/else` | 조건 분기 |
| case | `case/when` | 다중 선택 (완전 커버) |
| matching case | `case?/when` | don't care 지원 (VHDL-2008) |
| for 루프 | `for ... in ... loop` | 고정 반복 |
| while 루프 | `while ... loop` | 조건 반복 |
| 무한 루프 | `loop` | exit으로 탈출 |
| next | `next` | 루프 현재 반복 건너뜀 |
| exit | `exit` | 루프 탈출 |
| wait | `wait` | 이벤트/시간/조건 대기 |
| 변수 할당 | `:=` | 즉시 반영 |
| 신호 할당 | `<=` | delta cycle 후 반영 |
| assert | `assert` | 조건 검사 + 메시지 |
| report | `report` | 메시지 출력 |
| return | `return` | 함수·프로시저 반환 |

---

## if / elsif / else

```vhdl
if condition1 then
  statements;
elsif condition2 then
  statements;
elsif condition3 then
  statements;
else
  statements;
end if;
```

`elsif` 절과 `else` 절은 모두 선택적. `elsif`는 개수 제한 없음.

### 동기 레지스터 패턴

```vhdl
process(clk, rst_n)
begin
  if rst_n = '0' then          -- 비동기 리셋
    q <= (others => '0');
  elsif rising_edge(clk) then  -- 동기 로직
    if en = '1' then
      q <= d;
    end if;
  end if;
end process;
```

### 조합 논리 패턴

```vhdl
process(all)
begin
  -- 모든 분기 커버 필수 (else 없으면 래치 추론)
  if sel = '0' then
    y <= a;
  else
    y <= b;
  end if;
end process;
```

---

## case

```vhdl
case expression is
  when value1 =>
    statements;
  when value2 | value3 =>    -- OR: 여러 값을 하나의 분기로
    statements;
  when value4 to value7 =>   -- 범위 (integer/열거형)
    statements;
  when others =>             -- 나머지 전체 (선택적이지만 권장)
    null;
end case;
```

- 선택 항목은 **완전 커버** 필수 (`when others`로 마무리하거나 명시적으로 모두 나열).
- choices **중복 불허** — 하나의 값이 두 branch에 동시에 속하면 컴파일 오류.
- `null`은 아무 동작도 하지 않는 명시적 빈 분기.

### 4-to-1 MUX 예시

```vhdl
process(all)
begin
  case sel is
    when "00" => y <= a;
    when "01" => y <= b;
    when "10" => y <= c;
    when "11" => y <= d;
  end case;
end process;

-- std_logic_vector는 'U','X' 등 추가 값이 있으므로 when others 권장
process(all)
begin
  case sel is
    when "00"   => y <= a;
    when "01"   => y <= b;
    when "10"   => y <= c;
    when others => y <= d;
  end case;
end process;
```

### 상태기계 패턴

```vhdl
type state_t is (IDLE, RUN, DONE);
signal state : state_t;

process(clk)
begin
  if rising_edge(clk) then
    case state is
      when IDLE =>
        if start = '1' then state <= RUN; end if;
      when RUN  =>
        if done_flag = '1' then state <= DONE;
        else state <= RUN; end if;
      when DONE =>
        state <= IDLE;
    end case;
  end if;
end process;
```

---

## case? (Matching Case) — VHDL-2008

일반 `case`의 확장. `?=` 매칭 연산자를 이용해 `'-'`(don't care)와 `std_logic` 약값(`'H'`='1', `'L'`='0')을 처리한다.

```vhdl
case? opcode is
  when "1---" => execute_branch;    -- '-' = don't care: 1xxx 모두 일치
  when "01--" => execute_load;
  when "001-" => execute_store;
  when "0001" => execute_halt;
  when others => execute_nop;
end case?;
```

### case vs case? 비교

| 항목 | `case` | `case?` |
|------|--------|---------|
| `'-'` 처리 | 값 `'-'` 그대로 비교 | don't care (모든 값 일치) |
| `'H'`/`'L'` | 리터럴 비교 | `'1'`/`'0'`로 매칭 |
| 연산자 | `=` | `?=` |
| 키워드 | `case` / `end case;` | `case?` / `end case?;` |
| 도입 버전 | VHDL-87 이후 | VHDL-2008 |

### 패턴 중복 주의

```vhdl
-- 잘못된 예: "1-" 와 "11" 중복
case? sel is
  when "1-" => ...   -- "11"도 일치
  when "11" => ...   -- 중복 — 컴파일러/툴 오류
  when others => ...
end case?;
```

don't care를 포함하는 패턴 설계자가 중복이 없도록 보장해야 한다.

### 툴 지원

- GHDL: 구현 이슈 보고 (issue #1940). 버전 확인 필요.
- Vivado / Quartus: 컴파일 시 `-2008` 옵션 또는 VHDL-2008 프로젝트 설정 필요.
- ModelSim: `vcom -2008` 옵션 필요.

---

## 루프 (Loop)

### for 루프

```vhdl
for i in 0 to 7 loop          -- 오름차순 (0, 1, ..., 7)
  result(i) := data(i) xor mask(i);
end loop;

for i in 7 downto 0 loop      -- 내림차순
  sum := sum + to_integer(unsigned'(0 => vec(i)));
end loop;

-- 레이블 붙이기
outer : for i in 0 to N-1 loop
  inner : for j in 0 to M-1 loop
    matrix(i, j) := i * M + j;
  end loop inner;
end loop outer;
```

- 인덱스 변수(`i`)는 루프 내에서 자동 선언 — 별도 변수 선언 불필요.
- 인덱스 변수는 **읽기 전용** — 루프 내에서 수정 불가.
- 범위 표현식은 상수여야 한다 (합성 시).

### while 루프

```vhdl
while count < LIMIT loop
  count := count + 1;
  data(count) := some_val;
end loop;

-- 조건이 처음부터 false면 루프 본체 실행 안 됨
while false loop   -- 실행 안 됨
  ...
end loop;
```

### 무한 루프

```vhdl
-- exit으로 탈출하는 무한 루프
clock_gen : loop
  clk <= '0';  wait for CLK_PERIOD/2;
  clk <= '1';  wait for CLK_PERIOD/2;
end loop clock_gen;

-- 조건부 탈출
scan : loop
  read_byte(byte_val);
  exit scan when byte_val = STOP_BYTE;
end loop scan;
```

---

## next / exit

### next — 현재 반복 건너뜀

```vhdl
for i in 0 to 15 loop
  next when data(i) = '0';    -- '0'이면 이 반복 건너뜀
  process_bit(i);
end loop;

-- 중첩 루프에서 외부 루프 레이블 지정
outer : for i in 0 to N-1 loop
  for j in 0 to M-1 loop
    next outer when skip_row(i) = '1';   -- outer 루프 다음 반복으로
    matrix(i, j) := compute(i, j);
  end loop;
end loop outer;
```

### exit — 루프 탈출

```vhdl
for i in 0 to 255 loop
  exit when found = '1';     -- 조건 만족 시 루프 종료
  search(i, found);
end loop;

-- 중첩 루프 탈출
outer : for i in 0 to N-1 loop
  for j in 0 to M-1 loop
    exit outer when matrix(i, j) = TARGET;  -- 두 루프 모두 탈출
  end loop;
end loop outer;
```

### next vs exit

| | `next` | `exit` |
|--|--------|--------|
| 동작 | 현재 반복의 나머지 건너뛰고 다음 반복 | 루프 전체 탈출 |
| 레이블 없음 | 가장 안쪽 루프에 적용 | 가장 안쪽 루프에 적용 |
| 레이블 있음 | 해당 루프의 다음 반복으로 | 해당 루프 탈출 |

---

## wait 문

`wait`는 프로세스를 일시 정지한다. **감도 목록이 있는 프로세스에는 사용 불가** (LRM §11.3).

### 네 가지 형태

```vhdl
wait;                                  -- (1) 무한 대기
wait on sig1, sig2;                    -- (2) 이벤트 대기
wait until condition;                  -- (3) 조건 대기
wait for time_expression;             -- (4) 시간 대기
```

조합도 가능:

```vhdl
wait on clk until clk = '1';          -- 이벤트 + 조건
wait until clk = '1' for 100 ns;      -- 조건 + 타임아웃
wait on sig1, sig2 until cond for 50 ns;
```

### 사용 예시

```vhdl
-- testbench 리셋 시퀀스
process
begin
  rst_n <= '0';
  wait for 20 ns;               -- 20ns 동안 리셋
  rst_n <= '1';
  wait;                         -- 이후 무한 대기 (한 번만 실행)
end process;

-- 클록 에지 기다리기
process
begin
  wait until rising_edge(clk);  -- 상승 에지 대기
  data <= test_vector;
  wait until rising_edge(clk);
  check_output(expected, actual);
end process;

-- 타임아웃 대기 (handshake)
process
begin
  req <= '1';
  wait until ack = '1' for TIMEOUT;
  if ack /= '1' then
    report "Handshake timeout" severity ERROR;
  end if;
  req <= '0';
  wait;
end process;
```

---

## 변수 할당 `:=` vs 신호 할당 `<=`

### 즉시 반영 vs delta cycle 후 반영

```vhdl
process
  variable v : std_logic_vector(7 downto 0) := X"00";
begin
  v := X"FF";          -- 즉시 반영
  result1 <= v;        -- X"FF" 전달 (v 현재 값)

  sig <= X"AA";        -- 스케줄: 다음 delta에 반영
  result2 <= sig;      -- 이전 sig 값 전달 (X"AA" 아직 아님)

  wait for 10 ns;
end process;
```

### 언제 무엇을 쓰는가

| 상황 | 권장 |
|------|------|
| 중간 계산 결과 저장 | `variable` + `:=` |
| 조합 계산 후 출력 | 마지막에 `<=` |
| 레지스터 (FF) | `signal` + `<=` |
| 루프 카운터 | `variable` + `:=` |
| 공유 상태 (주의) | `shared variable` + `:=` |

### 신호 할당 지연 모델

```vhdl
-- inertial (기본): 지연보다 짧은 펄스 필터링
y <= a after 5 ns;
y <= inertial a after 5 ns;   -- 명시적 동일

-- transport: 모든 전이 전달 (전송선 모델)
y <= transport a after 5 ns;

-- reject N ns inertial: 최소 펄스폭 N ns 지정
y <= reject 2 ns inertial a after 5 ns;   -- 2ns 미만 펄스 필터

-- 파형 (waveform): 여러 전이 예약
clk <= '1', '0' after 5 ns, '1' after 10 ns, '0' after 15 ns;
```

---

## assert / report / severity

### assert

```vhdl
-- 기본 형태
assert boolean_condition
  [report string_expression]
  [severity severity_level];

-- 예시: 리셋 후 카운터 초기값 검증
assert counter = 0
  report "Counter not zero after reset, got: " & integer'image(counter)
  severity ERROR;

-- report 없이
assert a /= b severity WARNING;

-- severity 없이 (기본: ERROR)
assert valid = '1'
  report "Data not valid";
```

### report

```vhdl
-- assert 없이 메시지만 출력
report string_expression [severity severity_level];

report "Simulation started at " & time'image(now);
report "Test case 1 passed" severity NOTE;
report "Unexpected condition" severity FAILURE;  -- 즉시 중단
```

### severity 레벨

| 레벨 | 값 | 기본 동작 | 용도 |
|------|----|----------|------|
| `NOTE` | 0 | 메시지 출력, 계속 | 정보성 메시지, 진행 상황 |
| `WARNING` | 1 | 메시지 출력, 계속 | 비정상이지만 치명적이지 않은 상황 |
| `ERROR` | 2 | 메시지 출력, 계속 | 설계 오류, 기본 severity |
| `FAILURE` | 3 | 시뮬레이션 즉시 중단 | 복구 불가 오류 |

- `assert`의 기본 severity: `ERROR` (report/severity 모두 생략 시).
- `report`의 기본 severity: `NOTE`.
- 툴 설정으로 특정 severity에서 시뮬레이션 중단 임계값을 조정할 수 있다.

### 실용 패턴

```vhdl
-- testbench 자기 검사 (self-checking testbench)
process
  variable pass_count : integer := 0;
  variable fail_count : integer := 0;
begin
  -- 테스트 케이스 실행
  apply_stimulus(X"AA");
  wait until rising_edge(clk);
  if output = X"55" then
    pass_count := pass_count + 1;
    report "TC1 PASS" severity NOTE;
  else
    fail_count := fail_count + 1;
    report "TC1 FAIL: expected 0x55, got " & to_hstring(output)
      severity ERROR;
  end if;

  -- 최종 결과
  report "Total: " & integer'image(pass_count) & " pass, "
       & integer'image(fail_count) & " fail";
  assert fail_count = 0
    report "Test FAILED"
    severity FAILURE;

  wait;
end process;
```

---

## return

```vhdl
-- procedure: 값 없이 반환 (조기 탈출)
procedure check_range(val : integer; lo, hi : integer) is
begin
  if val < lo or val > hi then
    report "Out of range" severity ERROR;
    return;             -- 조기 반환
  end if;
  -- 범위 내 처리 계속
end procedure;

-- function: 반드시 값 반환
function max_val(a, b : integer) return integer is
begin
  if a >= b then
    return a;
  else
    return b;
  end if;
end function max_val;
```

---

## 순차문 합성 체크리스트

| 패턴 | 합성 결과 | 주의 |
|------|----------|------|
| `if rising_edge(clk)` | 플립플롭 | 표준 클록 에지 패턴 |
| `if` (else 없음) | 래치 (조합 process) | 모든 분기 커버 필수 |
| `case` (others 없음) | 일부 값 래치 가능 | `when others` 권장 |
| `for i in 0 to N-1` | N번 언롤(unroll) | N은 컴파일 시간 상수 |
| `while` | 합성 제한 — 종료 보장 불가 | RTL에서 회피 |
| `wait` | 합성 제한 | testbench 전용 |
| `variable :=` | 와이어(조합) or FF(동기) | 문맥에 따라 결정 |
| `assert/report` | 합성 툴 무시 | 시뮬레이션 전용 |

---

## Sources

- IEEE 1076-2008 §10 (Sequential statements)
- portal.cs.umbc.edu/help/VHDL/sequential.html ✓
- fpgatutorial.com/vhdl-for-while-loop-if-case-statement/ ✓
- vhdlwhiz.com/sensitivity-list/ ✓ (wait equivalence)
- Doulos VHDL-2008 small changes (case? — 403, snippet 검증)
- GHDL issue #1940 (case? implementation notes)
- Research log: vhdl-design-units-statements-2026-05-28.md
