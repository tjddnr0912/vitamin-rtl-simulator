# SystemVerilog 인터페이스·패키지·클래스 OOP 조사
**대상**: IEEE 1800-2017 §25 (인터페이스) / §26 (패키지·$unit) / §8 (클래스) / §18 (랜덤화)
**날짜**: 2026-05-28
**조사 방식**: WebSearch 3라운드 + WebFetch 8회 1차 검증

---

## 조사 동기

SystemVerilog는 Verilog의 하드웨어 모델링 능력에 세 가지 큰 검증·구조화 확장을 추가했다.
(A) **인터페이스**: 모듈 간 신호 묶음을 하나의 개체로 정의하고, modport로 방향성을,
clocking block으로 타이밍을 명시한다.
(B) **패키지**: 타입·상수·함수를 컴파일 단위 밖에서 공유하는 네임스페이스.
$unit 컴파일 단위 스코프의 한계와 비교된다.
(C) **클래스 OOP + 랜덤화**: 동적 객체, 상속, 가상 메서드, 파라미터화, 제약 랜덤화.
100% 비합성(시뮬레이션·검증 전용).
(D) **가상 인터페이스**: 정적 인터페이스 인스턴스와 동적 클래스 검증 환경을 잇는 핸들.

---

## (A) 인터페이스

### 선언 구조

IEEE 1800-2017 §25.3. 인터페이스는 모듈처럼 독립 컴파일 단위로 선언된다.
포트 목록에 클럭 등 전역 신호를 받을 수 있고, 내부에 신호·파라미터·태스크·함수·
modport·clocking block을 포함할 수 있다.

```systemverilog
interface apb_if (input pclk);
    logic [31:0] paddr;
    logic [31:0] pwdata;
    logic [31:0] prdata;
    logic        penable;
    logic        pwrite;
    logic        psel;
endinterface
```

**파라미터화 인터페이스** (§25.3.3):

```systemverilog
interface myBus #(parameter D_WIDTH = 32) (input clk);
    logic [D_WIDTH-1:0] data;
    logic               enable;
endinterface
```

파라미터는 모듈 인스턴스화와 동일하게 `#(.D_WIDTH(16))` 형태로 재정의된다.

### modport (§25.5)

modport는 인터페이스 내부에서 특정 연결 방향 뷰를 선언한다.
모듈이 인터페이스 포트를 modport로 받으면 컴파일러가 방향 검사를 수행한다.

**방향 키워드**:

| 키워드 | 의미 |
|--------|------|
| `input` | 해당 모듈이 읽는 신호 |
| `output` | 해당 모듈이 구동하는 신호 |
| `inout` | 양방향 (트라이스테이트 버스 등) |
| `import` | 인터페이스 내 태스크·함수를 이 modport에서 호출 가능하도록 가져옴 |
| `export` | 이 modport를 통해 접속한 모듈이 태스크·함수 본체를 구현함 |

```systemverilog
interface bus_if (input clk);
    logic [7:0] data;
    logic       valid;
    logic       ready;

    task automatic write(input logic [7:0] d);
        data  = d;
        valid = 1;
        @(posedge clk);
        valid = 0;
    endtask

    modport Master (output data, valid,
                    input  ready, clk,
                    import write);   // write 태스크 호출 가능

    modport Slave  (input  data, valid, clk,
                    output ready,
                    export write);   // Slave 모듈이 write 본체를 제공
endinterface
```

`import`는 인터페이스 안에 이미 정의된 태스크/함수를 그 modport를 통해 사용 가능하게
노출한다. `export`는 반대로 접속한 모듈 쪽이 함수 본체를 구현하도록 선언한다.

패키지에서 가져온 함수도 modport에 import할 수 있다:

```systemverilog
package math_pkg;
    function automatic int add(int a, b); return a + b; endfunction
endpackage

interface calc_if;
    import math_pkg::*;
    modport mp (import add);   // 패키지 함수를 modport로 노출
endinterface
```

### clocking block (§14.12)

클럭 블록은 특정 클록 이벤트에 대한 신호의 샘플링(input skew)과 구동(output skew)
타이밍을 한 곳에 명시한다. 테스트벤치가 DUT와의 타이밍 레이스 컨디션을 피하기 위해
사용한다.

```systemverilog
interface apb_if (input pclk);
    logic [31:0] paddr;
    logic [31:0] prdata;
    logic        psel;
    logic        penable;
    logic        pwrite;

    clocking cb @(posedge pclk);
        default input #1step output #1;  // input: 클럭 직전 1step 시간에 샘플
                                          // output: 클럭 후 1ns에 구동
        input  prdata;
        output paddr, psel, penable, pwrite;
    endclocking

    modport TB  (clocking cb, input pclk);
    modport DUT (input paddr, psel, penable, pwrite,
                 output prdata);
endinterface
```

