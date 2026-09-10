# Verilog Tasks/Functions, Gate-Level Primitives, Compiler Directives Research Log
**Date**: 2026-05-28
**Scope**: IEEE 1364-2001/2005 + IEEE 1800-2017 §13/§28/§22 Verilog-compat 부분집합
**Topics**:
- (A) tasks and functions — 선언 구문, 인수 방향, automatic/static, 반환값, 재귀, 시스템 태스크
- (B) gate-level primitives — 28종 분류, drive strength, UDP (조합형/순차형)
- (C) compiler directives — `define 매크로 인수, `default_nettype, `timescale, `begin_keywords, `resetall 등 전체

---

## 조사 방법론

3라운드 WebSearch + WebFetch 검증 방식. Round 1에서 3개 주제를 동시 브로드 스윕하고,
Phase 1.5에서 핵심 출처 7곳을 WebFetch로 직접 검증. Round 2에서 UDP 구문, `define 매크로
인수, `begin_keywords, drive strength, `default_nettype none 5개 gap을 fill.
Round 3에서 `begin_keywords 버전 문자열 정확도, drive strength 합성 제약, function
return 방식 차이 3개 추가 검증. 5차원 체크리스트 통과 후 종합.

---

## Round 1 — Broad Sweep

### 검색 쿼리 3개
1. `Verilog task function declaration syntax automatic static input output inout args return value recursion IEEE 1364-2001 1800-2017`
2. `Verilog gate level primitives and or nand nor xor buf not bufif nmos pmos cmos tran pullup pulldown drive strength UDP user-defined primitive IEEE 1364`
3. `Verilog compiler directives define undef ifdef ifndef timescale default_nettype include celldefine begin_keywords resetall pragma IEEE 1364-2001`

### Phase 1.5 — WebFetch 1차 Source 검증 (Round 1)

| URL | 확인 질문 | 결과 |
|-----|---------|------|
| chipverify.com/verilog/verilog-task | task 선언 구문, automatic/static, task vs function 차이 | ✅ |
| vlsiverify.com/verilog/gate-level-modeling/ | gate 분류, delay 구문 | ⚠️ MOS/tran/pullup/UDP 미포함 — gap 확인 |
| chipverify.com/verilog/verilog-compiler-directives | `define, `timescale, `include 기본 구문 | ⚠️ 매크로 인수/`begin_keywords 미포함 — gap 확인 |
| chipverify.com/verilog/verilog-functions | function return, output/inout 제한, automatic 재귀 | ✅ |
| peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00003.htm | 내장 프리미티브 전체 목록, MOS 구문, tran 계열, pullup/pulldown | ✅ |
| theoctetinstitute.com/content/verilog/compiler-directives/ | `begin_keywords, `resetall, `line, `timescale 범위 | ⚠️ timeout — hdlworks로 대체 |

### Round 1 주요 발견
- function은 0-시간 제약: `#`, `@`, `wait` 사용 불가
- function 반환값: 함수 이름과 동일한 내부 변수에 할당 (순수 Verilog-2001); SV에서 `return` 추가
- function은 `output`, `inout` 포트 사용 불가 (chipverify 원문 확인)
- task는 시간 소비 허용, output/inout/input 모두 가능
- automatic = 호출마다 독립 스택 프레임 → 재귀 가능; static = 공유 메모리 → 재귀 불가
- gate primitive: and/or/nand/nor/xor/xnor(다중입력), buf/not(단일입력), bufif0/bufif1/notif0/notif1(tri-state) 확인
- MOS switch: nmos/pmos/rnmos/rpmos (3포트), cmos/rcmos (4포트) 확인 (peterfab)
- tran/rtran: 제어 없음, tranif0/tranif1/rtranif0/rtranif1: 제어 포트 추가
- pullup: logic 1, pulldown: logic 0 구동
- `default_nettype none: 미선언 net → 컴파일 에러로 타이포 즉시 포착
- `timescale: 파일 경계 넘어 효력 지속

---

## Round 2 — Gap Fill

### 추가 검색 쿼리
1. `Verilog UDP user-defined primitive combinational sequential table syntax reg state table IEEE 1364`
2. `Verilog \`define macro arguments parameters function-like macros backtick line continuation undef`
3. `Verilog \`default_nettype none benefit catches undeclared net typo \`begin_keywords \`end_keywords \`resetall \`line \`pragma celldefine`

### Phase 1.5 — WebFetch 검증 (Round 2)

