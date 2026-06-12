//! Static description of a configuration type, used by help output, env lookup
//! and warnings.

/// What a [`FromNode`](crate::FromNode) type accepts.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Shape {
    /// A text leaf with the type label shown in help: `"string"`, `"u16"`,
    /// `"duration"`, ...
    Scalar(&'static str),
    /// A boolean leaf.
    Bool,
    /// A value that may be absent.
    Optional(fn() -> Shape),
    /// A list of values.
    List(fn() -> Shape),
    /// A string-keyed map of values.
    Map(fn() -> Shape),
    /// A table with declared fields.
    Struct(&'static StructInfo),
    /// A set of named variants.
    Enum(&'static EnumInfo),
    /// Any node, passed through untouched.
    Any,
}

/// Description of a struct type.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct StructInfo {
    name: &'static str,
    fields: &'static [FieldInfo],
    doc: Option<&'static str>,
    env_prefix: Option<&'static str>,
}

impl StructInfo {
    /// Describe struct `name` with `fields`.
    pub const fn new(name: &'static str, fields: &'static [FieldInfo]) -> Self {
        Self {
            name,
            fields,
            doc: None,
            env_prefix: None,
        }
    }

    /// Set the doc text.
    #[must_use]
    pub const fn doc(mut self, doc: &'static str) -> Self {
        self.doc = Some(doc);
        self
    }

    /// Set the environment variable prefix.
    #[must_use]
    pub const fn env_prefix(mut self, prefix: &'static str) -> Self {
        self.env_prefix = Some(prefix);
        self
    }

    /// Type name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Declared fields in declaration order.
    pub const fn fields(&self) -> &'static [FieldInfo] {
        self.fields
    }

    /// Doc text.
    pub const fn docs(&self) -> Option<&'static str> {
        self.doc
    }

    /// Environment variable prefix.
    pub const fn prefix(&self) -> Option<&'static str> {
        self.env_prefix
    }
}

/// Description of one struct field.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FieldInfo {
    key: &'static str,
    shape: fn() -> Shape,
    aliases: &'static [&'static str],
    doc: Option<&'static str>,
    env: Option<&'static str>,
    flag: Option<&'static str>,
    short: Option<char>,
    skip_cli: bool,
    default_text: Option<&'static str>,
    sensitive: bool,
    deprecated: Option<&'static str>,
    flatten: bool,
}

impl FieldInfo {
    /// Describe the field stored under `key`.
    pub const fn new(key: &'static str, shape: fn() -> Shape) -> Self {
        Self {
            key,
            shape,
            aliases: &[],
            doc: None,
            env: None,
            flag: None,
            short: None,
            skip_cli: false,
            default_text: None,
            sensitive: false,
            deprecated: None,
            flatten: false,
        }
    }

    /// Set alternative keys.
    #[must_use]
    pub const fn aliases(mut self, aliases: &'static [&'static str]) -> Self {
        self.aliases = aliases;
        self
    }

    /// Set the doc text.
    #[must_use]
    pub const fn doc(mut self, doc: &'static str) -> Self {
        self.doc = Some(doc);
        self
    }

    /// Set the environment variable name.
    #[must_use]
    pub const fn env(mut self, env: &'static str) -> Self {
        self.env = Some(env);
        self
    }

    /// Set the long command-line flag name.
    #[must_use]
    pub const fn flag(mut self, flag: &'static str) -> Self {
        self.flag = Some(flag);
        self
    }

    /// Set the short command-line flag.
    #[must_use]
    pub const fn short(mut self, short: char) -> Self {
        self.short = Some(short);
        self
    }

    /// Hide the field from the command line.
    #[must_use]
    pub const fn skip_cli(mut self) -> Self {
        self.skip_cli = true;
        self
    }

    /// Set the default value as shown in help.
    #[must_use]
    pub const fn default_text(mut self, text: &'static str) -> Self {
        self.default_text = Some(text);
        self
    }

    /// Mark the value as secret.
    #[must_use]
    pub const fn sensitive(mut self) -> Self {
        self.sensitive = true;
        self
    }

    /// Mark the field as deprecated with `message` (empty for none).
    #[must_use]
    pub const fn deprecated(mut self, message: &'static str) -> Self {
        self.deprecated = Some(message);
        self
    }

    /// Mark the field's own keys as living at the parent level.
    #[must_use]
    pub const fn flatten(mut self) -> Self {
        self.flatten = true;
        self
    }

    /// Key in the configuration tree.
    pub const fn key(&self) -> &'static str {
        self.key
    }

    /// Alternative keys.
    pub const fn alias_keys(&self) -> &'static [&'static str] {
        self.aliases
    }

    /// Doc text.
    pub const fn docs(&self) -> Option<&'static str> {
        self.doc
    }

    /// Environment variable name.
    pub const fn env_var(&self) -> Option<&'static str> {
        self.env
    }

    /// Long command-line flag name.
    pub const fn flag_name(&self) -> Option<&'static str> {
        self.flag
    }

    /// Short command-line flag.
    pub const fn short_flag(&self) -> Option<char> {
        self.short
    }

    /// Whether the field is hidden from the command line.
    pub const fn is_skip_cli(&self) -> bool {
        self.skip_cli
    }

    /// Default value as shown in help.
    pub const fn default_value(&self) -> Option<&'static str> {
        self.default_text
    }

    /// Whether the value is secret.
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }

    /// Deprecation message; empty when none was given.
    pub const fn deprecation(&self) -> Option<&'static str> {
        self.deprecated
    }

    /// Whether the field's keys live at the parent level.
    pub const fn is_flatten(&self) -> bool {
        self.flatten
    }

    /// Shape of the field's value.
    pub fn shape(&self) -> Shape {
        (self.shape)()
    }
}

