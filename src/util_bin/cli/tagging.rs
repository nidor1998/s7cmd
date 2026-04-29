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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_yields_empty_vec() {
        let tags = parse_tagging_to_tags("").unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn single_pair() {
        let tags = parse_tagging_to_tags("k1=v1").unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key(), "k1");
        assert_eq!(tags[0].value(), "v1");
    }

    #[test]
    fn multiple_pairs() {
        let tags = parse_tagging_to_tags("a=1&b=2&c=3").unwrap();
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].key(), "a");
        assert_eq!(tags[0].value(), "1");
        assert_eq!(tags[2].key(), "c");
        assert_eq!(tags[2].value(), "3");
    }

    #[test]
    fn percent_encoded_value_is_decoded() {
        // "hello world" — space encoded as %20
        let tags = parse_tagging_to_tags("greeting=hello%20world").unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].value(), "hello world");
    }

    #[test]
    fn percent_encoded_key_is_decoded() {
        let tags = parse_tagging_to_tags("hello%20world=v").unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key(), "hello world");
    }

    #[test]
    fn key_without_value_yields_empty_value() {
        let tags = parse_tagging_to_tags("k").unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key(), "k");
        assert_eq!(tags[0].value(), "");
    }

    #[test]
    fn equals_in_value_is_kept() {
        // splitn(2, '=') means only the first '=' is the separator;
        // subsequent '=' are part of the value.
        let tags = parse_tagging_to_tags("k=a=b=c").unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key(), "k");
        assert_eq!(tags[0].value(), "a=b=c");
    }
}
