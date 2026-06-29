# 07 · Verilog Tasks와 Functions

IEEE 1364-2001/2005 기준. 재사용 가능한 코드 블록을 두 가지 방식으로 묶는다:
task는 시간을 소비할 수 있는 절차적 서브루틴, function은 0 시간 안에 단일 값을
반환하는 순수 계산 블록이다.

---

## Task vs Function — 핵심 차이

| 항목 | `task` | `function` |
|------|--------|-----------|
| 실행 시간 | 시간 소비 허용 (`#`, `@`, `wait` 가능) | 반드시 0 시뮬레이션 시간 |
| 포트 방향 | `input` / `output` / `inout` 모두 허용 | `input`만 허용 |
| 반환값 | 없음 (output 포트로 결과 전달) | 단일 값 (함수 이름 변수에 할당) |
| 호출 위치 | 절차 블록(`initial`, `always`) 안 | 절차 블록 + 연속 대입(assign) 안 |
| 최소 포트 수 | 0개 이상 | 1개 이상 (input 최소 1개) |

function이 시간 제어 구문을 가지면 컴파일 에러다:

```verilog
// ❌ function 안에서 시간 소비 — 불법
function [7:0] bad_func(input [7:0] a);
    #10 bad_func = a;    // 에러: function은 #delay 불가
endfunction
```

---

## Task 선언

두 가지 스타일이 모두 유효하다.

### 전통 스타일 (IEEE 1364-1995 호환)

```verilog
task drive_bus;
    input  [7:0] addr;
    input  [7:0] wdata;
    output [7:0] rdata;
    output       ack;
    begin
        @(posedge clk);          // 클록 에지 대기 — task에서만 허용
        bus_addr  = addr;
        bus_wdata = wdata;
        @(posedge ack_signal);
        rdata = bus_rdata;
        ack   = 1;
    end
endtask
```

### ANSI 포트 스타일 (IEEE 1364-2001+)

```verilog
task drive_bus(
    input  [7:0] addr,
    input  [7:0] wdata,
    output [7:0] rdata,
    output       ack
);
    @(posedge clk);
    bus_addr  = addr;
    bus_wdata = wdata;
    @(posedge ack_signal);
    rdata = bus_rdata;
    ack   = 1;
endtask
```

### 호출

```verilog
initial begin
    drive_bus(8'hA0, 8'hFF, read_data, ack_flag);
    $display("rdata=%0h ack=%b", read_data, ack_flag);
end
```

---

## Function 선언

### 전통 스타일

```verilog
function [7:0] add8;
    input [7:0] a;
    input [7:0] b;
    add8 = a + b;    // 함수 이름 변수에 할당 = 반환값
endfunction
```

### ANSI 스타일

```verilog
function [7:0] add8(input [7:0] a, b);
    add8 = a + b;
endfunction
```

### 반환값 — 이름 할당 방식

Verilog-2001에서 반환값은 **함수 이름과 동일한 내부 변수에 대입**하는 방식만 허용된다.
SystemVerilog(IEEE 1800)에서 추가된 `return` 키워드는 순수 Verilog-2001 파싱에서
사용할 수 없다:

```verilog
// ✅ 순수 Verilog-2001: 이름 대입
function [31:0] max(input [31:0] a, b);
    max = (a > b) ? a : b;
endfunction

// ✅ SystemVerilog / IEEE 1800: return 키워드
function automatic [31:0] max(input [31:0] a, b);
    return (a > b) ? a : b;
endfunction
```

### 호출

function은 연속 대입과 절차 블록 양쪽에서 호출할 수 있다:

```verilog
assign y = add8(sig_a, sig_b);

always @(*) begin
    result = max(data_a, data_b);
end
```

---

## automatic vs static

### static (기본값)

선언된 지역 변수가 **모든 호출 간에 공유**된다. 동시 호출이 발생하면 값을 덮어쓴다.

```verilog
task static_counter;
    integer count = 0;    // 모든 호출이 같은 count를 공유
    count = count + 1;
    $display("count = %0d", count);
endtask
```

