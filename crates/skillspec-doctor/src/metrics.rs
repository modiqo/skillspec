pub(super) fn estimate_tokens(text: &str) -> usize {
    let by_bytes = text.len().div_ceil(4);
    let by_words = text.split_whitespace().count();
    by_bytes.max(by_words)
}

pub(super) fn percentage(numerator: usize, denominator: usize) -> u8 {
    if denominator == 0 {
        return 0;
    }
    let value = ((numerator as f64 / denominator as f64) * 100.0).round();
    u8::try_from(value as usize).unwrap_or(100).min(100)
}
