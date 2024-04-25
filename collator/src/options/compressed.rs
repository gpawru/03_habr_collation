use super::CollatorOptions;

/// числовое значение опций - для сохранения, битовых операций
#[derive(Copy, Clone)]
pub struct CollatorOptionsValue(u16);

impl From<CollatorOptionsValue> for u16
{
    fn from(value: CollatorOptionsValue) -> Self
    {
        value.0
    }
}

impl From<CollatorOptions> for CollatorOptionsValue
{
    fn from(options: CollatorOptions) -> Self
    {
        Self(
            options.strength as u16
                | ((options.alternate as u16) << 3)
                
        )
    }
}

impl From<CollatorOptionsValue> for CollatorOptions
{
    fn from(value: CollatorOptionsValue) -> Self
    {
        unsafe {
            Self {
                strength: core::mem::transmute((value.0 as u8) & 7),
                alternate: core::mem::transmute(((value.0 as u8) >> 3) & 1),
            }
        }
    }
}
