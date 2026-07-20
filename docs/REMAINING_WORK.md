# vitamin — 잔여 작업 트래커 (Remaining Work)

> **"goal까지 남은 것"의 상위 스냅샷.** 재계획 시점마다 통째로 갱신한다(과거 판본은 git 이력이 보존).
>
> - **기준(2026-07-21)**: format_version **22** · **3785 tests green** · 3-OS CI green · MsgCode 58 · **MSRV 1.85** · 최신 완료 §4.5.176(**`foreach` over dynamic array/queue/assoc inside FUNCTION/SUBSET-task body loud→supported** — §4.5.175 loud를 supported로: iteration key가 항상 frame-local이라 `&self` 동기 executor가 interior-mutable frame window로 advance 가능·`assoc_iter_step` read/compute를 `SimState::assoc_iter_compute`로 추출[process path byte-identical]·`frame_assoc_iter`가 key write·적대 differential 全 검증·잔여 loud=non-local key direct iter·format 22 불변·+2) · §4.5.175(**SILENT-WRONG 수정: `foreach` … → loud** — desugar된 walk step `a.first/next(__i)`이 iteration key를 side-effect write하는데 동기 `&self` frame executor[`run_frame_call`/`run_task`]가 못 advance→조용히 0 반환하던 것을 runtime fatal F4004로 loud화·적대 de-risk probe서 vita 0 vs iverilog 15 발굴·suspendable task/`for`-loop workaround/fixed-array 미영향·format 22 불변·+8) · §4.5.174(**inline static-task `foreach`-on-dyn-formal loud→supported** — 파서가 `foreach(b[i])`를 `b.first/next(__i)`로 desugar하는데 dyn/queue dense-walk dispatch가 `dyn_handle`로 array를 resolve해 `dyn_subst` formal alias를 못 봐 E3009였음→`dyn_handle_read`[alias 우선]로 2 site 교체·formal이 module dyn-array와 동일 dense walk·`dyn_subst` empty 밖에선 모든 array byte-identical·적대 differential 全 MATCH·`inline_foreach_dyn_formal.rs`×8·format 22 불변·+8) · §4.5.173(**V2A-frame: AUTOMATIC task `input` dynamic-array formal loud→supported** — formal을 per-activation `NetKind::DynArray` heap slot으로 예약[`reserve_frame_task`]하고 caller 배열을 frame ENTRY서 deep-copy[pass-by-VALUE·IEEE §13.5.1·`frame_dyn_snapshot_formals`·exec.rs 2 suspendable site]·V5 net-range lifecycle이 formal 자동 커버[free/reentry]·**wire 무변화**[snapshot src를 새 IR 필드 대신 in-bind Signal서 엔진-side 회수→format 22 불변·golden churn 0]·적대 differential 全 MATCH[특히 **snapshot-immune-across-suspend**=동시 resize 누출 0]·LOUD=recursion/concurrent/non-suspendable-subset/sign-mismatch·`frame_dyn_formal.rs`×13·+13) · §4.5.172(**frame-body validator over-scan false-REJECT 수정** — subset task가 not-defined-last일 때 `classify_frame_body`의 linear `block_base..func_blocks.len()` 스캔이 post-pass서 뒤 func 블록까지 over-read→그들의 out-of-frame write를 오판해 E3009 false-reject하던 것을 **reachable-block CFG walk**[entry서 자기 CFG 엣지만·`Call`→`ret_bb`라 함수 밖 안 나감]로 교체·correct-or-loud[미방문=타 func/dead→false-reject만 DROP·silent-wrong 불가]·적대 differential 全 MATCH[nested call·control flow·suspendable-mixed·still-loud 유지]·`frame_subset_overscan.rs`×4·format 22 불변·+4) · §4.5.171(**V5: frame-local(task-body) DYNAMIC array loud→supported** — TASK 한정·per-net heap + lifecycle[reentry guard=recursion/concurrent fatal F4004·free-at-exit]·30-site 재키잉 회피·function은 `&self` executor 한계로 loud·회귀 발굴[frame-local string `dyn_is_handle` 공유→`kind!=String` 가드]·적대 differential ~18 probe MATCH·format 22 불변·+13) · §4.5.170(**V2A: TASK `input` dynamic-array formal loud→supported** — static task 한정·**pass-by-value SNAPSHOT**[적대 soundness agent가 초판 direct-alias의 IEEE §13.5.1 위반 silent-wrong 발굴→`alloc_dyn_snapshot`+`handle_copy_stmts` entry deep-copy로 수정]·automatic/frame은 V5[handle-in-slot] 필요→loud-defer·공통 `dyn_array_actual_net` fix로 re-forward가 함수·태스크 양쪽 동작·format 22 불변·+10) · §4.5.169(**frame-local unpacked ARRAY loud→supported** — §4.5.168 잔여 loud 첫 항목·"순차적으로 전부" 1순위·배열 FORMAL의 md-packed 표현 재사용[`classify_unpacked_array` 공유·`reserve_frame_local_decl` 4-site 통일·`frame_arr_formal_meta`]·엔진 frame_local=net-range라 md-packed net 자동 per-activation/재귀 격리·적대 differential ~20 probe common 全 MATCH+automatic-array 격리/subroutine array port는 vita>iverilog[hand-IEEE]·잔여 loud safe gap=multi-dim/non-zero-based/non-simple-element/whole-copy/foreach/NBA-elem/`'{}`init·format 22 불변·+1) · 직전 §4.5.168(**V3/V4 suspendable tasks** — task 内 `@`/`#`/wait/NBA/$systask/재귀/nested가 scheduler call-stack + window stash/restore로 동작·round-14 gating ①·format 22 불변[recompute]·적대 2-lens core SOUND+5 over-lift correct-or-loud화) · §4.5.167(round-14 loud→supported: **V9** 64-bit MSB-set scalar-localparam 리터럴 fold[`const_eval_i64_lit`의 signed-only width-64 bit-reinterpret arm을 `cv.width==64`로 확장·§4.5.151 defer 해소·magnitude sink 全 loud 확인] + **V1** function/task/block/class-method-local `string` 변수[`frame_local_net_kind`·4 사이트 heap NetKind::String slot] + **V1-twin** 엔진 str_bytes frame-aware[심층검증서 loud→silent 회귀 발굴: NetKind만 고치면 `.len()` 등 method가 frame slot 못 읽어 조용히 0→`read_net` 미러로 수정]; round-14=§4.5.166 ★V10의 confirmation 리포트; 적대 2-lens differential CLEAN[~40 probe·zero silent-wrong·오히려 iverilog가 magnitude-misuse silent] + soundness SOUND[4 change 全]; elaborate value-only+engine eval-only·format 불변). 미착수 후보("순차적으로 전부" 잔여)=**function-frame dyn-formal(framed function의 dyn-array input formal·§4.5.176로 함수 内 dyn foreach는 동작하나 formal이 classification[md-packed]서 rejected→formal을 `NetKind::DynArray` net으로 reserve[V2A-frame처럼]+call-site snapshot 필요·이제 read path는 준비됨)·V5 follow-on(function dyn-array local[`new[]`=`&mut` heap는 여전히 `&self` 불가]·recursion/concurrent[per-activation heap stash])·V3/V4 잔여 loud(in-frame fork·per-activation repeat counter)·V6/V8 unpacked struct(oracle-reject→hand-IEEE)** · 함수/태스크 body-내부 read→caller comb 감도(§2 DEEP·§4.5.166) · size-cast `N'(expr)` context width(§2 DEEP).
> - 잔여 상세 목록(정본) = [ROADMAP.md](ROADMAP.md) · 완료 상세 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존) · 이력 = [DEVLOG.md](DEVLOG.md) · 실행 큐 = `LOOPROMPT.md` NEXT.
> - **최종 목표**: **G1** = icarus·verilator·xcelium·vcs급 *정확한* RTL 시뮬레이터(correct-or-loud) · **G2** = AI-Agent 친화 simulator(SPEC=[preview/19](preview/19-ai-agent-observability.md)).

