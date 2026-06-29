# 09 · Verilog 컴파일러 지시자 (Compiler Directives)

IEEE 1364-2001/2005 기준. 백틱(`` ` ``)으로 시작하며 컴파일러·시뮬레이터에게 명령을
내린다. 한 번 선언되면 **파일 경계와 모듈 경계를 모두 넘어** 이후 소스에 계속 효력이
미친다. 합성 대상 하드웨어를 기술하는 것이 아니라 컴파일 방식을 제어한다.

---

## `define — 매크로 정의

### 단순 상수 매크로

```verilog
`define DATA_WIDTH 8
`define RESET_VAL  8'h00

// 사용 — 항상 백틱으로 참조
wire [`DATA_WIDTH-1:0] bus;
assign bus = `RESET_VAL;
```

### 인수 있는 함수형 매크로

```
`define MACRO_NAME(arg1, arg2, ...) 매크로_본문
```

```verilog
`define MAX(a, b)   ((a) > (b) ? (a) : (b))
`define ADD3(x, y, z) ((x) + (y) + (z))

// 사용
assign result = `MAX(sig_a, sig_b);
assign sum    = `ADD3(p, q, r);
```

**인수를 반드시 괄호로 감싸는 이유**: 인수에 연산자가 들어오면 우선순위 문제가 생긴다.

```verilog
`define DOUBLE(x) x * 2          // ❌ 위험
`define DOUBLE(x) ((x) * 2)      // ✅ 안전

assign y = `DOUBLE(a + b);
// ❌ 전개: a + b * 2  → 잘못된 결과
// ✅ 전개: ((a + b) * 2)
```

### 여러 줄 매크로 (백슬래시 줄 이어 쓰기)

```verilog
`define LONG_EXPR(a, b, c) \
    ((a) * (b) + \
     (c))

assign result = `LONG_EXPR(p, q, r);
```

마지막 줄 뒤에는 백슬래시가 없어야 한다.

### 토큰 붙이기 (Token Paste)

두 백틱 ` `` ` 으로 인수를 서로 붙인다:

```verilog
`define SIGNAL(n) sig_``n

// `SIGNAL(a) → sig_a
// `SIGNAL(7) → sig_7
```

---

## `undef — 매크로 해제

```verilog
`define TEMP 100
// ... TEMP 사용 ...
`undef TEMP
// 이후 `TEMP 참조 → 컴파일 에러
```

파일 범위를 제한하거나 헤더 파일 끝에서 정리하는 용도로 사용한다.

---

## `ifdef / `ifndef / `elsif / `else / `endif — 조건부 컴파일

```verilog
`define SYNTHESIS

`ifdef SYNTHESIS
    // 합성 전용 코드 (시뮬레이터는 이 구간을 읽지 않음)
    assign out = fast_path;
`elsif FPGA_TARGET
    // FPGA 전용 코드
    assign out = fpga_path;
`else
    // 나머지 (시뮬레이션)
    assign out = sim_path;
`endif
```

`ifndef`는 `ifdef`의 반전이다:

```verilog
`ifndef GATE_SIM
initial $display("RTL simulation");
`endif
```

`ifdef / `else / `endif 는 중첩 가능하다. 하지만 단계가 깊어지면 가독성이 떨어지므로
최소화한다.

---

## `include — 파일 삽입

```verilog
`include "defs.vh"
`include "../common/params.vh"
`include "/abs/path/to/defines.vh"
```

해당 위치에 파일 전체 내용이 삽입된다. 검색 경로는 컴파일러 옵션으로 추가한다:

```
iverilog -I ./include -I ../shared ...
vcs     +incdir+./include+../shared ...
```

상대 경로는 **현재 소스 파일 위치** 기준이다 (컴파일러 실행 위치가 아님). 헤더 파일은
중복 삽입 방지를 위해 include guard 패턴을 사용한다:

```verilog
// defs.vh
`ifndef DEFS_VH
`define DEFS_VH
`define CLK_PERIOD 10
// ...
`endif  // DEFS_VH
```

---

## `timescale — 시간 단위와 정밀도

