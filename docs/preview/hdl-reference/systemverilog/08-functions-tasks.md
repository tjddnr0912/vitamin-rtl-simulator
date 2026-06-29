# 08 · SystemVerilog 함수·태스크 확장

IEEE 1800-2017 §13 기준. 이 문서는 Verilog-2005 대비 SystemVerilog가 추가한
함수·태스크 기능을 다룬다. Verilog 기초는 `../verilog/06-tasks-functions.md` 참조.

---

## Verilog 대비 주요 추가 사항

| 기능 | Verilog-2005 | SystemVerilog |
|------|-------------|---------------|
| void 반환 함수 | ❌ (반환값 필수) | ✅ `function void f(...)` |
| ref 인자 | ❌ | ✅ `ref type var` |
| const ref 인자 | ❌ | ✅ `const ref type var` |
| `let` 선언 | ❌ | ✅ compile-time inline |
| `return;` (void) | ❌ | ✅ |
| automatic 기본 | ❌ (static 기본) | class/program에서 automatic 기본 |
| ANSI 포트 스타일 | ❌ | ✅ |

---

## let 선언 — compile-time inline (§11.13)

`let`은 모듈·패키지·블록 내부에서 표현식에 이름을 부여하는 **컴파일 타임 인라인 치환**이다.
`define` 매크로와 달리 **로컬 스코프**를 가지며 타입 안전성이 보장된다.

```systemverilog
// 기본 형태: 인자 없음
let max_addr = (1 << ADDR_W) - 1;

// 인자 있는 형태
let compare(a, b) = (a == b) ? "Pass" : "Fail";
let in_range(x, lo, hi) = (x >= lo) && (x <= hi);

// 사용
$display("max_addr = %0h", max_addr);
$display("%s", compare(exp_data, act_data));
assert (in_range(addr, BASE_ADDR, BASE_ADDR + SIZE - 1));
```

### let vs `define 비교

