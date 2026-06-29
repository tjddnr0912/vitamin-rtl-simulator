# 01 · SystemVerilog 데이터 타입

IEEE 1800-2017 §6/§7 기준. Verilog가 4-state 타입만 가졌던 것에 비해 SV는 2-state 타입,
집합 타입(enum/struct/union), 그리고 검증용 특수 타입을 추가했다.

---

## 2-state vs 4-state

Verilog의 모든 타입은 4-state(0/1/X/Z)였다. SV는 X/Z가 없는 2-state 타입을 도입해
시뮬레이션 속도를 높이고 소프트웨어 정수 연산과의 호환성을 개선했다.

### 타입 비교표

| 타입 | state | 크기 | 부호 | Verilog 대응 |
|------|-------|------|------|-------------|
| `logic` | 4 | 가변 | unsigned | wire + reg 통합 대체 |
| `reg` | 4 | 가변 | unsigned | reg (레거시) |
| `integer` | 4 | 32비트 | signed | integer (레거시) |
| `time` | 4 | 64비트 | unsigned | time (합성 불가) |
| `bit` | 2 | 가변 | unsigned | — (SV 신규) |
| `byte` | 2 | 8비트 | signed | — (SV 신규) |
| `shortint` | 2 | 16비트 | signed | — (SV 신규) |
| `int` | 2 | 32비트 | signed | — (SV 신규) |
| `longint` | 2 | 64비트 | signed | — (SV 신규) |

- 2-state 타입의 초기화 기본값은 `0` (4-state는 `X`)
- 2-state는 시뮬레이션 속도 이점이 있으나, RTL에서 X-propagation이 필요하면 `logic`을 유지
- `real`(64비트 IEEE 754) / `shortreal`(32비트)은 SV에서도 유지 — 합성 불가

### logic vs reg 구분

`logic`은 SV §6.3.4에서 정의된 4-state 변수 타입이다. Verilog에서 `wire`는 `assign`/포트
연결에만, `reg`는 `always`/`initial`에만 쓸 수 있어 컨텍스트마다 암기가 필요했다.
`logic`은 두 컨텍스트를 모두 허용하되, 단일 드라이버를 컴파일 타임에 강제한다.

```systemverilog
logic       flag;           // 1비트
logic [7:0] data;           // 8비트 벡터
bit   [3:0] nibble;         // 4비트 2-state
int         count;          // 32비트 signed 2-state
```

다중 드라이버 버스(`tri`, `wor` 등)는 기존 net 타입을 그대로 사용한다.

---

## Enum

사용자 정의 열거 타입. 기본 타입(`int`, `logic` 등)을 지정할 수 있다.

```systemverilog
// 명시 값 — 지정하지 않은 항목은 이전+1 자동 증가
typedef enum logic [1:0] {
    IDLE  = 2'b00,
    RUN   = 2'b01,
    DONE  = 2'b10,
    ERROR             // 자동 = 2'b11
} state_e;

state_e s = IDLE;
```

### 내장 메소드 (§6.19)

| 메소드 | 동작 |
|--------|------|
| `.first()` | 첫 번째 멤버 값 반환 |
| `.last()` | 마지막 멤버 값 반환 |
| `.next(N)` | N번째 다음 값 (기본 N=1) |
| `.prev(N)` | N번째 이전 값 (기본 N=1) |
| `.num()` | 멤버 총 개수 반환 |
| `.name()` | 현재 값의 문자열 표현 반환 |

```systemverilog
state_e s = IDLE;
s = s.next();              // RUN
$display("%s", s.name());  // "RUN"
int n = state_e.num();     // 4
```

---

## Struct

여러 타입의 멤버를 하나의 이름으로 묶는다.

### packed struct

멤버들이 연속 비트 벡터로 매핑된다. 슬라이싱 가능. 합성 가능.

```systemverilog
typedef struct packed {
    logic [3:0]  opcode;
    logic [11:0] address;
    logic [7:0]  data;
} instr_t;   // 24비트 단일 벡터

instr_t ins;
ins.opcode = 4'hA;
logic [23:0] raw = ins;  // 전체를 벡터로 접근
```

### unpacked struct

멤버 사이 갭을 허용하며 합성 불가. 검증/모델링용.

```systemverilog
typedef struct {
    int     id;
    string  name;
    real    score;
} student_t;
```

### rand / randc 필드

클래스 내 struct 필드에 `rand`/`randc` 한정자를 붙이면 `randomize()` 호출 시 무작위화에 포함된다.

---

## Union

모든 멤버가 동일한 저장 공간을 공유한다.

### packed union

모든 멤버가 동일 비트 폭이어야 한다. 합성 가능.

```systemverilog
typedef union packed {
    logic [31:0]       word;
    logic [3:0][7:0]   bytes;   // 4 × 8비트
} word_u;

word_u u;
u.word = 32'hDEAD_BEEF;
$display("%h", u.bytes[3]);   // DE
```

### tagged union

마지막에 쓴 멤버를 내부 태그로 기록한다. 다른 멤버로 읽으면 런타임 에러.
멤버 크기가 달라도 되며, 합성 불가.

```systemverilog
typedef union tagged {
    int        a;
    byte       b;
    bit [15:0] c;
} data_t;

data_t d;
d = tagged a 32'hffff;
// d.b;  → 런타임: "Invalid member usage of a tagged union"
```

---

## typedef

타입 별칭 선언. struct/union/enum과 함께 이름을 부여하는 것이 표준 패턴.

```systemverilog
typedef logic [7:0] byte_t;
typedef struct packed { logic [7:0] r, g, b; } rgb_t;
```

---

## string

동적 크기 문자열. 합성 불가.

```systemverilog
string s = "Hello";
int    n = s.len();         // 길이
s = {s, " World"};         // 연결 연산
s = s.toupper();           // 대문자 변환
int v = s.atoi();          // 문자열 → 정수
```

---

## chandle

DPI(Direct Programming Interface)를 통해 C/C++ 함수가 반환한 포인터를 SV에서 저장·전달하는
불투명 핸들 타입. SV 코드에서는 비교(`==`, `!=`, `null` 체크)만 가능하고 역참조는 불가.

```systemverilog
import "DPI-C" function chandle alloc_ctx();
chandle ctx = alloc_ctx();
```

---

## virtual interface

interface 인스턴스에 대한 핸들. 클래스는 포트를 가질 수 없기 때문에, 클래스 기반 검증
컴포넌트가 하드웨어 신호에 접근하기 위해 사용한다. interface 문서는 `04-interfaces.md` 참조.

```systemverilog
interface axi_if(input logic clk);
    logic [31:0] addr;
    // ...
endinterface

class Driver;
    virtual axi_if vif;   // 핸들 — 실제 인터페이스 인스턴스는 외부에서 주입
    task run();
        vif.addr = 32'h0;
    endtask
endclass
```

---

## Sources

- IEEE 1800-2017 §6 (Data types), §7 (Aggregate types)
- chipverify.com/systemverilog/systemverilog-quick-refresher
- chipverify.com/systemverilog/systemverilog-enumeration
- verilogpro.com/systemverilog-structures-unions-design/
- vlsitrainers.com/system-verilog-union-packed-unpacked-and-tagged/
- circuitcove.com/data-types-enum/
