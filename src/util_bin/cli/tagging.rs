// Vendored from s3util-rs@0.2.0
//   src/bin/s3util/cli/tagging.rs
// Adjustments: no adjustments

use anyhow::Result;
use aws_sdk_s3::types::Tag;

/// Parse a URL-encoded tagging string (e.g. `"key1=val1&key2=val2"`) into a
/// `Vec<Tag>`. Each `key=value` pair is percent-decoded before being passed to
/// the SDK.
///
/// Shared by `create-bucket` (with `--tagging`), `put-bucket-tagging`, and
/// `put-object-tagging` so the percent-decoding semantics stay consistent
/// across all three commands.
pub fn parse_tagging_to_tags(s: &str) -> Result<Vec<Tag>> {
    if s.is_empty() {
        return Ok(vec![]);
    }
    let mut tags = Vec::new();
    for pair in s.split('&') {
        let mut parts = pair.splitn(2, '=');
        let raw_key = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("invalid tagging pair: {pair}"))?;
        let raw_val = parts.next().unwrap_or("");
        let key = urlencoding::decode(raw_key)
            .map_err(|e| anyhow::anyhow!("invalid percent-encoding in tag key: {e}"))?
            .into_owned();
        let value = urlencoding::decode(raw_val)
            .map_err(|e| anyhow::anyhow!("invalid percent-encoding in tag value: {e}"))?
            .into_owned();
        tags.push(Tag::builder().key(key).value(value).build()?);
    }
    Ok(tags)
}
