use crate::interpreter::wordexp::parameter::expand_parameter_into;
use crate::interpreter::wordexp::pathname::glob;
use crate::interpreter::wordexp::pattern::FilenamePattern;
use crate::interpreter::wordexp::tilde::tilde_expansion;
use crate::interpreter::Interpreter;
use crate::program::{Word, WordPart};
use std::path::Path;

mod parameter;
pub mod pathname;
pub mod pattern;
mod tilde;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpandedWordPart {
    QuotedLiteral(String),
    UnquotedLiteral(String),
    GeneratedUnquotedLiteral(String),
    // terminates a field
    FieldEnd,
}

impl ExpandedWordPart {
    fn new(value: String, quoted: bool, generated: bool) -> Self {
        if quoted {
            ExpandedWordPart::QuotedLiteral(value)
        } else if generated {
            ExpandedWordPart::GeneratedUnquotedLiteral(value)
        } else {
            ExpandedWordPart::UnquotedLiteral(value)
        }
    }
}

/// Word that has undergone:
/// - tilde expansion
/// - parameter expansion
/// - command substitution
/// - arithmetic expansion
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct ExpandedWord {
    parts: Vec<ExpandedWordPart>,
}

impl From<ExpandedWord> for String {
    fn from(value: ExpandedWord) -> Self {
        value.to_string()
    }
}

impl ToString for ExpandedWord {
    fn to_string(&self) -> String {
        self.parts
            .iter()
            .map(|p| match p {
                ExpandedWordPart::UnquotedLiteral(s) => s.as_str(),
                ExpandedWordPart::QuotedLiteral(s) => s.as_str(),
                ExpandedWordPart::GeneratedUnquotedLiteral(s) => s.as_str(),
                ExpandedWordPart::FieldEnd => "",
            })
            .collect()
    }
}

impl ExpandedWord {
    /// joins adjacent parts that are the same
    fn normalize(self) -> Self {
        if self.parts.is_empty() {
            return Self { parts: Vec::new() };
        }
        let mut word_iter = self.parts.into_iter();
        let mut merged_parts = vec![word_iter.next().unwrap()];
        for part in word_iter {
            match (part, merged_parts.last_mut().unwrap()) {
                (
                    ExpandedWordPart::UnquotedLiteral(lit),
                    ExpandedWordPart::UnquotedLiteral(dest),
                ) => dest.push_str(&lit),
                (
                    ExpandedWordPart::GeneratedUnquotedLiteral(lit),
                    ExpandedWordPart::GeneratedUnquotedLiteral(dest),
                ) => dest.push_str(&lit),
                (ExpandedWordPart::QuotedLiteral(lit), ExpandedWordPart::QuotedLiteral(dest)) => {
                    dest.push_str(&lit)
                }
                (part, _) => merged_parts.push(part),
            }
        }
        Self {
            parts: merged_parts,
        }
    }

    fn append<S: AsRef<str> + Into<String>>(&mut self, value: S, quoted: bool, generated: bool) {
        if let Some(last) = self.parts.last_mut() {
            match last {
                ExpandedWordPart::GeneratedUnquotedLiteral(last) if generated && !quoted => {
                    last.push_str(value.as_ref());
                }
                ExpandedWordPart::UnquotedLiteral(last) if !generated && !quoted => {
                    last.push_str(value.as_ref())
                }
                ExpandedWordPart::QuotedLiteral(last) if quoted => {
                    last.push_str(value.as_ref());
                }
                _ => self
                    .parts
                    .push(ExpandedWordPart::new(value.into(), quoted, generated)),
            }
        } else {
            self.parts
                .push(ExpandedWordPart::new(value.into(), quoted, generated));
        }
    }

    fn end_field(&mut self) {
        self.parts.push(ExpandedWordPart::FieldEnd);
    }

