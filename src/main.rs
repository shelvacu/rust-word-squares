#![feature(exclusive_range_pattern)]
#![feature(nll)]

extern crate fnv;
extern crate unicode_skeleton;


use std::default::Default;

use std::vec::Vec;

use std::io::{self, BufReader, BufWriter};
use std::io::prelude::*;
use std::fs::File;

use fnv::FnvHashMap;

use unicode_skeleton::UnicodeSkeleton;

macro_rules! make_encode_decode {
    (
        $( $num:expr => $char:expr; )+
    ) => {
        fn encode(from:char) -> Option<u8> {
            let res:u8 = match from {
                $(
                    $char => $num,
                )+
                _ => return None,
            };
            return Some(res)
        }

        fn decode(code:u8) -> Option<char> {
            let res:char = match code {
                $(
                    $num => $char,
                )+
                _ => return None,
            };
            return Some(res)
        }
    }
}

make_encode_decode!{
    0 => 'a';
    1 => 'e';
    2 => 'i';
    3 => 'o';
    4 => 'r';
    5 => 'n';
    6 => 'l';
    7 => 's';
    8 => 't';
    9 => 'u';
    10 => 'p';
    11 => 'c';
    12 => 'd';
    13 => 'k';
    14 => 'y';
    15 => 'g';
    16 => 'h';
    17 => 'b';
    18 => 'v';
    19 => 'f';
    20 => 'w';
    21 => 'z';
    22 => 'j';
    23 => 'x';
    24 => '\'';
    25 => '-';
    26 => '1';
    27 => '2';
    28 => '3';
    29 => '4';
    30 => 'm';
    31 => 'q';
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
struct CharSet {
    pub internal:u32
}

impl CharSet {
    fn new(internal:u32) -> CharSet {
        return CharSet{internal}
    }

    fn add(&mut self, val:u8) {
        if val > 31 {panic!("Invalid val {}", val)}
        self.internal |= 2u32.pow(val as u32)
    }

    fn and(&self, other:&Self) -> Self {
        Self{ internal: self.internal & other.internal }
    }

    fn has(&self, val:u8) -> bool {
        if val > 31 {
            panic!("Invalid val {}", val)
        } else {
            return (self.internal & 2u32.pow(val as u32)) > 0
        }
    }
}

impl Default for CharSet {
    fn default() -> Self {
        CharSet::new(0)
    }
}


// NOTE: can only go up to 15. 16 would break everything
const WORD_SQUARE_ORDER:usize = 8;

const WORD_ORDER_U8:u8 = WORD_SQUARE_ORDER as u8;

const WORD_SQUARE_SIZE:usize = WORD_SQUARE_ORDER * WORD_SQUARE_ORDER;

type Word = [u8; WORD_SQUARE_ORDER];
type WordSquare = [u8; WORD_SQUARE_SIZE];

fn print_word_square( sq:[u8; WORD_SQUARE_SIZE] ){
    for i in 0..WORD_SQUARE_ORDER {
        let mut chars = Vec::new();
        for j in 0..WORD_SQUARE_ORDER {
            chars.push(decode(sq[i*WORD_SQUARE_ORDER + j]).unwrap());
        }
        let word = chars.iter().collect::<String>();
        println!("{}", word);
    }
    println!();
}

fn main() -> io::Result<()> {
    let mut args:Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Must have at least one argument (which sub-thing to run)");
        return Ok(());
    }

    eprintln!("{:?}", args);
    args.remove(0);
    eprintln!("{:?}", args);
    let name:&str = &(args.remove(0));
    eprintln!("{:?}", args);
    match name {
        "wordlist-preprocess" => return wordlist_preprocess(args),
        "compute" => return compute_command(args),
        unfound_command => eprintln!("unrecognized command {}", unfound_command),
    }
    return Ok(())
}

fn filter_word(word:&str) -> Option<String> {
    let mut success = true;
    let res = Some(word.chars().map(|c| {
        match encode(c) {
            Some(_) => c,
            None => {
                let chars:Vec<char> = c.to_string().skeleton_chars().collect();
                if chars.len() != 1 {
                    success = false;
                    'a'
                } else {
                    match encode(chars[0]) {
                        Some(_) => chars[0],
                        None => {success = false; 'a'},
                    }
                }
            },
        }
    }).collect::<String>());
    if success {
        return res
    } else {
        return None
    }
}

