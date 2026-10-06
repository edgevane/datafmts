use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, GenericParam, Type};

/// Derive Encode+Decode+Schema. Attrs: rename, rename_all, default, skip.
#[proc_macro_derive(DataStruct, attributes(datafmt))]
pub fn derive_data_struct(input: TokenStream) -> TokenStream {
    impl_derive(parse_macro_input!(input as DeriveInput))
}

/// Lowercase alias for DataStruct derive.
#[proc_macro_derive(data_struct, attributes(datafmt))]
pub fn derive_data_struct_snake(input: TokenStream) -> TokenStream {
    impl_derive(parse_macro_input!(input as DeriveInput))
}

#[derive(Default, Clone)]
struct ContainerMeta {
    rename: Option<String>,
    rename_all: Option<String>,
}

#[derive(Default, Clone)]
struct FieldMeta {
    rename: Option<String>,
    skip: bool,
    default: Option<Option<syn::Path>>,
}

fn parse_container_meta(attrs: &[syn::Attribute]) -> ContainerMeta {
    let mut out = ContainerMeta::default();
    for attr in attrs {
        if !attr.path().is_ident("datafmt") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let v: syn::LitStr = meta.value()?.parse()?;
                out.rename = Some(v.value());
            } else if meta.path.is_ident("rename_all") {
                let v: syn::LitStr = meta.value()?.parse()?;
                out.rename_all = Some(v.value());
            }
            Ok(())
        });
    }
    out
}

fn parse_field_meta(attrs: &[syn::Attribute]) -> FieldMeta {
    let mut out = FieldMeta::default();
    for attr in attrs {
        if !attr.path().is_ident("datafmt") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let v: syn::LitStr = meta.value()?.parse()?;
                out.rename = Some(v.value());
            } else if meta.path.is_ident("skip") {
                out.skip = true;
            } else if meta.path.is_ident("default") {
                if meta.input.peek(syn::Token![=]) {
                    let v: syn::LitStr = meta.value()?.parse()?;
                    let p: syn::Path = syn::parse_str(&v.value()).unwrap_or_else(|_| {
                        syn::parse_str::<syn::Path>("::core::default::Default::default")
                            .expect("fallback path")
                    });
                    out.default = Some(Some(p));
                } else {
                    out.default = Some(None);
                }
            }
            Ok(())
        });
    }
    // skip implies default so decode can construct field
    if out.skip && out.default.is_none() {
        out.default = Some(None);
    }
    out
}

