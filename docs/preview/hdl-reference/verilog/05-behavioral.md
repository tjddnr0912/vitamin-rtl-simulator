# 05 · Verilog 동작 구문 (Behavioral)

IEEE 1364-2001/2005 기준. `initial`과 `always`는 Verilog의 두 가지 절차적 실행
컨텍스트다. 이 안에서 사용하는 대입 연산자의 선택 — blocking(`=`) vs
non-blocking(`<=`) — 이 합성 결과와 시뮬레이션 정확도를 결정한다.

---

## initial vs always

| 항목 | `initial` | `always` |
|------|----------|--------|
| 실행 횟수 | 시뮬레이션 t=0에 한 번, 자연 종료 | 영구 반복 (무한 루프) |
| 주요 용도 | 테스트벤치 초기화, 파형 생성 | RTL 논리 기술 |
| 합성 가능 여부 | ❌ 비합성 | ✅ (조건 충족 시) |
| 중복 존재 | 가능 — 여러 initial이 t=0에 병렬 시작 | 가능 — 여러 always가 병렬 실행 |

```verilog
// initial — 테스트벤치 초기화
initial begin
    clk = 0;
    rst = 1;
    #20 rst = 0;
    #200 $finish;
end

// always — 클록 생성
always #5 clk = ~clk;   // 10 time-unit 주기
```

---

## always 블록의 sensitivity list

sensitivity list는 `@(...)` 안에 나열한 신호 중 하나가 변화할 때 always 블록을
트리거한다.

### 레벨 감지 (조합 논리)

```verilog
// 명시적 목록 — Verilog-1995 스타일
always @(a or b or sel) begin
    y = sel ? a : b;
end

// 암시적 전체(@*) — Verilog-2001 권장
always @(*) begin
    y = sel ? a : b;    // a, b, sel 자동 추가
end
```

`@(*)` 는 블록 내에서 **읽히는** 모든 신호를 자동으로 sensitivity list에 포함한다.
명시적 목록에서 신호를 누락하면 시뮬레이션은 래치처럼 동작하지만 합성은 멀티플렉서로
만들어 **시뮬레이션/합성 불일치**가 발생한다.

### 에지 감지 (순서 논리)

```verilog
// 동기 리셋
always @(posedge clk) begin
    if (rst)
        q <= 0;
    else
        q <= d;
end

// 비동기 리셋 (negedge active-low)
always @(posedge clk or negedge rst_n) begin
    if (!rst_n)
        q <= 0;
    else
        q <= d;
end
```

| 키워드 | 트리거 조건 |
|--------|-----------|
| `posedge sig` | sig의 0→1 전환 (x/z→1, 0→x 포함) |
| `negedge sig` | sig의 1→0 전환 (x/z→0, 1→x 포함) |
| `sig` (에지 없음) | sig의 어떠한 값 변화도 |

---

## blocking(=) vs non-blocking(<=)

### blocking 대입 (=)

RHS를 평가한 즉시 LHS에 갱신하고 다음 문장으로 넘어간다.
시뮬레이션 스케줄러의 **Active region**에서 순서대로 실행된다.

```verilog
always @(*) begin
    // 조합 논리 — blocking 사용
    a = b & c;
    y = a | d;   // a는 이미 b&c로 갱신된 상태
end
```

### non-blocking 대입 (<=)

RHS를 **Active region**에서 샘플(평가)하고, LHS 갱신은 **NBA region**으로 예약한다.
NBA region에서 해당 time-step의 모든 non-blocking 갱신이 일괄 적용된다.

```verilog
always @(posedge clk) begin
    // 순서 논리 — non-blocking 사용
    b <= a;   // Active: a 샘플. NBA: b ← a_old
    c <= b;   // Active: b 샘플(아직 이전 값). NBA: c ← b_old
end
```

NBA region과 stratified event queue의 상세 동작은
[06-simulation-engine.md](../../../06-simulation-engine.md)를 참고한다.

### 정준 shift register 예시

```verilog
// ✅ 올바른 4비트 shift register — non-blocking
module shift4 (
    input        clk,
    input        d,
    output reg   q3
);
    reg q0, q1, q2;

    always @(posedge clk) begin
        q0 <= d;    // NBA: q0 ← d
        q1 <= q0;   // NBA: q1 ← q0(이전 값)
        q2 <= q1;   // NBA: q2 ← q1(이전 값)
        q3 <= q2;   // NBA: q3 ← q2(이전 값)
    end
    // 결과: 매 클럭마다 한 비트씩 우측 이동
endmodule
```

```verilog
// ❌ 잘못된 shift register — blocking
always @(posedge clk) begin
    q0 = d;     // 즉시 q0 = d
    q1 = q0;   // q0가 이미 d → q1도 d
    q2 = q1;   // 마찬가지로 d
    q3 = q2;   // 결국 q3 = d (shift 없음, 4개 모두 d)
end
```

### 대입 연산자 선택 규칙

| 상황 | 연산자 | 이유 |
|------|--------|------|
| 조합 논리 (`always @(*)`) | `=` blocking | 순서적 RHS 의존 관계 표현 |
| 순서 논리 (`always @(posedge clk)`) | `<=` non-blocking | NBA region 분리로 플립플롭 동작 보장 |
| 같은 always 내 혼용 | ❌ 금지 | 시뮬레이션/합성 불일치 원인 |
| testbench 자극 | `=` blocking | 결정론적 자극 순서 보장 |

---

## 주요 주의사항

**always 안에서 대입되는 변수는 reg여야 한다.** (SystemVerilog의 `logic`과 달리
Verilog `wire`는 continuous assign으로만 구동한다.)

```verilog
wire  y_wire;
reg   y_reg;

assign y_wire = a & b;       // ✅ wire → continuous assign
always @(*) y_reg = a & b;  // ✅ reg → always 내 blocking
```

**initial 블록에서의 non-blocking**: 문법적으로 허용되지만 NBA region 스케줄링이
발생하므로 테스트벤치 자극에는 blocking을 쓰는 것이 더 예측 가능하다.

---

## Sources

- IEEE 1364-2001 §9.7–§9.8 (procedural blocks), §9.10 (sensitivity list)
- IEEE 1800-2017 §9.2 (initial/always), §9.4 (event control), §10.4 (blocking), §10.4.2 (non-blocking)
- chipverify.com/verilog/verilog-always-block (sensitivity list — WebFetch 검증)
- chipverify.com/verilog/verilog-blocking-non-blocking-statements (NBA 메커니즘 — WebFetch 검증)
- analogcircuitdesign.com/verilog-blocking-and-non-blocking/ (timing semantics)
- eclipse.umbc.edu/robucci/cmpe316/lectures/L04__VerilogIntroII/ (initial vs always)
- 06-simulation-engine.md (NBA region 상세, stratified event queue)
