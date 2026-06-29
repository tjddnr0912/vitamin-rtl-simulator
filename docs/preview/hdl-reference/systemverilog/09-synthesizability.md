# 09 · SystemVerilog 합성 가능성

IEEE 1800-2017 + Vivado UG901 + Synopsys DC 기준.
Verilog 합성 가능성은 `../verilog/11-synthesizability.md` 참조.

> **범례**:
> ✅ 합성 가능 — Vivado/DC/Genus 공통 지원
> ⚠️ 조건부 — 도구·형식 제약 있음
> ❌ 비합성 — 시뮬레이션·검증 전용

---

## ✅ 합성 가능 (Universal RTL Subset)

| 구문 | 비고 |
|------|------|
| `logic` | `reg`/`wire` 통합 타입. RTL에서 `reg` 대신 사용 권장 |
| `bit` | 2-state 단일 비트. `logic` (4-state)과 달리 X/Z 없음 — 도구 호환성 확인 필요 |
| `byte` / `shortint` / `int` / `longint` | 2-state 정수. signed. 비트 폭 명시적; RTL에서 적극 사용 가능 |
| `logic [N-1:0]` 벡터 | 기본 RTL 타입 |
| `enum` | `enum logic [1:0] {IDLE, BUSY, DONE}` — FSM 상태 인코딩에 활용 |
| `typedef` | 타입 별칭. 패키지와 결합해 재사용 |
| `struct` (packed) | `typedef struct packed { logic [7:0] data; logic valid; } pkt_t;` |
| `always_comb` | 조합 논리. sensitivity list 자동 추론 + 누락 신호 경고 |
| `always_ff` | 플립플롭. 엣지 이벤트 하나만 허용 |
| `always_latch` | 래치. `always_comb`보다 사용 자제 권장 |
| packed multi-dim array | `logic [3:0][7:0] mat;` — 2D/3D packed array |
| `interface`/`modport` | RTL 포트 묶음. 합성 가능 (virtual interface 제외) |
| `package`/`import` | 타입·상수·함수 공유. 합성 가능 |
| `generate`/`genvar` | 파라미터화된 인스턴스 생성 |
| `for` loop (bounded) | 컴파일 타임 상수 경계 — unroll됨 |
| `parameter` / `localparam` | 합성 상수 |

### always_comb / always_ff 사용 패턴

```systemverilog
// 조합 논리 — always_comb 권장
always_comb begin
    unique case (state)
        IDLE:  next_state = req ? BUSY : IDLE;
        BUSY:  next_state = done ? DONE : BUSY;
        DONE:  next_state = IDLE;
        default: next_state = IDLE;
    endcase
end

// 플립플롭 — always_ff 권장
always_ff @(posedge clk or negedge rst_n) begin
    if (!rst_n) state <= IDLE;
    else        state <= next_state;
end
```

### enum + typedef + struct (packed) 패턴

```systemverilog
package bus_pkg;
    typedef enum logic [1:0] {
        IDLE  = 2'b00,
        WRITE = 2'b01,
        READ  = 2'b10,
        ERR   = 2'b11
    } bus_state_e;

    typedef struct packed {
        logic [31:0] addr;
        logic [31:0] data;
        logic        write;
        logic        valid;
    } bus_txn_t;
endpackage

module my_ctrl
    import bus_pkg::*;
(
    input  bus_txn_t txn,
    output bus_state_e state
);
    // ...
endmodule
```

---

## ⚠️ 조건부 합성 (도구·형식 제약)

| 구문 | 조건 | 비고 |
|------|------|------|
| `foreach` | 경계가 정적(컴파일 타임 상수)인 경우 ✅ | 동적 크기 배열 순회는 ❌ |
| `unique case` / `priority case` | ✅ 합성 힌트로 처리 | 시뮬레이션 경고 기능(assertion)은 비합성 |
| `unique if` / `priority if` | ✅ 합성 힌트 | 동일 |
| 2-state types (`bit`, `int`) | ✅ 대부분 도구 지원. X/Z 없음에 주의 | 4-state `logic` 대비 이식성 확인 필요 |
| `struct` (unpacked) | ⚠️ 도구 의존. 일부 합성 도구 미지원 | packed 사용 권장 |
| `union` (packed) | ✅ 일부 도구 지원 | unpacked union은 비합성 |
| `initial` 블록 | **FPGA**: Vivado/Quartus → 파워업 초기값 ✅ | **ASIC**(DC): 무시 ❌ |
| 동적 배열 크기 (parameter-driven) | ✅ 파라미터로 고정 크기 결정 시 | 런타임 `new[]` 는 비합성 |
| interface (RTL 제한) | ✅ modport + 신호만 | `virtual interface`, task/function 포함 복잡 interface는 ⚠️ |
| typedef forward reference | ⚠️ 일부 도구 미지원 | package 내 완전 선언 권장 |
| `$bits()` / `$size()` / `$clog2()` | ✅ 합성 가능 system function | |
| recursive function | ❌ DC/Vivado 모두 불지원 | 루프로 변환 |

### foreach 합성 예시

```systemverilog
// ✅ 합성 가능 — 정적 크기 packed array
logic [3:0][7:0] vec;
always_comb begin
    foreach (vec[i])     // i: 0~3, 컴파일 타임 결정
        vec[i] = 8'hFF;
end

// ❌ 비합성 — 동적 배열
logic [7:0] dyn [];     // 런타임 크기
```

### unique/priority 합성 동작

```systemverilog
// unique case: 합성 도구에 "항상 하나의 case만 매칭됨" 힌트
// → 디코더 최적화에 활용, priority MUX 불필요
always_comb begin
    unique case (sel)   // 합성: 병렬 디코더 생성 가능
        2'b00: out = a;
        2'b01: out = b;
        2'b10: out = c;
        2'b11: out = d;
    endcase
end
```

