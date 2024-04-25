use crate::{options::*, weights::Weights};

/// ключ сопоставления с дополнительной информацией о нём
#[derive(Clone)]
pub struct Key
{
    /// u16 веса
    pub weights: Vec<u16>,
    // кол-во весов первичного уровня
    pub l1_len: usize,
    /// кол-во весов вторичного уровня
    pub l2_len: usize,
    /// кол-во весов третичного уровня
    pub l3_len: usize,
}

/// создать ключ из u32-весов
#[inline]
pub fn compose_key(u32_weights: &Vec<u32>, options: CollatorOptions) -> Key
{
    let weights: &Vec<Weights> = unsafe { core::mem::transmute(u32_weights) };

    let result = match options.alternate {
        AlternateHandling::NonIgnorable => compose_non_ignorable_key(weights, options.strength),
        AlternateHandling::Shifted => compose_shifted_key(weights, options.strength),
    };

    result
}

/// Non Ignorable
#[inline]
fn compose_non_ignorable_key(weights: &Vec<Weights>, strength: Strength) -> Key
{
    let mut primary = vec![];
    let mut secondary = vec![];
    let mut tetriary = vec![];

    macro_rules! push {
        ($to: ident, $entry: expr) => {
            let value = $entry;

            if value != 0 {
                $to.push(value);
            }
        };
    }

    match strength {
        Strength::Primary => {
            for entry in weights {
                push!(primary, entry.l1());
            }
        }
        Strength::Secondary => {
            for entry in weights {
                push!(primary, entry.l1());
                push!(secondary, entry.l2());
            }
        }
        _ => {
            for entry in weights {
                push!(primary, entry.l1());
                push!(secondary, entry.l2());
                push!(tetriary, entry.l3());
            }
        }
    }

    macro_rules! append {
        ($($level: expr, $from:ident),+) => {
            $(
                if strength as u8 >= $level {
                    primary.push(0);
                    primary.append(&mut $from);
                }
            )+
        }
    }

    let l1_len = primary.len();
    let l2_len = secondary.len();
    let l3_len = tetriary.len();

    append!(2, secondary, 3, tetriary);

    Key {
        weights: primary,
        l1_len,
        l2_len,
        l3_len,
    }
}

/// Shifted
#[inline]
fn compose_shifted_key(weights: &Vec<Weights>, strength: Strength) -> Key
{
    let mut primary = vec![];
    let mut secondary = vec![];
    let mut tetriary = vec![];
    let mut quaternary = vec![];

    macro_rules! push {
        ($to: ident, $value: expr) => {
            if ($value != 0) {
                $to.push($value);
            }
        };
    }

    let mut following_a_variable = false;

    match strength {
        Strength::Primary => {
            for entry in weights {
                if !entry.is_variable() {
                    let l1 = entry.l1();

                    push!(primary, l1);
                }
            }
        }
        Strength::Secondary => {
            for entry in weights {
                if entry.is_variable() {
                    following_a_variable = true;
                    continue;
                }

                let l1 = entry.l1();

                if following_a_variable && l1 == 0 {
                    continue;
                }

                let l2 = entry.l2();

                push!(primary, l1);
                push!(secondary, l2);

                following_a_variable = false;
            }
        }
        Strength::Tetriary => {
            for entry in weights {
                if entry.is_variable() {
                    following_a_variable = true;
                    continue;
                }

                let l1 = entry.l1();

                if following_a_variable && l1 == 0 {
                    continue;
                }

                let l2 = entry.l2();
                let l3 = entry.l3();

                push!(primary, l1);
                push!(secondary, l2);
                push!(tetriary, l3);

                following_a_variable = false;
            }
        }
        Strength::Quaternary => {
            for entry in weights {
                // правила из TR #10: (https://www.unicode.org/reports/tr10/tr10-49.html#Variable_Weighting)
                //
                // L1, L2, L3 = 0                               -> [.0000.0000.0000.0000]
                // L1 = 0, L3 ≠ 0,   following a Variable       -> [.0000.0000.0000.0000] combining grave
                // L1 ≠ 0,           Variable	                -> old L1 [.0000.0000.0000.0209] space
                // L1 = 0, L3 ≠ 0,   not following a Variable   -> FFFF [.0000.0035.0002.FFFF] combining grave
                // L1 ≠ 0,           not Variable               -> FFFF [.06D9.0020.0008.FFFF] Capital A

                // игнорируемый вес
                if entry.value() == 0 {
                    continue;
                }

                let l1 = entry.l1();
                let l3 = entry.l3();

                // L1 = 0, L3 ≠ 0
                if (l1 == 0) && (l3 != 0) {
                    match following_a_variable {
                        true => continue,
                        false => {
                            push!(quaternary, 0xFFFF)
                        }
                    }
                }

                let is_variable = entry.is_variable();

                // L1 ≠ 0
                if l1 != 0 {
                    match is_variable {
                        true => {
                            following_a_variable = true;

                            push!(quaternary, l1);
                            continue;
                        }
                        false => {
                            // непонятно, насколько это корректно.
                            // U+FFFE - в правилах: FFFF. в тестах CLDR: 0001
                            if l1 == 1 {
                                push!(quaternary, 0x0001);
                            } else
                            // эта проверка - не соответствует официальной документации, но
                            // соответствует результатам, получаемым в тестах.
                            // пытаюсь уточнить :(
                            if l3 != 0 {
                                push!(quaternary, 0xFFFF)
                            }
                        }
                    }
                }

                let l2 = entry.l2();

                push!(primary, l1);
                push!(secondary, l2);
                push!(tetriary, l3);

                following_a_variable = is_variable;
            }
        }
    }

    let l1_len = primary.len();
    let l2_len = secondary.len();
    let l3_len = tetriary.len();

    macro_rules! append {
        ($($level: expr, $from:ident),+) => {
            $(
                if strength as u8 >= $level {
                    primary.push(0);
                    primary.append(&mut $from);
                }
            )+
        }
    }

    append!(2, secondary, 3, tetriary, 4, quaternary);

    Key {
        weights: primary,
        l1_len,
        l2_len,
        l3_len,
    }
}
