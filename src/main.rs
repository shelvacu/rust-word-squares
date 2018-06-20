//#![feature(exclusive_range_pattern)]
#![feature(nll)]

extern crate fnv;
extern crate spmc;
extern crate unicode_skeleton;
#[macro_use]
extern crate clap;

use std::default::Default;
use std::vec::Vec;
use std::io::{self, BufReader, BufWriter};
use std::io::prelude::*;
use std::fs::File;
use std::thread;

use fnv::FnvHashMap;

use unicode_skeleton::UnicodeSkeleton;

use clap::{Arg, App, SubCommand, ArgMatches};

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
                255 => '#',
                32 => '$',
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
    24 => 'A';//'\'';
    25 => 'B';//'-';
    26 => 'C';//'è';
    27 => 'D';//'ê';
    28 => 'E';//'ñ';
    29 => 'F';//'é';
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
const WORD_SQUARE_ORDER:usize = 7;

const WORD_ORDER_U8:u8 = WORD_SQUARE_ORDER as u8;

const WORD_SQUARE_SIZE:usize = WORD_SQUARE_ORDER * WORD_SQUARE_ORDER;

type Word = [u8; WORD_SQUARE_ORDER];
type WordSquare = [u8; WORD_SQUARE_SIZE];

fn print_word_square(sq:WordSquare){
    let mut first = true;
    for i in 0..WORD_SQUARE_ORDER {
        let mut chars = Vec::new();
        for j in 0..WORD_SQUARE_ORDER {
            chars.push(decode(sq[i*WORD_SQUARE_ORDER + j]).unwrap());
        }
        let word = chars.iter().collect::<String>();
        if !first {
            print!("-");
        }
        print!("{}", word);
        first = false;
    }
    println!();
}

