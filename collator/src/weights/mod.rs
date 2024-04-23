use core::fmt::Debug;

mod output;

pub use output::output_weights;

/// веса для кодпоинта, 3 уровня
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Weights
{
    pub l1: u16,
    pub l2: u16,
    pub l3: u16,
    pub is_variable: bool,
}

impl Weights
{
    /// в виде, в котором веса представлены в allkeys
    pub fn format(&self) -> String
    {
        let is_variable = match self.is_variable {
            true => '*',
            false => '.',
        };

        format!(
            "[{}{:04X}.{:04X}.{:04X}]",
            is_variable, self.l1, self.l2, self.l3,
        )
    }
}

impl From<u32> for Weights
{
    fn from(value: u32) -> Self
    {
        Self {
            l1: value as u16,
            l2: (value >> 16) as u16 & 0x1FF,
            l3: (value >> 25) as u16 & 0x1F,
            is_variable: (value >> 30) != 0,
        }
    }
}

impl Debug for Weights
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        f.write_str(self.format().as_str())
    }
}
