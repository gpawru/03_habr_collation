use collation_element::{CollationElement, CollationElementValue};
use data::WeightsData;
use implicit::push_implicit_weights;
use slice::{
    aligned::Aligned,
    iter::CharsIter,
    trie::{TrieIter, TrieNode},
};

mod collation_element;
mod data;
mod implicit;
mod slice;
mod utf8;
mod weights;

/// блок последнего кодпоинта с весами / декомпозицией (U+2FA1D)
pub const LAST_CODEPOINT_BLOCK: u16 = (0x2FA1D >> (18 - 11)) as u16;
/// блок U+E0000 .. U+E0200 сдвинут, чтобы уменьшить размер индекса
pub const IGNORABLES_BLOCKS: core::ops::Range<u16> = 0x1C00 .. 0x1C04;
pub const IGNORABLES_SHIFT: u16 = 0x160B;

pub const MARKER_MASK: u8 = 0b_111;
pub const MARKER_CCC_HANGUL: u8 = 0xFE;
pub const MARKER_CCC_SEQUENCE: u8 = 0xFF;

/// веса считаются алгоритмически
pub const MARKER_IMPLICIT: u8 = 0b_000;

/// обычный стартер, одинарные веса
pub const MARKER_STARTER_SINGLE_WEIGHTS: u8 = 0b_001;
/// стартер, расширение
pub const MARKER_STARTER_EXPANSION: u8 = 0b_010;
/// декомпозиция, начинается со стартера или начало последовательности (сокращение или many-to-many)
pub const MARKER_STARTER_DECOMPOSITION_OR_TRIE: u8 = 0b_011;

/// обычный нестартер, одинарные веса
pub const MARKER_NONSTARTER_SINGLE_WEIGHTS: u8 = 0b_100;
/// нестартер - расширение, сокращение или декомпозиция
pub const MARKER_NONSTARTER_TRIE: u8 = 0b_101;

/// коллатор
#[repr(C, align(16))]
pub struct Collator<'a>
{
    /// стартеры и нестартеры с одинарными весами
    scalars64: Aligned<'a, u64>,
    /// декомпозиции, сокращения и т.д.
    scalars32: Aligned<'a, u32>,
    /// индексы
    index: Aligned<'a, u16>,
    /// расширения
    pub expansions: Aligned<'a, u32>,
    /// сокращения, many-to-many, декомпозиции
    pub tries: Aligned<'a, u32>,
    /// с U+0000 и до этого кодпоинта включительно блоки в data идут последовательно
    continuous_block_end: u32,
}

impl<'a> Collator<'a>
{
    /// создать ключ сопоставления
    /// TODO: добавить опции
    #[inline(never)]
    pub fn get_key(&self, input: &str, _options: bool) -> Vec<u16>
    {
        let result = self.get_weights(input);

        // сформируем ключ
        output_weights(&result)
    }

    /// ключ как вектор весов
    #[inline(always)]
    pub fn get_weights(&self, input: &str) -> Vec<u32>
    {
        let mut iter = CharsIter::new(input);
        let mut result = Vec::<u32>::with_capacity(input.len());

        self.ce_buffer_loop(&mut iter, &mut result);

        result
    }

    /// быстрый цикл - только стартеры (сразу пишем результат без использования буфера)
    #[inline(always)]
    fn starters_loop(
        &self,
        iter: &mut CharsIter,
        result: &mut Vec<u32>,
        buffer_not_empty: bool,
    ) -> Option<(u32, u64)>
    {
        loop {
            let code = unsafe { utf8::next_char(iter)? };
            let data_value = self.get_data_value(code);

            if buffer_not_empty {
                return Some((code, data_value));
            }

            let marker = data_value as u8 & MARKER_MASK;

            match marker {
                // стартеры, синглтоны
                MARKER_STARTER_SINGLE_WEIGHTS => {
                    result.push((data_value >> 4) as u32);
                }
                // расширения стартеров
                MARKER_STARTER_EXPANSION => {
                    result.extend_from_slice(
                        self.get_starter_expansion_weights_slice(data_value as u32),
                    );
                }
                // прочие кейсы
                _ => return Some((code, data_value)),
            }
        }
    }

