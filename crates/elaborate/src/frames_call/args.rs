//! ACTUAL → FORMAL binding for a frame call: the named-argument reorder (IEEE 1800
//! §13.5.4) and the omitted-actual default fill. Split from `frames_call.rs` (R19) to
//! keep every module under the 1000-line cap.

use super::*;

impl Elaborator<'_> {
    pub(crate) fn resolve_named_args(
        &mut self,
        fname: &str,
        ports: &[ast::TfPort],
        args: &[ast::Expr],
    ) -> Option<Vec<ast::Expr>> {
        let mut slots: Vec<Option<ast::Expr>> = vec![None; ports.len()];
        let mut seen_named = false;
        let mut pos = 0usize;
        for a in args {
            if let ast::ExprKind::NamedArg { formal, value } = &a.kind {
                seen_named = true;
                let Some(idx) = ports.iter().position(|p| p.name.name == formal.name) else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("call to `{fname}`: no formal named `{}`", formal.name),
                    );
                    return None;
                };
                if slots[idx].is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "call to `{fname}`: formal `{}` is bound more than once",
                            formal.name
                        ),
                    );
                    return None;
                }
                match value {
                    Some(v) => slots[idx] = Some((**v).clone()),
                    None => match &ports[idx].default {
                        Some(def) => slots[idx] = Some(def.clone()),
                        None => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "call to `{fname}`: `.{}()` has no default value",
                                    formal.name
                                ),
                            );
                            return None;
                        }
                    },
                }
            } else {
                if seen_named {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "call to `{fname}`: a positional argument cannot follow a named one"
                        ),
                    );
                    return None;
                }
                if pos >= ports.len() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("call to `{fname}`: too many positional arguments"),
                    );
                    return None;
                }
                slots[pos] = Some(a.clone());
                pos += 1;
            }
        }
        let mut out = Vec::with_capacity(ports.len());
        for (i, slot) in slots.into_iter().enumerate() {
            match slot {
                Some(e) => out.push(e),
                None => match &ports[i].default {
                    Some(def) => {
                        // R19-X1: a default is lowered in the CALLER's scope, but IEEE
                        // 1800 §13.5.4 evaluates it where the subroutine is DECLARED.
                        // Loud when a name in it actually binds differently here.
                        if !self.default_binding_matches_decl_scope(def) {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "call to `{fname}`: the omitted actual for formal `{}` \
                                     takes a DEFAULT value whose names bind differently at \
                                     this call site than where `{fname}` is declared — IEEE \
                                     1800 §13.5.4 evaluates a default in the subroutine's own \
                                     scope, so pass the argument explicitly here, or make the \
                                     default a literal / `pkg::` constant",
                                    ports[i].name.name
                                ),
                            );
                            return None;
                        }
                        // Same guard as `fill_default_args`: a default referencing another
                        // formal would wrongly bind to a caller variable (silent-wrong).
                        if ports.iter().any(|q| expr_reads_ident(def, &q.name.name)) {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "function/task `{fname}`: a default argument value that references another formal is unsupported"
                                ),
                            );
                            return None;
                        }
                        out.push(def.clone());
                    }
                    None => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "call to `{fname}`: missing actual for formal `{}` (no default value)",
                                ports[i].name.name
                            ),
                        );
                        return None;
                    }
                },
            }
        }
        Some(out)
    }

    pub(crate) fn fill_default_args<'a>(
        &mut self,
        fname: &str,
        ports: &'a [ast::TfPort],
        args: &'a [ast::Expr],
    ) -> Option<Vec<&'a ast::Expr>> {
        if args.len() > ports.len() {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "function/task `{fname}`: {} args for {} formals",
                    args.len(),
                    ports.len()
                ),
            );
            return None;
        }
        let mut eff: Vec<&'a ast::Expr> = args.iter().collect();
        for p in &ports[args.len()..] {
            match &p.default {
                Some(def) => {
                    // R19-X1: a default is lowered in the CALLER's scope, but IEEE
                    // 1800 §13.5.4 evaluates it where the subroutine is DECLARED.
                    // Loud when a name in it actually binds differently here.
                    if !self.default_binding_matches_decl_scope(def) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "call to `{fname}`: the omitted actual for formal `{}` \
                                 takes a DEFAULT value whose names bind differently at \
                                 this call site than where `{fname}` is declared — IEEE \
                                 1800 §13.5.4 evaluates a default in the subroutine's own \
                                 scope, so pass the argument explicitly here, or make the \
                                 default a literal / `pkg::` constant",
                                p.name.name
                            ),
                        );
                        return None;
                    }
                    // The default is lowered in the CALLER scope; a default that
                    // references another FORMAL (`int b = a + 1`) would wrongly bind to
                    // a same-named caller variable (a silent-wrong vs iverilog, which
                    // resolves it in the subroutine scope). Loud-reject that case.
                    if ports.iter().any(|q| expr_reads_ident(def, &q.name.name)) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "function/task `{fname}`: a default argument value that references another formal is unsupported"
                            ),
                        );
                        return None;
                    }
                    eff.push(def);
                }
                None => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "function/task `{fname}`: missing actual for formal `{}` (no default value)",
                            p.name.name
                        ),
                    );
                    return None;
                }
            }
        }
        Some(eff)
    }

    // ── B1 frame-call: automatic/recursive function lowering ────────────────
}
