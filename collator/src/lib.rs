use codepoint::{CodepointWithData, CodepointsIter};
use collation_element::{CollationElement, CollationElementValue};
use data::WeightsData;
use hangul::write_hangul_syllable;
use implicit::implicit_weights;
use slice::aligned::Aligned;
use trie::{TrieIter, TrieNode};
use weights::output_weights;

mod codepoint;
mod collation_element;
mod data;
mod implicit;
mod slice;
mod trie;
mod weights;
mod hangul;

/// веса считаются алгоритмически
pub const MARKER_IMPLICIT: u8 = 0b_000;

/// обычный стартер, одинарные веса
pub const MARKER_STARTER_SINGLE_WEIGHTS: u8 = 0b_001;
/// стартер, расширение
pub const MARKER_STARTER_EXPANSION: u8 = 0b_010;
/// декомпозиция, начинается со стартера
pub const MARKER_STARTER_DECOMPOSITION: u8 = 0b_011;
/// стартер, начало последовательности (сокращение или many-to-many)
pub const MARKER_STARTER_TRIE: u8 = 0b100;

/// обычный нестартер, одинарные веса
pub const MARKER_NONSTARTER_SINGLE_WEIGHTS: u8 = 0b_101;
/// нестартер - расширение, сокращение или декомпозиция
pub const MARKER_NONSTARTER_TRIE: u8 = 0b_110;

/// частный случай декомпозиции - кодпоинт - слог хангыль
pub const MARKER_CCC_HANGUL: u8 = 0xFF;

/// коллатор
#[repr(C, align(16))]
pub struct Collator<'a>
{
    /// расширения
    expansions: Aligned<'a, u32>,
    /// сокращения, many-to-many, декомпозиции
    tries: Aligned<'a, u32>,
    /// стартеры и нестартеры с одинарными весами
    scalars64: Aligned<'a, u64>,
    /// декомпозиции, сокращения и т.д.
    scalars32: Aligned<'a, u32>,
    /// индексы
    index: Aligned<'a, u16>,
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
        let mut codepoints = CodepointsIter::new(
            input,
            &self.scalars64,
            &self.scalars32,
            &self.index,
            self.continuous_block_end,
        );

        let mut result = Vec::<u32>::with_capacity(input.len());
        let mut buffer = Vec::<CollationElement>::new();

        self.ce_buffer_loop(&mut codepoints, &mut result, &mut buffer);

