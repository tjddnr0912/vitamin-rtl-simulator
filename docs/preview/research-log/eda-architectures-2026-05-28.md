---
topic: eda-architectures
date: 2026-05-28
rounds: 2
primary_sources_fetched:
  - https://steveicarus.github.io/iverilog/developer/guide/index.html
  - https://steveicarus.github.io/iverilog/developer/guide/vvp/vvp.html
  - https://steveicarus.github.io/iverilog/developer/guide/vvp/vthread.html
  - https://yosyshq.readthedocs.io/projects/yosys/en/stable/yosys_internals/formats/rtlil_rep.html
  - https://github.com/verilator/verilator/blob/master/docs/internals.rst
  - https://verilator.org/guide/latest/verilating.html
queries:
  - "Round 1-A: Icarus Verilog internal architecture pipeline vvp bytecode IR lex parse elaborate"
  - "Round 1-B: Verilator internal architecture C++ code generation stratified scheduling mt task graph"
  - "Round 1-C: Yosys RTLIL internal representation IR design synthesis passes"
  - "Round 1-D: Icarus Verilog developer guide source architecture iverilog compiler elaboration"
  - "Round 2-A: WebFetch steveicarus.github.io/developer/guide/index.html"
  - "Round 2-B: WebFetch yosyshq.readthedocs.io RTLIL rep"
  - "Round 2-C: WebFetch verilator/internals.rst"
  - "Round 2-D: WebFetch steveicarus.github.io/vvp/vvp.html"
---

# EDA 시뮬레이터 내부 아키텍처 조사 (2026-05-28)

오픈소스 HDL 도구 세 종(Icarus Verilog, Verilator, Yosys)의 내부 파이프라인을 조사했다. 목적은 이벤트 구동 인터프리터 방식 Rust 시뮬레이터(vitamin)의 IR 설계, 계층 평탄화, 언어 프론트엔드와 시뮬레이션 코어 분리 전략을 확립하는 데 실질적인 선례를 확보하는 것이다.

## Icarus Verilog — 인터프리터의 선례

Icarus Verilog는 두 바이너리로 구성된다. `iverilog`가 컴파일러, `vvp`가 런타임 실행기다. 컴파일러 파이프라인은 다음 순서로 동작한다.

**pform (parse form):** flex/gperf 렉서 + bison 파서가 소스를 읽어 "장식된 파스 트리(decorated parse tree)"를 만든다. 파일들은 `PScope.h`, `Module.h`, `PGenerate.h`, `Statement.h`, `PExpr.h`가 정의하는 클래스로 표현된다. 이 시점에는 아직 계층이 해소되지 않은 원문 구조다.

**elaborate → netlist form:** elaboration이 pform을 소비해 "netlist form"을 만든다. 이 단계에서 루트 모듈을 선택하고, 모듈 인스턴스를 재귀적으로 만들며 계층을 평탄화한다. 파라미터 오버라이드 평가와 generate 스킴 처리가 스코프 계층화와 인터리빙되어 진행된다. 연결성·타입 검사도 이 단계에서 수행된다.

**ivl_target API → code gen:** elaborated netlist는 functor/optimizer 패스를 거쳐 `ivl_target.h` API가 정의하는 내부 표현으로 변환된다. 코드 생성기(tgt-vvp 등)는 이 API를 통해 결과를 받아 대상 포맷으로 출력한다. tgt-vvp는 vvp 텍스트 바이트코드 파일을 생성한다.

**vvp 런타임 — functor + thread 이중 아키텍처:** vvp 파일은 텍스트 형식의 명령어 집합이다. 내부는 두 층으로 분리된다.

구조층(functor net)은 조합 논리를 표현한다. functor는 최대 4개 입력을 받아 1개 출력을 내는 논리 단위이며, 4-state(0/1/x/z)를 각 2비트로 인코딩한다(0→00, 1→01, x→10, z→11). 64바이트 진리표 배열이 4입력 × 4state의 모든 조합을 커버한다. AND/OR/NAND 같은 기본 게이트부터 DFF/래치, tri-state 리졸버, 산술/비교 노드, 파트 선택자까지 functor 변종으로 표현된다. net(와이어 구조)과 var(행동 대입 가능 변수)가 functor를 연결한다.

