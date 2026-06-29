---
date: 2026-05-28
topic: IEEE HDL 표준 버전 맵 (1364 / 1800 / 1076 / 1164)
rounds: 2
queries:
  - "IEEE 1364 Verilog standard versions 1995 2001 2005 history withdrawn merged SystemVerilog"
  - "IEEE 1800 SystemVerilog versions 2005 2009 2012 2017 2023 changes history 1364 merger"
  - "IEEE 1076 VHDL standard versions 1987 1993 2002 2008 2019 major changes std_logic_1164 merged"
  - "IEEE 1800-2023 SystemVerilog new features changes approved December 2023"
  - "IEEE GET program free download 1800-2017 1800-2023 1076-2019 VHDL available no cost"
  - "IEEE 1076-2019 VHDL changes improvements vs 2008 what is new"
primary_sources:
  - https://en.wikipedia.org/wiki/SystemVerilog
  - https://en.wikipedia.org/wiki/VHDL
  - https://en.wikipedia.org/wiki/IEEE_1364
  - https://standards.ieee.org/ieee/1800/7743/
  - https://www.accellera.org/downloads/ieee
  - https://www.accellera.org/news/press-releases/394-accellera-announces-ieee-1800-2023-standard-available-through-ieee-get-program
  - https://vhdlwhiz.com/vhdl-2019/
---

# IEEE HDL 표준 버전 맵 리서치 로그

조사 목적: `hdl-reference/00-standards-map.md` 작성을 위해 IEEE 1364 · 1800 · 1076 · 1164 각 표준의 버전 이력, 주요 변경, 상호 관계, 현재 상태, 무료 접근 가능성을 확인한다.

---

## IEEE 1364 — Verilog

Verilog는 원래 Cadence Design Systems의 독점 언어였다. 1990년 Cadence가 Open Verilog International(OVI)에 이전하면서 공개 표준화 절차가 시작됐고, 약 18개월의 작업 후 **IEEE 1364-1995**로 최초 제정됐다.

**IEEE 1364-1995 (Verilog-95)**: 초판. 기본 게이트 수준부터 RTL까지 커버하는 HDL 표준. `module`, `always`, `assign`, `initial`, 4-state (`0 1 x z`) 논리, `$display` 등 핵심 system tasks 포함.

**IEEE 1364-2001 (Verilog-2001)**: 5년간 사용자 피드백을 반영한 대규모 개정. 주요 추가 사항: `generate-endgenerate` 구문(파라미터 기반 반복 생성), signed 네트/변수 명시 선언, 내장 산술 연산자(`+`, `-`, `*`, `/`, `>>>` 등), `always @*` (감도 목록 자동 추론), 명명된 파라미터 오버라이드(`#(.WIDTH(8))`), 향상된 파일 I/O. Verilog-2001은 사실상 대부분 툴이 기준으로 삼는 버전이 됐다.

**IEEE 1364-2005 (Verilog-2005)**: 마지막 독립 Verilog 표준. 대규모 신기능 추가 없이 1995·2001의 모호한 정의를 수정하고, IEEE 1800-2005(SystemVerilog 초판)와의 비호환성을 해소하는 데 집중했다. 추가된 언어 요소는 `uwire` 키워드 정도. 2004년 중반 IEEE 1364 위원회가 해산되고 표준 유지보수가 IEEE 1800 워킹그룹으로 이전됐다.

**통합 · 철수**: 2009년 IEEE 1364-2005가 IEEE 1800-2009에 공식 흡수됐다. 이후 Verilog는 별도 표준으로 존재하지 않으며, 모든 Verilog RTL 구조는 SystemVerilog의 부분집합으로 정의된다. IEEE 1364는 superseded(대체됨) 상태다.

---

## IEEE 1800 — SystemVerilog

**IEEE 1800-2005**: SystemVerilog 최초 IEEE 표준. 2002년 Superlog 언어 기증과 OpenVera 검증 기능을 기반으로 형성됐다. 이 버전에서는 Verilog(IEEE 1364)가 별도 표준으로 공존했고, SV는 그 위에 쌓인 확장 집합이었다. `logic` 타입, `always_ff/always_comb/always_latch`, interface, `struct/union/enum`, assertion(SVA), clocking block, class, randomization 등을 도입했다.

