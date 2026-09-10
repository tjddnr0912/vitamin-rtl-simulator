# Spike — SV interface/modport 평탄화 (Phase-2 관문 (D))

> **2026-06-11 · 구현 완료** — §2 스케치 그대로 랜딩(⑥ front-end 일괄, 722 green).
> 차이점 1개: 인스턴스 평탄화를 **nets 단계(4c)로 조기화**(부모 body가 `i.sig`를
> 참조하려면 pass 7 이전에 심볼이 있어야 — 스케치는 child-인스턴스 단계를 가정).
> iverilog 13.0이 interface-typed port를 거부해 차분 불가 → hand-IEEE 핀(§25).
>
> **2026-06-10 · 판정: GO — "bump 불요" 가설 지지.** SimIr(frozen)·`.velab` 무변경으로
> interface를 elaborate에서 net 번들로 평탄화할 수 있다. 비용은 front-end뿐이며,
> 유일한 형상 영향은 **AST(`.vu` 스키마 해시) 1회 flip** — 커밋 골든 churn 0(핀 골든
> 없음 확인), 스테일 게이트가 vcmp 재실행만 요구. ROADMAP §F (D)의 검증 결과 문서.

## 1. 코드-다이브로 확정한 사실 (가설의 근거)

| # | 사실 | 위치 |
|---|---|---|
| F1 | 모듈 포트는 **cont-assign으로 lower**(input: `child=parent_expr`, output: `parent_lv=child`), inout은 단방향 근사+warn | `elaborate::bind_ports` |
| F2 | `symbols: BTreeMap<FQ,NetId>`는 **여러 FQ 이름 → 단일 NetId aliasing을 이미 지원** — `net_name_table`이 사전순 최소 FQ를 canonical로 선택(3-OS 안정) | `net_name_table` 주석 |
| F3 | 표현식의 다중 세그먼트 경로(`bus.sig`)는 파서가 이미 `Ident(HierPath{2 seg})`로 만들고, elaborate `resolve_net`이 **loud-deferred**("hierarchical name reference … deferred") | parser primary / `resolve_net` |
| F4 | VCD 계층은 net_names 사이드카의 dotted FQ 이름에서 **sorted-leaf walk로 `$scope` 중첩을 일반 생성**(멀티-top 작업에서 단일-root 미가정 확인) | vcd-writer |
| F5 | `.vu`는 `schema_hash::<hdl_ast::SourceUnit>()`을 내장 — AST variant 추가는 `.vu` 스테일화(캐시 무효)일 뿐, **`.velab`/SimIr/format_version과 무관**. AST 해시를 핀한 골든 테스트는 없음(grep 확인) | `cli::run_vcmp`/`artifact_header` |
| F6 | `intf i();` 인스턴스화 구문은 **기존 ModuleInstance 문법과 동일**(이미 파싱됨 — 현재는 elaborate "unknown module"로 loud) | parser |
| F7 | 파서엔 **parse-time member desugar 선례**가 있음(packed-struct `s.field` → PartSelect) — 단 interface member는 elaborate-시점 해석이 맞음(인스턴스 바인딩이 elaborate 소관) | `struct_field_select` |

## 2. 설계 스케치 (구현 시점에 그대로 사용)

**원칙: interface 신호 = 평범한 net.** 새 IR 노드 0개. 참조 의미론은 cont-assign이
아니라 **심볼 aliasing**으로 — 방향이 없으므로 assign을 쓰면 멀티드라이버 의미론이
왜곡된다(F1의 inout 근사를 반복하지 말 것).

1. **lexer**: `Kw::{Interface, Endinterface, Modport}` 추가 (Kw enum은 비동결 — assert 선례).
2. **AST**: `TopItem::Interface(ModuleDecl)` 1 variant(본문 형상은 ModuleDecl 재사용 —
   param/신호/cont-assign/proc까지 공짜). modport는 `ModuleItem::Modport{name, dirs}`
   1 variant. → `.vu` 해시 flip은 이 2개로 끝.
3. **elaborate**:
   - interface decl을 모듈 맵과 분리된 `interface_map`에 등록.
   - `intf i();` → 모듈 인스턴스처럼 `cur_prefix=…​.i`로 신호 net 생성(+interface 내부
     cont-assign/proc도 그대로 lower — ModuleDecl 재사용의 보상).
   - 모듈 포트가 interface 타입(`intf bus` / `intf.mp bus`)이면 `bind_ports`에서
     cont-assign 대신: 연결 식이 interface 인스턴스명인지 확인 →
     **child 스코프에 `child….bus.<sig> → NetId(parent….i.<sig>)` 심볼 alias 일괄 삽입**
     (net 생성 0; F2가 canonical naming을 자동 처리).
   - `resolve_net` 확장: 다중 세그먼트를 dot-join해 `lookup_net_scoped` 워크 →
     실패 시 기존 loud-deferred 유지(진짜 cross-hierarchy ref는 계속 거부).
     lvalue 쪽도 동일 경로.
   - **modport** = elaborate-시점 방향 체크 사이드테이블(input member에 쓰기 = E).
     MVP는 파싱+수용, 체크는 후속 증분으로 분리 가능.
4. **VCD**: 작업 0 — alias된 net은 canonical FQ(선언 위치 `tb.i.sig`)로 한 번만 방출(F2+F4).

## 3. MVP 경계 (명시 제외)

- virtual interface(Phase-3), interface 배열, generate 내 interface 포트 전달, `import`/
  package(동류이나 별도), interface 안 modport task/function. 전부 **loud**로 남길 것.
- ANSI 헤더의 `intf.mp p` 타입 파싱이 유일한 문법 모험 지점(타입 자리의 `ident.ident`) —
  파서 lookahead 1로 충분.

## 4. 구현 시점 권장

(C) dynamic-storage·(B) named-event도 **AST 추가**(타입/decl)를 동반하므로, `.vu` flip을
1회로 모으려면 **interface front-end는 v5 bump 묶음과 같은 시기**에 랜딩하는 것이 최적.
단, SimIr와는 독립이므로 기술적으로는 아무 때나 가능(이 문서가 그 근거).
