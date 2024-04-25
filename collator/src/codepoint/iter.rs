use super::CodepointWithData;

/// блок последнего кодпоинта с весами / декомпозицией (U+2FA1D)
const LAST_CODEPOINT_BLOCK: u16 = (0x2FA1D >> (18 - 11)) as u16;
/// блоки кодпоинтов с нулевыми весами, U+E0000 .. U+E0200
const IGNORABLES_BLOCKS: core::ops::Range<u16> = 0x1C00 .. 0x1C04;
/// первичные индексы для U+E0000.. сдвинуты для уменьшения размера
const IGNORABLES_SHIFT: u16 = 0x160B;

/// итератор по кодпоинтам
pub struct CodepointsIter<'a>
{
    iter: core::str::Chars<'a>,
    /// стартеры и нестартеры с одинарными весами
    scalars64: &'a [u64],
    /// декомпозиции, сокращения и т.д.
    scalars32: &'a [u32],
    /// индексы
    index: &'a [u16],
    /// с U+0000 и до этого кодпоинта включительно блоки в data идут последовательно
    continuous_block_end: u32,
}

impl<'a> Iterator for CodepointsIter<'a>
{
    type Item = CodepointWithData;

    /// получаем кодпоинт с данными о нём
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item>
    {
        let code = self.iter.next()? as u32;
        let data = self.get_data_value(code);

        Some(CodepointWithData { data, code })
    }
}

impl<'a> CodepointsIter<'a>
{
    /// запись о кодпоинте
    #[inline(always)]
    fn get_data_value(&self, code: u32) -> u64
    {
        let data_block_base = match code <= self.continuous_block_end {
            true => 0x600 | (((code >> 3) as u16) & !0xF),
            false => {
                let mut group_index = (code >> 7) as u16;

                // все кодпоинты, следующие за U+2FA1D имеют вычисляемые веса и не имеют декомпозиции
                // кроме диапазона U+E0000 .. U+E01EF, где кодпоинты могут иметь нулевые веса
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

    /// получить следующий кодпоинт, в случае None - выполнить код
    #[inline(always)]
    pub fn next_or_else<F>(&mut self, on_none: F) -> Option<CodepointWithData>
    where
        F: FnOnce(),
    {
        match self.next() {
            None => {
                on_none();
                None
            }
            value => value,
        }
    }

    /// итератор по кодпоинтам строки, с данными о весах, декомпозиции, последовательностях
    pub fn new(
        input: &'a str,
        scalars64: &'a [u64],
        scalars32: &'a [u32],
        index: &'a [u16],
        continuous_block_end: u32,
    ) -> Self
    {
        Self {
            iter: input.chars(),
            scalars64,
            scalars32,
            index,
            continuous_block_end,
        }
    }
}
