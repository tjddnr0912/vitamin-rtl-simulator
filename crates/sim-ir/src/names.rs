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
//! and that copy is now this one's caller.
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
}
