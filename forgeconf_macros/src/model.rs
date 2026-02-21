use forgeconf_core::FileFormat;
use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Error, Expr, Field, Ident, ItemStruct, LitBool, LitInt, LitStr, Result, Token, Type};

#[derive(Default)]
pub struct ForgeconfAttr {
    pub files: Vec<ConfigFile>,
}

pub struct ConfigFile {
    pub path: Expr,
    pub format: Option<FileFormat>,
    pub priority: Option<u8>,
}

#[derive(Clone)]
pub struct FieldSpec {
    pub ident: Ident,
    pub ty: Type,
    pub options: FieldOptions,
}

#[derive(Clone, Default)]
pub struct FieldOptions {
    pub rename: Option<String>,
    pub insensitive: bool,
    pub env: Option<String>,
    pub cli: Option<String>,
    pub default: Option<Expr>,
    pub optional: bool,
    pub validators: Vec<Expr>,
}

impl Parse for ForgeconfAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut files = Vec::new();

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            if ident != "config" {
                return Err(Error::new(ident.span(), "expected `config(...)`"));
            }

            let content;
            syn::parenthesized!(content in input);
            files.push(ConfigFile::parse(&content)?);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self { files })
    }
}

impl ConfigFile {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut path = None;
        let mut format = None;
        let mut priority = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident
                .to_string()
                .as_str()
            {
                "path" => {
                    let expr: Expr = input.parse()?;
                    path = Some(expr);
                },
                "format" => {
                    let lit: LitStr = input.parse()?;
                    format = Some(
                        lit.value()
                            .parse::<FileFormat>()
                            .map_err(|err| {
                                Error::new(lit.span(), format!("invalid format: {err}"))
                            })?,
                    );
                },
                "priority" => {
                    let lit: LitInt = input.parse()?;
                    priority = Some(lit.base10_parse()?);
                },
                other => {
                    return Err(Error::new(ident.span(), format!("unknown argument `{other}`")));
                },
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let path = path.ok_or_else(|| Error::new(Span::call_site(), "missing `path`"))?;
        Ok(Self { path, format, priority })
    }
}

pub fn collect_fields(item: &mut ItemStruct) -> Result<Vec<FieldSpec>> {
    let mut specs = Vec::new();

    for field in item
        .fields
        .iter_mut()
    {
        specs.push(parse_field(field)?);
    }

    Ok(specs)
}

fn parse_field(field: &mut Field) -> Result<FieldSpec> {
    let ident = field
        .ident
        .clone()
        .ok_or_else(|| Error::new(field.span(), "tuple structs are not supported"))?;

    let mut options = FieldOptions::default();

    let mut retained = Vec::new();
    for attr in field
        .attrs
        .drain(..)
    {
        if attr
            .path()
            .is_ident("field")
        {
            let parsed = attr.parse_args_with(FieldOptions::parse)?;
            options.merge(parsed)?;
        } else {
            retained.push(attr);
        }
    }

    field.attrs = retained;

    options.validate(&field.ty, &ident)?;

    Ok(FieldSpec {
        ident,
        ty: field
            .ty
            .clone(),
        options,
    })
}

impl FieldOptions {
    fn parse(input: ParseStream) -> Result<Self> {
        let pairs: Punctuated<MetaEntry, Token![,]> =
            input.parse_terminated(MetaEntry::parse, Token![,])?;
        let mut options = FieldOptions::default();

        for pair in pairs {
            match pair {
                MetaEntry::Rename(value) => options.rename = Some(value.value()),
                MetaEntry::Insensitive(flag) => options.insensitive = flag.value(),
                MetaEntry::Env(value) => options.env = Some(value.value()),
                MetaEntry::Cli(value) => options.cli = Some(value.value()),
                MetaEntry::Optional(flag) => options.optional = flag.value(),
                MetaEntry::Default(expr) => options.default = Some(expr),
                MetaEntry::Validator(expr) => options
                    .validators
                    .push(expr),
            }
        }

        Ok(options)
    }

    // TODO: refactor this in a more efficient way
    fn merge(&mut self, other: FieldOptions) -> Result<()> {
        if other
            .rename
            .is_some()
        {
            self.rename = other.rename;
        }
        if other.insensitive {
            self.insensitive = true;
        }
        if other
            .env
            .is_some()
        {
            self.env = other.env;
        }
        if other
            .cli
            .is_some()
        {
            self.cli = other.cli;
        }
        if other
            .default
            .is_some()
        {
            self.default = other.default;
        }
        if other.optional {
            self.optional = true;
        }
        if !other
            .validators
            .is_empty()
        {
            self.validators
                .extend(other.validators);
        }
        Ok(())
    }

    fn validate(&self, ty: &Type, ident: &Ident) -> Result<()> {
        if self.optional && !is_option_type(ty) {
            return Err(Error::new(ident.span(), "optional fields must use Option<T>"));
        }
        if self.optional
            && self
                .default
                .is_some()
        {
            return Err(Error::new(ident.span(), "an optional field cannot declare a default"));
        }
        Ok(())
    }
}

enum MetaEntry {
    Rename(LitStr),
    Insensitive(LitBool),
    Env(LitStr),
    Cli(LitStr),
    Optional(LitBool),
    Default(Expr),
    Validator(Expr),
}

impl Parse for MetaEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident: Ident = input.parse()?;
        input.parse::<Token![=]>()?;

        match ident
            .to_string()
            .as_str()
        {
            "name" => Ok(MetaEntry::Rename(input.parse()?)),
            "insensitive" => Ok(MetaEntry::Insensitive(input.parse()?)),
            "env" => Ok(MetaEntry::Env(input.parse()?)),
            "cli" => Ok(MetaEntry::Cli(input.parse()?)),
            "optional" => Ok(MetaEntry::Optional(input.parse()?)),
            "default" => Ok(MetaEntry::Default(input.parse()?)),
            "validate" => Ok(MetaEntry::Validator(input.parse()?)),
            other => Err(Error::new(ident.span(), format!("unknown field attribute `{other}`"))),
        }
    }
}

pub fn is_scalar_type(ty: &Type) -> bool {
    if let Type::Path(path) = ty
        && let Some(segment) = path
            .path
            .segments
            .last()
    {
        let ident = segment
            .ident
            .to_string();
        return matches!(
            ident.as_str(),
            "String"
                | "bool"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "usize"
                | "f32"
                | "f64"
                | "char"
        ) || ident == "Vec";
    }

    false
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(path) = ty
        && let Some(segment) = path
            .path
            .segments
            .last()
    {
        return segment.ident == "Option";
    }

    false
}
