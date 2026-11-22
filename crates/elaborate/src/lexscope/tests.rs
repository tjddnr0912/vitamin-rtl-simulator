//! FB: the funnel's own unit tests (§5.2 Q-LEXSCOPE LS2a): every entry keeps the legacy text
//! rule and the id agrees with the text after each call.

use super::Scopes;

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
    let h = s.scope_hold();
    s.scope_repoint("top.v".to_string());
    assert_eq!(s.cur_prefix_text(), "top.v");
    s.scope_repoint("top.u".to_string());
    s.scope_restore(h);
    assert_eq!(s.cur_prefix_text(), "top.u");
    s.scope_restore(a);
    assert_eq!(s.cur_prefix_text(), "");
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