    /// цикл с использованием буфера кодпоинтов (не делаем декомпозицию, когда она не нужна)
    #[inline(always)]
    fn ce_buffer_loop(&'a self, iter: &mut CharsIter, result: &mut Vec<u32>)
    {
        let buffer = &mut Vec::<CollationElement>::new();
        let mut last_ccc = 0;
        let mut previous = None;

        loop {
            let (code, data_value) = match previous {
                Some(values) => {
                    previous = None;
                    values
                }
                None => match self.starters_loop(iter, result, !buffer.is_empty()) {
                    Some(e) => e,
                    None => {
                        if !buffer.is_empty() {
                            self.handle_buffer(result, buffer, last_ccc != 0xFF);
                        }
                        return;
                    }
                },
            };

            let marker = data_value as u8 & MARKER_MASK;

            match marker {
                // стартеры, синглтоны
                MARKER_STARTER_SINGLE_WEIGHTS => {
                    self.handle_buffer(result, buffer, last_ccc != 0xFF);
                    result.push((data_value >> 4) as u32);

                    last_ccc = 0;
                }
                // расширения стартеров
                MARKER_STARTER_EXPANSION => {
                    // сокращение из двух стартеров (и, возможно, нестартера)?
                    self.handle_buffer(result, buffer, last_ccc != 0xFF);
                    result.extend_from_slice(
                        self.get_starter_expansion_weights_slice(data_value as u32),
                    );

                    last_ccc = 0;
                }
                // декомпозиция, начинается со стартера или начало последовательности (сокращение или many-to-many)
                MARKER_STARTER_DECOMPOSITION_OR_TRIE => {
                    if !buffer.is_empty() {
                        self.handle_buffer(result, buffer, last_ccc != 0xFF);
                    }

                    let (pos, ccc) = parse_expansion_or_trie_info(data_value as u32);
                    last_ccc = ccc;

                    // маркер начала последовательности - 0xFF вместо CCC
                    if last_ccc == MARKER_CCC_SEQUENCE {
                        let node = TrieNode::from_slice(&self.tries, pos);
                        previous = self.handle_starters_sequence(node, result, buffer, iter);

                        continue;
                    }

                    // маркер слога хангыль является 0xFE вместо CCC
                    if last_ccc == MARKER_CCC_HANGUL {
                        // last_ccc = 0;
                        println!("HANGUL!");
                        todo!();
                    }

                    buffer.push(CollationElement {
                        ccc: 0,
                        code,
                        value: CollationElementValue::Decomposition(pos),
                    });
                }
                // обычный нестартер, одинарные веса
                MARKER_NONSTARTER_SINGLE_WEIGHTS => {
                    let ccc = (data_value >> 36) as u8;

                    // декомпозицию делать всё-таки придётся
                    last_ccc = match ccc < last_ccc {
                        true => 0xFF,
                        false => ccc,
                    };

                    let weights = (data_value >> 4) as u32;

                    buffer.push(CollationElement {
                        ccc,
                        code,
                        value: CollationElementValue::SingleWeights(weights),
                    });
                }
                // нестартер - расширение, сокращение или декомпозиция
                MARKER_NONSTARTER_TRIE => {
                    last_ccc = 0xFF;

                    let (pos, _) = parse_expansion_or_trie_info(data_value as u32);
                    let mut trie_iter = TrieIter::new(&self.tries, pos as usize);

                    while let Some(node) = trie_iter.next() {
                        // P.S. единственный trie с потомками, который может попасть сюда - U+0F71 - TIBETAN VOWEL SIGN AA
                        buffer.push(CollationElement {
                            ccc: node.ccc(),
                            code: node.code(),
                            value: CollationElementValue::Trie(node.pos()),
                        });
                    }
                }
                // вычисляемые веса
                MARKER_IMPLICIT => {
                    if !buffer.is_empty() {
                        self.handle_buffer(result, buffer, last_ccc != 0xFF);
                    }
                    last_ccc = 0;

                    push_implicit_weights(code, result);
                }
                _ => unreachable!(),
            }
        }
    }

    /// запись о кодпоинте
    #[inline(always)]
    fn get_data_value(&self, code: u32) -> u64
    {
        let data_block_base = match code <= self.continuous_block_end {
            true => 0x600 | (((code >> 3) as u16) & !0xF),
            false => {
                let mut group_index = (code >> 7) as u16;

                // все кодпоинты, следующие за U+2FA1D имеют вычисляемые веса, не имеют декомпозиции
                // кроме диапазона U+E0000 .. U+E01EF
                if group_index > LAST_CODEPOINT_BLOCK {
                    if IGNORABLES_BLOCKS.contains(&group_index) {
                        group_index -= IGNORABLES_SHIFT;
                    } else {
                        return 0;
                    }
                };

                self.index[group_index as usize]
            }
        };

        let code_offsets = (code as u16) & 0x7F;
        let data_block_index = data_block_base | (code_offsets >> 3) as u16;

        let index = self.index[data_block_index as usize];
        let data_index = ((index >> 1) | code_offsets & 0x7) as usize;

        match index & 1 != 0 {
            true => self.scalars64[data_index],
            false => self.scalars32[data_index] as u64,
        }
    }

    /// записать веса из из буффера CE
    #[inline(always)]
    fn handle_buffer(
        &self,
        result: &mut Vec<u32>,
        buffer: &mut Vec<CollationElement>,
        simple_case: bool,
    )
    {
        // декомпозиция не требуется: отдельно идущий кодпоинт с декомпозицией или кодпоинт с декомпозицией + нестартеры
        if simple_case {
            // почему только Decomposition и SingleWeights: Trie записываются только с установлением флага обязательной
            // декомпозиции, TrieWeights пишутся в буфер только в полной декомпозиции
            for ce in buffer.iter() {
                match ce.value {
                    CollationElementValue::Decomposition(pos) => {
                        let weights_start = pos as usize + 1;
                        let weights_end = weights_start + (self.tries[pos as usize] >> 26) as usize;

                        self.tries[weights_start .. weights_end]
                            .iter()
                            .for_each(|&w| result.push(w));
                    }
                    CollationElementValue::SingleWeights(weights) => result.push(weights),
                    _ => unreachable!(),
                }
            }

            buffer.clear();

            return;
        }

        // требуется декомпозиция и/или проверка сокращений/many-to-many

        // один элемент в буфере - либо начало комбинаций, либо частный случай нестартеров
        // почему не попадают:
        //  - Decomposition: обработан в simple_case, т.к. имеем только один элемент
        //  - TrieWeights: эти записи весов получаем при декомпозиции
        if buffer.len() == 1 {
            match buffer[0].value {
                CollationElementValue::Trie(pos) => {
                    let node = TrieNode::from_slice(&self.tries, pos);
                    self.write_node_weights(node, result);
                }
                CollationElementValue::SingleWeights(weights) => {
                    result.push(weights);
                }
                _ => unreachable!(),
            }

            buffer.clear();
            return;
        }

        // делаем декомпозицию
        if buffer[0].is_starter() {
            // если в начале буфера - стартер, то он может оказаться только стартером с декомпозицией
            // CollationElementValue::Decomposition включает в себя только случаи декомпозиции на 1 стартер + 1-3 нестартера

            let starter = self.decompose(buffer);

            // если стартер может быть скомбинирован с нестартерами - сделаем это
            if starter.has_children() {
                self.handle_trie_nonstarters_sequence(starter, result, buffer);
                return;
            }

            self.write_node_weights(starter, result);
        } else {
            buffer.sort_by_key(|ce| ce.ccc);
        }

        self.write_buffer(buffer, result);
        buffer.clear();
    }

    /// пробуем искать последовательность (сокращение или many-to-many) с идущими следом стартерами
    #[inline(always)]
    fn handle_starters_sequence(
        &self,
        node: TrieNode,
        result: &mut Vec<u32>,
        buffer: &mut Vec<CollationElement>,
        chars_iter: &mut CharsIter,
    ) -> Option<(u32, u64)>
    {
        let mut node = node;
        let mut trie_iter = TrieIter::new(&self.tries, node.next_offset() as usize);

        // если кодпоинт может быть объединён в последовательность только с нестартерами - отправляем его в буфер
        if !trie_iter.current_node().is_starter() {
            buffer.push(CollationElement {
                ccc: 0,
                code: node.code(),
                value: CollationElementValue::Trie(node.pos()),
            });
            return None;
        }

        // получаем следующий элемент, если конец строки - дописываем результат
        let (mut code, mut data_value, mut marker) =
            self.get_next_or_write_to_result(node, result, chars_iter)?;

        // встретили обычный стартер
        if is_starter_marker(marker) {
            loop {
                let iter_node = match trie_iter.next() {
                    Some(iter_node) => iter_node,
                    None => {
                        // второй кодпоинт последовательности мог быть только стартером, но мы проверили все варианты
                        self.write_node_weights(node, result);
                        self.write_starter(data_value, result);
                        return None;
                    }
                };

                // нашли искомый стартер
                if iter_node.code() == code {
                    // потомков нет - значит, записываем последовательность
                    if !(iter_node.has_children()) {
                        self.write_node_weights(iter_node, result);
                        return None;
                    }

                    node = iter_node;

                    (code, data_value, marker) =
                        self.get_next_or_write_to_result(node, result, chars_iter)?;

                    // получили кодпоинт, который не является стартером - элементом последовательности
                    if !is_starter_marker(marker) {
                        buffer.push(CollationElement {
                            ccc: 0,
                            code: 0xFFFF,
                            value: CollationElementValue::Trie(node.pos()),
                        });

                        return Some((code, data_value));
                    }

                    // получили стартер - продолжаем цикл с новыми вводными
                    trie_iter = TrieIter::new(&self.tries, node.next_offset() as usize);
                }

                // проверяемый стартер отсутствует среди возможных комбинаций
                // пишем оба CE в результат
                if iter_node.ccc() != 0 {
                    self.write_node_weights(node, result);
                    self.write_starter(data_value, result);
                    return None;
                }
            }
        }

        // кодпоинт не является стартером - элементом последовательности
        // - пишем в буфер узел trie
        // - кодпоинт отдаём обратно в цикл обработки
        buffer.push(CollationElement {
            ccc: 0,
            code,
            value: CollationElementValue::Trie(node.pos()),
        });

        Some((code, data_value))
    }

    /// ищем последовательность (сокращение или many-to-many) у стартера (или нестартера) и нестартеров (отсортированных по CCC)
    #[inline(always)]
    fn handle_trie_nonstarters_sequence(
        &self,
        node: TrieNode,
        result: &mut Vec<u32>,
        buffer: &mut Vec<CollationElement>,
    )
    {
        let mut node = node;
        let mut trie_iter = TrieIter::new(&self.tries, node.next_offset() as usize);
        let mut index = 0;

        // получаем первый кодпоинт из буфера
        let mut ce = match index < buffer.len() {
            true => buffer[index],
            false => {
                self.write_node_weights(node, result);
                return;
            }
        };

        'outer: loop {
            loop {
                // получаем следующий вариант комбинации в trie
                let iter_node = match trie_iter.next() {
                    Some(iter_node) => iter_node,
                    None => break 'outer,
                };

                let trie_ccc = iter_node.ccc();

                // CCC варианта в trie > CCC рассматриваемого кодпоинта из буфера
                if trie_ccc > ce.ccc {
                    loop {
                        index += 1;

                        ce = match index < buffer.len() {
                            true => buffer[index],
                            false => break 'outer,
                        };

                        if ce.ccc >= trie_ccc {
                            break;
                        }
                    }
                }

                // если совпадает CCC - выходим из цикл промотки
                if trie_ccc == ce.ccc {
                    break;
                }
            }

            // совпадает CCC, сравниваем кодпоинты
            let code = ce.code;
            let trie_code = trie_iter.current_node().code();

            if code == trie_code {
                buffer.remove(index);
                node = trie_iter.current_node();

                if !node.has_children() {
                    break 'outer;
                }

                trie_iter = TrieIter::new(&self.tries, node.next_offset() as usize);

                ce = match index < buffer.len() {
                    true => buffer[index],
                    false => break 'outer,
                };
            }
        }

        self.write_node_weights(node, result);
        self.write_buffer(buffer, result);

        buffer.clear();
    }

