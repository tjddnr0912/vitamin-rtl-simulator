# 14 · 단계별 산출물 · 해시 결합 · CLI 표면

> 04-architecture.md "실행 모델 — 원샷과 단계별 실행" 절의 권위 있는 상세 명세다.
> 단계 간 산출물의 온디스크 포맷, staleness(신선도) 해시 결합 규칙, 멀티 라이브러리
> 주소화, CLI 플래그 표면, 구조적 schema 해시 메커니즘을 정의한다.
> 04는 요약이며, 단계별 실행의 단일 진실 공급원은 본 문서다.

---

## 설계 결정 (확정)

| # | 결정 | 내용 |
|---|---|---|
| D1 | 전용 크레이트 | (역)직렬화 · 헤더 · 버전 · staleness · `--dump` 로직은 전용 크레이트 `vita-artifact`가 보유. 형상 해시는 proc-macro 크레이트 `vita-artifact-derive`가 산출하고 런타임 합성은 leaf 크레이트 `vita-schema`가 담당(16). 워크스페이스 11→15(+`vita-artifact`·`vita-artifact-derive`·`vita-schema`·`vita-log`). `cli`는 얇은 글루로 유지. |
| D2 | 구조적 schema 해시 | schema 버전은 직렬화 타입 **형상의 구조적 파생 해시**(`#[derive(SchemaHash)]`). 손으로 올리는 const가 아니다. 빌드 지문(git sha/dirty/profile)은 provenance 전용으로 별도 stamp(해시 키 아님). |
| D3 | 멀티 라이브러리 | 설계 단계부터 도입. 단위는 `library:unit` 논리 키로 주소화하며, 논리명→디렉터리 매핑(`cds.lib`/`synopsys_sim.setup` 계열)을 처음부터 지원. |
| D4 | span-free IR | `sim-ir`는 언어 중립·소스 span 비보유. 런타임 진단 위치는 IR 노드 인덱스로 키잉된 **선택적·독립 버전 사이드테이블**에 둔다. `file_id→path` 맵은 work 매니페스트에. |

---

## §1 산출물 레이아웃

파이프라인은 두 경계에서 디스크 산출물을 남긴다. 두 산출물은 **모양이 다르다** —
compile 산출물은 단위별 디렉터리, elaborate 산출물은 단일 자기완결 파일이다.

```
vcmp (compile)  →  work/            ← 디렉터리: 설계단위별 분석 결과 (언어 의존)
velab (elaborate) →  <top>.velab    ← 단일 파일: elaborate된 sim-ir (언어 중립, 자기완결)
vrun (simulation) →  (VCD, dump 호출 시) + stdout
```

### vcmp 산출물 — work 라이브러리 (디렉터리)

설계단위(모듈/패키지/엔티티) 하나당 **파싱·국소검사된 AST**를 직렬화한 blob 하나.
parse가 마지막 언어 의존 단계이므로 이 산출물은 **언어 의존**이며, 계층·파라미터는
아직 미해소다. 단위별 입도(granularity)가 증분 재분석을 가능하게 한다.

```
work/
├── lib.toml                 # 사람이 읽는 TOML 매니페스트 (버전 독립 텍스트)
└── units/
    ├── <unit-a>.vu          # 단위별 바이너리 blob (헤더 + postcard 본문)
    └── <unit-b>.vu
```

`work/lib.toml` (매니페스트):

```toml
format_version = 8   # work/lib.toml 매니페스트 포맷(현재 8). 산출물 컨테이너 format_version도 8 (CURRENT_FORMAT_VERSION).
tool           = { version = "0.1.0", git_sha = "…", dirty = false, profile = "release" }
[library]
name = "work"                # 논리 라이브러리 이름 (D3 — 기본 work)
dialect_digest = "blake3:…"  # 전역 dialect(--std/-sv) 다이제스트 (§2 RULE A)

# 정렬된 컴파일 단위 파일 목록 — 추가/삭제/재정렬이 매니페스트 해시를 바꾼다 (§2 RULE S)
files = [
  { path = "rtl/a.sv", src_sha256 = "blake3:…" },
  { path = "rtl/b.sv", src_sha256 = "blake3:…" },
]

[[unit]]
name       = "cnt"
kind       = "module"        # module | package | udp | (Phase3: entity/arch)
lang       = "sv"            # sv | v
generation = 2012            # IEEE-1364-2005 / IEEE-1800-201x
timescale  = "1ns/1ps"       # 이 단위에 유효한 (상속 반영 후) timescale
src_sha256 = "blake3:…"      # 상속 반영 후 전처리 바이트의 다이제스트 (§2 RULE S)
blob       = "units/cnt.vu"
```

각 `<unit>.vu` 헤더(본문과 독립 디코드 가능, 길이 프리픽스):

```
magic              "VITWORKU"
format_version     u32
schema_hash        [u8; 32]   # hdl-ast 형상의 구조적 해시 (§5)
lang / generation  태그
unit_name          문자열
src_sha256         [u8; 32]   # 상속 반영 후 전처리 소스 다이제스트
tool_fingerprint   version + git_sha + dirty + profile  (provenance 전용)
─────── 이하 postcard 본문 ───────
hdl-ast 단위 트리 (포트·파라미터·문장·식·builtin-call 노드 + 소스 스팬)
```

### velab 산출물 — sim-ir 스냅샷 (단일 파일)

