use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::{Ident, ItemStruct, Path, Result, Token};

use crate::model::{ConfigFile, FieldSpec, ForgeconfAttr};

mod clap;
mod field;

pub fn render(
    item: &mut ItemStruct,
    args: &ForgeconfAttr,
    fields: &[FieldSpec],
) -> Result<TokenStream> {
    let has_sensitive = fields.iter().any(|f| f.options.sensitive);
    if has_sensitive {
        strip_debug_derive(item);
    }

    let ident = &item.ident;
    let loader_ident = format_ident!("{}Loader", ident);

    let add_config_stmts = args.files.iter().map(render_config_addition);
    let debug_impl = if has_sensitive {
        generate_debug_impl(ident, fields)
    } else {
        TokenStream::new()
    };
    let env_prefix = args.env_prefix.as_deref();
    let field_inits = fields
        .iter()
        .map(|f| field::render_field_init(f, env_prefix));

    let parse_methods = generate_parse_methods();
    let clap_methods = clap::generate_clap_methods();
    let forgeconf_clap_impl = clap::generate_forgeconf_clap_impl(ident, fields);
    let clap_companion = clap::generate_clap_companion(ident, fields);

    let result = quote! {
        // Allow unexpected_cfgs to prevent warnings about parse/toml/yaml/json features
        // that may not be defined in the consumer crate
        #[allow(unexpected_cfgs)]
        #item

        #[allow(unexpected_cfgs)]
        impl #ident {
            pub fn loader() -> #loader_ident {
                let mut __builder = ::forgeconf::ConfigBuilder::new();
                #(#add_config_stmts)*
                #loader_ident { builder: __builder }
            }

            pub fn load_from(node: &::forgeconf::ConfigNode) -> Result<Self, ::forgeconf::ConfigError> {
                let mut map = node.to_owned_table()?;
                Ok(Self {
                    #(#field_inits),*
                })
            }

            #parse_methods
            #clap_methods
        }

        pub struct #loader_ident {
            builder: ::forgeconf::ConfigBuilder,
        }

        impl #loader_ident {
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

        #forgeconf_clap_impl
        #clap_companion
        #debug_impl
    };

    Ok(result)
}

fn strip_debug_derive(item: &mut ItemStruct) {
    let attrs = std::mem::take(&mut item.attrs);
    item.attrs = attrs
        .into_iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("derive") {
                return Some(attr);
            }
            let Ok(paths) = attr.parse_args_with(
                |input: syn::parse::ParseStream<'_>| -> syn::Result<Punctuated<Path, Token![,]>> {
                    input.parse_terminated(parse_path, Token![,])
                },
            ) else {
                return Some(attr);
            };
            let filtered: Punctuated<Path, Token![,]> =
                paths.into_iter().filter(|p| !p.is_ident("Debug")).collect();
            if filtered.is_empty() {
                None
            } else {
                Some(syn::parse_quote!(#[derive(#filtered)]))
            }
        })
        .collect();
}

fn parse_path(input: syn::parse::ParseStream<'_>) -> syn::Result<Path> {
    input.parse()
}

fn generate_debug_impl(ident: &Ident, fields: &[FieldSpec]) -> TokenStream {
    let struct_name = ident.to_string();
    let field_debugs = fields.iter().map(|f| {
        let fname = &f.ident;
        let fname_str = fname.to_string();
        if f.options.sensitive {
            quote! { .field(#fname_str, &"***") }
        } else {
            quote! { .field(#fname_str, &self.#fname) }
        }
    });

    quote! {
        impl ::std::fmt::Debug for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(#struct_name)
                    #(#field_debugs)*
                    .finish()
            }
        }
    }
}

fn generate_parse_methods() -> TokenStream {
    quote! {
        /// Parse TOML text directly into this configuration struct.
        ///
        /// Requires the `toml` and `parse` features to be enabled on `forgeconf`.
        #[cfg(all(feature = "parse", feature = "toml"))]
        pub fn parse_toml(input: &str) -> Result<Self, ::forgeconf::ConfigError> {
            let node = ::forgeconf::parse_toml(input)?;
            Self::load_from(&node)
        }

        /// Parse YAML text directly into this configuration struct.
        ///
        /// Requires the `yaml` and `parse` features to be enabled on `forgeconf`.
        #[cfg(all(feature = "parse", feature = "yaml"))]
        pub fn parse_yaml(input: &str) -> Result<Self, ::forgeconf::ConfigError> {
            let node = ::forgeconf::parse_yaml(input)?;
            Self::load_from(&node)
        }

        /// Parse JSON text directly into this configuration struct.
        ///
        /// Requires the `json` and `parse` features to be enabled on `forgeconf`.
        #[cfg(all(feature = "parse", feature = "json"))]
        pub fn parse_json(input: &str) -> Result<Self, ::forgeconf::ConfigError> {
            let node = ::forgeconf::parse_json(input)?;
            Self::load_from(&node)
        }
    }
}

fn render_config_addition(cfg: &ConfigFile) -> TokenStream {
    let path = &cfg.path;
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
        .map(|value| quote! { .with_priority(#value) })
        .unwrap_or_default();

    quote! {
        __builder = __builder.add_source(
            ::forgeconf::ConfigFile::new(#path)
                #format_chain
                #priority_chain
        );
    }
}
