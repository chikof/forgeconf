//! Struct and newtype expansion, plus the field machinery enum struct
//! variants reuse.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ext::IdentExt;
use syn::{
    Attribute,
    DataStruct,
    DeriveInput,
    Error,
    Expr,
    ExprLit,
    Fields,
    FieldsNamed,
    Ident,
    Lit,
    Meta,
    MetaNameValue,
    Type,
};

use crate::attr::{DefaultSpec, Errors, FieldAttrs, FileSpec, Format, field_attrs, struct_attrs};
use crate::case::Case;

pub(crate) struct FieldDef {
    pub ident: Ident,
    pub ty: Type,
    pub key: String,
    pub docs: Option<String>,
    pub attrs: FieldAttrs,
}

impl FieldDef {
    /// Reads a table key (not skipped, not flattened).
    fn is_keyed(&self) -> bool {
        !self.attrs.skip && !self.attrs.flatten
    }

    fn names(&self) -> impl Iterator<Item = String> + '_ {
        std::iter::once(self.key.clone()).chain(self.attrs.aliases.iter().map(syn::LitStr::value))
    }
}

pub(crate) fn doc_string(attrs: &[Attribute]) -> Option<String> {
    let lines: Vec<String> = attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .filter_map(|a| match &a.meta {
            Meta::NameValue(MetaNameValue {
                value: Expr::Lit(ExprLit { lit: Lit::Str(s), .. }),
                ..
            }) => Some(s.value().trim().to_owned()),
            _ => None,
        })
        .collect();
    if lines.is_empty() { None } else { Some(lines.join("\n")) }
}

pub(crate) fn parse_fields(
    fields: &FieldsNamed,
    rename_all: Option<Case>,
    errors: &mut Errors,
) -> Vec<FieldDef> {
    let mut out = Vec::new();
    for field in &fields.named {
        let Some(ident) = field.ident.clone() else { continue };
        let attrs = field_attrs(&field.attrs, false, errors);
        let plain = ident.unraw().to_string();
        let key = match (&attrs.rename, rename_all) {
            (Some(lit), _) => lit.value(),
            (None, Some(case)) => case.apply_field(&plain),
            (None, None) => plain,
        };
        out.push(FieldDef {
            ident,
            ty: field.ty.clone(),
            key,
            docs: doc_string(&field.attrs),
            attrs,
        });
    }
    let mut seen = HashSet::new();
    for field in out.iter().filter(|f| f.is_keyed()) {
        for name in field.names() {
            if !seen.insert(name.clone()) {
                errors.push(Error::new(field.ident.span(), format!("duplicate key \"{name}\"")));
            }
        }
    }
    out
}