| 특성 | `let` | `` `define `` |
|------|-------|--------------|
| 스코프 | 선언된 블록/모듈/패키지 내 로컬 | 파일 전체(전역) |
| 타입 체크 | ✅ (인자 타입 추론) | ❌ |
| 다중 선언 | 스코프별로 동일 이름 가능 | 마지막 선언이 덮어씀 |
| 사용 목적 | 표현식 재사용, SVA 보조 | 텍스트 치환 |

assertion 내부에서 자주 쓰인다:

```systemverilog
// SVA 보조로 let 활용
let addr_aligned = (addr[1:0] == 2'b00);
assert property (@(posedge clk) wr_en |-> addr_aligned);
```

---

## void Function (§13.4)

반환값이 없는 함수. `return;`으로 조기 종료 가능.
Verilog에서는 함수가 반드시 하나의 값을 반환해야 했으나 SV에서 side-effect 전용
함수를 선언할 수 있다.

```systemverilog
function void print_state(input logic [1:0] state);
    case (state)
        2'b00: $display("IDLE");
        2'b01: $display("BUSY");
        2'b10: $display("DONE");
        default: begin
            $warning("Unknown state: %02b", state);
            return;    // void function의 조기 종료
        end
    endcase
endfunction
```

void function 호출 시 반환값을 받지 않아도 된다:

```systemverilog
print_state(curr_state);   // 반환값 무시 — SV에서는 합법적
```

> **Verilog와의 차이**: Verilog 함수는 반드시 반환값이 있고, 호출 결과를 반드시 사용해야 한다. SV의 non-void function을 void처럼 호출하려면 `void'(func_call)` cast를 사용한다.

```systemverilog
void'(some_func_with_return());   // 반환값 명시적 무시
```

---

## ref 인자 — 참조 전달 (§13.5.2)

`ref`로 선언된 인자는 원본 변수의 참조를 전달한다. 함수/태스크 내에서의 변경이
호출자에 즉시 반영된다.

```systemverilog
// ref로 swap 구현
task automatic swap(ref logic [7:0] a, ref logic [7:0] b);
    logic [7:0] tmp;
    tmp = a;
    a   = b;
    b   = tmp;
endtask

// 호출
logic [7:0] x = 8'hAA, y = 8'h55;
swap(x, y);   // x = 0x55, y = 0xAA
```

### const ref — 읽기 전용 참조

`const ref`는 참조를 전달하되 내부에서 변경을 금지한다. 대형 배열이나 구조체를
값 복사(by value) 없이 전달할 때 **성능 최적화**에 사용한다.

```systemverilog
function automatic logic [31:0] calc_checksum(
    const ref logic [7:0] data [],   // 동적 배열을 복사 없이 전달
    input int size
);
    logic [31:0] sum = 0;
    for (int i = 0; i < size; i++)
        sum += data[i];
    return sum;
endfunction
```

### ref 인자 제약

- `ref` 인자는 **`automatic` 서브루틴에서만 사용** 가능하다.
- `static` lifetime 서브루틴에서 ref 인자 사용 시 컴파일 에러.

```systemverilog
function static void bad_ref(ref int x);   // ❌ 컴파일 에러
    x = 0;
endfunction

function automatic void ok_ref(ref int x);  // ✅
    x = 0;
endfunction
```

---

## automatic vs static — lifetime 맥락 (§13.4.2)

Verilog에서 모든 서브루틴은 기본적으로 `static` lifetime이다.
SV는 컨텍스트에 따라 기본값이 다르다.

| 컨텍스트 | 기본 lifetime |
|----------|--------------|
| `module` 내 task/function | **static** (Verilog 호환) |
| `class` 메서드 | **automatic** (§8.6) |
| `program` 블록 내 | **automatic** |
| `package` 함수/태스크 | static (명시 권장) |
| 명시 선언 | `automatic`/`static` 키워드로 재정의 |

```systemverilog
// module 내부 — static이 기본값이므로 automatic 명시 필요
module my_mod;
    task automatic reentrant_task(input int n);
        // 재귀 가능, 각 호출마다 독립 스택 프레임
        if (n > 0) reentrant_task(n - 1);
    endtask
endmodule

// package 내 — automatic 명시 권장
package util_pkg;
    function automatic int abs_val(input int x);
        return (x >= 0) ? x : -x;
    endfunction
endpackage
```

### automatic이 필요한 경우

1. **재귀(recursive) 호출** — static에서는 지역 변수 공유로 오동작
2. **ref 인자 사용** — static에서는 컴파일 에러
3. **병렬 태스크 인스턴스** — fork-join 내에서 태스크를 여러 번 동시 실행
4. **클래스 메서드** — 이미 automatic (별도 명시 불필요)

---

## 함수/태스크 선언 스타일 — ANSI 포트 (§13.4~13.5)

SV는 Verilog 전통 방식 외에 더 간결한 ANSI C 스타일 선언을 지원한다.

```systemverilog
// ANSI 스타일 (권장)
function automatic logic [31:0] adder(
    input  logic [31:0] a,
    input  logic [31:0] b,
    output logic        carry
);
    {carry, adder} = {1'b0, a} + {1'b0, b};
endfunction

// 태스크 ANSI 스타일
task automatic wait_for_ack(
    input  logic       clk,
    input  logic       req,
    output logic       ack,
    input  int         timeout_cycles = 100
);
    int cnt = 0;
    while (!ack) begin
        @(posedge clk);
        if (++cnt >= timeout_cycles) begin
            $error("wait_for_ack timeout");
            return;
        end
    end
endtask
```

---

## 관련 문서

- `../verilog/06-tasks-functions.md` — Verilog 함수/태스크 기초
- `../system-tasks/` — SV system functions: `$urandom`, `$bits`, `$cast`, `$past`
- `../system-tasks/11-assertion-sampling.md` — `$past`, `$rose`, `$fell`, `$stable`, `$sampled` 상세
- [07-assertions-sva.md](07-assertions-sva.md) — `let`의 SVA 활용 패턴
- [05-packages.md](05-packages.md) — package 내 function automatic 패턴
- [06-classes-oop.md](06-classes-oop.md) — 클래스 메서드 (automatic 기본)

---

## Sources

- IEEE 1800-2017 §13 (Tasks and functions), §11.13 (let expression)
- asic4u.wordpress.com/2015/12/26/the-let-construct/ — let 선언 (WebFetch ✓)
- chipverify.com/systemverilog/systemverilog-functions — ref/void/automatic (WebFetch ✓ 부분)
- fpgatutorial.com/systemverilog-functions/ — ANSI 스타일, automatic 컨텍스트
