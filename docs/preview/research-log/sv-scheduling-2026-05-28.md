---
topic: sv-scheduling
date: 2026-05-28
rounds: 2
primary_sources_fetched:
  - https://verificationguide.com/systemverilog/systemverilog-scheduling-semantics/
  - https://vlsiverify.com/verilog/verilog-scheduling-semantics/
  - https://verificationacademy.com/forums/t/0-in-systemverilog-inactive-event-region/35190
  - https://24x7fpga.com/sv_directory/2025_01_14_scheduling_semantics/
  - https://www.maven-silicon.com/blog/systemverilog-event-scheduler/
  - https://arxiv.org/html/2502.19348v1
  - https://www.accellera.org/images/eda/sv-ec/att-4051/Event_Scheduling_20070205a.pdf
queries:
  - "Round 1 영문: IEEE 1800 stratified event scheduling regions"
  - "Round 1 영문: SystemVerilog NBA region semantics"
  - "Round 1 한국어: Verilog 이벤트 영역 Active Inactive NBA Monitor"
  - "Round 2 영문: delta cycle infinite loop simulator"
  - "Round 2 영문: SV Preponed Observed Reactive Postponed region order"
---

# Research: IEEE 1800 / 1364 Stratified Event Scheduling

## Verilog(IEEE 1364)의 4개 기본 region

IEEE 1364는 한 simulation time slot을 네 개의 region으로 나눈다. 실행 순서는 Active → Inactive → NBA(Non-Blocking Assignment) → Monitor.

| Region | 실행 대상 | 비고 |
|--------|-----------|------|
| Active | 모듈의 blocking(`=`) assignment, continuous assign, 기본 소자 출력 갱신, NBA의 RHS 평가 | 임의 순서 허용 → 비결정론 가능 |
| Inactive | `#0` 지연이 달린 이벤트 | Active 소진 후 처리 |
| NBA | non-blocking(`<=`) LHS 갱신 | Inactive 소진 후 처리 |
| Monitor | `$monitor`·`$strobe` 시스템 태스크 | 시각 내 값 변화 종료 후 실행 |

핵심: NBA region이 Active와 분리되어 있어, 동일 시각에 실행되는 여러 non-blocking assignment의 LHS 갱신은 서로 RHS를 "샘플한 값"으로 일괄 갱신된다 — 플립플롭 체인이 동작하는 원리.

## delta cycle — 조합 논리 전파의 기반

delta cycle은 동일 simulation time에서 신호가 안정될 때까지 Active→Inactive→NBA 루프를 반복하는 0-time 이터레이션. `always @(a) b = a; always @(b) c = b;` 같은 조합 체인에서 `a` 변화 → delta 1 (b=a) → delta 2 (c=b) → delta 3 안정 → 다음 시각으로 전진.

무한 delta: 일부 상용은 이벤트 카운트 threshold(예: 10M)로 경고 발행. Icarus는 자동 감지 없음, 사용자 watchdog 필요.

## IEEE 1800 (SystemVerilog) 17개 region

SV는 assertion·program block·PLI 콜백 지원 위해 4개를 17개로 확장:

```
Preponed
Pre-Active
Active          ← blocking(=), continuous assign, NBA RHS 평가
Inactive        ← #0 이벤트
Pre-NBA
NBA             ← non-blocking(<=) LHS 갱신
Post-NBA        ← PLI 콜백
Pre-Observed
Observed        ← concurrent assertion 평가
Post-Observed
Reactive        ← program block blocking(=), assertion action
Re-Inactive
Pre-Re-NBA
Re-NBA
Post-Re-NBA
Pre-Postponed
Postponed       ← $monitor, $strobe, 커버리지
```

Active region set (Active/Inactive/Pre-NBA/NBA/Post-NBA) = RTL 설계 코드.
Reactive region set (Reactive/Re-Inactive/Pre-Re-NBA/Re-NBA/Post-Re-NBA) = testbench(program block).
이 이원 분리가 SV의 핵심 — DUT와 TB의 race condition 구조적 제거.

## blocking vs non-blocking — region 분리가 결정론을 보장

```systemverilog
// 쉬프트 레지스터 (NBA)
always_ff @(posedge clk) begin
  b <= a;  // Active에서 RHS(a) 샘플, NBA에서 LHS(b) 갱신
  c <= b;  // Active에서 RHS(b 현재값) 샘플, NBA에서 LHS(c) 갱신
end
// 결과: 한 클럭당 한 단계씩 쉬프트 — 결정론적

// blocking으로 바꾸면
always_ff @(posedge clk) begin
  b = a;  // Active에서 즉시 갱신
  c = b;  // 이미 갱신된 b를 읽어 c = a — 쉬프트 안 됨
end
```

## #0 시맨틱스 — "미루기"이지 "해결"이 아님

`#0`은 현재 active event를 Inactive로 이동시키는 메커니즘. race condition을 "제거"가 아니라 "한 delta 뒤로 미루는 것". `#0`이 또 `#0`을 부르는 악순환 위험. 실무에서는 NBA(`<=`) 또는 mailbox/semaphore 권장.

## Preponed / Observed / Reactive — assertion 파이프라인

| Region | 역할 |
|--------|------|
| Preponed | concurrent assertion용 신호를 "time slot 시작 직전"에 샘플링 |
| Observed | Preponed 샘플로 concurrent property/sequence 평가 |
| Reactive | Observed 결과의 pass/fail action + program block 실행 |
| Postponed | `$strobe`/`$monitor` + functional coverage. 이후 값 변화 금지 |

## Icarus Verilog 구현 관점

vvp 런타임은 skip list 이벤트 큐 사용. NBA의 RHS는 현재 delta에서 샘플, LHS 갱신은 "다음 delta"로 예약 — IEEE 1364 4 region 의미론 일치. SV의 Pre-Active/Post-NBA/Pre-Observed 같은 PLI·assertion 보조 region은 부분 구현.

## Sources

- https://verificationguide.com/systemverilog/systemverilog-scheduling-semantics/
- https://vlsiverify.com/verilog/verilog-scheduling-semantics/
- https://verificationacademy.com/forums/t/0-in-systemverilog-inactive-event-region/35190
- https://24x7fpga.com/sv_directory/2025_01_14_scheduling_semantics/
- https://www.maven-silicon.com/blog/systemverilog-event-scheduler/
- https://arxiv.org/html/2502.19348v1
- https://www.accellera.org/images/eda/sv-ec/att-4051/Event_Scheduling_20070205a.pdf