**IEEE 1800-2009**: 핵심 분기점. **IEEE 1364-2005를 완전 흡수**해 단일 문서로 통합했다. 이로써 Verilog는 더 이상 독립 표준이 아니며, 1800이 Verilog를 포함하는 상위 집합이 됐다. 기능 추가로는 동적 배열(dynamic array)·큐(queue) 강화, assertion 속성 명세 개선 등이 있다.

**IEEE 1800-2012**: 점진적 개정. `unique if / priority if` 조건 구문 정제, 이전 버전 에라타 수정이 주된 내용이다. 새 언어 기능보다 일관성·명확성 확보에 집중한 버전이다.

**IEEE 1800-2017**: 2017년 12월 6일 승인. 에러 수정 위주의 보수적 개정. 이 버전부터 IEEE GET Program을 통해 **무료 다운로드** 가능(Accellera 후원). 현재 많은 툴의 실질 기준 버전이다.

**IEEE 1800-2023**: 2023년 12월 6일 이사회 승인, 2024년 2월 28일 출판. 현행 최신 표준. 언어 기능 확장과 이전 버전 에라타 수정을 포함한다. 주목할 추가 사항 중 하나는 `ref static` 인수 방향 지원 — task/function의 인수에 `ref static` 방향을 지정해 FSM 분해에 활용 가능. IEEE GET Program을 통해 **무료 다운로드** 가능(Accellera 후원). 업계 통계 기준 전체 전자 설계의 75% 이상이 SystemVerilog를 사용한다.

---

## IEEE 1164 — Std_logic_1164

**IEEE 1164-1993**: VHDL에서 `std_logic`(9-value logic: U X 0 1 Z W L H -)을 정의하는 독립 표준. `std_logic_vector`, 해상도 함수, 변환 함수를 포함한다. VHDL-1987/1993에서 사실상 필수 패키지로 쓰였으나, 표준 언어가 아닌 별도 IEEE 표준이었다.

**1076-2008 흡수**: IEEE 1076-2008에서 IEEE 1164의 내용이 VHDL 표준 언어 정의 내부로 통합됐다. 동시에 `std_logic_vector`가 `std_ulogic_vector`의 서브타입이 됐고, 축소 연산자, 매칭 연산자, min/max 함수, 시프트 연산자, 문자열 변환 함수 등이 추가됐다. IEEE 1164는 이후 superseded 상태이며, 독립 표준으로 갱신되지 않는다.

---

## IEEE 1076 — VHDL

**IEEE 1076-1987**: 미국 공군(DoD) 요청으로 개발된 VHDL의 최초 IEEE 표준. 정수·실수·논리·문자·시간 타입, `bit_vector`, `string` 등 풍부한 타입 시스템을 제공했다.

**IEEE 1076-1993**: 수년간 피드백을 반영한 대규모 개정. 문법 일관성 향상, 명명 유연성 증가, 문자 타입을 ISO-8859-1 인쇄 가능 문자로 확장, `xnor` 연산자 추가, `postponed process` 도입. 1993 버전은 이후 오랫동안 사실상 표준 기준 버전으로 사용됐다.

**IEEE 1076-2000**: 소규모 개정. 보호 타입(protected type, C++ 클래스 유사) 도입.

**IEEE 1076-2002**: 2000 개정의 소규모 수정. 버퍼 포트 규칙 완화.

**IEEE 1076c-2007**: VHPI(VHDL Procedural Interface) 도입 — C/C++ 등 외부 언어와의 인터페이스를 제공하는 부속 표준. 2008에 통합됐다.

**IEEE 1076-2008**: 2009년 1월 26일 발행. **대규모 개정**. 주요 변경:
- IEEE 1164 (`std_logic_1164`), IEEE 1076.2 (수치 패키지), IEEE 1076.3 (합성 패키지)를 VHDL 본 표준으로 통합
- VHPI 통합 (C 인터페이스 공식화)
- PSL(Property Specification Language) 부분집합 포함
- 패키지·서브프로그램에 제네릭(generic) 허용
- 외부 이름(external names) 기능 도입
- `case`·`generate` 구문 유연성 향상

