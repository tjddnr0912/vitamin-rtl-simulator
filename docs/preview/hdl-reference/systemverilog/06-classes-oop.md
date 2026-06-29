# 06 · SystemVerilog 클래스와 OOP

IEEE 1800-2017 §8 (클래스), §18 (랜덤화) 기준.

> **비합성 전용**: 이 문서에서 다루는 모든 기능 — 클래스, 상속, 가상 메서드,
> 파라미터화 클래스, static 멤버, 랜덤화 — 은 **100% 비합성(non-synthesizable)**이다.
> 시뮬레이션·검증·UVM 테스트벤치 전용. RTL 설계 코드에 클래스를 포함하면
> 합성 툴이 에러를 발생시킨다.

---

## 클래스 기본 선언 (§8.3)

```systemverilog
class Packet;
    // 인스턴스 변수 (멤버)
    bit [31:0] addr;
    bit [31:0] data;
    int        id;

    // 생성자 — 반환 타입 없음, 비블로킹
    function new(int init_id = 0);
        addr = 32'hDEAD_BEEF;
        data = 32'h0;
        id   = init_id;
    endfunction

    // 메서드
    function void display();
        $display("[Packet] id=%0d addr=%08h data=%08h", id, addr, data);
    endfunction
endclass
```

객체 생성과 사용:

```systemverilog
Packet pkt;          // 핸들 선언 — 초기값 null
pkt = new(42);       // 힙에 객체 할당, id=42
pkt.display();

Packet pkt2 = new;   // 선언과 동시에 생성, 기본 인자 사용
```

핸들은 포인터와 유사하다. `pkt = null`로 핸들을 해제하면 가비지 컬렉터가
참조 없는 객체를 회수한다.

---

## 생성자 new() (§8.7)

`new()`는 클래스의 유일한 생성자다. 반환 타입이 없고 오버로드가 불가능하다.
인자를 받을 수 있으며, 기본값을 지정해 선택적으로 만들 수 있다.

```systemverilog
class Config;
    int width;
    int depth;

    function new(int w = 8, int d = 16);
        width = w;
        depth = d;
    endfunction
endclass

Config c1 = new;          // width=8, depth=16 (기본값)
Config c2 = new(32);      // width=32, depth=16
Config c3 = new(64, 128); // width=64, depth=128
```

### this 키워드 (§8.11)

`this`는 현재 객체 인스턴스를 참조한다. 생성자 인자와 클래스 멤버 이름이 겹칠 때 구별한다.

```systemverilog
class Packet;
    int id;
    function new(int id);
        this.id = id;   // this.id = 클래스 멤버, id = 인자
    endfunction
endclass
```

---

## 상속 — extends / super.new() (§8.13)

```systemverilog
class BasePacket;
    int size;
    logic [7:0] kind;

    function new(int s = 64, logic [7:0] k = 8'h01);
        size = s;
        kind = k;
    endfunction

    function void show();
        $display("BasePacket: size=%0d kind=%02h", size, kind);
    endfunction
endclass

class ExtPacket extends BasePacket;
    logic [31:0] payload;

    function new(int s = 64, logic [31:0] p = 0);
        super.new(s);       // 부모 생성자 명시 호출 — 첫 번째 실행 가능 구문이어야 함
        payload = p;
    endfunction

    function void show();
        super.show();       // 부모 메서드 호출
        $display("ExtPacket: payload=%08h", payload);
    endfunction
endclass
```

**super.new() 규칙**:
- 자식 생성자의 **첫 번째 실행 가능 구문**이어야 한다.
- 부모 생성자가 인자를 요구하면 `super.new(args)` 형태로 전달해야 한다.
- 자식 생성자에 `super.new()`가 없으면 컴파일러가 `super.new()`를 첫 줄에 자동 삽입한다
  (부모 생성자가 인자 없이도 호출 가능한 경우에만).

---

## 다형성 — virtual 메서드 (§8.20)

`virtual` 없이 재정의하면 **정적 디스패치** — 핸들의 선언 타입을 기준으로 호출된다.
`virtual`을 붙이면 **동적 디스패치** — 실제 객체 타입을 기준으로 호출된다.

```systemverilog
class Shape;
    virtual function real area();
        return 0.0;
    endfunction
    virtual function void print();
        $display("Shape area = %0.2f", area());
    endfunction
endclass

class Circle extends Shape;
    real radius;
    function new(real r); radius = r; endfunction
    function real area();
        return 3.14159 * radius * radius;   // virtual 재정의
    endfunction
endclass

class Rect extends Shape;
    real w, h;
    function new(real w, real h); this.w = w; this.h = h; endfunction
    function real area();
        return w * h;                        // virtual 재정의
    endfunction
endclass

// 다형성 사용
Shape s;
Circle c = new(5.0);
Rect   r = new(4.0, 3.0);

s = c;  s.print();   // "Shape area = 78.54" — Circle::area() 호출
s = r;  s.print();   // "Shape area = 12.00" — Rect::area()   호출
```