**skew 의미론**:

| 항목 | 방향 | 의미 |
|------|------|------|
| `input  #T` | 클럭 **이전** T만큼 | 클록 엣지 전 T 시간에 샘플. 기본 `#1step` (직전 delta) |
| `output #T` | 클럭 **이후** T만큼 | 클록 엣지 후 T 시간에 값을 구동 |

`#1step`은 클럭 엣지의 직전 시뮬레이션 time step에서 샘플하므로 조합 논리의
글리치를 피할 수 있다.

modport에 clocking block을 참조(`clocking cb`)하면, 해당 modport를 사용하는
모듈은 클럭 블록을 통해서만 신호에 접근해야 하므로 타이밍 규율이 강제된다.

### 가상 인터페이스 (§25.9)

클래스는 동적(dynamic)이고, 인터페이스 인스턴스는 정적(static, 모듈 계층에 고정)이다.
`virtual interface`는 인터페이스 인스턴스를 가리키는 핸들로, 클래스 안에서 인터페이스를
참조할 수 있게 한다. 자세한 내용은 (D) 절 참고.

---

## (B) 패키지

### 선언과 본문 (§26.2)

패키지는 타입 정의·파라미터·열거형·함수·태스크·클래스 등을 컴파일 단위 밖에서
이름 공간과 함께 공유하는 구조체다.

```systemverilog
package bus_pkg;
    typedef enum logic [1:0] {
        IDLE  = 2'b00,
        WRITE = 2'b01,
        READ  = 2'b10
    } bus_state_e;

    parameter int DATA_W = 32;

    function automatic logic [DATA_W-1:0] swap32(input logic [DATA_W-1:0] d);
        return {d[7:0], d[15:8], d[23:16], d[31:24]};
    endfunction
endpackage
```

### import — 명시적·와일드카드 (§26.3)

**명시적 import** (권장): 특정 항목만 현재 스코프로 가져온다.

```systemverilog
import bus_pkg::bus_state_e;  // 타입 하나만
import bus_pkg::DATA_W;       // 파라미터 하나만
```

**와일드카드 import**: 패키지 전체를 한 번에 가져온다. 이름 충돌 위험이 있으므로
소규모 설계나 테스트벤치에서 편의 목적으로 사용한다.

```systemverilog
import bus_pkg::*;            // bus_pkg의 모든 공개 식별자
```

모듈 선언에 import를 포함시키면 포트 선언에서도 패키지 타입을 쓸 수 있다
(IEEE 1800-2009 이후 허용):

```systemverilog
module my_dut
    import bus_pkg::bus_state_e;
    (
        input  logic         clk,
        input  bus_state_e   state_in,
        output bus_state_e   state_out
    );
endmodule
```

### export (§26.5)

패키지가 다른 패키지에서 가져온 항목을 자신의 사용자에게 재노출할 때 쓴다.

```systemverilog
package top_pkg;
    import low_pkg::my_type;    // low_pkg에서 가져와
    export low_pkg::my_type;    // top_pkg 사용자에게 재노출
    // 또는: export low_pkg::*;
endpackage
```

`export` 없이 `import`만 하면 top_pkg를 import하는 쪽에서는 low_pkg의 항목이
보이지 않는다. 레이어링된 패키지 구조에서 필요하다.

### $unit과 그 위험성 (§3.12)

`$unit`은 어떤 모듈·패키지·인터페이스 선언보다 앞에 나오는 파일 최상위 스코프다.
형식상 "컴파일 단위 스코프(compilation unit scope)"라 불린다.

```systemverilog
// 파일 최상위 — $unit 스코프
typedef logic [7:0] byte_t;   // 모든 모듈에서 보임(같은 컴파일 단위에서)

module foo;
    byte_t x;  // $unit에 있는 byte_t 사용
endmodule
```

**$unit의 위험성**:

1. **툴 의존성**: 컴파일 단위 경계를 어디서 나누는지는 툴마다 다르다.
   Simulator A는 파일 하나를 한 컴파일 단위로 처리하고,
   Simulator B는 전체 파일을 하나로 묶을 수 있다.
   $unit에 선언한 항목이 다른 툴에서는 보이지 않을 수 있다.
2. **재현성 부재**: 파일 컴파일 순서에 따라 $unit에 있는 항목의 가시성이 달라진다.
3. **검색 우선순위의 바닥**: 식별자 검색 순서는
   `로컬 선언 > 명시적 import > 와일드카드 import > $unit`으로,
   $unit은 가장 낮다.

