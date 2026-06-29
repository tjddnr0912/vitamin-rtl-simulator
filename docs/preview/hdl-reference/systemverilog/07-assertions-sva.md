# 07 · SystemVerilog Assertions (SVA)

IEEE 1800-2017 §16 기준.

> **비합성 전용**: assertion은 시뮬레이션·형식 검증(Formal Verification) 전용이다.
> `assert`/`assume`/`cover`를 포함한 RTL 코드는 합성 도구가 무시하거나 경고를 발생시킨다.
> 합성 가능성 요약은 [09-synthesizability.md](09-synthesizability.md) 참조.

---

## Immediate Assertion (§16.4)

즉각 assertion은 절차적 블록 안에서 현재 시뮬레이션 시간의 값을 직접 평가한다.
`if/else`문과 동작 방식이 같다 — 클록과 무관하다.

> **vitamin 구현 상태 (2026-06-10):** **immediate `assert`는 구현됨** — 파서가
> `Stmt::If`로 desugar(AST 동결 유지, X/Z cond → 실패 분기 = `if` 의미론과 동일).
> else절 생략 시 IEEE 디폴트 동작 `$error("Assertion failed")` 합성(stderr 진단 +
> exit 1, 런 계속). `assert property`(동시)는 **구현됨**(아래 '§16.14 Concurrent
> Assertion' 섹션의 vitamin 구현 상태 참조). ✅ `assert #0`(Observed 연기)/`assert final`(Reactive
> 연기) 충실 구현(2026-06-16) — `Stmt::DeferredAssert`로 파스, 어셔션당 flush 마커 +
> `defer_marks`/`defer_acts` 사이드카(순수 IR-0, format_version 8 무변경), 엔진이 NBA-후
> 진짜 Observed→Reactive 성숙 큐를 돌려 reach 시점에 평가·later region에 보고(`(marker,aid,gen)`
> 키로 flush-once, 재활용 aid 세대 스탬프). 非-`#0`/非-zero `#n` 어셔션 딜레이는 여전히 **loud**,
> `assume`/`cover`는 미지원(키워드 아님 → loud).

```systemverilog
always_ff @(posedge clk) begin
    // 즉각 assert: 현재 시뮬레이션 시간에 평가
    assert (data_valid || !enable)
        else $error("enable high but data_valid low");

    // assert/assume/cover 세 가지 형태
    assert (cnt < MAX_CNT);          // 실패 시 $error (기본 severity)
    assume (req == 1'b0);            // formal 검증에서 환경 가정
    cover  (state == IDLE);          // coverage 포인트 수집
end
```

### assert / assume / cover 의미론

| 키워드 | 의미 | 실패 시 |
|--------|------|---------|
| `assert` | 속성이 반드시 참이어야 함 | $error (기본), 또는 명시 severity |
| `assume` | 형식 검증에서 환경 제약 조건. 시뮬레이션에서는 assert처럼 동작 | $error |
| `cover` | 속성이 한 번이라도 참이 되었는지 추적 (coverage) | — (실패 없음) |

### Severity system tasks

```systemverilog
assert (expr) else $fatal(1, "critical: %s", msg);   // 시뮬레이션 종료
assert (expr) else $error("error: %0d", val);         // 에러 카운트 증가
assert (expr) else $warning("warn");                  // 경고만
assert (expr) else $info("note");                     // 정보
```

---

## Concurrent Assertion (§16.14)

동시 assertion은 **클록 엣지마다** 평가되며, 여러 사이클에 걸친 시퀀스 동작을 기술한다.
모듈·인터페이스·program 블록에 직접 배치하거나 `always` 블록 안에 넣을 수 있다.

