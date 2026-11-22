//! FB: the funnel's own unit tests (§5.2 Q-LEXSCOPE LS2a): every entry keeps the legacy text
//! rule and the id agrees with the text after each call.

use super::{scope_out, shown_from_spelling, wrap_ctx, Scopes};

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
    assert_eq!(s.key_under(&root, "n"), "n");
    let a = s.scope_swap_text("top.u".to_string());
    let u = s.scope_carry();
    s.scope_restore(a);
    assert_eq!(s.key_under(&u, "n"), "top.u.n");
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
