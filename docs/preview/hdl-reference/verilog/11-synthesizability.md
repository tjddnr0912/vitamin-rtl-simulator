# 11 · Verilog 합성 가능성

## 개요

본 프로젝트 Vitamin은 시뮬레이터다 — 합성은 비목표(본 spec §2.1). 그러나 참조 문서에
합성 가능 여부를 명기하는 이유는 두 가지다.

1. 사용자가 RTL 작성 시 합성 친화적 코드를 식별할 수 있게 돕는다.
2. Phase 1 MVP 구현 우선순위 기준으로 사용한다 — ✅ 항목이 Phase 1 대상, ❌ 항목은
   후속 단계.

범례: [../01-synthesizability-legend.md](../01-synthesizability-legend.md) — 세 도구
(Synopsys DC, Xilinx Vivado, Cadence Genus) 교차 기준.

---

## 카테고리별 매핑

### ✅ 합성 가능 (universal)

세 도구 공통으로 RTL 게이트 변환이 지원된다.

**모듈 / 구조**

- 모듈 선언, 포트, `parameter`, `localparam`
- 모듈 인스턴스화 (named / positional 포트 연결)
- `generate for` / `generate if` — 파라미터화된 구조 생성에 권장

**데이터 타입**

- `wire`, `reg` — 합성의 핵심 타입
- `reg [N:0]` 명시 비트폭 벡터, `signed` 수식어 포함
- `integer` — 32-bit signed, 합성 도구가 사용 범위에 맞춰 비트 트리밍
  (단, 명시적 `reg [N:0]` 선언이 더 안전 — ⚠️ 주의사항 참고)

**절차 블록 / 제어 흐름**

- `always @(posedge clk)` / `always @(posedge clk, posedge rst)` — 클록드 FF 추론
- `always @(*)` — 조합 논리 추론
- `if/else` — 우선순위 MUX 추론
- `case` / `casez` (with `?` wildcard only) — 합성 가능
- `for` loop (bounded) — 컴파일 타임 상수 경계 → unroll됨
- `assign` (연속 대입) — 조합 논리

**태스크 / 함수**

- `function` — 0-시간 제약, `input` 포트만 사용, 합성 가능
- `task` (static, 시간 소비 없음) — 합성 가능; `@`/`#`/`wait` 없는 형태

**게이트 프리미티브**

- 조합형 내장 게이트: `and`, `or`, `nand`, `nor`, `xor`, `xnor`, `buf`, `not`
- 삼상 버퍼(`bufif1`, `bufif0` 등) — FPGA top-level I/O에서 합성 가능

---

### ⚠️ 조건부 합성

도구별 지원 차이 또는 특정 형식에서만 합성 가능하거나, 합성은 되지만 사용을 권장하지
않는 구문이다.

**`initial` — FPGA OK / ASIC 주의**

FPGA(Vivado/Quartus/XST)는 `initial` 블록 내 대입을 파워업 초기값으로 변환한다.
Synopsys DC(ASIC)는 `VER-708: The construct 'declaration initial assignment' is not
supported in synthesis; it is ignored` 경고를 내고 무시한다.
Cadence Genus도 ASIC 플로우에서 일반적으로 지원하지 않는다.

```verilog
// FPGA: 합성 가능 — power-up 초기값으로 처리
// ASIC(DC/Genus): 경고 후 무시
initial begin
    state <= 3'd0;
    count <= 8'hFF;
end
```

**`casex` — 합성 가능하나 사용 금지 권장**

합성 도구는 처리하지만, `x`가 와일드카드로 동작해 시뮬레이션의 X-propagation과
다른 결과를 낼 수 있다. lowRISC 스타일 가이드는 명시적으로 금지하며,
`casez`(Verilog-2001) 또는 SV `case inside`를 대체로 권장한다.

**`integer` 타입 — 조건부**

합성 가능하지만 합성 도구가 사용 범위에 따라 32비트보다 좁게 트리밍한다.
RTL에서는 `reg [N:0]`으로 명시적 비트폭을 지정하는 것이 안전하다.

**`defparam` — deprecated, 사용 금지**

일부 합성 도구가 여전히 처리하지만 IEEE 1800-2012에서 deprecated 예고,
lowRISC 스타일 가이드에서 `Do not use defparam` 명시.
파라미터 override는 `#(...)` 인스턴스 파라미터 방식으로 대체한다.

```verilog
// ❌ defparam — 사용 금지
defparam u_adder.WIDTH = 8;

// ✅ 권장: 인스턴스 파라미터 override
adder #(.WIDTH(8)) u_adder (...);
```

**Tristate (`1'bz`) — FPGA top-level OK / on-chip 제한**

FPGA top-level I/O 핀의 삼상 버퍼는 합성 가능하다.
on-chip 내부 muxing 용도로 Z를 사용하는 것은 비권장 — lowRISC 금지 명시.

