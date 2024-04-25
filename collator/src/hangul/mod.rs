/// начало блока слогов хангыль
const HANGUL_S_BASE: u32 = 0xAC00;
/// количество гласных * количество завершающих согласных
const HANGUL_N_COUNT: u32 = 588;
/// количество завершающих согласных
const HANGUL_T_COUNT: u32 = 27;
/// количество кодпоинтов на блок LV
const HANGUL_T_BLOCK_SIZE: u32 = HANGUL_T_COUNT + 1;

/*
    в базовом CLDR веса L1 у чамо хангыль идут последовательно, и соответствуют сортировке T < V < L для
    большинства алгоритмов сопоставления хангыль.

    в текущей реализации L1 располагается в младших 16 битах u32-весов, следовательно, мы можем просто 
    вычислить все веса, не полагаясь на данные из таблицы.

    тем не менее, текущая реализация - скорее "заглушка", т.к., очевидно, не решает проблему trailing weights.
    более детально - см. спецификацию, раздел #10.

    TODO.
*/

const HANGUL_L_BASE_WEIGHTS: u32 = 0x4204323;
const HANGUL_V_BASE_WEIGHTS: u32 = 0x42043A1;
const HANGUL_T_BASE_WEIGHTS: u32 = 0x42043FE;

/// слог хангыль
#[inline(always)]
pub fn write_hangul_syllable(code: u32, result: &mut Vec<u32>)
{
    let lvt = code.wrapping_sub(HANGUL_S_BASE);

    let l = (lvt / HANGUL_N_COUNT) as u8;
    let v = ((lvt % HANGUL_N_COUNT) / HANGUL_T_BLOCK_SIZE) as u8;
    let t = (lvt % HANGUL_T_BLOCK_SIZE) as u8;

    result.push(HANGUL_L_BASE_WEIGHTS + l as u32);
    result.push(HANGUL_V_BASE_WEIGHTS + v as u32);

    if t != 0 {
        result.push(HANGUL_T_BASE_WEIGHTS + t as u32);
    }
}
