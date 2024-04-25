// все опции - см. UTS #35, https://www.unicode.org/reports/tr35/tr35-collation.html

mod compressed;

pub use compressed::CollatorOptionsValue;

/// уровень сравнения
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Strength
{
    Primary = 1,    // базовые символы
    Secondary = 2,  // диакритические знаки
    Tetriary = 3,   // регистр / варианты
    Quaternary = 4, // пунктуация
}

/// тип сравнения переменных весов
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum AlternateHandling
{
    NonIgnorable = 0, // переменные веса не игнорируются
    Shifted = 1,      // со сдвигом переменных весов
}

#[derive(Debug, Copy, Clone)]
pub struct CollatorOptions
{
    /// уровень сравнения
    pub strength: Strength,
    /// тип сравнения
    pub alternate: AlternateHandling,
}

impl Default for Strength
{
    fn default() -> Self
    {
        Self::Tetriary
    }
}

impl Default for AlternateHandling
{
    fn default() -> Self
    {
        Self::NonIgnorable
    }
}

impl Default for CollatorOptions
{
    fn default() -> Self
    {
        Self {
            strength: Default::default(),
            alternate: Default::default(),
        }
    }
}
