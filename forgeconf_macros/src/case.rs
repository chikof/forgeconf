//! `rename_all` conversions, following serde's rules: field idents arrive as
//! `snake_case`, variant idents as `PascalCase`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Case {
    Lower,
    Upper,
    Snake,
    Kebab,
    Camel,
    Pascal,
    ScreamingSnake,
    ScreamingKebab,
}

impl Case {
    pub(crate) const NAMES: &'static [&'static str] = &[
        "lowercase",
        "UPPERCASE",
        "snake_case",
        "kebab-case",
        "camelCase",
        "PascalCase",
        "SCREAMING_SNAKE_CASE",
        "SCREAMING-KEBAB-CASE",
    ];

    pub(crate) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "lowercase" => Self::Lower,
            "UPPERCASE" => Self::Upper,
            "snake_case" => Self::Snake,
            "kebab-case" => Self::Kebab,
            "camelCase" => Self::Camel,
            "PascalCase" => Self::Pascal,
            "SCREAMING_SNAKE_CASE" => Self::ScreamingSnake,
            "SCREAMING-KEBAB-CASE" => Self::ScreamingKebab,
            _ => return None,
        })
    }

    /// `ident` is a `snake_case` field name without `r#`.
    pub(crate) fn apply_field(self, ident: &str) -> String {
        match self {
            Self::Lower | Self::Snake => ident.to_owned(),
            Self::Upper | Self::ScreamingSnake => ident.to_ascii_uppercase(),
            Self::Pascal => pascal_from_snake(ident),
            Self::Camel => lower_first(&pascal_from_snake(ident)),
            Self::Kebab => ident.replace('_', "-"),
            Self::ScreamingKebab => ident.to_ascii_uppercase().replace('_', "-"),
        }
    }

    /// `ident` is a `PascalCase` variant name without `r#`.
    pub(crate) fn apply_variant(self, ident: &str) -> String {
        match self {
            Self::Pascal => ident.to_owned(),
            Self::Lower => ident.to_ascii_lowercase(),
            Self::Upper => ident.to_ascii_uppercase(),
            Self::Camel => lower_first(ident),
            Self::Snake => snake_from_pascal(ident),
            Self::ScreamingSnake => snake_from_pascal(ident).to_ascii_uppercase(),
            Self::Kebab => snake_from_pascal(ident).replace('_', "-"),
            Self::ScreamingKebab => snake_from_pascal(ident)
                .to_ascii_uppercase()
                .replace('_', "-"),
        }
    }
}

fn pascal_from_snake(ident: &str) -> String {
    let mut out = String::with_capacity(ident.len());
    let mut upper_next = true;
    for c in ident.chars() {
        if c == '_' {
            upper_next = true;
        } else if upper_next {
            out.push(c.to_ascii_uppercase());
            upper_next = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn snake_from_pascal(ident: &str) -> String {
    let mut out = String::with_capacity(ident.len() + 4);
    for (i, c) in ident.char_indices() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::Case;

    const FIELD: &str = "server_port";
    const VARIANT: &str = "ServerPort";

    fn check(name: &str, field: &str, variant: &str) {
        let case = Case::parse(name).expect(name);
        assert_eq!(case.apply_field(FIELD), field, "field {name}");
        assert_eq!(case.apply_variant(VARIANT), variant, "variant {name}");
    }

    #[test]
    fn lowercase() {
        check("lowercase", "server_port", "serverport");
    }

    #[test]
    fn uppercase() {
        check("UPPERCASE", "SERVER_PORT", "SERVERPORT");
    }

    #[test]
    fn snake() {
        check("snake_case", "server_port", "server_port");
    }

    #[test]
    fn kebab() {
        check("kebab-case", "server-port", "server-port");
    }

    #[test]
    fn camel() {
        check("camelCase", "serverPort", "serverPort");
    }

    #[test]
    fn pascal() {
        check("PascalCase", "ServerPort", "ServerPort");
    }

    #[test]
    fn screaming_snake() {
        check("SCREAMING_SNAKE_CASE", "SERVER_PORT", "SERVER_PORT");
    }

    #[test]
    fn screaming_kebab() {
        check("SCREAMING-KEBAB-CASE", "SERVER-PORT", "SERVER-PORT");
    }

    #[test]
    fn acronym_variant_splits_per_capital() {
        assert_eq!(Case::Snake.apply_variant("HTTPServer"), "h_t_t_p_server");
        assert_eq!(Case::Pascal.apply_field("http_server"), "HttpServer");
    }

    #[test]
    fn unknown_style() {
        assert_eq!(Case::parse("Title Case"), None);
        for name in Case::NAMES {
            assert!(Case::parse(name).is_some(), "{name}");
        }
    }
}
