# 04 · SystemVerilog 인터페이스

IEEE 1800-2017 §25 기준. 인터페이스는 모듈 간 신호 묶음을 하나의 개체로 캡슐화하고,
modport로 방향성을, clocking block으로 타이밍을 명시한다. 대규모 설계에서 포트 목록
반복을 없애고 검증 환경과의 타이밍 계약을 코드에 내재화한다.

---

## 기본 선언

인터페이스는 `interface` ... `endinterface` 블록으로 선언하며, 모듈처럼 포트 목록에
클럭 등 외부 신호를 받을 수 있다.

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

모듈 포트에서 인터페이스를 받을 때:

```systemverilog
module apb_slave (apb_if.Slave bus);  // modport 한정
    always_ff @(posedge bus.pclk) begin
        if (bus.psel && bus.penable && !bus.pwrite)
            bus.prdata <= mem[bus.paddr];
    end
endmodule
```

최상위 모듈에서 인스턴스화:

```systemverilog
module tb_top;
    logic clk;
    apb_if dut_bus(.pclk(clk));           // 인터페이스 인스턴스

    apb_slave slave(.bus(dut_bus.Slave)); // modport 뷰로 연결
endmodule
```

---

## 인터페이스 파라미터 (§25.3.3)

모듈 파라미터와 동일한 `#(parameter ...)` 문법을 사용한다.

```systemverilog
interface myBus #(
    parameter int D_WIDTH = 32,
    parameter int A_WIDTH = 32
) (input clk);
    logic [D_WIDTH-1:0] data;
    logic [A_WIDTH-1:0] addr;
    logic               valid;
endinterface
```

인스턴스화 시 재정의:

```systemverilog
myBus #(.D_WIDTH(64), .A_WIDTH(40)) wide_bus(.clk(clk));
```

---

## modport — 방향 뷰 (§25.5)

modport는 인터페이스 내 신호에 대한 방향 관점(view)을 정의한다.
같은 인터페이스를 마스터·슬레이브·테스트벤치 등 서로 다른 역할로 분리할 수 있다.

```systemverilog
interface bus_if (input clk);
    logic [7:0] data;
    logic       valid;
    logic       ready;

    modport Master (
        output data, valid,
        input  ready, clk
    );

    modport Slave (
        input  data, valid, clk,
        output ready
    );

    modport Monitor (
        input data, valid, ready, clk  // 읽기 전용 모니터
    );
endinterface
```

**방향 키워드 요약**:

| 키워드 | 의미 |
|--------|------|
| `input`  | 이 modport 사용자가 읽는 신호 |
| `output` | 이 modport 사용자가 구동하는 신호 |
| `inout`  | 양방향 (트라이스테이트 버스 등) |
| `import` | 인터페이스 내 태스크·함수를 이 modport에서 호출 가능하도록 노출 |
| `export` | 이 modport를 통해 접속한 모듈이 태스크·함수 본체를 구현함 |

### modport에 태스크·함수 import/export

인터페이스 내부에 태스크나 함수를 정의하고, modport를 통해 특정 뷰에서만 접근 가능하게
만들 수 있다.

```systemverilog
interface bus_if (input clk);
    logic [7:0] data;
    logic       valid;

    // 인터페이스 내 태스크 정의
    task automatic wait_valid();
        @(posedge clk);
        while (!valid) @(posedge clk);
    endtask

    modport TB (
        output data,
        input  valid, clk,
        import wait_valid   // 태스크를 이 modport에서 호출 가능
    );
endinterface

// 사용 측
module monitor(bus_if.TB bus);
    initial begin
        bus.wait_valid();   // modport import로 접근
        $display("data = %0h", bus.data);
    end
endmodule
```

패키지에서 가져온 함수도 modport에 import할 수 있다:

```systemverilog
package util_pkg;
    function automatic logic [7:0] flip8(input logic [7:0] d);
        return {d[0],d[1],d[2],d[3],d[4],d[5],d[6],d[7]};
    endfunction
endpackage

interface proc_if;
    import util_pkg::*;
    modport mp (import flip8);  // 패키지 함수를 modport로 노출
endinterface
```

`export`는 `import`의 반대 방향 — 접속한 모듈이 함수 본체를 제공한다
(추상 인터페이스 패턴, §25.9).

---

## clocking block (§14.12)

clocking block은 클럭 이벤트에 대한 신호의 **샘플 타이밍**(input skew)과
**구동 타이밍**(output skew)을 한 곳에 정의한다.
테스트벤치가 DUT와의 타이밍 레이스를 피하기 위해 사용한다.

### 선언

```systemverilog
clocking cb @(posedge clk);
    default input #1step output #1;  // input: 클럭 직전 샘플, output: 1ns 후 구동
    input  data, valid;
    output ready;
endclocking
```

