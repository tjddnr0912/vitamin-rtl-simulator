# 06 · Verilog 절차 제어문 (Procedural Statements)

IEEE 1364-2001/2005 기준. `initial` / `always` 블록 안에서 실행 흐름을 제어하는
구문들을 다룬다: 조건문, case, 루프, fork-join, 시간 제어(delay/event).

---

## if / else

```verilog
// 기본 if-else
always @(*) begin
    if (sel == 2'b00)
        y = a;
    else if (sel == 2'b01)
        y = b;
    else if (sel == 2'b10)
        y = c;
    else
        y = d;
end
```

**우선순위 인코더 의미**: `else if` 체인은 상단 조건이 하단보다 높은 우선순위를 갖는다.
의도적 우선순위 인코더라면 `if-else if`로 명시하고, 병렬 선택이라면 `case`를 쓴다.

**래치 방지**: 조합 always에서 `else` 없는 `if`나 모든 조건을 커버하지 못하는 분기는
합성 도구가 래치를 생성한다.

```verilog
// ❌ 래치 생성 — else 없음
always @(*) begin
    if (en) q = d;   // en=0일 때 q의 처리 없음 → 래치
end

// ✅ 래치 없음 — else 추가
always @(*) begin
    if (en) q = d;
    else    q = q_default;
end
```

---

## case / casez / casex

### case

4-state 완전 비교(===와 동일). x/z 값도 정확히 비교한다.

```verilog
always @(*) begin
    case (opcode)
        2'b00: result = a + b;
        2'b01: result = a - b;
        2'b10: result = a & b;
        2'b11: result = a | b;
        default: result = '0;
    endcase
end
```

### casez

case item의 `z` 또는 `?`를 don't-care로 처리한다. data(case expression) 측의
x/z는 그대로 비교에 참여한다.

```verilog
// 우선순위 인코더 — casez + ? 패턴
always @(*) begin
    casez (req)          // req[3:0]
        4'b1???: grant = 4'b1000;   // bit3 우선
        4'b01??: grant = 4'b0100;
        4'b001?: grant = 4'b0010;
        4'b0001: grant = 4'b0001;
        default: grant = 4'b0000;
    endcase
end
```

### casex ⚠️ — RTL 사용 금지

casex는 case item **과** case expression 양쪽의 x/z/? 를 don't-care로 처리한다.
시뮬레이션에서 전파된 X 값이 의도치 않은 분기를 활성화할 수 있다.

```verilog
// ❌ casex 위험 예시
reg [1:0] sig = 2'bxx;  // 초기화 전 값이 x

always @(*) begin
    casex (sig)
        2'b1?: y = a;   // x가 x와 don't-care 매칭 → 이 분기 활성화 가능
        2'b0?: y = b;
        default: y = c;
    endcase
end
// 시뮬레이션에서는 y=a, 합성 후 하드웨어는 y=c → 불일치
```

**권장**: don't-care가 필요하면 `casez`와 `?`를 사용한다. casex는 코드베이스에서
`casez`로 교체하는 것이 원칙이다.

### case 비교 요약

| 구문 | case item의 z/? | case item의 x | expression의 x/z |
|------|----------------|--------------|-----------------|
| `case` | 그대로 비교 | 그대로 비교 | 그대로 비교 |
| `casez` | don't-care | 그대로 비교 | 그대로 비교 |
| `casex` | don't-care | don't-care | don't-care ⚠️ |

---

## full_case / parallel_case pragma ⚠️

### 정의

- `full case`: case expression의 모든 가능한 값이 case item 중 하나에 매칭됨
  → 합성 시 default 래치 없음
- `parallel case`: case item 중 동시에 두 개 이상이 매칭되는 조합 없음
  → 합성 시 병렬 로직(우선순위 없음)으로 구현

### 함정 — 시뮬레이션 완전 무시

```verilog
// ❌ pragma 사용 — 위험
always @(*) begin
    case (state) // synthesis full_case parallel_case
        2'b00: next = 2'b01;
        2'b01: next = 2'b10;
        2'b10: next = 2'b00;
        // 2'b11 없음 — pragma로 합성은 래치 없이 처리
        // 시뮬레이터는 이 주석을 무시 → 2'b11 입력 시 next 유지(래치 동작)
    endcase
end
```

