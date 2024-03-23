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

#[derive(Clone, Debug)]
struct LexRule {
    ere: String,
    action: String,
}

impl LexRule {
    fn new() -> LexRule {
        LexRule {
            ere: String::new(),
            action: String::new(),
        }
    }
}

#[derive(Debug)]
struct LexInfo {
    external_def: Vec<String>,
    subs: HashMap<String, String>,
    internal_defs: Vec<String>,
    cond_start: Vec<String>,
    cond_xstart: Vec<String>,
    yyt_is_ptr: bool,
    user_subs: Vec<String>,
    rules: Vec<LexRule>,
}

impl LexInfo {
    fn from(state: &ParseState) -> LexInfo {
        LexInfo {
            external_def: state.external_def.clone(),
            subs: state.subs.clone(),
            internal_defs: state.internal_defs.clone(),
            cond_start: state.cond_start.clone(),
            cond_xstart: state.cond_xstart.clone(),
            yyt_is_ptr: state.yyt_is_ptr,
            user_subs: state.user_subs.clone(),
            rules: state.rules.clone(),
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
    open_braces: u32,
    in_def: bool,
    external_def: Vec<String>,
    sub_re: Regex,
    subs: HashMap<String, String>,
    internal_defs: Vec<String>,
    user_subs: Vec<String>,
    cond_start: Vec<String>,
    cond_xstart: Vec<String>,
    yyt_is_ptr: bool,
    rules: Vec<LexRule>,
    tmp_rule: LexRule,
}

impl ParseState {
    fn new() -> ParseState {
        ParseState {
            section: LexSection::Definitions,
            open_braces: 0,
            in_def: false,
            external_def: Vec::new(),
            sub_re: Regex::new(r"(\w+)\s+(.*)").unwrap(),
            subs: HashMap::new(),
            internal_defs: Vec::new(),
            user_subs: Vec::new(),
            cond_start: Vec::new(),
            cond_xstart: Vec::new(),
            yyt_is_ptr: true,
            rules: Vec::new(),
            tmp_rule: LexRule::new(),
        }
    }

    fn push_rule(&mut self, ere: &str, action: &str) {
        self.rules.push(LexRule {
            ere: String::from(ere),
            action: String::from(action),
        });
    }
}

fn parse_def_line(state: &mut ParseState, line: &str) -> Result<(), &'static str> {
    if line.len() == 0 {
        return Ok(());
    }

    let first_char = line.chars().next().unwrap();

    if first_char == '%' {
        let mut words = Vec::new();
        for word in line.split_whitespace() {
            words.push(String::from(word));
        }

        let cmd = words.remove(0);
        match cmd.to_lowercase().as_str() {
            "%{" => {
                state.in_def = true;
            }
            "%}" => {
                state.in_def = false;
            }
            "%%" => {
                state.section = LexSection::Rules;
            }
            "%s" | "%start" => {
                state.cond_start = words;
            }
            "%x" => {
                state.cond_xstart = words;
            }
            "%array" => {
                state.yyt_is_ptr = false;
            }
            "%pointer" => {
                state.yyt_is_ptr = true;
            }
            "%p" | "%n" | "%a" | "%e" | "%k" | "%o" => {
                // do nothing; skip these
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

fn parse_braces(open_braces: u32, line: &str) -> Result<u32, &'static str> {
    let mut open_braces = open_braces;
    for c in line.chars() {
        if c == '{' {
            open_braces += 1;
        } else if c == '}' {
            if open_braces == 0 {
                return Err("Unmatched closing brace");
            }
            open_braces -= 1;
        }
    }
    Ok(open_braces)
}

#[derive(PartialEq)]
enum RegexType {
    Square,
    Paren,
    Curly,
}

// find the end of the regex in a rule line, by matching [ and ( and { and }
fn find_ere_end(line: &str) -> Result<usize, &'static str> {
    let mut stack: Vec<RegexType> = Vec::new();
    let mut inside_brackets = false;

    eprintln!("find_ere_end: {}", line);

    for (i, ch) in line.chars().enumerate() {
        match ch {
            '[' => {
                if !inside_brackets {
                    stack.push(RegexType::Square);
                    inside_brackets = true;
                }
            }
            '(' => {
                if !inside_brackets {
                    stack.push(RegexType::Paren);
                }
            }
            '{' => {
                if !inside_brackets {
                    stack.push(RegexType::Curly);
                }
            }
            ']' => {
                inside_brackets = false;
                if stack.pop() != Some(RegexType::Square) {
                    return Err("Unmatched closing square bracket");
                }
            }
            ')' => {
                if !inside_brackets && stack.pop() != Some(RegexType::Paren) {
                    return Err("Unmatched closing parenthesis");
                }
            }
            '}' => {
                if !inside_brackets && stack.pop() != Some(RegexType::Curly) {
                    return Err("Unmatched closing curly brace");
                }
            }
            _ => {
                if ch.is_whitespace() && stack.is_empty() {
                    return Ok(i);
                }
            }
        }
    }

    Err("Unterminated regular expression")
}

fn parse_rule(line: &str) -> Result<(String, String, u32), &'static str> {
    let pos = find_ere_end(line)?;
    let ere = String::from(&line[..pos]);
    let action_ws = String::from(&line[pos..]);
    let action = action_ws.trim_start();
    let open_braces = parse_braces(0, action)?;

    Ok((ere.to_string(), action.to_string(), open_braces))
}

fn parse_rule_line(state: &mut ParseState, line: &str) -> Result<(), &'static str> {
    if line.len() == 0 {
        return Ok(());
    }

    let first_char = line.chars().next().unwrap();

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
                state.section = LexSection::UserCode;
            }
            _ => {
                eprintln!("Unexpected command in Rules section: {}", cmd);
            }
        }
    } else if state.open_braces > 0 {
        state.tmp_rule.action.push_str(line);
        state.open_braces = parse_braces(state.open_braces, line)?;
        if state.open_braces == 0 {
            let ere = state.tmp_rule.ere.clone();
            let action = state.tmp_rule.action.clone();
            state.push_rule(&ere, &action);
            state.tmp_rule = LexRule::new();
        }
    } else if state.in_def || (first_char.is_whitespace() && line.len() > 1) {
        state.internal_defs.push(String::from(line));
    } else if line.trim().is_empty() {
        return Ok(());
    } else {
        let (ere, action, open_braces) = parse_rule(line)?;
        if open_braces == 0 {
            state.push_rule(&ere, &action);
        } else {
            state.tmp_rule = LexRule { ere, action };
            state.open_braces = open_braces;
        }
    }
    Ok(())
}

fn parse_user_line(state: &mut ParseState, line: &str) -> Result<(), &'static str> {
    state.user_subs.push(String::from(line));
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
        let file: Box<dyn Read>;
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