    /// получить следующий кодпоинт. если его нет - дописываем веса текущего кодпоинта в результат
    #[inline(always)]
    fn get_next_or_write_to_result(
        &self,
        node: TrieNode,
        result: &mut Vec<u32>,
        iter: &mut CharsIter,
    ) -> Option<(u32, u64, u8)>
    {
        let code = unsafe { utf8::next_char(iter) };

        match code {
            Some(code) => {
                let data_value = self.get_data_value(code);
                let marker = data_value as u8 & MARKER_MASK;

                Some((code, data_value, marker))
            }
            None => {
                self.write_node_weights(node, result);
                return None;
            }
        }
    }

    /// дописать веса из узла trie
    #[inline(always)]
    fn write_node_weights(&self, node: TrieNode, result: &mut Vec<u32>)
    {
        result.extend_from_slice(
            &self.tries[node.weights_offset() as usize .. node.next_offset() as usize],
        );
    }

    /// записать веса стартера в результат
    #[inline(always)]
    fn write_starter(&self, data_value: u64, result: &mut Vec<u32>)
    {
        let marker = data_value as u8 & MARKER_MASK;

        // стартеры, синглтоны
        if marker == MARKER_STARTER_SINGLE_WEIGHTS {
            result.push((data_value >> 4) as u32);
        } else {
            // расширения стартеров (MARKER_STARTER_EXPANSION)
            result.extend_from_slice(self.get_starter_expansion_weights_slice(data_value as u32));
        }
    }

