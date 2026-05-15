use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemStruct, LitStr, Result};

use crate::model::{ConfigFile, FieldSpec, ForgeconfAttr};

mod clap;
mod field;

pub fn render(
    item: &ItemStruct,
    args: &ForgeconfAttr,
    fields: &[FieldSpec],
) -> Result<TokenStream> {
    let ident = &item.ident;
    let loader_ident = format_ident!("{}Loader", ident);

    let add_config_stmts = args.files.iter().map(render_config_addition);
    let field_inits = fields.iter().map(field::render_field_init);

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
    };

    Ok(result)
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