```
`timescale <time_unit> / <time_precision>
```

```verilog
`timescale 1ns  / 1ps    // 단위 1 ns, 정밀도 1 ps
`timescale 10ns / 1ns    // 단위 10 ns, 정밀도 1 ns
`timescale 1us  / 100ns  // 단위 1 µs, 정밀도 100 ns
```

허용 단위: `1`, `10`, `100` 조합 + `s / ms / us / ns / ps / fs`.
정밀도는 반드시 단위보다 작거나 같아야 한다 (`1ns/10ns`는 불법).

### 범위 동작

`timescale`은 선언된 뒤 이후 모든 모듈에 적용된다. 파일 경계를 넘는다.
여러 파일에서 서로 다른 `timescale`을 선언하면 **마지막 선언이 이후를 덮어쓴다**.

```verilog
// fileA.v
`timescale 1ns/1ps
module A; ... endmodule

// fileB.v  (fileA.v 후에 컴파일)
`timescale 1us/1ns
module B; ... endmodule
// 이 시점부터 A 모듈의 내부도 1us/1ns로 재해석될 수 있음
```

이 누수를 막으려면 각 파일 앞에 `` `resetall ``을 놓고 원하는 `timescale`을 재선언한다:

```verilog
// fileA.v — 안전 패턴
`resetall
`timescale 1ns/1ps
`default_nettype none
module A; ... endmodule
```

---

## `default_nettype — 암묵적 net 타입 지정

```verilog
`default_nettype none    // 암묵적 net 선언 비활성화
`default_nettype wire    // 기본값 (암묵적 wire 허용)
```

### `default_nettype none 의 실용적 이점

기본값 `wire` 상태에서는 선언하지 않은 신호 이름이 자동으로 1비트 `wire`로 만들어진다.
오탈자 net이 조용히 생성되어 연결이 끊긴 채 시뮬레이션이 X를 전파한다:

```verilog
// ❌ default_nettype wire (기본값) — 위험
module buggy(output y, input a, b);
    assign y = aaaa & b;   // 오타: 'a' → 'aaaa'
    // 'aaaa'가 암묵적 wire로 생성됨 → a와 단절, y는 항상 0
    // 컴파일 에러 없음 → 디버깅 매우 어려움
endmodule
```

```verilog
// ✅ default_nettype none — 안전
`default_nettype none
module safe(output y, input a, b);
    assign y = aaaa & b;   // 'aaaa' 미선언 → 즉시 컴파일 에러
endmodule
```

파일 끝에서 기본값을 복원해 다른 파일에 영향을 주지 않는다:

```verilog
`default_nettype none
// ... 모듈 선언 ...
`default_nettype wire   // 복원 (또는 `resetall)
```

---

## `begin_keywords / `end_keywords — 예약어 집합 제어

```verilog
`begin_keywords "1364-2001"
// 이 구간에서 Verilog-2001 키워드만 예약어로 인식
// SV 추가 키워드(interface, program 등)는 식별자로 사용 가능
module old_code;
    wire interface;   // SV에서는 예약어지만 이 구간에서는 허용
endmodule
`end_keywords
```

### 유효 버전 문자열

| 버전 문자열 | 키워드 집합 |
|-----------|-----------|
| `"1364-1995"` | Verilog-95 키워드 |
| `"1364-2001"` | Verilog-2001 키워드 |
| `"1364-2005"` | Verilog-2005 키워드 |
| `"1800-2005"` | SystemVerilog-2005 키워드 |
| `"1800-2009"` | SystemVerilog-2009 키워드 |
| `"1800-2012"` | SystemVerilog-2012 키워드 |
| `"1800-2017"` | SystemVerilog-2017 키워드 (기본) |

모듈·프리미티브·인터페이스·프로그램·패키지 **바깥에서만** 선언할 수 있다.
주 용도: SV 툴체인으로 구버전 Verilog 코드를 처리할 때 새 키워드가 기존 식별자와
충돌하는 것을 막는다.

---

## `resetall — 모든 지시자 초기화

