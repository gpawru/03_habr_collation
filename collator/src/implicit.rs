// веса получаются следующим образом: [.AAAA.0020.0002][.BBBB.0000.0000]
// запись весов: 1111 1111  1111 1111    2222 2222  2333 33v_

/// т.к. L2, L3 у первого кодпоинта всегда одинаковы - вычислим их заранее
const FIRST_WEIGHT_L2_L3: u32 = (0x20 << 16) | (0x02 << 25);

/// рассчитать вычисляемые веса и добавить их к результату (представленному в виде u32)
#[inline(never)]
pub fn implicit_weights(code: u32) -> [u32; 2]
{
    // сначала проверим хань

    if is_han_core(code) {
        return [
            (0xFB40 + (code >> 15)) | FIRST_WEIGHT_L2_L3,
            (code & 0x7FFF) | 0x8000,
        ];
    }

    if is_han_other(code) {
        return [
            (0xFB80 + (code >> 15)) | FIRST_WEIGHT_L2_L3,
            (code & 0x7FFF) | 0x8000,
        ];
    }

    // вымершие языки. вообще, можно смело выкинуть этот блок для оптимизации

    if is_tangut(code) {
        return [0xFB00 | FIRST_WEIGHT_L2_L3, (code - 0x17000) | 0x8000];
    }

    if is_nushu(code) {
        return [0xFB01 | FIRST_WEIGHT_L2_L3, (code - 0x1B170) | 0x8000];
    }

    if is_khitan(code) {
        return [0xFB02 | FIRST_WEIGHT_L2_L3, (code - 0x18B00) | 0x8000];
    }

    // любой другой кодпоинт
    [
        (0xFBC0 + (code >> 15)) | FIRST_WEIGHT_L2_L3,
        (code & 0x7FFF) | 0x8000,
    ]
}

/// Основные иероглифы унификации Хань
/// https://en.wikipedia.org/wiki/CJK_Unified_Ideographs
///
/// Базовый блок иероглифов унификации Хань
///     U+4E00 ..= U+9FFF
///
/// Находятся в блоке совместимости (12 иероглифов):
///     U+FA0E, U+FA0F, U+FA11, U+FA13, U+FA14, U+FA1F, U+FA21, U+FA23, U+FA24, U+FA27, U+FA28, U+FA29
/// эти веса записаны в allkeys (!)
///
/// TR10: Unified_Ideograph=True AND ((Block=CJK_Unified_Ideograph) OR (Block=CJK_Compatibility_Ideographs))
pub fn is_han_core(code: u32) -> bool
{
    (0x4E00 ..= 0x9FFF).contains(&code)
}

/// Иероглифы унификации Хань (прочие)
///
///     U+3400 ..= U+4DBF
///     U+20000 ..= U+2A6DF
///     U+2A700 ..= U+2B739
///     U+2B740 ..= U+2B81D
///     U+2B820 ..= U+2CEA1
///     U+2CEB0 ..= U+2EBE0
///     U+2EBF0 ..= U+2EE5D
///     U+30000 ..= U+3134A
///     U+31350 ..= U+323AF
///
/// TR10: Unified_Ideograph=True AND NOT ((Block=CJK_Unified_Ideograph) OR (Block=CJK_Compatibility_Ideographs))
pub fn is_han_other(code: u32) -> bool
{
    [
        (0x3400 ..= 0x4DBF),
        (0x20000 ..= 0x2A6DF),
        (0x2A700 ..= 0x2B739),
        (0x2B740 ..= 0x2B81D),
        (0x2B820 ..= 0x2CEA1),
        (0x2CEB0 ..= 0x2EBE0),
        (0x2EBF0 ..= 0x2EE5D),
        (0x30000 ..= 0x3134A),
        (0x31350 ..= 0x323AF),
    ]
    .iter()
    .any(|range| range.contains(&code))
}

/// Тангутское письмо (вымершая письменность)
/// https://ru.wikipedia.org/wiki/Тангутское_письмо
///
/// Tangut:
///     назначенные: U+17000 ..= U+187F7
///     блок: U+17000 ..= U+187FF
/// Tangut Components Range:
///     U+18800 ..= U+18AFF
/// Tangut Supplement:
///     назначенные: U+18D00 ..= U+18D08,
///     блок: U+18D00 ..= U+18D7F
///
/// TR10: Assigned code points in Block=Tangut OR Block=Tangut_Components OR Block=Tangut_Supplement
pub fn is_tangut(code: u32) -> bool
{
    [
        (0x17000 ..= 0x187F7),
        (0x18800 ..= 0x18AFF),
        (0x18D00 ..= 0x18D08),
    ]
    .iter()
    .any(|range| range.contains(&code))
}

/// Нюй-шу (не используется, представляет исключительно исследовательский интерес)
/// https://ru.wikipedia.org/wiki/Нюй-шу
///
/// диапазон Nushu:
///     назначенные: U+1B170 ..= U+1B2FB
///     блок: U+1B170 ..= U+1B2FF
///
/// TR10: Assigned code points in Block=Nushu
pub fn is_nushu(code: u32) -> bool
{
    (0x1B170 ..= 0x1B2FB).contains(&code)
}

/// Киданьская письменность (вымершая)
/// https://ru.wikipedia.org/wiki/Киданьская_письменность
///
/// Khitan Small Script:
///     назначенные: U+18B00 ..= U+18CD5
///     блок: U+18B00 ..= U+18CFF
///
/// TR10: Assigned code points in Block=Khitan_Small_Script
pub fn is_khitan(code: u32) -> bool
{
    (0x18B00 ..= 0x18CD5).contains(&code)
}