**가상 메서드 규칙**:
- 부모에서 `virtual`로 선언하면 자식에서 재정의 시 `virtual` 키워드는 선택 사항이다
  (상속됨).
- `virtual` 없는 메서드를 자식에서 재정의하면 부모 핸들로 호출 시 부모 메서드가 실행된다.

---

## 파라미터화 클래스 (§8.25)

정수 파라미터 또는 타입 파라미터로 제네릭 클래스를 정의한다.

### 정수 파라미터

```systemverilog
class FIFO #(int DEPTH = 8, int WIDTH = 8);
    logic [WIDTH-1:0] mem [DEPTH];
    int               head = 0;
    int               tail = 0;
    int               count = 0;

    function void push(logic [WIDTH-1:0] val);
        if (count < DEPTH) begin
            mem[tail] = val;
            tail = (tail + 1) % DEPTH;
            count++;
        end
    endfunction

    function logic [WIDTH-1:0] pop();
        if (count > 0) begin
            pop = mem[head];
            head = (head + 1) % DEPTH;
            count--;
        end
    endfunction
endclass

// 인스턴스화
FIFO #(16, 32) deep_fifo;
FIFO #(.DEPTH(4)) nibble_fifo;   // WIDTH = 8 기본값 유지
```

### 타입 파라미터

```systemverilog
class Stack #(type T = int);
    T    items [];
    int  sp = 0;

    function void push(T val);
        items = new[sp + 1](items);
        items[sp++] = val;
    endfunction

    function T pop();
        return items[--sp];
    endfunction
endclass

// 다양한 타입으로 인스턴스화
Stack #(bit [3:0]) nibble_stack = new;
Stack #(real)      float_stack  = new;
Stack              int_stack    = new;   // T = int (기본값)

// typedef로 별칭 생성
typedef Stack #(logic [7:0]) ByteStack;
ByteStack bstack = new;
```

---

## static 멤버 (§8.9)

`static` 변수는 클래스의 모든 인스턴스가 공유한다.
`static` 메서드는 인스턴스 없이 클래스 이름으로 직접 호출 가능하다.

```systemverilog
class Packet;
    static int obj_count = 0;   // 모든 인스턴스 공유
    int id;

    function new();
        obj_count++;
        id = obj_count;
    endfunction

    // static 메서드 — non-static 멤버(id) 접근 불가
    static function int get_count();
        return obj_count;
        // id = 1;  // 컴파일 에러: non-static 멤버 접근 불가
    endfunction

    function void display();
        $display("id=%0d / total=%0d", id, obj_count);
    endfunction
endclass

// 사용
Packet p1 = new;
Packet p2 = new;
Packet p3 = new;

$display("total = %0d", Packet::get_count());  // 3
p1.display();  // id=1 / total=3
```

---

## 스코프 해석 연산자 :: (§23.8)

`::`는 클래스 외부에서 클래스 스코프 내 항목을 참조하거나,
파라미터화 클래스의 특수화된 스코프에 접근할 때 사용한다.

```systemverilog
// static 메서드 호출
Packet::get_count();

// static 변수 직접 접근
$display("count = %0d", Packet::obj_count);

// 파라미터화 클래스: 특정 특수화(specialization)를 명시해야 함
FIFO #(16, 32)::DEPTH;        // ✓ 특수화된 클래스 스코프로 접근
// FIFO::DEPTH;               // ✗ 비특수화 스코프 — 컴파일 경고 또는 에러

// 자식 클래스에서 부모 메서드 호출
// (super는 내부에서 부모 스코프를 가리키는 :: 변형)
super.show();
```

---

## 랜덤화 — rand / randc / constraint (§18)

> **비합성**: 랜덤화 관련 기능 전체가 합성 불가. 시뮬레이션 전용.

### rand와 randc

클래스 멤버를 `rand` 또는 `randc`로 선언하면 `randomize()` 호출 시 자동 랜덤화된다.

```systemverilog
class Transaction;
    rand  bit [7:0]  opcode;   // 매 호출마다 균등(uniform) 랜덤
    randc bit [2:0]  tag;      // 0~7 사이클을 순환, 반복 전에 모두 방문
    rand  int        length;
    rand  logic [31:0] addr;
endclass
```

| 선언 | 동작 |
|------|------|
| `rand`  | 매 `randomize()` 호출마다 독립적으로 균등 분포 랜덤값 |
| `randc` | 가능한 모든 값을 순환한 후에야 동일 값 반복 (카드 덱처럼) |
| (없음)  | `randomize()` 대상에서 제외 |

### constraint 블록 (§18.5)

제약 블록은 클래스 내에서 선언하며, 랜덤 변수가 만족해야 할 조건을 정의한다.
여러 제약 블록은 암묵적으로 AND 관계다.

