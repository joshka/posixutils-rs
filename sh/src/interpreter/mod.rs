use crate::interpreter::wordexp::{expand_word, expand_word_to_string};
use crate::program::{
    Assignment, Command, CompleteCommand, CompoundCommand, Conjunction, IORedirectionKind,
    LogicalOp, Name, Pipeline, Program, Redirection, RedirectionKind, SimpleCommand,
};
use std::collections::HashMap;
use std::ffi::{c_char, CString, OsString};
use std::os::fd::{AsRawFd, IntoRawFd};
use std::path::PathBuf;
use std::rc::Rc;

mod wordexp;

trait BuiltinUtility {
    fn exec(&self);
}

fn get_special_builtin_utility(name: &str) -> Option<&dyn BuiltinUtility> {
    match name {
        _ => None,
    }
}

fn get_bultin_utility(name: &str) -> Option<&dyn BuiltinUtility> {
    match name {
        _ => None,
    }
}

#[derive(Clone)]
struct Variable {
    value: String,
    export: bool,
}

impl Variable {
    fn new_exported(value: String) -> Self {
        Variable {
            value,
            export: true,
        }
    }

    fn new(value: String) -> Self {
        Variable {
            value,
            export: false,
        }
    }
}

fn find_in_path(command: &str, env_path: &str) -> Option<String> {
    for path in env_path.split(':') {
        let mut command_path = PathBuf::from(path);
        command_path.push(command);
        if command_path.is_file() {
            return Some(command_path.into_os_string().to_string_lossy().into());
        }
    }
    None
}

pub type Environment = HashMap<String, Variable>;

#[derive(Clone)]
pub struct Interpreter {
    environment: Environment,
    opened_files: HashMap<u32, Rc<std::fs::File>>,
    functions: HashMap<Name, Rc<CompoundCommand>>,
    most_recent_pipeline_status: i32,
    last_command_substitution_status: i32,
    shell_pid: i32,
    most_recent_background_command_pid: i32,
    current_directory: OsString,
}

