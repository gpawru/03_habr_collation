#![allow(dead_code)]
#![allow(unused)]

use std::cmp::{min, Ordering};

use unicode_collator::Collator;
use unicode_data::{
    codepoint::Codepoint, CollationTest, COLLATION_TEST_CLDR_NON_IGNORABLE,
    COLLATION_TEST_DUCET_NON_IGNORABLE, UNICODE,
};

// в блоке чамо (U+1100..U+11FF) могут быть скомбинированы кодпоинты:
//  - U+1100..=U+1112 (L, ведущие согласные)
//  - U+1161..=U+1176 (V, гласные)
//  - U+11A8..=U+11C3 (T, завершающие согласные)
// все они находятся в пределах диапазона U+1100..=U+11C3 (196 кодпоинтов)

/// начало блока ведущих согласных чамо
const HANGUL_L_BASE: u32 = 0x1100;
/// количество ведущих согласных
const HANGUL_L_COUNT: u32 = 19;
/// начало блока гласных чамо
const HANGUL_V_BASE: u32 = 0x1161;
/// количество гласных
const HANGUL_V_COUNT: u32 = 21;
/// начало блока завершающих согласных
const HANGUL_T_BASE: u32 = 0x11A8;
/// количество завершающих согласных
const HANGUL_T_COUNT: u32 = 27;
/// количество кодпоинтов на блок LV
const HANGUL_T_BLOCK_SIZE: u32 = HANGUL_T_COUNT + 1;
/// начало блока слогов хангыль
const HANGUL_S_BASE: u32 = 0xAC00;
/// количество слогов хангыль в Unicode
const HANGUL_S_COUNT: u32 = 11172;
/// количество гласных * количество завершающих согласных
const HANGUL_N_COUNT: u32 = 588;

#[test]
fn jamo()
{
    let collator = Collator::cldr_und();

    let ranges = (HANGUL_L_BASE .. HANGUL_L_BASE + HANGUL_L_COUNT)
        .chain(0 .. 1)
        .chain((HANGUL_V_BASE .. HANGUL_V_BASE + HANGUL_V_COUNT))
        .chain(0 .. 1)
        .chain((HANGUL_T_BASE .. HANGUL_T_BASE + HANGUL_T_COUNT));

    for code in ranges {
        if code == 0 {
            println!();
            continue;
        }

        let input = format!("{}", char::from_u32(code).unwrap());

        let key = collator.get_weights(&input);

        key.iter().for_each(|k| print!("{:04X} ", k));
        println!();
    }
}

#[test]
fn test()
{
    /*
        132226. AC00 (0) 0021 (0)  - (가) HANGUL SYLLABLE GA
        test: 4323 43A1 0167 0000 0020 0020 0020 0000 0002 0002 0002 
        key : 0167 0000 0020 0000 0002 

    */
    let collator = Collator::cldr_und();
    let test_num = 132226;

    /*







    */

    let tests = &COLLATION_TEST_CLDR_NON_IGNORABLE;

    let test = &tests[test_num];

    let codepoints = test
        .codes
        .iter()
        .map(|c| match UNICODE.get(c) {
            Some(codepoint) => (codepoint.code, codepoint.ccc.u8()),
            None => (*c, 0),
        })
        .collect::<Vec<(u32, u8)>>();

    for codepoint in codepoints.iter() {
        print!("{:04X} ({}) ", codepoint.0, codepoint.1);
    }
    println!(" - {}", test.description);

    let test_key = test_to_key(test);
    let key = collator.get_key(&test.as_string(), false);

    if test_key != key {
        println!(
            "test: {}",
            test_key
                .iter()
                .map(|e| format!("{:04X} ", e))
                .collect::<String>()
        );
        println!(
            "key : {}",
            key.iter()
                .map(|e| format!("{:04X} ", e))
                .collect::<String>()
        );

        return;
    }

    println!("ok?")
}

#[test]
fn collation()
{
    let collator = Collator::cldr_und();

    let tests = &COLLATION_TEST_CLDR_NON_IGNORABLE;

    for (i, test) in tests.iter().enumerate() {
        let codepoints = test
            .codes
            .iter()
            .map(|c| match UNICODE.get(c) {
                Some(codepoint) => (codepoint.code, codepoint.ccc.u8()),
                None => (*c, 0),
            })
            .collect::<Vec<(u32, u8)>>();

        let test_key = test_to_key(test);
        let key = collator.get_key(&test.as_string(), false);

        if test_key != key {
            print!("{}. ", i);

            for codepoint in codepoints.iter() {
                print!("{:04X} ({}) ", codepoint.0, codepoint.1);
            }
            println!(" - {}", test.description);

            println!(
                "test: {}",
                test_key
                    .iter()
                    .map(|e| format!("{:04X} ", e))
                    .collect::<String>()
            );
            println!(
                "key : {}",
                key.iter()
                    .map(|e| format!("{:04X} ", e))
                    .collect::<String>()
            );

            return;
        }
    }
}

// //    A646 0062;	# (Ꙇ) CYRILLIC CAPITAL LETTER IOTA	[251A 20C3 | 0020 0020 | 0008 0002 |]
// //    0438 0306 0334;	# (й) CYRILLIC SMALL LETTER I, COMBINING BREVE	[251B | 0020 004A | 0002 0002 |]

// let normalizer = DecomposingNormalizer::nfd();

// let b = "\u{0438}\u{0306}\u{0334}";
// let b_norm = normalizer.normalize(b);

// let key = compose_key(b_norm.as_str(), &DUCET_FILTERED_TRIE);

// print!("norm:  ");
// b.chars().for_each(|c| print!("{:04X} ", u32::from(c)));
// println!();

// print!("key:  ");
// key.iter().for_each(|&c| print!("{:04X} ", c));
// println!();

fn main() {}

fn compare_keys(a: &Vec<u16>, b: &Vec<u16>) -> Ordering
{
    if a == b {
        return Ordering::Equal;
    }

    let common_len = min(a.len(), b.len());

    match a[.. common_len].cmp(&b[.. common_len]) {
        Ordering::Less => Ordering::Less,
        Ordering::Equal => a.len().cmp(&b.len()),
        Ordering::Greater => Ordering::Greater,
    }
}

fn test_to_key(source: &CollationTest) -> Vec<u16>
{
    let mut result = vec![];

    for &weights in source.l1.iter() {
        result.push(weights);
    }

    result.push(0);

    for &weights in source.l2.iter() {
        result.push(weights);
    }

    result.push(0);

    for &weights in source.l3.iter() {
        result.push(weights);
    }

    result
}