    fn extend(&mut self, mut other: Self) {
        self.parts.reserve(other.parts.len());
        let mut iter = other.parts.into_iter();
        if let Some(first) = iter.next() {
            if self.parts.is_empty() {
                self.parts.push(first);
            } else {
                match (first, self.parts.last_mut().unwrap()) {
                    (
                        ExpandedWordPart::UnquotedLiteral(lit),
                        ExpandedWordPart::UnquotedLiteral(dest),
                    ) => dest.push_str(&lit),
                    (
                        ExpandedWordPart::GeneratedUnquotedLiteral(lit),
                        ExpandedWordPart::GeneratedUnquotedLiteral(dest),
                    ) => dest.push_str(&lit),
                    (
                        ExpandedWordPart::QuotedLiteral(lit),
                        ExpandedWordPart::QuotedLiteral(dest),
                    ) => dest.push_str(&lit),
                    (part, _) => self.parts.push(part),
                }
            }
        }
        self.parts.extend(iter);
    }
}

fn is_ifs_whitespace(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\n'
}

fn split_fields(expanded_word: ExpandedWord, ifs: Option<&str>) -> Vec<ExpandedWord> {
    // TODO: look into "Note that the shell processes arbitrary bytes from the input fields;
    // there is no requirement that those bytes form valid characters."
    if expanded_word.parts.is_empty() {
        return Vec::new();
    }
    // > If the IFS variable is unset [..], its value shall be considered to contain the three
    // > single-byte characters <space>, <tab>, and <newline>
    let ifs = ifs.unwrap_or(" \t\n");
    if ifs.is_empty() {
        // > If the IFS variable is set and has an empty string as its value, no field
        // > splitting shall occur
        return vec![expanded_word];
    }

    let normalized_word = expanded_word.normalize();

    let mut result = Vec::with_capacity(normalized_word.parts.len());
    let mut parts_for_last_word = Vec::new();
    for part in normalized_word.parts.into_iter() {
        match part {
            // > Fields which contain no results from expansions shall not be affected by
            // > field splitting, and shall remain unaltered, simply moving from the list
            // > of input fields to be next in the list of output fields.
            ExpandedWordPart::UnquotedLiteral(lit) => {
                parts_for_last_word.push(ExpandedWordPart::UnquotedLiteral(lit));
            }
            ExpandedWordPart::QuotedLiteral(lit) => {
                parts_for_last_word.push(ExpandedWordPart::QuotedLiteral(lit));
            }
            ExpandedWordPart::FieldEnd => {
                result.push(
                    ExpandedWord {
                        parts: parts_for_last_word,
                    }
                    .normalize(),
                );
                parts_for_last_word = Vec::new();
            }
            ExpandedWordPart::GeneratedUnquotedLiteral(lit) => {
                if lit.is_empty() {
                    continue;
                }
                let mut accumulator = String::new();
                // TODO: this could probably be done more cleanly
                let mut iter = lit.chars();
                let mut next_char = iter.next().unwrap();
                'outer: loop {
                    if ifs.contains(next_char) {
                        if is_ifs_whitespace(next_char) {
                            loop {
                                match iter.next() {
                                    Some(c) => {
                                        next_char = c;
                                        if !is_ifs_whitespace(next_char) {
                                            break;
                                        }
                                    }
                                    None => break 'outer,
                                }
                            }
                        } else {
                            if let Some(c) = iter.next() {
                                next_char = c;
                            } else {
                                break;
                            }
                        }
                        if !accumulator.is_empty() {
                            parts_for_last_word
                                .push(ExpandedWordPart::UnquotedLiteral(accumulator));
                            accumulator = String::new();
                            result.push(
                                ExpandedWord {
                                    parts: parts_for_last_word,
                                }
                                .normalize(),
                            );
                            parts_for_last_word = Vec::new();
                        }
                    } else {
                        accumulator.push(next_char);
                        if let Some(c) = iter.next() {
                            next_char = c;
                        } else {
                            break;
                        }
                    }
                }
                if !accumulator.is_empty() {
                    parts_for_last_word.push(ExpandedWordPart::UnquotedLiteral(accumulator));
                }
            }
        }
    }
    if !parts_for_last_word.is_empty() {
        result.push(
            ExpandedWord {
                parts: parts_for_last_word,
            }
            .normalize(),
        );
    }
    result
}

