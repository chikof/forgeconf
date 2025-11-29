use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, ItemStruct, LitStr, Result};

use crate::model::{is_scalar_type, ConfigFile, FieldSpec, ForgeconfAttr};

pub fn render(
    item: &ItemStruct,
    args: &ForgeconfAttr,
    fields: &[FieldSpec],
) -> Result<TokenStream> {
    let ident = &item.ident;
    let loader_ident = format_ident!("{}Loader", ident);

    let add_config = args
        .files
        .iter()
        .map(render_config_addition);

    let field_inits = fields
        .iter()
        .map(render_field_init);

    let result = quote! {
        #item

        impl #ident {
            pub fn loader() -> #loader_ident {
                #loader_ident {
                    builder: ::forgeconf::ConfigBuilder::new(),
                }
            }

            pub fn load_from(node: &::forgeconf::ConfigNode) -> Result<Self, ::forgeconf::ConfigError> {
                let mut map = node.to_owned_table()?;
                Ok(Self {
                    #(#field_inits),*
                })
            }
        }

        pub struct #loader_ident {
            builder: ::forgeconf::ConfigBuilder,
        }

        impl #loader_ident {
            pub fn with_cli(mut self, priority: u8) -> Self {
                self.builder = self
                    .builder
                    .add_source(::forgeconf::CliArguments::new().with_priority(priority));
                self
            }

            pub fn with_config(mut self) -> Self {
                #(#add_config)*
                self
            }

            pub fn add_source<S>(mut self, source: S) -> Self
            where
                S: ::forgeconf::ConfigSource + 'static,
            {
                self.builder = self.builder.add_source(source);
                self
            }

            pub fn load(self) -> Result<#ident, ::forgeconf::ConfigError> {
                let value = self.builder.load()?;
                #ident::load_from(&value)
            }
        }

        impl ::forgeconf::FromNode for #ident {
            fn from_node(node: &::forgeconf::ConfigNode, key: &str) -> Result<Self, ::forgeconf::ConfigError> {
                #ident::load_from(node).map_err(|err| ::forgeconf::ConfigError::nested(key, err))
            }
        }
    };

    Ok(result)
}

fn render_config_addition(cfg: &ConfigFile) -> TokenStream {
    let path = LitStr::new(&cfg.path, proc_macro2::Span::call_site());
    let format_chain = cfg
        .format
        .as_ref()
        .map(|fmt| match fmt {
            forgeconf_core::FileFormat::Toml => {
                quote! { .with_format(::forgeconf::FileFormat::Toml) }
            },
            forgeconf_core::FileFormat::Yaml => {
                quote! { .with_format(::forgeconf::FileFormat::Yaml) }
            },
            forgeconf_core::FileFormat::Json => {
                quote! { .with_format(::forgeconf::FileFormat::Json) }
            },
        })
        .unwrap_or_default();

    let priority_chain = cfg
        .priority
        .map(|value| {
            quote! { .with_priority(#value) }
        })
        .unwrap_or_default();

    quote! {
        self.builder = self.builder.add_source(
            ::forgeconf::ConfigFile::new(#path)
                #format_chain
                #priority_chain
        );
    }
}

fn render_field_init(field: &FieldSpec) -> TokenStream {
    let ident = &field.ident;
    let ty = &field.ty;
    let key = field
        .options
        .rename
        .clone()
        .unwrap_or_else(|| ident.to_string());
    let key_lit = LitStr::new(&key, ident.span());

    let lookup_expr = if field
        .options
        .insensitive
    {
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
        quote! {
            map.remove(#key_lit)
        }
    };

    let override_expr = match (
        field
            .options
            .cli
            .as_ref(),
        field
            .options
            .env
            .as_ref(),
    ) {
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
                    .or_else(|| std::env::var(#env_lit).ok().map(::forgeconf::ConfigNode::Scalar))
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

    match field_kind(field) {
        FieldKind::Optional => {
            quote! {
                #ident: {
                    let node = #fetch_value.unwrap_or(::forgeconf::ConfigNode::Null);
                    <#ty as ::forgeconf::FromNode>::from_node(&node, #key_lit)?
                }
            }
        },
        FieldKind::Default(expr) => {
            quote! {
                #ident: {
                    if let Some(node) = #fetch_value {
                        <#ty as ::forgeconf::FromNode>::from_node(&node, #key_lit)?
                    } else {
                        #expr
                    }
                }
            }
        },
        FieldKind::Scalar => {
            quote! {
                #ident: {
                    if let Some(node) = #fetch_value {
                        <#ty as ::forgeconf::FromNode>::from_node(&node, #key_lit)?
                    } else {
                        return Err(::forgeconf::ConfigError::missing(#key_lit));
                    }
                }
            }
        },
        FieldKind::Nested => {
            quote! {
                #ident: {
                    if let Some(node) = #fetch_value {
                        <#ty as ::forgeconf::FromNode>::from_node(&node, #key_lit)?
                    } else {
                        let fallback = ::forgeconf::ConfigNode::Table(map.clone());
                        <#ty as ::forgeconf::FromNode>::from_node(&fallback, #key_lit)?
                    }
                }
            }
        },
    }
}

fn field_kind(field: &FieldSpec) -> FieldKind<'_> {
    if field
        .options
        .optional
    {
        FieldKind::Optional
    } else if let Some(expr) = field
        .options
        .default
        .as_ref()
    {
        FieldKind::Default(expr)
    } else if is_scalar_type(&field.ty) {
        FieldKind::Scalar
    } else {
        FieldKind::Nested
    }
}

enum FieldKind<'a> {
    Optional,
    Default(&'a Expr),
    Scalar,
    Nested,
}
