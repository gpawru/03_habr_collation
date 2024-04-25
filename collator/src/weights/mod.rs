use core::fmt::Debug;

/// веса для кодпоинта, 3 уровня
#[derive(Clone, Copy)]
pub struct Weights(u32);

impl Weights
{
    /// сжатое значение
    #[inline(always)]
    pub fn value(&self) -> u32
    {
        self.0
    }

    /// первичные веса
    #[inline(always)]
    pub fn l1(&self) -> u16
    {
        self.0 as u16
    }

    /// вторичные веса
    #[inline(always)]
    pub fn l2(&self) -> u16
    {
        (self.0 >> 16) as u16 & 0x1FF
    }

    /// третичные веса
    #[inline(always)]
    pub fn l3(&self) -> u16
    {
        (self.0 >> 25) as u16 & 0x1F
    }

    /// переменный вес
    #[inline(always)]
    pub fn is_variable(&self) -> bool
    {
        (self.0 >> 30) != 0
    }

    /// в виде, в котором веса представлены в allkeys
    pub fn format(&self) -> String
    {
        let is_variable = match self.is_variable() {
            true => '*',
            false => '.',
        };

        format!(
            "[{}{:04X}.{:04X}.{:04X}]",
            is_variable,
            self.l1(),
            self.l2(),
            self.l3(),
        )
    }
}

impl From<u32> for Weights
{
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl Debug for Weights
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        f.write_str(self.format().as_str())
    }
}
