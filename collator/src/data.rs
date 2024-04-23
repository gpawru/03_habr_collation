pub struct WeightsData<'a>
{
    /// индексы
    pub index: &'a [u16],
    /// данные u32
    pub scalars32: &'a [u32],
    /// данные u64
    pub scalars64: &'a [u64],
    /// расширения
    pub expansions: &'a [u32],
    /// сокращения, many-to-many, декомпозиции
    pub tries: &'a [u32],
    /// с U+0000 и до этого кодпоинта включительно блоки в data идут последовательно
    pub continuous_block_end: u32,
}

pub fn cldr_und<'a>() -> WeightsData<'a>
{
    include!("./../../data/cldr_und.txt")
}
