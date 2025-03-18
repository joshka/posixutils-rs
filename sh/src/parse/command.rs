//
// Copyright (c) 2024 Hemi Labs, Inc.
//
// This file is part of the posixutils-rs project covered under
// the MIT License.  For the full license text, please see the LICENSE
// file in the root directory of this project.
// SPDX-License-Identifier: MIT
//

use crate::nonempty::NonEmpty;
use crate::parse::word::{Word, WordPair};
use std::fmt::{Debug, Display, Formatter, Write};
use std::rc::Rc;

pub type CompleteCommandList = Vec<CompleteCommand>;
pub type Name = Rc<str>;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum IORedirectionKind {
    // >
    RedirectOutput,
    // >|
    RedirectOutputClobber,
    // >>
    RedirectOuputAppend,
    // >&
    DuplicateOutput,
    // <
    RedirectInput,
    // <&
    DuplicateInput,
    // <>
    OpenRW,
}

impl Display for IORedirectionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IORedirectionKind::RedirectOutput => write!(f, ">"),
            IORedirectionKind::RedirectOutputClobber => write!(f, ">|"),
            IORedirectionKind::RedirectOuputAppend => write!(f, ">>"),
            IORedirectionKind::DuplicateOutput => write!(f, ">&"),
            IORedirectionKind::RedirectInput => write!(f, "<"),
            IORedirectionKind::DuplicateInput => write!(f, "<&"),
            IORedirectionKind::OpenRW => write!(f, "<>"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum RedirectionKind {
    IORedirection {
        kind: IORedirectionKind,
        file: WordPair,
    },
    HereDocument(Word),
    QuotedHereDocument(String),
}

impl Display for RedirectionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RedirectionKind::IORedirection { kind, file } => {
                write!(f, "{} {}", kind, file.as_string)
            }
            _ => todo!(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Redirection {
    pub file_descriptor: Option<u32>,
    pub kind: RedirectionKind,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Assignment {
    pub name: Name,
    pub value: WordPair,
}

impl Display for Assignment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}={}", self.name, self.value.as_string)
    }
}

#[derive(PartialEq, Default, Clone, Debug)]
pub struct SimpleCommand {
    pub assignments: Vec<Assignment>,
    pub redirections: Vec<Redirection>,
    pub words: Vec<WordPair>,
}

impl Display for SimpleCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for assignment in &self.assignments {
            write!(f, "{} ", assignment)?;
        }
        for word in &self.words {
            write!(f, "{} ", word.as_string)?;
        }
        for redirection in &self.redirections {
            write!(f, "{} ", redirection.kind)?;
        }
        Ok(())
    }
}

