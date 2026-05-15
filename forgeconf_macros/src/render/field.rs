use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, LitStr};

use crate::model::FieldSpec;

pub(super) fn render_field_init(field: &FieldSpec) -> TokenStream {
    let ident = &field.ident;
    let ty = &field.ty;
    let key = field.options.rename.clone().unwrap_or(ident.to_string());
    let key_lit = LitStr::new(&key, ident.span());
    let value_ident = format_ident!("__forgeconf_value");

    let lookup_expr = if field.options.insensitive {
        quote! {
            {
                let target = #key_lit;
                let actual = map
                    .keys()
                    .find(|candidate| candidate.eq_ignore_ascii_case(target))
                    .cloned();
                actual.and_then(|real| map.remove(&real))
            }
        }
    } else {
        quote! { map.remove(#key_lit) }
    };

    let override_expr = match (field.options.cli.as_ref(), field.options.env.as_ref()) {
        (Some(cli), Some(env)) => {
            let cli_lit = LitStr::new(cli, ident.span());
            let env_lit = LitStr::new(env, ident.span());
            quote! {
                std::env::args()
                    .skip(1)
                    .find_map(|arg| {
                        if let Some(value) = arg.strip_prefix(concat!("--", #cli_lit, "=")) {
                            Some(::forgeconf::ConfigNode::Scalar(value.to_string()))
                        } else {
                            None
                        }
                    })
                    .or(std::env::var(#env_lit).ok().map(::forgeconf::ConfigNode::Scalar))
            }
        },
        (Some(cli), None) => {
            let cli_lit = LitStr::new(cli, ident.span());
            quote! {
                std::env::args()
                    .skip(1)
                    .find_map(|arg| {
                        if let Some(value) = arg.strip_prefix(concat!("--", #cli_lit, "=")) {
                            Some(::forgeconf::ConfigNode::Scalar(value.to_string()))
                        } else {
                            None
                        }
                    })
            }
        },
        (None, Some(env)) => {
            let env_lit = LitStr::new(env, ident.span());
            quote! {
                std::env::var(#env_lit)
                    .ok()
                    .map(::forgeconf::ConfigNode::Scalar)
            }
        },
        (None, None) => quote! { None },
    };

    let fetch_value = quote! {
        {
            let mut value: Option<::forgeconf::ConfigNode> = #override_expr;
            if value.is_none() {
                value = #lookup_expr;
            }
            value
        }
    };

    let base_expr = match field_kind(field) {
        FieldKind::Optional => {
            quote! {
                let node = #fetch_value.unwrap_or(::forgeconf::ConfigNode::Null);
                <#ty as ::forgeconf::FromNode>::from_node(&node, #key_lit)?
            }
        },
        FieldKind::Default(expr) => {
            quote! {
                if let Some(node) = #fetch_value {
                    <#ty as ::forgeconf::FromNode>::from_node(&node, #key_lit)?
                } else {
                    #expr
                }
            }
        },
        FieldKind::Scalar => {
            quote! {
                if let Some(node) = #fetch_value {
                    <#ty as ::forgeconf::FromNode>::from_node(&node, #key_lit)?
                } else {
                    return Err(::forgeconf::ConfigError::missing(#key_lit));
                }
            }
        },
        FieldKind::Nested => {
            quote! {
                if let Some(node) = #fetch_value {
                    <#ty as ::forgeconf::FromNode>::from_node(&node, #key_lit)?
                } else {
                    let fallback = ::forgeconf::ConfigNode::Table(map.clone());
                    <#ty as ::forgeconf::FromNode>::from_node(&fallback, #key_lit)?
                }
            }
        },
    };

    let validator_calls = render_validator_calls(field, &key_lit, &value_ident);

    quote! {
        #ident: {
            let #value_ident = { #base_expr };
            #validator_calls
            #value_ident
        }
    }
}

fn field_kind(field: &FieldSpec) -> FieldKind<'_> {
    if field.options.optional {
        FieldKind::Optional
    } else if let Some(expr) = field.options.default.as_ref() {
        FieldKind::Default(expr)
    } else if field.options.nested {
        FieldKind::Nested
    } else {
        FieldKind::Scalar
    }
}

enum FieldKind<'a> {
    Optional,
    Default(&'a Expr),
    Scalar,
    Nested,
}

fn render_validator_calls(
    field: &FieldSpec,
    key_lit: &LitStr,
    value_ident: &proc_macro2::Ident,
) -> TokenStream {
    if field.options.validators.is_empty() {
        return TokenStream::new();
    }

    let validators = field.options.validators.iter().map(|expr| {
        quote! { (#expr)(&#value_ident, #key_lit)?; }
    });

    quote! { #(#validators)* }
}
