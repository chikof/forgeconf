use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, LitStr};

use crate::model::FieldSpec;

pub(super) fn render_field_init(field: &FieldSpec, env_prefix: Option<&str>) -> TokenStream {
    let ident = &field.ident;
    let ty = &field.ty;
    let key = field.options.rename.clone().unwrap_or(ident.to_string());
    let key_lit = LitStr::new(&key, ident.span());
    let value_ident = format_ident!("__forgeconf_value");

    let alias_lits: Vec<LitStr> = field
        .options
        .aliases
        .iter()
        .map(|a| LitStr::new(a, ident.span()))
        .collect();
    let lookup_expr = {
        let primary = single_lookup(field.options.insensitive, &key_lit);
        let alias_lookups = alias_lits
            .iter()
            .map(|a| single_lookup(field.options.insensitive, a));
        quote! { #primary #(.or_else(|| #alias_lookups))* }
    };

    let ident_upper = ident.to_string().to_uppercase();
    let effective_env: Option<String> = field
        .options
        .env
        .clone()
        .or_else(|| env_prefix.map(|p| format!("{}{}", p, ident_upper)));

    let override_expr = match (field.options.cli.as_ref(), effective_env.as_deref()) {
        (Some(cli), Some(env)) => {
            let cli_lit = LitStr::new(cli, ident.span());
            let env_lit = LitStr::new(env, ident.span());
            quote! {
                std::env::args()
                    .skip(1)
                    .find_map(|arg| {
                        if let Some(value) = arg.strip_prefix(concat!("--", #cli_lit, "=")) {
                            Some(::forgeconf::ConfigNode::scalar(value))
                        } else {
                            None
                        }
                    })
                    .or(std::env::var(#env_lit).ok().map(::forgeconf::ConfigNode::scalar))
            }
        },
        (Some(cli), None) => {
            let cli_lit = LitStr::new(cli, ident.span());
            quote! {
                std::env::args()
                    .skip(1)
                    .find_map(|arg| {
                        if let Some(value) = arg.strip_prefix(concat!("--", #cli_lit, "=")) {
                            Some(::forgeconf::ConfigNode::scalar(value))
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
                    .map(::forgeconf::ConfigNode::scalar)
            }
        },
        (None, None) => quote! { None },
    };

    let found = format_ident!("__forgeconf_found");
    let convert = quote! {
        .map_err(|__forgeconf_err| __forgeconf_err.at_key(#key_lit))?
    };

    let base_expr = match field_kind(field) {
        FieldKind::Optional => {
            quote! {
                let __forgeconf_null = ::forgeconf::ConfigNode::null();
                let __forgeconf_node = #found.unwrap_or(&__forgeconf_null);
                <#ty as ::forgeconf::FromNode>::from_node(__forgeconf_node)#convert
            }
        },
        FieldKind::Default(expr) => {
            quote! {
                if let ::core::option::Option::Some(__forgeconf_node) = #found {
                    <#ty as ::forgeconf::FromNode>::from_node(__forgeconf_node)#convert
                } else {
                    #expr
                }
            }
        },
        FieldKind::Scalar => {
            quote! {
                if let ::core::option::Option::Some(__forgeconf_node) = #found {
                    <#ty as ::forgeconf::FromNode>::from_node(__forgeconf_node)#convert
                } else {
                    return ::core::result::Result::Err(
                        ::forgeconf::ConfigError::missing()
                            .at_key(#key_lit)
                            .with_origin(node.origin()),
                    );
                }
            }
        },
        FieldKind::Nested => {
            quote! {
                if let ::core::option::Option::Some(__forgeconf_node) = #found {
                    <#ty as ::forgeconf::FromNode>::from_node(__forgeconf_node)#convert
                } else {
                    <#ty as ::forgeconf::FromNode>::from_node(node)#convert
                }
            }
        },
    };

    let validator_calls = render_validator_calls(field, &key_lit, &value_ident);
    let deprecated_warn = render_deprecated_warn(field, &key_lit);

    quote! {
        #ident: {
            #deprecated_warn
            let __forgeconf_override: ::core::option::Option<::forgeconf::ConfigNode> =
                #override_expr;
            let #found: ::core::option::Option<&::forgeconf::ConfigNode> =
                match &__forgeconf_override {
                    ::core::option::Option::Some(__forgeconf_node) => {
                        ::core::option::Option::Some(__forgeconf_node)
                    },
                    ::core::option::Option::None => #lookup_expr,
                };
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

fn render_deprecated_warn(field: &FieldSpec, key_lit: &LitStr) -> TokenStream {
    let Some(msg) = field.options.deprecated.as_deref() else {
        return TokenStream::new();
    };
    if msg.is_empty() {
        quote! {
            eprintln!("[forgeconf] warning: config field `{}` is deprecated", #key_lit);
        }
    } else {
        let msg_lit = LitStr::new(msg, key_lit.span());
        quote! {
            eprintln!("[forgeconf] warning: config field `{}` is deprecated: {}", #key_lit, #msg_lit);
        }
    }
}

fn single_lookup(insensitive: bool, key_lit: &LitStr) -> TokenStream {
    if insensitive {
        quote! {
            table
                .iter()
                .find(|(candidate, _)| candidate.eq_ignore_ascii_case(#key_lit))
                .map(|(_, value)| value)
        }
    } else {
        quote! { table.get(#key_lit) }
    }
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
        quote! { (#expr)(&#value_ident).map_err(|e| e.at_key(#key_lit))?; }
    });

    quote! { #(#validators)* }
}
