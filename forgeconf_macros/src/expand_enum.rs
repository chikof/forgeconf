//! Enum expansion: unit, newtype and struct variants, externally tagged by
//! default or internally tagged with `tag = "..."`.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::{DataEnum, DeriveInput, Error, Fields, Ident, Type};

use crate::attr::{Errors, enum_attrs, variant_attrs};
use crate::expand_struct::{
    FieldDef,
    doc_string,
    from_table,
    owned_supported,
    parse_fields,
    struct_info_statics,
};

enum Kind {
    Unit,
    Newtype(Box<Type>),
    Struct(Vec<FieldDef>),
}

struct Variant {
    ident: Ident,
    key: String,
    aliases: Vec<String>,
    docs: Option<String>,
    kind: Kind,
}

impl Variant {
    fn names(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.key.as_str()).chain(self.aliases.iter().map(String::as_str))
    }

    fn pattern(&self) -> TokenStream {
        let names = self.names();
        quote!(#(#names)|*)
    }
}

fn parse_variants(
    data: &DataEnum,
    rename_all: Option<crate::case::Case>,
    errors: &mut Errors,
) -> Vec<Variant> {
    let mut out = Vec::new();
    for variant in &data.variants {
        let attrs = variant_attrs(&variant.attrs, errors);
        let plain = variant.ident.unraw().to_string();
        let key = match (&attrs.rename, rename_all) {
            (Some(lit), _) => lit.value(),
            (None, Some(case)) => case.apply_variant(&plain),
            (None, None) => plain,
        };
        let kind = match &variant.fields {
            Fields::Unit => Kind::Unit,
            Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                for field in &unnamed.unnamed {
                    if let Some(attr) = field.attrs.iter().find(|a| a.path().is_ident("config")) {
                        errors.push(Error::new_spanned(
                            attr,
                            "`config` attributes are not supported on the field of a newtype \
                             variant",
                        ));
                    }
                }
                Kind::Newtype(Box::new(unnamed.unnamed[0].ty.clone()))
            },
            Fields::Unnamed(unnamed) => {
                errors.push(Error::new_spanned(
                    unnamed,
                    "tuple variants must have exactly one field; use a struct variant instead",
                ));
                Kind::Unit
            },
            Fields::Named(named) => Kind::Struct(parse_fields(named, None, errors)),
        };
        out.push(Variant {
            ident: variant.ident.clone(),
            key,
            aliases: attrs.aliases.iter().map(syn::LitStr::value).collect(),
            docs: doc_string(&variant.attrs),
            kind,
        });
    }
    let mut seen = HashSet::new();
    for (variant, source) in out.iter().zip(&data.variants) {
        for name in variant.names() {
            if !seen.insert(name.to_owned()) {
                errors.push(Error::new(
                    source.ident.span(),
                    format!("duplicate variant key \"{name}\""),
                ));
            }
        }
    }
    out
}

fn unknown_variant(found: &TokenStream, origin: &TokenStream, suffix: &TokenStream) -> TokenStream {
    quote! {
        ::core::result::Result::Err(
            ::forgeconf::ConfigError::new(::forgeconf::ErrorKind::UnknownVariant {
                found: ::core::convert::Into::into(#found),
                expected: __EXPECTED,
            })
            .with_origin(#origin) #suffix
        )
    }
}

fn keys<'a>(variants: impl Iterator<Item = &'a Variant>) -> Vec<&'a str> {
    variants.map(|v| v.key.as_str()).collect()
}

pub(crate) fn expand(input: &DeriveInput, data: &DataEnum, errors: &mut Errors) -> TokenStream {
    let ident = &input.ident;
    let name = ident.unraw().to_string();
    let attrs = enum_attrs(&input.attrs, errors);
    if data.variants.is_empty() {
        errors.push(Error::new_spanned(ident, "enums without variants cannot derive `Config`"));
    }
    let variants = parse_variants(data, attrs.rename_all, errors);
    let tag = attrs.tag.as_ref().map(syn::LitStr::value);

    let from_node = match &tag {
        Some(tag) => tagged_body(&name, tag, &variants, false),
        None => external_body(&name, &variants, false),
    };
    let owned = owned_variants(&variants).then(|| {
        let body = match &tag {
            Some(tag) => tagged_body(&name, tag, &variants, true),
            None => external_body(&name, &variants, true),
        };
        quote! {
            fn from_node_owned(
                node: ::forgeconf::ConfigNode,
            ) -> ::core::result::Result<Self, ::forgeconf::ConfigError> {
                #body
            }
        }
    });
    let shape = shape_body(&name, doc_string(&input.attrs).as_deref(), tag.as_deref(), &variants);

    quote! {
        #[automatically_derived]
        impl ::forgeconf::FromNode for #ident {
            fn from_node(
                node: &::forgeconf::ConfigNode,
            ) -> ::core::result::Result<Self, ::forgeconf::ConfigError> {
                #from_node
            }

            #owned

            fn shape() -> ::forgeconf::Shape {
                #shape
            }
        }
    }
}

