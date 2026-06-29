# 02 · Verilog Data Types

IEEE 1364-2005 §4 기준. Verilog 데이터 타입은 크게 **넷(net)**과 **변수(variable)**
두 범주로 나뉜다. 넷은 구조적 연결을 표현하고, 변수는 값을 저장한다.

---

## Net vs. Variable 구분

| 구분 | 대표 타입 | 대입 방법 | 값 저장 |
|------|----------|----------|---------|
| Net | wire, tri, wand, … | `assign` (continuous), 포트 연결 | 드라이버가 결정 — 자체 저장 없음 |
| Variable | reg, integer, real | `always`, `initial` (procedural) | 다음 대입까지 값 유지 |

- `wire`에 `always` 블록에서 대입하면 문법 오류.
- `reg`를 `assign`으로 구동하는 것도 불가. (SV의 `logic`은 둘 다 허용)

---

## Net 타입 9종

IEEE 1364-2005는 아래 9종 + `uwire`(2005 추가)를 정의한다.

| 타입 | 비구동 기본값 | 다중 드라이버 충돌 해소 | 주요 용도 |
|------|-------------|----------------------|----------|
| `wire` | `z` | `x` (unknown) | 일반 배선 — 가장 많이 쓰임 |
| `tri` | `z` | `x` | 다중 드라이버 버스 (wire와 동일, 가독성 구분) |
| `wand` | `z` | AND 해소 (0 우세) | 와이어드-AND |
| `triand` | `z` | AND 해소 (0 우세) | 다중 드라이버 와이어드-AND |
| `wor` | `z` | OR 해소 (1 우세) | 와이어드-OR |
| `trior` | `z` | OR 해소 (1 우세) | 다중 드라이버 와이어드-OR |
| `tri0` | `0` (pull 강도) | `x` | 내장 풀다운 저항 모델링 |
| `tri1` | `1` (pull 강도) | `x` | 내장 풀업 저항 모델링 |
| `supply0` | `0` (supply 강도) | — | GND / 전원 핀 |
| `supply1` | `1` (supply 강도) | — | VCC / 전원 핀 |
| `trireg` | 직전 값 유지 | `x` | 커패시티브 노드 (charge storage) |
| `uwire` | `z` | 컴파일 에러 | 단일 드라이버 강제 (1364-2005 추가) |

### trireg 상세

`trireg`는 Verilog 넷 중 유일하게 값을 저장한다. 드라이버가 활성화되면
드라이버 값(0/1/x)을 따르고, 드라이버가 모두 `z`가 되면 직전 값을
`small`/`medium`/`large` 중 하나의 전하 강도로 유지한다.

```verilog
trireg (small)  cap_node;   // 약한 전하 유지
trireg          bus_node;   // 기본(medium) 전하
trireg (large)  strong_cap; // 강한 전하 유지
```

### supply0 / supply1

전원 핀은 드라이버 충돌이 의미 없다 — 항상 최강(supply) 강도로 구동.
게이트 레벨 회로에서 VCC/GND를 명시할 때 사용한다.

```verilog
supply1 vcc;
supply0 gnd;
```

### wand / wor (와이어드 로직)

오픈-드레인 출력 여러 개를 직접 연결하는 회로 구조를 표현할 때 사용.

```verilog
wand  pull_low;  // 어느 드라이버 하나라도 0을 구동하면 0
wor   bus_req;   // 어느 드라이버 하나라도 1을 구동하면 1
```

### 넷 선언 문법

```
net_type [signed] [drive_strength] [vectored|scalared] [range] [delay] identifier [= expression];
```

```verilog
wire                data;           // 1비트 wire
wire [7:0]          data_bus;       // 8비트 wire 벡터
wire signed [15:0]  offset;         // signed wire (1364-2001+)
tri  (strong1, weak0) [3:0] bus;    // 드라이브 강도 지정
wand #(5, 3)        w;              // rise=5, fall=3 지연
```

---

## 변수(Variable) 타입

### reg

값을 저장하는 가장 기본적인 변수. 하드웨어 레지스터와 반드시 일치하지 않는다
(combinational logic도 `reg`로 모델링됨).

```verilog
reg         flag;           // 1비트 unsigned
reg [7:0]   data;           // 8비트 unsigned
reg signed [7:0]  acc;      // 8비트 signed (1364-2001+)
```

