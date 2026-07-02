//! Procedural macros for [`forgeconf`](https://docs.rs/forgeconf).
//!
//! This crate only exports `#[derive(Config)]`. Depend on `forgeconf`, which
//! re-exports it next to the `Config` trait; the attribute reference lives on
//! the derive itself.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{Data, DeriveInput, Error};

mod attr;
mod case;
mod expand_enum;
mod expand_struct;

use attr::Errors;

/// Derives `FromNode` (and, for structs, `Config`) for a configuration type.
///
/// Structs get `FromNode`, `Config` (`Type::loader()`) and a generated
/// `Shape`. Enums get `FromNode` and a `Shape` only. Generic types, lifetimes,
/// unions and unit structs are rejected. The helper attribute is `config`; the
/// derive emits only a compile error when any attribute is invalid, and
/// reports every error it finds.
///
/// # Struct attributes
///
/// - `file = "path"`: add a file source. Repeatable, kept in declaration order,
///   priority 10 unless overridden.
/// - `file(path = EXPR, format = "toml" | "json" | "yaml" | "yml", priority =
///   N, optional)`: the long form. `path` is required; `format` overrides
///   extension detection; `priority` is a `u8`; `optional` turns a missing file
///   into an empty table.
/// - `env_prefix = "APP_"`: prefix for derived environment variable names.
/// - `rename_all = "..."`: key style for every field, one of `lowercase`,
///   `UPPERCASE`, `snake_case`, `kebab-case`, `camelCase`, `PascalCase`,
///   `SCREAMING_SNAKE_CASE`, `SCREAMING-KEBAB-CASE`.
/// - `deny_unknown_fields`: unknown keys in the table become errors, with a
///   "did you mean" hint when a close key exists. Cannot be combined with
///   `flatten` fields.
/// - `validate = EXPR`: callable as `fn(&Self) -> Result<(), ConfigError>`.
///   Repeatable. Runs only after every field loaded.
///
/// # Field attributes
///
/// - `rename = "key"`: explicit key; wins over `rename_all`.
/// - `alias = "key"`: extra accepted key. Repeatable. The primary key wins when
///   several are present.
/// - `default` / `default = EXPR`: value used when the key is absent. Bare
///   `default` calls `Default::default()`. A string literal goes through
///   `Into`, so `default = "x"` fills `String` and `PathBuf` fields.
/// - `env = "VAR"`: explicit environment variable name.
/// - `flag = "long-name"` and `short = 'c'`: command-line flag names. `short`
///   takes a single character literal.
/// - `skip_cli`: hide the field from command-line flags.
/// - `insensitive`: match the key ignoring ASCII case.
/// - `validate = EXPR`: callable as `fn(&FieldType) -> Result<(),
///   ConfigError>`. Repeatable. Runs after the value parsed.
/// - `sensitive`: redact the value in error messages. The derive then also
///   implements `Debug`, printing `***` for these fields, so do not also derive
///   `Debug` on such a struct.
/// - `deprecated` / `deprecated = "message"`: warn when the key is present. The
///   default message is `is deprecated`.
/// - `skip`: not loaded, not in the shape; the field is `Default::default()`.
///   Cannot be combined with other attributes.
/// - `flatten`: the field type's keys live at this level. Only `sensitive` may
///   accompany it. A non-struct field type fails at load time.
///
/// The inner field of a newtype struct (`struct Port(u16);`) accepts only
/// `validate`; the struct itself only `file`. Newtypes load transparently as
/// their inner type.
///
/// # Enum attributes
///
/// - `rename_all = "..."`: key style for variant names.
/// - `tag = "type"`: internally tagged representation. The node is a table and
///   the tag key selects the variant. Without `tag`, a unit variant is a plain
///   string and a data variant is a table with exactly one key.
///
/// # Variant attributes
///
/// - `rename = "key"` and `alias = "key"` (repeatable). Struct variants take
///   the field attributes above; their field keys are never case-converted.
///   Newtype variants wrap one value; tuple variants with more fields are
///   rejected.
#[proc_macro_derive(Config, attributes(config))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let result = syn::parse::<DeriveInput>(input).and_then(|input| expand(&input));
    match result {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Expansion entry point, free of `proc_macro` types so unit tests can call it.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let mut errors = Errors::default();
    if !input.generics.params.is_empty() {
        errors.push(Error::new_spanned(
            &input.generics,
            "`Config` cannot be derived for generic types or types with lifetimes",
        ));
    }
    let tokens = match &input.data {
        Data::Struct(data) => expand_struct::expand(input, data, &mut errors),
        Data::Enum(data) => expand_enum::expand(input, data, &mut errors),
        Data::Union(_) => {
            errors.push(Error::new_spanned(&input.ident, "`Config` cannot be derived for unions"));
            TokenStream2::new()
        },
    };
    errors.finish(tokens)
}