/// performs:
/// - tilde expansion
/// - parameter expansion
/// - command substitution
/// - arithmetic expansion
fn simple_word_expansion_into(
    result: &mut ExpandedWord,
    word: &Word,
    is_assignment: bool,
    interpreter: &mut Interpreter,
) {
    let mut word = word.clone();
    tilde_expansion(&mut word, is_assignment, &interpreter.environment);
    for part in word.parts.into_iter() {
        match part {
            WordPart::UnquotedLiteral(lit) => result.append(lit, false, false),
            WordPart::QuotedLiteral(lit) => result.append(lit, true, false),
            WordPart::ParameterExpansion {
                expansion,
                inside_double_quotes,
            } => {
                expand_parameter_into(result, &expansion, inside_double_quotes, true, interpreter);
            }
            WordPart::ArithmeticExpansion(_) => {
                todo!()
            }
            WordPart::CommandSubstitution {
                command,
                inside_double_quotes,
            } => {
                // > If a command substitution occurs inside double-quotes, field splitting
                // > and pathname expansion shall not be performed on the results of
                // > the substitution.
                if inside_double_quotes {
                    todo!()
                } else {
                    todo!()
                }
            }
        }
    }
}

/// performs:
/// - tilde expansion
/// - parameter expansion
/// - command substitution
/// - arithmetic expansion
pub fn expand_word_to_string(
    word: &Word,
    is_assignment: bool,
    interpreter: &mut Interpreter,
) -> String {
    let mut expanded_word = ExpandedWord::default();
    simple_word_expansion_into(&mut expanded_word, word, is_assignment, interpreter);
    expanded_word.to_string()
}

/// performs general word expansion (similar to `wordexp` from libc)
pub fn expand_word(word: &Word, is_assignment: bool, interpreter: &mut Interpreter) -> Vec<String> {
    let mut expanded_word = ExpandedWord::default();
    simple_word_expansion_into(&mut expanded_word, word, is_assignment, interpreter);
    let ifs = interpreter.environment.get("IFS").map(|v| v.value.as_str());
    let mut result = Vec::new();
    for field in split_fields(expanded_word, ifs) {
        // TODO: handle error
        let pattern = FilenamePattern::new(&field).unwrap();
        let files = glob(&pattern, Path::new(&interpreter.current_directory));
        if files.is_empty() {
            result.push(pattern.into())
        } else {
            // TODO: handle error
            result.extend(files.into_iter().map(|s| s.into_string().unwrap()))
        }
    }
    result
}

#[cfg(test)]
pub mod tests {
    use super::*;

    impl ExpandedWord {
        pub fn unquoted_literal(s: &str) -> Self {
            Self {
                parts: vec![ExpandedWordPart::UnquotedLiteral(s.to_string())],
            }
        }

        pub fn quoted_literal(s: &str) -> Self {
            Self {
                parts: vec![ExpandedWordPart::QuotedLiteral(s.to_string())],
            }
        }

        pub fn generated_unquoted_literal(s: &str) -> Self {
            Self {
                parts: vec![ExpandedWordPart::GeneratedUnquotedLiteral(s.to_string())],
            }
        }

        pub fn from_parts(parts: Vec<ExpandedWordPart>) -> Self {
            Self { parts }
        }
    }

    #[test]
    fn split_fields_on_empty_literal() {
        assert_eq!(
            split_fields(
                ExpandedWord {
                    parts: vec![ExpandedWordPart::GeneratedUnquotedLiteral("".to_string())]
                },
                None
            ),
            Vec::<ExpandedWord>::new()
        );
    }

    #[test]
    fn split_fields_on_single_non_whitespace_char() {
        assert_eq!(
            split_fields(
                ExpandedWord {
                    parts: vec![ExpandedWordPart::GeneratedUnquotedLiteral(
                        "a:b:c:".to_string()
                    )]
                },
                Some(":")
            ),
            vec![
                ExpandedWord::unquoted_literal("a"),
                ExpandedWord::unquoted_literal("b"),
                ExpandedWord::unquoted_literal("c")
            ]
        );
    }

