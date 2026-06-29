# 05 · SystemVerilog 패키지

IEEE 1800-2017 §26 기준. 패키지는 타입·상수·함수·태스크·클래스를 컴파일 단위 밖에서
이름 공간과 함께 공유하는 구조체다. Verilog 시절의 `\`include` + 전역 선언 방식이 가진
이름 충돌과 이식성 문제를 해결한다.

---

## 패키지 선언과 본문 (§26.2)

```systemverilog
package bus_pkg;
    // 타입 정의
    typedef enum logic [1:0] {
        IDLE  = 2'b00,
        WRITE = 2'b01,
        READ  = 2'b10,
        RESP  = 2'b11
    } bus_state_e;

    // 파라미터
    parameter int DATA_W = 32;
    parameter int ADDR_W = 32;

    // 함수
    function automatic logic [DATA_W-1:0] swap32(
        input logic [DATA_W-1:0] d
    );
        return {d[7:0], d[15:8], d[23:16], d[31:24]};
    endfunction

    // 태스크
    task automatic print_state(input bus_state_e s);
        $display("state = %s", s.name());
    endtask
endpackage
```

패키지는 모듈·인터페이스·프로그램 블록에서 공유하는 타입과 서브루틴의 중앙 저장소다.
패키지 내 항목은 기본적으로 패키지 이름으로 한정된 스코프를 가진다.

---

## import — 명시적·와일드카드 (§26.3)

### 명시적 import (권장)

특정 항목만 현재 스코프로 가져온다. 어디서 왔는지 코드에서 명확히 드러난다.

```systemverilog
import bus_pkg::bus_state_e;  // 타입만
import bus_pkg::DATA_W;        // 파라미터만
import bus_pkg::swap32;        // 함수만
```

명시적 import 후에는 스코프 해석 연산자 없이 이름을 바로 쓸 수 있다:

```systemverilog
module foo
    import bus_pkg::bus_state_e;
    (
        input  logic         clk,
        input  bus_state_e   state_in,   // 패키지 타입을 포트 선언에 사용
        output bus_state_e   state_out
    );
    always_ff @(posedge clk)
        state_out <= state_in;
endmodule
```

모듈 선언에 `import bus_pkg::bus_state_e;`를 포함시키는 것은
IEEE 1800-2009 이후 허용된 문법이다. 포트 선언 이전에 import가 처리되므로
포트 타입에도 패키지 타입을 바로 쓸 수 있다.

### 와일드카드 import

패키지 전체를 한 번에 가져온다. 이름 충돌 위험이 있으므로 소규모 설계나 테스트벤치의
편의 목적에 적합하다.

```systemverilog
import bus_pkg::*;  // bus_pkg의 모든 공개 식별자
```

와일드카드 import는 이름을 **예약(reserve)**하지 않는다 — 로컬 선언이 같은 이름을
가지면 로컬이 이긴다(shadowing). 두 개의 와일드카드 import가 같은 이름을 가져오면
컴파일 에러(ambiguous identifier)가 발생한다.

```systemverilog
import pkg_a::*;  // pkg_a::foo 존재
import pkg_b::*;  // pkg_b::foo 존재
// foo 사용 시 컴파일 에러: ambiguous
```

---

## export (§26.5)

패키지가 다른 패키지에서 가져온 항목을 자신의 사용자에게 재노출할 때 쓴다.

```systemverilog
package low_pkg;
    typedef int my_type;
endpackage

package mid_pkg;
    import low_pkg::my_type;   // low_pkg에서 가져와
    export low_pkg::my_type;   // mid_pkg 사용자에게 재노출
    // 또는 전체 재노출: export low_pkg::*;
endpackage

// 사용 측: mid_pkg만 import해도 my_type 접근 가능
module foo;
    import mid_pkg::my_type;
    my_type x;
endmodule
```

`export` 없이 `import`만 하면 `import low_pkg::my_type`은 `mid_pkg` 내부에서만
보이고, `mid_pkg`를 import한 외부에서는 `my_type`이 보이지 않는다.

---

## $unit — 컴파일 단위 스코프와 그 위험성 (§3.12)

