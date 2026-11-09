//! R2 (round-36) — the DISPLAY NAME of a builtin, once, for the whole tree.
//!
//! ## Why this is here and not in `sim-engine`
//!
//! [`SysTaskId`] and [`SysFuncId`] are `SchemaHash`-frozen: their variant ORDER
//! is part of the golden root, so the enums cannot move and cannot be
//! re-spelled. The name a human reads for one of them is a property OF the
//! variant, and the only way a second copy of that mapping stays honest is by
//! not existing. `state/frame_eval.rs::fatal_frame_sysread` already carried a
//! partial one (11 of 81 ids, everything else folded into `"a seeded $dist_*"`)
//! and that copy is now this one's caller (ROADMAP §5.2 SYS-1a: its ten `$name` arms
//! read [`sysfunc_name`]; its three phrases, a seeded `$random`, a queue pop and a
//! seeded `$dist_*`, are its own wording).
//!
//! ## The inverse direction
//!
//! [`sysfunc_id`] is the other direction of the same fact for the `$name` value calls
//! elaborate maps (40 names): the id a name lowers to. It sits here, beside
//! [`sysfunc_name`], so the two directions are read together, and the unit test
//! `sysfunc_id_round_trips` pins the round trip: every name `sysfunc_id` answers prints
//! back as itself, and every other `$name` [`sysfunc_name`] prints belongs to an id a
//! special form mints in elaborate (listed there with its site). The facts a `$name`
//! alone decides (membership lists) live in hdl-ast `sysfn.rs`, which the parser reaches.
//!
//! ## Why the matches have no `_` arm
//!
//! That is the whole maintenance argument. Appending a variant to either enum
//! — which happens on nearly every format bump — must not silently produce a
//! profile row labelled `"unknown"`; it must fail the build here, where the
//! author has the new construct in front of them. A `_ => "?"` arm would turn a
//! frozen-enum extension into a reporting silent-wrong, which is the one thing
//! the `--obs-procs` rail cannot afford (doc-19 §3: a wrong log is worse than
//! no log).
//!
//! ## The spelling convention, and why it is not uniform
//!
//! A `$`-prefixed name is the IEEE system task/function spelling and is what
//! the user typed. A `.name()` name is a METHOD-form builtin — a queue push, a
//! string conversion, an array reduction — which the user did NOT type with a
//! `$`, and printing `$qpushback` for `q.push_back(v)` would send them looking
//! for a system task that does not exist. Two vocabularies because the source
//! has two.
//!
//! ⚠️ `$cast` is deliberately AMBIGUOUS across the two tables: the task form
//! (`SysTaskId::Cast`) and the function form (`SysFuncId::Cast`) are the same
//! construct to a reader, so both render `"$cast"` and a profile merges them
//! into one row. That is a decision, not an oversight — the alternative
//! (`"$cast (task)"`) names an implementation detail the RTL author cannot see.

use crate::{SysFuncId, SysTaskId};

/// The `$name` / `.method()` a [`SysTaskId`] came from.
pub fn systask_name(which: SysTaskId) -> &'static str {
    use SysTaskId as T;
    match which {
        T::Display => "$display",
        T::Write => "$write",
        T::Monitor => "$monitor",
        T::Strobe => "$strobe",
        T::Finish => "$finish",
        T::Stop => "$stop",
        T::DumpFile => "$dumpfile",
        T::DumpVars => "$dumpvars",
        T::DumpOn => "$dumpon",
        T::DumpOff => "$dumpoff",
        T::DumpAll => "$dumpall",
        T::DumpFlush => "$dumpflush",
        T::DumpLimit => "$dumplimit",
        T::DynNew => ".new[]()",
        T::DynDelete => ".delete()",
        T::QPushBack => ".push_back()",
        T::QPushFront => ".push_front()",
        T::AssocDeleteKey => ".delete(key)",
        T::QInsert => ".insert()",
        T::QDeleteIdx => ".delete(index)",
        T::Fclose => "$fclose",
        T::Fdisplay => "$fdisplay",
        T::Fwrite => "$fwrite",
        T::Sformat => "$sformat",
        T::ReadmemB => "$readmemb",
        T::ReadmemH => "$readmemh",
        T::StrPutC => ".putc()",
        T::WritememB => "$writememb",
        T::WritememH => "$writememh",
        T::Cast => "$cast",
        T::MonitorOn => "$monitoron",
        T::MonitorOff => "$monitoroff",
        T::ClassRandomize => ".randomize()",
        T::ArrSort => ".sort()",
        T::ArrRsort => ".rsort()",
        T::ArrReverse => ".reverse()",
        T::StrItoa => ".itoa()",
        T::StrHextoa => ".hextoa()",
        T::StrOcttoa => ".octtoa()",
        T::StrBintoa => ".bintoa()",
        T::ArrLocator => ".find()/.min()/.max()",
    }
}

