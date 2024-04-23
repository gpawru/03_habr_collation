/// элемент сопоставления
#[derive(Debug, Clone, Copy)]
pub struct CollationElement
{
    pub ccc: u8,
    pub code: u32,
    pub value: CollationElementValue,
}

#[derive(Debug, Clone, Copy)]
pub enum CollationElementValue
{
    /// одиночные веса
    SingleWeights(u32),
    /// декомпозиция
    Decomposition(u16),
    /// кодпоинт - начало последовательности
    Trie(u16),
    /// элемент с весами, записанными в таблице tries
    TrieWeights(u16, u8),
}

impl CollationElement
{
    /// стартер?
    #[inline(always)]
    pub fn is_starter(&self) -> bool
    {
        self.ccc == 0
    }
}
