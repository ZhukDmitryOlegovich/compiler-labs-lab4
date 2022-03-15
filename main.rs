use colored::Colorize;
use std::env;
use std::vec::Vec;

#[derive(Debug)]
enum TokenType {
    Spaces(String),
    String(String),
    Identifier(String),
    Number(i64),
    Error(char),
    SimpleError,
    End,
}

#[derive(Debug)]
#[allow(dead_code)]
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
}

struct ParseToken(SmartIterator);

impl ParseToken {
    fn next_spaces(&mut self) -> TokenType {
        let mut ans = String::new();
        while let Some(s @ (' ' | '\t' | '\n')) = self.0.see() {
            ans.push(s);
            self.0.next();
        }
        TokenType::Spaces(ans)
    }

    fn next_string(&mut self) -> TokenType {
        if Some('"') == self.0.next() {
            let mut ans = String::new();
            if loop {
                match self.0.next() {
                    Some('"') => {
                        if Some('"') == self.0.see() {
                            ans.push('"');
                            self.0.next();
                        } else {
                            break true;
                        };
                    }
                    Some('\\') => {
                        if Some('\n') == self.0.see() {
                            ans.push('\n');
                            self.0.next();
                        } else {
                            ans.push('\\');
                        }
                    }
                    Some('\n') | None => break false,
                    Some(x) => ans.push(x),
                }
            } {
                TokenType::String(ans)
            } else {
                TokenType::SimpleError
            }
        } else {
            TokenType::SimpleError
        }
    }

    fn next_number10(&mut self) -> TokenType {
        let mut ans = 0;
        TokenType::Number(loop {
            match self.0.see() {
                Some(x) if x.is_digit(10) => {
                    ans = ans * 10 + x.to_digit(10).unwrap() as i64;
                    self.0.next();
                }
                _ => break ans,
            }
        })
    }

    fn next_number16(&mut self) -> TokenType {
        if Some('$') == self.0.next() {
            let mut ans = 0;
            let mut is_empty = true;
            loop {
                match self.0.see() {
                    Some(x) if x.is_digit(16) => {
                        is_empty = false;
                        ans = ans * 16 + x.to_digit(16).unwrap() as i64;
                        self.0.next();
                    }
                    _ => break,
                }
            }
            if is_empty {
                TokenType::SimpleError
            } else {
                TokenType::Number(ans)
            }
        } else {
            TokenType::SimpleError
        }
    }

    fn next_identifier(&mut self) -> TokenType {
        if let Some(x) = self.0.next() {
            if x.is_alphabetic() {
                let mut ans = String::from(x);
                loop {
                    match self.0.see() {
                        Some(x) if x.is_alphabetic() || x.is_digit(10) || x == '$' => {
                            ans.push(x);
                            self.0.next();
                        }
                        _ => break,
                    }
                }
                TokenType::Identifier(ans)
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
            let st_ind = self.0.ind;
            let from = self.0.pos;
            let token = match l {
                ' ' | '\t' | '\n' => self.next_spaces(),
                '"' => self.next_string(),
                x if x.is_digit(10) => self.next_number10(),
                '$' => self.next_number16(),
                x if x.is_alphabetic() => self.next_identifier(),
                _ => TokenType::SimpleError,
            };
            match token {
                TokenType::SimpleError => {
                    self.0.ind = st_ind;
                    self.0.pos = from;
                    if env::var("SKIP_ERRORS").is_ok() {
                        self.0.next();
                        self.next()
                    } else {
                        Some(Token {
                            from,
                            value: TokenType::Error(self.0.next().unwrap()),
                            to: from,
                        })
                    }
                }
                TokenType::Spaces(_) if !env::var("NEED_SPACES").is_ok() => self.next(),
                _ => Some(Token {
                    from,
                    value: token,
                    to: self.0.prev_pos,
                }),
            }
        } else if self.0.ind == self.0.arr.len() && !env::var("SKIP_EOF").is_ok() {
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
        let (name, val) = match x.value {
            TokenType::Spaces(sp) => ("SPA".white(), format!("{:?}", sp).white()),
            TokenType::String(str) => ("STR".green(), format!("{:?}", str).green()),
            TokenType::Identifier(id) => ("IDN".blue(), id.blue()),
            TokenType::Number(num) => ("NUM".yellow(), format!("{}", num).yellow()),
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
        // println!("{:?}", x);
    }
}
