//
// Copyright (c) 2024 Jeff Garzik
//
// This file is part of the posixutils-rs project covered under
// the MIT License.  For the full license text, please see the LICENSE
// file in the root directory of this project.
// SPDX-License-Identifier: MIT
//

const TABSTOP: i32 = 8;
const EOF: i32 = -1;

enum CTokenType {
    Invalid,
    StreamBegin,
    StreamEnd,
    Number(String),
    Str(bool, bool, String),
}

pub struct CToken {
    ttype: CTokenType,
}

impl CToken {
    fn new() -> CToken {
        CToken {
            ttype: CTokenType::Invalid,
        }
    }

    fn new_type(ttype: CTokenType) -> CToken {
        CToken { ttype }
    }
}

pub struct CStream {
    fd: i32,
    offset: usize,
    size: usize,
    pos: i32,
    line: i32,
    nr: i32,
    newline: i32,
    whitespace: i32,

    token: CToken,
    tokenlist: Vec<CToken>,
    buffer: Vec<u8>,
}

enum NcState {
    Restart,
    Repeat,
    Norm,
    Out,
    CheckLF,
    GotEOF,
}

const CC_DIGIT: u32 = 1 << 0;
const CC_LETTER: u32 = 1 << 1;
const CC_HEX: u32 = 1 << 2;
const CC_EXP: u32 = 1 << 3;
const CC_SECOND: u32 = 1 << 4;
const CC_QUOTE: u32 = 1 << 5;
const CC_DOT: u32 = 1 << 6;

fn classify_char(c32: i32) -> u32 {
    let ch = (c32 as u8) as char;

    match ch {
        '0'..='9' => CC_DIGIT | CC_HEX,
        'A'..='D' => CC_LETTER | CC_HEX,
        'E' => CC_LETTER | CC_HEX | CC_EXP, /* E<exp> */
        'F' => CC_LETTER | CC_HEX,
        'G'..='O' => CC_LETTER,
        'P' => CC_LETTER | CC_EXP, /* P<exp> */
        'Q'..='Z' => CC_LETTER,
        'a'..='d' => CC_LETTER | CC_HEX,
        'e' => CC_LETTER | CC_HEX | CC_EXP, /* e<exp> */
        'f' => CC_LETTER | CC_HEX,
        'g'..='o' => CC_LETTER,
        'p' => CC_LETTER | CC_EXP, /* p<exp> */
        'q'..='z' => CC_LETTER,
        '_' => CC_LETTER,
        '.' => CC_DOT | CC_SECOND,
        '=' => CC_SECOND,
        '+' => CC_SECOND,
        '-' => CC_SECOND,
        '>' => CC_SECOND,
        '<' => CC_SECOND,
        '&' => CC_SECOND,
        '|' => CC_SECOND,
        '#' => CC_SECOND,
        '\'' => CC_QUOTE,
        '"' => CC_QUOTE,
        _ => 0,
    }
}

impl CStream {
    pub fn from_buffer(idx: i32, buf: Vec<u8>) -> CStream {
        CStream {
            fd: -1,
            nr: idx,
            line: 1,
            newline: 1,
            whitespace: 0,
            pos: 0,
            offset: 0,
            size: buf.len(),
            token: CToken::new(),
            tokenlist: vec![CToken::new_type(CTokenType::StreamBegin)],
            buffer: buf,
        }
    }

    pub fn mark_eof(&mut self) {
        let token = CToken::new_type(CTokenType::StreamEnd);
        self.tokenlist.push(token);
    }

    pub fn nextchar_slow(&mut self) -> i32 {
        let size = self.size;
        let mut offset = self.offset;
        let mut c: i32 = EOF;
        let mut spliced = false;
        let mut had_cr = false;
        let mut had_backslash = false;
        let mut state = NcState::Restart;

        loop {
            match state {
                NcState::Restart => {
                    had_cr = false;
                    had_backslash = false;
                    state = NcState::Repeat;
                }
                NcState::Repeat => {
                    if offset >= size {
                        if self.fd < 0 {
                            state = NcState::GotEOF;
                            continue;
                        }

                        panic!("reading files not implemented");
                    }

                    let ch = self.buffer[offset];
                    c = ch as i32;
                    offset += 1;
                    if had_cr {
                        state = NcState::CheckLF;
                        continue;
                    }

                    if ch as char == '\r' {
                        had_cr = true;
                        continue; // state remains Repeat
                    }

                    state = NcState::Norm;
                }
                NcState::Norm => {
                    if had_backslash {
                        match (c as u8) as char {
                            '\t' => {
                                self.pos += TABSTOP - (self.pos % TABSTOP);
                            }
                            '\n' => {
                                self.line += 1;
                                self.pos = 0;
                                self.newline = 1;
                            }
                            '\\' => {
                                had_backslash = true;
                                self.pos += 1;
                                state = NcState::Repeat;
                                continue;
                            }
                            _ => {
                                self.pos += 1;
                            }
                        }
                    } else {
                        if (c as u8) as char == '\n' {
                            self.line += 1;
                            self.pos = 0;
                            spliced = true;
                            state = NcState::Restart;
                            continue;
                        }

                        offset -= 1;
                        c = ('\\' as u8) as i32;
                    }

                    state = NcState::Out;
                }
                NcState::Out => {
                    self.offset = offset;
                    return c;
                }
                NcState::CheckLF => {
                    if (c as u8) as char != '\n' {
                        offset -= 1;
                    }
                    c = ('\n' as u8) as i32;
                    state = NcState::Norm;
                }
                NcState::GotEOF => {
                    if had_backslash {
                        c = ('\\' as u8) as i32;
                        state = NcState::Out;
                        continue;
                    }

                    // TODO pass no-newline-at-EOF warnings

                    return EOF;
                }
            }
        }
    }