**결론**: $unit을 사용하지 않고 패키지로 대체한다. IEEE 1800-2017 §3.12에서도
툴 포터빌리티를 위해 패키지 사용을 권장한다.

### import 우선순위 요약

```
로컬 선언 (가장 높음)
  ↓
명시적 import (import pkg::item)
  ↓
와일드카드 import (import pkg::*)
  ↓
$unit 스코프 (가장 낮음)
```

같은 이름이 여러 단계에 있으면 상위 단계가 이긴다.
와일드카드 import끼리 이름이 충돌하면 컴파일 에러(ambiguous) 발생.

---

## (C) 클래스 OOP

> **비합성 주의**: SystemVerilog 클래스(§8), 랜덤화(§18), 관련 모든 OOP 기능은
> 100% 비합성(non-synthesizable)이다. 시뮬레이션·검증·UVM 테스트벤치 전용.
> RTL 설계 코드에 클래스를 넣으면 합성 툴이 에러를 낸다.

### 기본 클래스 선언과 생성자 (§8.3, §8.7)

```systemverilog
class Packet;
    bit [31:0] addr;
    bit [31:0] data;
    int        id;

    // 생성자: 반환 타입 없음, 비블로킹
    function new(int init_id = 0);
        addr = 32'hDEAD_BEEF;
        data = 32'h0;
        id   = init_id;
    endfunction

    function void display();
        $display("id=%0d addr=%08h data=%08h", id, addr, data);
    endfunction
endclass
```

객체 생성:

```systemverilog
Packet pkt;         // 핸들 선언 (null)
pkt = new(42);      // 힙에 객체 할당, id=42로 초기화
pkt.display();
```

### this 키워드 (§8.11)

`this`는 현재 객체 인스턴스를 가리킨다. 생성자 인자 이름이 클래스 멤버와 같을 때 구별에 쓴다.

```systemverilog
class Packet;
    int id;
    function new(int id);
        this.id = id;   // this.id = 클래스 멤버, id = 인자
    endfunction
endclass
```

### 상속 — extends / super.new() (§8.13)

```systemverilog
class BasePacket;
    int size;
    function new(int s = 64);
        size = s;
    endfunction
    function void show();
        $display("BasePacket size=%0d", size);
    endfunction
endclass

class ExtPacket extends BasePacket;
    logic [7:0] tag;
    function new(int s = 64, logic [7:0] t = 8'hFF);
        super.new(s);   // 부모 생성자 명시 호출 (필수, 첫 줄)
        tag = t;
    endfunction
    function void show();
        super.show();
        $display("ExtPacket tag=%02h", tag);
    endfunction
endclass
```

`super.new()`는 자식 생성자의 **첫 번째 실행 가능 구문**이어야 한다.
부모 생성자가 인자를 요구하면 반드시 `super.new(args)`로 전달해야 한다.

### 다형성 — virtual 메서드 (§8.20)

`virtual` 키워드 없이 재정의하면 정적 디스패치(핸들 타입 기준)가 적용된다.
`virtual`을 붙이면 동적 디스패치(실제 객체 타입 기준)가 적용된다.

```systemverilog
class Animal;
    virtual function void speak();
        $display("...");
    endfunction
endclass

class Dog extends Animal;
    function void speak();          // virtual 재정의
        $display("Woof!");
    endfunction
endclass

class Cat extends Animal;
    function void speak();
        $display("Meow!");
    endfunction
endclass

// 다형성 사용
Animal a;
Dog d = new;
Cat c = new;

a = d; a.speak();  // "Woof!" — 실제 타입(Dog) 기준 호출
a = c; a.speak();  // "Meow!" — 실제 타입(Cat) 기준 호출
```

### 파라미터화 클래스 (§8.25)

정수 파라미터 또는 타입 파라미터로 제네릭 클래스를 만든다.

```systemverilog
// 정수 파라미터
class Queue #(int DEPTH = 8);
    bit [7:0] mem [DEPTH];
    int       head, tail;
    // ...
endclass

// 타입 파라미터
class Stack #(type T = int);
    T items [];
    function void push(T val);
        items = new[items.size() + 1](items);
        items[items.size()-1] = val;
    endfunction
endclass

// 인스턴스화
Queue #(16) q16;
Stack #(bit [3:0]) nibble_stack;
Stack #(real)      float_stack;

// typedef로 별칭 생성
typedef Stack #(int) IntStack;
```

### static 멤버 (§8.9)

