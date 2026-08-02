//! 手机号校验工具

/// 校验中国大陆手机号
///
/// 规则：11 位、以 1 开头、第二位为 3-9、其余 9 位为数字。
/// 该规则基于工信部规范，覆盖现有全部号段（13x-19x，含 192/195/196/197/198/199 等新号段）
/// 及未来 1[3-9] 范围内的扩展号段。
///
/// 等价正则：`^1[3-9]\d{9}$`
pub fn is_valid_phone(phone: &str) -> bool {
    let bytes = phone.as_bytes();
    bytes.len() == 11
        && bytes[0] == b'1'
        && (b'3'..=b'9').contains(&bytes[1])
        && bytes[2..].iter().all(|b| b.is_ascii_digit())
}
