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
use std::fs;
use std::io::{self, Read};

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

// concatenate input files, handling special filename "-" as stdin
fn concat_input_files(files: &[String]) -> io::Result<String> {
    let mut input = String::new();
    for file in files {
        if file == "-" {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            input.push_str(&buf);
        } else {
            let tmpstr = fs::read_to_string(file)?;
            input.push_str(&tmpstr);
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

    let _input = concat_input_files(&args.files)?;

    Ok(())
}
