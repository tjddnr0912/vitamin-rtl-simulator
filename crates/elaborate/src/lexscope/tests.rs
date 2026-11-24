//! FB: the funnel's own unit tests (§5.2 Q-LEXSCOPE LS2a): every entry keeps the legacy text
//! rule and the id agrees with the text after each call.

use super::{scope_out, shown_from_spelling, wrap_ctx, DeclKey, Mark, ScopeKeyed, Scopes};

#[test]
fn fb_new_is_the_empty_path() {
    let s = Scopes::new();
    assert_eq!(s.cur_prefix_text(), "");
}

#[test]
fn fb_enter_and_restore() {
    let mut s = Scopes::new();
    let a = s.scope_enter("top");
    assert_eq!(s.cur_prefix_text(), "top");
    let b = s.scope_enter("g[0]");
    assert_eq!(s.cur_prefix_text(), "top.g[0]");
    s.scope_restore(b);
    assert_eq!(s.cur_prefix_text(), "top");
    s.scope_restore(a);
    assert_eq!(s.cur_prefix_text(), "");
}

#[test]
fn fb_swap_take_pkg_repoint_hold() {
    let mut s = Scopes::new();
    let a = s.scope_swap_text("top.u".to_string());
    let b = s.scope_swap_empty();
    assert_eq!(s.cur_prefix_text(), "");
    let c = s.scope_swap_pkg("p");
    assert_eq!(s.cur_prefix_text(), "$pkg$p");
    s.scope_restore(c);
    s.scope_restore(b);
    assert_eq!(s.cur_prefix_text(), "top.u");
    let u = s.scope_carry();
    let v0 = s.scope_swap_text("top.v".to_string());
    let v = s.scope_carry();
    s.scope_restore(v0);
    let h = s.scope_hold();
    s.scope_repoint(&v);
    assert_eq!(s.cur_prefix_text(), "top.v");
    s.scope_repoint(&u);
    s.scope_restore(h);
    assert_eq!(s.cur_prefix_text(), "top.u");
    s.scope_restore(a);
    assert_eq!(s.cur_prefix_text(), "");
}

// LS3: the carried scope — carry, re-entry, the two text exits.

#[test]
fn fb_scope_text_of_carry_is_the_current_text_after_each_opener() {
    let mut s = Scopes::new();
    let check = |s: &Scopes| assert_eq!(s.scope_text(&s.scope_carry()), s.cur_prefix_text());
    check(&s);
    let a = s.scope_enter("top");
    check(&s);
    let b = s.scope_swap_text("top.u.g[0]".to_string());
    check(&s);
    let c = s.scope_swap_empty();
    check(&s);
    let d = s.scope_swap_pkg("p");
    check(&s);
    let e = s.scope_hold();
    check(&s);
    s.scope_restore(e);
    s.scope_restore(d);
    s.scope_restore(c);
    check(&s);
    s.scope_restore(b);
    s.scope_restore(a);
    check(&s);
}

#[test]
fn fb_scope_swap_reenters_the_carried_scope_without_interning() {
    let mut s = Scopes::new();
    let a = s.scope_swap_text("top.u".to_string());
    let u = s.scope_carry();
    s.scope_restore(a);
    let n = s.names.len();
    let b = s.scope_swap(&u);
    assert_eq!(s.cur_prefix_text(), "top.u");
    assert_eq!(s.names.len(), n);
    let c = s.scope_enter("x");
    let ux = s.scope_carry();
    s.scope_restore(c);
    s.scope_restore(b);
    assert_eq!(s.cur_prefix_text(), "");
    let d = s.scope_swap(&ux);
    assert_eq!(s.cur_prefix_text(), "top.u.x");
    s.scope_restore(d);
}

#[test]
fn fb_key_under_and_scope_debug() {
    let mut s = Scopes::new();
    let root = s.scope_carry();
    assert_eq!(super::at_text(&s.key_under(&root, "n")), "n");
    let a = s.scope_swap_text("top.u".to_string());
    let u = s.scope_carry();
    s.scope_restore(a);
    assert_eq!(super::at_text(&s.key_under(&u, "n")), "top.u.n");
    assert_eq!(s.scope_text(&u), "top.u");
    // Debug writes no text: two different scopes print the same
    assert_eq!(format!("{root:?}"), format!("{u:?}"));
    assert_eq!(format!("{u:?}"), "ScopeRef(..)");
}