    /// записать веса из буффера
    #[inline(always)]
    fn write_buffer(&self, buffer: &mut Vec<CollationElement>, result: &mut Vec<u32>)
    {
        let mut buffer_iter = buffer.iter();

        while let Some(ce) = buffer_iter.next() {
            match ce.value {
                CollationElementValue::SingleWeights(weights) => result.push(weights),
                CollationElementValue::TrieWeights(pos, len) => {
                    result.extend_from_slice(
                        &self.tries[pos as usize .. pos as usize + len as usize],
                    );
                }
                CollationElementValue::Trie(pos) => {
                    let node = TrieNode::from_slice(&self.tries, pos);
                    let mut buffer = buffer_iter.as_slice().to_owned();

                    if node.has_children() {
                        self.handle_trie_nonstarters_sequence(node, result, &mut buffer);
                        break;
                    }

                    self.write_node_weights(node, result);
                }
                _ => unreachable!(),
            }
        }
    }

    /// слайс весов расширения стартера
    #[inline(always)]
    fn get_starter_expansion_weights_slice(&self, data_value: u32) -> &[u32]
    {
        let (pos, len) = parse_expansion_or_trie_info(data_value);

        &self.expansions[pos as usize .. pos as usize + len as usize]
    }

    /// декомпозиция буфера, возвращает стартер, буфер - нестартеры, отсортированные по CCC
    #[inline(always)]
    fn decompose(&self, buffer: &mut Vec<CollationElement>) -> TrieNode
    {
        let pos = match buffer[0].value {
            CollationElementValue::Decomposition(pos) => {
                TrieNode::from_slice(&self.tries, pos).next_offset()
            }
            CollationElementValue::Trie(pos) => {
                buffer.remove(0);
                buffer.sort_by_key(|ce| ce.ccc);

                return TrieNode::from_slice(&self.tries, pos);
            }
            _ => unreachable!(),
        };

        let mut trie_iter = TrieIter::new(&self.tries, pos as usize);

        let starter = trie_iter.next().unwrap();

        // стартер + нестартер - наиболее встречаемая комбинация, поэтому использование
        // дополнительного буфера для нестартеров декомпозиции может быть избыточным

        buffer[0] = CollationElement::from(trie_iter.next().unwrap());

        while let Some(nonstarter) = trie_iter.next() {
            buffer.insert(1, CollationElement::from(nonstarter));
        }

        buffer.sort_by_key(|ce| ce.ccc);

        starter
    }