static 변수는 클래스의 모든 인스턴스가 공유한다.
static 메서드는 인스턴스 없이도 호출 가능하지만 non-static 멤버에 접근할 수 없다.

```systemverilog
class Packet;
    static int obj_count = 0;   // 모든 인스턴스 공유
    int id;

    function new();
        obj_count++;
        id = obj_count;
    endfunction

    static function int get_count();
        return obj_count;
        // id = 1; // 컴파일 에러: non-static 멤버 접근 불가
    endfunction
endclass

// 인스턴스 없이도 호출 가능
$display("count = %0d", Packet::get_count());
```

### 스코프 해석 연산자 :: (§8.23)

`::`는 클래스 외부에서 클래스 스코프 내 식별자를 참조할 때 쓴다.

```systemverilog
Packet::get_count();           // static 메서드
$display(Packet::obj_count);   // static 변수

// 파라미터화 클래스: 특정 특수화 명시 필요
Stack #(int)::count_all();     // ✓ 특수화된 스코프로 접근
// Stack::count_all();         // ✗ 비특수화 스코프 — 컴파일 경고/에러
```

자식 클래스 생성자에서 `super.new()`도 `super`가 부모 클래스 스코프를 가리키는
`::`의 맥락이다.

---

## (D) 가상 인터페이스

### 문제 배경

인터페이스 인스턴스는 모듈 계층(정적)에 고정되어 있다.
클래스는 동적으로 생성·소멸된다.
클래스가 직접 인터페이스 인스턴스를 멤버로 가질 수 없다 — 타입이 다르다.

### virtual interface 선언과 사용 (§25.9)

```systemverilog
interface dut_if (input clk);
    logic [7:0] data;
    logic       valid;
    logic       ready;

    clocking cb @(posedge clk);
        default input #1step output #1;
        input  data, valid;
        output ready;
    endclocking

    modport TB (clocking cb, input clk);
endinterface

// 클래스에서 virtual interface 사용
class Driver;
    virtual dut_if.TB vif;     // modport를 명시한 virtual interface 핸들

    function new(virtual dut_if.TB if_handle);
        vif = if_handle;
    endfunction

    task automatic drive(logic [7:0] d);
        @(vif.cb);             // clocking block 이벤트 대기
        vif.cb.ready <= 1;     // clocking block을 통해 구동
    endtask
endclass

// 최상위 테스트벤치 모듈
module tb_top;
    logic clk;
    dut_if dif(.clk(clk));     // 실제 인터페이스 인스턴스

    Driver drv;
    initial begin
        drv = new(dif.TB);     // 인터페이스 인스턴스를 virtual interface로 전달
        drv.drive(8'hAA);
    end
endmodule
```

**동작 원리**:

1. `tb_top` 모듈에서 `dut_if dif(...)` 인터페이스 인스턴스를 생성(정적).
2. `new(dif.TB)`로 실제 인스턴스의 TB modport를 Driver 생성자에 전달.
3. Driver 내부의 `vif`는 핸들(포인터)로 `dif`를 가리킨다.
4. 이후 `vif.cb.ready`처럼 클럭 블록을 통해 신호에 접근.

virtual interface는 합성 불가(테스트벤치/검증 전용)이며,
UVM에서는 `uvm_config_db`를 통해 계층적으로 전달하는 것이 표준 패턴이다.

---

## Sources

- chipverify.com/systemverilog/systemverilog-interface ✓ (WebFetch 검증)
- chipverify.com/systemverilog/systemverilog-modport
- vlsiverify.com/system-verilog/systemverilog-clocking-block/ ✓ (WebFetch 검증)
- chipverify.com/systemverilog/systemverilog-package ✓ (WebFetch 검증)
- blogs.sw.siemens.com/verificationhorizons/2009/09/25/unit-vs-root/ ✓ (WebFetch 검증)
- bradpierce.wordpress.com/2016/02/28/dont-use-unit (WebFetch 검증)
- chipverify.com/systemverilog/systemverilog-class-constructor ✓ (WebFetch 검증)
- chipverify.com/systemverilog/systemverilog-constraints ✓ (WebFetch 검증)
- chipverify.com/systemverilog/systemverilog-static-variables-functions ✓ (WebFetch 검증)
- chipverify.com/systemverilog/systemverilog-parameterized-classes ✓ (WebFetch 검증)
- medium.com/@vimala.learnvlsi (virtual interface in class) ✓ (WebFetch 검증)
- verificationacademy.com (modport import/export tasks, IEEE 1800 §26.3 인용)
- IEEE 1800-2017 §8, §14.12, §18, §25, §26