#[test]
fn fb_reentry_reuses_the_id() {
    let mut s = Scopes::new();
    let a = s.scope_swap_text("x".to_string());
    s.scope_restore(a);
    let n = s.names.len();
    let b = s.scope_swap_text("x".to_string());
    assert_eq!(s.names.len(), n);
    let w = s.scope_witness();
    s.scope_witness_check(&w);
    s.scope_restore(b);
}

// LS2-d: the display chain's entries and the door's text exit.

#[test]
fn fb_disp_chain_take_push_pop_restore() {
    let mut s = Scopes::new();
    assert!(s.disp_chain_is_empty());
    s.disp_chain_push("a".to_string());
    s.disp_chain_push("b".to_string());
    assert_eq!(s.disp_chain, ["a", "b"]);
    let saved = s.disp_chain_take();
    assert!(s.disp_chain_is_empty());
    s.disp_chain_push("x".to_string());
    s.disp_chain_pop();
    s.disp_chain_pop();
    assert!(s.disp_chain_is_empty());
    s.disp_restore_chain(saved);
    assert_eq!(s.disp_chain, ["a", "b"]);
    let single = s.disp_chain_swap_single("t");
    assert_eq!(s.disp_chain, ["t"]);
    s.disp_restore_chain(single);
    assert_eq!(s.disp_chain, ["a", "b"]);
}

#[test]
fn fb_disp_root_and_method_swap_restore() {
    let mut s = Scopes::new();
    s.disp_root = Some("top.u".to_string());
    let r = s.disp_root_swap_empty();
    assert_eq!(s.disp_root.as_deref(), Some(""));
    s.disp_restore_root(r);
    assert_eq!(s.disp_root.as_deref(), Some("top.u"));
    assert!(s.class_method_is_none());
    let m = s.disp_class_method_swap("C.f".to_string());
    assert!(!s.class_method_is_none());
    assert_eq!(s.disp_method.as_deref(), Some("C.f"));
    s.disp_restore_method(m);
    assert!(s.class_method_is_none());
}

#[test]
fn fb_shown_leaves_only_through_scope_out() {
    let a = shown_from_spelling("C.f".to_string());
    let b = shown_from_spelling("D.g".to_string());
    // Debug writes no text: two different spellings print the same
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    assert_eq!(format!("{a:?}"), "Shown(..)");
    assert_eq!(scope_out(a.clone()), "C.f");
    assert_eq!(scope_out(a), "C.f");
}

#[test]
fn fb_shown_ctx_debug_writes_no_text() {
    let frame = |l: &str| diag::Frame {
        label: l.to_string(),
        location: None,
    };
    let a = wrap_ctx(vec![frame("top.u")]);
    let b = wrap_ctx(Vec::new());
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    assert_eq!(format!("{a:?}"), "ShownCtx(..)");
    let out = scope_out(a);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].label, "top.u");
}

/// DELTA13 D3's truth table for `hier_leaf_scope`'s guard (expr_size_hier.rs:579-590 on
/// ffc2171f): `cur_is_inst_top_or_synth_below` over (cur, the instance mark).
#[test]
fn fb_cur_is_inst_top_or_synth_below_truth_table() {
    let rows: [(&str, &str, bool); 7] = [
        ("top.u1", "top.u1", true),
        ("top.u1", "top.u1.$blk$a.$func$f", true),
        ("top.u1", "top.u1.g[0]", false),
        ("top.u1", "top.u10", false),
        ("top.u1", "top.x", false),
        ("", "", true),
        ("", "$func$f", false),
    ];
    for (mark, cur, want) in rows {
        let mut s = Scopes::new();
        let m0 = s.scope_swap_text(mark.to_string());
        let old = s.mark_swap_here(Mark::Inst);
        let c0 = s.scope_swap_text(cur.to_string());
        assert_eq!(
            s.cur_is_inst_top_or_synth_below(),
            want,
            "mark `{mark}` cur `{cur}`"
        );
        s.scope_restore(c0);
        s.mark_put(Mark::Inst, old);
        s.scope_restore(m0);
    }
}