fn apply_rename_all(rule: &str, name: &str) -> String {
    match rule {
        "camelCase" => {
            let mut out = String::new();
            for (i, part) in name.split('_').enumerate() {
                if i == 0 {
                    out.push_str(part);
                } else {
                    let mut c = part.chars();
                    if let Some(f) = c.next() {
                        out.extend(f.to_uppercase());
                        out.push_str(c.as_str());
                    }
                }
            }
            out
        }
        "PascalCase" => name
            .split('_')
            .map(|p| {
                let mut c = p.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect(),
        "kebab-case" => name.replace('_', "-"),
        "SCREAMING_SNAKE_CASE" => name.to_uppercase(),
        "lowercase" => name.to_lowercase().replace('_', ""),
        "UPPERCASE" => name.to_uppercase().replace('_', ""),
        "snake_case" => name.to_owned(),
        _ => name.to_owned(),
    }
}

fn effective_name(raw: &str, field_meta: Option<&FieldMeta>, container: &ContainerMeta) -> String {
    if let Some(m) = field_meta {
        if let Some(r) = &m.rename {
            return r.clone();
        }
    }
    if let Some(rule) = &container.rename_all {
        apply_rename_all(rule, raw)
    } else {
        raw.to_owned()
    }
}

fn used_type_params(ty: &Type, params: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let s = quote!(#ty).to_string();
    for p in params {
        let needle: Vec<String> = vec![
            format!(" {p} "),
            format!(" {p},"),
            format!(" {p}>"),
            format!("<{p} "),
            format!("<{p},"),
            format!("<{p}>"),
            format!("({p} "),
            format!("({p},"),
            format!("[{p} "),
        ];
        let padded = format!(" {s} ");
        if needle.iter().any(|n| padded.contains(n)) || padded.contains(&format!(" {p} ")) {
            if !out.contains(p) {
                out.push(p.clone());
            }
        }
    }
    out
}

// generated generics use __Enc/__Dec: plain E/D would collide with user type E
fn impl_derive(input: DeriveInput) -> TokenStream {
    let name = input.ident.clone();
    let container = parse_container_meta(&input.attrs);
    let type_name = container.rename.clone().unwrap_or_else(|| name.to_string());

    let type_params: Vec<String> = input
        .generics
        .params
        .iter()
        .filter_map(|p| match p {
            GenericParam::Type(t) => Some(t.ident.to_string()),
            _ => None,
        })
        .collect();

    match &input.data {
        Data::Struct(ds) => impl_struct(
            &input,
            &name,
            &type_name,
            &container,
            &ds.fields,
            &type_params,
        ),
        Data::Enum(de) => impl_enum(&input, &name, &type_name, &container, de, &type_params),
        Data::Union(_) => {
            syn::Error::new_spanned(&name, "datafmts: unions not supported, use struct or enum")
                .to_compile_error()
                .into()
        }
    }
}

struct FieldInfo {
    raw: String,
    member: proc_macro2::TokenStream,
    binding: proc_macro2::Ident,
    ty: Type,
    schema_name: String,
    has_default: bool,
    default_expr: proc_macro2::TokenStream,
    active: bool, // false when skipped
}

fn collect_fields(fields: &Fields, container: &ContainerMeta, is_tuple: bool) -> Vec<FieldInfo> {
    let mut out = Vec::new();
    for (i, f) in fields.iter().enumerate() {
        let meta = parse_field_meta(&f.attrs);
        let raw = if let Some(id) = &f.ident {
            id.to_string()
        } else {
            i.to_string()
        };
        let schema_name = if is_tuple {
            meta.rename.clone().unwrap_or_else(|| raw.clone())
        } else {
            effective_name(&raw, Some(&meta), container)
        };
        let member = if let Some(id) = &f.ident {
            quote!(#id)
        } else {
            let idx = syn::Index::from(i);
            quote!(#idx)
        };
        let binding = format_ident!("__f{}", i);
        let has_default = meta.default.is_some();
        let default_expr = match &meta.default {
            None => quote!(return ::core::result::Result::Err(__e)),
            Some(None) => quote!(::core::default::Default::default()),
            Some(Some(p)) => quote!(#p()),
        };
        out.push(FieldInfo {
            raw,
            member,
            binding,
            ty: f.ty.clone(),
            schema_name,
            has_default,
            default_expr,
            active: !meta.skip,
        });
    }
    out
}

fn add_bounds(
    generics: &syn::Generics,
    used: &[String],
    bound: proc_macro2::TokenStream,
) -> syn::Generics {
    let mut g = generics.clone();
    for tp in g.type_params_mut() {
        if used.contains(&tp.ident.to_string()) {
            tp.bounds.push(syn::parse2(bound.clone()).expect("bound"));
        }
    }
    g
}

fn impl_struct(
    input: &DeriveInput,
    name: &syn::Ident,
    type_name: &str,
    container: &ContainerMeta,
    fields: &Fields,
    type_params: &[String],
) -> TokenStream {
    let is_unit = matches!(fields, Fields::Unit);
    let is_tuple = matches!(fields, Fields::Unnamed(_));

    if is_unit {
        let (impl_g, ty_g, where_c) = input.generics.split_for_impl();
        return quote! {
            impl #impl_g ::datafmts::Encode for #name #ty_g #where_c {
                fn encode<__Enc: ::datafmts::Encoder + ?Sized>(&self, e: &mut __Enc)
                    -> ::core::result::Result<(), __Enc::Error> {
                    e.encode_unit()
                }
            }
            impl #impl_g ::datafmts::Decode for #name #ty_g #where_c {
                fn decode<__Dec: ::datafmts::Decoder + ?Sized>(d: &mut __Dec)
                    -> ::core::result::Result<Self, __Dec::Error>
                where
                    __Dec::Error: ::core::convert::From<::datafmts::Error>,
                {
                    d.decode_unit()?;
                    ::core::result::Result::Ok(Self)
                }
            }
            impl #impl_g ::datafmts::DataStruct for #name #ty_g #where_c {
                const SCHEMA: &'static ::datafmts::Schema = &::datafmts::Schema {
                    name: #type_name,
                    kind: ::datafmts::Kind::Struct(&::datafmts::StructSchema {
                        name: #type_name,
                        fields: &[],
                        kind: ::datafmts::StructKind::Unit,
                    }),
                };
            }
        }
        .into();
    }

    let infos = collect_fields(fields, container, is_tuple);
    let active: Vec<&FieldInfo> = infos.iter().filter(|f| f.active).collect();

    let mut used = Vec::new();
    for f in &active {
        for p in used_type_params(&f.ty, type_params) {
            if !used.contains(&p) {
                used.push(p);
            }
        }
    }

    let enc_g = add_bounds(&input.generics, &used, quote!(::datafmts::Encode));
    let dec_g = add_bounds(&input.generics, &used, quote!(::datafmts::Decode));
    let sch_g = add_bounds(&input.generics, &used, quote!(::datafmts::DataStruct));
    let (enc_impl, _, enc_where) = enc_g.split_for_impl();
    let (dec_impl, _, dec_where) = dec_g.split_for_impl();
    let (sch_impl, _, sch_where) = sch_g.split_for_impl();
    let (_, ty_g, _) = input.generics.split_for_impl();

    let enc_stmts: Vec<_> = active
        .iter()
        .map(|f| {
            let m = &f.member;
            if is_tuple {
                quote!(::datafmts::Encode::encode(&self.#m, e)?;)
            } else {
                quote!(::datafmts::Encode::encode(&self.#m, e)?;)
            }
        })
        .collect();

    let dec_stmts: Vec<_> = infos
        .iter()
        .filter(|f| f.active)
        .map(|f| {
            let b = &f.binding;
            let ty = &f.ty;
            if f.has_default {
                let dflt = &f.default_expr;
                quote! {
                    let #b: #ty = match ::datafmts::Decode::decode(d) {
                        ::core::result::Result::Ok(v) => v,
                        ::core::result::Result::Err(__e) => {
                            let __dflt: #ty = #dflt;
                            __dflt
                        }
                    };
                }
            } else {
                quote! {
                    let #b: #ty = ::datafmts::Decode::decode(d)?;
                }
            }
        })
        .collect();

    let skipped_inits: Vec<_> = infos
        .iter()
        .filter(|f| !f.active)
        .map(|f| {
            let m = &f.member;
            let ty = &f.ty;
            let dflt = &f.default_expr;
            if is_tuple {
                quote!(#dflt)
            } else {
                let _ty = ty;
                quote!(#m: #dflt)
            }
        })
        .collect();

    let ctor = if is_tuple {
        let bindings: Vec<_> = infos
            .iter()
            .map(|f| {
                if f.active {
                    let b = &f.binding;
                    quote!(#b)
                } else {
                    let d = &f.default_expr;
                    quote!(#d)
                }
            })
            .collect();
        let _ = skipped_inits;
        quote!(Self(#(#bindings,)*))
    } else {
        let inits: Vec<_> = infos
            .iter()
            .map(|f| {
                let m = &f.member;
                if f.active {
                    let b = &f.binding;
                    quote!(#m: #b)
                } else {
                    let d = &f.default_expr;
                    quote!(#m: #d)
                }
            })
            .collect();
        quote!(Self { #(#inits,)* })
    };

    let schema_fields: Vec<_> = active
        .iter()
        .map(|f| {
            let n = &f.schema_name;
            let ty = &f.ty;
            let hd = f.has_default;
            quote!(::datafmts::Field {
                name: #n,
                schema: ::datafmts::schema_of::<#ty>,
                has_default: #hd,
            })
        })
        .collect();

    let struct_kind = if is_tuple {
        quote!(::datafmts::StructKind::Tuple)
    } else {
        quote!(::datafmts::StructKind::Named)
    };

    let _raws: Vec<&str> = active.iter().map(|f| f.raw.as_str()).collect();

    quote! {
        impl #enc_impl ::datafmts::Encode for #name #ty_g #enc_where {
            fn encode<__Enc: ::datafmts::Encoder + ?Sized>(&self, e: &mut __Enc)
                -> ::core::result::Result<(), __Enc::Error> {
                #(#enc_stmts)*
                ::core::result::Result::Ok(())
            }
        }
        impl #dec_impl ::datafmts::Decode for #name #ty_g #dec_where {
            fn decode<__Dec: ::datafmts::Decoder + ?Sized>(d: &mut __Dec)
                -> ::core::result::Result<Self, __Dec::Error>
            where
                __Dec::Error: ::core::convert::From<::datafmts::Error>,
            {
                #(#dec_stmts)*
                ::core::result::Result::Ok(#ctor)
            }
        }
        impl #sch_impl ::datafmts::DataStruct for #name #ty_g #sch_where {
            const SCHEMA: &'static ::datafmts::Schema = &::datafmts::Schema {
                name: #type_name,
                kind: ::datafmts::Kind::Struct(&::datafmts::StructSchema {
                    name: #type_name,
                    fields: &[#(#schema_fields,)*],
                    kind: #struct_kind,
                }),
            };
        }
    }
    .into()
}

fn impl_enum(
    input: &DeriveInput,
    name: &syn::Ident,
    type_name: &str,
    container: &ContainerMeta,
    de: &syn::DataEnum,
    type_params: &[String],
) -> TokenStream {
    let mut used = Vec::new();
    struct VarInfo {
        ident: syn::Ident,
        raw: String,
        schema_name: String,
        index: u32,
        fields: Vec<FieldInfo>,
        is_tuple: bool,
        is_unit: bool,
    }
    let mut vars: Vec<VarInfo> = Vec::new();
    for (idx, v) in de.variants.iter().enumerate() {
        let vmeta_raw = parse_field_meta(&v.attrs);
        let cmeta_for_variant = ContainerMeta {
            rename: vmeta_raw.rename.clone(),
            rename_all: container.rename_all.clone(),
        };
        let schema_name = if cmeta_for_variant.rename.is_some() {
            cmeta_for_variant.rename.clone().unwrap()
        } else if let Some(rule) = &container.rename_all {
            apply_rename_all(rule, &v.ident.to_string())
        } else {
            v.ident.to_string()
        };
        let is_tuple = matches!(v.fields, Fields::Unnamed(_));
        let is_unit = matches!(v.fields, Fields::Unit);
        let v_container = ContainerMeta {
            rename: None,
            rename_all: container.rename_all.clone(),
        };
        let finfos = collect_fields(&v.fields, &v_container, is_tuple);
        for f in finfos.iter().filter(|f| f.active) {
            for p in used_type_params(&f.ty, type_params) {
                if !used.contains(&p) {
                    used.push(p);
                }
            }
        }
        vars.push(VarInfo {
            ident: v.ident.clone(),
            raw: v.ident.to_string(),
            schema_name,
            index: idx as u32,
            fields: finfos,
            is_tuple,
            is_unit,
        });
    }

    let enc_g = add_bounds(&input.generics, &used, quote!(::datafmts::Encode));
    let dec_g = add_bounds(&input.generics, &used, quote!(::datafmts::Decode));
    let sch_g = add_bounds(&input.generics, &used, quote!(::datafmts::DataStruct));
    let (enc_impl, _, enc_where) = enc_g.split_for_impl();
    let (dec_impl, _, dec_where) = dec_g.split_for_impl();
    let (sch_impl, _, sch_where) = sch_g.split_for_impl();
    let (_, ty_g, _) = input.generics.split_for_impl();

    let enc_arms: Vec<_> = vars
        .iter()
        .map(|v| {
            let ident = &v.ident;
            let idx = v.index;
            if v.is_unit {
                quote!(#name::#ident => { e.encode_u32(#idx)?; })
            } else if v.is_tuple {
                let binds: Vec<_> = (0..v.fields.iter().filter(|f| f.active).count())
                    .map(|i| format_ident!("__v{}", i))
                    .collect();
                quote!(#name::#ident(#(#binds,)* ..) => {
                    e.encode_u32(#idx)?;
                    #(::datafmts::Encode::encode(#binds, e)?;)*
                })
            } else {
                let members: Vec<_> = v
                    .fields
                    .iter()
                    .filter(|f| f.active)
                    .map(|f| f.member.clone())
                    .collect();
                quote!(#name::#ident { #(#members,)* .. } => {
                    e.encode_u32(#idx)?;
                    #(::datafmts::Encode::encode(#members, e)?;)*
                })
            }
        })
        .collect();

    let dec_arms: Vec<_> = vars
        .iter()
        .map(|v| {
            let ident = &v.ident;
            let idx = v.index;
            if v.is_unit {
                quote!(#idx => ::core::result::Result::Ok(#name::#ident),)
            } else {
                let stmts: Vec<_> = v
                    .fields
                    .iter()
                    .filter(|f| f.active)
                    .map(|f| {
                        let b = &f.binding;
                        let ty = &f.ty;
                        if f.has_default {
                            let dflt = &f.default_expr;
                            quote! {
                                let #b: #ty = match ::datafmts::Decode::decode(d) {
                                    ::core::result::Result::Ok(x) => x,
                                    ::core::result::Result::Err(__e) => {
                                        let __d: #ty = #dflt;
                                        __d
                                    }
                                };
                            }
                        } else {
                            quote! {
                                let #b: #ty = ::datafmts::Decode::decode(d)?;
                            }
                        }
                    })
                    .collect();
                let ctor = if v.is_tuple {
                    let parts: Vec<_> = v
                        .fields
                        .iter()
                        .map(|f| {
                            if f.active {
                                let b = &f.binding;
                                quote!(#b)
                            } else {
                                let d = &f.default_expr;
                                quote!(#d)
                            }
                        })
                        .collect();
                    quote!(#name::#ident(#(#parts,)*))
                } else {
                    let parts: Vec<_> = v
                        .fields
                        .iter()
                        .map(|f| {
                            let m = &f.member;
                            if f.active {
                                let b = &f.binding;
                                quote!(#m: #b)
                            } else {
                                let d = &f.default_expr;
                                quote!(#m: #d)
                            }
                        })
                        .collect();
                    quote!(#name::#ident { #(#parts,)* })
                };
                quote!(#idx => { #(#stmts)* ::core::result::Result::Ok(#ctor) },)
            }
        })
        .collect();

    let schema_vars: Vec<_> = vars
        .iter()
        .map(|v| {
            let n = &v.schema_name;
            let idx = v.index;
            if v.is_unit {
                quote!(::datafmts::Variant {
                    name: #n,
                    index: #idx,
                    kind: ::datafmts::VariantKind::Unit,
                })
            } else if v.is_tuple {
                let elems: Vec<_> = v
                    .fields
                    .iter()
                    .filter(|f| f.active)
                    .map(|f| {
                        let ty = &f.ty;
                        let hd = f.has_default;
                        quote!(::datafmts::TupleElem {
                            schema: ::datafmts::schema_of::<#ty>,
                            has_default: #hd,
                        })
                    })
                    .collect();
                quote!(::datafmts::Variant {
                    name: #n,
                    index: #idx,
                    kind: ::datafmts::VariantKind::Tuple(&[#(#elems,)*]),
                })
            } else {
                let sfields: Vec<_> = v
                    .fields
                    .iter()
                    .filter(|f| f.active)
                    .map(|f| {
                        let n = &f.schema_name;
                        let ty = &f.ty;
                        let hd = f.has_default;
                        quote!(::datafmts::Field {
                            name: #n,
                            schema: ::datafmts::schema_of::<#ty>,
                            has_default: #hd,
                        })
                    })
                    .collect();
                let vstruct = format!("{}::{}", type_name, v.schema_name);
                quote!(::datafmts::Variant {
                    name: #n,
                    index: #idx,
                    kind: ::datafmts::VariantKind::Struct(&::datafmts::StructSchema {
                        name: #vstruct,
                        fields: &[#(#sfields,)*],
                        kind: ::datafmts::StructKind::Named,
                    }),
                })
            }
        })
        .collect();

    quote! {
        impl #enc_impl ::datafmts::Encode for #name #ty_g #enc_where {
            fn encode<__Enc: ::datafmts::Encoder + ?Sized>(&self, e: &mut __Enc)
                -> ::core::result::Result<(), __Enc::Error> {
                match self {
                    #(#enc_arms)*
                }
                ::core::result::Result::Ok(())
            }
        }
        impl #dec_impl ::datafmts::Decode for #name #ty_g #dec_where {
            fn decode<__Dec: ::datafmts::Decoder + ?Sized>(d: &mut __Dec)
                -> ::core::result::Result<Self, __Dec::Error>
            where
                __Dec::Error: ::core::convert::From<::datafmts::Error>,
            {
                let __idx: u32 = ::datafmts::Decode::decode(d)?;
                match __idx {
                    #(#dec_arms)*
                    __other => ::core::result::Result::Err(
                        __Dec::Error::from(::datafmts::Error::InvalidVariant(__other)),
                    ),
                }
            }
        }
        impl #sch_impl ::datafmts::DataStruct for #name #ty_g #sch_where {
            const SCHEMA: &'static ::datafmts::Schema = &::datafmts::Schema {
                name: #type_name,
                kind: ::datafmts::Kind::Enum(&::datafmts::EnumSchema {
                    name: #type_name,
                    variants: &[#(#schema_vars,)*],
                }),
            };
        }
    }
    .into()
}