`default input <skew> output <skew>`로 모든 신호에 적용할 기본 skew를 설정한다.
개별 신호에 다른 skew가 필요하면 각 라인에서 재정의할 수 있다.

### input skew vs output skew

| 항목 | 방향 | 의미 |
|------|------|------|
| `input  #T` | 클럭 **이전** T | 클럭 엣지 전 T 시간에 신호를 샘플 |
| `input  #1step` | 클럭 직전 | 클럭 엣지 직전 시뮬레이션 스텝에서 샘플 (권장 기본값) |
| `output #T` | 클럭 **이후** T | 클럭 엣지 후 T 시간에 신호를 구동 |
| `output #0` | 클럭과 동시 | Non-blocking 대입 정산 후 즉시 구동 |

`#1step`은 클럭 엣지의 직전 시뮬레이션 time step에서 샘플하므로 설정/홀드 타임
시뮬레이션에 적합하다.

### 인터페이스 내 clocking block + modport 조합

```systemverilog
interface apb_if (input pclk);
    logic [31:0] paddr;
    logic [31:0] pwdata;
    logic [31:0] prdata;
    logic        psel;
    logic        penable;
    logic        pwrite;

    clocking cb @(posedge pclk);
        default input #1step output #1;
        input  prdata;
        output paddr, pwdata, psel, penable, pwrite;
    endclocking

    modport TB  (clocking cb, input pclk);     // 테스트벤치: clocking block 통해 접근
    modport DUT (input  paddr, pwdata, psel,   // DUT: 직접 신호 접근
                         penable, pwrite,
                 output prdata);
endinterface
```

modport에 `clocking cb`를 포함하면, 그 modport를 사용하는 쪽은 클럭 블록을
통해서만 신호를 접근해야 하므로 타이밍 규율이 강제된다.

클럭 블록을 통한 신호 접근은 `vif.cb.signal` 또는 `vif.cb.signal <= value`
형태로 사용한다.

---

## 가상 인터페이스 (§25.9)

인터페이스 인스턴스는 모듈 계층에 정적으로 고정된다.
클래스 기반 테스트벤치(동적)에서 인터페이스 신호에 접근하려면
`virtual interface`(핸들)를 사용한다.

### 선언과 전달

```systemverilog
// 클래스 내 virtual interface 멤버
class Driver;
    virtual apb_if.TB vif;   // modport를 명시한 virtual interface 핸들

    function new(virtual apb_if.TB handle);
        vif = handle;        // 실제 인터페이스 인스턴스를 핸들에 연결
    endfunction

    task automatic write(logic [31:0] addr, data);
        @(vif.cb);           // clocking block 이벤트 대기
        vif.cb.paddr   <= addr;
        vif.cb.pwdata  <= data;
        vif.cb.pwrite  <= 1;
        vif.cb.psel    <= 1;
        vif.cb.penable <= 1;
        @(vif.cb);
        vif.cb.psel    <= 0;
        vif.cb.penable <= 0;
    endtask
endclass

// 최상위 테스트벤치 모듈
module tb_top;
    logic clk = 0;
    always #5 clk = ~clk;

    apb_if dut_bus(.pclk(clk));    // 정적 인터페이스 인스턴스

    Driver drv;
    initial begin
        drv = new(dut_bus.TB);     // virtual interface에 실제 인스턴스 바인딩
        drv.write(32'h1000, 32'hDEAD);
    end
endmodule
```

**동작 원리**:

1. `tb_top`에서 `apb_if dut_bus`가 정적으로 생성됨.
2. `new(dut_bus.TB)` — 실제 인스턴스의 TB modport 핸들을 Driver 생성자에 전달.
3. `vif`는 포인터처럼 `dut_bus`를 가리킴. 객체가 여러 개여도 같은 인터페이스를 공유 가능.
4. `vif.cb.paddr <= addr`처럼 clocking block을 통해 신호를 구동.

**모듈 계층이 깊은 경우**: `config_db`(UVM) 또는 최상위에서 핸들을 전달하는
계층적 패턴이 표준이다. `$cast`나 직접 포트 전달로 처리하기도 한다.

---

## Sources

- IEEE 1800-2017 §25 (Interfaces), §14.12 (Clocking blocks), §26.3 (Package import)
- chipverify.com/systemverilog/systemverilog-interface
- chipverify.com/systemverilog/systemverilog-modport
- vlsiverify.com/system-verilog/systemverilog-clocking-block/
- vlsiworlds.com/system-verilog/clocking-blocks-and-modports/
- verificationacademy.com/forums (modport import/export tasks, IEEE §26.3 인용)
- medium.com/@vimala.learnvlsi (virtual interface in class example)
