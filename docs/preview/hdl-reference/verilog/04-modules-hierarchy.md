# 04 · Verilog 모듈과 계층 구조

IEEE 1364-2001/2005 기준. 모듈(module)은 Verilog 설계의 기본 단위다. 포트,
파라미터, 인스턴스, generate 블록을 통해 계층적 설계(hierarchical design)를 구성한다.

---

## 모듈 선언 — ANSI vs non-ANSI 포트 스타일

### non-ANSI 스타일 (Verilog-1995 이래)

포트 이름은 헤더 목록에, 방향·타입은 본문에서 따로 선언한다.

```verilog
module adder (a, b, cin, sum, cout);
    input  [3:0] a, b;
    input        cin;
    output [3:0] sum;
    output       cout;

    assign {cout, sum} = a + b + cin;
endmodule
```

### ANSI 스타일 (Verilog-2001 추가 — 권장)

방향·타입·폭을 포트 목록에 한 번에 선언한다. 중복이 없고 실수를 줄인다.

```verilog
module adder #(
    parameter WIDTH = 4
)(
    input  [WIDTH-1:0] a,
    input  [WIDTH-1:0] b,
    input              cin,
    output [WIDTH-1:0] sum,
    output             cout
);
    assign {cout, sum} = a + b + cin;
endmodule
```

### 비교 요약

| 항목 | non-ANSI | ANSI |
|------|---------|------|
| 도입 표준 | IEEE 1364-1995 | IEEE 1364-2001 |
| 포트 방향 위치 | 모듈 본문 별도 선언 | 포트 목록 내 인라인 |
| 가독성 | 선언이 분산됨 | 한 곳에 집중 |
| 추천 여부 | 레거시 코드 리딩 시 | 신규 코드에 권장 |
| interface 포트 | 불가 | 가능 (SV) |

---

## 포트 방향

| 키워드 | 방향 | 기본 net 타입 | 설명 |
|--------|------|-------------|------|
| `input` | 외부 → 모듈 | wire | 읽기 전용, reg 대입 불가 |
| `output` | 모듈 → 외부 | wire (reg 지정 가능) | 내부에서 구동 |
| `inout` | 양방향 | wire | tri-state 버스; 비사용 시 `z` 구동 필요 |

```verilog
// inout 사용 예 (양방향 버스 드라이버)
module bus_driver (
    inout  [7:0] data_bus,
    input        oe,        // output enable
    input  [7:0] tx_data,
    output [7:0] rx_data
);
    assign data_bus = oe ? tx_data : 8'bz;
    assign rx_data  = data_bus;
endmodule
```

---

## 파라미터 (parameter / localparam)

### parameter — 인스턴스 시점에 override 가능

```verilog
module fifo #(
    parameter DEPTH = 16,
    parameter WIDTH = 8
)(
    input              clk, rst,
    input  [WIDTH-1:0] din,
    output [WIDTH-1:0] dout
);
    // ...
endmodule
```

### localparam — 내부 상수 (외부 override 불가)

```verilog
module fsm (input clk, rst, input go, output done);
    localparam IDLE  = 2'b00;
    localparam RUN   = 2'b01;
    localparam DONE  = 2'b10;

    reg [1:0] state;
    // ...
endmodule
```

### defparam — deprecated ⚠️

`defparam`은 인스턴스 선언과 분리된 위치에서 파라미터를 강제 변경할 수 있었다.
IEEE 1800-2017 §23.10에서 명시적으로 deprecated 처리되었다.

```verilog
// ❌ defparam — 사용 금지
defparam u_fifo.DEPTH = 32;
defparam u_fifo.WIDTH = 16;

// ✅ 대체: named parameter override (Verilog-2001 이후 권장)
fifo #(.DEPTH(32), .WIDTH(16)) u_fifo (.clk(clk), .rst(rst), ...);
```

**deprecated 이유**: defparam 문이 인스턴스와 파일이나 위치가 달라도 적용되어
가독성과 도구 복잡도 문제를 만든다. 현대 Lint/합성 도구는 대부분 경고 또는 오류를 출력한다.

---

## 모듈 인스턴스화

### named 연결 (권장)

```verilog
adder #(.WIDTH(8)) u_adder (
    .a   (op_a),
    .b   (op_b),
    .cin (carry_in),
    .sum (result),
    .cout(carry_out)
);
```

포트 이름을 명시하므로 모듈 포트 순서가 바뀌어도 연결이 유지된다.

### positional 연결 (비권장)

```verilog
adder u_adder (op_a, op_b, carry_in, result, carry_out);
```