impl SimpleCommand {
    pub fn none_if_empty(self) -> Option<Self> {
        if self == Self::default() {
            None
        } else {
            Some(self)
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct CaseItem {
    pub pattern: NonEmpty<WordPair>,
    pub body: CompleteCommand,
}

impl Display for CaseItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}", self.pattern.first().as_string)?;
        for pattern in self.pattern.tail() {
            write!(f, " | {}", pattern.as_string)?;
        }
        write!(f, ")")?;
        write!(f, " {}", &self.body)?;
        write!(f, ";;")
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct If {
    pub condition: CompleteCommand,
    pub body: CompleteCommand,
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub enum CompoundCommand {
    BraceGroup(CompleteCommand),
    Subshell(CompleteCommand),
    ForClause {
        iter_var: Name,
        words: Vec<WordPair>,
        body: CompleteCommand,
    },
    CaseClause {
        arg: WordPair,
        cases: Vec<CaseItem>,
    },
    IfClause {
        if_chain: NonEmpty<If>,
        else_body: Option<CompleteCommand>,
    },
    WhileClause {
        condition: CompleteCommand,
        body: CompleteCommand,
    },
    UntilClause {
        condition: CompleteCommand,
        body: CompleteCommand,
    },
}

impl Display for CompoundCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CompoundCommand::BraceGroup(commands) => {
                write!(f, "{{ {} }}", commands)
            }
            CompoundCommand::Subshell(commands) => {
                write!(f, "({})", commands)
            }
            CompoundCommand::ForClause {
                iter_var,
                words,
                body,
            } => {
                write!(f, "for {} in", iter_var)?;
                for word in words {
                    write!(f, " {}", word.as_string)?;
                }
                write!(f, "; do {body} done")?;
                write!(f, "{}", body)?;
                writeln!(f, "; done")
            }
            CompoundCommand::CaseClause { arg, cases } => {
                write!(f, "case {} in", arg.as_string)?;
                for case in cases {
                    write!(f, " {}", case)?;
                }
                write!(f, " esac")
            }
            CompoundCommand::IfClause {
                if_chain,
                else_body,
            } => {
                write!(
                    f,
                    "if {} then {}",
                    if_chain.first().condition,
                    if_chain.first().body
                )?;
                for if_ in if_chain.tail() {
                    write!(f, "else if {} then {}", if_.condition, if_.body)?;
                }
                if let Some(else_body) = else_body {
                    write!(f, "else")?;
                    write!(f, "{}", else_body)?;
                }
                write!(f, "fi")
            }
            CompoundCommand::WhileClause { condition, body } => {
                write!(f, "while {} do {}", condition, body)?;
                write!(f, "{}", body)?;
                write!(f, "done")
            }
            CompoundCommand::UntilClause { condition, body } => {
                write!(f, "until {} do {}", condition, body)?;
                write!(f, "{}", body)?;
                write!(f, "done")
            }
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct FunctionDefinition {
    pub name: Name,
    pub body: Rc<CompoundCommand>,
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub enum CommandType {
    FunctionDefinition(FunctionDefinition),
    SimpleCommand(SimpleCommand),
    CompoundCommand {
        command: CompoundCommand,
        redirections: Vec<Redirection>,
    },
}

impl Display for CommandType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandType::FunctionDefinition(func) => {
                write!(f, "{} () {}", func.name, func.body)
            }
            CommandType::SimpleCommand(cmd) => {
                write!(f, "{}", cmd)
            }
            CommandType::CompoundCommand {
                command,
                redirections,
            } => {
                write!(f, "{}", command)?;
                for redirection in redirections {
                    write!(f, "{}", redirection.kind)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Command {
    pub type_: CommandType,
    pub lineno: u32,
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.type_)
    }
}

impl Command {
    pub fn new(type_: CommandType, lineno: u32) -> Self {
        Self { type_, lineno }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Pipeline {
    pub commands: NonEmpty<Command>,
    pub negate_status: bool,
}

impl Display for Pipeline {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.commands.first())?;
        for command in self.commands.tail() {
            write!(f, " | {}", command)?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LogicalOp {
    And,
    Or,
    None,
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Conjunction {
    pub elements: NonEmpty<(Pipeline, LogicalOp)>,
    pub is_async: bool,
}

impl Conjunction {
    fn write_into_string(&self, string: &mut String, print_semicolon: bool) {
        for (pipeline, op) in &self.elements {
            match op {
                LogicalOp::And => write!(string, "{} && ", pipeline).unwrap(),
                LogicalOp::Or => write!(string, "{} || ", pipeline).unwrap(),
                LogicalOp::None => write!(string, "{}", pipeline).unwrap(),
            }
        }
        if self.is_async {
            string.push('&');
        } else if print_semicolon {
            string.push(';');
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct CompleteCommand {
    pub commands: NonEmpty<Conjunction>,
}

impl CompleteCommand {
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        for conjunction in &self.commands {
            conjunction.write_into_string(&mut result, true);
        }
        result
    }
}

impl Display for CompleteCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl From<CommandType> for Command {
        fn from(value: CommandType) -> Self {
            Command::new(value, 0)
        }
    }

    impl PartialEq for Command {
        fn eq(&self, other: &Self) -> bool {
            self.type_ == other.type_
        }
    }
}