/// Description of an enum type.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EnumInfo {
    name: &'static str,
    variants: &'static [VariantInfo],
    doc: Option<&'static str>,
    tag: Option<&'static str>,
}

impl EnumInfo {
    /// Describe enum `name` with `variants`.
    pub const fn new(name: &'static str, variants: &'static [VariantInfo]) -> Self {
        Self { name, variants, doc: None, tag: None }
    }

    /// Set the doc text.
    #[must_use]
    pub const fn doc(mut self, doc: &'static str) -> Self {
        self.doc = Some(doc);
        self
    }

    /// Set the key that names the variant inside a table.
    #[must_use]
    pub const fn tag(mut self, tag: &'static str) -> Self {
        self.tag = Some(tag);
        self
    }

    /// Type name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Variants in declaration order.
    pub const fn variants(&self) -> &'static [VariantInfo] {
        self.variants
    }

    /// Doc text.
    pub const fn docs(&self) -> Option<&'static str> {
        self.doc
    }

    /// Key that names the variant inside a table.
    pub const fn tag_key(&self) -> Option<&'static str> {
        self.tag
    }
}

/// Description of one enum variant.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct VariantInfo {
    key: &'static str,
    doc: Option<&'static str>,
    data: Option<fn() -> Shape>,
}

impl VariantInfo {
    /// Describe the unit variant stored under `key`.
    pub const fn new(key: &'static str) -> Self {
        Self { key, doc: None, data: None }
    }

    /// Set the doc text.
    #[must_use]
    pub const fn doc(mut self, doc: &'static str) -> Self {
        self.doc = Some(doc);
        self
    }

    /// Give the variant a payload of the given shape.
    #[must_use]
    pub const fn data(mut self, data: fn() -> Shape) -> Self {
        self.data = Some(data);
        self
    }

    /// Key that selects the variant.
    pub const fn key(&self) -> &'static str {
        self.key
    }

    /// Doc text.
    pub const fn docs(&self) -> Option<&'static str> {
        self.doc
    }

    /// Shape of the payload; `None` for a unit variant.
    pub fn payload(&self) -> Option<Shape> {
        self.data.map(|shape| shape())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn text_shape() -> Shape {
        Shape::Scalar("string")
    }

    static FIELDS: [FieldInfo; 2] = [
        FieldInfo::new("port", text_shape)
            .aliases(&["p"])
            .doc("Listen port")
            .env("APP_PORT")
            .flag("port")
            .short('p')
            .default_text("8080"),
        FieldInfo::new("token", text_shape)
            .skip_cli()
            .sensitive()
            .deprecated("use key")
            .flatten(),
    ];
    static SERVER: StructInfo = StructInfo::new("Server", &FIELDS)
        .doc("Server settings")
        .env_prefix("APP_");
    static VARIANTS: [VariantInfo; 2] =
        [VariantInfo::new("off").doc("disabled"), VariantInfo::new("on").data(text_shape)];
    static MODE: EnumInfo = EnumInfo::new("Mode", &VARIANTS).doc("Run mode").tag("type");

    #[test]
    fn struct_info_getters() {
        assert_eq!(SERVER.name(), "Server");
        assert_eq!(SERVER.docs(), Some("Server settings"));
        assert_eq!(SERVER.prefix(), Some("APP_"));
        assert_eq!(SERVER.fields().len(), 2);
    }

    #[test]
    fn field_info_getters() {
        let port = &SERVER.fields()[0];
        assert_eq!(port.key(), "port");
        assert_eq!(port.alias_keys(), ["p"]);
        assert_eq!(port.docs(), Some("Listen port"));
        assert_eq!(port.env_var(), Some("APP_PORT"));
        assert_eq!(port.flag_name(), Some("port"));
        assert_eq!(port.short_flag(), Some('p'));
        assert_eq!(port.default_value(), Some("8080"));
        assert!(!port.is_skip_cli() && !port.is_sensitive() && !port.is_flatten());
        assert!(matches!(port.shape(), Shape::Scalar("string")));
        let token = &SERVER.fields()[1];
        assert!(token.is_skip_cli() && token.is_sensitive() && token.is_flatten());
        assert_eq!(token.deprecation(), Some("use key"));
        assert_eq!(token.docs(), None);
    }

    #[test]
    fn enum_info_getters() {
        assert_eq!(MODE.name(), "Mode");
        assert_eq!(MODE.docs(), Some("Run mode"));
        assert_eq!(MODE.tag_key(), Some("type"));
        let [off, on] = MODE.variants() else { panic!("two variants") };
        assert_eq!((off.key(), off.docs()), ("off", Some("disabled")));
        assert!(off.payload().is_none());
        assert!(matches!(on.payload(), Some(Shape::Scalar("string"))));
    }
}