/// The marks: a swap returns what the mark held, a put restores it, the tests read the
/// mark's text, `swap_to_mark` re-enters it.
#[test]
fn fb_marks_swap_put_exchange_and_ask() {
    let mut s = Scopes::new();
    assert!(
        !s.cur_is_inst_top(),
        "the empty instance mark is never the top"
    );
    assert!(s.cur_is_tf_decl(), "both empty");
    let a = s.scope_swap_text("top.u".to_string());
    let u = s.scope_carry();
    let old_inst = s.mark_swap_here(Mark::Inst);
    let old_tf = s.mark_swap(Mark::TfDecl, &u);
    assert!(s.cur_is_inst_top());
    assert!(s.cur_is_tf_decl());
    let g = s.scope_swap_text("top.u.g[0]".to_string());
    assert!(!s.cur_is_inst_top());
    assert!(!s.cur_is_tf_decl());
    let back = s.swap_to_mark(Mark::TfDecl);
    assert_eq!(s.cur_prefix_text(), "top.u");
    s.scope_restore(back);
    assert_eq!(s.cur_prefix_text(), "top.u.g[0]");
    let mut other = s.scope_carry();
    s.mark_exchange(Mark::Inst, &mut other);
    assert!(s.cur_is_inst_top(), "the mark now holds top.u.g[0]");
    s.mark_exchange(Mark::Inst, &mut other);
    assert_eq!(s.scope_text(&other), "top.u.g[0]");
    s.scope_restore(g);
    s.mark_put(Mark::TfDecl, old_tf);
    s.mark_put(Mark::Inst, old_inst);
    assert!(!s.cur_is_inst_top());
    s.scope_restore(a);
}

/// `scope_child` is `child_prefix`'s text (a child of the empty scope is its name), and a
/// re-entry of it interns nothing new.
#[test]
fn fb_scope_child_text_and_reentry() {
    let mut s = Scopes::new();
    let top = s.scope_child("top");
    assert_eq!(s.scope_text(&top), "top");
    let a = s.scope_swap(&top);
    let u = s.scope_child("u[3]");
    assert_eq!(s.scope_text(&u), "top.u[3]");
    let n = s.names.len();
    let again = s.scope_child("u[3]");
    assert_eq!(s.names.len(), n);
    assert_eq!(scope_out(s.scope_shown(&again)), "top.u[3]");
    s.scope_restore(a);
}

/// The open generate scopes: a binding is enclosed by the genvar's construct, by the
/// instance's own scope, or by an open scope whose construct holds the position.
#[test]
fn fb_gen_binding_encloses() {
    let sp = |lo, hi| hdl_ast::Span { lo, hi };
    // a found key as the walk would return it (DK1-4b-2: the entry takes a DeclKey)
    let k = |t: &str| DeclKey(t.to_string());
    let mut s = Scopes::new();
    let a = s.scope_swap_text("top".to_string());
    let old = s.mark_swap_here(Mark::Inst);
    let gv = s.new_key("i");
    assert!(
        s.gen_binding_encloses(&k("top.P"), "P", sp(5, 6)),
        "the instance's own scope"
    );
    let b = s.scope_swap_text("top.g[0]".to_string());
    s.gen_text_push(sp(10, 20), Some(gv));
    assert!(
        s.gen_binding_encloses(&k("top.i"), "i", sp(12, 13)),
        "the genvar, inside"
    );
    assert!(
        !s.gen_binding_encloses(&k("top.i"), "i", sp(30, 31)),
        "the genvar, outside"
    );
    assert!(
        s.gen_binding_encloses(&k("top.g[0].K"), "K", sp(11, 12)),
        "the open scope"
    );
    assert!(!s.gen_binding_encloses(&k("top.g[0].K"), "K", sp(25, 26)));
    assert!(
        !s.gen_binding_encloses(&k("top.h[0].K"), "K", sp(11, 12)),
        "no such open scope"
    );
    s.gen_text_pop();
    assert!(
        !s.gen_binding_encloses(&k("top.g[0].K"), "K", sp(11, 12)),
        "closed"
    );
    s.scope_restore(b);
    s.mark_put(Mark::Inst, old);
    s.scope_restore(a);
}

