# 01 · 합성 가능성 표기 범례

본 폴더의 모든 참조 문서는 각 구문/기능마다 다음 마커를 사용한다.

## 범례

| 마커 | 의미 |
|---|---|
| ✅ | **합성 가능** — 표준 합성 도구가 게이트로 변환 가능 |
| ⚠️ | **조건부** — 특정 형식만 합성 가능 / 도구 의존 / 합성 가능하나 권장 안 됨 |
| ❌ | **비합성** — 시뮬레이션 · 검증 전용 (예: `class`, assertion, `wait`, dynamic memory) |

## 사용 예

```
### `always_ff @(posedge clk)`
✅ 합성 가능. 표준 클록드 레지스터로 합성.

### `initial`
⚠️ 시뮬레이션용. FPGA 합성은 일부 지원(초기값), ASIC은 일반적으로 비합성.

### `class`
❌ 비합성. 검증 전용(SV OOP 확장).
```

## 판단 기준

합성 가능성 분류는 세 도구 기준을 교차 참조한다.

- **Synopsys Design Compiler (DC)** — ASIC 합성 업계 표준
- **Xilinx Vivado Synthesis** — FPGA 합성 대표 도구
- **Cadence Genus** — ASIC 합성 주요 도구

세 도구 중 두 곳 이상에서 RTL로 인식·변환되면 ✅. 도구별로 결과가 다르거나 구조에 따라 합성 여부가 달라지면 ⚠️. 어느 도구도 게이트 변환을 지원하지 않으면 ❌.

## 본 프로젝트와의 관계

본 프로젝트 Vitamin은 **시뮬레이터**다 — 합성은 비목표(본 spec §2.1). 그러나 참조 문서에 합성 가능 여부를 명기하는 이유는 두 가지다.

1. 사용자가 실제 RTL 작성 시 합성 친화적 코드를 식별할 수 있도록 돕는다.
2. Phase 1 MVP 범위인 "SV 합성 가능 RTL 서브셋"(= Verilog-2005 RTL 전부)을 구현 우선순위 기준으로 사용한다 — ✅ 항목이 Phase 1 대상, ❌ 항목은 후속 단계.

## Sources

- 본 spec §2.1 (참조 문서에 합성 가능 여부 명기 요구사항)
- Synopsys Design Compiler Synthesis User Guide
- Xilinx Vivado Design Suite User Guide: Synthesis (UG901)
- Cadence Genus Synthesis Solution User Guide
- IEEE 1800-2017 §A (Synthesizable subset annex)