모든 기본값이 있는 지시자를 초기값으로 되돌린다. 영향을 받는 지시자:
- `` `timescale `` (제거)
- `` `default_nettype `` → `wire`
- `` `unconnected_drive `` (제거)
- `` `celldefine `` / `` `endcelldefine `` (제거)

```verilog
// 파일 시작에 놓는 관례 — 다른 파일에서 흘러든 설정을 초기화
`resetall
`timescale 1ns/1ps
`default_nettype none

module my_module;
    // ...
endmodule

`resetall   // 파일 끝에서 복원 (선택적)
```

---

## `celldefine / `endcelldefine — 라이브러리 셀 표시

사이에 선언된 모듈을 라이브러리 셀로 마킹한다. SDF(Standard Delay Format)
back-annotation 도구나 타이밍 분석기가 이 플래그를 보고 내부를 블랙박스로 처리한다:

```verilog
`celldefine
module AND2X1 (output Y, input A, B);
    assign Y = A & B;
endmodule
`endcelldefine

`celldefine
module DFFX1 (output Q, input D, CK);
    always @(posedge CK) Q <= D;
endmodule
`endcelldefine
```

표준 셀 라이브러리 작성 시 모든 셀 모듈에 적용한다.

---

## `pragma — 툴 전용 힌트

표준은 `pragma 구문을 정의하지만 키워드 의미는 툴마다 다르다:

```verilog
// synthesis translate_off (Synopsys/Xilinx 관용)
initial $display("debug: x=%0h", x);
// synthesis translate_on

`pragma protect begin    // IP 암호화 시작 (Xilinx/Cadence)
// ... 암호화 대상 코드 ...
`pragma protect end
```

`pragma`는 표준화되지 않아 컴파일러 간 이식성이 없다. 조건부 컴파일이 목적이라면
`` `ifdef SYNTHESIS `` 패턴을 우선 사용한다.

> **vitamin 정책(수용-무시):** `` `pragma <한 줄> ``은 무진단으로 줄 전체를 소비하고 버린다
> (IEEE 1800 §22.11 — 의미 해석은 툴 재량). 어느 pragma 키워드도 동작에 영향을 주지 않는다.

---

## `line — 소스 위치 정보 삽입

코드 생성기나 전처리기가 에러 메시지에 원본 파일·라인 번호를 표시하도록 삽입한다:

```
`line <line_number> "<filename>" <level>
```

- `level = 0`: 일반 (현재 파일 내)
- `level = 1`: include 파일 진입
- `level = 2`: include 파일에서 복귀

```verilog
`line 42 "original_source.v" 0
// 이후 컴파일 에러는 "original_source.v:42"로 표시
```

직접 작성하는 RTL 코드에서는 거의 쓰지 않는다. 자동 생성 코드나 매크로 전개 툴의
출력에 삽입된다.

---

## 지시자 범위와 파일 패턴 요약

| 지시자 | 기본값 | 파일 경계 유지 | 권장 패턴 |
|--------|-------|--------------|----------|
| `` `define `` | 없음 | ❌ (파일 넘어 효력) | 헤더 파일 + include guard |
| `` `timescale `` | 없음 | ❌ | 각 파일 앞에 명시 + `resetall 패턴 |
| `` `default_nettype `` | `wire` | ❌ | `none` 선언 + 파일 끝에 `resetall |
| `` `celldefine `` | 비활성 | ❌ | 셀 라이브러리 파일 전체에 적용 |
| `` `begin_keywords `` | "1800-2017" | ✅ (`end_keywords` 구간) | 구버전 코드 구간만 한정 |
| `` `resetall `` | — | — | 파일 시작에 삽입 |

---

## Sources

- IEEE 1364-2001 §19 (compiler directives)
- IEEE 1800-2017 §22 (compiler directives)
- chipverify.com/verilog/verilog-compiler-directives (WebFetch 검증 ✓)
- chipverify.com/verilog/verilog-define-macros (WebFetch 검증 ✓, 매크로 인수 구문)
- hdlworks.com/hdl_corner/verilog_ref/items/CompilerDirectives.htm (WebFetch 검증 ✓, `resetall/`line/`celldefine)
- vlsiverify.com/verilog/compiler-directives/ (조건부 컴파일 예제)
- accellera.org P1800 keyword compatibility directive proposal (`begin_keywords 버전 문자열 교차 확인)
- front-end-verification.blogspot.com (default_nettype none 타이포 포착 동작 검증)
- analogcircuitdesign.com/verilog-compiler-directives/ (`timescale 범위 동작)