        result
    }

    /// быстрый цикл - только стартеры (сразу пишем результат без использования буфера)
    #[inline(always)]
    fn starters_loop(
        &self,
        codepoints: &mut CodepointsIter,
        result: &mut Vec<u32>,
    ) -> Option<CodepointWithData>
    {
        loop {
            let codepoint = codepoints.next()?;

            match codepoint.marker() {
                // стартеры, синглтоны
                MARKER_STARTER_SINGLE_WEIGHTS => {
                    result.push(codepoint.single_weights());
                }
                // расширения стартеров
                MARKER_STARTER_EXPANSION => {
                    result.extend_from_slice(codepoint.expansion_weights(&self.expansions))
                }
                // прочие кейсы
                _ => return Some(codepoint),
            }
        }
    }

    /// цикл с использованием буфера кодпоинтов (не делаем декомпозицию, когда она не нужна)
    #[inline(always)]
    fn ce_buffer_loop(
        &'a self,
        codepoints: &mut CodepointsIter,
        result: &mut Vec<u32>,
        buffer: &mut Vec<CollationElement>,
    )
    {
        let mut previous_ccc = 0;
        let mut previous = None;

        loop {
            // самый частый случай - последовательно идущие обычные стартеры, для них - цикл без избыточных проверок
            let codepoint = match previous {
                None => match buffer.is_empty() {
                    true => match self.starters_loop(codepoints, result) {
                        Some(codepoint) => codepoint,
                        None => return,
                    },
                    false => match codepoints.next() {
                        Some(codepoint) => codepoint,
                        None => {
                            self.handle_buffer(result, buffer, previous_ccc != 0xFF);
                            return;
                        }
                    },
                },
                Some(codepoint) => {
                    previous = None;
                    codepoint
                }
            };

            match codepoint.marker() {
                // стартеры, синглтоны
                MARKER_STARTER_SINGLE_WEIGHTS => {
                    self.handle_buffer(result, buffer, previous_ccc != 0xFF);

                    result.push(codepoint.single_weights());

                    previous_ccc = 0;
                }
                // расширения стартеров
                MARKER_STARTER_EXPANSION => {
                    self.handle_buffer(result, buffer, previous_ccc != 0xFF);

                    result.extend_from_slice(codepoint.expansion_weights(&self.expansions));

                    previous_ccc = 0;
                }
                // декомпозиция, начинается со стартера
                MARKER_STARTER_DECOMPOSITION => {
                    self.handle_buffer(result, buffer, previous_ccc != 0xFF);

                    previous_ccc = match codepoint.ccc_or_len() {
                        // частный случай - слог хангыль
                        MARKER_CCC_HANGUL => {
                            write_hangul_syllable(codepoint.code, result);
                            0
                        }
                        ccc => {
                            buffer.push(codepoint.as_ce_decomposition());
                            ccc
                        }
                    };
                }
                // стартер, начало последовательности (сокращение или many-to-many)
                MARKER_STARTER_TRIE => {
                    self.handle_buffer(result, buffer, previous_ccc != 0xFF);

                    previous = self.handle_starter_trie(codepoint, result, buffer, codepoints);

                    // если буфер не пуст (содержит узел), то это означает, что возможно
                    // продолжение последовательности с далее идущими нестартерами
                    previous_ccc = match buffer.is_empty() {
                        true => 0,
                        false => 0xFF,
                    };
                }
                // обычный нестартер, одинарные веса
                MARKER_NONSTARTER_SINGLE_WEIGHTS => {
                    let ce = codepoint.as_ce_single_weights();

                    // потребуется декомпозиция - нарушен порядок CCC
                    previous_ccc = match ce.ccc < previous_ccc {
                        true => 0xFF,
                        false => ce.ccc,
                    };

                    buffer.push(ce);
                }
                // нестартер - расширение, сокращение или декомпозиция
                MARKER_NONSTARTER_TRIE => {
                    let mut trie_iter = TrieIter::new(&self.tries, codepoint.data_pos());

                    while let Some(node) = trie_iter.next() {
                        let ccc = node.ccc();

                        // кодпоинт - начало последовательности / обычное расширение
                        // декомпозицию придётся делать, если нарушен порядок CCC или кодпоинт - начало последовательности
                        match node.has_children() {
                            true => {
                                previous_ccc = 0xFF;

                                buffer.push(node.as_ce_trie());
                            }
                            false => {
                                previous_ccc = match ccc < previous_ccc {
                                    true => 0xFF,
                                    false => ccc,
                                };

                                buffer.push(node.as_ce_weights());
                            }
                        };
                    }
                }
                // вычисляемые веса
                MARKER_IMPLICIT => {
                    self.handle_buffer(result, buffer, previous_ccc != 0xFF);

                    result.extend_from_slice(&implicit_weights(codepoint.code));

                    previous_ccc = 0;
                }
                _ => unreachable!(),
            }
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
        if buffer.is_empty() {
            return;
        }

        // декомпозиция не требуется: отдельно идущий кодпоинт с декомпозицией или кодпоинт с декомпозицией + нестартеры
        if simple_case {
            // почему только Decomposition, SingleWeights и TrieWeights:
            // CollationElementValue::Trie записываются только с установлением флага обязательной декомпозиции
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
                    CollationElementValue::TrieWeights(pos, len) => {
                        result.extend_from_slice(
                            &self.tries[pos as usize .. pos as usize + len as usize],
                        );
                    }
                    _ => unreachable!(),
                }
            }

            buffer.clear();
            return;
        }

        // один элемент - никакой декомпозиции
        if buffer.len() == 1 {
            match buffer[0].value {
                CollationElementValue::Trie(pos) => {
                    let node = TrieNode::from(&self.tries, pos);
                    result.extend_from_slice(node.weights(&self.tries));
                }
                _ => unreachable!(),
            }

            buffer.clear();
            return;
        }

        // делаем декомпозицию и(или) сортируем по CCC
        if buffer[0].is_starter() {
            let starter = self.decompose(buffer);

            // стартер может быть скомбинирован с нестартерами?
            if starter.has_children() {
                self.handle_trie_nonstarters_sequence(starter, result, buffer);
                return;
            }

            result.extend_from_slice(starter.weights(&self.tries));
        } else {
            buffer.sort_by_key(|ce| ce.ccc);
        }

        // во время записи будет проверен вариант с последовательностями, начинающихся с нестартера
        self.flush_buffer(buffer, result);
    }

    /// пробуем искать последовательность (сокращение или many-to-many) с идущими следом стартерами
    #[inline(always)]
    fn handle_starter_trie(
        &self,
        codepoint: CodepointWithData,
        result: &mut Vec<u32>,
        buffer: &mut Vec<CollationElement>,
        codepoints: &mut CodepointsIter,
    ) -> Option<CodepointWithData>
    {
        let mut node = TrieNode::from(&self.tries, codepoint.data_pos());
        let mut children = TrieIter::new(&self.tries, node.next_pos());

        // среди потомков только нестартеры - отправляем узел в буфер
        if !children.current_node().is_starter() {
            buffer.push(node.as_ce_trie());

            return None;
        }

        // получаем следующий кодпоинт
        let mut second = codepoints.next_or_else(|| {
            result.extend_from_slice(&node.weights(&self.tries));
        })?;

        // встретили обычный стартер
        if second.is_starter() {
            loop {
                // следующий потомок текущего узла, не нашли - пишем веса текущиго узела + веса стартера
                let child_node = children.next_or_else(|| {
                    result.extend_from_slice(&node.weights(&self.tries));
                    second.write_starter_weights(result, &self.expansions);
                })?;

                // нашли искомый стартер
                if child_node.code() == second.code {
                    // потомков нет - записываем веса текущего узла
                    if !(child_node.has_children()) {
                        result.extend_from_slice(&child_node.weights(&self.tries));

                        return None;
                    }

                    // есть потомки - передвигаем указатель на узел, получаем следующий кодпоинт
                    node = child_node;

                    second = codepoints.next_or_else(|| {
                        result.extend_from_slice(&node.weights(&self.tries));
                    })?;

                    // получили кодпоинт, который не является стартером - элементом последовательности
                    if !second.is_starter() {
                        buffer.push(node.as_ce_trie_node());

                        return Some(second);
                    }

                    // получили стартер - продолжаем цикл с потомками нового узла
                    children = TrieIter::new(&self.tries, node.next_pos());
                }

                // проверяемый стартер отсутствует среди возможных комбинаций
                // пишем оба CE в результат
                if child_node.ccc() != 0 {
                    result.extend_from_slice(&node.weights(&self.tries));
                    second.write_starter_weights(result, &self.expansions);

                    return None;
                }
            }
        }

        // второй кодпоинт не является стартером - пишем в буфер узел, а кодпоинт отдаём обратно в цикл обработки
        buffer.push(node.as_ce_trie_node());

        Some(second)
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
        let mut children = TrieIter::new(&self.tries, node.next_pos());
        let mut index = 0;

        // получаем первый кодпоинт из буфера
        let mut ce = match index < buffer.len() {
            true => buffer[index],
            false => {
                result.extend_from_slice(&node.weights(&self.tries));

                return;
            }
        };

        'outer: loop {
            loop {
                // получаем следующий вариант комбинации в дереве
                let child_node = match children.next() {
                    Some(child_node) => child_node,
                    None => break 'outer,
                };

                let child_ccc = child_node.ccc();

                // CCC варианта в дереве > CCC рассматриваемого кодпоинта из буфера
                if child_ccc > ce.ccc {
                    loop {
                        index += 1;

                        ce = match index < buffer.len() {
                            true => buffer[index],
                            false => break 'outer,
                        };

                        if ce.ccc >= child_ccc {
                            break;
                        }
                    }
                }

                // если совпадает CCC - выходим из цикл промотки
                if child_ccc == ce.ccc {
                    break;
                }
            }

            // совпадает CCC, сравниваем кодпоинты
            let code = ce.code;
            let child_code = children.current_node().code();

            if code == child_code {
                buffer.remove(index);
                node = children.current_node();

                if !node.has_children() {
                    break 'outer;
                }

                children = TrieIter::new(&self.tries, node.next_pos());

                ce = match index < buffer.len() {
                    true => buffer[index],
                    false => break 'outer,
                };
            }
        }

        result.extend_from_slice(node.weights(&self.tries));

        // запишем и очистим буфер
        self.flush_buffer(buffer, result);
    }

    /// записать веса из буффера, обработав случай нестартеров с декомпозицией, очистка буфера
    #[inline(always)]
    fn flush_buffer(&self, buffer: &mut Vec<CollationElement>, result: &mut Vec<u32>)
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
                    let node = TrieNode::from(&self.tries, pos);
                    let mut buffer = buffer_iter.as_slice().to_owned();

                    if node.has_children() {
                        self.handle_trie_nonstarters_sequence(node, result, &mut buffer);
                        break;
                    }

                    result.extend_from_slice(node.weights(&self.tries));
                }
                _ => unreachable!(),
            }
        }

        buffer.clear();
    }

    /// декомпозиция буфера, возвращает стартер, буфер - нестартеры, отсортированные по CCC
    #[inline(always)]
    fn decompose(&self, buffer: &mut Vec<CollationElement>) -> TrieNode
    {
        let pos = match buffer[0].value {
            // получаем указатель на декомпозицию - она записана сразу после основного узла
            CollationElementValue::Decomposition(pos) => {
                TrieNode::from(&self.tries, pos).next_pos()
            }
            // узел, полученный из обработки MARKER_STARTER_TRIE
            CollationElementValue::Trie(pos) => {
                buffer.remove(0);
                buffer.sort_by_key(|ce| ce.ccc);

                return TrieNode::from(&self.tries, pos);
            }
            _ => unreachable!(),
        };

        let mut decomposition = TrieIter::new(&self.tries, pos);

        let starter = decomposition.next().unwrap();

        // стартер + нестартер - наиболее встречаемая комбинация, поэтому использование
        // дополнительного буфера для нестартеров декомпозиции может быть избыточным

        buffer[0] = decomposition.next().unwrap().as_ce_weights();

        let mut i = 1;

        while let Some(nonstarter) = decomposition.next() {
            buffer.insert(i, nonstarter.as_ce_weights());
            i += 1;
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
