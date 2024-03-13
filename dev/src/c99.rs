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

mod libc99;

use clap::Parser;
use gettextrs::{bind_textdomain_codeset, textdomain};
use libc99::CStream;
use plib::PROJECT_NAME;
use std::fs;
use std::io;

/// c99 - compile standard C programs
#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    /// Suppress the link-edit phase of the compilation, and do not remove any object files that are produced.
    #[arg(short, long)]
    compile_only: bool,

    /// Files to process
    files: Vec<String>,
}

#[derive(Debug)]
struct BuildPlan {
    sources: Vec<String>,
    objs: Vec<String>,
}

impl BuildPlan {
    fn new() -> BuildPlan {
        BuildPlan {
            sources: Vec::new(),
            objs: Vec::new(),
        }
    }

    fn add_src(&mut self, filename: &str) {
        self.sources.push(String::from(filename));
    }

    fn add_obj(&mut self, filename: &str) {
        self.objs.push(String::from(filename));
    }
}

fn has_src_ext(filename: &str) -> bool {
    filename.ends_with(".c")
}

fn has_obj_ext(filename: &str) -> bool {
    filename.ends_with(".o")
}

fn c99_plan(_args: &Args, plan: &mut BuildPlan, filearg: &str) -> io::Result<()> {
    if has_src_ext(filearg) {
        plan.add_src(filearg);
    } else if has_obj_ext(filearg) {
        plan.add_obj(filearg);
    }

    println!("{:?}", plan);

    Ok(())
}

fn c99_build_one(_plan: &BuildPlan, filename: &String) -> io::Result<()> {
    // read input source file into buffer
    let bytes = fs::read(filename)?;

    let mut stream = CStream::from_buffer(0, bytes);
    stream.tokenize();

    Ok(())
}

fn c99_build(plan: BuildPlan) -> io::Result<()> {
    for src in &plan.sources {
        c99_build_one(&plan, src)?;
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // parse command line arguments
    let args = Args::parse();

    textdomain(PROJECT_NAME)?;
    bind_textdomain_codeset(PROJECT_NAME, "UTF-8")?;

    let mut plan = BuildPlan::new();
    let mut exit_code = 0;

    for filearg in &args.files {
        match c99_plan(&args, &mut plan, filearg) {
            Ok(()) => {}
            Err(e) => {
                exit_code = 1;
                eprintln!("{}: {}", filearg, e);
            }
        }
    }

    match c99_build(plan) {
        Ok(()) => {}
        Err(e) => {
            exit_code = 1;
            eprintln!("{}", e);
        }
    }

    std::process::exit(exit_code)
}
