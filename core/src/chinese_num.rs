const CHINESE_NUMBERS: &[&str] = &[
    "零", "一", "二", "三", "四", "五", "六", "七", "八", "九", "十", "甲", "乙", "丙", "丁", "戊",
    "己", "庚", "辛", "壬", "癸",
];

/// Number → text: 0 → "零", 1 → "一", …
pub(crate) fn fmt_num(n: u8) -> &'static str {
    CHINESE_NUMBERS.get(n as usize).copied().unwrap_or("?")
}

/// Text → number: "零" → 0, "一" → 1, …
pub(crate) fn parse_num(s: &str) -> Option<u8> {
    CHINESE_NUMBERS.iter().position(|&x| x == s).map(|i| i as u8)
}