fn wordlist_preprocess(args:Vec<String>) -> io::Result<()> {
    if args.len() != 2 {
        eprintln!("Expecting exactly two arguments (in file and out file)");
        return Ok(());
    }

    let in_file =  File::open(args[0].clone())?;
    let out_file = File::create(args[1].clone())?;

    let f = BufReader::new(in_file);
    let mut fo = BufWriter::new(out_file);
    let mut lines = f.lines();
    lines.next().unwrap()?;
    for line_result in lines {
        let line = line_result?;
        let mut split = line.split('\t');
        split.next().unwrap(); // skip before tab
        let word = split.next().unwrap();
        match split.next() {
            Some(_) => panic!("Only one tab expected per line"),
            None => (),
        }
        match filter_word(word) {
            Some(word) => writeln!(&mut fo, "{}", word)?,
            None => (),
        }
    }
    return Ok(());
}

fn compute_command(args:Vec<String>) -> io::Result<()> {
    //println!("{:?}", "abcdefghijklmnopqrstuvwxyz".skeleton_chars().collect::<Vec<char>>());
    //return Ok(());

    eprintln!("Start.");
    
    let mut words_index = FnvHashMap::default();
    //let mut unused_chars = HashMap::new();
        
    let plain_f = File::open(args[0].clone())?;
    let f = BufReader::new(plain_f);
    let lines = f.lines();
    for line_result in lines {
        let word = line_result?;

        let chars:Vec<char> = word.chars().collect();
        if chars.len() != WORD_SQUARE_ORDER { continue }
        let mut codes = Vec::new();
        let mut all_encoded = true;
        for c in chars {
            match encode(c) {
                Some(code) => codes.push(code),
                None => {
                    all_encoded = false;

                    continue
                    /*
                    if !unused_chars.contains_key(&c) {
                        unused_chars.insert(c, 0u64);
                    }
                    let count = unused_chars[&c];
                    unused_chars.insert(c, count + 1);
                    */
                },
            }
        }
        if !all_encoded { continue }
        assert_eq!(codes.len(), WORD_SQUARE_ORDER);
        let mut word = Word::default();
        for (i, code) in codes.iter().enumerate() {
            word[i] = *code;
        }
        for j in 0..WORD_SQUARE_ORDER {
            let i = (WORD_SQUARE_ORDER - 1) - j;
            // for i in WORD_SQUARE_ORDER..0 including 0, excluding WORD_SQUARE_ORDER
            let code = word[i];
            word[i] = 255u8;
            if !words_index.contains_key(&word) {
                words_index.insert(word, CharSet::default());
            }
            words_index.get_mut(&word).unwrap().add(code);
        }
    }


    eprintln!("Finished creating index");


    let code_array = [255u8; WORD_SQUARE_SIZE];
    let start_idx:u8 = 0u8;
    let target_idx = WORD_SQUARE_SIZE as u8;

    compute(&words_index, code_array, start_idx, target_idx);
    
    /*let mut char_counts:Vec<(char,u64)> = unused_chars.drain().collect();
    char_counts.sort_unstable_by_key(|t| t.1);
    for (k,v) in char_counts.iter() {
        println!("Char {:?} had {} instances", k, v);
    }*/
    Ok(())
}

fn compute(
    words_index:&FnvHashMap<Word, CharSet>,
    mut code_array:WordSquare,
    start_idx:u8,
    target_idx:u8,
) {
    let mut at_idx = start_idx;
    let mut charset_array = [CharSet::new(std::u32::MAX); WORD_SQUARE_SIZE];
    // wrap to go from 0 to 255
    let end_idx = start_idx.wrapping_sub(1);
    while at_idx != end_idx {
        // wrap to go from 255 (initial) to 0
        code_array[at_idx as usize] = code_array[at_idx as usize].wrapping_add(1);
        let cur_code = code_array[at_idx as usize];
        let cur_charset = charset_array[at_idx as usize];
        if cur_code == 32 {
            at_idx = at_idx.wrapping_sub(1)
        } else if cur_charset.has(cur_code) {
            at_idx += 1;
            if at_idx == target_idx {
                print_word_square(code_array);
                at_idx -= 1;
            } else {
                code_array[at_idx as usize] = 0;

                let row_idx = at_idx / WORD_ORDER_U8;
                let col_idx = at_idx % WORD_ORDER_U8;
                
                let row_start = row_idx*WORD_ORDER_U8;
                let mut row_word = [255u8; WORD_SQUARE_ORDER];
                for i in 0..col_idx {
                    row_word[i as usize] = code_array[ (row_start+i) as usize ];
                }
                let row_wordset = words_index[&row_word];

                let mut col_word = [255u8; WORD_SQUARE_ORDER];
                for i in 0..row_idx {
                    col_word[i as usize] = code_array[ (col_idx + i*WORD_ORDER_U8) as usize ];
                }
                let col_wordset = words_index[&col_word];
                
                charset_array[at_idx as usize] = col_wordset.and(&row_wordset);
            }
        }
    }

}