시뮬레이터는 `// synthesis ...` 를 **일반 주석**으로 처리한다. 합성 도구만 해석한다.
이로 인해 pre-synthesis 시뮬레이션과 post-synthesis 게이트 네트리스트가 다르게 동작하는
**시뮬레이션/합성 불일치**가 발생한다.

### 올바른 대안

```verilog
// ✅ full case: default 추가
always @(*) begin
    case (state)
        2'b00: next = 2'b01;
        2'b01: next = 2'b10;
        2'b10: next = 2'b00;
        default: next = 2'b00;   // 시뮬레이터도 처리
    endcase
end

// ✅ parallel case 의도라면: if-else 대신 case 사용하되
//    두 item이 동시에 true인 조합이 없도록 설계 자체에서 보장
```

**결론**: `full_case` / `parallel_case` pragma는 Cliff Cummings(SNUG 1999)가
"Verilog 합성의 Evil Twins"로 명명한 안티패턴이다. 신규 코드에서 사용을 금지한다.

---

## 루프

### for

Verilog `for`는 정적 반복 — elaboration time에 loop body를 전개한다.

```verilog
integer i;
always @(*) begin
    for (i = 0; i < 8; i = i + 1) begin
        if (data[i])
            count = count + 1;
    end
end
```

### while

조건이 참인 동안 반복. 합성 가능하려면 루프 횟수가 정적으로 결정되어야 한다.

```verilog
integer cnt;
always @(*) begin
    cnt = 0;
    while (cnt < 8) begin
        result[cnt] = data[cnt] ^ key[cnt];
        cnt = cnt + 1;
    end
end
```

### repeat

지정 횟수만큼 반복.

```verilog
// 테스트벤치: 8개 클록 펄스 대기
initial begin
    repeat (8) @(posedge clk);
    $display("8 clocks elapsed");
end
```

### forever

무한 반복. `always`를 쓸 수 없는 맥락(initial, task 내부)에서 사용한다.
**반드시 delay 또는 event control을 포함**해야 한다 — 없으면 시뮬레이션이 멈춘다.

```verilog
initial begin
    forever begin
        @(posedge clk);
        // 매 클럭 상승 에지에서 체크
        if (dut_error) $error("DUT error detected at %0t", $time);
    end
end
```

---

## disable

`disable`은 명명된 블록 또는 태스크를 조기 종료한다. C의 `break` / `return`에 해당한다.

```verilog
// 첫 번째 1 비트 위치 탐색
integer idx;
always @(*) begin : search_loop
    idx = -1;
    for (i = 0; i < 8; i = i + 1) begin
        if (data[i]) begin
            idx = i;
            disable search_loop;   // 찾으면 루프 탈출
        end
    end
end
```

태스크 이름으로 `disable task_name;`을 호출하면 해당 태스크를 즉시 종료한다.

---

## fork-join

`fork-join`은 여러 문장을 **병렬**로 실행한다. initial/always 블록 안에서 사용한다.

### Verilog: fork-join (모두 완료 대기)

IEEE 1364에서 제공하는 유일한 형태. 부모 프로세스는 모든 자식 스레드가 완료될 때까지 블록된다.

```verilog
initial begin
    fork
        #10 a = 1;   // 스레드 1: t=10에 a=1
        #20 b = 1;   // 스레드 2: t=20에 b=1
        #30 c = 1;   // 스레드 3: t=30에 c=1
    join
    // t=30에 모두 완료 후 여기 도달
    $display("all done at t=%0t", $time);
end
```

### SystemVerilog 확장: fork-join_any / fork-join_none

IEEE 1364 (Verilog)에는 없고 IEEE 1800 (SystemVerilog)에서 추가된다.
SV 프론트엔드를 사용하는 testbench에서 활용 가능하다.