포트 선언 순서에 의존 — 순서 변경 시 무성하게 잘못 연결될 수 있다.

### 포트 연결 규칙

| 경우 | 결과 |
|------|------|
| output → wire | net에 직접 연결 |
| output → reg | 포트에서는 wire, 내부 reg가 assign으로 구동 |
| input 미연결 | 고임피던스(z)로 처리 |
| output 미연결 | 허용 (단, 경고) |

---

## generate 블록 (Verilog-2001)

generate 블록은 **elaboration time**에 구문을 조건부로 확장하거나 반복 인스턴스화한다.
`genvar`는 elaboration time 전용 정수 변수 — 시뮬레이션에는 존재하지 않는다.

### generate-for: 반복 인스턴스화

```verilog
module ripple_carry #(parameter N = 4)(
    input  [N-1:0] a, b,
    input          cin,
    output [N-1:0] sum,
    output         cout
);
    wire [N:0] carry;
    assign carry[0] = cin;

    genvar i;
    generate
        for (i = 0; i < N; i = i + 1) begin : gen_fa
            full_adder u_fa (
                .a   (a[i]),
                .b   (b[i]),
                .cin (carry[i]),
                .sum (sum[i]),
                .cout(carry[i+1])
            );
        end
    endgenerate

    assign cout = carry[N];
endmodule
```

`begin : gen_fa` — 레이블을 붙이면 `gen_fa[0].u_fa`, `gen_fa[1].u_fa` 계층명으로 접근 가능.
레이블 없는 generate loop에 대해 일부 도구는 경고를 내므로 항상 이름을 붙이는 것이 좋다.

### generate-if: 조건부 구현 선택

```verilog
module adder_impl #(parameter USE_CARRY_LOOKAHEAD = 0)(
    input  [7:0] a, b,
    input        cin,
    output [7:0] sum,
    output       cout
);
    generate
        if (USE_CARRY_LOOKAHEAD) begin : gen_cla
            cla_adder u_add (.a(a), .b(b), .cin(cin), .sum(sum), .cout(cout));
        end else begin : gen_rca
            rca_adder u_add (.a(a), .b(b), .cin(cin), .sum(sum), .cout(cout));
        end
    endgenerate
endmodule
```

### generate-case: 다중 구현 선택

```verilog
module encoder #(parameter TYPE = 0)( ... );
    generate
        case (TYPE)
            0: priority_enc u_enc ( ... );
            1: onehot_enc   u_enc ( ... );
            default: $error("Unknown encoder type");
        endcase
    endgenerate
endmodule
```

### generate 블록 내 허용/금지 항목

| 허용 | 금지 |
|------|------|
| module instance | port declaration (input/output) |
| gate primitive | specify block |
| continuous assign | parameter/localparam 선언 |
| initial / always 블록 | |
| 데이터 타입 (net, reg, integer, real) | |
| task/function (if/case generate 내에서만) | |

---

## 계층적 이름 접근 (Hierarchical Name)

dot-separated path로 설계의 어느 위치에 있는 신호든 참조할 수 있다.

```verilog
// 계층: tb → dut → core → alu
$display("ALU result: %h", tb.dut.core.alu.result);

// generate block 레이블이 있는 경우
$display("FA cout[2]: %b", tb.dut.u_rca.gen_fa[2].u_fa.cout);
```

주요 사용처:

- 테스트벤치에서 내부 신호 관측 (`$monitor`, `$display`, force/release)
- `defparam`의 대상 지정 (비권장 — 위 항목 참고)
- 디버그 중 특정 레지스터 직접 읽기

**주의**: 계층적 참조는 합성 불가 — 시뮬레이션/검증 전용.

---

## Sources

- IEEE 1364-2001 §12 (module declaration, ANSI port syntax)
- IEEE 1364-2005 §12, §14 (parameter, generate)
- IEEE 1800-2017 §23 (module definitions and hierarchy), §23.10 (defparam deprecated)
- sigasi.com/tech/ansi-vs-non-ansi/ (ANSI vs non-ANSI 포트 — WebFetch 검증)
- chipverify.com/verilog/verilog-parameters (parameter/defparam — WebFetch 검증)
- chipverify.com/verilog/verilog-generate-block (generate 구문 — WebFetch 검증)
- vlsiverify.com/verilog/generate-blocks-in-verilog/ (genvar, 계층명 — WebFetch 검증)
- sutherland-hdl.com/pdfs/verilog_2001_ref_guide.pdf (Verilog-2001 Quick Reference)
