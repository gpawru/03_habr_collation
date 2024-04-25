use crate::ce::{CollationElement, CollationElementValue};

/// узел бора весов
#[derive(Debug, Clone, Copy)]
pub struct TrieNode
{
    value: u32,
    pos: u16,
}

impl TrieNode
{
    /// из переданного значения
    #[inline(always)]
    pub fn new(value: u32, pos: u16) -> Self
    {
        Self { value, pos }
    }

    /// из массива, где хранится бор
    #[inline(always)]
    pub fn from(source: &[u32], pos: u16) -> Self
    {
        Self {
            value: source[pos as usize],
            pos,
        }
    }

    /// кодпоинт
    #[inline(always)]
    pub fn code(&self) -> u32
    {
        (self.value >> 8) & 0x3FFFF
    }

    /// CCC
    #[inline(always)]
    pub fn ccc(&self) -> u8
    {
        (self.value >> 2) as u8 & 0x3F
    }

    /// стартер?
    #[inline(always)]
    pub fn is_starter(&self) -> bool
    {
        self.ccc() == 0
    }

    /// позиция в массиве
    #[inline(always)]
    pub fn pos(&self) -> u16
    {
        self.pos
    }

    /// флаг наличия потомков
    #[inline(always)]
    pub fn has_children(&self) -> bool
    {
        (self.value & 1) != 0
    }

    /// позиция следующего элемента в массиве
    #[inline(always)]
    pub fn next_pos(&self) -> u16
    {
        self.pos + self.weights_len() as u16 + 1
    }

    /// элемент сопоставления - веса
    #[inline(always)]
    pub fn as_ce_weights(&self) -> CollationElement
    {
        CollationElement {
            ccc: self.ccc(),
            code: self.code(),
            value: CollationElementValue::TrieWeights(self.weights_pos(), self.weights_len()),
        }
    }

    /// элемент сопоставления - начало последовательности
    #[inline(always)]
    pub fn as_ce_trie(&self) -> CollationElement
    {
        CollationElement {
            ccc: self.ccc(),
            code: self.code(),
            value: CollationElementValue::Trie(self.pos()),
        }
    }

    /// элемент сопоставления - узел последовательности, пишем как несуществующий кодпоинт-стартер
    #[inline(always)]
    pub fn as_ce_trie_node(&self) -> CollationElement
    {
        CollationElement {
            ccc: 0,
            code: 0,
            value: CollationElementValue::Trie(self.pos()),
        }
    }

    /// слайс весов узла
    #[inline(always)]
    pub fn weights<'a>(&self, from: &'a [u32]) -> &'a [u32]
    {
        let start = (self.pos + 1) as usize;

        &from[start .. start + self.weights_len() as usize]
    }

    /// позиция начала записи весов в массиве
    #[inline(always)]
    pub fn weights_pos(&self) -> u16
    {
        self.pos + 1
    }

    /// длина весов
    #[inline(always)]
    pub fn weights_len(&self) -> u8
    {
        (self.value >> 26) as u8
    }
}