**조합형 UDP — 일부 도구 지원**

`primitive...endprimitive` 조합형 UDP는 일부 합성 도구가 지원하나 이식성이 낮다.
`assign` 또는 `always @(*)` 로 대체하는 것이 표준이다.

**`full_case` / `parallel_case` pragma — 사용 금지**

```verilog
// ❌ pragma — simulation/synthesis mismatch 위험
case (sel) // synthesis full_case parallel_case
```

합성 도구는 처리하지만, 시뮬레이션에서 latch를 추론하는 반면 합성은 조합 로직을
만드는 sim/synth mismatch가 발생한다. lowRISC: "Never use either pragma."
대체: SV `unique case` / `priority case`, 또는 모든 경로에 명시적 대입.

---

### ❌ 비합성

어떤 합성 도구도 게이트로 변환하지 않는다. 시뮬레이션/검증 전용.

| 구문 | 이유 |
|------|------|
| `real`, `time` 데이터 타입 | 부동소수점 / 64-bit 시뮬레이션 전용 |
| `#delay` in `always` | 합성 도구 무시 또는 에러 — "FPGA has no concept of time" |
| `$display`, `$monitor`, `$finish` 등 모든 system tasks | 시뮬레이션 I/O 전용 |
| `fork-join` | 병렬 실행 의미론 — 합성 불가; testbench 전용 |
| 순차형 UDP (edge-sensitive / level-sensitive latch UDP) | 합성 도구 미지원 |
| `force` / `release` | testbench 전용 |
| Recursive `function` / `task` | DC/Vivado 모두 재귀 합성 미지원; IEEE 1364.1-2002에도 없음 |
| `while` / `forever` loop (unbounded) | 컴파일 타임 종료 조건 없는 루프 |
| 게이트 지연 (`and #(2) g(...)`) | 타이밍 시뮬레이션 전용; 합성 무시 |
| Drive strength (`strong1`, `weak0` 등) | 시뮬레이션 전용; 합성 무시 또는 경고 |
| MOS 스위치 / 양방향 스위치 (`nmos`, `pmos`, `tran` 등) | 아날로그/스위치 레벨 시뮬레이션 전용 |

---

## 합성 친화적 RTL 패턴

**Latch 방지**: `case` / `if-else` 모든 경로에 명시적 대입. 비어 있는 경로가 있으면
latch 추론.

```verilog
// ❌ latch 추론: sel=2 경로 누락
always @(*) begin
    case (sel)
        2'd0: out = a;
        2'd1: out = b;
        // sel=2, sel=3 누락 → latch
    endcase
end

// ✅ default로 모든 경로 커버
always @(*) begin
    case (sel)
        2'd0: out = a;
        2'd1: out = b;
        default: out = '0;
    endcase
end
```

**클록 / 조합 분리**: 클록드 로직과 조합 로직을 별도 `always` 블록으로 분리.

```verilog
// ✅ 권장 패턴
always @(posedge clk or posedge rst) begin
    if (rst) q <= '0;
    else     q <= d_next;
end

always @(*) begin
    d_next = (en) ? data_in : q;
end
```

**Bounded for-loop**: 루프 경계를 `parameter` 또는 상수로 고정.

```verilog
parameter N = 8;
integer i;
always @(*) begin
    for (i = 0; i < N; i = i + 1)  // N이 상수 → unroll됨
        out[i] = in[N-1-i];
end
```

**Non-synthesizable 코드 격리**: system tasks나 initial 블록이 simulation 전용임을
명시할 때 `` `ifndef SYNTHESIS `` 를 사용한다.

```verilog
`ifndef SYNTHESIS
    initial $display("Testbench: reset applied");
`endif
```

---

## Sources

- IEEE 1364.1-2002 "IEEE Standard for Verilog Register Transfer Level Synthesis"
  (IEEE Xplore, withdrawn but remains the formal RTL synthesis subset definition)
- Vivado Design Suite User Guide Synthesis UG901 — Verilog Language Support
  (docs.amd.com/r/en-US/ug901-vivado-synthesis)
- Synopsys Design Compiler — VER-708 warning (initial block ignored in ASIC flow)
- lowRISC Verilog Coding Style Guide (github.com/lowRISC/style-guides) — defparam /
  casex / full_case / parallel_case 금지 근거 (WebFetch 검증 ✓)
- billauer.se/blog/2018/02/verilog-initial-xst-quartus-vivado/ — FPGA initial block
  처리 방식 (WebFetch 검증 ✓)
- asic-soc.blogspot.com/2013/06/synthesizable-and-non-synthesizable.html — 구문
  분류 테이블 (WebFetch 검증 ✓)
- ../01-synthesizability-legend.md (공통 범례, 도구 기준)
- ../research-log/verilog-synthesizability-2026-05-28.md (조사 전체 로그)