행동층(thread)은 절차적 코드를 실행한다. `.thread <symbol>` 문이 initial/always 블록에 대응하는 스레드를 만든다. 각 스레드는 프로그램 카운터, 4-value 비트 메모리, 64비트 워드 메모리를 갖는다. `%set`(블로킹 대입), `%assign`(논블로킹 대입), `%load`(functor 출력 읽기), `%wait`(이벤트 대기)가 스레드와 functor net을 연결하는 핵심 명령어다. `%fork`/`%join`으로 fork-join 동시성을 구현한다.

이벤트 큐는 skip list(정렬된 단방향 연결 리스트 + 건너뛰기 포인터)로 구현되어 있으며 delta-time 이벤트를 효율적으로 처리한다. 이벤트는 세 종류다: 전파 이벤트(functor 출력 변화), 대입 이벤트(논블로킹 대입), 스레드 스케줄 이벤트(스레드 재개). 이 세 종류가 단일 이벤트 큐에서 처리된다.

## Verilator — 컴파일드 방식의 설계

Verilator는 Verilog/SV를 C++로 변환해 일반 C++ 컴파일러가 최적화하게 하는 "컴파일드" 시뮬레이터다. 공식 internals 문서가 이 방식의 핵심 구조를 명확히 설명한다.

**파이프라인:** Flex + Bison 렉스/파스로 AstNode 계층을 만들고, 약 20단계의 V3 pass가 순차로 AST를 변환·최적화한다. 링크·elaborate, 상수 전파, dead code 제거, 스코프 참조(pseudo-flattening), V3Order(정적 스케줄 계산), V3EmitC(C++ 출력) 순이다. 각 패스는 Visitor 패턴(VNVisitor 서브클래스)으로 구현돼 알고리즘과 AST 구조가 분리된다.

**정적 스케줄링 — V3Sched:** Verilator가 인터프리터와 근본적으로 다른 지점이다. Verilator는 정적 macro-task 스케줄링을 채택하며 동적 macro-dataflow 모델을 명시적으로 거부한다 — internals.rst는 "Sarkar describes two options: you can dynamically schedule tasks at runtime... Verilator does not support this... The other option is to statically assign macro-tasks to threads... Verilator takes this static approach"라고 설명한다. 대신 V3Sched가 설계 전체의 논리를 정적으로 분류한다: 초기 프로세스, 정적 초기화, 조합 논리, 클로킹 논리, 하이브리드(조합 루프 파단)로 나뉘며 각각 `ico`(입력 조합), `act`(active 영역), `nba`(비블로킹 대입) 파티션으로 배분된다. 결과로 생성된 `_eval()` 함수는 이 파티션을 위에서 아래로 순서대로 실행한다 — 이벤트 큐 없이.

**MTask (매크로태스크) 멀티스레딩:** V3Order가 세밀한 의존성 그래프를 만들고 V3Partition이 엣지 수축으로 이를 거칠게 합쳐 MTask를 만든다. 각 MTask는 동기화 없이 단일 CPU에서 start-to-end 실행된다. InstrCountVisitor가 비용을 추정하고, par-factor(전체 비용 / 최장 경로 비용)를 기준으로 병렬성을 평가한다. 변수는 TSP 근사로 공간 지역성을 최적화한다.

**핵심 교훈:** Verilator는 인터프리터가 아니기 때문에 완전한 IEEE 1800 stratified scheduler를 구현하지 않는다. 표준 준수(정확성)와 속도는 트레이드오프이며, vitamin처럼 정확성을 먼저 확보하는 인터프리터 방식이 표준에 더 충실할 수 있다.

## Yosys RTLIL — 언어 중립 IR 설계의 교과서

Yosys는 합성 도구이므로 시뮬레이터가 아니지만, RTLIL(RTL Intermediate Language) 설계는 언어 중립 IR을 만드는 방법의 교과서적 선례다.

**계층 구조:** RTLIL::Design(루트) → RTLIL::Module(복수) → {Cell, Wire, Process, Memory}. Cell은 로직 게이트/기능 단위(이름, 타입, 어트리뷰트, 파라미터, 포트), Wire는 신호/버스(폭, 방향, 포트 정보), Process는 동작 제어 논리(SyncRule + CaseRule 결정트리), Memory는 어드레서블 배열이다.

