use crate::{ConfigNode, Value};

/// Merge `overlay` into `base` in place.
///
/// A null overlay is a no-op, so an empty file never erases lower-priority
/// sources. Tables merge per key; any other overlay replaces `base`, so arrays
/// never append.
pub(crate) fn merge(base: &mut ConfigNode, overlay: ConfigNode) {
    if overlay.is_null() {
        return;
    }
    match (&mut base.value, overlay.value) {
        (Value::Table(left), Value::Table(right)) => {
            for (key, value) in right {
                match left.get_mut(&key) {
                    Some(existing) => merge(existing, value),
                    None => {
                        left.insert(key, value);
                    },
                }
            }
        },
        (slot, value) => {
            *slot = value;
            base.origin = overlay.origin;
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Table;

    fn scalar_table(entries: &[(&str, &str)]) -> ConfigNode {
        let mut table = Table::new();
        for (key, value) in entries {
            table.insert((*key).to_owned(), ConfigNode::scalar(*value));
        }
        ConfigNode::table(table)
    }

    #[test]
    fn null_overlay_keeps_base() {
        let mut base = scalar_table(&[("port", "8080")]);
        merge(&mut base, ConfigNode::null());
        assert_eq!(base, scalar_table(&[("port", "8080")]));
    }

    #[test]
    fn null_overlay_keeps_scalar_base() {
        let mut base = ConfigNode::scalar("a");
        merge(&mut base, ConfigNode::null());
        assert_eq!(base, ConfigNode::scalar("a"));
    }

    #[test]
    fn arrays_are_replaced() {
        let mut base = ConfigNode::array(vec![ConfigNode::scalar("a"), ConfigNode::scalar("b")]);
        merge(&mut base, ConfigNode::array(vec![ConfigNode::scalar("c")]));
        assert_eq!(base, ConfigNode::array(vec![ConfigNode::scalar("c")]));
    }

    #[test]
    fn scalars_are_replaced() {
        let mut base = ConfigNode::scalar("base");
        merge(&mut base, ConfigNode::scalar("override"));
        assert_eq!(base, ConfigNode::scalar("override"));
    }

    #[test]
    fn tables_merge_per_key() {
        let mut inner_base = Table::new();
        inner_base.insert("host".into(), ConfigNode::scalar("localhost"));
        inner_base.insert("port".into(), ConfigNode::scalar("80"));
        let mut base = Table::new();
        base.insert("server".into(), ConfigNode::table(inner_base));
        base.insert("name".into(), ConfigNode::scalar("app"));
        let mut base = ConfigNode::table(base);

        let mut inner_overlay = Table::new();
        inner_overlay.insert("port".into(), ConfigNode::scalar("9090"));
        inner_overlay.insert("tls".into(), ConfigNode::scalar("on"));
        let mut overlay = Table::new();
        overlay.insert("server".into(), ConfigNode::table(inner_overlay));
        merge(&mut base, ConfigNode::table(overlay));

        let mut expected_inner = Table::new();
        expected_inner.insert("host".into(), ConfigNode::scalar("localhost"));
        expected_inner.insert("port".into(), ConfigNode::scalar("9090"));
        expected_inner.insert("tls".into(), ConfigNode::scalar("on"));
        let mut expected = Table::new();
        expected.insert("server".into(), ConfigNode::table(expected_inner));
        expected.insert("name".into(), ConfigNode::scalar("app"));
        assert_eq!(base, ConfigNode::table(expected));
    }

    #[test]
    fn table_overlay_replaces_scalar_base() {
        let mut base = ConfigNode::scalar("x");
        merge(&mut base, scalar_table(&[("a", "1")]));
        assert_eq!(base, scalar_table(&[("a", "1")]));
    }
}
