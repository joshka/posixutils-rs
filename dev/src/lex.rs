//
// Copyright (c) 2024 Jeff Garzik
//
// This file is part of the posixutils-rs project covered under
// the MIT License.  For the full license text, please see the LICENSE
// file in the root directory of this project.
// SPDX-License-Identifier: MIT
//

extern crate clap;
extern crate plib;

use clap::Parser;
use gettextrs::{bind_textdomain_codeset, textdomain};
use plib::PROJECT_NAME;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Read};

/// lex - generate programs for lexical tasks
#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    /// Suppress the summary of statistics usually written with the -v option.
    #[arg(short, long)]
    no_stats: bool,

    /// Write the resulting program to standard output instead of lex.yy.c.
    #[arg(short = 't', long)]
    stdout: bool,

    /// Write a summary of lex statistics to the standard output.
    #[arg(short, long)]
    verbose: bool,

    /// Files to read as input.
    files: Vec<String>,
}

#[derive(Debug)]
struct LexInfo {
    external_def: Vec<String>,
    subs: HashMap<String, String>,
}

impl LexInfo {
    fn from(state: &ParseState) -> LexInfo {
        LexInfo {
            external_def: state.external_def.clone(),
            subs: state.subs.clone(),
        }
    }
}

#[derive(Debug)]
enum LexSection {
    Definitions,
    Rules,
    UserCode,
}

#[derive(Debug)]
struct ParseState {
    section: LexSection,
    in_def: bool,
    external_def: Vec<String>,
    sub_re: Regex,
    subs: HashMap<String, String>,
}

impl ParseState {
    fn new() -> ParseState {
        ParseState {
            section: LexSection::Definitions,
            in_def: false,
            external_def: Vec::new(),
            sub_re: Regex::new(r"(\w+)\s+(.*)").unwrap(),
            subs: HashMap::new(),
        }
    }
}

fn parse_def_line(state: &mut ParseState, line: &str) -> Result<(), &'static str> {
    if line.len() == 0 {
        return Ok(());
    }

    let mut char_iter = line.chars();
    let first_char = char_iter.next().unwrap();

    if first_char == '%' {
        let mut words = Vec::new();
        for word in line.split_whitespace() {
            words.push(String::from(word));
        }

        let cmd = words.remove(0);
        match cmd.as_str() {
            "%{" => {
                state.in_def = true;
            }
            "%}" => {
                state.in_def = false;
            }
            "%%" => {
                state.section = LexSection::Rules;
            }
            _ => {
                eprintln!("Unexpected command in definitions section: {}", cmd);
            }
        }
    } else if state.in_def || (first_char.is_whitespace() && line.len() > 1) {
        state.external_def.push(String::from(line));
    } else if let Some(caps) = state.sub_re.captures(line) {
        let name = caps.get(1).unwrap().as_str();
        let value = caps.get(2).unwrap().as_str();
        state.subs.insert(String::from(name), String::from(value));
    } else {
        eprintln!("Unexpected line in definitions section: {}", line);
    }
    Ok(())
}

fn parse_rule_line(_state: &mut ParseState, _line: &str) -> Result<(), &'static str> {
    Ok(())
}

fn parse_user_line(_state: &mut ParseState, _line: &str) -> Result<(), &'static str> {
    Ok(())
}

fn parse_lex_finalize(_state: &mut ParseState) -> Result<(), &'static str> {
    Ok(())
}

fn parse_lex_input(input: &[String]) -> Result<LexInfo, &'static str> {
    let mut state = ParseState::new();

    for line in input {
        match state.section {
            LexSection::Definitions => parse_def_line(&mut state, line)?,
            LexSection::Rules => parse_rule_line(&mut state, line)?,
            LexSection::UserCode => parse_user_line(&mut state, line)?,
        }
    }

    parse_lex_finalize(&mut state)?;

    let lexinfo = LexInfo::from(&state);

    eprintln!("PARSE STATE: {:#?}", state);

    Ok(lexinfo)
}

// concatenate input files, handling special filename "-" as stdin
fn concat_input_files(files: &[String]) -> io::Result<Vec<String>> {
    let mut input = Vec::new();

    for filename in files {
        let mut file: Box<dyn Read>;
        if filename == "-" {
            file = Box::new(io::stdin().lock());
        } else {
            file = Box::new(fs::File::open(filename)?);
        }
        let mut reader = io::BufReader::new(file);

        loop {
            let mut line = String::new();
            let n_read = reader.read_line(&mut line);
            match n_read {
                Ok(0) => break,
                Ok(_) => {
                    input.push(line);
                }
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    return Err(e);
                }
            }
        }
    }

    Ok(input)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // parse command line arguments
    let mut args = Args::parse();

    textdomain(PROJECT_NAME)?;
    bind_textdomain_codeset(PROJECT_NAME, "UTF-8")?;

    // if no files, read from stdin
    if args.files.is_empty() {
        args.files.push(String::from("-"));
    }

    let rawinput = concat_input_files(&args.files)?;
    let lexinfo = parse_lex_input(&rawinput)?;

    println!("{:#?}", lexinfo);

    Ok(())
}