/// Whether an owned conversion is worth generating and reads every struct
/// variant like the borrowed one; unit-only enums gain nothing from it.
fn owned_variants(variants: &[Variant]) -> bool {
    variants.iter().any(|v| !matches!(v.kind, Kind::Unit))
        && variants.iter().all(|v| match &v.kind {
            Kind::Struct(fields) => owned_supported(fields),
            _ => true,
        })
}

fn struct_variant_body(
    enum_name: &str,
    variant: &Variant,
    fields: &[FieldDef],
    prefixed: bool,
    owned: bool,
) -> TokenStream {
    let id = &variant.ident;
    let body = from_table(fields, &quote!(Self::#id), false, &[], owned);
    let key = &variant.key;
    let type_name = format!("{enum_name}::{}", variant.ident.unraw());
    match (prefixed, owned) {
        (false, false) => quote!({ #body }),
        (false, true) => quote! {{
            #[allow(unused_mut)]
            let mut __owned = ::forgeconf::__private::OwnedTable::new(node, #enum_name)?;
            #body
        }},
        (true, false) => quote! {
            (|| -> ::core::result::Result<Self, ::forgeconf::ConfigError> {
                let node = __value;
                let __table = ::forgeconf::__private::table(node, #type_name)?;
                #body
            })()
            .map_err(|e| e.at_key(#key))
        },
        (true, true) => quote! {
            (|| -> ::core::result::Result<Self, ::forgeconf::ConfigError> {
                #[allow(unused_mut)]
                let mut __owned = ::forgeconf::__private::OwnedTable::new(__value, #type_name)?;
                #body
            })()
            .map_err(|e| e.at_key(#key))
        },
    }
}

fn external_body(name: &str, variants: &[Variant], owned: bool) -> TokenStream {
    let units: Vec<&Variant> = variants
        .iter()
        .filter(|v| matches!(v.kind, Kind::Unit))
        .collect();
    let data: Vec<&Variant> = variants
        .iter()
        .filter(|v| !matches!(v.kind, Kind::Unit))
        .collect();

    let scalar = if units.is_empty() {
        quote! {{
            ::core::result::Result::Err(
                ::forgeconf::ConfigError::mismatch("table with exactly one key", node.kind())
                    .with_origin(node.origin()),
            )
        }}
    } else {
        let unit_keys = keys(units.iter().copied());
        let expected = format!("one of: {}", unit_keys.join(", "));
        let arms = units.iter().map(|v| {
            let (pattern, id) = (v.pattern(), &v.ident);
            quote!(#pattern => ::core::result::Result::Ok(Self::#id))
        });
        let unknown = unknown_variant(&quote!(__name), &quote!(node.origin()), &quote!());
        quote! {{
            const __EXPECTED: &[&str] = &[#(#unit_keys),*];
            let __name = ::forgeconf::__private::scalar(node, #expected)?;
            match __name {
                #(#arms,)*
                _ => #unknown,
            }
        }}
    };
    if data.is_empty() {
        return scalar;
    }

    let data_keys = keys(data.iter().copied());
    let arms = data.iter().map(|v| {
        let pattern = v.pattern();
        let key = &v.key;
        let arm = match &v.kind {
            Kind::Newtype(inner) => {
                let id = &v.ident;
                let from = if owned { quote!(from_node_owned) } else { quote!(from_node) };
                quote! {
                    <#inner as ::forgeconf::FromNode>::#from(__value)
                        .map(Self::#id)
                        .map_err(|e| e.at_key(#key))
                }
            },
            Kind::Struct(fields) => struct_variant_body(name, v, fields, true, owned),
            Kind::Unit => TokenStream::new(),
        };
        quote!(#pattern => #arm)
    });
    let unknown = unknown_variant(&quote!(__key.as_str()), &quote!(__value.origin()), &quote!());
    if owned {
        return quote! {
            match ::forgeconf::__private::single_entry(node)? {
                ::forgeconf::__private::Single::Entry(__key, __value) => {
                    const __EXPECTED: &[&str] = &[#(#data_keys),*];
                    match __key.as_str() {
                        #(#arms,)*
                        _ => #unknown,
                    }
                }
                ::forgeconf::__private::Single::Other(node) => {
                    let node = &node;
                    #scalar
                }
            }
        };
    }
    quote! {
        if let ::core::option::Option::Some(__map) = node.as_table() {
            const __EXPECTED: &[&str] = &[#(#data_keys),*];
            let mut __entries = __map.iter();
            let (
                ::core::option::Option::Some((__key, __value)),
                ::core::option::Option::None,
            ) = (__entries.next(), __entries.next())
            else {
                return ::core::result::Result::Err(
                    ::forgeconf::ConfigError::mismatch(
                        "table with exactly one key",
                        ::std::format!("table with {} keys", __map.len()),
                    )
                    .with_origin(node.origin()),
                );
            };
            match __key.as_str() {
                #(#arms,)*
                _ => #unknown,
            }
        } else #scalar
    }
}

fn tagged_body(name: &str, tag: &str, variants: &[Variant], owned: bool) -> TokenStream {
    let all_keys = keys(variants.iter());
    let expected = format!("one of: {}", all_keys.join(", "));
    let arms = variants.iter().map(|v| {
        let (pattern, id) = (v.pattern(), &v.ident);
        let arm = match &v.kind {
            Kind::Unit => quote!(::core::result::Result::Ok(Self::#id)),
            Kind::Newtype(inner) => {
                let from = if owned { quote!(from_node_owned) } else { quote!(from_node) };
                quote!(<#inner as ::forgeconf::FromNode>::#from(node).map(Self::#id))
            },
            Kind::Struct(fields) => struct_variant_body(name, v, fields, false, owned),
        };
        (pattern, arm)
    });
    let unknown = unknown_variant(&quote!(__name), &quote!(__tag.origin()), &quote!(.at_key(#tag)));
    let lookup = quote! {
        const __EXPECTED: &[&str] = &[#(#all_keys),*];
        let __table = ::forgeconf::__private::table(node, #name)?;
        let __tag = match __table.get(#tag) {
            ::core::option::Option::Some(n) if !n.is_null() => n,
            _ => {
                return ::core::result::Result::Err(
                    ::forgeconf::ConfigError::missing().at_key(#tag).with_origin(node.origin()),
                );
            }
        };
        let __name = ::forgeconf::__private::scalar(__tag, #expected).map_err(|e| e.at_key(#tag))?;
    };
    if !owned {
        let arms = arms.map(|(pattern, arm)| quote!(#pattern => #arm));
        return quote! {
            #lookup
            match __name {
                #(#arms,)*
                _ => #unknown,
            }
        };
    }
    // The tag name borrows the node, so pick the variant first and hand the
    // node over afterwards.
    let (patterns, arms): (Vec<_>, Vec<_>) = arms.unzip();
    let picks = patterns
        .iter()
        .enumerate()
        .map(|(index, pattern)| quote!(#pattern => #index));
    let last = arms.len().saturating_sub(1);
    let dispatch = arms
        .iter()
        .enumerate()
        .map(|(index, arm)| if index == last { quote!(_ => #arm) } else { quote!(#index => #arm) });
    quote! {
        let __variant: usize = {
            let node = &node;
            #lookup
            match __name {
                #(#picks,)*
                _ => return #unknown,
            }
        };
        match __variant {
            #(#dispatch,)*
        }
    }
}

fn shape_body(
    name: &str,
    docs: Option<&str>,
    tag: Option<&str>,
    variants: &[Variant],
) -> TokenStream {
    let mut statics = Vec::new();
    let infos: Vec<TokenStream> = variants
        .iter()
        .enumerate()
        .map(|(index, v)| {
            let key = &v.key;
            let mut info = quote!(::forgeconf::VariantInfo::new(#key));
            if let Some(doc) = &v.docs {
                info = quote!(#info.doc(#doc));
            }
            match &v.kind {
                Kind::Unit => {},
                Kind::Newtype(inner) => {
                    info = quote!(#info.data(<#inner as ::forgeconf::FromNode>::shape));
                },
                Kind::Struct(fields) => {
                    let info_name = format_ident!("__V{}_INFO", index);
                    let type_name = format!("{name}::{}", v.ident.unraw());
                    statics.push(struct_info_statics(
                        &format_ident!("__V{}_FIELDS", index),
                        &info_name,
                        &type_name,
                        fields,
                        None,
                        None,
                    ));
                    info = quote!(#info.data(|| ::forgeconf::Shape::Struct(&#info_name)));
                },
            }
            info
        })
        .collect();
    let count = infos.len();
    let doc = docs.map(|d| quote!(.doc(#d)));
    let tag = tag.map(|t| quote!(.tag(#t)));
    quote! {
        #(#statics)*
        static __VARIANTS: [::forgeconf::VariantInfo; #count] = [#(#infos),*];
        static __INFO: ::forgeconf::EnumInfo =
            ::forgeconf::EnumInfo::new(#name, &__VARIANTS) #doc #tag;
        ::forgeconf::Shape::Enum(&__INFO)
    }
}
