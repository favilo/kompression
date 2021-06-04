pub(crate) fn format_bits(data: u64) -> String {
    let mut s = String::new();
    data.to_be_bytes().iter().for_each(|b| {
        s.push_str(&format!("{:08b} ", b));
    });
    s
}