    /// создать коллатор из заранее подготовленных данных
    #[inline(never)]
    pub fn from_baked(weights_data: WeightsData) -> Self
    {
        Self {
            scalars64: Aligned::from(weights_data.scalars64),
            scalars32: Aligned::from(weights_data.scalars32),
            index: Aligned::from(weights_data.index),
            expansions: Aligned::from(weights_data.expansions),
            tries: Aligned::from(weights_data.tries),
            continuous_block_end: weights_data.continuous_block_end,
        }
    }

    /// CLDR undefined
    pub fn cldr_und() -> Self
    {
        Self::from_baked(data::cldr_und())
    }
}

/// если расширение: позиция + кол-во весов, если trie - позиция и CCC
#[inline(always)]
fn parse_expansion_or_trie_info(data_value: u32) -> (u16, u8)
{
    let pos = (data_value >> 4) as u16;
    let len_or_ccc = (data_value >> 20) as u8;

    (pos, len_or_ccc)
}

/// маркер стартера с весами
#[inline(always)]
fn is_starter_marker(marker: u8) -> bool
{
    marker == MARKER_STARTER_SINGLE_WEIGHTS || marker == MARKER_STARTER_EXPANSION
}

/// записать результат как последовательность u16
#[inline(always)]
fn output_weights(from: &Vec<u32>) -> Vec<u16>
{
    let mut primary = vec![];
    let mut secondary = vec![];
    let mut tetriary = vec![];

    // веса L1, L2, L3 / маркер переменного веса как u32
    // 1111 1111  1111 1111    2222 2222  2333 33v_

    for &weights in from.iter() {
        let l1 = weights as u16;
        let l2 = (weights >> 16) as u16 & 0x1FF;
        let l3 = (weights >> 25) as u16 & 0x1F;

        if l1 != 0 {
            primary.push(l1);
        };

        if l2 != 0 {
            secondary.push(l2);
        }

        if l3 != 0 {
            tetriary.push(l3);
        }
    }

    primary.push(0);
    primary.extend(secondary);
    primary.push(0);
    primary.extend(tetriary);

    primary
}
