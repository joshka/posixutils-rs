//
// Copyright (c) 2024 Jeff Garzik
//
// This file is part of the posixutils-rs project covered under
// the MIT License.  For the full license text, please see the LICENSE
// file in the root directory of this project.
// SPDX-License-Identifier: MIT
//

use regex::Regex;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct LexRule {
    pub ere: String,
    pub action: String,
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
pub struct LexInfo {
    pub external_def: Vec<String>,
    pub subs: HashMap<String, String>,
    pub internal_defs: Vec<String>,
    pub cond_start: Vec<String>,
    pub cond_xstart: Vec<String>,
    pub yyt_is_ptr: bool,
    pub user_subs: Vec<String>,
    pub rules: Vec<LexRule>,
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
pub fn parse(input: &[String]) -> Result<LexInfo, String> {
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