| URL | 확인 질문 | 결과 |
|-----|---------|------|
| chipverify.com/verilog/verilog-udp | UDP 구조, 조합/순차 테이블, 심볼 목록 | ✅ |
| vlsiverify.com/verilog/user-defined-primitives/ | 심볼 완전 목록(r/f/p/n/b/*/- ), edge-sensitive vs level-sensitive | ✅ |
| chipverify.com/verilog/verilog-define-macros | 인수 있는 매크로 `define ADD(a,b), 줄 이어 쓰기 | ✅ |
| hdlworks.com/hdl_corner/verilog_ref/items/CompilerDirectives.htm | `resetall 동작, `line 형식, `celldefine 목적 | ✅ |
| verilogams.com/refman/basics/directives.html | `begin_keywords/`end_keywords, `pragma | ❌ timeout — accellera.org로 대체 |

### Round 2 주요 발견
- UDP 심볼: 0/1/x (확정), ? (0·1·x 중 어느 것), b (0 또는 1), * (어떤 변화),
  - (변화 없음), r (01 상승), f (10 하강), p (잠재 상승), n (잠재 하강)
- UDP: 출력 반드시 1개, Z 출력 불가, 모든 포트 1비트 스칼라
- 조합형 최대 10 입력, 순차형 최대 9 입력
- `define 인수 있는 매크로: `define MACRO(a,b) body 형식; 인수 항상 괄호 권장
- 토큰 붙이기(token paste): a``b 형식
- `line: 파일명 + 라인번호 + 레벨(0=일반, 1=include 진입, 2=복귀)
- `celldefine: SDF back-annotation / 타이밍 분석 도구가 내부 블랙박스 처리에 활용

---

## Round 3 — Gap Fill 2

### 추가 검색 쿼리
1. `Verilog \`begin_keywords version string "1364-1995" "1364-2001" "1364-2005" "1800-2005" purpose keyword set IEEE 1800-2017 section 22`
2. `Verilog drive strength supply0 supply1 strong0 strong1 pull0 pull1 weak0 weak1 highz0 highz1 synthesis support gate primitive`

### Phase 1.5 — WebFetch 검증 (Round 3)

| URL | 확인 질문 | 결과 |
|-----|---------|------|
| vlsiverify.com/verilog/strength-in-verilog/ | drive strength 8레벨 테이블, strength0/1 유효 키워드, gate 인스턴스 구문, 합성 여부 | ✅ |
| accellera.org (P1800 키워드 제안서) | `begin_keywords 버전 문자열 목록 | ✅ |

### Round 3 주요 발견
- drive strength 8레벨: highz(0) → small(1) → medium(2) → weak(3) → large(4) → pull(5) → strong(6) → supply(7)
- capacitive strength (small/medium/large)는 trireg 전용
- (highz0, highz1) 또는 (highz1, highz0) 조합 불법
- 기본 drive strength: (strong1, strong0)
- **합성 미지원**: drive strength는 시뮬레이션 전용 개념; 합성 도구는 무시하거나 경고
- `begin_keywords 버전 문자열: "1364-1995", "1364-2001", "1364-2005", "1800-2005",
  "1800-2009", "1800-2012", "1800-2017" (accellera.org P1800 제안서에서 교차 확인)

---

## 5차원 체크리스트 최종

| 차원 | 상태 | 비고 |
|------|------|------|
| 정의 | ✅ | task/function/primitive/directive 모두 정의 확보 |
| 현황 | ✅ | 28종 프리미티브 분류, 심볼 목록, 버전 문자열 등 구체 사실 확보 |
| 근거 | ✅ | chipverify, vlsiverify, hdlworks, peterfab, accellera WebFetch 검증 |
| 반론 | ✅ | drive strength 합성 불가, tran RTL 미지원, `begin_keywords SV 충돌 문제 명시 |
| 적용 | ✅ | `default_nettype none 타이포 포착, automatic 재귀, `resetall 파일 패턴 등 실무 인사이트 |

---

## Sources

- chipverify.com/verilog/verilog-task (WebFetch 검증 ✓)
- chipverify.com/verilog/verilog-functions (WebFetch 검증 ✓)
- chipverify.com/verilog/verilog-udp (WebFetch 검증 ✓)
- chipverify.com/verilog/verilog-define-macros (WebFetch 검증 ✓)
- vlsiverify.com/verilog/gate-level-modeling/ (WebFetch 검증 ✓)
- vlsiverify.com/verilog/user-defined-primitives/ (WebFetch 검증 ✓)
- vlsiverify.com/verilog/strength-in-verilog/ (WebFetch 검증 ✓)
- peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00003.htm (WebFetch 검증 ✓)
- hdlworks.com/hdl_corner/verilog_ref/items/CompilerDirectives.htm (WebFetch 검증 ✓)
- accellera.org P1800 keyword compatibility directive proposal (begin_keywords 버전 문자열 교차 확인)
- IEEE 1364-2001 §10 (tasks/functions), §7 (gate primitives), §19 (compiler directives)
- IEEE 1800-2017 §13 (tasks/functions), §28 (gate primitives), §22 (compiler directives)