## A. 현재 상태 한 줄 요약

- **oracle-backed ① silent-wrong = 소진**(발굴 즉시 수정 체제). known silent-wrong 잔여는 전부 **pre-existing·baseline 동일·deep-defer 기록됨**(ROADMAP §2).
- 외부 리포트 1·2차(EXT2)·round 3~11 = **사실상 완결**(잔여 3건=A2c·NAP·DOC, 전부 no-oracle/docs).
- 나머지 잔여는 **honest-loud=안전**(ROADMAP §3~§5) + **G2 OBS 트랙**(ROADMAP §6).

## B. 다음 착수 후보 (우선순위순)

| # | 항목 | 근거/오라클 |
|---|---|---|
| 1 | 신규 CRITICAL silent-wrong 발굴분(fresh-area probe) | iverilog 라이브 차분 |
| 2 | 소형 loud→supported 큐(ROADMAP §3) | iverilog ✓ 대부분 |
| 3 | OBS-2 sva.jsonl(R-L6) 또는 OBS-1 잔여(staged obs·`--seed`) | 3-way 내부 차분 |
| 4 | DEEP-defer 재개(%c/%s UTF-8 pipeline·derived-localparam self-width·top-level typedef ②) | 전용 인프라 슬라이스 |

## C. 잔여 분류 (요약 — 상세=ROADMAP 해당 §)