fn default_tokens(field: &FieldDef) -> TokenStream {
    let ty = &field.ty;
    match &field.attrs.default {
        None => quote!(::core::option::Option::None),
        Some(DefaultSpec::Trait) => {
            quote!(::core::option::Option::Some(<#ty as ::core::default::Default>::default))
        },
        Some(DefaultSpec::Expr(expr)) => {
            if matches!(**expr, Expr::Lit(ExprLit { lit: Lit::Str(_), .. })) {
                quote!(::core::option::Option::Some(|| ::core::convert::Into::into(#expr)))
            } else {
                quote!(::core::option::Option::Some(|| #expr))
            }
        },
    }
}

fn default_text(expr: &Expr) -> String {
    match expr {
        Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => s.value(),
        Expr::Lit(lit) => lit.to_token_stream().to_string(),
        other => compact(&other.to_token_stream().to_string()),
    }
}

// Token streams print with a space between every token (`Log :: Info`);
// help text should read like the source (`Log::Info`).
fn compact(text: &str) -> String {
    [
        (" :: ", "::"),
        (" ! ", "!"),
        (" (", "("),
        ("( ", "("),
        (" )", ")"),
        (" [", "["),
        ("[ ", "["),
        (" ]", "]"),
        (" ,", ","),
        (" . ", "."),
    ]
    .iter()
    .fold(text.to_owned(), |acc, (from, to)| acc.replace(from, to))
}

fn validate_tokens(field: &FieldDef) -> TokenStream {
    let ty = &field.ty;
    let validators = &field.attrs.validators;
    if validators.is_empty() {
        quote!(|_: &#ty| ::core::result::Result::Ok(()))
    } else {
        quote! {
            |__value: &#ty| {
                #( (#validators)(__value)?; )*
                ::core::result::Result::Ok(())
            }
        }
    }
}

fn field_stmt(index: usize, field: &FieldDef) -> TokenStream {
    let var = quote::format_ident!("__f{}", index);
    let ty = &field.ty;
    if field.attrs.flatten {
        return quote! {
            let #var = __errors.take(<#ty as ::forgeconf::FromNode>::from_node(node));
        };
    }
    let names = field.names();
    let insensitive = field.attrs.insensitive;
    let default = default_tokens(field);
    let validate = validate_tokens(field);
    let redact = field
        .attrs
        .sensitive
        .then(|| quote!(.map_err(::forgeconf::ConfigError::redacted)));
    quote! {
        let #var = __errors.take(
            ::forgeconf::__private::field::<#ty>(node, __table, &[#(#names),*], #insensitive, #default, #validate)
                #redact
        );
    }
}

/// Statements plus tail expression reading `fields` from `node` / `__table`
/// and building `ctor { .. }`. The tail has type `Result<_, ConfigError>`.
pub(crate) fn from_table(
    fields: &[FieldDef],
    ctor: &TokenStream,
    deny_unknown_fields: bool,
    validators: &[Expr],
) -> TokenStream {
    let mut stmts = Vec::new();
    let mut vars = Vec::new();
    let mut inits = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let ident = &field.ident;
        if field.attrs.skip {
            inits.push(quote!(#ident: ::core::default::Default::default()));
            continue;
        }
        let var = quote::format_ident!("__f{}", index);
        stmts.push(field_stmt(index, field));
        inits.push(quote!(#ident: #var));
        vars.push(var);
    }
    let unknown = deny_unknown_fields.then(|| {
        let known = fields.iter().filter(|f| f.is_keyed()).flat_map(FieldDef::names);
        let insensitive = fields
            .iter()
            .filter(|f| f.is_keyed() && f.attrs.insensitive)
            .flat_map(FieldDef::names);
        quote!(::forgeconf::__private::unknown_keys(__table, &[#(#known),*], &[#(#insensitive),*], &mut __errors);)
    });
    let build = quote! {
        let __value = #ctor { #(#inits),* };
        #( __errors.take((#validators)(&__value)); )*
        __errors.finish(__value)
    };
    let finish = if vars.is_empty() {
        build
    } else {
        quote! {
            if let (#(::core::option::Option::Some(#vars),)*) = (#(#vars,)*) {
                #build
            } else {
                ::core::result::Result::Err(__errors.into_error())
            }
        }
    };
    quote! {
        #[allow(unused_mut)]
        let mut __errors = ::forgeconf::__private::Collector::new();
        #(#stmts)*
        #unknown
        #finish
    }
}

pub(crate) fn field_info(field: &FieldDef) -> TokenStream {
    let key = &field.key;
    let ty = &field.ty;
    let attrs = &field.attrs;
    let mut info = quote!(::forgeconf::FieldInfo::new(#key, <#ty as ::forgeconf::FromNode>::shape));
    if !attrs.aliases.is_empty() {
        let aliases = &attrs.aliases;
        info = quote!(#info.aliases(&[#(#aliases),*]));
    }
    if let Some(doc) = &field.docs {
        info = quote!(#info.doc(#doc));
    }
    if let Some(env) = &attrs.env {
        info = quote!(#info.env(#env));
    }
    if let Some(flag) = &attrs.flag {
        info = quote!(#info.flag(#flag));
    }
    if let Some(short) = &attrs.short {
        info = quote!(#info.short(#short));
    }
    if attrs.skip_cli {
        info = quote!(#info.skip_cli());
    }
    if let Some(DefaultSpec::Expr(expr)) = &attrs.default {
        let text = default_text(expr);
        info = quote!(#info.default_text(#text));
    }
    if attrs.sensitive {
        info = quote!(#info.sensitive());
    }
    if let Some(message) = &attrs.deprecated {
        info = quote!(#info.deprecated(#message));
    }
    if attrs.flatten {
        info = quote!(#info.flatten());
    }
    info
}

/// `static` field array named `fields_name` plus `static` struct info named
/// `info_name`, to be emitted inside a function body.
pub(crate) fn struct_info_statics(
    fields_name: &Ident,
    info_name: &Ident,
    type_name: &str,
    fields: &[FieldDef],
    docs: Option<&str>,
    env_prefix: Option<&str>,
) -> TokenStream {
    let infos: Vec<_> = fields
        .iter()
        .filter(|f| !f.attrs.skip)
        .map(field_info)
        .collect();
    let count = infos.len();
    let doc = docs.map(|d| quote!(.doc(#d)));
    let prefix = env_prefix.map(|p| quote!(.env_prefix(#p)));
    quote! {
        static #fields_name: [::forgeconf::FieldInfo; #count] = [#(#infos),*];
        static #info_name: ::forgeconf::StructInfo =
            ::forgeconf::StructInfo::new(#type_name, &#fields_name) #doc #prefix;
    }
}

fn config_impl(ident: &Ident, files: &[FileSpec]) -> TokenStream {
    let files = files.iter().map(|file| {
        let path = &file.path;
        let mut tokens = quote!(::forgeconf::ConfigFile::new(#path));
        if let Some(format) = file.format {
            let variant = match format {
                Format::Toml => quote!(Toml),
                Format::Json => quote!(Json),
                Format::Yaml => quote!(Yaml),
            };
            tokens = quote!(#tokens.format(::forgeconf::FileFormat::#variant));
        }
        if let Some(priority) = file.priority {
            tokens = quote!(#tokens.priority(#priority));
        }
        if file.optional {
            tokens = quote!(#tokens.optional());
        }
        quote!(.file(#tokens))
    });
    quote! {
        #[automatically_derived]
        impl ::forgeconf::Config for #ident {
            fn loader() -> ::forgeconf::Loader<Self> {
                ::forgeconf::Loader::new() #(#files)*
            }
        }
    }
}

fn debug_impl(ident: &Ident, fields: &[FieldDef]) -> TokenStream {
    let name = ident.unraw().to_string();
    let entries = fields.iter().map(|field| {
        let id = &field.ident;
        let label = id.unraw().to_string();
        if field.attrs.sensitive {
            quote!(.field(#label, &::core::format_args!("***")))
        } else {
            quote!(.field(#label, &self.#id))
        }
    });
    quote! {
        #[automatically_derived]
        impl ::core::fmt::Debug for #ident {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(#name) #(#entries)* .finish()
            }
        }
    }
}

pub(crate) fn expand(input: &DeriveInput, data: &DataStruct, errors: &mut Errors) -> TokenStream {
    match &data.fields {
        Fields::Named(named) => named_struct(input, named, errors),
        Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => newtype(input, unnamed, errors),
        Fields::Unnamed(unnamed) => {
            errors.push(Error::new_spanned(
                unnamed,
                "tuple structs must have exactly one field; use a named struct instead",
            ));
            TokenStream::new()
        },
        Fields::Unit => {
            errors.push(Error::new_spanned(&input.ident, "unit structs cannot derive `Config`"));
            TokenStream::new()
        },
    }
}

fn named_struct(input: &DeriveInput, named: &FieldsNamed, errors: &mut Errors) -> TokenStream {
    let ident = &input.ident;
    let name = ident.unraw().to_string();
    let attrs = struct_attrs(&input.attrs, false, errors);
    let fields = parse_fields(named, attrs.rename_all, errors);
    if let Some(span) = attrs.deny_unknown_fields {
        if fields.iter().any(|f| f.attrs.flatten) {
            errors.push(Error::new(
                span,
                "`deny_unknown_fields` cannot be combined with `flatten` fields",
            ));
        }
    }
    let deny = attrs.deny_unknown_fields.is_some();
    let body = from_table(&fields, &quote!(Self), deny, &attrs.validators);
    let statics = struct_info_statics(
        &quote::format_ident!("__FIELDS"),
        &quote::format_ident!("__INFO"),
        &name,
        &fields,
        doc_string(&input.attrs).as_deref(),
        attrs.env_prefix.as_ref().map(syn::LitStr::value).as_deref(),
    );
    let config = config_impl(ident, &attrs.files);
    let debug = fields
        .iter()
        .any(|f| f.attrs.sensitive)
        .then(|| debug_impl(ident, &fields));
    quote! {
        #[automatically_derived]
        impl ::forgeconf::FromNode for #ident {
            fn from_node(
                node: &::forgeconf::ConfigNode,
            ) -> ::core::result::Result<Self, ::forgeconf::ConfigError> {
                let __table = ::forgeconf::__private::table(node, #name)?;
                #body
            }

            fn shape() -> ::forgeconf::Shape {
                #statics
                ::forgeconf::Shape::Struct(&__INFO)
            }
        }
        #config
        #debug
    }
}

fn newtype(input: &DeriveInput, unnamed: &syn::FieldsUnnamed, errors: &mut Errors) -> TokenStream {
    let ident = &input.ident;
    let attrs = struct_attrs(&input.attrs, true, errors);
    let Some(field) = unnamed.unnamed.first() else { return TokenStream::new() };
    let inner = &field.ty;
    let field_attrs = field_attrs(&field.attrs, true, errors);
    let validators = &field_attrs.validators;
    let config = config_impl(ident, &attrs.files);
    quote! {
        #[automatically_derived]
        impl ::forgeconf::FromNode for #ident {
            fn from_node(
                node: &::forgeconf::ConfigNode,
            ) -> ::core::result::Result<Self, ::forgeconf::ConfigError> {
                let __value = <#inner as ::forgeconf::FromNode>::from_node(node)?;
                #(
                    (#validators)(&__value)
                        .map_err(|e: ::forgeconf::ConfigError| e.with_origin(node.origin()))?;
                )*
                ::core::result::Result::Ok(Self(__value))
            }

            fn from_missing() -> ::core::option::Option<Self> {
                <#inner as ::forgeconf::FromNode>::from_missing().map(Self)
            }

            fn shape() -> ::forgeconf::Shape {
                <#inner as ::forgeconf::FromNode>::shape()
            }
        }
        #config
    }
}

#[cfg(test)]
mod test {
    use super::default_text;

    #[test]
    fn default_text_reads_like_source() {
        let text = |src: &str| default_text(&syn::parse_str(src).expect("expression parses"));
        assert_eq!(text("Duration::from_secs(30)"), "Duration::from_secs(30)");
        assert_eq!(text("Log::Info"), "Log::Info");
        assert_eq!(text("vec![1, 2]"), "vec![1, 2]");
        assert_eq!(text("\"h\""), "h");
        assert_eq!(text("8080"), "8080");
    }
}