**언어 비의존성 달성 방식:** 모든 HDL 프론트엔드(Verilog, VHDL 등)는 자신의 언어 특유 기능을 RTLIL 호환 표현으로 변환해야 한다. 파라미터화된 모듈은 콜백 함수로 변형 인스턴스를 생성한다. 이름 충돌은 백슬래시(사용자 정의) / 달러(자동 생성) 접두사 규약으로 방지한다. 변환 후에도 `hdlname` 어트리뷰트로 원래 식별자를 보존한다.

**고수준 구조 보존:** Process와 Memory 같은 고수준 구조를 패스 후반까지 유지한다. 이 덕분에 "proc → cells" 패스와 "memory → cells" 패스가 각각 독립적으로 다른 전략을 선택할 수 있다. 이 패턴 — IR에 의미론적 구조를 유지하고 백엔드가 소비하는 방식을 각자 결정 — 은 vitamin의 sim-ir 설계에 직접 적용 가능하다.

**4-state 및 coarse-grain 신호:** 버스를 개별 비트가 아닌 폭을 가진 단일 객체(Wire)로 표현한다. 비트 0은 항상 LSB다. 이 coarse-grain 모델은 IR 복잡도를 줄이면서도 임의 비트 슬라이스를 지원한다.

## vitamin Rust 시뮬레이터 설계에 대한 시사점

세 도구를 비교해 도출할 수 있는 교훈:

**IR 경계가 모든 것을 결정한다.** Icarus는 프론트엔드(pform→netlist)와 런타임(vvp)이 명확히 분리됐다. Yosys의 RTLIL은 언어 비의존 IR을 중간에 놓아 어떤 프론트엔드도, 어떤 백엔드도 교체 가능하게 만든다. vitamin의 sim-ir이 이 역할을 해야 한다.

**4-state 1급 표현.** Icarus의 functor는 0/1/x/z를 2비트로 인코딩해 64바이트 진리표로 처리한다. vitamin sim-ir의 net/value 타입도 4-state를 1급으로 표현해야 한다. 2-state 최적화는 나중에.

**인터프리터 vs 컴파일드의 정확성 트레이드오프.** Verilator는 속도를 위해 표준 stratified scheduler를 포기했다. Icarus는 인터프리터이기 때문에 표준에 더 충실하다. vitamin의 MVP가 인터프리터로 출발하는 것은 정확성 먼저 확보하는 올바른 순서다.

**builtin/system task 처리 분리.** Icarus의 tgt-vvp는 시스템 태스크를 VPI 모듈(system.vpi 등)로 별도 처리한다. vitamin에서는 hdl-builtins 크레이트가 이 역할을 담당하며, IR의 builtin-call 노드를 통해 인터프리터/컴파일드 양쪽이 동일한 dispatch 테이블에 접근한다.

**계층 평탄화는 elaborate 단계에서.** Icarus는 elaborate 단계에서 루트 모듈부터 재귀적으로 인스턴스화해 계층을 해소한다. vitamin의 elaborate 크레이트도 파라미터 해소와 계층 평탄화를 이 단계에서 수행하고 언어 중립 IR을 생성해야 한다.

## Sources

- https://steveicarus.github.io/iverilog/developer/guide/index.html — Icarus Verilog Developer Guide (WebFetch 검증)
- https://steveicarus.github.io/iverilog/developer/guide/vvp/vvp.html — VVP Simulation Engine (WebFetch 검증)
- https://steveicarus.github.io/iverilog/developer/guide/vvp/vthread.html — VVP Thread Architecture (WebFetch 검증)
- https://yosyshq.readthedocs.io/projects/yosys/en/stable/yosys_internals/formats/rtlil_rep.html — Yosys RTLIL Representation (WebFetch 검증)
- https://github.com/verilator/verilator/blob/master/docs/internals.rst — Verilator Internals (WebFetch 검증)
- https://verilator.org/guide/latest/verilating.html — Verilator Guide: Verilating (WebFetch 검증)