### automatic

호출마다 **독립 스택 프레임**이 동적으로 할당된다. 동시 호출, 재귀에 모두 안전하다.

```verilog
task automatic safe_counter;
    integer count = 0;    // 이 호출 전용 count
    count = count + 1;
    $display("count = %0d", count);
endtask
```

**규칙**: automatic으로 선언하지 않은 task/function에서 재귀를 시도하면
동일 변수를 덮어써 결과가 틀린다. 재귀가 필요하면 반드시 `automatic`.

---

## 재귀 — automatic만 가능

재귀는 `automatic` 키워드 없이는 동작하지 않는다:

```verilog
// ✅ automatic function — 팩토리얼 재귀
function automatic integer factorial(input integer n);
    if (n <= 1)
        factorial = 1;
    else
        factorial = n * factorial(n - 1);
endfunction

// 호출
initial begin
    $display("4! = %0d", factorial(4));  // 출력: 4! = 24
end
```

```verilog
// ✅ automatic task — 재귀적 트리 순회 예시
task automatic traverse(input integer node);
    if (node == 0) return;          // SV return; Verilog-2001에서는 disable 사용
    $display("node %0d", node);
    traverse(node / 2);
endtask
```

재귀 종료 조건(base case) 없이 무한 재귀하면 시뮬레이터 스택 오버플로가 발생한다.

---

## inout 포트

task는 `inout` 포트를 가질 수 있다. 호출 시 값을 받아 수정 후 돌려준다:

```verilog
task automatic swap(inout [7:0] a, b);
    logic [7:0] tmp;
    tmp = a;
    a   = b;
    b   = tmp;
endtask

initial begin
    logic [7:0] x = 8'hAA, y = 8'h55;
    swap(x, y);
    $display("x=%0h y=%0h", x, y);  // x=55 y=aa
end
```

---

## disable — task 조기 종료

`disable task_name;`으로 실행 중인 task를 즉시 종료한다. C의 `return`에 해당한다
(Verilog-2001에는 `return` 없음):

```verilog
task automatic find_first(input [7:0] data, output integer idx);
    integer i;
    idx = -1;
    for (i = 0; i < 8; i = i + 1) begin
        if (data[i]) begin
            idx = i;
            disable find_first;    // 즉시 종료
        end
    end
endtask
```

---

## 시스템 태스크 vs 사용자 태스크

| 구분 | 형태 | 예시 | 합성 |
|------|------|------|------|
| 시스템 태스크 | `$name(...)` | `$display`, `$finish`, `$random` | ❌ 시뮬레이션 전용 |
| 사용자 태스크 | `task...endtask` | 직접 정의 | 조건부 (시간 소비 없으면) |

시스템 태스크는 Verilog 표준이 정의하고 시뮬레이터가 구현한다. RTL 코드에서는
시뮬레이션 전용 코드임을 `` `ifdef `` 조건부 컴파일로 감싸거나 testbench로 분리한다.

```verilog
// ✅ 합성 도구가 무시하도록 conditionally compile
`ifdef SIMULATION
initial begin
    $monitor("q = %b", q);
end
`endif
```

---

## task / function 선택 기준

```
시간 소비가 필요한가?
  └─ YES → task
  └─ NO  → 여러 출력이 필요한가?
              └─ YES → task (output 포트 사용)
              └─ NO  → function
```

---

## Sources

- IEEE 1364-2001 §10.1–§10.3 (tasks and functions)
- IEEE 1800-2017 §13.2–§13.5 (tasks), §13.4 (functions), §13.4.4 (return)
- chipverify.com/verilog/verilog-task (WebFetch 검증 ✓)
- chipverify.com/verilog/verilog-functions (WebFetch 검증 ✓)
- vlsiverify.com/system-verilog/tasks/ (automatic vs static 동작 비교)
- fpgatutorial.com/verilog-function-and-task/ (선택 기준 가이드)
