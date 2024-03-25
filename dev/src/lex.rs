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
extern crate regex_syntax;

use clap::Parser;
use gettextrs::{bind_textdomain_codeset, textdomain};
use plib::PROJECT_NAME;
use regex::Regex;
use regex_syntax::hir::{self, Class, ClassUnicode, Hir};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Read};

const YYLVAR: &'static str = "yyl";
const YYLCONST: &'static str = "YYL";

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

    /// maximal compatibility with POSIX lex
    #[arg(short = 'X', long)]
    posix_compat: bool,

    /// Write output to this filename (unless superceded by -t).
    #[arg(short, long, default_value = "lex.yy.c")]
    outfile: String,

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
struct LexStateRule {
    ere: String,
    action: String,
    re: Hir,
}

// post-processed tables and other pre-computed data based on LexInfo input
#[derive(Debug)]
struct LexState {
    rules: Vec<LexStateRule>,
    classes: Vec<hir::Class>,
    literals: Vec<hir::Literal>,
}

impl LexState {
    fn new() -> LexState {
        LexState {
            rules: Vec::new(),
            classes: Vec::new(),
            literals: Vec::new(),
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

// parse line from Definitions section
fn parse_def_line(state: &mut ParseState, line: &str) -> Result<(), String> {
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
    } else if !line.trim().is_empty() {
        let msg = format!("Unexpected line in definitions section: {}", line);
        return Err(msg);
    }
    Ok(())
}

// parse continued action line, counting open braces
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

// translate lex-specific regex syntax to regex crate syntax
fn translate_ere(state: &mut ParseState, ere: &str) -> Result<String, String> {
    let mut re = String::new();
    let mut in_quotes = false;
    let mut in_sub = false;
    let mut sub_name = String::new();

    for ch in ere.chars() {
        if in_quotes && ch == '"' {
            in_quotes = false;
        } else if in_quotes {
            match ch {
                '*' => re.push_str(r"\x2a"),
                '+' => re.push_str(r"\x2b"),
                '.' => re.push_str(r"\x2e"),
                '{' => re.push_str(r"\x7b"),
                _ => re.push(ch),
            }
        } else if in_sub && ch == '}' {
            match state.subs.get(&sub_name) {
                Some(value) => re.push_str(value),
                None => {
                    let msg = format!("Unknown substitution: {}", sub_name);
                    return Err(msg);
                }
            }
            in_sub = false;
            sub_name.clear();
        } else if in_sub {
            sub_name.push(ch);
        } else if ch == '"' {
            in_quotes = true;
        } else if ch == '{' {
            in_sub = true;
        } else {
            re.push(ch);
        }
    }

    Ok(re)
}

// parse a lex rule line, returning the ERE and action
fn parse_rule(state: &mut ParseState, line: &str) -> Result<(String, String, u32), String> {
    let pos = find_ere_end(line)?;
    let ere = String::from(&line[..pos]);
    let ere = translate_ere(state, &ere)?;
    let action_ws = String::from(&line[pos..]);
    let action = action_ws.trim_start();
    let open_braces = parse_braces(0, action)?;

    Ok((ere.to_string(), action.to_string(), open_braces))
}

// parse line from Rules section
fn parse_rule_line(state: &mut ParseState, line: &str) -> Result<(), String> {
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
        let (ere, action, open_braces) = parse_rule(state, line)?;
        if open_braces == 0 {
            state.push_rule(&ere, &action);
        } else {
            state.tmp_rule = LexRule { ere, action };
            state.open_braces = open_braces;
        }
    }
    Ok(())
}

// parse line from UserCode section
fn parse_user_line(state: &mut ParseState, line: &str) -> Result<(), &'static str> {
    state.user_subs.push(String::from(line));
    Ok(())
}

