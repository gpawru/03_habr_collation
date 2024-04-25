mod iter;

pub use iter::CodepointsIter;

use crate::ce::*;
use crate::{MARKER_STARTER_EXPANSION, MARKER_STARTER_SINGLE_WEIGHTS};

/// битовая маска маркера хранимого значения информации о свойствах кодпоинта
const MARKER_MASK: u8 = 0b_111;

/// кодпоинт и сжатая информация о нём
#[derive(Debug, Clone, Copy)]
pub struct CodepointWithData
{
    pub data: u64,
    pub code: u32,
}

impl CodepointWithData
{
    /// маркер типа данных кодпоинта
    #[inline(always)]
    pub fn marker(&self) -> u8
    {
        self.data as u8 & MARKER_MASK
    }

    /// стартер с одинарными весами или расширение
    #[inline(always)]
    pub fn is_starter(&self) -> bool
    {
        let marker = self.marker();

        marker == MARKER_STARTER_SINGLE_WEIGHTS || marker == MARKER_STARTER_EXPANSION
    }

    /// только стартеры: записать веса стартера в результат
    #[inline(always)]
    pub fn write_starter_weights(&self, result: &mut Vec<u32>, expansions: &[u32])
    {
        match self.marker() {
            MARKER_STARTER_SINGLE_WEIGHTS => {
                result.push(self.single_weights());
            }
            MARKER_STARTER_EXPANSION => {
                result.extend_from_slice(self.expansion_weights(expansions))
            }
            _ => unreachable!(),
        };
    }

    /// случай одинарных весов - они хранятся непосредственно в значении data
    #[inline(always)]
    pub fn single_weights(&self) -> u32
    {
        (self.data >> 4) as u32
    }

    /// веса из таблицы расширений стартеров
    #[inline(always)]
    pub fn expansion_weights<'a>(&self, expansions: &'a [u32]) -> &'a [u32]
    {
        let start = self.data_pos();
        let end = start + self.ccc_or_len() as u16;

        &expansions[start as usize .. end as usize]
    }

    /// CCC кодпоинта с одинарными весами
    #[inline(always)]
    pub fn single_weights_ccc(&self) -> u8
    {
        (self.data >> 36) as u8
    }

    /// индекс начала данных в expansions / tries
    #[inline(always)]
    pub fn data_pos(&self) -> u16
    {
        (self.data >> 4) as u16
    }

    /// - ССС кодпоинта / последнего элемента декомпозиции кодпоинта - если кодпоинт является
    ///   началом последовательности / имеет декомпозицию
    /// - длина весов, если он - стартер с несколькими весами (расширение)
    #[inline(always)]
    pub fn ccc_or_len(&self) -> u8
    {
        (self.data >> 20) as u8
    }

    /// элемент сопоставления - одинарные веса
    #[inline(always)]
    pub fn as_ce_single_weights(&self) -> CollationElement
    {
        CollationElement {
            ccc: self.single_weights_ccc(),
            code: self.code,
            value: CollationElementValue::SingleWeights(self.single_weights()),
        }
    }

    /// элемент сопоставления - декомпозиция
    #[inline(always)]
    pub fn as_ce_decomposition(&self) -> CollationElement
    {
        CollationElement {
            ccc: 0,
            code: self.code,
            value: CollationElementValue::Decomposition(self.data_pos()),
        }
    }
}
