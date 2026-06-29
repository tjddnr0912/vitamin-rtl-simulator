//! vita-artifact-derive — `#[derive(SchemaHash)]`
//! Emits `impl vita_schema::SchemaShape` for the annotated (concrete) type.
//! Verified against syn 2.0.x. MSRV 1.82, edition 2021.
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Error, Expr, Fields,
    GenericArgument, GenericParam, LitStr, Meta, PathArguments, Type, TypeArray, TypePath,
    TypeTuple, Variant,
};

#[proc_macro_derive(SchemaHash, attributes(serde))]
pub fn derive_schema_hash(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match expand(&ast) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(ast: &DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    // (6) Hard guard: no generics of any kind (frozen schema types are monomorphic).
    if let Some(p) = ast.generics.params.iter().next() {
        let what = match p {
            GenericParam::Type(_) => "type",
            GenericParam::Lifetime(_) => "lifetime",
            GenericParam::Const(_) => "const-generic",
        };
        return Err(Error::new_spanned(
            p,
            format!("SchemaHash does not support {what} parameters; schema types must be concrete"),
        ));
    }

    let ident = &ast.ident;
    let ident_str = ident.to_string();
    let container_serde = render_serde_attrs(&ast.attrs)?;

    let mut children: Vec<String> = Vec::new();
    let shape_body = render_local_shape(ast, &mut children)?;
    // local_shape carries the structural body only; the registry prepends
    // schema_name() (module_path! is a runtime token, not bakeable here).
    let local_shape_str = format!("repr=@#[{container_serde}]{shape_body}");

    let child_paths: Vec<Type> = children
        .iter()
        .map(|s| syn::parse_str::<Type>(s))
        .collect::<Result<_, _>>()?;
    let register_calls = child_paths.iter().map(|ty| {
        quote! { <#ty as vita_schema::SchemaShape>::register(reg); }
    });

    Ok(quote! {
        impl vita_schema::SchemaShape for #ident {
            fn schema_name() -> &'static str {
                // module_path! emitted as a TOKEN — expanded by the compiler in the
                // crate/module where #[derive] is written (the type's defining module),
                // NOT inside this macro body. So schema_name() is the type's own FQ path.
                ::core::concat!(::core::module_path!(), "::", #ident_str)
            }
            fn local_shape() -> &'static str { #local_shape_str }
            fn register(reg: &mut vita_schema::ShapeRegistry) {
                if !reg.insert_once(
                    <Self as vita_schema::SchemaShape>::schema_name(),
                    <Self as vita_schema::SchemaShape>::local_shape(),
                ) {
                    return;
                }
                #( #register_calls )*
            }
        }
    })
}

fn render_local_shape(ast: &DeriveInput, children: &mut Vec<String>) -> Result<String, Error> {
    match &ast.data {
        Data::Struct(s) => render_struct(s, children),
        Data::Enum(e) => render_enum(e, children),
        Data::Union(u) => Err(Error::new_spanned(
            u.union_token,
            "SchemaHash does not support unions",
        )),
    }
}

fn render_struct(s: &DataStruct, children: &mut Vec<String>) -> Result<String, Error> {
    Ok(match &s.fields {
        Fields::Named(named) => {
            let mut parts = Vec::new();
            for f in &named.named {
                let attrs = render_serde_attrs(&f.attrs)?;
                let name = f.ident.as_ref().unwrap();
                let texpr = render_type_expr(&f.ty, children)?;
                parts.push(format!("#[{attrs}]{name}:{texpr}"));
            }
            format!("struct{{{}}}", parts.join(","))
        }
        Fields::Unnamed(unnamed) => {
            let mut parts = Vec::new();
            for f in &unnamed.unnamed {
                let attrs = render_serde_attrs(&f.attrs)?;
                let texpr = render_type_expr(&f.ty, children)?;
                parts.push(format!("#[{attrs}]{texpr}"));
            }
            if parts.len() == 1 {
                format!("newtype({})", parts[0])
            } else {
                format!("tuple({})", parts.join(","))
            }
        }
        Fields::Unit => "unit".to_string(),
    })
}

fn render_enum(e: &DataEnum, children: &mut Vec<String>) -> Result<String, Error> {
    let mut variants = Vec::new();
    for v in &e.variants {
        variants.push(render_variant(v, children)?);
    }
    Ok(format!("enum{{{}}}", variants.join(",")))
}

fn render_variant(v: &Variant, children: &mut Vec<String>) -> Result<String, Error> {
    let attrs = render_serde_attrs(&v.attrs)?;
    let name = v.ident.to_string();
    let disc = match &v.discriminant {
        Some((_eq, expr)) => format!("={}", render_disc_expr(expr)),
        None => String::new(),
    };
    let body = match &v.fields {
        Fields::Unit => String::new(),
        Fields::Named(named) => {
            let mut parts = Vec::new();
            for f in &named.named {
                let fattrs = render_serde_attrs(&f.attrs)?;
                let fname = f.ident.as_ref().unwrap();
                let texpr = render_type_expr(&f.ty, children)?;
                parts.push(format!("#[{fattrs}]{fname}:{texpr}"));
            }
            format!("{{{}}}", parts.join(","))
        }
        Fields::Unnamed(unnamed) => {
            let mut parts = Vec::new();
            for f in &unnamed.unnamed {
                let fattrs = render_serde_attrs(&f.attrs)?;
                let texpr = render_type_expr(&f.ty, children)?;
                parts.push(format!("#[{fattrs}]{texpr}"));
            }
            format!("({})", parts.join(","))
        }
    };
    Ok(format!("#[{attrs}]{name}{disc}{body}"))
}

fn render_disc_expr(expr: &Expr) -> String {
    quote!(#expr).to_string().split_whitespace().collect()
}

const PRIMITIVES: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128", "bool", "char", "str",
    "String",
];

fn render_type_expr(ty: &Type, children: &mut Vec<String>) -> Result<String, Error> {
    match ty {
        Type::Array(TypeArray { elem, len, .. }) => {
            let inner = render_type_expr(elem, children)?;
            let n: String = quote!(#len).to_string().split_whitespace().collect();
            Ok(format!("[{inner};{n}]"))
        }
        Type::Tuple(TypeTuple { elems, .. }) => {
            let mut inner = Vec::new();
            for e in elems {
                inner.push(render_type_expr(e, children)?);
            }
            Ok(format!("({})", inner.join(",")))
        }
        Type::Reference(r) => render_type_expr(&r.elem, children),
        Type::Path(tp) => render_path_type(tp, children),
        Type::Paren(p) => render_type_expr(&p.elem, children),
        Type::Group(g) => render_type_expr(&g.elem, children),
        other => Err(Error::new_spanned(
            other,
            "SchemaHash: unsupported field type construct",
        )),
    }
}

fn render_path_type(tp: &TypePath, children: &mut Vec<String>) -> Result<String, Error> {
    if tp.qself.is_some() {
        return Err(Error::new_spanned(
            tp,
            "SchemaHash: qualified-path (<T as Trait>) field types are not supported",
        ));
    }
    let last = tp
        .path
        .segments
        .last()
        .ok_or_else(|| Error::new_spanned(tp, "SchemaHash: empty type path"))?;
    let head = last.ident.to_string();
    if head == "HashMap" || head == "HashSet" {
        return Err(Error::new_spanned(
            tp,
            format!(
                "SchemaHash: `{head}` is forbidden (nondeterministic order); use BTreeMap/BTreeSet"
            ),
        ));
    }
    if matches!(head.as_str(), "usize" | "isize" | "f32" | "f64") {
        return Err(Error::new_spanned(
            tp,
            format!(
                "SchemaHash: `{head}` is forbidden in schema types \
                 (platform-variant width / float breaks 3-OS byte-identity); \
                 use a fixed-width integer (u32/u64) instead"
            ),
        ));
    }
    let args: Vec<&Type> = match &last.arguments {
        PathArguments::AngleBracketed(ab) => ab
            .args
            .iter()
            .filter_map(|a| match a {
                GenericArgument::Type(t) => Some(t),
                _ => None,
            })
            .collect(),
        PathArguments::None => Vec::new(),
        PathArguments::Parenthesized(p) => {
            return Err(Error::new_spanned(
                p,
                "SchemaHash: Fn-style type args not supported",
            ))
        }
    };
    match (head.as_str(), args.len()) {
        ("Option", 1) => Ok(format!("Option<{}>", render_type_expr(args[0], children)?)),
        ("Vec", 1) => Ok(format!("Vec<{}>", render_type_expr(args[0], children)?)),
        ("BTreeSet", 1) => Ok(format!(
            "BTreeSet<{}>",
            render_type_expr(args[0], children)?
        )),
        ("BTreeMap", 2) => Ok(format!(
            "BTreeMap<{},{}>",
            render_type_expr(args[0], children)?,
            render_type_expr(args[1], children)?
        )),
        // `Box<T>` is a transparent heap pointer: postcard serializes `Box<T>`
        // byte-identically to `T`, and the schema shape must match too. Render it
        // as the inner type (so a field `Box<Expr>` has the SAME shape as `Expr`,
        // and the recursive `Box<Expr>` inside `Expr` registers `Expr` once —
        // the registry's `insert_once` guard short-circuits the self-reference).
        ("Box", 1) => render_type_expr(args[0], children),
        (h, 0) if PRIMITIVES.contains(&h) => Ok(h.to_string()),
        _ => {
            let full = render_full_path(tp);
            if !children.iter().any(|c| c == &full) {
                children.push(full.clone()); // macro-time dedup, source order, plain Vec
            }
            Ok(full)
        }
    }
}

fn render_full_path(tp: &TypePath) -> String {
    tp.path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn render_serde_attrs(attrs: &[syn::Attribute]) -> Result<String, Error> {
    let mut slots: Vec<(usize, String)> = Vec::new();
    let order = |k: &str| -> usize {
        match k {
            "rename" => 0,
            "rename_all" => 1,
            "skip" => 2,
            "skip_serializing_if" => 3,
            "with" => 4,
            "default" => 5,
            "flatten" => 6,
            "tag" => 7,
            "content" => 8,
            "untagged" => 9,
            "transparent" => 10,
            "deny_unknown_fields" => 11,
            "alias" => 12,
            "other" => 13,
            _ => usize::MAX,
        }
    };
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        if let Meta::Path(_) = &attr.meta {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            let key = meta
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();
            let o = order(&key);
            match key.as_str() {
                "rename"
                | "rename_all"
                | "skip_serializing_if"
                | "with"
                | "tag"
                | "content"
                | "alias" => {
                    let lit: LitStr = meta.value()?.parse()?;
                    slots.push((o, format!("{key}={:?}", lit.value())));
                    Ok(())
                }
                "default" => {
                    if meta.input.peek(syn::Token![=]) {
                        let lit: LitStr = meta.value()?.parse()?;
                        slots.push((o, format!("default={:?}", lit.value())));
                    } else {
                        slots.push((o, "default".to_string()));
                    }
                    Ok(())
                }
                "skip"
                | "flatten"
                | "untagged"
                | "transparent"
                | "deny_unknown_fields"
                | "other" => {
                    slots.push((o, key));
                    Ok(())
                }
                _ => {
                    if meta.input.peek(syn::Token![=]) {
                        let _: Expr = meta.value()?.parse()?;
                    } else if meta.input.peek(syn::token::Paren) {
                        meta.parse_nested_meta(|_| Ok(()))?;
                    }
                    Ok(())
                }
            }
        })
        .map_err(|e| Error::new_spanned(attr, format!("SchemaHash: bad #[serde(..)]: {e}")))?;
    }
    slots.sort();
    Ok(slots
        .into_iter()
        .map(|(_, s)| s)
        .collect::<Vec<_>>()
        .join(","))
}