### integer

32비트 signed 정수. RTL에서는 루프 카운터나 정수 연산에 사용.
합성 시 레지스터로 추론될 수 있다.

```verilog
integer i;        // 루프 카운터
integer count;    // 32비트 signed
```

### real / realtime

64비트 IEEE 754 배정도 부동소수점. 시뮬레이션 전용 — **합성 불가**.

```verilog
real     tau = 1.5e-9;
realtime current_time;
```

### time

64비트 unsigned 정수. `$time` 반환값 저장에 주로 사용. **합성 불가**.

```verilog
time     start_time;
time     elapsed;
```

> **vitamin: IN-MVP** — 64-bit unsigned 4-state 변수(절차 대입 가능, `assign` 구동 불가,
> 초기값 all-X). unpacked 배열·`parameter time`까지 수용(iverilog 차분 일치).

---

## 벡터(Vector) 선언

```verilog
// 선언 문법
data_type [msb:lsb] identifier;

// 예시
wire  [7:0]   byte_bus;    // 8비트, MSB=7, LSB=0
reg   [15:0]  word_reg;    // 16비트
reg   [0:7]   rev_byte;    // 역순 (MSB=0, LSB=7) — 합성 가능하나 비권장
```

- `[msb:lsb]` 에서 msb ≥ lsb 관례 (big-endian 비트 순서)를 따르는 것이 표준
- 역순 `[0:N-1]` 도 문법상 허용되나 슬라이스 방향이 반전됨

### 비트/부분 선택

```verilog
data_bus[3]        // 단일 비트 선택
data_bus[5:2]      // 4비트 파트 선택 (범위는 상수)
data_bus[base+:4]  // base부터 4비트 위로 (1364-2001+)
data_bus[base-:4]  // base부터 4비트 아래로 (1364-2001+)
```

### scalared / vectored 키워드

```verilog
reg  scalared [7:0] a;   // 비트 단위 접근 허용
wire vectored [7:0] b;   // 벡터 전체 단위 최적화 힌트
```

시뮬레이터 최적화 힌트일 뿐, 동작 의미는 동일하다.

---

## parameter / localparam

### parameter

모듈 외부(인스턴스화 시)에서 override 가능한 상수.

```verilog
module fifo #(
    parameter DATA_W  = 8,
    parameter DEPTH   = 16
) (
    input  wire [DATA_W-1:0] din,
    output wire [DATA_W-1:0] dout
);
    // ...
endmodule

// 인스턴스화 시 override
fifo #(.DATA_W(16), .DEPTH(256)) u_fifo (…);
```

타입·범위 명시:

```verilog
parameter signed [7:0]  OFFSET = -1;
parameter integer       MAX_COUNT = 100;
parameter real          CLK_PERIOD = 10.0;
```

### localparam

모듈 내부 상수 — `defparam`이나 `#()` override 불가.

```verilog
localparam HALF_W  = DATA_W / 2;
localparam NUM_SEL = $clog2(DEPTH);  // 합성 환경에서 사용 가능
```

---

## 메모리(배열) 선언

```verilog
// 기본 문법
reg [width-1:0] mem_name [0:depth-1];

// 예시
reg [7:0]    ram  [0:255];      // 256 × 8비트 RAM (2048비트)
reg [31:0]   rom  [0:1023];     // 1K × 32비트 ROM
reg [N-1:0]  lut  [0:M-1];     // 파라미터화 LUT

// 접근
ram[addr]         = data;       // 워드 쓰기
data = ram[addr];               // 워드 읽기
// 단, 배열 전체를 한 번에 읽거나 벡터로 쓰는 것은 불가
// 비트 선택도 불가: ram[addr][3]  (시뮬레이터마다 다름 — 피할 것)
```

Verilog-2005의 메모리는 **1차원 배열**만 지원. 다차원 배열은 SV에서 추가.

---

## Sources

- IEEE 1364-2005 §4 (Declarations)
- IEEE 1800-2017 §6 (Data types — Verilog-compat)
- chipverify.com/verilog/verilog-net-types
- peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00030.htm
- verilogpro.com/verilog-reg-verilog-wire-systemverilog-logic/
- circuitcove.com/data-types-net-types/