/// ScopeKeyed: keyed by the scope's text, the instance-name slot is `<cur>.<name>` even
/// under the empty scope, the drain is in text order and leaves the store empty.
#[test]
fn fb_scope_keyed_take_put_entry_drain() {
    let mut s = Scopes::new();
    let mut m: ScopeKeyed<Vec<u32>> = ScopeKeyed::new();
    m.entry_under_cur(&s, "u").push(1);
    let a = s.scope_swap_text("top".to_string());
    m.entry_under_cur(&s, "v").push(2);
    m.entry_under_cur(&s, "v").push(3);
    let v = s.scope_child("v");
    assert_eq!(m.take_at(&s, &v), Some(vec![2, 3]));
    assert_eq!(m.take_at(&s, &v), None);
    m.put_at(&s, &v, vec![4]);
    let b = s.scope_child("b");
    m.put_at(&s, &b, vec![5]);
    let keys: Vec<String> = m.drain_keys_shown().into_iter().map(scope_out).collect();
    assert_eq!(keys, [".u", "top.b", "top.v"]);
    assert!(m.drain_keys_shown().is_empty());
    s.scope_restore(a);
}

/// LS3b-2: `cur_shown` is the current scope's raw text (a profile row's scope), the empty
/// path included ; `cur_shown_quoted_or` backquotes it, or gives its argument under the empty
/// path ; `Shown::default()` is the empty text.
#[test]
fn fb_cur_shown_and_quoted_or() {
    let mut s = Scopes::new();
    assert_eq!(scope_out(s.cur_shown()), "");
    assert_eq!(
        scope_out(s.cur_shown_quoted_or("the enclosing scope")),
        "the enclosing scope"
    );
    let a = s.scope_swap_text("top.g[0].$blk$12".to_string());
    assert_eq!(scope_out(s.cur_shown()), s.cur_prefix_text());
    assert_eq!(scope_out(s.cur_shown_quoted_or("x")), "`top.g[0].$blk$12`");
    s.scope_restore(a);
    assert_eq!(scope_out(super::Shown::default()), "");
}

/// LS3b-2: `push_here` appends to the list stored at the CURRENT scope's text (the empty
/// path's key is ``), the store keeps text order.
#[test]
fn fb_scope_keyed_push_here() {
    let mut s = Scopes::new();
    let mut m: ScopeKeyed<Vec<u32>> = ScopeKeyed::new();
    m.push_here(&s, 1);
    let a = s.scope_swap_text("top".to_string());
    m.push_here(&s, 2);
    m.push_here(&s, 3);
    let here = s.scope_carry();
    assert_eq!(m.take_at(&s, &here), Some(vec![2, 3]));
    s.scope_restore(a);
    let top = s.scope_carry();
    assert_eq!(m.take_at(&s, &top), Some(vec![1]));
    assert!(m.drain_keys_shown().is_empty());
}

