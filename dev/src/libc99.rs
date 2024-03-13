//
// Copyright (c) 2024 Jeff Garzik
//
// This file is part of the posixutils-rs project covered under
// the MIT License.  For the full license text, please see the LICENSE
// file in the root directory of this project.
// SPDX-License-Identifier: MIT
//

const TABSTOP: i32 = 8;

enum CTokenType {
    Invalid,
    StreamBegin,
    StreamEnd,
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
        let mut c: i32 = -1;
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

                    return -1;
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

    pub fn get_one_token(&mut self, _c: i32) -> i32 {
        -1
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
