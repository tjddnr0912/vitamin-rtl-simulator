# 00 · IEEE HDL 표준 매핑

## 빠른 참조 표

| 표준 | 버전 | 발표 | 주요 변경 | 상태 |
|---|---|---|---|---|
| IEEE 1364 (Verilog) | 1995 | 1995 | 초판 — 4-state 논리, module/always/assign/initial, $display 등 system tasks | superseded |
| IEEE 1364 | 2001 | 2001 | generate, signed 네트, `always @*`, 내장 산술 연산자, 명명된 파라미터 오버라이드, 파일 I/O 강화 | superseded |
| IEEE 1364 | 2005 | 2005 | 마지막 독립 Verilog 표준. 모호한 정의 수정 + 1800-2005와 비호환 해소. `uwire` 추가 | merged into 1800-2009 |
| IEEE 1800 (SV) | 2005 | 2005 | SV 초판 — 1364와 별도 공존. `logic`, `always_ff/comb/latch`, interface, struct/union/enum, SVA, class, randomization | superseded |
| IEEE 1800 | 2009 | 2009 | **IEEE 1364-2005 완전 흡수** — SV가 Verilog의 상위 집합이 됨. 단일 문서. 동적 배열·큐 강화, assertion 개선 | superseded |
| IEEE 1800 | 2012 | 2012 | `unique if / priority if` 정제, 에라타 수정. 일관성·명확성 확보 중심 | superseded |
| IEEE 1800 | 2017 | 2017-12-06 | 에러 수정 + 소규모 정제. **IEEE GET Program 무료 제공 시작** | active (widely deployed) |
| IEEE 1800 | 2023 | 2024-02-28 | `ref static` 인수 방향 추가, 언어 확장 + 에라타 수정. **IEEE GET Program 무료 제공** | latest |
| IEEE 1076 (VHDL) | 1987 | 1987 | 초판 — DoD 요청 개발. 정수·실수·논리·문자·시간 타입, bit_vector, string | superseded |
| IEEE 1076 | 1993 | 1993 | 문법 일관성 향상, ISO-8859-1 문자 확장, `xnor` 추가, `postponed process` 도입 | superseded |
| IEEE 1076 | 2000 | 2000 | 소규모 — 보호 타입(protected type, C++ class 유사) 도입 | superseded |
| IEEE 1076 | 2002 | 2002 | 소규모 — 버퍼 포트 규칙 완화 | superseded |
| IEEE 1076 | 2008 | 2009-01-26 | **대규모 개정** — IEEE 1164/1076.2/1076.3 흡수, VHPI 통합, PSL 부분집합, 패키지/서브프로그램 generic | active |
| IEEE 1076 | 2019 | 2019-12-23 | 정수 64비트 확장, 조건부 분석(conditional analysis), 보호 타입 제네릭, PSL 강화, TEXTIO 확장. **IEEE GET Program 무료 제공** | latest |
| IEEE 1164 (std_logic_1164) | 1993 | 1993 | 독립 표준 — `std_logic` 9-value 논리, `std_logic_vector`, 해상도 함수 | merged into 1076-2008 |

## 언어 간 관계

**SystemVerilog ⊃ Verilog (2009 이후)**

IEEE 1800-2009부터 Verilog(IEEE 1364)의 모든 구조가 SystemVerilog 표준 문서 내부에 정의된다. 1364는 더 이상 독립 표준으로 존재하지 않는다. 본 프로젝트는 단일 SV 프론트엔드로 Verilog RTL을 포함해 처리한다.

**VHDL ⊃ Std_logic_1164 (2008 이후)**

IEEE 1076-2008이 IEEE 1164를 흡수했다. `use ieee.std_logic_1164.all;` 선언은 여전히 동작하지만 정의의 원천은 1076이다. IEEE 1164는 superseded 상태로 독립 갱신이 없다.

**VHDL은 SV와 독립 언어**

문법 · 의미론 · 타입 시스템 · 라이브러리 생태계가 완전히 다르다. 두 언어는 공유 IR(sim-ir) 이전 단계에서 완전히 분리된 프론트엔드를 요구한다.

## 본 프로젝트의 타겟 버전

| 언어 | 시작 기준 버전 | 후속 확장 |
|---|---|---|
| SystemVerilog | IEEE 1800-2017 | 2023 이슈 후속 검토 |
| Verilog (SV 내) | 1800에 흡수된 부분 전부 (= 1364-2005 RTL 전체) | — |
| VHDL | IEEE 1076-2008 | 2019 후속 검토 |

Phase 1 MVP 범위는 **SV 합성 가능 RTL 서브셋** (Verilog-2005 RTL 전부 포함). 상세는 [01-goals-and-scope.md](../01-goals-and-scope.md) 참조.

## 자유 접근 자료 (IEEE GET Program)

Accellera 후원으로 다음 표준을 무료 다운로드할 수 있다.

| 표준 | 접근 |
|---|---|
| IEEE 1800-2017 (SystemVerilog) | [IEEE Xplore GET](https://ieeexplore.ieee.org/browse/standards/get-program/page/) |
| IEEE 1800-2023 (SystemVerilog) | [IEEE Xplore GET](https://ieeexplore.ieee.org/browse/standards/get-program/page/) / [Accellera](https://www.accellera.org/downloads/ieee) |
| IEEE 1076-2019 (VHDL) | [IEEE Xplore GET](https://ieeexplore.ieee.org/browse/standards/get-program/page/) / [Accellera](https://www.accellera.org/downloads/ieee) |
| IEEE 1666-2023 (SystemC) | [Accellera](https://www.accellera.org/downloads/ieee) (본 프로젝트 범위 외) |

IEEE 1364 (1995/2001/2005)와 IEEE 1164 (1993)는 superseded 상태로 GET Program 대상이 아니며, 구매가 필요하다. 대부분의 내용은 IEEE 1800-2017/2023 LRM에서 확인할 수 있다.

## Sources

- 본 spec §10 (구조)
- research-log: [hdl-standards-versions-2026-05-28.md](../research-log/hdl-standards-versions-2026-05-28.md)
- IEEE 1800-2023: https://standards.ieee.org/ieee/1800/7743/
- IEEE 1364-2005 Xplore abstract: https://ieeexplore.ieee.org/document/1620780
- IEEE 1076-2019 Xplore abstract: https://ieeexplore.ieee.org/document/8938196
- Accellera GET Program downloads: https://www.accellera.org/downloads/ieee