#[cfg(test)]
mod tests {
    use super::expand;

    fn run(source: &str) -> Result<String, Vec<String>> {
        let input = syn::parse_str(source).expect("test input parses");
        expand(&input)
            .map(|tokens| tokens.to_string())
            .map_err(|error| error.into_iter().map(|e| e.to_string()).collect())
    }

    fn errors(source: &str) -> Vec<String> {
        run(source).expect_err("expected derive errors")
    }

    fn one_error(source: &str, needle: &str) {
        let errors = errors(source);
        assert!(errors.iter().any(|e| e.contains(needle)), "{needle:?} not in {errors:?}");
    }

    fn compact(tokens: &str) -> String {
        tokens.split_whitespace().collect()
    }

    #[test]
    fn rejects_generics_and_lifetimes() {
        one_error("struct S<T> { a: T }", "generic types");
        one_error("struct S<'a> { a: &'a str }", "lifetimes");
    }

    #[test]
    fn rejects_unions_unit_and_wide_tuples() {
        one_error("union U { a: u8 }", "unions");
        one_error("struct S;", "unit structs");
        one_error("struct S(u8, u8);", "exactly one field");
        one_error("enum E { A(u8, u8) }", "exactly one field");
    }

    #[test]
    fn rejects_deny_unknown_fields_with_flatten() {
        one_error(
            "#[config(deny_unknown_fields)] struct S { #[config(flatten)] a: A }",
            "cannot be combined with `flatten`",
        );
    }

    #[test]
    fn rejects_default_with_skip() {
        one_error(
            "struct S { #[config(default, skip)] a: u8 }",
            "`default` cannot be combined with `skip`",
        );
    }

