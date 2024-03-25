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

mod lexfile;

use clap::Parser;
use gettextrs::{bind_textdomain_codeset, textdomain};
use lexfile::LexInfo;
use plib::PROJECT_NAME;
use regex_syntax::hir::{self, Class, ClassUnicode, Hir};
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
) -> io::Result<()> {
    writeln!(output, "/* Rules - char classes */")?;
    for (i, cls) in lexstate.classes.iter().enumerate() {
        match cls {
            Class::Unicode(val) => {
                let ranges = extract_ranges(val);
                write_lexer_char_class_unicode(output, i, ranges)?;
            }
            Class::Bytes(_val) => {
                todo!();
            }
        }
    }

    Ok(())
}

fn write_lexer_literals(output: &mut Box<dyn io::Write>, lexstate: &LexState) -> io::Result<()> {
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

    write_lexer_char_classes(&mut output, lexstate)?;

    write_lexer_literals(&mut output, lexstate)?;

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
    let lexinfo = lexfile::parse(&rawinput)?;

    // calculate tables and other pre-computed data based on LexInfo input
    let lexstate = process_lex_info(&lexinfo)?;

    // write output to stdout or a file
    write_lexer(&args, &lexinfo, &lexstate)?;

    println!("PARSED_LEX {:#?}", lexinfo);
    println!("LEX_STATE {:#?}", lexstate);

    Ok(())
}
