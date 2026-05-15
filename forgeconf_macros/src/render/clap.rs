use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{LitChar, LitStr};

use crate::model::{FieldSpec, is_vec_type};

/// Generates `augment_clap` and `from_clap` methods on the config struct.
/// Both delegate to the `ForgeconfClap` trait impl so nested fields are
/// transparently handled at any nesting depth.
pub(super) fn generate_clap_methods() -> TokenStream {
    if !cfg!(feature = "clap") {
        return TokenStream::new();
    }
    quote! {
        /// Augment a [`clap::Command`] with CLI arguments for every config
        /// field, including nested structs (prefixed with `parent-field-`).
        pub fn augment_clap(cmd: ::forgeconf::clap::Command) -> ::forgeconf::clap::Command {
            <Self as ::forgeconf::ForgeconfClap>::augment_clap_with_prefix(cmd, None)
        }

        /// Extract matched CLI values and return them as a [`CliArgsSource`].
        ///
        /// Only arguments actually provided on the command line are inserted;
        /// the rest continue to be resolved from files, env, or defaults.
        pub fn from_clap(matches: &::forgeconf::clap::ArgMatches) -> ::forgeconf::CliArgsSource {
            let mut __map = ::std::collections::BTreeMap::new();
            <Self as ::forgeconf::ForgeconfClap>::extract_clap_with_prefix(matches, None, &mut __map);
            ::forgeconf::CliArgsSource::new(__map)
        }
    }
}

/// Generates `impl ForgeconfClap for Struct` — the prefix-aware augment/extract
/// trait that lets parent structs delegate to nested types.
///
/// Flat fields become `--field-name` args. Nested (non-Vec) fields recursively
/// delegate with a `"parent."` prefix, producing `--parent-field-name` args.
/// `Vec<T>` nested fields and `no_cli` fields are skipped.
pub(super) fn generate_forgeconf_clap_impl(
    ident: &syn::Ident,
    fields: &[FieldSpec],
) -> TokenStream {
    if !cfg!(feature = "clap") {
        return TokenStream::new();
    }

    let flat_fields: Vec<&FieldSpec> = fields
        .iter()
        .filter(|f| !f.options.no_cli && !f.options.nested)
        .collect();

    let nested_fields: Vec<&FieldSpec> = fields
        .iter()
        .filter(|f| f.options.nested && !f.options.no_cli && !is_vec_type(&f.ty))
        .collect();

    let flat_augment: Vec<TokenStream> =
        flat_fields.iter().map(|f| render_prefixed_arg(f)).collect();

    let nested_augment: Vec<TokenStream> = nested_fields
        .iter()
        .map(|f| {
            let ty = &f.ty;
            let name = f.options.rename.clone().unwrap_or(f.ident.to_string());
            let name_lit = LitStr::new(&name, f.ident.span());
            quote! {
                let cmd = {
                    let __nested_prefix = match prefix {
                        Some(p) => ::std::format!("{}.{}", p, #name_lit),
                        None => #name_lit.to_string(),
                    };
                    <#ty as ::forgeconf::ForgeconfClap>::augment_clap_with_prefix(
                        cmd,
                        Some(&__nested_prefix),
                    )
                };
            }
        })
        .collect();

    let flat_extract: Vec<TokenStream> = flat_fields
        .iter()
        .map(|f| render_prefixed_extract(f))
        .collect();

    let nested_extract: Vec<TokenStream> = nested_fields
        .iter()
        .map(|f| {
            let ty = &f.ty;
            let name = f.options.rename.clone().unwrap_or(f.ident.to_string());
            let name_lit = LitStr::new(&name, f.ident.span());
            quote! {
                {
                    let __nested_prefix = match prefix {
                        Some(p) => ::std::format!("{}.{}", p, #name_lit),
                        None => #name_lit.to_string(),
                    };
                    <#ty as ::forgeconf::ForgeconfClap>::extract_clap_with_prefix(
                        matches,
                        Some(&__nested_prefix),
                        out,
                    );
                }
            }
        })
        .collect();

    quote! {
        impl ::forgeconf::ForgeconfClap for #ident {
            #[allow(unused_variables)]
            fn augment_clap_with_prefix(
                cmd: ::forgeconf::clap::Command,
                prefix: Option<&str>,
            ) -> ::forgeconf::clap::Command {
                #(#flat_augment)*
                #(#nested_augment)*
                cmd
            }

            #[allow(unused_variables)]
            fn extract_clap_with_prefix(
                matches: &::forgeconf::clap::ArgMatches,
                prefix: Option<&str>,
                out: &mut ::std::collections::BTreeMap<String, String>,
            ) {
                #(#flat_extract)*
                #(#nested_extract)*
            }
        }
    }
}