/// The `$name` / `.method()` a [`SysFuncId`] came from.
pub fn sysfunc_name(which: SysFuncId) -> &'static str {
    use SysFuncId as F;
    match which {
        F::Time => "$time",
        F::Realtime => "$realtime",
        F::Signed => "$signed",
        F::Unsigned => "$unsigned",
        F::Clog2 => "$clog2",
        F::Rtoi => "$rtoi",
        F::Itor => "$itor",
        F::RealToBits => "$realtobits",
        F::BitsToReal => "$bitstoreal",
        F::DynSize => ".size()",
        F::QPopBack => ".pop_back()",
        F::QPopFront => ".pop_front()",
        F::AssocExists => ".exists()",
        F::AssocNum => ".num()",
        F::AssocFirst => ".first()",
        F::AssocNext => ".next()",
        F::AssocLast => ".last()",
        F::AssocPrev => ".prev()",
        F::Random => "$random",
        F::Urandom => "$urandom",
        F::UrandomRange => "$urandom_range",
        F::CountOnes => "$countones",
        F::OneHot => "$onehot",
        F::OneHot0 => "$onehot0",
        F::IsUnknown => "$isunknown",
        F::Stime => "$stime",
        F::Fopen => "$fopen",
        F::Sformatf => "$sformatf",
        F::TestPlusargs => "$test$plusargs",
        F::ValuePlusargs => "$value$plusargs",
        F::StrLen => ".len()",
        F::StrGetC => ".getc()",
        F::StrSubstr => ".substr()",
        F::StrToUpper => ".toupper()",
        F::StrToLower => ".tolower()",
        F::StrCmp => ".compare()",
        F::StrCast => "string'()",
        F::RealToInt => "real->int",
        F::TwoState => "2-state",
        F::Fgets => "$fgets",
        F::Fscanf => "$fscanf",
        F::Sscanf => "$sscanf",
        F::Fread => "$fread",
        F::Feof => "$feof",
        F::Fgetc => "$fgetc",
        F::Ungetc => "$ungetc",
        F::DistUniform => "$dist_uniform",
        F::DistNormal => "$dist_normal",
        F::DistExponential => "$dist_exponential",
        F::DistPoisson => "$dist_poisson",
        F::DistChiSquare => "$dist_chi_square",
        F::Cast => "$cast",
        F::ArrSum => ".sum()",
        F::ArrProduct => ".product()",
        F::ArrAnd => ".and()",
        F::ArrOr => ".or()",
        F::ArrXor => ".xor()",
        F::StrAtoi => ".atoi()",
        F::StrAtohex => ".atohex()",
        F::StrAtooct => ".atooct()",
        F::StrAtobin => ".atobin()",
        F::StrAtoreal => ".atoreal()",
        F::Ln => "$ln",
        F::Log10 => "$log10",
        F::Exp => "$exp",
        F::Sqrt => "$sqrt",
        F::Pow => "$pow",
        F::Floor => "$floor",
        F::Ceil => "$ceil",
        F::Sin => "$sin",
        F::Cos => "$cos",
        F::Tan => "$tan",
        F::Asin => "$asin",
        F::Acos => "$acos",
        F::Atan => "$atan",
        F::Atan2 => "$atan2",
        F::Hypot => "$hypot",
        F::Sinh => "$sinh",
        F::Cosh => "$cosh",
        F::Tanh => "$tanh",
        F::Asinh => "$asinh",
        F::Acosh => "$acosh",
        F::Atanh => "$atanh",
        F::DistT => "$dist_t",
        F::DistErlang => "$dist_erlang",
        F::ReadmemAssocTarget => "$readmem target",
    }
}