// parse lex input, returning a LexInfo struct
fn parse_lex_input(input: &[String]) -> Result<LexInfo, String> {
    let mut state = ParseState::new();

    for line in input {
        match state.section {
            LexSection::Definitions => parse_def_line(&mut state, line)?,
            LexSection::Rules => parse_rule_line(&mut state, line)?,
            LexSection::UserCode => parse_user_line(&mut state, line)?,
        }
    }

    let lexinfo = LexInfo::from(&state);

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

fn gather_literals(hir: &Hir) -> Vec<hir::Literal> {
    let mut literals = Vec::new();

    match hir.kind() {
        hir::HirKind::Literal(literal) => {
            literals.push(literal.clone());
        }
        hir::HirKind::Concat(concat) => {
            for hir in concat.iter() {
                literals.append(&mut gather_literals(hir));
            }
        }
        hir::HirKind::Alternation(alt) => {
            for hir in alt.iter() {
                literals.append(&mut gather_literals(hir));
            }
        }
        hir::HirKind::Repetition(rep) => {
            literals.append(&mut gather_literals(&rep.sub));
        }
        _ => {}
    }

    literals
}

// add class to vec, with de-duplication
fn add_class(classes: &mut Vec<hir::Class>, class: hir::Class) {
    if !classes.contains(&class) {
        classes.push(class);
    }
}

// add a vec of classes to another vec, with de-duplication
fn add_classes(classes: &mut Vec<hir::Class>, new_classes: &Vec<hir::Class>) {
    for class in new_classes {
        add_class(classes, class.clone());
    }
}

fn gather_classes(hir: &Hir) -> Vec<hir::Class> {
    let mut classes = Vec::new();

    match hir.kind() {
        hir::HirKind::Class(class) => {
            add_class(&mut classes, class.clone());
        }
        hir::HirKind::Concat(concat) => {
            for hir in concat.iter() {
                add_classes(&mut classes, &gather_classes(hir));
            }
        }
        hir::HirKind::Alternation(alt) => {
            for hir in alt.iter() {
                add_classes(&mut classes, &gather_classes(hir));
            }
        }
        hir::HirKind::Repetition(rep) => {
            add_classes(&mut classes, &gather_classes(&rep.sub));
        }
        _ => {}
    }

    classes
}

// process LexInfo input, returning a LexState struct
fn process_lex_info(lexinfo: &LexInfo) -> Result<LexState, String> {
    let mut lexstate = LexState::new();

    for rule in &lexinfo.rules {
        let re = regex_syntax::parse(&rule.ere);
        if let Err(e) = re {
            eprintln!("ERE failed: {}", &rule.ere);
            return Err(e.to_string());
        } else {
            let re = re.unwrap();

            let mut classes = gather_classes(&re);
            lexstate.classes.append(&mut classes);

            let mut literals = gather_literals(&re);
            lexstate.literals.append(&mut literals);

            lexstate.rules.push(LexStateRule {
                ere: rule.ere.clone(),
                action: rule.action.clone(),
                re,
            });
        }
    }

    Ok(lexstate)
}

fn extract_ranges(cls: &ClassUnicode) -> Vec<(char, char)> {
    let mut ranges = Vec::new();

    for val in cls.iter() {
        ranges.push((val.start(), val.end()));
    }

    ranges
}

fn write_lexer_char_class_unicode(
    output: &mut Box<dyn io::Write>,
    i: usize,
    ranges: Vec<(char, char)>,
) -> io::Result<()> {
    writeln!(output, "/* Char class {} */", i)?;
    writeln!(
        output,
        r#"static bool {}_class_{}(char ch)
{{
	switch (ch) {{"#,
        YYLVAR, i
    )?;

    for (start, end) in ranges {
        if start == end {
            writeln!(output, "\t\tcase '{}':", start)?;
        } else {
            writeln!(output, "\t\tcase '{}' ... '{}':", start, end)?;
        }
    }

    writeln!(output, "\t\t\treturn true;")?;
    writeln!(output, "\t\tdefault:")?;
    writeln!(output, "\t\t\treturn false;")?;
    writeln!(output, "\t}}")?;
    writeln!(output, "}}\n")?;

    Ok(())
}

fn write_lexer_char_classes(
    output: &mut Box<dyn io::Write>,
    lexstate: &LexState,
    lexinfo: &LexInfo,
) -> io::Result<()> {
    writeln!(output, "/* Rules - char classes */")?;
    for (i, cls) in lexstate.classes.iter().enumerate() {
        match cls {
            Class::Unicode(val) => {
                let ranges = extract_ranges(val);
                write_lexer_char_class_unicode(output, i, ranges)?;
            }
            Class::Bytes(val) => {
                todo!();
            }

            _ => {}
        }
    }

    Ok(())
}

fn write_lexer_literals(
    output: &mut Box<dyn io::Write>,
    lexstate: &LexState,
    lexinfo: &LexInfo,
) -> io::Result<()> {
    writeln!(output, "/* Rules - literals */")?;
    let n_literals = lexstate.literals.len();
    writeln!(output, "#define {}_N_LITERALS {}", YYLCONST, n_literals)?;
    writeln!(
        output,
        r#"
static const struct {{
	const char *name;
	size_t len;
}} {}_literals[{}_N_LITERALS] = {{"#,
        YYLVAR, YYLCONST
    )?;

    for literal in &lexstate.literals {
        let bytes: Vec<u8> = literal.0.to_vec();
        let lit_str = String::from_utf8(bytes).unwrap();
        writeln!(output, r#"	{{ "{}", {} }},"#, lit_str, lit_str.len())?;
    }

    writeln!(output, "}};\n")?;

    Ok(())
}

fn write_lexer(args: &Args, lexinfo: &LexInfo, lexstate: &LexState) -> io::Result<()> {
    let mut output: Box<dyn io::Write>;

    if args.stdout {
        output = Box::new(io::stdout());
    } else {
        output = Box::new(fs::File::create(&args.outfile)?);
    }

    writeln!(output, "/* Generated by lex.rs */")?;
    writeln!(output, "/* External definitions */")?;
    for line in &lexinfo.external_def {
        write!(output, "{}", line)?;
    }

    writeln!(output, "/* Internal definitions */")?;
    for line in &lexinfo.internal_defs {
        write!(output, "{}", line)?;
    }

    writeln!(output, "/* Start conditions */")?;
    writeln!(output, "%s {}", lexinfo.cond_start.join(" "))?;
    writeln!(output, "%x {}", lexinfo.cond_xstart.join(" "))?;

    writeln!(output, "/* Rules */")?;

    write_lexer_char_classes(&mut output, lexstate, lexinfo)?;

    write_lexer_literals(&mut output, lexstate, lexinfo)?;

    writeln!(output, "/* Rules - table */")?;
    for rule in &lexinfo.rules {
        writeln!(output, "{} {}", rule.ere, rule.action)?;
    }

    writeln!(output, "/* User code */")?;
    for line in &lexinfo.user_subs {
        write!(output, "{}", line)?;
    }

    Ok(())
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

    // POSIX says multiple input files are concatenated
    let rawinput = concat_input_files(&args.files)?;

    // parse input lex file into a data structure containing the rules table
    let lexinfo = parse_lex_input(&rawinput)?;

    // calculate tables and other pre-computed data based on LexInfo input
    let lexstate = process_lex_info(&lexinfo)?;

    // write output to stdout or a file
    write_lexer(&args, &lexinfo, &lexstate)?;

    println!("PARSED_LEX {:#?}", lexinfo);
    println!("LEX_STATE {:#?}", lexstate);

    Ok(())
}