impl Interpreter {
    fn exec(&self, command: &str, args: &[String]) -> i32 {
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            todo!("error: fork failed")
        } else if pid == 0 {
            // child
            for (id, file) in &self.opened_files {
                let dest = *id as i32;
                let src = file.as_raw_fd();
                unsafe { libc::dup2(src, dest) };
            }

            let command = CString::new(command).unwrap();
            let args = args
                .iter()
                .map(|s| CString::new(s.as_str()).unwrap())
                .collect::<Vec<_>>();
            let args = std::iter::once(command.as_ptr())
                .chain(args.iter().map(|s| s.as_ptr()))
                .chain(std::iter::once(std::ptr::null() as *const c_char))
                .collect::<Vec<_>>();

            let env = self
                .environment
                .iter()
                .filter_map(|(name, value)| {
                    if value.export {
                        // TODO: look into this unwrap
                        Some(CString::new(format!("{name}={}", value.value)).unwrap())
                    } else {
                        None
                    }
                })
                .collect::<Vec<CString>>();
            let env = env
                .iter()
                .map(|s| s.as_ptr())
                .chain(std::iter::once(std::ptr::null()))
                .collect::<Vec<_>>();
            unsafe { libc::execve(command.as_ptr(), args.as_ptr(), env.as_ptr()) }
        } else {
            // parent
            let mut status = 0;
            let wait_result = unsafe { libc::waitpid(pid, &mut status, 0) };
            if wait_result != pid {
                panic!("failed to wait for child process");
            }
            libc::WEXITSTATUS(status)
        }
    }

    fn perform_redirections(&mut self, redirections: &[Redirection]) {
        for redir in redirections {
            match &redir.kind {
                RedirectionKind::IORedirection { kind, file } => {
                    // > the word that follows the redirection operator shall be subjected to tilde
                    // > expansion, parameter expansion, command substitution, arithmetic expansion,
                    // > and quote removal.
                    let path = expand_word_to_string(file, false, self);
                    // TODO: pathname expansion is not allowed if the shell is non-interactive,
                    // optional otherwise. Bash does implement this, maybe we should too.
                    match kind {
                        IORedirectionKind::RedirectOutput
                        | IORedirectionKind::RedirectOutputClobber
                        | IORedirectionKind::RedirectOuputAppend => {
                            // TODO: fail if noclobber is set and file exists and is a regular file

                            // TODO: fix unwrap
                            let file = if *kind == IORedirectionKind::RedirectOuputAppend {
                                std::fs::OpenOptions::new()
                                    .append(true)
                                    .create(true)
                                    .open(path)
                                    .unwrap()
                            } else {
                                std::fs::File::create(path).unwrap()
                            };
                            let source_fd =
                                redir.file_descriptor.unwrap_or(libc::STDOUT_FILENO as u32);
                            self.opened_files.insert(source_fd, Rc::new(file));
                        }
                        IORedirectionKind::DuplicateOutput => {}
                        IORedirectionKind::RedirectInput => {}
                        IORedirectionKind::DuplicateInput => {}
                        IORedirectionKind::OpenRW => {}
                    }
                }
                RedirectionKind::HereDocument { contents } => {}
            }
        }
    }

    fn perform_assignments(&mut self, assignments: &[Assignment]) {
        for assignment in assignments {
            let word_str = expand_word_to_string(&assignment.value, true, self);
            self.environment
                .insert(assignment.name.to_string(), Variable::new(word_str));
        }
    }

    fn interpret_simple_command(&mut self, simple_command: &SimpleCommand) -> i32 {
        let mut expanded_words = Vec::new();
        // reset
        self.last_command_substitution_status = 0;
        for word in &simple_command.words {
            expanded_words.extend(expand_word(word, false, self));
        }
        if expanded_words.is_empty() {
            // no commands to execute, perform assignments and redirections
            self.perform_assignments(&simple_command.assignments);
            if !simple_command.redirections.is_empty() {
                let mut subshell = self.clone();
                subshell.perform_redirections(&simple_command.redirections);
            }
            return self.last_command_substitution_status;
        }

        if expanded_words[0].contains('/') {
            let mut command_environment = self.clone();
            command_environment.perform_assignments(&simple_command.assignments);
            command_environment.perform_redirections(&simple_command.redirections);
            let command = &expanded_words[0];
            let arguments = expanded_words[1..]
                .iter()
                .map(|w| w.clone())
                .collect::<Vec<String>>();
            command_environment.exec(&command, &arguments)
        } else {
            if let Some(_special_builtin_utility) = get_special_builtin_utility(&expanded_words[0])
            {
                self.perform_assignments(&simple_command.assignments);
                todo!()
            }

            if let Some(_function_body) = self.functions.get(expanded_words[0].as_str()) {
                self.perform_assignments(&simple_command.assignments);
                todo!()
            }

            if let Some(_builtin_utility) = get_bultin_utility(&expanded_words[0]) {
                todo!()
            }

            let mut command_environment = self.clone();
            command_environment.perform_assignments(&simple_command.assignments);
            command_environment.perform_redirections(&simple_command.redirections);
            if let Some(command) = find_in_path(
                &expanded_words[0],
                &self.environment.get("PATH").unwrap().value,
            ) {
                let arguments = expanded_words[1..]
                    .iter()
                    .map(|w| w.clone())
                    .collect::<Vec<String>>();
                command_environment.exec(&command, &arguments)
            } else {
                eprintln!("{}: command not found", expanded_words[0]);
                127
            }
        }
    }

    fn interpret_command(&mut self, command: &Command) -> i32 {
        match command {
            Command::SimpleCommand(simple_command) => self.interpret_simple_command(simple_command),
            Command::CompoundCommand { .. } => {
                todo!()
            }
            _ => todo!("not implemented"),
        }
    }

    fn interpret_pipeline(&mut self, pipeline: &Pipeline) -> i32 {
        let pipeline_exit_status;
        if pipeline.commands.len() == 1 {
            let command = &pipeline.commands[0];
            pipeline_exit_status = self.interpret_command(command);
        } else {
            let mut current_stdin = libc::STDIN_FILENO;
            for command in pipeline.commands.iter().take(pipeline.commands.len() - 1) {
                let mut pipe: [libc::c_int; 2] = [0, 0];
                if unsafe { libc::pipe(pipe.as_mut_ptr()) } == -1 {
                    todo!("handle error");
                }
                let pid = unsafe { libc::fork() };

                if pid < 0 {
                    todo!("failed to fork")
                }
                if pid == 0 {
                    unsafe { libc::close(pipe[0]) };
                    unsafe { libc::dup2(current_stdin, libc::STDIN_FILENO) };
                    unsafe { libc::dup2(pipe[1], libc::STDOUT_FILENO) };
                    let return_status = self.interpret_command(command);
                    if current_stdin != 0 {
                        unsafe { libc::close(current_stdin) };
                    }
                    unsafe { libc::close(pipe[1]) };
                    std::process::exit(return_status);
                }
                if current_stdin != 0 {
                    unsafe { libc::close(current_stdin) };
                }
                unsafe { libc::close(pipe[1]) };
                current_stdin = pipe[0];
            }
            let last_command_pid = unsafe { libc::fork() };

            if last_command_pid < 0 {
                todo!("failed to fork")
            } else if last_command_pid == 0 {
                unsafe { libc::dup2(current_stdin, libc::STDIN_FILENO) };
                let return_status = self.interpret_command(pipeline.commands.last().unwrap());
                unsafe { libc::close(current_stdin) };
                std::process::exit(return_status);
            }
            unsafe { libc::close(current_stdin) };

            let mut wait_status = 0;
            let wait_result = unsafe { libc::waitpid(last_command_pid, &mut wait_status, 0) };
            if wait_result != last_command_pid {
                panic!("failed to wait for child process");
            }
            pipeline_exit_status = libc::WEXITSTATUS(wait_status);
        }
        if pipeline.negate_status {
            (pipeline_exit_status == 0) as i32
        } else {
            pipeline_exit_status
        }
    }

    fn interpret_conjunction(&mut self, conjunction: &Conjunction) -> i32 {
        let mut status = 0;
        for (pipeline, op) in &conjunction.elements {
            status = self.interpret_pipeline(pipeline);
            if status != 0 && *op == LogicalOp::And {
                break;
            } else if status == 0 && *op == LogicalOp::Or {
                break;
            }
        }
        status
    }

    fn interpret_complete_command(&mut self, command: &CompleteCommand) {
        for conjunction in &command.commands {
            self.interpret_conjunction(conjunction);
        }
    }

    pub fn interpret(&mut self, program: Program) {
        for command in &program.commands {
            self.interpret_complete_command(command);
        }
    }

    pub fn initialize_from_system() -> Interpreter {
        // > If a variable is initialized from the environment, it shall be marked for
        // > export immediately
        let variables = std::env::vars()
            .into_iter()
            .map(|(k, v)| (k, Variable::new_exported(v)))
            .collect();
        let pid = unsafe { libc::getpid() };
        Interpreter {
            environment: variables,
            shell_pid: pid,
            // TODO: handle error
            current_directory: std::env::current_dir().unwrap().into_os_string(),
            ..Default::default()
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Interpreter {
            environment: Environment::default(),
            opened_files: HashMap::default(),
            functions: HashMap::default(),
            most_recent_pipeline_status: 0,
            last_command_substitution_status: 0,
            shell_pid: 0,
            most_recent_background_command_pid: 0,
            current_directory: OsString::from("/"),
        }
    }
}
