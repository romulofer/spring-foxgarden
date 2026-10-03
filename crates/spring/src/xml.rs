//! The one XML helper every report parser here needs.

use quick_xml::events::BytesStart;

/// One attribute's unescaped value, or `None` when the element doesn't carry
/// it.
pub(crate) fn attr(start: &BytesStart<'_>, name: &[u8]) -> Result<Option<String>, String> {
    for a in start.attributes() {
        let a = a.map_err(|e| e.to_string())?;
        if a.key.as_ref() == name {
            return a
                .unescape_value()
                .map(|v| Some(v.into_owned()))
                .map_err(|e| e.to_string());
        }
    }
    Ok(None)
}