> **vitamin 구현 상태 (2026-06-16, format_version 8):** concurrent assertion 서브셋이
> **구현됨** — 전부 **순수 IR-0 desugar**(ConcurrentAssert → 합성 clocked-always 체커,
> `materialize_sva_checkers`; sim-ir 골든 무변경, `.vu` AST-해시만 재핀). 구현 범위:
> - **함의**: 단일 클럭 `@(clk) a |-> b`(overlap)·`a |=> b`(non-overlap)·무-implication
>   `property(@(clk) e)`(=`1'b1 |-> e`); **multi-clock 정준** `@(c1) a |=> @(c2) b`(두-프로세스 핸드오프).
> - **시퀀스(antecedent)**: `##n`·`##[m:n]`·`##[m:$]`·`[*n]`·`[*m:n]`·`throughout`·`[->n]`·`[=n]`·`within`.
> - **sampled value func**: `$past`/`$rose`/`$fell`/`$stable`(prev-reg desugar; property body + pass/fail action block 내부).
> - **named 선언**: `sequence NAME;…endsequence`·`property NAME;…endproperty` + `assert property(NAME)`
>   + **formal arguments**(`sequence s(x,y)` 위치 바인딩) + generate-scope 선언 + property-references-property(overlap inner).
> - **action block** `[pass] else fail` · **disable iff**.
> - **SVA-REST 완결 (2026-06-14~16)**: `assume property`·`cover property`(히트 카운트)·property 연산자
>   `always`/`not`/`implies`/`iff`/`until`/`s_until`/`s_eventually`/`nexttime`·`let` 매크로·`$assertoff`/`$asserton`/`$assertkill`
>   런타임 게이트·`seq[+]`(=`[*1:$]`). liveness(`s_eventually`/`s_until`)는 end-of-sim `final` pend 체크로 실패 발행.
>
> **⭐오라클 NULL → 전면 hand-IEEE 핀**: iverilog 13.0이 concurrent assertion·named property/sequence·
> `$past`/`$rose`/`$fell`/`$stable`를 **모두 거부**('not supported'/'not defined')해 차분 오라클이 없다 —
> 검증은 named≡inline 행동 등가 + 합성 체커 byte-identity로 수행.
>
> **LOUD(미구현·거부)**: multi-term 시퀀스 cross-clock·sequence local variable·recursive property·
> outer-`|=>` prop-ref skew. **선행-`##` consequent**(`a |-> ##1 b` / `a |-> ##[1:3] b` — consequent가 `##`로
> 시작하는 형태)는 현재 파서가 `E2002 'expected expression, found HashHash'`로 **loud-reject**한다(아래 예시는
> 의미상 `a |=> b`와 같으며 그 형태는 동작) — 잔여 파서 작업(REMAINING_WORK). consequent의 range/goto/
> nonconsec/unbounded/throughout/within도 loud(antecedent에서는 동작).

```systemverilog
// property + assert property 분리 선언 (권장)
property req_ack_handshake;
    @(posedge clk) disable iff (!rst_n)
    req |-> ##[1:3] ack;
endproperty

assert property (req_ack_handshake)
    else $error("ack did not arrive within 3 cycles of req");

// 인라인 선언
// ⚠️ consequent가 `##`로 시작하는 이 형태는 현재 파서가 loud-reject한다(위 LOUD 항목).
// 등가 형태 `req |=> gnt`(non-overlap)를 쓰거나 boolean-선두 consequent로 작성한다.
assert property (@(posedge clk) req |=> gnt)
    else $warning("gnt not granted in 1 cycle");

