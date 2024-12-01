pub fn is_text(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }

    // Check for null bytes and non-text characters
    let text_chars = data.iter().take(512).filter(|&&b| {
        b != 0 && (b >= 32 || b == b'\n' || b == b'\r' || b == b'\t')
    }).count();

    // Consider it text if >90% of first 512 bytes are text characters
    (text_chars as f32 / data.len().min(512) as f32) > 0.9
}