fn main() -> io::Result<()> {
    let matches = App::new(format!("Rust Word Square Finder o{}", WORD_SQUARE_ORDER))
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .setting(clap::AppSettings::SubcommandRequired)
        .subcommand(SubCommand::with_name("compute")
                    .about("Does the actual computation.")
                    .arg(Arg::with_name("threads")
                         .default_value("4")
                         .takes_value(true)
                         .validator(|arg| {
                             match arg.parse::<u32>() {
                                 Ok(_) => Ok(()),
                                 Err(e) => Err(String::from(format!("Must provide a valid integer. {:?}", e))),
                             }
                         })
                         .help("Number of threads to use.")
                         .long("threads")
                         .short("t")
                    )
                    .arg(Arg::with_name("wordlist")
                         .required(true)
                         .help("the wordlist file path, a plain-text UTF-8 file with each word separated by a newline")
                    )
        )
        .subcommand(SubCommand::with_name("wordlist-preprocess")
                    .about("Takes in a wordlist (of various formats) and converts characters to a consistent set, for example 'а' (U+0430 CYRILLIC SMALL LETTER A) becomes 'a' (U+0061 LATIN SMALL LETTER A). Any words that would be ignored by the compute function are also filtered out.")
                    .arg(Arg::with_name("wiktionary-list-format")
                         .long("wiktionary-format")
                         .short("w")
                         .long_help("Input wordlist is in wiktionary \"all-titles\" format.")
                         .group("format")
                    )
                    .arg(Arg::with_name("plain-list-format")
                         .long("plain-format")
                         .short("p")
                         .long_help("Input wordlist is a plaintext UTF-8 newline-separated list of words")
                         .group("format")
                    )
                    .arg(Arg::with_name("input-filename")
                         .required(true)
                         .help("The path to the wordlist to read from, or \"-\" for stdin")
                    )
                    .arg(Arg::with_name("output-filename")
                         .required(true)
                         .help("The path to the wordlist to write to, or \"-\" for stdout")
                    )
        ).get_matches();
        ;           
    
    //println!("{:?}", matches.is_present("wordlist-preprocess"));

    return match matches.subcommand() {
        ("compute", Some(m)) => compute_command(m),
        ("wordlist-preprocess", Some(m)) => wordlist_preprocess(m),
        _ => panic!("This shouldn't happen"),
    }
    /*let mut args:Vec<String> = std::env::args().collect();
    
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
    }*/
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

fn wordlist_preprocess(args:&ArgMatches) -> io::Result<()> {

    let in_file =  File::open(  args.value_of("input-filename" ).unwrap())?;
    let out_file = File::create(args.value_of("output-filename").unwrap())?;

    let wik_format = args.is_present("wiktionary-list-format");
    
    let f = BufReader::new(in_file);
    let mut fo = BufWriter::new(out_file);
    let mut lines = f.lines();
    if wik_format {
        //Skip the first line
        lines.next().unwrap()?;
    }
    for line_result in lines {
        let line = line_result?;
        let word;
        if wik_format {
            let mut split = line.split('\t');
            split.next().unwrap(); // skip before tab
            word = split.next().unwrap();
            match split.next() {
                Some(_) => panic!("Only one tab expected per line"),
                None => (),
            }
        } else {
            word = &line
        }
        match filter_word(word) {
            Some(word) => writeln!(&mut fo, "{}", word)?,
            None => (),
        }
    }
    fo.flush()?;
    return Ok(());
}

//static mut global_words_index:&FnvHashMap<Word,CharSet>;// = FnvHashMap::default();


fn compute_command(args:&ArgMatches) -> io::Result<()> {
    //println!("{:?}", "abcdefghijklmnopqrstuvwxyz".skeleton_chars().collect::<Vec<char>>());
    //return Ok(());

    eprintln!("Word square order is {}", WORD_SQUARE_ORDER);
    eprintln!("Start: creating index.");

    let num_threads:u32 = args.value_of("threads").unwrap().parse().unwrap();

    //:&'static mut FnvHashMap<Word,CharSet>
    let mut words_index = FnvHashMap::default();
    let mut words_list  = Vec::new();
    //let mut unused_chars = HashMap::new();

    let plain_f = File::open(args.value_of("wordlist").unwrap())?;
    let f = BufReader::new(plain_f);
    let lines = f.lines();
    for line_result in lines {
        let word = line_result?;

        let chars:Vec<char> = word.chars().collect();
        //eprintln!("processing {:?} length {}", chars, chars.len());
        if chars.len() != WORD_SQUARE_ORDER { continue }
        let mut codes = Vec::new();
        let mut all_encoded = true;
        for c in chars.clone() {
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
        if !all_encoded {
            eprintln!("Skipping {:?}, not all could be encoded",chars);
            continue
        }
        assert_eq!(codes.len(), WORD_SQUARE_ORDER);
        let mut word = Word::default();
        for (i, code) in codes.iter().enumerate() {
            word[i] = *code;
        }
        words_list.push(word.clone());
        for j in 0..WORD_SQUARE_ORDER {
            let i = (WORD_SQUARE_ORDER - 1) - j;
            // for i in WORD_SQUARE_ORDER..0 including 0, excluding WORD_SQUARE_ORDER
            let code = word[i];
            word[i] = 255u8;
            if !words_index.contains_key(&word) {
                //println!("Inserting {:?}", word);
                words_index.insert(word, CharSet::default());
            }
            words_index.get_mut(&word).unwrap().add(code);
        }
    }

    if words_index.len() == 0 {
        panic!("No words in wordlist!");
    }
    eprintln!("Finished creating index, {} words.", words_list.len());


    let (m2w_tx, m2w_rx) = spmc::channel::<(WordSquare,u8)>();
    let (w2m_tx, w2m_rx) = std::sync::mpsc::sync_channel(16);
    let mut worker_handles = Vec::new();

    eprintln!("Creating {} worker threads.", num_threads);
    
    for _ in 0..num_threads {
        let rxc = m2w_rx.clone();
        let txc = w2m_tx.clone();
        let my_word = words_index.clone();
        worker_handles.push(
            thread::spawn( move || {
                while let Ok(msg) = rxc.recv() {
                    compute(
                        &my_word,
                        msg.0,
                        msg.1,
                        WORD_SQUARE_SIZE as u8,
                        |a,b| txc.send((a,b)).unwrap()
                    );
                }
            })
        );
    }

    drop(w2m_tx);

    let printing_thread = thread::spawn(move || {
        while let Ok(msg) = w2m_rx.recv() {
            print_word_square(msg.0);
        }
    });
    
    let code_array = [255u8; WORD_SQUARE_SIZE];

    eprintln!("Starting.");
    
    compute(
        &words_index,
        code_array,
        0u8,
        WORD_ORDER_U8,
        |ca, idx| m2w_tx.send((ca,idx)).unwrap()
    );

    drop(m2w_tx);
    //println!("Dropped");
    for h in worker_handles {
        h.join().unwrap();
        //println!("Worker finished");
    }
    printing_thread.join().unwrap();
    //println!("printing thread finished");
    
    /*let mut char_counts:Vec<(char,u64)> = unused_chars.drain().collect();
    char_counts.sort_unstable_by_key(|t| t.1);
    for (k,v) in char_counts.iter() {
        println!("Char {:?} had {} instances", k, v);
    }*/
    Ok(())
}

const DEBUG_MODE:bool = false;


fn compute<T:FnMut(WordSquare,u8)>(
    words_index_arg:&FnvHashMap<Word, CharSet>,
    mut code_array:WordSquare,
    start_idx:u8,
    target_idx:u8,
    mut on_result:T,
) {
    let mut at_idx = start_idx;
    let mut charset_array = [CharSet::new(std::u32::MAX); WORD_SQUARE_SIZE];


    let row_idx = at_idx / WORD_ORDER_U8;
    let col_idx = at_idx % WORD_ORDER_U8;
    let row_start = row_idx*WORD_ORDER_U8;
    let mut row_word = [255u8; WORD_SQUARE_ORDER];
    for i in 0..col_idx {
        row_word[i as usize] = code_array[ (row_start+i) as usize ];
    }
    let row_wordset = words_index_arg[&row_word];

    let mut col_word = [255u8; WORD_SQUARE_ORDER];
    for i in 0..row_idx {
        col_word[i as usize] = code_array[ (col_idx + i*WORD_ORDER_U8) as usize ];
    }
    let col_wordset = words_index_arg[&col_word];
    
    charset_array[at_idx as usize] = col_wordset.and(&row_wordset);

    // wrap to go from 0 to 255
    let end_idx = start_idx.wrapping_sub(1);
    while at_idx != end_idx {
        // wrap to go from 255 (initial) to 0
        if DEBUG_MODE {
            println!();
            println!(
                "idx {} before wrapping add is {}",
                at_idx,
                code_array[at_idx as usize]
            );
        }
        
        code_array[at_idx as usize] = code_array[at_idx as usize].wrapping_add(1);


        if DEBUG_MODE {
            let row_idx = at_idx / WORD_ORDER_U8;
            let col_idx = at_idx % WORD_ORDER_U8;
            for row in 0..WORD_SQUARE_ORDER {
                for col in 0..WORD_SQUARE_ORDER {
                    print!("{}, ", decode(code_array[row*WORD_SQUARE_ORDER + col]).unwrap());
                }
                println!();
            }
            println!("row_idx {}, col_idx {}", row_idx, col_idx);
        }

        
        let cur_code = code_array[at_idx as usize];
        if DEBUG_MODE { println!("cur_code {}", cur_code); }
        let cur_charset = charset_array[at_idx as usize];
        if cur_code == 32 {
            code_array[at_idx as usize] = 255u8;
            at_idx = at_idx.wrapping_sub(1)
        } else if cur_charset.has(cur_code) {
            at_idx += 1;
            if at_idx == target_idx {
                //print_word_square(code_array);
                (&mut on_result)(code_array, at_idx);
                at_idx -= 1;
            } else {
                code_array[at_idx as usize] = 255;

                let row_idx = at_idx / WORD_ORDER_U8;
                let col_idx = at_idx % WORD_ORDER_U8;
                let row_start = row_idx*WORD_ORDER_U8;
                let mut row_word = [255u8; WORD_SQUARE_ORDER];
                for i in 0..col_idx {
                    row_word[i as usize] = code_array[ (row_start+i) as usize ];
                }
                //println!("row_word {:?}", row_word);
                let row_wordset = words_index_arg[&row_word];

                let mut col_word = [255u8; WORD_SQUARE_ORDER];
                for i in 0..row_idx {
                    col_word[i as usize] = code_array[ (col_idx + i*WORD_ORDER_U8) as usize ];
                }
                //println!("col_word {:?}", row_word);
                let col_wordset = words_index_arg[&col_word];
                
                charset_array[at_idx as usize] = col_wordset.and(&row_wordset);
            }
        }
    }

}