// cover — 커버리지 수집
cover property (@(posedge clk) req ##2 ack);

// assume — 형식 검증 환경 가정
assume property (@(posedge clk) $onehot(mode));
```

### Observed Region 샘플링

concurrent assertion은 SV 스케줄러의 **Observed region**에서 평가된다.

```
Active → NBA → Observed(assertion eval) → Reactive → Postponed
```

클록 엣지 이후 Active/NBA region의 논리 연산이 완료된 뒤 안정된 값을 샘플링한다.
→ assertion이 `glitch`에 의해 오동작하는 것을 방지한다.

`$past(sig, N)`: N 클록 전의 sampled value를 반환한다.

```systemverilog
// 2 사이클 전 req가 참이었을 때 현재 ack도 참이어야 함
assert property (@(posedge clk)
    $past(req, 2) |-> ack);
```

---

## Sequence 선언 (§16.9~16.10)

sequence는 클록 사이클 단위의 시간 관계를 기술하는 primitive 블록이다.

```systemverilog
sequence s_req_to_ack;
    req ##[1:3] ack;
endsequence

// 인자를 받는 sequence
sequence s_check_burst(logic start, logic done, int delay);
    start ##[1:delay] done;
endsequence
```

### 사이클 딜레이 ##N / ##[m:n]

| 문법 | 의미 |
|------|------|
| `seq1 ##1 seq2` | seq1 종료 다음 1 사이클에서 seq2 시작 |
| `seq1 ##0 seq2` | 같은 사이클에서 seq2 시작 (overlapping) |
| `seq1 ##N seq2` | N 사이클 후 seq2 시작 |
| `seq1 ##[m:n] seq2` | m~n 사이클 사이 seq2 시작 |
| `seq1 ##[1:$] seq2` | 1 사이클 이상 임의 시간 후 seq2 |

```systemverilog
// 정확히 2 사이클 후
assert property (@(posedge clk) req |-> ##2 ack);

// 1~5 사이클 이내
assert property (@(posedge clk) req |-> ##[1:5] ack);

// 반드시 언젠가 (1 사이클 이상)
assert property (@(posedge clk) req |-> ##[1:$] ack);
```

### 반복 연산자 (§16.9.2~16.9.4)

#### Consecutive repetition [*N]

연속 N 사이클 동안 1 사이클 간격으로 반복 매칭. 시퀀스에도 적용 가능.

```systemverilog
// sig가 4 사이클 연속으로 참
sig[*4]                    // = sig ##1 sig ##1 sig ##1 sig

// 2~5 사이클 연속
sig[*2:5]

// 0회 이상 (빈 매칭 포함)
sig[*]                     // = sig[*0:$]

// 1회 이상
sig[+]                     // = sig[*1:$]

// req 이후, ack가 올 때까지 data_valid가 4 사이클 연속
assert property (@(posedge clk)
    req |-> data_valid[*4] ##1 ack);
```

#### Non-consecutive repetition [=N]

Boolean expression이 N번 비연속적으로 참이 됨. 중간에 임의의 클록이 끼어도 됨.
**Boolean expression에만 적용** (sequence에는 불가).

```systemverilog
// start와 end 사이에 data_rdy가 3번 비연속으로 참
assert property (@(posedge clk)
    start ##1 data_rdy[=3] ##1 end_flag);

// 범위 지정
assert property (@(posedge clk)
    req |-> ack_pulse[=1:4] ##[0:1] done);
```

#### Goto repetition [->N]

Non-consecutive와 유사하지만 **N번째 마지막 매칭 시점에서 시퀀스가 완료**된다.
**Boolean expression에만 적용**.

```systemverilog
// rd_en이 4번 매칭되는 마지막 시점에서 완료, 이후 intr_en 확인
assert property (@(posedge clk)
    $rose(rdy) ##1 rd_en[->4] ##1 intr_en);

// [=N] vs [->N] 차이:
// [=N]: 마지막 매칭 이후 종료 조건까지 추가 클록 허용
// [->N]: 마지막 매칭 직후 다음 표현식 시작
```

---

## Property 선언 (§16.12~16.13)

property는 시간 속성을 기술하는 상위 구조다. sequence보다 풍부한 표현력을 가진다.

```systemverilog
property p_name [#(params)] [port_list] ;
    [disable iff (expr)]
    property_expr
endproperty
```

### 함의 연산자 (§16.12.7)

| 연산자 | 이름 | 의미 |
|--------|------|------|
| `seq \|-> prop` | overlapping implication | seq 매칭 사이클과 **같은** 사이클에서 prop 평가 |
| `seq \|=> prop` | non-overlapping implication | seq 매칭 사이클 **다음** 사이클에서 prop 평가 |

```systemverilog
// overlapping: req 참인 사이클에 ack도 참이어야 함
property p1;
    @(posedge clk) req |-> ack;
endproperty

// non-overlapping: req 참인 다음 사이클에 ack 참 (|-> ##1과 동등)
property p2;
    @(posedge clk) req |=> ack;
endproperty

// 복합: req 이후 1~3 사이클 안에 ack
property p3;
    @(posedge clk) req |-> ##[1:3] ack;
endproperty
```

### throughout — sequence operator (§16.9.9)

시퀀스 전체 기간 동안 조건이 항상 참이어야 한다.

```systemverilog
// req가 참인 상태로 ack_pulse가 올 때까지 data_valid가 유지
property p_burst_valid;
    @(posedge clk)
    $rose(req) |-> (data_valid throughout ack_pulse[->1]);
endproperty
```

### until / until_with — property operators (§16.13.4)

| 연산자 | 형태 | 의미 |
|--------|------|------|
| `p1 until p2` | non-overlapping | p2가 참이 되기 **직전**까지 p1 참 (p2 포함 안 함) |
| `p1 until_with p2` | overlapping | p2가 참이 되는 그 사이클 **포함**해서 p1 참 |

```systemverilog
// req가 gnt 직전까지 참 (gnt 사이클은 req 불필요)
property p_req_until_gnt;
    @(posedge clk) req until gnt;
endproperty

// req가 gnt 사이클 포함해서 참
property p_req_until_with_gnt;
    @(posedge clk) req until_with gnt;
endproperty
```

**주의**: `until_with`는 property이므로 `##N`으로 직접 연결할 수 없다.

### implies (§16.12.9)

property 수준의 함의. 선행이 거짓이면 전체가 vacuously true.

```systemverilog
// mode_A이면 반드시 output_en이 참이어야 함
property p_mode;
    @(posedge clk) (mode == MODE_A) implies output_en;
endproperty
```

`|->` (sequence implication)와 달리 `implies`는 양쪽이 모두 property.

### always / s_eventually (§16.13.1~16.13.2)

```systemverilog
// always: 무한히 모든 클록에서 p가 참 (safety property)
property p_safety;
    @(posedge clk) always (fifo_count <= FIFO_DEPTH);
endproperty

// s_eventually: 언젠가 반드시 참이 됨 (strong liveness)
property p_liveness;
    @(posedge clk) s_eventually (req_served);
endproperty

// always s_eventually: 주기적으로 반드시 참 (strong recurrence)
property p_recurrence;
    @(posedge clk) always s_eventually (heartbeat);
endproperty
```

| 연산자 | 종류 | 의미 |
|--------|------|------|
| `always p` | safety | 모든 시점에서 p 참 |
| `s_eventually p` | strong liveness | 반드시 언젠가 p 참 |
| `eventually p` | weak liveness | 언젠가 참이 될 수도 있음 (유한 trace에서 vacuously true) |
| `always s_eventually p` | strong recurrence | 주기적으로 p 참 |

---

## Clocking Block과 Assertion (§14.3, §16.14.6)

interface의 clocking block을 assertion 클록으로 사용하면 설계-검증 타이밍 계약이
코드에 내재화된다.

```systemverilog
interface bus_if (input clk);
    logic req, ack;

    clocking cb @(posedge clk);
        input  #1step req;   // 1step 전에 샘플링
        output #1 ack;
    endclocking

    // clocking block을 assertion 클록으로 사용
    property p_req_ack;
        @(cb) req |-> ##[1:4] ack;
    endproperty
    assert property (p_req_ack);
endinterface
```

`default clocking`으로 assertion별 클록 생략:

```systemverilog
module checker_blk (input clk, req, ack);
    default clocking main_clk @(posedge clk); endclocking

    // 클록 명시 생략 — default clocking 사용
    assert property (req |-> ##[1:3] ack);
    assert property (ack |=> !ack);   // ack는 1 사이클 펄스
endmodule
```

---

## Action Block (§16.14.7)

assertion 결과에 따라 pass/fail 콜백을 정의한다.

```systemverilog
assert property (p_req_ack)
    $display("PASS: req->ack handshake OK at %0t", $time)  // pass action
    else $error("FAIL: req->ack timeout at %0t", $time);   // fail action

// pass action만 (fail action 생략 시 기본 $error 적용)
// fail action만 (else 절만 있으면 pass action 없음)
assert property (p_cnt_no_overflow)
    else $fatal(1, "Counter overflow detected!");
```

---

## disable iff — 리셋 처리 (§16.14.3)

```systemverilog
property p_with_reset;
    @(posedge clk) disable iff (!rst_n)
    req |-> ##[1:3] ack;
endproperty
```

`disable iff (cond)`: cond가 참인 동안 assertion 평가를 비활성화한다.
리셋 활성화 구간에서 assertion이 오동작하는 것을 방지하는 관용 패턴이다.

---

## 관련 문서

- [04-interfaces.md](04-interfaces.md) — clocking block 선언 상세
- [06-classes-oop.md](06-classes-oop.md) — UVM 기반 검증 패턴 (클래스 + randomization)
- [09-synthesizability.md](09-synthesizability.md) — assertion ❌ 비합성 매핑
- `../system-tasks/11-assertion-sampling.md` — `$past`, `$rose`, `$fell`, `$stable`, `$sampled`

---

## Sources

- IEEE 1800-2017 §16 (Assertions), §14 (Clocking blocks)
- chipverify.com/systemverilog/systemverilog-assertions (WebFetch ✓)
- vlsi.pro/sva-sequences-repetition-operators/ — [*N] [=N] [->N] (WebFetch ✓)
- verificationguide.com/systemverilog/systemverilog-implication-operator/ — \|-> \|=> (WebFetch ✓)
- verificationacademy.com/forums/systemverilog/sva-throughout-vs-until — throughout/until_with (WebFetch ✓)