이 버전부터 별도로 IEEE 1164를 `use ieee.std_logic_1164.all;`로 참조해도 내용은 동일하게 동작한다.

**IEEE 1076-2019**: 2019년 12월 23일 승인. 현행 최신 표준. 주요 추가·변경:
- 정수 타입 32비트 → 64비트 확장 (시뮬레이션 용량 증가)
- 조건부 분석(conditional analysis) — 상수값 기반 코드 포함·제외 전처리기 지시어 (`\`if`, `\`elsif` 유사 문법)
- 보호 타입 제네릭 파라미터 지원 (C++ 템플릿 유사)
- 레코드/복합 타입 개선: 빈 레코드 허용, 요소별 방향 지정 가능, 다른 레코드 타입 간 암묵적 변환
- PSL 통합 강화 (assertion·coverage 상태 조회 함수)
- `TEXTIO` 패키지 확장: seek·rewind·truncate, 환경변수 읽기, 디렉토리 API
- 스택 introspection (디버깅 지원)
- 대부분의 신기능은 시뮬레이션·검증 대상; 합성 가능 부분은 제한적

---

## 언어 간 관계 요약

- **SV ⊃ Verilog (2009 이후)**: IEEE 1800-2009 이후 Verilog 구조 전체가 SystemVerilog 문서 내에 정의된다. 별도 Verilog 표준 없음.
- **VHDL ⊃ Std_logic_1164 (2008 이후)**: IEEE 1076-2008이 IEEE 1164를 흡수. 별도 1164 표준 없음(superseded).
- **VHDL은 SV와 독립 언어**: 문법·의미론·생태계가 완전히 다르다. 두 언어는 컴파일러 구현에서 별도 프론트엔드를 요구한다.

---

## IEEE GET Program — 무료 접근

다음 표준이 IEEE GET Program을 통해 무료 다운로드 가능하다 (Accellera 후원):

| 표준 | 버전 | 무료 여부 |
|---|---|---|
| IEEE 1800 (SystemVerilog) | 2017 | ✅ GET Program |
| IEEE 1800 (SystemVerilog) | 2023 | ✅ GET Program |
| IEEE 1076 (VHDL) | 2019 | ✅ GET Program (Accellera 후원) |
| IEEE 1666 (SystemC) | 2023 | ✅ GET Program |
| IEEE 1364 (Verilog) | 1995/2001/2005 | ❌ superseded / 구매 필요 |
| IEEE 1164 | 1993 | ❌ superseded / 구매 필요 |

접근 경로: `https://ieeexplore.ieee.org/browse/standards/get-program/page/` 또는 `https://www.accellera.org/downloads/ieee`

---

## Sources

- Wikipedia — Verilog (IEEE 1364): https://en.wikipedia.org/wiki/IEEE_1364
- Wikipedia — SystemVerilog: https://en.wikipedia.org/wiki/SystemVerilog
- Wikipedia — VHDL: https://en.wikipedia.org/wiki/VHDL
- IEEE Xplore — IEEE 1800-2023: https://standards.ieee.org/ieee/1800/7743/
- Accellera — IEEE 1800-2023 announcement: https://www.accellera.org/news/press-releases/394-accellera-announces-ieee-1800-2023-standard-available-through-ieee-get-program
- Accellera downloads: https://www.accellera.org/downloads/ieee
- VHDLwhiz — What's new in VHDL-2019: https://vhdlwhiz.com/vhdl-2019/
- Doulos — VHDL-2008 incorporates existing standards: https://www.doulos.com/knowhow/vhdl/vhdl-2008-incorporates-existing-standards/
- Siemens Verification Horizons — IEEE 1800-2023 LRM: https://blogs.sw.siemens.com/verificationhorizons/2024/03/04/get-your-free-copy-of-the-ieee-1800-2023-systemverilog-lrm/
- IEEE Xplore — 1364-2005 abstract: https://ieeexplore.ieee.org/document/1620780
- GlobalSpec — IEEE 1364 standard page: https://standards.globalspec.com/std/649467/IEEE%201364
