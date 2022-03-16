use colored::Colorize;
use std::env;
use std::vec::Vec;

#[derive(Debug)]
enum TokenType {
    Spaces(String),
    Symbol(char),
    Identifier(String),
    Key(String),
    Error(char),
    SimpleError,
    End,
}

#[derive(Debug)]
struct Token {
    from: (usize, usize),
    to: (usize, usize),
    value: TokenType,
}

#[derive(Debug)]
struct SmartIterator {
    arr: Vec<char>,
    prev_pos: (usize, usize),
    pos: (usize, usize),
    ind: usize,
}

#[derive(Debug)]
struct Position{
    ind: usize, 
    pos: (usize, usize), 
    prev_pos: (usize, usize),
}

impl SmartIterator {
    fn new(s: String) -> Self {
        Self {
            arr: s.chars().collect(),
            pos: (1, 1),
            prev_pos: (1, 0),
            ind: 0,
        }
    }

    fn see(&self) -> Option<char> {
        if self.ind < self.arr.len() {
            Some(self.arr[self.ind])
        } else {
            None
        }
    }

    fn next(&mut self) -> Option<char> {
        if self.ind < self.arr.len() {
            Some({
                let x = self.arr[self.ind];
                self.ind += 1;
                self.prev_pos = self.pos;
                self.pos = if x == '\n' {
                    (self.pos.0 + 1, 1)
                } else {
                    (self.pos.0, self.pos.1 + 1)
                };
                x
            })
        } else {
            None
        }
    }

    fn save_pos(&self) -> Position {
        Position{ind: self.ind, pos: self.pos, prev_pos: self.prev_pos}
    }

    fn load_pos(&mut self, a: Position) {
        self.ind = a.ind;
        self.pos = a.pos;
        self.prev_pos = a.prev_pos;
    }
}

struct ParseToken(SmartIterator);

fn to_digit_16(d: char) -> Option<u32> {
    d.to_digit(16)
}

impl ParseToken {
    fn next_spaces(&mut self) -> TokenType {
        let mut ans = String::new();
        while let Some(s @ (' ' | '\t' | '\n')) = self.0.see() {
            ans.push(s);
            self.0.next();
        }
        TokenType::Spaces(ans)
    }

    fn next_number16_4(&mut self, x: char) -> Option<char> {
        to_digit_16(x)
            .zip(self.0.next().and_then(to_digit_16))
            .zip(self.0.next().and_then(to_digit_16))
            .zip(self.0.next().and_then(to_digit_16))
            .and_then(|(((x1, x2), x3), x4)| {
                char::from_u32(((x1 * 16 + x2) * 16 + x3) * 16 + x4)
            })
    }

    fn next_symbol(&mut self) -> TokenType {
        if Some('\'') == self.0.next() {
            if let Some(ans) = match self.0.next() {
                Some('\'') => None,
                Some('\n') => None,
                Some('\\') => match self.0.next() {
                    Some('n') => Some('\n'),
                    Some(x @ ('\'' | '\\')) => Some(x),
                    Some(x1) => self.next_number16_4(x1),
                    _ => None,
                },
                x @ _ => x,
            } {
                if Some('\'') == self.0.next() {
                    TokenType::Symbol(ans)
                } else {
                    TokenType::SimpleError
                }
            } else {
                TokenType::SimpleError
            }
        } else {
            TokenType::SimpleError
        }
    }

    fn next_identifier_or_key(&mut self) -> TokenType {
        if let Some(x) = self.0.next() {
            if x.is_alphabetic() {
                let mut ans = String::from(x);
                let mut last_true_save = Some((self.0.save_pos(), ans.clone()));
                loop {
                    match self.0.see() {
                        Some(x) if x.is_alphabetic() || x.is_digit(10) => {
                            ans.push(x);
                            self.0.next();
                            if x.is_alphabetic() {
                                last_true_save = Some((self.0.save_pos(), ans.clone()))
                            }
                        }
                        _ => break,
                    }
                    if ans.len() == 10 {
                        break;
                    }
                }
                match last_true_save {
                    Some((save, ans)) if ans.eq("z") || ans.eq("for") || ans.eq("forward") => {
                        self.0.load_pos(save);
                        TokenType::Key(ans)
                    },
                    Some((save, ans)) if ans.len() >= 2 => {
                        self.0.load_pos(save);
                        TokenType::Identifier(ans)
                    },
                    _ => TokenType::SimpleError,
                }
            } else {
                TokenType::SimpleError
            }
        } else {
            TokenType::SimpleError
        }
    }
}

impl std::iter::Iterator for ParseToken {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        if let Some(l) = self.0.see() {
            let save = self.0.save_pos();
            let token = match l {
                ' ' | '\t' | '\n' => self.next_spaces(),
                '\'' => self.next_symbol(),
                x if x.is_alphabetic() => self.next_identifier_or_key(),
                _ => TokenType::SimpleError,
            };
            match token {
                TokenType::SimpleError => {
                    self.0.load_pos(save);
                    if env::var("SKIP_ERRORS").is_ok() {
                        self.0.next();
                        self.next()
                    } else {
                        Some(Token {
                            from: self.0.pos,
                            value: TokenType::Error(self.0.next().unwrap()),
                            to: self.0.pos,
                        })
                    }
                }
                TokenType::Spaces(_) if !env::var("NEED_SPACES").is_ok() => self.next(),
                _ => Some(Token {
                    from: save.pos,
                    value: token,
                    to: self.0.prev_pos,
                }),
            }
        } else if !env::var("SKIP_EOF").is_ok() {
            self.0.ind += 1;
            Some(Token {
                from: self.0.pos,
                value: TokenType::End,
                to: self.0.pos,
            })
        } else {
            None
        }
    }
}

fn main() {
    let filename = std::env::args().nth(1).unwrap();

    println!("filename = {:?}", filename);

    let content = std::fs::read_to_string(filename).unwrap();

    println!("content  = {:?}", content);
    println!();

    for x in ParseToken(SmartIterator::new(content)) {
        let is_eof = matches!(x.value, TokenType::End);
        let (name, val) = match x.value {
            TokenType::Spaces(sp) => ("SPA".white(), format!("{:?}", sp).white()),
            TokenType::Symbol(str) => ("SYM".green(), format!("{:?}", str).green()),
            TokenType::Identifier(id) => ("IDN".blue(), id.blue()),
            TokenType::Key(num) => ("KEY".yellow(), format!("{}", num).yellow()),
            TokenType::Error(err) => ("ERR".red().bold(), format!("{:?}", err).red().bold()),
            TokenType::SimpleError => ("ERR".red().bold(), "ERR".red().bold()),
            TokenType::End => ("END".purple().bold(), "EOF".purple().bold()),
        };
        println!(
            "{} {} {}",
            name,
            format!("{:>2?}-{:>2?}:", x.from, x.to).truecolor(128, 128, 128),
            val
        );
        if is_eof { break; }
    }
}