```systemverilog
class Packet;
    rand bit [7:0]  opcode;
    rand bit [31:0] addr;
    rand int        length;

    constraint opcode_c {
        opcode inside {8'h01, 8'h02, 8'h04, 8'h08};  // 특정 값 집합
    }

    constraint addr_c {
        addr[1:0] == 2'b00;     // 4바이트 정렬
        addr inside {[32'h1000 : 32'hFFFF]};  // 범위 제한
    }

    constraint length_c {
        length > 0;
        length <= 256;
        length % 4 == 0;        // 4의 배수
    }

    // 상관 제약
    constraint correlated_c {
        if (opcode == 8'h04)    // opcode가 0x04이면
            length == 64;       //  length는 반드시 64
    }
endclass
```

### randomize() 메서드

```systemverilog
Packet pkt = new;

// 기본 호출 — 반환값 반드시 확인
if (!pkt.randomize())
    $fatal(1, "Randomization failed!");

// assert 형태 (권장)
assert(pkt.randomize()) else $fatal(1, "Randomization failed!");
```

### 인라인 제약 with (§18.7)

일회성 제약을 `randomize()` 호출 시 추가한다. 클래스 내 기존 제약과 AND 관계.

```systemverilog
// addr을 특정 값으로 고정하는 일회성 제약
assert(pkt.randomize() with { addr == 32'hA000; });

// 복잡한 인라인 제약
assert(pkt.randomize() with {
    opcode == 8'h01;
    length inside {[64:128]};
});
```

### pre_randomize / post_randomize (§18.8)

`randomize()` 전후에 자동 호출되는 훅 메서드.
`pre_randomize()`에서 제약 설정, `post_randomize()`에서 파생 필드 계산에 사용한다.

```systemverilog
class Packet;
    rand  bit [31:0] addr;
    rand  int        length;
    bit   [31:0]     end_addr;   // 파생 필드 (랜덤화 대상 아님)

    function void pre_randomize();
        // randomize() 직전 — 제약 활성화/비활성화 등
        $display("before randomize");
    endfunction

    function void post_randomize();
        // randomize() 직후 — 파생 필드 계산
        end_addr = addr + length - 1;
    endfunction
endclass
```

### 제약 상속과 재정의 (§18.9)

자식 클래스는 부모의 제약을 상속받는다.
같은 이름의 제약 블록으로 재정의(override)할 수 있다.

```systemverilog
class ExtPacket extends Packet;
    // 부모의 addr_c 제약을 더 넓은 범위로 재정의
    constraint addr_c {
        addr inside {[32'h0 : 32'hFFFF_FFFF]};  // 전체 범위 허용
    }
endclass
```

제약 블록 비활성화:

```systemverilog
pkt.opcode_c.constraint_mode(0);  // 특정 제약 비활성화
pkt.randomize();
pkt.opcode_c.constraint_mode(1);  // 다시 활성화
```

---

## 클래스 기반 검증 패턴

UVM 스타일의 기본 구조:

```systemverilog
// 트랜잭션 클래스
class APBTxn;
    rand bit [31:0] addr;
    rand bit [31:0] data;
    rand bit        write;

    constraint addr_aligned { addr[1:0] == 2'b00; }
endclass

// 드라이버 클래스 (virtual interface 사용 — 04-interfaces.md 참고)
class APBDriver;
    virtual apb_if.TB vif;

    function new(virtual apb_if.TB handle);
        vif = handle;
    endfunction

    task automatic drive(APBTxn txn);
        @(vif.cb);
        vif.cb.paddr   <= txn.addr;
        vif.cb.pwdata  <= txn.data;
        vif.cb.pwrite  <= txn.write;
        vif.cb.psel    <= 1;
        vif.cb.penable <= 1;
        @(vif.cb);
        vif.cb.psel    <= 0;
        vif.cb.penable <= 0;
    endtask
endclass

// 테스트 시퀀스
initial begin
    APBTxn txn;
    APBDriver drv = new(dut_bus.TB);

    repeat (10) begin
        txn = new;
        assert(txn.randomize()) else $fatal(1, "rand failed");
        drv.drive(txn);
    end
end
```

---

## Sources

- IEEE 1800-2017 §8 (Classes), §18 (Constrained random value generation), §23.8 (Scope resolution)
- chipverify.com/systemverilog/systemverilog-class-constructor
- chipverify.com/systemverilog/systemverilog-constraints
- chipverify.com/systemverilog/systemverilog-randomization-methods
- chipverify.com/systemverilog/systemverilog-static-variables-functions
- chipverify.com/systemverilog/systemverilog-parameterized-classes
- verificationguide.com/systemverilog/systemverilog-constraints/
- dvcon-proceedings.org/the-top-most-common-systemverilog-constrained-random-gotchas.pdf