    #[test]
    fn split_fields_on_multiple_non_whitespace_char() {
        assert_eq!(
            split_fields(
                ExpandedWord {
                    parts: vec![ExpandedWordPart::GeneratedUnquotedLiteral(
                        "a:b/c-d-:/x y".to_string()
                    )]
                },
                Some(":/-")
            ),
            vec![
                ExpandedWord::unquoted_literal("a"),
                ExpandedWord::unquoted_literal("b"),
                ExpandedWord::unquoted_literal("c"),
                ExpandedWord::unquoted_literal("d"),
                ExpandedWord::unquoted_literal("x y")
            ]
        );
    }

    #[test]
    fn split_fields_on_single_whitespace_char() {
        assert_eq!(
            split_fields(
                ExpandedWord {
                    parts: vec![ExpandedWordPart::GeneratedUnquotedLiteral(
                        "a b c ".to_string()
                    )]
                },
                Some(" ")
            ),
            vec![
                ExpandedWord::unquoted_literal("a"),
                ExpandedWord::unquoted_literal("b"),
                ExpandedWord::unquoted_literal("c"),
            ]
        );
    }

    #[test]
    fn split_fields_on_multiple_whitespace_char() {
        assert_eq!(
            split_fields(
                ExpandedWord {
                    parts: vec![ExpandedWordPart::GeneratedUnquotedLiteral(
                        "  a\t\n\t\nb  \tc\nd  e f".to_string()
                    )]
                },
                Some("\t\n")
            ),
            vec![
                ExpandedWord::unquoted_literal("  a"),
                ExpandedWord::unquoted_literal("b  "),
                ExpandedWord::unquoted_literal("c"),
                ExpandedWord::unquoted_literal("d  e f"),
            ]
        )
    }

    #[test]
    fn split_fields_default_ifs() {
        assert_eq!(
            split_fields(
                ExpandedWord {
                    parts: vec![ExpandedWordPart::GeneratedUnquotedLiteral(
                        "\t\n   a\n\n\t  \n\t word,and\n\t\n \n\tc\n\n\n   \t\n\t ".to_string()
                    )]
                },
                None
            ),
            vec![
                ExpandedWord::unquoted_literal("a"),
                ExpandedWord::unquoted_literal("word,and"),
                ExpandedWord::unquoted_literal("c")
            ]
        );
    }

    #[test]
    fn split_fields_by_mixed_ifs() {
        assert_eq!(
            split_fields(
                ExpandedWord {
                    parts: vec![ExpandedWordPart::GeneratedUnquotedLiteral(
                        "a,b.c  d\n\ne  f".to_string()
                    )]
                },
                Some(",.:  \n")
            ),
            vec![
                ExpandedWord::unquoted_literal("a"),
                ExpandedWord::unquoted_literal("b"),
                ExpandedWord::unquoted_literal("c"),
                ExpandedWord::unquoted_literal("d"),
                ExpandedWord::unquoted_literal("e"),
                ExpandedWord::unquoted_literal("f")
            ]
        );
    }

    #[test]
    fn field_splitting_does_not_affect_non_generated_literals() {
        assert_eq!(
            split_fields(
                ExpandedWord {
                    parts: vec![ExpandedWordPart::UnquotedLiteral("a:b:c".to_string()),]
                },
                Some(":")
            ),
            vec![ExpandedWord::unquoted_literal("a:b:c")]
        );
        assert_eq!(
            split_fields(
                ExpandedWord {
                    parts: vec![
                        ExpandedWordPart::UnquotedLiteral("a:".to_string()),
                        ExpandedWordPart::GeneratedUnquotedLiteral("b:c".to_string()),
                        ExpandedWordPart::UnquotedLiteral(":d".to_string())
                    ]
                },
                Some(":")
            ),
            vec![
                ExpandedWord::unquoted_literal("a:b"),
                ExpandedWord::unquoted_literal("c:d")
            ]
        );
    }
}
