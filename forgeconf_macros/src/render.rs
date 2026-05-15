use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, ItemStruct, LitChar, LitStr, Result};

use crate::model::{ConfigFile, FieldSpec, ForgeconfAttr};

pub fn render(
    item: &ItemStruct,
    args: &ForgeconfAttr,
    fields: &[FieldSpec],
) -> Result<TokenStream> {
    let ident = &item.ident;
    let loader_ident = format_ident!("{}Loader", ident);

    let add_config_stmts = args
        .files
        .iter()
        .map(render_config_addition);

    let field_inits = fields
        .iter()
        .map(render_field_init);

    let parse_methods = generate_parse_methods(ident);
    let clap_methods = generate_clap_methods(ident, fields);

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
    };

    Ok(result)
}

fn generate_parse_methods(_ident: &syn::Ident) -> TokenStream {
    // Generate parse methods with cfg gates. We use #[allow(unexpected_cfgs)]
    // on the generated impl block to suppress warnings about these feature flags
    // not being defined in the consumer crate.
    quote! {
        /// Parse TOML text directly into this configuration struct.
        ///
        /// Requires the `toml` and `parse` features to be enabled on `forgeconf`.
        ///
        /// # Example
        /// ```no_run
        /// let config_text = r#"
        ///     port = 8080
        ///     host = "localhost"
        /// "#;
        /// let config = MyConfig::parse_toml(config_text)?;
        /// # Ok::<(), ::forgeconf::ConfigError>(())
        /// ```
        #[cfg(all(feature = "parse", feature = "toml"))]
        pub fn parse_toml(input: &str) -> Result<Self, ::forgeconf::ConfigError> {
            let node = ::forgeconf::parse_toml(input)?;
            Self::load_from(&node)
        }

        /// Parse YAML text directly into this configuration struct.
        ///
        /// Requires the `yaml` and `parse` features to be enabled on `forgeconf`.
        ///
        /// # Example
        /// ```no_run
        /// let config_text = r#"
        ///     port: 8080
        ///     host: localhost
        /// "#;
        /// let config = MyConfig::parse_yaml(config_text)?;
        /// # Ok::<(), ::forgeconf::ConfigError>(())
        /// ```
        #[cfg(all(feature = "parse", feature = "yaml"))]
        pub fn parse_yaml(input: &str) -> Result<Self, ::forgeconf::ConfigError> {
            let node = ::forgeconf::parse_yaml(input)?;
            Self::load_from(&node)
        }

        /// Parse JSON text directly into this configuration struct.
        ///
        /// Requires the `json` and `parse` features to be enabled on `forgeconf`.
        ///
        /// # Example
        /// ```no_run
        /// let config_text = r#"
        ///     {
        ///         "port": 8080,
        ///         "host": "localhost"
        ///     }
        /// "#;
        /// let config = MyConfig::parse_json(config_text)?;
        /// # Ok::<(), ::forgeconf::ConfigError>(())
        /// ```
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
        .map(|value| {
            quote! { .with_priority(#value) }
        })
        .unwrap_or_default();

    quote! {
        __builder = __builder.add_source(
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
        .unwrap_or(ident.to_string());
    let key_lit = LitStr::new(&key, ident.span());
    let value_ident = format_ident!("__forgeconf_value");

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
    } else if field
        .options
        .nested
    {
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

fn generate_clap_methods(_ident: &syn::Ident, fields: &[FieldSpec]) -> TokenStream {
    if !cfg!(feature = "clap") {
        return TokenStream::new();
    }

    let augment_args: Vec<TokenStream> = fields
        .iter()
        .filter(|f| {
            !f.options
                .no_cli
                && !f
                    .options
                    .nested
        })
        .map(render_clap_arg)
        .collect();

    let extract_args: Vec<TokenStream> = fields
        .iter()
        .filter(|f| {
            !f.options
                .no_cli
                && !f
                    .options
                    .nested
        })
        .map(render_clap_extract)
        .collect();

    quote! {
        /// Build a [`clap::Command`] augmented with an argument for every config field.
        ///
        /// Use [`Self::from_clap`] to convert the resulting [`clap::ArgMatches`] into
        /// a [`forgeconf::CliArgsSource`] that can be passed to the loader.
        pub fn augment_clap(cmd: ::forgeconf::clap::Command) -> ::forgeconf::clap::Command {
            cmd #(#augment_args)*
        }

        /// Extract config values from clap [`ArgMatches`](forgeconf::clap::ArgMatches)
        /// and return them as a [`CliArgsSource`](forgeconf::CliArgsSource).
        ///
        /// Only fields that were actually provided on the command line are inserted;
        /// the rest continue to be resolved from other sources (files, env, defaults).
        pub fn from_clap(matches: &::forgeconf::clap::ArgMatches) -> ::forgeconf::CliArgsSource {
            let mut __map = ::std::collections::BTreeMap::new();
            #(#extract_args)*
            ::forgeconf::CliArgsSource::new(__map)
        }
    }
}

fn render_clap_arg(field: &FieldSpec) -> TokenStream {
    let ident_str = field
        .ident
        .to_string();
    let long = field
        .options
        .cli
        .clone()
        .unwrap_or(ident_str.replace('_', "-"));
    let long_lit = LitStr::new(
        &long,
        field
            .ident
            .span(),
    );
    let id_lit = LitStr::new(
        &ident_str,
        field
            .ident
            .span(),
    );

    let short_chain = field
        .options
        .short
        .map(|c| {
            let lit = LitChar::new(
                c,
                field
                    .ident
                    .span(),
            );
            quote! { .short(#lit) }
        })
        .unwrap_or_default();

    let help_chain = field
        .options
        .help
        .as_ref()
        .map(|h| {
            let h_lit = LitStr::new(
                h,
                field
                    .ident
                    .span(),
            );
            quote! { .help(#h_lit) }
        })
        .unwrap_or_default();

    quote! {
        .arg(
            ::forgeconf::clap::Arg::new(#id_lit)
                .long(#long_lit)
                #short_chain
                #help_chain
        )
    }
}

fn render_clap_extract(field: &FieldSpec) -> TokenStream {
    let ident_str = field
        .ident
        .to_string();
    let id_lit = LitStr::new(
        &ident_str,
        field
            .ident
            .span(),
    );
    let config_key = field
        .options
        .rename
        .clone()
        .unwrap_or(ident_str.clone());
    let config_key_lit = LitStr::new(
        &config_key,
        field
            .ident
            .span(),
    );

    quote! {
        if let Some(__v) = matches.get_one::<String>(#id_lit) {
            __map.insert(#config_key_lit.to_string(), __v.clone());
        }
    }
}

fn render_validator_calls(
    field: &FieldSpec,
    key_lit: &LitStr,
    value_ident: &proc_macro2::Ident,
) -> TokenStream {
    if field
        .options
        .validators
        .is_empty()
    {
        return TokenStream::new();
    }

    let validators = field
        .options
        .validators
        .iter()
        .map(|expr| {
            quote! {
                (#expr)(&#value_ident, #key_lit)?;
            }
        });

    quote! {
        #(#validators)*
    }
}