| 분류 | 내용 | 정본 |
|---|---|---|
| 🔴 silent-wrong 잔여 | DEEP 5(UTF-8 pipeline·derived-param width·$unit typedef·enclosing-const·packed-WIDTH sibling) + 중형 ~15(enum·sign-loss·repl-count·residual sub-select·param-scope·iface-2D·$dist_*·string-concat-width 등) — 전부 pre-existing·기록됨 | ROADMAP §2 |
| honest-loud 잔여 | string/heap·함수/formal·소형 큐·EXT2 3건·deep 저우선 | ROADMAP §3 |
| SVA/검증 잔여 | empty-match 융합·N2c full·prop-ref skew 고급형·QUAD default-flip·N4 clocking 잔여·class down-cast | ROADMAP §4 |
| perf/하드닝 | 전부 보류 판정(SVA-QUAD flip·FMT-CACHE b·GEN-3X-STR a·QUEUE-MID) | ROADMAP §5 |
| G2 OBS | sva.jsonl·staged obs·R-L4·control API·snapshot·X-origin | ROADMAP §6 |

## D. 별도 관리 — 트리거 충족 시에만 승격 (정확성과 직교)

| id | 항목 | 트리거 |
|---|---|---|
| BACKEND | cycle-based 컴파일드 · PDES BSP(T4≈2.5x 상한) · native-eval 잔여 lane | 대형 RTL 실수요 · W≥64+grain≥200ns · 저ROI defer |
| VHDL | VHDL 프론트엔드(GHDL 오라클·E7xxx) | SV plateau + 값도메인 결정 + GHDL 셋업 |
| VCD-EXT | `$dumpports*`(포트 strength) | 파형 툴 수요 (FST=**§4.5.149·150 지원**; known-edge=소형 fst-writer issue #4) |
| MVP-CUT | string concat-nonassign·wildcard assoc `[*]`·cross-frame disable 등 | 개별 수요 시 |

## E. 비계획 — 영구 비목표 (gap 아님)

- **DEFPARAM**(deprecated) · **IMPLICIT-NET**(정책=E3010) · **OOS**(synthesis·waveform GUI·UPF/SDF/DPI-C·shortreal·trireg·UVM·unique/priority 다중-match).