/// LS4: the scope questions, each a bool over the text the site tested before (PLAN7 LS4) —
/// a truth table per entry, the current scope (or the carried one) entered by its text.
#[test]
fn fb_scope_questions_truth_tables() {
    let ask = |cur: &str, q: &dyn Fn(&Scopes) -> bool| {
        let mut s = Scopes::new();
        let a = s.scope_swap_text(cur.to_string());
        let got = q(&s);
        s.scope_restore(a);
        got
    };
    // scope_is_pkg (lexscope/hier.rs hier_resolve): the carried scope's text starts with `$pkg$`
    for (cur, want) in [
        ("$pkg$p", true),
        ("$pkg$p.$func$f", true),
        ("top", false),
        ("", false),
        ("top.$pkg$p", false),
    ] {
        let got = ask(cur, &|s: &Scopes| s.scope_is_pkg(&s.scope_carry()));
        assert_eq!(got, want, "scope_is_pkg `{cur}`");
    }
    // cur_under_func_or_itask (struct_arm.rs): any segment starts with `$func$` / `$itask$`
    for (cur, want) in [
        ("top.$func$f", true),
        ("top.$itask$t.$blk$1", true),
        ("$func$f", true),
        ("top.g[0]", false),
        ("top.x$func$f", false),
        ("", false),
    ] {
        let got = ask(cur, &|s: &Scopes| s.cur_under_func_or_itask());
        assert_eq!(got, want, "cur_under_func_or_itask `{cur}`");
    }
    // in_block_local_scope (string_array_route.rs): the LAST segment starts with `$blk$`
    for (cur, want) in [
        ("top.$blk$12", true),
        ("$blk$3", true),
        ("top.$blk$1.g[0]", false),
        ("top.x$blk$1", false),
        ("", false),
    ] {
        let got = ask(cur, &|s: &Scopes| s.in_block_local_scope());
        assert_eq!(got, want, "in_block_local_scope `{cur}`");
    }
    // cur_frame_routine_is_any (frames_call/emit.rs): the trailing `$func$<n>` / `$itask$<n>`
    // name is one of the targets' names
    let f = vec![(7, "f".to_string())];
    let t = vec![(1, "g".to_string()), (2, "t".to_string())];
    for (cur, fids, want) in [
        ("top.$func$f", &f, true),
        ("top.$itask$t", &t, true),
        ("top.$func$g", &f, false),
        ("top.$func$f.$blk$1", &f, false),
        ("top.f", &f, false),
        ("", &f, false),
    ] {
        let got = ask(cur, &|s: &Scopes| s.cur_frame_routine_is_any(fids));
        assert_eq!(got, want, "cur_frame_routine_is_any `{cur}` {fids:?}");
    }
    assert!(!ask("top.$func$f", &|s: &Scopes| s.cur_frame_routine_is_any(&[])));
    // hier_path_is_self (hoist): some segment of the path, its `[..]` cut, is the head's name
    let id = |n: &str| hdl_ast::Ident {
        name: n.to_string(),
        span: hdl_ast::Span::new(0, 0),
    };
    for (cur, segs, want) in [
        ("top.u1.blk", vec![id("u1"), id("o")], true),
        ("top.u1.blk", vec![id("top"), id("o")], true),
        ("top.u1.blk", vec![id("sub"), id("o")], false),
        ("top.g[0]", vec![id("g"), id("o")], true),
        ("top.g[0]", vec![id("g[0]"), id("o")], false),
        ("top", vec![], false),
        ("", vec![id("o")], false),
    ] {
        let got = ask(cur, &|s: &Scopes| s.hier_path_is_self(&segs));
        assert_eq!(got, want, "hier_path_is_self `{cur}` {segs:?}");
    }
}

/// §5.2 Q-DECLKEY DK1-3: the read key a current-scope read forms (`at_cur`) probes the bytes the
/// declaration key (`new_key`) stored in the same scope — the current scope's, not an outer one's ;
/// a miss names the same text through `show_miss` (display), and `at_text` (L9) is that text.
#[test]
fn fb_read_key_meets_the_declared_key() {
    use super::{at_text, Keyed, KeyedSet};
    let mut s = Scopes::new();
    let mut m: Keyed<u32> = Keyed::default();
    let mut set = KeyedSet::default();
    m.insert_new(s.new_key("a"), 1);
    assert_eq!(m.get_at(&s.at_cur("a")), Some(&1));
    assert_eq!(at_text(&s.at_cur("a")), "a");
    assert_eq!(scope_out(s.show_miss("a")), "a");
    let r = s.scope_swap_text("top.u".to_string());
    assert_eq!(
        m.get_at(&s.at_cur("a")),
        None,
        "the outer scope's key is not the current one's"
    );
    m.insert_new(s.new_key("a"), 2);
    set.insert_new(s.new_key("w"));
    assert_eq!(m.get_at(&s.at_cur("a")), Some(&2));
    assert!(m.contains_at(&s.at_cur("a")));
    assert!(set.contains_at(&s.at_cur("w")));
    assert!(!m.contains_at(&s.at_cur("b")));
    assert_eq!(at_text(&s.at_cur("a")), "top.u.a");
    assert_eq!(scope_out(s.show_miss("b")), "top.u.b");
    s.scope_restore(r);
    assert_eq!(m.get_at(&s.at_cur("a")), Some(&1));
    assert!(!set.contains_at(&s.at_cur("w")));
}
