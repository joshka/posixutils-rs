//
// Copyright (c) 2024 Hemi Labs, Inc.
//
// This file is part of the posixutils-rs project covered under
// the MIT License.  For the full license text, please see the LICENSE
// file in the root directory of this project.
// SPDX-License-Identifier: MIT
//

use crate::cli::{parse_args, ExecutionMode};
use crate::shell::{ExecutionError, Shell};
use atty::Stream;
use std::io;

mod builtin;
mod cli;
mod parse;
mod program;
mod shell;
mod utils;
mod wordexp;

fn execute_string(string: &str, shell: &mut Shell) {
    match shell.execute_program(string) {
        Ok(_) => {}
        Err(ExecutionError::ParserError(err)) => {
            eprintln!("{}", err.message);
            // both bash and sh use 2 as the exit code for a syntax error
            std::process::exit(2);
        }
    }
}

fn main() {
    let is_attached_to_terminal = atty::is(Stream::Stdin) && atty::is(Stream::Stdout);
    let args = parse_args(std::env::args().collect(), is_attached_to_terminal).unwrap();
    let mut shell =
        Shell::initialize_from_system(args.program_name, args.arguments, args.set_options);
    match args.execution_mode {
        ExecutionMode::Interactive | ExecutionMode::ReadCommandsFromStdin => {
            let mut buffer = String::new();
            let stdin = io::stdin();
            while stdin.read_line(&mut buffer).is_ok_and(|n| n > 0) {
                if buffer.ends_with("\\\n") {
                    continue;
                }
                match shell.execute_program(&buffer) {
                    Ok(_) => {
                        buffer.clear();
                    }
                    Err(ExecutionError::ParserError(err)) => {
                        if !err.could_be_resolved_with_more_input {
                            eprintln!("{}", err.message);
                            if args.execution_mode != ExecutionMode::Interactive {
                                std::process::exit(2);
                            }
                        }
                    }
                }
            }
        }
        other => {
            match other {
                ExecutionMode::ReadCommandsFromString(command_string) => {
                    execute_string(&command_string, &mut shell);
                }
                ExecutionMode::ReadFromFile(file) => {
                    let file_contents = std::fs::read_to_string(file).expect("could not read file");
                    execute_string(&file_contents, &mut shell);
                }
                _ => unreachable!(),
            }
        }
    }
    std::process::exit(shell.most_recent_pipeline_exit_status);
}
