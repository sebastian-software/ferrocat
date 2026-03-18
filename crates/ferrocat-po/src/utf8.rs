/// Converts a byte slice borrowed from an existing UTF-8 input string back to `&str`.
#[inline]
pub(crate) fn input_slice_as_str(bytes: &[u8]) -> &str {
    // SAFETY: Callers only pass slices that originate from an already-valid input `&str`
    // and are cut on ASCII token boundaries, so the slice remains valid UTF-8.
    unsafe { std::str::from_utf8_unchecked(bytes) }
}

/// Builds a `String` from bytes that were assembled from known-valid UTF-8 fragments.
#[inline]
pub(crate) fn string_from_utf8(bytes: Vec<u8>) -> String {
    // SAFETY: Callers only append bytes from valid UTF-8 source strings or from UTF-8
    // encodings of decoded scalar values, so the buffer remains valid UTF-8.
    unsafe { String::from_utf8_unchecked(bytes) }
}