---

## ❌ 비합성 (시뮬레이션·검증 전용)

| 구문 | 비고 |
|------|------|
| `class` + 모든 OOP | `extends`, `virtual`, `super`, `new`, 소멸자. 합성 도구 에러 |
| `rand` / `randc` / `constraint` | 랜덤화 전용. 합성 도구 완전 거부 |
| 동적 메모리 (`new[]`, `delete`) | 런타임 메모리 할당. 합성 불가 |
| `queue` (`$q`) — 일반 용도 | RTL 큐는 파라미터화된 배열/FIFO IP로 구현 |
| `string` — 복잡 연산 | `$sformat`, `str.len()`, `str.substr()` 등 비합성. 상수 문자열은 시뮬레이션 전용 |
| `chandle` | C DPI 포인터. 합성 불가 |
| `real` / `shortreal` | 부동소수점. 합성 도구 미지원 |
| `virtual interface` | 클래스 기반 검증 브리지. 합성 불가 |
| `program` 블록 | 검증 전용. 합성 에러 |
| immediate assertion (`assert`, `assume`, `cover`) | 합성 도구가 무시 또는 경고 |
| concurrent assertion (`assert property`) | 합성 도구가 무시 또는 경고 |
| `fork-join` | 병렬 실행 의미론. 합성 불가 |
| `$display` / `$monitor` / `$finish` 등 simulation tasks | 합성 도구 무시 |
| DPI-C import (일부) | `context` import는 비합성; `pure`/`DPI-C`는 도구 의존 |
| `mailbox` / `semaphore` / `event` (process sync) | 프로세스 동기화 객체. 합성 불가 |

### class + assertions → RTL 경계 명확화 패턴

```systemverilog
// ❌ RTL 파일에 class 혼입 금지
// ✅ 검증 전용 파일에 분리

// rtl/my_ctrl.sv — 합성 가능 RTL
module my_ctrl (input clk, input logic req, output logic ack);
    always_ff @(posedge clk) ack <= req;
endmodule

// tb/my_ctrl_tb.sv — 비합성 검증 전용
module my_ctrl_tb;
    // assertions (비합성), class (비합성) 여기에만
    assert property (@(posedge clk) req |-> ack);

    class Checker;
        // ...
    endclass
endmodule
```

---

## 합성 가능성 요약 — 주요 SV 확장

| SV 구문 | ✅/⚠️/❌ | Verilog 동등 대안 |
|---------|---------|----------------|
| `logic` | ✅ | `reg`/`wire` |
| `always_comb` | ✅ | `always @(*)` |
| `always_ff` | ✅ | `always @(posedge clk)` |
| `enum` | ✅ | `parameter` + integer |
| `struct` packed | ✅ | packed vector |
| `struct` unpacked | ⚠️ | separate signals |
| `interface`/`modport` | ✅ | port list |
| `package`/`import` | ✅ | `include` + global |
| `unique/priority case` | ⚠️ (hint) | `full_case`/`parallel_case` pragma (금지) |
| `foreach` (정적) | ✅ | `for (genvar i=...)` |
| `class` | ❌ | — |
| `rand`/constraint | ❌ | — |
| `virtual interface` | ❌ | — |
| `queue` | ❌ | FIFO IP / 파라미터화 배열 |
| `dynamic array` (new[]) | ❌ | 파라미터화 고정 배열 |
| assertions (any) | ❌ | — |
| `program` | ❌ | — |
| `string` ops | ❌ | — |

---

## 도구별 차이점

| 항목 | Vivado (FPGA) | Synopsys DC (ASIC) |
|------|-------------|-------------------|
| `initial` 블록 | ✅ 파워업 초기값으로 변환 | ❌ VER-708 경고 후 무시 |
| `interface` | ✅ modport 지원 | ✅ (일부 제약) |
| `unique/priority` | ✅ 최적화 힌트 | ✅ |
| `assertion` | ❌ 무시 | ❌ 무시 |
| `class` | ❌ 에러 | ❌ 에러 |
| packed struct | ✅ | ✅ |
| unpacked struct | ⚠️ (도구 버전 의존) | ⚠️ |

---

## 관련 문서

- `../verilog/11-synthesizability.md` — Verilog-2005 합성 가능성 (supertype 관계)
- [01-data-types.md](01-data-types.md) — logic/bit/int/enum/struct 타입 상세
- [03-procedural.md](03-procedural.md) — always_comb/_ff/_latch 의미론
- [04-interfaces.md](04-interfaces.md) — interface/modport RTL 사용 패턴
- [06-classes-oop.md](06-classes-oop.md) — class OOP (❌ 비합성 전용)
- [07-assertions-sva.md](07-assertions-sva.md) — assertions (❌ 비합성 전용)

---

## Sources

- IEEE 1800-2017 §13/§16 + 학습 데이터 기반
- vlsi.pro/sva-sequences-repetition-operators/ (WebFetch ✓)
- chipverify.com/systemverilog/systemverilog-quick-refresher — 합성 가능 구문 (WebSearch)
- chipverify.com/systemverilog/systemverilog-always — always_comb/_ff 합성 (WebSearch)
- systemverilog.dev/2.html, systemverilog.dev/3.html — RTL 모델링 패턴 (WebSearch)
- Vivado UG901 HTML — "SystemVerilog Support" 섹션 존재 확인 (목차 수준; 세부 PDF 접근 불가)
- Synopsys DC VER-708 warning — initial block ASIC 처리 (이전 조사 carryover; copyprogramming.com WebFetch ✓)
