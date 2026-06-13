use crate::RedactionMeta;

/// Render a redacted placeholder from redaction metadata.
///
/// Format: `<REDACTED:type=<class>, len=<bucket>>`
///
/// Per ADR-028 D3: type/class + bucketed length ONLY.
/// No content-derived hash. No exact byte length.
pub fn render_placeholder(meta: &RedactionMeta) -> String {
    format!(
        "<REDACTED:type={}, len={}>",
        meta.class, meta.original_len_bucket
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(class: &str, bucket: u64) -> RedactionMeta {
        RedactionMeta {
            class: class.to_owned(),
            original_len_bucket: bucket,
        }
    }

    #[test]
    fn placeholder_pii_class() {
        assert_eq!(
            render_placeholder(&meta("pii", 128)),
            "<REDACTED:type=pii, len=128>"
        );
    }

    #[test]
    fn placeholder_secret_class() {
        assert_eq!(
            render_placeholder(&meta("secret", 256)),
            "<REDACTED:type=secret, len=256>"
        );
    }

    #[test]
    fn placeholder_zero_length_bucket() {
        assert_eq!(
            render_placeholder(&meta("content", 0)),
            "<REDACTED:type=content, len=0>"
        );
    }

    #[test]
    fn placeholder_large_bucket() {
        assert_eq!(
            render_placeholder(&meta("blob", 4096)),
            "<REDACTED:type=blob, len=4096>"
        );
    }

    #[test]
    fn placeholder_deterministic() {
        let m = meta("pii", 64);
        let a = render_placeholder(&m);
        let b = render_placeholder(&m);
        assert_eq!(a, b, "placeholder must be deterministic");
    }

    #[test]
    fn placeholder_format_stable() {
        // Verify exact format matches ADR-028 D3 spec
        let result = render_placeholder(&meta("credential", 512));
        assert!(result.starts_with("<REDACTED:"));
        assert!(result.ends_with('>'));
        assert!(result.contains("type=credential"));
        assert!(result.contains("len=512"));
    }
}