    #[test]
    fn rejects_unknown_key_and_keeps_going() {
        let errors = errors("struct S { #[config(bogus = 1, skip_cli, nope)] a: u8 }");
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].contains("unknown `config` key `bogus`"));
        assert!(errors[1].contains("unknown `config` key `nope`"));
    }

    #[test]
    fn short_needs_char_literal() {
        one_error("struct S { #[config(short = \"c\")] a: u8 }", "single character literal");
        one_error("struct S { #[config(short = 5)] a: u8 }", "single character literal");
        assert!(run("struct S { #[config(short = 'c')] a: u8 }").is_ok());
    }

    #[test]
    fn rejects_wrong_position() {
        one_error(
            "#[config(rename = \"x\")] struct S { a: u8 }",
            "`rename` is not allowed on a struct",
        );
        one_error("struct S { #[config(env_prefix = \"A_\")] a: u8 }", "not allowed on a field");
        one_error("#[config(file = \"a.toml\")] enum E { A }", "`file` is not allowed on an enum");
        one_error("enum E { #[config(tag = \"t\")] A }", "`tag` is not allowed on a variant");
        one_error("struct P(#[config(default)] u8);", "newtype struct");
        one_error(
            "#[config(rename_all = \"kebab-case\")] struct P(u8);",
            "not allowed on a newtype struct",
        );
    }

    #[test]
    fn rejects_bad_values() {
        one_error(
            "#[config(rename_all = \"Title\")] struct S { a: u8 }",
            "unknown rename_all style",
        );
        one_error(
            "#[config(file(format = \"xml\", path = \"a\"))] struct S { a: u8 }",
            "unknown format",
        );
        one_error("#[config(file(format = \"toml\"))] struct S { a: u8 }", "requires `path");
        one_error("#[config(file(path = \"a\", priority = 300))] struct S { a: u8 }", "too large");
        one_error("struct S { #[config(skip = true)] a: u8 }", "takes no value");
        one_error("struct S { #[config(rename = \"a\", rename = \"b\")] a: u8 }", "duplicate");
    }

    #[test]
    fn rejects_duplicate_keys() {
        one_error("struct S { a: u8, #[config(rename = \"a\")] b: u8 }", "duplicate key \"a\"");
        one_error("enum E { A, #[config(alias = \"A\")] B }", "duplicate variant key");
    }

    #[test]
    fn combines_errors_across_items() {
        let errors = errors(
            "struct S<T> { #[config(default, skip)] a: u8, #[config(short = 1)] b: u8, \
             #[config(env_prefix = \"X\")] c: T }",
        );
        assert!(errors.len() >= 4, "{errors:?}");
    }

    #[test]
    fn struct_expansion_shape() {
        let tokens = run(r#"
            /// Server settings.
            #[config(file = "app.toml", file(path = "local.json", format = "yml", priority = 20, optional))]
            #[config(env_prefix = "APP_", rename_all = "kebab-case", deny_unknown_fields, validate = check)]
            struct Server {
                /// Listen port.
                #[config(default = 8080, alias = "p", env = "PORT", flag = "port", short = 'p', validate = positive)]
                listen_port: u16,
                #[config(sensitive, default = "x", deprecated = "use token")]
                api_key: String,
                #[config(skip)]
                cache: Vec<u8>,
            }
        "#)
        .expect("expands");
        let t = compact(&tokens);
        for needle in [
            "impl::forgeconf::FromNodeforServer",
            "let__table=::forgeconf::__private::table(node,\"Server\")?;",
            "::forgeconf::__private::field::<u16>(node,__table,&[\"listen-port\",\"p\"],false,",
            "::core::option::Option::Some(||8080)",
            "::forgeconf::__private::field::<String>(node,__table,&[\"api-key\"],false,",
            "::core::option::Option::Some(||::core::convert::Into::into(\"x\"))",
            ".map_err(::forgeconf::ConfigError::redacted)",
            "cache:::core::default::Default::default()",
            "::forgeconf::__private::unknown_keys(__table,&[\"listen-port\",\"p\",\"api-key\"],&\
             [],&mut__errors);",
            "__errors.take((check)(&__value));",
            "iflet(::core::option::Option::Some(__f0),::core::option::Option::Some(__f1),)=(__f0,\
             __f1,)",
            "static__FIELDS:[::forgeconf::FieldInfo;2usize]",
            ".default_text(\"8080\")",
            ".default_text(\"x\")",
            ".short('p')",
            ".deprecated(\"usetoken\")",
            ".sensitive()",
            ".doc(\"Listenport.\")",
            ".env_prefix(\"APP_\")",
            ".doc(\"Serversettings.\")",
            "::forgeconf::ConfigFile::new(\"app.toml\")",
            ".format(::forgeconf::FileFormat::Yaml).priority(20u8).optional()",
            "impl::core::fmt::DebugforServer",
            ".field(\"api_key\",&::core::format_args!(\"***\"))",
        ] {
            assert!(t.contains(needle), "{needle} missing in {t}");
        }
    }

    #[test]
    fn newtype_expansion_delegates() {
        let t = compact(
            &run("#[config(file = \"p.toml\")] struct Port(#[config(validate = nonzero)] u16);")
                .expect("expands"),
        );
        assert!(t.contains("<u16as::forgeconf::FromNode>::from_missing().map(Self)"));
        assert!(t.contains("<u16as::forgeconf::FromNode>::shape()"));
        assert!(t.contains("(nonzero)(&__value)"));
        assert!(t.contains("impl::forgeconf::ConfigforPort"));
    }

    #[test]
    fn enum_expansion_shape() {
        let t = compact(
            &run(r#"
                #[config(rename_all = "snake_case")]
                enum Backend {
                    /// In memory.
                    InMemory,
                    #[config(alias = "pg")]
                    Postgres { host: String },
                    Redis(String),
                }
            "#)
            .expect("expands"),
        );
        for needle in [
            "const__EXPECTED:&[&str]=&[\"in_memory\"];",
            "\"in_memory\"=>::core::result::Result::Ok(Self::InMemory)",
            "\"postgres\"|\"pg\"=>",
            ".map_err(|e|e.at_key(\"postgres\"))",
            "::forgeconf::__private::scalar(node,\"oneof:in_memory\")?",
            "::forgeconf::VariantInfo::new(\"postgres\").data(||::forgeconf::Shape::Struct(&\
             __V1_INFO))",
            "::forgeconf::VariantInfo::new(\"redis\").data(<Stringas::forgeconf::FromNode>::shape)",
            "::forgeconf::Shape::Enum(&__INFO)",
        ] {
            assert!(t.contains(needle), "{needle} missing in {t}");
        }
        assert!(!t.contains("Config for"), "enums must not implement Config");
    }

    #[test]
    fn tagged_enum_reads_tag_key() {
        let t = compact(&run("#[config(tag = \"type\")] enum E { A, B(u8) }").expect("expands"));
        assert!(t.contains("__table.get(\"type\")"));
        assert!(t.contains(".tag(\"type\")"));
        assert!(t.contains("ConfigError::missing().at_key(\"type\")"));
    }
}
