#[derive(Debug, Copy, Clone, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Strength
{
    Primary = 0b_000,
    Secondary = 0b_001,
    Tetriary = 0b_010,
    Quaternary = 0b_011,
    Identical = 0b_111,
}

#[derive(Debug, Copy, Clone, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum CaseFirst
{
    Off = 0,
    LowerFirst = 1,
    UpperFirst = 2,
}

#[derive(Debug, Copy, Clone)]
pub struct CollatorOptions
{
    pub strength: Option<Strength>,
    pub case_first: Option<CaseFirst>,
}