    pub fn nextchar(&mut self) -> i32 {
        let mut offset = self.offset;
        if offset < self.size {
            let ch = self.buffer[offset];
            let c = ch as i32;
            offset += 1;

            match ch as char {
                '\t' | '\r' | '\n' | '\\' => {}
                _ => {
                    self.offset = offset;
                    self.pos += 1;
                    return c;
                }
            }
        }

        return self.nextchar_slow();
    }

    pub fn drop_comment(&mut self) -> i32 {
        // todo: drop_token() -- adjust stream {newline,whitespace}

        let newline = self.newline;
        let mut next = self.nextchar();
        loop {
            let curr = next;
            if curr < 0 {
                // todo: warning EOF-in-comment
                return curr;
            }

            next = self.nextchar();
            if curr == (b'*' as i32) && next == (b'/' as i32) {
                break;
            }
        }

        self.newline = newline;

        self.nextchar()
    }

    pub fn drop_eoln(&mut self) -> i32 {
        // todo: drop_token() -- adjust stream {newline,whitespace}
        loop {
            let c = self.nextchar();
            if c < 0 {
                return c;
            }

            if (c as u8) as char == '\n' {
                return self.nextchar();
            }
        }
    }

    pub fn eat_string(&mut self, mut next: i32, is_str: bool, is_wide: bool) -> i32 {
        let delim = match is_str {
            true => '"',
            false => '\'',
        };
        let mut escape = false;
        let mut want_hex = false;
        let mut tmpstr = String::with_capacity(80);

        while escape || next != delim as i32 {
            tmpstr.push((next as u8) as char);
            if next == b'\n' as i32 {
                // todo -- warning -- unterminated string
                break;
            }
            if next < 0 {
                // todo -- warning -- EOF in string
                return next;
            }
            if !escape {
                if want_hex && classify_char(next) & CC_HEX != 0 {
                    // todo -- warning -- no following hex digits
                }
                want_hex = false;
                if next == b'\\' as i32 {
                    escape = true;
                }
            } else {
                escape = false;
                if next == b'x' as i32 {
                    want_hex = true;
                }
            }
            next = self.nextchar();
        }

        if want_hex {
            // todo -- warning -- no following hex digits
        }

        self.tokenlist.push(CToken {
            ttype: CTokenType::Str(is_str, is_wide, tmpstr),
        });

        self.nextchar()
    }

    pub fn get_one_number(&mut self, c: i32, mut next: i32) -> i32 {
        let mut numstr = String::with_capacity(80);

        numstr.push((c as u8) as char);
        loop {
            let class = classify_char(next);
            if (class & (CC_DOT | CC_DIGIT | CC_LETTER)) == 0 {
                break;
            }

            numstr.push((next as u8) as char);
            next = self.nextchar();
            if (class & CC_EXP) != 0 {
                let next_ch = (next as u8) as char;
                if next_ch == '-' || next_ch == '+' {
                    numstr.push(next_ch);
                    next = self.nextchar();
                }
            }
        }

        self.tokenlist.push(CToken {
            ttype: CTokenType::Number(numstr),
        });

        next
    }

    pub fn get_one_special(&mut self, c: i32) -> i32 {
        let next = self.nextchar();
        let next_ch = (next as u8) as char;
        match (c as u8) as char {
            '.' => {
                if next_ch >= '0' && next_ch <= '9' {
                    return self.get_one_number(c, next);
                }
            }
            '"' => {
                return self.eat_string(next, true, false);
            }
            '\'' => {
                return self.eat_string(next, false, false);
            }
            '/' => {
                if next_ch == '/' {
                    return self.drop_eoln();
                }
                if next_ch == '*' {
                    return self.drop_comment();
                }
            }
            _ => {}
        }

        let mut value = c;
        let mask = classify_char(next);
        if (mask & CC_SECOND) != 0 {
            // TODO
        }

        EOF
    }

    pub fn get_one_identifier(&mut self, _c: i32) -> i32 {
        EOF // TODO
    }

    pub fn get_one_token(&mut self, c: i32) -> i32 {
        let mask = classify_char(c);
        assert!(mask != 0);

        if (mask & CC_DIGIT) != 0 {
            let c2 = self.nextchar();
            return self.get_one_number(c, c2);
        }
        if (mask & CC_LETTER) != 0 {
            return self.get_one_identifier(c);
        }

        return self.get_one_special(c);
    }

    pub fn tokenize(&mut self) {
        let mut c = self.nextchar();
        while c >= 0 {
            let ch = (c as u8) as char;

            if !ch.is_whitespace() {
                self.token = CToken::new_type(CTokenType::Invalid);
                self.newline = 0;
                self.whitespace = 0;

                c = self.get_one_token(c);
                continue;
            }

            self.whitespace = 1;
            c = self.nextchar();
        }

        self.mark_eof();
    }
}