```verilog
// fork-join_any: 첫 번째 스레드 완료 시 main thread 재개
//                나머지 스레드는 백그라운드에서 계속 실행
initial begin
    fork
        #10 a = 1;
        #20 b = 1;
        #30 c = 1;
    join_any
    // t=10에 도달 (a=1만 완료) — b, c 스레드는 계속 실행 중
end

// fork-join_none: spawn 즉시 main thread 재개 (fire-and-forget)
initial begin
    fork
        monitor_bus();    // 백그라운드로 계속 실행
        drive_stimulus(); // 백그라운드로 계속 실행
    join_none
    // 즉시 여기 도달 — 두 스레드는 백그라운드에서 실행 중
end
```

### disable fork

현재 scope에서 spawn된 모든 활성 스레드를 즉시 종료한다.

```verilog
initial begin
    fork
        #100 a = 1;
        #200 b = 1;
    join_any
    // 첫 번째 완료 후 나머지 스레드 강제 종료
    disable fork;
end
```

### fork-join 종류 비교

| 구문 | 표준 | 대기 조건 | 나머지 스레드 |
|------|------|---------|------------|
| `fork...join` | IEEE 1364 | 모두 완료 | — |
| `fork...join_any` | IEEE 1800 (SV) | 첫 번째 완료 | 백그라운드 계속 |
| `fork...join_none` | IEEE 1800 (SV) | 없음 (즉시) | 백그라운드 계속 |

---

## 시간 제어 (Delay / Event Control)

### delay 제어 (#)

```verilog
#10 a = 1;          // 10 time-unit 후 a=1
#0  b = 2;          // 동일 시각 마지막에 실행 (Inactive region)

// intra-assignment delay: RHS는 즉시 평가, LHS는 d 후 갱신
data = #5 bus_in;   // bus_in 현재값 샘플 → 5 후 data에 저장
```

`#0` delay는 Active region 동시 실행 간 race condition을 피하기 위해 쓰이지만
남용하면 코드가 이해하기 어려워진다.

### event 제어 (@)

```verilog
@(posedge clk)           // rising edge 대기
@(negedge rst_n)         // falling edge 대기
@(a or b or c)           // a, b, c 중 하나 변화 시 재개
@(*)                     // 내부 참조 신호 변화 시 재개 (Verilog-2001)
@(posedge clk or posedge rst) // 복수 에지

// named event: 사용자 정의 이벤트 트리거
event ev_done;
-> ev_done;          // 이벤트 발생
@(ev_done);          // 이벤트 대기
```

### wait (레벨 감지)

```verilog
wait (ready == 1);    // ready가 1이 될 때까지 블록 (level-sensitive)
wait (count == 10);
```

`@(posedge)`는 edge 변화 시 한 번 재개하지만, `wait`은 조건이 이미 참이면 즉시 통과한다.

### 시간 제어 종류 비교

| 구문 | 방식 | 재개 조건 |
|------|------|---------|
| `#d` | 지연 기반 | d time-unit 경과 후 |
| `@(posedge/negedge)` | 에지 감지 | 신호 에지 발생 시 |
| `@(signal)` | 변화 감지 | 신호 값 변화 시 |
| `wait(cond)` | 레벨 감지 | cond가 참이 될 때 (이미 참이면 즉시) |

---

## Sources

- IEEE 1364-2001 §9.4–§9.6 (if, case, loop), §9.7 (timing control), §9.8 (fork-join)
- IEEE 1800-2017 §12 (procedural statements), §9.3 (fork-join_any/none)
- verilogpro.com/verilog-case-casez-casex/ (casex 위험, casez 권장 — WebFetch 검증)
- eclipse.umbc.edu/robucci/cmpe316/lectures/L08__Full_and_Parallel_Case/ (full/parallel_case pragma)
- verificationacademy.com: full_case vs parallel_case vs casex vs casez 포럼
- Cliff Cummings, "full_case parallel_case, the Evil Twins of Verilog Synthesis", SNUG 1999
- vlsiverify.com/verilog/procedural-timing-control/ (delay/event control — WebFetch 검증)
- chipverify.com/systemverilog/systemverilog-fork-join (fork-join 3종 — WebFetch 검증)
- theoctetinstitute.com/content/verilog/loops/ (for/while/repeat/forever)