/// Generates the companion `{Struct}CliArgs` struct that implements
/// `clap::Args + ConfigSource`, enabling `#[command(flatten)]`.
///
/// The companion delegates entirely to the struct's `ForgeconfClap` impl,
/// so nested fields work automatically.
pub(super) fn generate_clap_companion(ident: &syn::Ident, _fields: &[FieldSpec]) -> TokenStream {
    if !cfg!(feature = "clap") {
        return TokenStream::new();
    }

    let companion_ident = format_ident!("{}CliArgs", ident);

    quote! {
        #[derive(Debug, Clone, Default)]
        pub struct #companion_ident {
            __args: ::std::collections::BTreeMap<String, String>,
        }

        impl ::forgeconf::clap::FromArgMatches for #companion_ident {
            fn from_arg_matches(
                matches: &::forgeconf::clap::ArgMatches,
            ) -> Result<Self, ::forgeconf::clap::Error> {
                let mut __args = ::std::collections::BTreeMap::new();
                <#ident as ::forgeconf::ForgeconfClap>::extract_clap_with_prefix(
                    matches, None, &mut __args,
                );
                Ok(Self { __args })
            }

            fn update_from_arg_matches(
                &mut self,
                matches: &::forgeconf::clap::ArgMatches,
            ) -> Result<(), ::forgeconf::clap::Error> {
                <#ident as ::forgeconf::ForgeconfClap>::extract_clap_with_prefix(
                    matches, None, &mut self.__args,
                );
                Ok(())
            }
        }

        impl ::forgeconf::clap::Args for #companion_ident {
            fn augment_args(cmd: ::forgeconf::clap::Command) -> ::forgeconf::clap::Command {
                <#ident as ::forgeconf::ForgeconfClap>::augment_clap_with_prefix(cmd, None)
            }

            fn augment_args_for_update(
                cmd: ::forgeconf::clap::Command,
            ) -> ::forgeconf::clap::Command {
                Self::augment_args(cmd)
            }
        }

        impl ::forgeconf::ConfigSource for #companion_ident {
            fn priority(&self) -> u8 {
                u8::MAX
            }

            fn load(&self) -> Result<::forgeconf::ConfigNode, ::forgeconf::ConfigError> {
                ::forgeconf::CliArgsSource::new(self.__args.clone()).load()
            }
        }
    }
}

/// Generates a `let cmd = { ... };` binding that adds a single prefixed arg.
///
/// Arg ID uses dot notation (`watcher.watch_path`) so it matches the dotted
/// key that `CliArgsSource::load` expands via `insert_path`.
/// Long flag uses hyphens (`watcher-watch-path`) following CLI convention.
/// Short flags are only attached when `prefix.is_none()` to avoid collisions.
///
/// **Note on `Box::leak`**: clap 4.x requires `&'static str` for `Arg::new`
/// and `.long()`. When a runtime prefix is present, `Box::leak` converts the
/// formatted `String` to `&'static str`. This is intentional — `augment_clap`
/// runs once at startup, so the allocation is effectively permanent but
/// negligible in size (a few bytes per arg per nesting level).
fn render_prefixed_arg(field: &FieldSpec) -> TokenStream {
    let ident_str = field.ident.to_string();
    let config_key = field.options.rename.clone().unwrap_or(ident_str.clone());
    let long_base = field
        .options
        .cli
        .clone()
        .unwrap_or(ident_str.replace('_', "-"));

    let id_base_lit = LitStr::new(&config_key, field.ident.span());
    let long_base_lit = LitStr::new(&long_base, field.ident.span());

    let short_stmt = field
        .options
        .short
        .map(|c| {
            let lit = LitChar::new(c, field.ident.span());
            quote! { let arg = if prefix.is_none() { arg.short(#lit) } else { arg }; }
        })
        .unwrap_or_default();

    let help_stmt = field
        .options
        .help
        .as_ref()
        .map(|h| {
            let h_lit = LitStr::new(h, field.ident.span());
            quote! { let arg = arg.help(#h_lit); }
        })
        .unwrap_or_default();

    quote! {
        let cmd = {
            let __id: &'static str = match prefix {
                None => #id_base_lit,
                Some(p) => ::std::boxed::Box::leak(
                    ::std::format!("{}.{}", p, #id_base_lit).into_boxed_str()
                ),
            };
            let __long: &'static str = match prefix {
                None => #long_base_lit,
                Some(p) => ::std::boxed::Box::leak(
                    ::std::format!("{}-{}", p.replace('.', "-"), #long_base_lit)
                        .into_boxed_str()
                ),
            };
            let arg = ::forgeconf::clap::Arg::new(__id).long(__long);
            #short_stmt
            #help_stmt
            cmd.arg(arg)
        };
    }
}

/// Generates extraction of a single prefixed arg from `ArgMatches` into `out`.
///
/// The key inserted into `out` uses dot notation (`watcher.watch_path`) which
/// `CliArgsSource::load` then expands into a nested `ConfigNode` tree via
/// `insert_path`. `get_one` takes `&str` so no `Box::leak` is needed here.
fn render_prefixed_extract(field: &FieldSpec) -> TokenStream {
    let ident_str = field.ident.to_string();
    let config_key = field.options.rename.clone().unwrap_or(ident_str.clone());

    let id_base_lit = LitStr::new(&config_key, field.ident.span());
    let key_base_lit = LitStr::new(&config_key, field.ident.span());

    quote! {
        {
            let __id = match prefix {
                None => #id_base_lit.to_string(),
                Some(p) => ::std::format!("{}.{}", p, #id_base_lit),
            };
            let __key = match prefix {
                None => #key_base_lit.to_string(),
                Some(p) => ::std::format!("{}.{}", p, #key_base_lit),
            };
            if let Some(__v) = matches.get_one::<String>(__id.as_str()) {
                out.insert(__key, __v.clone());
            }
        }
    }
}
