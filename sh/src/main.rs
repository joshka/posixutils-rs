//
// Copyright (c) 2024 Hemi Labs, Inc.
//
// This file is part of the posixutils-rs project covered under
// the MIT License.  For the full license text, please see the LICENSE
// file in the root directory of this project.
// SPDX-License-Identifier: MIT
//

use crate::builtin::trap::{TrapAction, TrapCondition};
use crate::cli::{parse_args, ExecutionMode};
use crate::shell::Shell;
use atty::Stream;
use nix::libc;
use nix::sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal};
use std::io;

mod builtin;
mod cli;
mod parse;
mod program;
mod shell;
mod utils;
mod wordexp;

static mut GLOBAL_SHELL: Option<Shell> = None;

fn get_global_shell() -> &'static mut Shell {
    unsafe { GLOBAL_SHELL.as_mut().unwrap() }
}

fn execute_action(condition: TrapCondition) {
    if let TrapAction::Commands(commands) = &get_global_shell().trap_actions[condition as usize] {
        match get_global_shell().execute_program(commands) {
            Err(err) => {
                eprintln!("sh: error parsing action: {}", err.message);
            }
            Ok(_) => {}
        }
    }
}

extern "C" fn on_exit() {
    execute_action(TrapCondition::Exit);
}

pub extern "C" fn global_shell_signal_handler(signal: libc::c_int) {
    execute_action(signal.try_into().expect("invalid signal"));
}

fn execute_string(string: &str, shell: &mut Shell) {
    match shell.execute_program(string) {
        Ok(_) => {}
        Err(syntax_err) => {
            eprintln!(
                "sh({}): syntax error: {}",
                syntax_err.lineno, syntax_err.message
            );
            // both bash and sh use 2 as the exit code for a syntax error
            std::process::exit(2);
        }
    }
}

fn main() {
    let is_attached_to_terminal = atty::is(Stream::Stdin) && atty::is(Stream::Stdout);
    let args = parse_args(std::env::args().collect(), is_attached_to_terminal).unwrap();
    unsafe {
        GLOBAL_SHELL = Some(Shell::initialize_from_system(
            args.program_name,
            args.arguments,
            args.set_options,
            args.execution_mode == ExecutionMode::Interactive,
        ))
    };
    unsafe { libc::atexit(on_exit) };
    match args.execution_mode {
        ExecutionMode::Interactive => {
            let mut buffer = String::new();
            eprint!("{}", get_global_shell().get_ps1());
            while io::stdin().read_line(&mut buffer).is_ok_and(|n| n > 0) {
                if buffer.ends_with("\\\n") {
                    continue;
                }
                match get_global_shell().execute_program(&buffer) {
                    Ok(_) => {
                        buffer.clear();
                        eprint!("{}", get_global_shell().get_ps1());
                    }
                    Err(syntax_err) => {
                        if !syntax_err.could_be_resolved_with_more_input {
                            eprintln!("sh: syntax error: {}", syntax_err.message);
                            buffer.clear();
                            eprint!("{}", get_global_shell().get_ps1());
                        } else {
                            eprint!("{}", get_global_shell().get_ps2());
                        }
                    }
                }
            }
        }
        ExecutionMode::ReadCommandsFromStdin => {
            let mut buffer = String::new();
            while io::stdin().read_line(&mut buffer).is_ok_and(|n| n > 0) {
                if buffer.ends_with("\\\n") {
                    continue;
                }
                match get_global_shell().execute_program(&buffer) {
                    Ok(_) => {
                        buffer.clear();
                    }
                    Err(syntax_err) => {
                        if !syntax_err.could_be_resolved_with_more_input {
                            eprintln!(
                                "sh({}): syntax error: {}",
                                syntax_err.lineno, syntax_err.message
                            );
                            std::process::exit(2);
                        }
                    }
                }
            }
        }
        other => match other {
            ExecutionMode::ReadCommandsFromString(command_string) => {
                execute_string(&command_string, get_global_shell());
            }
            ExecutionMode::ReadFromFile(file) => {
                let file_contents = std::fs::read_to_string(file).expect("could not read file");
                execute_string(&file_contents, get_global_shell());
            }
            _ => unreachable!(),
        },
    }
    std::process::exit(get_global_shell().last_pipeline_exit_status);
}
