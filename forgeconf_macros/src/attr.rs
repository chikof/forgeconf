//! Parsing of `#[config(...)]` on structs, fields, enums and variants.

use proc_macro2::Span;
use syn::meta::ParseNestedMeta;
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::{Attribute, Error, Expr, ExprLit, Lit, LitChar, LitInt, LitStr, Result, Token, token};

use crate::case::Case;

/// Accumulates spanned errors so one derive run reports all of them.
#[derive(Default)]
pub(crate) struct Errors(Option<Error>);

impl Errors {
    pub(crate) fn push(&mut self, error: Error) {
        match &mut self.0 {
            Some(first) => first.combine(error),
            None => self.0 = Some(error),
        }
    }

    pub(crate) fn finish<T>(self, value: T) -> Result<T> {
        match self.0 {
            Some(error) => Err(error),
            None => Ok(value),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Format {
    Toml,
    Json,
    Yaml,
}

pub(crate) struct FileSpec {
    pub path: Expr,
    pub format: Option<Format>,
    pub priority: Option<u8>,
    pub optional: bool,
}

#[derive(Default)]
pub(crate) struct StructAttrs {
    pub files: Vec<FileSpec>,
    pub env_prefix: Option<LitStr>,
    pub rename_all: Option<Case>,
    pub deny_unknown_fields: Option<Span>,
    pub validators: Vec<Expr>,
}

#[derive(Default)]
pub(crate) struct EnumAttrs {
    pub rename_all: Option<Case>,
    pub tag: Option<LitStr>,
}

#[derive(Default)]
pub(crate) struct VariantAttrs {
    pub rename: Option<LitStr>,
    pub aliases: Vec<LitStr>,
}

pub(crate) enum DefaultSpec {
    Trait,
    Expr(Box<Expr>),
}

#[derive(Default)]
#[allow(clippy::struct_excessive_bools, reason = "one flag per user-facing field attribute")]
pub(crate) struct FieldAttrs {
    pub rename: Option<LitStr>,
    pub aliases: Vec<LitStr>,
    pub default: Option<DefaultSpec>,
    pub env: Option<LitStr>,
    pub flag: Option<LitStr>,
    pub short: Option<LitChar>,
    pub skip_cli: bool,
    pub insensitive: bool,
    pub validators: Vec<Expr>,
    pub sensitive: bool,
    pub deprecated: Option<String>,
    pub skip: bool,
    pub flatten: bool,
}

const KEYS: &[&str] = &[
    "file",
    "env_prefix",
    "rename_all",
    "deny_unknown_fields",
    "validate",
    "rename",
    "alias",
    "default",
    "env",
    "flag",
    "short",
    "skip_cli",
    "insensitive",
    "sensitive",
    "deprecated",
    "skip",
    "flatten",
    "tag",
];

#[derive(Default)]
#[allow(clippy::struct_excessive_bools, reason = "one flag per user-facing attribute key")]
struct Raw {
    seen: Vec<(&'static str, Span)>,
    files: Vec<FileSpec>,
    env_prefix: Option<LitStr>,
    rename_all: Option<Case>,
    deny_unknown_fields: Option<Span>,
    validators: Vec<Expr>,
    rename: Option<LitStr>,
    aliases: Vec<LitStr>,
    default: Option<DefaultSpec>,
    env: Option<LitStr>,
    flag: Option<LitStr>,
    short: Option<LitChar>,
    skip_cli: bool,
    insensitive: bool,
    sensitive: bool,
    deprecated: Option<String>,
    skip: bool,
    flatten: bool,
    tag: Option<LitStr>,
}

fn set_once<T>(slot: &mut Option<T>, value: T, meta: &ParseNestedMeta) -> Result<()> {
    if slot.is_some() {
        return Err(meta.error("duplicate attribute key"));
    }
    *slot = Some(value);
    Ok(())
}

fn no_value(meta: &ParseNestedMeta) -> Result<()> {
    if meta.input.peek(Token![=]) || meta.input.peek(token::Paren) {
        return Err(meta.error("this attribute takes no value"));
    }
    Ok(())
}

fn lit_str(meta: &ParseNestedMeta) -> Result<LitStr> {
    meta.value()?.parse()
}

fn skip_value(input: ParseStream) -> Result<()> {
    if input.peek(Token![=]) {
        input.parse::<Token![=]>()?;
        input.parse::<Expr>()?;
    } else if input.peek(token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        content.parse::<proc_macro2::TokenStream>()?;
    }
    Ok(())
}

impl Format {
    fn parse(lit: &LitStr) -> Result<Self> {
        match lit.value().as_str() {
            "toml" => Ok(Self::Toml),
            "json" => Ok(Self::Json),
            "yaml" | "yml" => Ok(Self::Yaml),
            other => Err(Error::new(
                lit.span(),
                format!(
                    "unknown format \"{other}\", expected \"toml\", \"json\", \"yaml\" or \"yml\""
                ),
            )),
        }
    }
}

fn parse_file(meta: &ParseNestedMeta) -> Result<FileSpec> {
    if meta.input.peek(Token![=]) {
        return Ok(FileSpec {
            path: meta.value()?.parse()?,
            format: None,
            priority: None,
            optional: false,
        });
    }
    let (mut path, mut format, mut priority, mut optional) = (None, None, None, false);
    meta.parse_nested_meta(|m| {
        if m.path.is_ident("path") {
            set_once(&mut path, m.value()?.parse::<Expr>()?, &m)
        } else if m.path.is_ident("format") {
            let lit: LitStr = m.value()?.parse()?;
            set_once(&mut format, Format::parse(&lit)?, &m)
        } else if m.path.is_ident("priority") {
            let lit: LitInt = m.value()?.parse()?;
            set_once(&mut priority, lit.base10_parse::<u8>()?, &m)
        } else if m.path.is_ident("optional") {
            no_value(&m)?;
            optional = true;
            Ok(())
        } else {
            Err(m.error("unknown `file` key, expected `path`, `format`, `priority` or `optional`"))
        }
    })?;
    let path = path.ok_or_else(|| meta.error("`file(...)` requires `path = ...`"))?;
    Ok(FileSpec { path, format, priority, optional })
}

impl Raw {
    fn apply(&mut self, key: &'static str, meta: &ParseNestedMeta) -> Result<()> {
        match key {
            "file" => self.files.push(parse_file(meta)?),
            "env_prefix" => set_once(&mut self.env_prefix, lit_str(meta)?, meta)?,
            "rename_all" => {
                let lit = lit_str(meta)?;
                let case = Case::parse(&lit.value()).ok_or_else(|| {
                    Error::new(
                        lit.span(),
                        format!(
                            "unknown rename_all style, expected one of: {}",
                            Case::NAMES.join(", ")
                        ),
                    )
                })?;
                set_once(&mut self.rename_all, case, meta)?;
            },
            "deny_unknown_fields" => {
                no_value(meta)?;
                self.deny_unknown_fields = Some(meta.path.span());
            },
            "validate" => self.validators.push(meta.value()?.parse()?),
            "rename" => set_once(&mut self.rename, lit_str(meta)?, meta)?,
            "alias" => self.aliases.push(lit_str(meta)?),
            "default" => {
                let spec = if meta.input.peek(Token![=]) {
                    DefaultSpec::Expr(Box::new(meta.value()?.parse()?))
                } else {
                    no_value(meta)?;
                    DefaultSpec::Trait
                };
                set_once(&mut self.default, spec, meta)?;
            },
            "env" => set_once(&mut self.env, lit_str(meta)?, meta)?,
            "flag" => set_once(&mut self.flag, lit_str(meta)?, meta)?,
            "short" => {
                let expr: Expr = meta.value()?.parse()?;
                match expr {
                    Expr::Lit(ExprLit { lit: Lit::Char(c), .. }) => {
                        set_once(&mut self.short, c, meta)?
                    },
                    other => {
                        return Err(Error::new(
                            other.span(),
                            "`short` expects a single character literal, e.g. `short = 'c'`",
                        ));
                    },
                }
            },
            "skip_cli" => {
                no_value(meta)?;
                self.skip_cli = true;
            },
            "insensitive" => {
                no_value(meta)?;
                self.insensitive = true;
            },
            "sensitive" => {
                no_value(meta)?;
                self.sensitive = true;
            },
            "deprecated" => {
                let message = if meta.input.peek(Token![=]) {
                    lit_str(meta)?.value()
                } else {
                    no_value(meta)?;
                    "is deprecated".to_owned()
                };
                set_once(&mut self.deprecated, message, meta)?;
            },
            "skip" => {
                no_value(meta)?;
                self.skip = true;
            },
            "flatten" => {
                no_value(meta)?;
                self.flatten = true;
            },
            "tag" => set_once(&mut self.tag, lit_str(meta)?, meta)?,
            _ => return Err(meta.error("unsupported `config` key")),
        }
        Ok(())
    }
}

fn parse_raw(attrs: &[Attribute], errors: &mut Errors) -> Raw {
    let mut raw = Raw::default();
    for attr in attrs.iter().filter(|a| a.path().is_ident("config")) {
        let mut unknown = Vec::new();
        let result = attr.parse_nested_meta(|meta| {
            if let Some(&key) = KEYS.iter().find(|k| meta.path.is_ident(*k)) {
                raw.seen.push((key, meta.path.span()));
                raw.apply(key, &meta)
            } else {
                unknown.push(meta.error(format!(
                    "unknown `config` key `{}`",
                    meta.path
                        .get_ident()
                        .map_or_else(|| "<path>".to_owned(), ToString::to_string)
                )));
                skip_value(meta.input)
            }
        });
        for error in unknown {
            errors.push(error);
        }
        if let Err(error) = result {
            errors.push(error);
        }
    }
    raw
}

fn check_allowed(raw: &Raw, allowed: &[&str], place: &str, errors: &mut Errors) {
    for (key, span) in &raw.seen {
        if !allowed.contains(key) {
            errors.push(Error::new(*span, format!("`{key}` is not allowed on {place}")));
        }
    }
}

pub(crate) fn struct_attrs(attrs: &[Attribute], newtype: bool, errors: &mut Errors) -> StructAttrs {
    let raw = parse_raw(attrs, errors);
    if newtype {
        check_allowed(&raw, &["file"], "a newtype struct", errors);
    } else {
        let allowed = &["file", "env_prefix", "rename_all", "deny_unknown_fields", "validate"];
        check_allowed(&raw, allowed, "a struct", errors);
    }
    StructAttrs {
        files: raw.files,
        env_prefix: raw.env_prefix,
        rename_all: raw.rename_all,
        deny_unknown_fields: raw.deny_unknown_fields,
        validators: raw.validators,
    }
}

pub(crate) fn enum_attrs(attrs: &[Attribute], errors: &mut Errors) -> EnumAttrs {
    let raw = parse_raw(attrs, errors);
    check_allowed(&raw, &["rename_all", "tag"], "an enum", errors);
    EnumAttrs { rename_all: raw.rename_all, tag: raw.tag }
}

pub(crate) fn variant_attrs(attrs: &[Attribute], errors: &mut Errors) -> VariantAttrs {
    let raw = parse_raw(attrs, errors);
    check_allowed(&raw, &["rename", "alias"], "a variant", errors);
    VariantAttrs { rename: raw.rename, aliases: raw.aliases }
}

pub(crate) fn field_attrs(attrs: &[Attribute], newtype: bool, errors: &mut Errors) -> FieldAttrs {
    let raw = parse_raw(attrs, errors);
    if newtype {
        check_allowed(&raw, &["validate"], "the field of a newtype struct", errors);
    } else {
        let allowed = &[
            "rename",
            "alias",
            "default",
            "env",
            "flag",
            "short",
            "skip_cli",
            "insensitive",
            "validate",
            "sensitive",
            "deprecated",
            "skip",
            "flatten",
        ];
        check_allowed(&raw, allowed, "a field", errors);
        combine_rules(&raw, errors);
    }
    FieldAttrs {
        rename: raw.rename,
        aliases: raw.aliases,
        default: raw.default,
        env: raw.env,
        flag: raw.flag,
        short: raw.short,
        skip_cli: raw.skip_cli,
        insensitive: raw.insensitive,
        validators: raw.validators,
        sensitive: raw.sensitive,
        deprecated: raw.deprecated,
        skip: raw.skip,
        flatten: raw.flatten,
    }
}

fn combine_rules(raw: &Raw, errors: &mut Errors) {
    let exclusive = |winner: &str, allowed: &[&str], errors: &mut Errors| {
        for (key, span) in &raw.seen {
            if *key != winner && !allowed.contains(key) {
                errors
                    .push(Error::new(*span, format!("`{key}` cannot be combined with `{winner}`")));
            }
        }
    };
    if raw.skip {
        exclusive("skip", &[], errors);
    }
    if raw.flatten {
        exclusive("flatten", &["sensitive", "skip"], errors);
    }
}