/// The [`SysFuncId`] a `$name` value call lowers to — the inverse of [`sysfunc_name`]
/// over the names elaborate maps (moved verbatim from elaborate `systask.rs`
/// `map_sysfunc`, ROADMAP §5.2 SYS-1a). `$time`→Time, `$realtime`→Realtime,
/// `$signed`→Signed, `$unsigned`→Unsigned, `$clog2`→Clog2. `name` retains the leading
/// `$` (verdict M6). Unknown → None, and each caller reads None its own way (elaborate
/// `expr_main.rs`: the refusal path; `override_type.rs`: not real; `inline_body_ctx.rs`:
/// unclassified), so adding or removing a name here moves a value: the ids a special form
/// mints (`$fgets`, `$cast`, `$dist_*`, …) are never added (the round-trip test lists them).
pub fn sysfunc_id(dollar_name: &str) -> Option<SysFuncId> {
    match dollar_name {
        "$time" => Some(SysFuncId::Time),
        "$realtime" => Some(SysFuncId::Realtime),
        "$signed" => Some(SysFuncId::Signed),
        "$unsigned" => Some(SysFuncId::Unsigned),
        "$clog2" => Some(SysFuncId::Clog2),
        "$rtoi" => Some(SysFuncId::Rtoi),
        "$itor" => Some(SysFuncId::Itor),
        "$realtobits" => Some(SysFuncId::RealToBits),
        "$bitstoreal" => Some(SysFuncId::BitsToReal),
        // v7 bit-vector predicates ($bits never reaches here — const-folded).
        "$countones" => Some(SysFuncId::CountOnes),
        "$onehot" => Some(SysFuncId::OneHot),
        "$onehot0" => Some(SysFuncId::OneHot0),
        "$isunknown" => Some(SysFuncId::IsUnknown),
        // v7 random + time + plusarg probe ($value$plusargs is a special
        // form — it writes its ref var, never mapped here).
        "$test$plusargs" => Some(SysFuncId::TestPlusargs),
        "$random" => Some(SysFuncId::Random),
        "$urandom" => Some(SysFuncId::Urandom),
        "$urandom_range" => Some(SysFuncId::UrandomRange),
        "$stime" => Some(SysFuncId::Stime),
        // Round-9 FIO: $feof is the ONLY PURE file function — it reads the fd's
        // EOF flag with no state mutation, so it is allowed in any expression /
        // condition context (`while (!$feof(fd))`). The fd-ADVANCING reads
        // ($fgetc/$fgets/$fread/$fscanf/$sscanf/$ungetc) are NOT mapped here — they
        // stay direct-rhs-only (a second evaluation under unspecified expression
        // order would double-advance the fd). The direct-rhs `e = $feof(fd)` form
        // is still intercepted first by `file_read_int_special` (byte-identical),
        // so this mapping is reached only for expression-context $feof.
        "$feof" => Some(SysFuncId::Feof),
        // v19: N6 real-math (IEEE §20.8.2) — pure value functions, lowered like
        // any other SysFunc. The non-uniform $dist_* are NOT here — they advance
        // the ref seed and route through `dist_seeded_special` (direct-rhs only).
        "$ln" => Some(SysFuncId::Ln),
        "$log10" => Some(SysFuncId::Log10),
        "$exp" => Some(SysFuncId::Exp),
        "$sqrt" => Some(SysFuncId::Sqrt),
        "$pow" => Some(SysFuncId::Pow),
        "$floor" => Some(SysFuncId::Floor),
        "$ceil" => Some(SysFuncId::Ceil),
        "$sin" => Some(SysFuncId::Sin),
        "$cos" => Some(SysFuncId::Cos),
        "$tan" => Some(SysFuncId::Tan),
        "$asin" => Some(SysFuncId::Asin),
        "$acos" => Some(SysFuncId::Acos),
        "$atan" => Some(SysFuncId::Atan),
        "$atan2" => Some(SysFuncId::Atan2),
        "$hypot" => Some(SysFuncId::Hypot),
        "$sinh" => Some(SysFuncId::Sinh),
        "$cosh" => Some(SysFuncId::Cosh),
        "$tanh" => Some(SysFuncId::Tanh),
        "$asinh" => Some(SysFuncId::Asinh),
        "$acosh" => Some(SysFuncId::Acosh),
        "$atanh" => Some(SysFuncId::Atanh),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every name is non-empty and starts with the marker its family promises.
    /// Cheap, but it is what catches a copy-paste that left a `.method()` name
    /// on a `$task` id (the mapping is 122 hand-written arms).
    #[test]
    fn names_are_well_formed() {
        for n in [
            systask_name(SysTaskId::Display),
            systask_name(SysTaskId::ArrLocator),
            sysfunc_name(SysFuncId::Time),
            sysfunc_name(SysFuncId::DistErlang),
        ] {
            assert!(!n.is_empty());
            assert!(n.starts_with('$') || n.starts_with('.'), "{n}");
        }
    }

    /// The two `$cast` spellings MERGE on purpose — pinned so a later edit that
    /// disambiguates them has to come here and read why it should not.
    #[test]
    fn cast_task_and_function_share_one_name() {
        assert_eq!(
            systask_name(SysTaskId::Cast),
            sysfunc_name(SysFuncId::Cast),
            "both forms are `$cast` to the RTL author"
        );
    }

    /// A `$name` the way the IEEE spells one: `$` then `[a-z0-9_$]` (so `"$readmem
    /// target"`, a label, and the `.method()` names are not names).
    fn is_name_shape(t: &str) -> bool {
        t.len() > 1
            && t.starts_with('$')
            && t[1..]
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'$')
    }

    /// The lines of `fn <name>`'s body in this file's source (the match arms).
    fn body_of<'s>(src: &'s str, header: &str) -> Vec<&'s str> {
        let start = src
            .find(header)
            .unwrap_or_else(|| panic!("no `{header}` in names.rs"));
        src[start..]
            .lines()
            .skip(1)
            .take_while(|l| *l != "}")
            .collect()
    }

    /// The ids no `$name` maps to through [`sysfunc_id`]: each is minted by a special
    /// form in elaborate (a guard plus the variant at its own site), never by the name
    /// table (adding one to [`sysfunc_id`] would move a value: ROADMAP §5.2 SYS-1a F24).
    const NOT_FROM_SYSFUNC_ID: [(&str, &str); 17] = [
        ("Cast", "expr_cast.rs cast_special"),
        ("Fgets", "sys_special.rs fgets_special"),
        ("Fopen", "sys_special.rs fopen_special"),
        ("Fread", "sys_special.rs fread_special"),
        ("Sformatf", "sys_special.rs sformatf_special"),
        ("ValuePlusargs", "sys_special.rs value_plusargs_special"),
        ("DistUniform", "crv.rs dist_uniform_special"),
        ("DistNormal", "crv.rs dist_uniform_special"),
        ("DistExponential", "crv.rs dist_uniform_special"),
        ("DistPoisson", "crv.rs dist_uniform_special"),
        ("DistChiSquare", "crv.rs dist_uniform_special"),
        ("DistT", "crv.rs dist_uniform_special"),
        ("DistErlang", "crv.rs dist_uniform_special"),
        ("Fgetc", "sys_special.rs file_read_int_special"),
        ("Ungetc", "sys_special.rs file_read_int_special"),
        ("Fscanf", "sys_special.rs scanf_special"),
        ("Sscanf", "sys_special.rs scanf_special"),
    ];

    /// The round trip of the two directions (ROADMAP §5.2 SYS-1a, PINS 2): every name
    /// [`sysfunc_id`] answers prints back as itself through [`sysfunc_name`], and every
    /// `$name` [`sysfunc_name`] prints is either one of those (naming the same id) or an
    /// id in [`NOT_FROM_SYSFUNC_ID`], exactly. The arms are read from this file's source,
    /// so an arm added to either match is counted here.
    #[test]
    fn sysfunc_id_round_trips() {
        let src = include_str!("names.rs");
        // `F::Variant => "text",`
        let name_arms: Vec<(&str, &str)> = body_of(src, "pub fn sysfunc_name(")
            .into_iter()
            .filter_map(|l| {
                let (v, t) = l.trim().strip_prefix("F::")?.split_once(" => \"")?;
                Some((v, t.strip_suffix("\",")?))
            })
            .collect();
        // `"$name" => Some(SysFuncId::Variant),`
        let id_arms: Vec<(&str, &str)> = body_of(src, "pub fn sysfunc_id(")
            .into_iter()
            .filter_map(|l| {
                let (n, v) = l
                    .trim()
                    .strip_prefix('"')?
                    .split_once("\" => Some(SysFuncId::")?;
                Some((n, v.strip_suffix("),")?))
            })
            .collect();
        assert_eq!(name_arms.len(), 86, "sysfunc_name's arms");
        assert_eq!(id_arms.len(), 40, "sysfunc_id's domain");

        for (n, v) in &id_arms {
            let id =
                sysfunc_id(n).unwrap_or_else(|| panic!("{n} read from the source, not mapped"));
            assert_eq!(format!("{id:?}"), *v, "{n}");
            assert_eq!(sysfunc_name(id), *n, "{n} does not print back as itself");
        }

        let (mut mapped, mut minted, mut other) = (0, Vec::new(), 0);
        for (v, t) in &name_arms {
            if !is_name_shape(t) {
                other += 1;
                continue;
            }
            match sysfunc_id(t) {
                Some(id) => {
                    assert_eq!(format!("{id:?}"), *v, "{t} maps to another id");
                    mapped += 1;
                }
                None => minted.push(*v),
            }
        }
        let listed: Vec<&str> = NOT_FROM_SYSFUNC_ID.iter().map(|(v, _)| *v).collect();
        minted.sort_unstable();
        let mut want = listed.clone();
        want.sort_unstable();
        assert_eq!(minted, want, "the `$name` ids sysfunc_id does not answer");
        assert_eq!((mapped, minted.len(), other), (40, 17, 29));
        assert!(!is_name_shape(sysfunc_name(SysFuncId::ReadmemAssocTarget)));
    }
}