`$unit`은 어떤 모듈·패키지·인터페이스 선언보다 앞에 나오는 파일 최상위 스코프다.
Verilog의 전통적인 전역 선언 방식을 형식화한 개념이다.

```systemverilog
// 파일 최상위 — $unit 스코프 (패키지 선언보다 위)
typedef logic [7:0] byte_t;   // $unit에 선언
parameter int TOP_W = 8;

module foo;
    byte_t x;    // $unit의 byte_t 참조
    // ...
endmodule
```

### $unit의 위험성

**1. 툴 의존적 컴파일 단위 경계**

각 툴이 "컴파일 단위"를 어떻게 정의하는지가 다르다.
- Simulator A: 파일 하나 = 컴파일 단위 하나 → $unit 선언이 해당 파일에서만 보임
- Simulator B: 전체 파일을 한 컴파일 단위로 처리 → $unit 선언이 모든 파일에서 보임
- EDA 툴이 각 파일을 분리 컴파일하면 $unit 선언이 다른 파일에서 보이지 않음

**2. 파일 컴파일 순서 의존성**

$unit 선언이 어떤 파일에서 먼저 컴파일되는지에 따라 가시성이 달라진다.
빌드 스크립트의 파일 순서가 바뀌면 동작이 변한다.

**3. 검색 우선순위의 최하위**

식별자 검색 순서:
```
로컬 선언 (최우선)
  ↓
명시적 import (import pkg::item)
  ↓
와일드카드 import (import pkg::*)
  ↓
$unit 스코프 (최하위)
```

**권장**: `$unit`을 사용하지 않는다. 공유 선언이 필요하면 항상 패키지를 쓴다.

---

## import 우선순위 규칙

같은 이름이 여러 스코프에 있을 때 컴파일러가 어떤 것을 선택하는지:

```systemverilog
package pkg_a;
    parameter int X = 1;
endpackage

package pkg_b;
    parameter int X = 2;
endpackage

module test;
    import pkg_a::*;   // X = 1 (와일드카드)
    import pkg_b::X;   // X = 2 (명시적)

    int X = 99;        // 로컬 선언

    initial $display(X);  // 99 — 로컬 선언이 최우선
endmodule
```

우선순위 정리:

| 우선순위 | 항목 | 설명 |
|---------|------|------|
| 1 (최우선) | 로컬 선언 | 해당 모듈·함수 내에서 직접 선언한 것 |
| 2 | 명시적 import | `import pkg::item` 형태 |
| 3 | 와일드카드 import | `import pkg::*` 형태 |
| 4 (최하위) | $unit | 컴파일 단위 최상위 스코프 |

**와일드카드 충돌 규칙**: 두 와일드카드 import에서 같은 이름이 가져와졌을 때,
그 이름을 **실제로 사용하는 시점에** 컴파일 에러가 발생한다
(가져온 것 자체는 에러가 아님 — 사용 시 ambiguous 에러).

---

## 실전 패턴

### 공통 패키지 구조

```systemverilog
// types_pkg.sv — 기본 타입 정의
package types_pkg;
    typedef logic [31:0] word_t;
    typedef logic [7:0]  byte_t;
    typedef enum {RD, WR, NOP} op_e;
endpackage

// params_pkg.sv — 설계 파라미터
package params_pkg;
    parameter int CLK_PERIOD = 10;
    parameter int FIFO_DEPTH = 16;
endpackage

// utils_pkg.sv — 공통 함수
package utils_pkg;
    import types_pkg::word_t;
    export types_pkg::word_t;  // 사용자가 utils_pkg만 import해도 word_t 접근

    function automatic word_t byteswap(input word_t d);
        return {d[7:0], d[15:8], d[23:16], d[31:24]};
    endfunction
endpackage
```

---

## Sources

- IEEE 1800-2017 §26 (Packages), §3.12 (Compilation units), §26.3 (Import precedence)
- chipverify.com/systemverilog/systemverilog-package
- blogs.sw.siemens.com/verificationhorizons/2009/09/25/unit-vs-root/ (Dave Rich)
- blogs.sw.siemens.com/verificationhorizons/2010/07/13/package-import-versus-include/
- bradpierce.wordpress.com (package import vs $unit)
- dvcon-proceedings.org/using-systemverilog-packages-in-real-verification-proj.pdf