완전 elaborate된 **언어 중립** sim-ir를 자기완결 스냅샷으로 직렬화한다. vrun이 다른
입력 없이 이 파일만으로 시뮬레이션할 수 있어야 한다(Xcelium 스냅샷의 "추가 정보 없이
실행 가능" 속성). Icarus `iverilog → .vvp → vvp` 2-바이너리 모델의 `.vvp` 자리다.

`<top>.velab` 헤더(본문과 독립 디코드 가능):

```
magic                 "VELAB\0"
format_version        u32
schema_hash           [u8; 32]   # sim-ir 형상의 구조적 해시 (§5)
composite_input_hash  [u8; 32]   # elaborate 입력 전체의 합성 해시 (§2 RULE B)
global_time_precision 정수배율    # 전체 소비단위 min() + --timescale 기본값 (§4)
consumed              [(lib:unit, src_sha256), …]   # 소비한 단위 트리플 집합
worklib_manifest_hash [u8; 32]   # 단위 추가/삭제/재정렬 무효화용 (§3)
uses_dump             bool       # "설계가 dump-family 태스크를 참조하는가" 중립 사실
tool_fingerprint      provenance 전용
─────── 이하 postcard 본문 ───────
SimIr 루트 1개:
  - 평탄화된 계층/인스턴스
  - net (폭 + 4-state, 0/1/x/z 각 2비트, 벡터 비트팩 순서는 포맷 일부)
  - process (sensitivity + 구조화 basic-block 본문 `Vec<BasicBlock>` + `SuspendState`{resume_pc/locals/join_state/wake_key/call_stack/frame_arena} — 06 "프로세스 실행 모델", 형상 FROZEN 2026-06-02), continuous assign
  - 해소된 초기값(time-0 x/z), builtin-call 노드(+ dump-family 마커)
  - arena/interner 평탄 벡터 (u32 인덱스 엣지 — 재로드 시 포인터 fixup 0)
```

> **엔진-facing 사이드테이블 트레일러(골든 SimIr 프레임 밖, append-only — 현재 13개).** 골든 `SimIr` postcard 프레임 뒤에 out-of-band 트레일러 세그먼트를 `write_velab_file`이 다음 순서로 append한다: ① fork_modes ② net_names ③ timescale(proc_multipliers+global_prec) ④ severities ⑤ radixes ⑥ proc_scopes ⑦ assign_ranks ⑧ queue_bounds ⑨ WorkConsumed(worklib v1; legacy explicit-path 빌드도 항상 기록) ⑩ net_dims(per-element VCD) ⑪ final_procs(P2-E `final`) ⑫ defer_marks ⑬ defer_acts(§16.4 deferred immediate assert). 이들은 `SimOpts`/elaborate IR-0 합성으로 **골든 해시(SimIr 루트)에 무영향** — SVA 체커·named-event·wait fork·frame-call 등 IR-0 기능이 여기 또는 elaborate-합성으로 얹힌다. format_version은 8 유지.

> **SCHEMA_HASH 루트 = `sim_ir::SimIr` (M3 동결, doc 17).** §5의 구조적 해시는 위 `SimIr` 루트(arena 전체를 `Vec`로 by-value 보유 → `Expr`/`Stmt`/`NetVar`/`ConstVal`까지 도달)에서 산출한다. `Process`만으로는 cross-arena u32 엣지라 arena에 미도달 → `Process`는 런타임 클러스터 sub-pin 골든. `Expr`/`Stmt`/`Lvalue`/`Terminator`/`Sensitivity`/`NetVar`/arena 형상은 doc 17이 동결.

> **백엔드 능력은 헤더에 넣지 않는다.** velab 스냅샷은 *해소된 IR 사실*만 담는다.
> "이 엔진이 어떤 builtin을 실행할 수 있는가/dump를 지원하는가"는 스냅샷을 로드하는
> 백엔드(vrun 인터프리터 또는 향후 컴파일드 엔진)의 런타임 능력이므로, elaborate
> 산출물에 박지 않는다. `uses_dump`는 능력 단언이 아니라 중립적 사용 사실이다.

> **프로세스 실행 모델 = basic-block PC 상태기계 (06 결정 2026-06-01 · 하위 형상 FROZEN 2026-06-02).**
> `process` 본문은 바이트코드 ISA가 아니라 **구조화된 basic-block 시퀀스**이며, 재개 상태는 sim-ir
> 스키마에 **물리적으로 예약·동결**됐다. SD1–SD5 하위 결정(스케줄 범위·fork-join·call-frame·op-set·
> wake_key)을 `SuspendState` 공유 형상으로 묶어 **하나의 원자 트랜잭션**으로 freeze한 것이다 — 부분
> 동결 불가(어느 필드든 후일 모양이 바뀌면 SCHEMA_HASH flip → 모든 `.velab` 무효, §5 RULE D2). 근거
> 메모는 06 "프로세스 실행 모델". **동결 형상(모든 필드 order-stable·u32 엣지·span-free·postcard 바이트
> 동일·`usize`/`isize` 금지):**
>
> ```rust
> struct Process {                       // velab 본문 process 노드
>     sensitivity: Sensitivity,          // [정적] 트리거/감지, 정렬 Vec
>     body:        Vec<BasicBlock>,      // [정적/SD3] 구조화 BB; index = resume_pc 도메인
>     entry:       u32,                  // [정적] 진입 BB (initial=t0 1회, always*=재무장)
>     suspend:     SuspendState,         // [재개/예약] RULE D2 원자 동결 단위
> }
> struct SuspendState {
>     resume_pc:   u32,                  // BB 인덱스(포인터/네이티브 PC 금지, operand-stack 누출 없음)
>     locals:      Vec<FourState>,       // 프로세스 frame-0 locals, 순서 Vec
>     join_state:  JoinState,            // [SD1]
>     wake_key:    WakeKey,              // [SD4]
>     call_stack:  Vec<Frame>,           // [SD2] LIFO; 인라인전용=빈 Vec(1바이트); 인라인 vs frame-call 선택 술어는 06 SD2
>     frame_arena: Vec<FourState>,       // [SD2] 프로세스별 flat arena; Frame이 (base,len)로 주소화
> }
> struct JoinState {                     // [SD1] vvp two-set 포팅
>     parent: Option<u32>,               // 런타임 프로세스 테이블 인덱스, None=top-level
>     children: Vec<u32>,                // active 자식, fork 선언순 append; order-preserving 제거
>     detached: Vec<u32>,                // join_any 잔여 + join_none 자식 — NOT optional(load-bearing)
>     flags:    ProcFlags,               // u8 bitset, vvp 1-bit 플래그 미러, bit5-7 예약
> }
> struct Frame {                         // [SD2] 정수인덱스 콜프레임(네이티브 콜스택 아님)
>     return_pc: u32, callee_entry: u32, // 복귀 BB / callee 진입 BB(body 1회 lowering 공유)
>     locals_base: u32, locals_len: u32, // frame_arena 내 locals window
>     is_automatic: bool,                // automatic⇒사설 window, static⇒고정 저장소 alias
> }
> struct WakeKey {                       // [SD4] region 명시 저장
>     cond: WakeCond, region: RegionTag, // region 재유도 금지(해시 밖 로직 차단)
>     tie_break: u32,                    // 평탄계층 선언순 노드 인덱스(사용자-가시 우선순위 아님)
> }
> struct ProcFlags(u8);                                // [SD1] newtype(SchemaHash 형상 명시 — bare u8과 구분); vvp 1-bit 플래그 미러
> enum RegionTag { Active, Inactive, Nba, Monitor }   // [SD4] IEEE 1364 4; 17-region=의도적 Phase-2 flip
> enum WakeCond {                                      // [SD5] process-suspend 조건의 닫힌 6-variant 집합
>     Edge{net:u32,kind:EdgeKind}, Level{nets:Vec<u32>}, WaitTrue{expr:u32},
>     TimeAbs{tick:u64}, NamedEvent{ev:u32}, Join{join_ref:u32},
> }   // 7+1 경계→6 variant 매핑(06 SD5): ⑧ #0 = TimeAbs{now}+region:Inactive(별도 variant 아님),
>     //   ⑥ intra-assign nonblocking = scheduled-assign 이벤트(미-suspend, wake_key 미사용)
> // SD3 Terminator(body 내부): Goto / Branch / Delay / Wait / Fork{children,join,resume_bb}
> //   / Call{target,ret_bb} / Return.  JoinKind{All,Any,None}는 compile-bake 값 → 형상 churn 없음.
> ```

### 직렬화 메커니즘

- **`serde` derive를 경계 trait으로, `postcard 1.x` 단일 인코더.** bincode 폴백 없음(상호
  비호환 artifact = 사실상 두 포맷). 텍스트는 `--dump` **뷰 전용**(RON, full-precision
  float — `$realtime`/timescale 정밀도 손실 방지).
- **다이제스트는 `blake3` 단일 계열.** 형상 해시·소스 해시·매니페스트 해시 모두 blake3
  (순수 Rust, C 의존 없음). `src_sha256` 필드명은 관례적 표기이며 실제 알고리즘은 blake3로
  통일한다(외부 SHA-256 상호운용 요구가 생기기 전까지).
- **원샷 `vita`는 serde를 호출하지 않는다.** 살아있는 `Module`/`SimIr` 값을 메모리로
  그대로 다음 단계에 넘긴다. 직렬화는 vcmp/velab가 "저장하라"고 할 때만 일어나는
  순수 선택적 경계 — 04:45 "디스크 안 남긴다"와 모순 없음.

---

## §2 해시 결합 규칙 (핵심)

> 이 절이 본 문서의 **load-bearing** 부분이다. "변경 없는 단계 스킵"(04:71)이 *건전*하려면,
> 산출물 내용을 바꾸는 **모든** 입력이 그 단계의 staleness 해시에 들어가야 한다. 하나라도
> 빠지면 stale artifact가 조용히 재사용되어 **틀린 시뮬레이션**이 나온다.

**RULE 0 (법칙).** 플래그를 *어느 바이너리가 파싱하느냐*가 아니라 *어느 단계의 출력을
교란하느냐*로 분류한다. 세 버킷:
- **(A)** 전처리 출력을 교란 → **vcmp 단위 소스 해시**에 들어간다.
- **(B)** elaborate된 sim-ir를 교란 → **velab 합성 해시**에 들어간다.
- **(C)** 둘 다 교란하지 않음 → 런타임 전용, **어떤 해시에도 안 들어간다.**

**RULE A (vcmp 소스 해시 입력).** `+incdir+`/`-I`, `+define+`/`-D`, `-y`, `-Y`/`+libext+`,
`-v`, `--std`/`-g<year>`/`-sv` 는 전처리 바이트 또는 단위 해소 결과를 바꾼다 → 단위별
전처리-소스 다이제스트의 **preimage**에 포함한다(헤더에 *stamp*만 하는 것과 구별 —
preimage에 넣어야 바이트가 같아도 dialect 전환이 무효화된다). 전역 dialect 다이제스트는
매니페스트 해시에도 넣어, `--std` 전역 전환이 모든 단위를 무효화하게 한다.

**RULE B (velab 합성 해시 입력).** `-s`/`--top-module`, `-G`/`-pvalue+`, `-P<path>=`
(defparam), `--multi-driver` 정책, `-L` 라이브러리 compose, `--lib-map` 해소 내용,
`--timescale`의 전역 정밀도 결과 는 평탄화된 sim-ir를 바꾼다 → velab 합성 해시에 포함한다.

**RULE C (런타임 전용 — 아무 것도 해시 안 함).** `+plusargs`, `-sv_seed`, `-n`/`-N`,
`--finish-at`, `--log`/`-l`, `-v`/`--verbose`, `-M`/`-m`, `-Wall` 계열, `-o`, `--dump`,
`--color`, `--version`. 런타임 동작 또는 패키징만 바꾼다. 이 중 하나라도 해시에 넣으면
"같은 `.velab`를 런타임 플래그만 달리해 두 번 돌리면 둘 다 유효"라는 보장이 깨지고
불필요한 재실행이 강제된다.

**RULE T (timescale 함정 — 이중 결합).** `--timescale`은 두 해시 모두에 정당하게 들어가는
유일한 플래그다. 주입된 전처리 디렉티브로서 단위별 전처리 소스를 바꾸고(RULE A), velab
헤더에 stamp되는 전역 정밀도의 근원으로서 elaborate된 시간 모델을 바꾼다(RULE B). vcmp
소스 해시와 velab 합성 해시 **둘 다**에 엮어야 한다.

**RULE S (디렉티브 상속 — BLOCKER 차단).** `` `timescale ``·`` `default_nettype `` 같은
**sticky 컴파일러 디렉티브**는 같은 컴파일 단위 안에서 *파일 경계를 넘어* 다음 단위로
캐리오버된다(08-timescale §). 따라서 단위 B의 전처리 출력은 형제 단위 A의 순서에 의존한다 —
B의 *자기 바이트*는 그대로인데 앞 단위가 바뀌면 B가 다른 timescale/nettype을 상속한다.
**단위별 소스 다이제스트는 반드시 "상속 반영 후" 전처리 바이트 위에서 계산한다**(B의
기록된 전처리 바이트가 상속된 `1ns/1ps`·`nettype`을 literal로 포함). 추가로 **정렬된
(파일경로, src_sha256) 컴파일-단위 파일 목록을 매니페스트 해시 입력으로** 둬, 파일
추가/삭제/**재정렬**이 무효화를 일으키게 한다. 미래의 모든 sticky 디렉티브는 동일 취급.
이 정렬된 파일 목록은 **filelist 전개기(§3.1)가 생성**하며, 그 깊이우선 선언순 평탄화의
결정론이 매니페스트 해시 건전성의 전제조건이다.

**RULE D2 (schema 해시도 입력).** 직렬화 타입 형상의 구조적 `SCHEMA_HASH`(§5)는 두 헤더에
모두 들어간다. 타입 형상이 바뀌면 해시가 뒤집혀 이전 모든 `.vu`/`.velab`를 무효화한다.
소스/합성 해시와 **합성**된다(단위가 신선 ⟺ schema_hash 일치 **그리고** 소스 해시 일치).
빌드 지문(git sha/dirty/profile)은 provenance용으로 stamp하지만 **staleness 키가 아니다** —
dirty 트리가 재컴파일을 강제해선 안 된다.

**RULE V (vrun 건전성 — xrun `-R`/`-r`로부터의 의도적 이탈).** vrun은 매 실행마다 **상류
체인 전체를 라이브 소스에 대해 재검증**한다 — 소비한 각 단위의 전처리-소스 다이제스트를
디스크에서 재계산해 스냅샷에 박힌 (lib:unit, src_sha256) 트리플 + 매니페스트 해시와
대조하고, 두 schema_hash도 재확인한다. 라이브 해시가 하나라도 다르면 vrun은 stale
스냅샷을 시뮬레이션하지 않고 **실패**한다(또는 명시적 `--rebuild` 시 stale 단계를 재실행).
**mtime은 절대 쓰지 않는다** — 내용 해시만이 건전한 신선도 신호다. (Xcelium `-R`/`-r`의
검사-생략 패스트패스는 의도적으로 복제하지 않는다.)

**RULE API (오결합을 구조적으로 불가능하게).** vita-artifact의 해시 함수는 **타입화된
입력 구조체**(`PreprocInputs { incdirs, defines, libdirs, libexts, std, … }` /
`ElabInputs { top, param_overrides, multi_driver, lib_bindings, lib_map_resolved,
time_precision, … }`)를 받고 **raw argv를 절대 받지 않는다.** 로깅·패키징·런타임 플래그는
이 구조체에 필드가 없으므로 `-Wall`이나 `--log`를 물리적으로 해시에 넣을 수 없다. 새
전처리/elaborate 플래그를 추가하려면 **구조체 필드를 추가해야 한다**(컴파일 타임 강제 함수).

**RULE F (filelist은 transport, 내용은 해시).** `-f`/`-F` command-file 자체는 bucket C다(파일은
절대 해시 안 함 — 소스를 어떻게 filelist로 묶었는지, `.f`를 개명했는지가 artifact를 무효화하면
안 됨). 그러나 전개된 *내용*은 inline 플래그와 똑같이 디렉티브 타입별로 해시된다(RULE 0): 중첩
`.f` 깊이의 `+define+`은 vcmp 소스 해시에(RULE A), `-L`은 velab 합성 해시에(RULE B). **출처 기반
버킷은 없다** — `.f`는 전송 수단, 내용은 typed 입력. 상세 §3.1.

### 플래그 → 버킷 표 (MVP)

| 플래그 | 단계 | 버킷 |
|---|---|---|
| `+incdir+`/`-I`, `+define+`/`-D`, `-y`, `-Y`/`+libext+`, `-v` | vcmp | A (소스 해시) |
| `--std`/`-g<year>`, `-sv` | vcmp | A (preimage + dialect 다이제스트) |
| `--timescale` | vcmp+velab | A **및** B (RULE T) |
| `--work`/`--workdir` | vcmp | A (lib:unit 키 네임스페이스) |
| `-s`/`--top-module` | velab | B (합성 해시) |
| `-G`/`-pvalue+`, `-P<path>=` | velab | B |
| `-L`, `--lib-map`, `-P<dir>` | velab | B (소비 트리플) |
| `--multi-driver` | velab | B |
| `+plusargs`, `-sv_seed`, `-n`/`-N`, `--finish-at`, `--log`/`-l`, `-o`, `-Wall`, `--dump` | vrun/공통 | C (해시 없음) |

### 실패 케이스 (왜 이게 중요한가)

> `+define+WIDTH=8`을 vcmp 단위 소스 해시에서 빠뜨리면 → 소스 바이트가 같으니 vcmp 스킵 →
> 다른 매크로로 컴파일된 `.vu` 재사용 → `WIDTH=16`으로 돌리려 했는데 8로 시뮬레이션 →
> **조용히 틀린 결과.** RULE A가 이 클래스를 차단한다. RULE S는 같은 사고를 *형제 단위
> 순서*에서 막는다.

---

## §3 멀티 라이브러리 (D3)

상용 EDA(`cds.lib`/`synopsys_sim.setup`)와 GHDL(`--work`/`-P`)처럼, 단위를 **경로가 아니라
논리 `library:unit` 키로 주소화**한다. 설계 단계부터 도입하되 MVP 해소 로직은 단순하게.

> **구현 상태(2026-06-12, P2-A v1):** `vcmp --work <name[=dir]>`/`--workdir` + `velab -L <name[=dir]>
> --top <unit>`(클로저 로딩·first-`-L`-wins) + **vrun RULE-V 자동 게이트**(아래 재검증 알고리즘의
> step 2~3 — 매니페스트 해시·blob 해시·소스/include raw 다이제스트, 불일치=E9003 exit 2) 동작.
> v1 단순화(의도): blob=**CU 단위**(단일-CU concat 모델이라 per-unit 전처리 바이트가 미정의 —
> 모든 unit 엔트리가 자기 CU blob을 가리킴), `src_sha256`=**raw 바이트**+(-D/-I)+include 폐쇄
> (검출력 동등), 매니페스트=정규형 텍스트(strict 파서, E9005). `-y`/`-v`/`-P`/`--lib-map`·
> per-unit blob 분리·vrun `-L` 재배치는 후속.

**플래그 표면:**

| 플래그 | 단계 | 의미 |
|---|---|---|
| `--work <logical>[=<dir>]` | vcmp | 분석 결과가 들어갈 논리 work 라이브러리 이름(기본 `work`) + 출력 디렉터리 |
| `--workdir <dir>` | vcmp | `--work`로 함의되지 않은 출력 디렉터리 (GHDL `--workdir` 계열) |
| `-y <libdir>` / `-Y <ext>`·`+libext+<ext>` | vcmp | 미정의 모듈 자동 해소용 소스 검색 디렉터리 + 확장자 |
| `-v <libfile>` | vcmp | 단일 파일 on-demand 소스 라이브러리(인스턴스화될 때만 편입). *MVP는 예약 — 멀티-lib corpus가 생기면 활성* |
| `-L <logical>[=<dir>]` | velab | 이미 컴파일된 논리 라이브러리를 이름으로 바인딩(compose; Xcelium `-reflib`/VCS `-L` 계열) |
| `-P <dir>` | vcmp+velab | 사전컴파일 라이브러리 검색 디렉터리(GHDL `-P`). analyze·elaborate 양쪽에서 일관 해소 |
| `--lib-map <file>` | velab(+vcmp) | 논리명→디렉터리 매핑 + 체이닝 + 검색순서를 담은 외부 설정 파일(`cds.lib`/`synopsys_sim.setup` 계열). **해소된 내용**이 해시됨 — 논리 lib 재지정이 인스턴스를 silently 재바인딩하므로 |

**불변식:**
- `-L`/`-reflib` 소비자는 외부 라이브러리의 **소비 트리플**(lib:unit, src_sha256)을 스냅샷
  입력 집합에 해시해야 한다 — 로컬 work lib만 보면 안 된다.
- `work/lib.toml` **내용 해시**가 단위 추가/삭제/재명을 무효화한다(§1 매니페스트의 `files`·
  `[[unit]]` 목록 포함).
- `INCLUDE`는 **설정 파일 내부의 체이닝 키워드**이며 CLI 플래그가 아니다. 명령줄에는
  `--lib-map`만 노출한다(중복 제거).

### §3.1 Filelist (`.f` / `-f`, `-F`) 전개

> **구현 상태(2026-06-10, v1 서브셋):** `-f`/`-F` argv-레벨 in-place 전개가 전 applet에서 동작 —
> 중첩 재귀·사이클 가드(lexical ∪ 물리 identity)·depth cap 256·주석/`\` 잇기·`$VAR` env(미정의=E8006)·
> glob 거부(E8004)·`W-FLIST-MIXED-BASE` lint·프레임별 베이스 해소. **2탄(같은 날): typed 버킷 합류** —
> `-D NAME[=VAL]`/`-I dir` + `+define+N=V+M`(verbatim — 경로 해소 금지)/`+incdir+a+b`(세그먼트별
> 프레임 베이스 해소) → `PreOpts{cli_defines, incdirs}`(전처리기 측은 기존재) · `E-FLIST-WRONG-STAGE`
> (velab/vrun에 전처리 버킷=exit 3) · `W-FLIST-OVERRIDE`(단일값 knob 재지정=always-logged 경고+last-wins).
> **잔여(Phase-1.x):** `E-FLIST-DUP-CTX-CONFLICT`(sticky 디렉티브 도입 시), `--dump-filelist`,
> 매니페스트 anchor(§1 work-lib와 함께).

대규모 프로젝트는 수백 개 RTL 경로 + `+incdir+`/`+define+`을 명령줄에 나열하지 않고 **filelist
(`.f`)** 로 집계한다. `.f`는 argv처럼 토큰화돼 참조 지점에 in-place 전개되므로 명령줄에서 합법인
모든 플래그가 `.f` 안에서도 합법이다. vcmp·velab·vrun·vita 모두 수용한다.

**두 플래그, 한 차이 — 경로 해소 베이스:**
- `-f <file>` — 안의 모든 *상대* 경로(소스, `+incdir+`, `-y`/`-v`/`-P` dir, 그리고 중첩 `-f`/`-F`
  타깃)를 **invocation CWD**(프로세스 시작 dir, 한 번 캡처, 깊이 무관) 기준으로 해소.
- `-F <file>` — 동일하되 상대 경로를 **그 `.f` 파일이 있는 디렉터리** 기준으로 해소. `-F` 트리는
  완전 재배치 가능(벤더 IP가 `-F`로 배포하는 이유). 베이스는 프레임마다 재-anchor.
- 절대 경로는 양쪽 모두 베이스 무관. `-f`/`-F` 자체는 **bucket C**(파일은 해시 안 함). `-c`
  동의어는 두지 않는다(정규 `-f`/`-F` 한 쌍).

**중첩·재귀 (top-down 집계):** `.f` 안의 `-f`/`-F` 줄이 다른 `.f`를 *그 위치*에 in-place 전개하며
임의 depth로 재귀한다. 평탄화된 토큰 스트림 = `.f` 포함 트리의 **깊이우선 pre-order 선언순**
순회(명령줄이 frame 0). 그래서 명령줄의 `+define+`과 세 단계 깊이의 `+define+`이 단일 평탄
스트림에서 결정론적 위치를 갖는다.

```
expand(path, base_mode):
  canon = lexical_normalize(resolve(path, base_mode, invocation_cwd))   # symlink 해소 안 함 (해시/경로용)
  if canon ∈ active_stack OR phys_id(canon) ∈ active_phys:             # phys_id=dev+inode / Windows file-id
      error E-FLIST-CYCLE (체인 출력)                                   # symlink 루프도 DEPTH 아닌 CYCLE로 정확 진단 (phys_id는 진단 전용, 해시 미투입)
  push canon; push phys_id(canon)
  for tok in lex(read(canon)) in file order:
    if tok == `-f T`: splice expand(T, CWD-relative)        # invocation_cwd anchor
    elif tok == `-F T`: splice expand(T, dir(canon)-relative)
    else: 상대 경로를 이 프레임 base로 해소 → typed token 방출
  pop canon; pop phys_id(canon)
```

베이스는 "프레임에 *어떻게 진입했나*"의 속성이며 전이 상속이 아니다 — 각 `-f`/`-F` 토큰이
타깃 프레임의 베이스를 새로 정한다. 깊이는 사실상 무제한(사이클 가드 + backstop depth cap
256 = `E-FLIST-DEPTH`).

> **사이클 가드는 물리 identity로도 검사(잠금).** lexical canon만으로 active-stack 멤버십을 보면
> symlink로 자기 자신을 가리키는 루프(`a` → 링크 → `a`의 실경로)는 canon이 달라 사이클 가드를
> 빠져나가 depth cap에서 `E-FLIST-DEPTH`로 *오보*된다(잘못된 처방 유도). 이를 막기 위해 active-stack
> 멤버십은 **lexical canon ∪ 물리 identity(dev+inode / Windows file-id)** 두 키로 검사하고, 둘 중
> 하나라도 스택에 있으면 `E-FLIST-CYCLE`로 정확히 진단한다. **물리 identity는 해시·매니페스트 경로에는
> 투입하지 않는다**(그건 lexical-only 유지 → 3-OS 바이트 결정성 보존). identity 검사는 진단 정확도 전용.

> **LINT `W-FLIST-MIXED-BASE`(경고).** `-F` 프레임 안의 `-f` 줄은 재배치 가능 서브트리를
> invocation CWD에 re-anchor하므로 거의 항상 벤더 버그다. 의미는 유효하나 경고로 표면화.

**canonicalization (잠금 — 3-OS 결정론 필수, ergonomics 아님):** 베이스로 해소한 뒤 dedup/해시
전에 정규화하되 — **(1) case-fold 안 함**: 모든 OS에서 경로를 대소문자 구분 바이트열로 취급
(case-only 차이는 모든 OS에서 두 identity; macOS의 실제 case 충돌은 `E-FLIST-NOT-FOUND`로
표면화). **(2) symlink 해소 안 함**: identity/dedup/매니페스트 경로는 순수 lexical `.`/`..`
정규화만 — `std::fs::canonicalize`(symlink 해소 + OS별 차이)를 쓰지 않는다. 이 둘이 깨지면
같은 `.f`·같은 바이트가 OS마다 다른 매니페스트 해시를 내어 RULE S/§5 "3-OS 바이트 동일"이
무너진다.

**매니페스트 경로 = build-root anchor 상대 (절대 경로 금지):** `work/lib.toml`의 `files[].path`는
**build root 상대**로 저장한다. anchor 결정 우선순위는 **① 명시 `--root <dir>`(CI 권장) → ②
git/프로젝트 루트 자동탐지(`.git` 등 상향 탐색) → ③ invocation CWD(폴백)**. anchor는 *소스 트리
루트*여야 체크아웃 독립이 된다(FuseSoC/Bender 모델) — 절대 경로를 박으면 동일 소스의 두
체크아웃(`/home/a/proj` vs `/home/b/proj`)이 다른 매니페스트 해시를 내어 RULE V 재사용이
깨진다. 파일 *접근*은 프레임 베이스(`-f`/`-F`) 해소+canonical로, *저장*은 anchor 상대 재표현
으로 한다. anchor 자체는 해시하지 않고 상대 경로만 해시하므로, anchor가 달라도 상대 경로가
같으면 매니페스트 해시가 같다(§1의 `path = "rtl/a.sv"`와 일관). (work-dir anchor는 소스가 work
밖일 때 `../` 누적 + work 위치 의존으로 비결정 위험이 있어 채택하지 않는다.)

**허용 inline 디렉티브 + RULE API 정규화:** 소스 경로, `+incdir+`/`-I`, `+define+`/`-D`,
`-y`/`-Y`/`+libext+`/`-v`, `--work`/`--workdir`/`-P`/`--lib-map`, `-L`, `--std`/`-g`/`-sv`,
`--timescale`, `-s`/`--top-module`, `-G`/`-P<path>=`, `--multi-driver`, 중첩 `-f`/`-F`, bucket C
(`+plusargs`/`--log`/`-Wall`). **"파일에서 왔음" 버킷은 없다** — 모든 디렉티브가 inline과
*동일한* typed `PreprocInputs`/`ElabInputs`로 정규화된다(한 코드 경로). 그래서 `-f`-출처
`+define`과 inline `+define`은 downstream에서 물리적으로 구별 불가 → 해싱에서 발산 불가.

> **wrong-stage 에러(silent no-op 금지).** 각 단계의 `.f` 전개기는 전체 토큰 문법을 파싱하지만
> (같은 `.f`가 어디서나 파싱됨), 호출 단계에 속하지 않는 버킷의 디렉티브는 **hard error
> `E-FLIST-WRONG-STAGE`**다. 예: `velab -f x.f`인데 `x.f`에 `+define+`(전처리 버킷)이 있으면
> — velab엔 전처리 패스가 없어 silently 무시될 위험을 막아, "preprocessor 디렉티브
> `+define+WIDTH`는 vcmp 단계 플래그 — elaborate에서 무효"로 거부한다.

**`.f`의 본분 = 소스 파일 + include path (+ define·라이브러리 설정).** `.f`는 최소한 컴파일할
**소스 파일 목록과 `+incdir+` include 검색 경로**를 담아야 하며(대규모 프로젝트의 핵심 용도),
`+define+`·`-y`/`-v`/`-L`/`--work` 같은 파일/라이브러리 스코프 디렉티브도 정상 내용이다. 반면
`--top-module`/`-s`, `--std`, `--timescale`, `--multi-driver` 같은 **단일값 elaborate 제어
knob을 `.f`에 넣는 것은 비권장**한다(빌드 의도를 명령줄에 두는 게 추적 가능). 금지는 아니지만
충돌 시 아래 경고로 표면화된다.

**충돌·우선순위 (last-wins + 명령줄 끝 append).** 멀티값 디렉티브(`+define+`/`+incdir+`/`-y`/
`-L`/소스)는 *누적*(`+define+`은 이름별 last-wins, `+incdir+`은 순서 리스트). 단일값 knob
(`--top-module`/`--std`/`--timescale`/`--multi-driver`)이 두 곳 이상에서 지정되면 **평탄
스트림의 마지막 값이 이긴다**. **명령줄 토큰은 모든 `-f`/`-F` 전개 뒤(스트림 끝)에 append**
되므로, 결과적으로 **명령줄이 `.f`를 override**하되 규칙은 "last-wins" 하나로 설명된다.

> **필수 경고 `W-FLIST-OVERRIDE` (항상 로깅).** 단일값 knob이 두 곳 이상에서 지정돼 override가
> 일어나면 **반드시 경고를 로그에 출력**한다 — silently 진행하지 않는다. 경고는 두 값, 각
> 출처(`build.f:3` vs 명령줄), 이긴 값을 보여준다: `W-FLIST-OVERRIDE: --top-module 'b'
> (vendor.f:3) overridden by 'a' (command line)`. 이 경고는 always-logged spine이라 `-q`로도
> 억제되지 않는다(13-diagnostics-and-logging.md). hard error가 아니라 진행 + 경고 — 명령줄
> override 워크플로(`velab -s top2 -f build.f`)는 정상 동작으로 두되, 의도치 않은 충돌은
> 사용자에게 큰 소리로 알린다.

**syntax:** 주석 `//`·`/* */`·줄머리 `#`; 줄 끝 `\` 잇기; whitespace/newline 구분자;
`+`-조인 멀티값(`+incdir+a+b`); env `$VAR`/`${VAR}`/`$(VAR)`(미정의 = `E-FLIST-UNDEF-ENV` —
silent 빈 문자열 금지). **glob/wildcard 금지(`E-FLIST-GLOB`)** — readdir 순서가 플랫폼마다
불안정해 RULE S 정렬을 비결정론으로 만든다. glob이 필요하면 생성기가 명시적 정렬 `.f`를 낸다.

**소스 dedup 금지 (BLOCKER 차단):** 소스 파일은 **기본적으로 dedup하지 않는다.** 같은 모듈이
두 번 나오면 parse 단계 재정의 에러(`E-DUP-UNIT`, 표준 EDA 동작)다. 이유: sticky 디렉티브
캐리오버(RULE S) 때문에 두 occurrence는 *서로 다른 상속 컨텍스트*에 있어 같은 컴파일 입력이
아니다 — first-occurrence dedup은 한 상속 컨텍스트를 silently 골라 다른 쪽을 떨군다. 같은
canonical 경로가 두 번 나오는 경우만 dedup하되, 두 occurrence가 **동일한 상속 sticky-directive
상태**로 해소돼야 하며 아니면 hard error `E-FLIST-DUP-CTX-CONFLICT`(양쪽 상속 컨텍스트 제시).
디렉티브는 누적된다(`+incdir+`는 순서 리스트, `+define+`는 이름별 last-wins).

**사이클:** active-stack(현재 열린 `.f`의 canonical 경로)에 이미 있는 경로로 `-f`/`-F`가 해소되면
`E-FLIST-CYCLE`(전체 체인) hard error — silent 스킵 금지(평탄 순서가 순회 우연에 의존하게 됨).
다른 가지에서 두 번 도달(diamond)은 사이클 아님(첫 방문 후 pop됨) — 정상 누적.

**전개기 위치:** filelist 전개는 `cli`/입력-조립 계층에서 typed `PreprocInputs`/`ElabInputs`를
생성한 뒤 `hdl-preprocess`가 *해소된 소스만* 보게 한다 — `--lib-map` 해소가 cli/vita-artifact
관심사인 것과 동형(`hdl-preprocess`의 leaf 의존 방향 보존).

**`--dump-filelist` (dry-run 디버그, MVP 포함).** 컴파일 없이 **완전 평탄화·canonical·정렬된
소스 목록 + 해소된 typed 입력 + 각 항목의 버킷**을 출력한다(`xrun.history`의 vitamin 버전).
각 소스는 `(origin .f:line, base=CWD|file-dir, canonical-path)`로, 디렉티브는
`(이름, 값, 버킷, 출처)`로 표기 — 깊은 중첩 `.f`의 RULE S 순서·경로 해소·wrong-stage·override를
사람이 검증할 수 있게 한다. bucket C(해시 무관), 어떤 단계/`vita`에서도 사용 가능.

---

## §4 vrun 전체 체인 재검증 + vita 동치

### vrun 재검증 알고리즘 (RULE V 구체화)

```
vrun <top>.velab:
  1. 헤더만 디코드 (본문 역직렬화 전): magic, format_version(현재 8), schema_hash 확인
     → format/schema 불일치면 hard error + 재빌드 힌트 (본문 안 읽고 거부)
  2. 스냅샷의 consumed[(lib:unit, src_sha256)] 각 항목에 대해:
       라이브 소스를 재전처리(상속 반영) → 다이제스트 재계산 → 박힌 값과 대조
  3. work 매니페스트 라이브 내용 해시 재계산 → worklib_manifest_hash와 대조
  4. 하나라도 불일치:
       기본: 실패 ("상류가 stale — vcmp/velab 재실행 또는 vrun --rebuild")
       --rebuild: stale 단계만 재실행 후 진행
  5. 전부 일치: 본문 역직렬화 → sim-engine이 SimIr를 walk
```

mtime 비교는 (있다면) 해싱 전 빠른 프리필터로만 쓰고, 단독 신호로는 절대 쓰지 않는다.

### vrun 재검증 입력 출처 (RULE V 입력 — 비가역 해시 보완)

재검증(step 2–3)은 라이브 소스를 다시 전처리·해싱해야 하므로 **소스 파일 목록·`+incdir+`·라이브러리 위치**가 필요하다. `.velab` 헤더는 비가역 해시만 담고 원본 argv(`-f`/`+incdir+`)를 보관하지 않으므로, 이 입력은 다음에서 얻는다:

- **work 매니페스트가 1차 출처.** `consumed[(lib:unit, …)]`의 각 라이브러리는 vcmp가 만든 `work/lib.toml`을 가리키고, 거기에 정렬된 `files = [{path, src_sha256}]`와 dialect가 있다. 재전처리·재해싱은 이 매니페스트의 경로·include 기록을 사용한다 — 원본 RTL argv를 통째로 재입력할 필요가 없다.
- **라이브러리 위치 해소.** `lib:unit`의 논리명→디렉터리는 D3 lib-map(`cds.lib`/`synopsys_sim.setup` 계열)으로 해소한다. `vrun`에 `-L`/`--lib-map`을 재전달하거나(또는 `-f` filelist), 매니페스트에 기록된 기본 위치를 쓴다.
- **bare `vrun top.velab`** 는 work 라이브러리가 기록된(또는 기본) 위치에 그대로 있을 때 동작한다. 위치가 옮겨졌으면 `-L`/`--lib-map`으로 재지정한다.
- `--rebuild`는 같은 출처로 stale 단계의 vcmp/velab를 재구성한다(추가 입력 불요).

### vita 인메모리 동치

원샷 `vita`는 살아있는 `Module`/`SimIr` 값을 메모리로 스트리밍하며 **serde 호출 0회,
디스크 0회, staleness 해시 계산 0회**다(디스크 산출물이 없으니 stale될 대상이 없다).
의미상 `vcmp→velab→vrun` 마이너스 디스크와 동일하다.

> **전역 정밀도 동치 불변식.** `global_time_precision = min(전체 소비단위의 유효
> timeprecision)` + `--timescale` 기본값이며, **velab 합성 해시의 일부**다. **소비단위 중 어떤
> 것도 timescale을 지정하지 않고 `--timescale`도 없으면**(min()의 공집합) → 08의 기저값
> `1ns/1ns`로 해소하고 `W-PP-TIMESCALE-DEFAULT`(W1017, §15 부록 A) 경고를 발화한다. 기저값은 상수라 합성 해시·
> 3-OS 결정성을 깨지 않는다. `vita`가 staged
> 흐름과 의미상 같으려면 동일 argv에서 **동일한 소비-단위 집합**을 구성하고 그 집합 위에서
> 같은 min()을 계산해야 한다. 소비 트리플 집합 `(lib:unit, src_sha256)`이 "설계"의 정규
> 정의이며, 두 경로 모두 이 집합을 참조한다. 동치 검증은 `vita`와 `vcmp+velab+vrun`의
> **VCD + stdout 관측 동치**(바이트 동일이 아니라 관측 동일)로 한다(09 테스트).

---

## §5 구조적 schema 해시 메커니즘 (D2)

03의 "cargo가 유일한 빌드 진입점 — cmake/make/별도 빌드 스크립트/codegen 없음" 규칙을
지키면서 구조적 해시를 산출하는 **3계층** 메커니즘. build.rs 셸아웃 없음, codegen 없음.

**Layer 1 — 구조적 schema 해시 (런타임 staleness/디코드 키).** 신규 proc-macro 크레이트
`vita-artifact-derive`가 `#[derive(SchemaHash)]`를 제공한다. proc-macro는 rustc *안*에서
도므로 100% cargo-native(`no build.rs`가 갖지 못하는 깨끗한 탈출구). derive는 타입의 syn
AST(필드명 + 필드 타입 + variant 형상)를 참여 타입 레지스트리 위에서 재귀적으로 walk해
정규 형상 문자열로 만들고 `const SCHEMA_HASH: [u8;32] = blake3(shape)`를 emit한다. hdl-ast
루트 타입(→ `.vu`)과 sim-ir 루트 타입(→ `.velab`)이 이를 derive하고, vita-artifact가 그
const를 읽어 헤더에 stamp한다. 필드/variant 추가·삭제·재정렬·타입변경이 해시를 뒤집어,
호환 안 되는 타입 형상으로 빌드된 도구는 silent misparse 대신 디코드 시점에 깨끗한
"호환 안 되는 도구로 재빌드됨 — 재컴파일" 오류를 낸다.

> **결정론 필수.** derive 구현은 **순서 안정 구조만** 사용한다(Vec/BTreeMap, syn의 소스
> 순서 필드/variant). 레지스트리는 타입명 사전식 정렬 등으로 **결정론적 순서**로
> 정규화하며, **HashMap/HashSet 반복을 절대 쓰지 않는다.** 정규 형상 문자열과 그 blake3는
> OS/arch/툴체인 호출에 무관하게 바이트 동일해야 한다(3-OS 계약 + `--locked` 재현성). CI는
> 두 플랫폼에서 같은 fixture 타입의 `SCHEMA_HASH`가 같음을 검증한다.

> **process 노드 동결 불변식 (SD1–SD5, FROZEN 2026-06-02).** `SuspendState`와 그 하위 타입
> (`JoinState`/`Frame`/`WakeKey`/`WakeCond`/`RegionTag` + `body`의 `Terminator`)은 **하나의 원자 단위로
> freeze**된다 — 어느 필드든 후일 형상이 바뀌면 `SCHEMA_HASH`가 flip돼 모든 `.velab`가 무효화되므로 부분
> 동결은 불가. 추가 불변식: **(1) `usize`/`isize` 절대 금지** — 32/64-bit에서 폭이 달라져 3-OS 바이트를
> 깬다(`Vec` 길이 prefix는 varint라 안전). **(2)** `children`/`detached`/`call_stack`/`frame_arena`는
> **선언순 append + order-preserving 제거**(`swap_remove`·freed-slot 재사용 풀 금지) — 타입이 아닌 엔진
> 규율 + 2-platform CI로 강제. teardown: `disable fork`는 `detached`만, 일반 `disable <scope>`는
> `children`(suspended·join-blocked 자식 포함)까지 재귀(06 SD1). **(3)** `WakeKey.region`은 elaborate 시점에 채워 **저장**하고 wake 시
> 재유도하지 않는다(스케줄을 해시 안에 둠). **(4)** 새 wait 종류·17-region 확장은 **의도된 `SCHEMA_HASH`
> flip + rebuild**가 유일한 정상 경로다(미리 variant 예약 금지 — 해시 표면 bloat). CI fixture에 fork-reap·
> delta 재진입·재귀 frame 케이스를 포함한다.

> **wire 민감성.** syn 형상 해시는 Rust 형상은 보지만 정확한 postcard *wire* 인코딩은 못
> 본다 — `#[serde(rename)]`/`skip`/`with`/`default`/`tag` 같은 serde 속성은 형상을 안
> 바꾸고도 바이트를 바꾼다. 따라서 derive는 **serde 컨테이너/필드 속성도 shape 문자열에
> 포함**해, wire에 영향을 주는 편집이 런타임에 `SCHEMA_HASH`를 뒤집게 한다(런타임 경로에서
> 갭을 닫음 — Layer 3에만 의존하지 않음).

**Layer 2 — 빌드 지문 (provenance, staleness 키 아님).** 모든 `.vu`/`.velab` 헤더와
`--dump` 출력에 git sha + dirty + profile + 도구 버전을 stamp한다.
`option_env!("VITA_GIT_SHA")`, `option_env!("VITA_GIT_DIRTY")`, `env!("CARGO_PKG_VERSION")`,
`cfg!(debug_assertions)`로 읽는다. CI/설치 래퍼/dist가 빌드 시 env로 주입하고, 평범한
로컬 `cargo build`에는 없으면 "version+profile, sha=unknown"으로 graceful 강등. **빌드
스크립트 0개.** 지문은 버그리포트 provenance 전용이며 staleness 키에서 의도적으로 제외한다
(staleness = SCHEMA_HASH + 전처리-소스 트리플 + 매니페스트 해시). 평범한 빌드에서 sha가
*꼭* 필요해지면, 허용되는 단 하나의 경계 예외는 셸아웃 없는 `vergen-gix` build.rs(순수
Rust `gix`, 외부 git 프로세스 없음) — 문서화된 최후 수단이며 env 주입 경로가 우선.

**Layer 3 — CI 골든-포맷 가드 (의도적 변경 리뷰, 테스트 전용).** `serde-reflection`은 샘플
값이 필요한 런타임 tracer라 헤더를 stamp할 수 없다(codegen으로 엮으면 "no codegen" 위반).
올바른 역할은 워크스페이스 **테스트**다 — hdl-ast·sim-ir의 serde 포맷 골든 Registry(RON)를
커밋하고, `cargo test`가 재추적해 골든과 diff, wire 포맷이 표류하면 CI 실패. Layer 1이
런타임 staleness를 게이트하고, Layer 3이 포맷 변경의 의도적 인간 리뷰를 게이트한다 —
함께 형상 편집과 wire 편집을 모두 커버.

**호환성 규칙:** format_version/schema_hash/tool-semver-major 불일치 시 **hard error,
silent 재사용 금지.** 진단은 원인 + 실행 가능한 재빌드 힌트를 제시
("work 라이브러리는 vitamin X(schema N) 산출 — 현재 Y(schema M); `vcmp`/`velab` 재실행
또는 `vcmp --clean`"). 재생성이 항상 가능하므로 정책은 version-MIGRATE가 아니라
version-GATE(refuse-and-rebuild). 마이그레이션 기계는 artifact가 배포 포맷이 되기 전까지 연기.

---

## §6 CLI 표면

GHDL의 `a`/`e`/`r` 3-verb 모델이 `vcmp`/`velab`/`vrun`에 1:1 대응하고, 원샷 `vita`는 그
union이다(xrun/GHDL `--elab-run`). iverilog/Verilator/VCS/GHDL 사용자가 익숙하도록 별칭을
받는다. 각 플래그의 **해시 결합 버킷**(§2)을 함께 명시한다.

**공통 (모든 단계 + vita):** `-f <file>` / `-F <file>` — filelist 전개(`-f`=CWD 상대, `-F`=파일-
디렉터리 상대 경로 해소; 중첩·재귀 top-down). 버킷 C(파일 자체) — **전개 내용은 디렉티브
타입별로 A/B/C**. 호출 단계에 안 맞는 디렉티브는 `E-FLIST-WRONG-STAGE`, 단일값 knob 충돌은
`W-FLIST-OVERRIDE`. `--dump-filelist`(평탄화·정렬 결과 dry-run 출력). 상세 §3.1. ·
로그는 **상시 자동 기록**(`<stage>.log`; `--log <file>`/`--log-dir`/`--no-log`로 제어),
`-q`/`-v`/`-vv`, `-Wno-<code>`/`-Werror[=<code>]`, `--color` 등 진단/로깅 플래그는 전부 버킷 C
(상세 [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md)).

### vcmp (compile)

| 플래그 (정규 / 별칭) | 의미 | 버킷 |
|---|---|---|
| `+incdir+<dir>` / `-I<dir>` | `` `include `` 검색 경로 | A |
| `+define+<name>[=<val>]` / `-D<name>[=<val>]` | 전처리 매크로 | A |
| `-y <libdir>` | 미정의 모듈 자동 해소 디렉터리 | A |
| `-Y <ext>` / `+libext+<ext>` | `-y` 디렉터리에서 고려할 확장자 | A |
| `-v <libfile>` | 단일 파일 on-demand 라이브러리 (예약) | A |
| `--work <logical>[=<dir>]` / `--workdir <dir>` | 논리 work 라이브러리 + 출력 디렉터리 (D3) | A |
| `--std <2005\|2012\|…>` / `-g<year>`, `-sv` | 언어 세대 / Verilog-vs-SV dialect | A (preimage) |
| `--timescale <unit>/<prec>` | `` `timescale `` 없는 모듈 기본값 | A+B (RULE T) |
| `--timescale-policy <strict\|lenient>` | 부분 지정(일부만 `` `timescale ``) 정책. 기본 lenient(W2016), strict는 E1011 (§15) | A |
| `-E` | 전처리만 하고 정지 — 전개 텍스트를 **stdout**(또는 `-o <file>`)으로 방출. staged artifact 아님(`work/`·`.velab` 미생성), 동치 모델 밖. gcc/iverilog `-E` 관행 | — (해시·동치 모델 밖) |
| `-o <work-dir>` | 출력 디렉터리 명명 (단, `-E`와 함께면 전처리 텍스트 파일 경로) | C |

### velab (elaborate)

| 플래그 (정규 / 별칭) | 의미 | 버킷 |
|---|---|---|
| `-s <top>` / `--top-module <top>` | elaborate 루트 단위 | B |
| `-G<name>=<val>` / `-pvalue+<name>=<val>` | top-level 파라미터 오버라이드(clean scoped) | B |
| `-P<hier.path.param>=<val>` | defparam 식 계층 파라미터 오버라이드 | B |
| `-L <logical>[=<dir>]` | 컴파일된 논리 라이브러리 compose (D3) | B |
| `-P <dir>` | 사전컴파일 라이브러리 검색 디렉터리 | B |
| `--lib-map <file>` | 논리명→dir 매핑 설정 파일(해소 내용 해시됨) | B |
| `--timescale <unit>/<prec>` | 전역 기본 시간 정밀도 | B |
| `--multi-driver <warn\|error>` | 다중구동 *검출* 시 심각도(해소는 항상 IEEE 4-state z-merge) | B |
| `-o <top>.velab` | 스냅샷 산출물 명명 | C |

### vrun (simulation)

| 플래그 (정규 / 별칭) | 의미 | 버킷 |
|---|---|---|
| `+<name>[=<value>]` | plusargs (`$value$plusargs`/`$test$plusargs`) | C |
| `-sv_seed <N\|random>` | `$urandom`/`$random` 시드 (재현성; 로그에 기록, 해시 안 함) | C |
| `-n` / `-N` | `$stop` 처리: `-n`=`$finish`처럼 클린 종료, `-N`=추가로 exit 1 (CI 실패 감지) | C |
| `--finish-at <time>` | 실행 길이 제한 (강제 `$finish`) | C |
| `--log <file>` / `-l <file>` | 런타임 transcript sink (`-`=stderr; `-l`은 vrun 한정 vvp 호환 별칭) | C |
| `-v` / `--verbose` | 런타임 진행 출력 | C |
| `-M <dir>` / `-m <module>` | VPI/플러그인 모듈 검색·로드 (예약, post-MVP — MVP는 builtin 정적 링크) | C |

> **VCD는 RTL 주도가 기본이자 MVP 전부.** 단일 실행에서 dump 시스템 태스크
> (`$dumpfile`/`$dumpvars` …)가 호출되지 않으면 VCD는 생성되지 않는다(no-op). RTL 수정 없이
> 강제/리다이렉트하는 `vrun --force-dump`/`+dumpfile`은 spec §7(147행) 방침대로 **선택적
> 후속 기능으로 연기**한다. velab 측 `--trace` 플래그는 두지 않는다(Verilator와 의도적 이탈).

### vita (one-shot)

`vita = UNION(vcmp, velab, vrun)` 한 번 호출. 위 모든 플래그를 받고 preprocess→…→VCD를
메모리로 스트리밍(serde/디스크 없음). `-E`는 elaborate 전에 short-circuit. **staleness 해시를
하나도 계산하지 않는다**(디스크 산출물이 없으므로).

### 플래그 충돌 / 별칭 정규화

| 충돌 | 정규 선택 |
|---|---|
| `-l` (iverilog=lib-file vs vvp=logfile) | lib-file은 `-v`로, log는 `--log`(+ vrun 한정 `-l` 별칭) |
| `-g<year>` 세대 vs `-g<NAME>=<VALUE>` VHDL generic | generic 오버라이드는 `=` 필수로 구분(VHDL은 Phase 3 예약) |
| `-P<dir>` 검색 vs `-P<path>=val` defparam | `=` 유무로 구분; **`-P`는 defparam 전용으로 한정**하고 compose는 `-L`, 검색은 `-y`/`--lib-map` 우선 |
| `-G` vs `-P` 파라미터 | `-G`=clean scoped 오버라이드, `-P<path>=`=defparam 식 |

별칭(무마찰 포팅): `+incdir+`↔`-I`, `+define+`↔`-D`, `-s`↔`--top-module`, `-G`↔`-pvalue+`.

> **04에 추가할 표.** 04 "실행 모델" 절 뒤에 바이너리별 플래그 표(정규/별칭/의미/**버킷**
> 열 필수)를 두되, 상세·충돌·규칙은 본 문서가 권위. 버킷 열을 필수로 둬 미래 플래그 추가가
> 반드시 결합을 명시하게 강제한다.

---

## §7 진단 위치 사이드테이블 (D4)

`sim-ir`는 **span-free**다 — Verilog 구문도, 파일 경로도 모르는 언어 중립 좁은 표면을
유지한다(04:131). 런타임 진단(`$fatal` at line 42, 런타임 range 위반 등)이 소스를 가리킬 수
있도록, 위치 정보는 **별도 경로**로 운반한다(Yosys RTLIL이 `src` 어트리뷰트로 위치를
오버레이하는 선례).

- **사이드테이블**: `node_index → { file_id, byte_range }` 매핑. 선택적이며(release
  스냅샷에선 생략 가능) **독립 버전**이다 — sim-ir schema_hash와 별개 축으로 진화.
- **`file_id → path` 맵**: `sim-ir` 타입이 아니라 work 매니페스트에 둔다.
- sim-ir 본문(언어 중립 코어)만 schema_hash 대상이며, 사이드테이블은 독립적으로
  해시/버전된다. 이로써 SchemaHash derive가 중립 코어만 해시하고, 진단 위치가 백엔드
  교체 경계를 넓히지 않는다.
- **소비자**: `vita-log`가 런타임 진단을 그릴 때 이 사이드테이블을 읽는다 — builtin-call
  IR 노드 인덱스 → `{file_id, byte_range}` → 매니페스트 `file_id→path` → SourceLoc 복원 →
  `diag`에 넘겨 compile 에러와 동일한 caret로 렌더. 상세
  [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md).
- **strip 정책 (기본 포함 + 명시 strip + 로드 경고).** 사이드테이블은 **기본적으로
  스냅샷에 포함**(개발 친화 — 런타임 진단이 항상 file:line을 가리킴)된다. 제거는 명시적
  opt-out으로만 한다 — `velab --strip-locations`(또는 release 배포 프로파일). strip된
  스냅샷을 `vrun`이 로드하면 **로드 시점에 1회 경고 `W-RUN-NO-LOCATIONS`**("이 스냅샷은
  위치 정보가 없어 런타임 진단이 file:line을 못 가리킴 — 위치 포함으로 재-elaborate
  권장")를 낸다. 이 경고는 `-Wno-W-RUN-NO-LOCATIONS`로 끌 수 있다. 진단이 실제 터질 때만
  알리는 침묵 방식은 (특히 비결정 실패에서) 너무 늦으므로 채택하지 않는다 — 단, 진단
  발생 시에도 위치만 "(불가)"로 graceful degrade하고 코드·severity·sim_time은 출력한다.

---

## Sources

- 04-architecture.md (실행 모델 · 파이프라인 · 크레이트 표 · IR 설계 원칙)
- 03-build-and-portability.md (cargo-only 빌드 · MSRV · 3-OS · 워크스페이스)
- 05-strategy-and-roadmap.md (MVP 범위 · 단계 게이트)
- 08-timescale-and-timing.md (전역 정밀도 min() · 디렉티브 캐리오버)
- 09-testing-and-verification.md (직렬화 round-trip · VCD 동치 테스트)
- 본 spec §5.2/§5.4 — `docs/superpowers/specs/2026-05-26-vitamin-rtl-simulator-design.md`
- 상용 단계 분리 선례: Cadence `xmvlog`/`xmelab`/`xmsim`·`cds.lib`, Synopsys
  `vlogan`/`vcs`/`simv`·`synopsys_sim.setup`
- 오픈소스 선례: Icarus `iverilog`→`.vvp`→`vvp`, GHDL `-a`/`-e`/`-r`·`.cf`, Yosys RTLIL
- rustc Strict Version Hash (SVH) / StableCrateId — version-gate 모델
