//! Splits one source line into tokens. Comments start with ';'.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tok {
    /// Name, mnemonic or register. May contain dots (`name.msb`).
    Ident(String),
    /// `.define`, `.include`, `.data` (stored lowercase, without the dot).
    Directive(String),
    Number(i64),
    Char(u8),
    Str(Vec<u8>),
    Comma,
    Colon,
    LBracket,
    RBracket,
    Plus,
    Minus,
    Dollar,
}

impl Tok {
    /// How the token is shown in error messages.
    pub fn describe(&self) -> String {
        match self {
            Tok::Ident(s) => format!("'{s}'"),
            Tok::Directive(s) => format!("'.{s}'"),
            Tok::Number(n) => format!("number {n}"),
            Tok::Char(c) => format!("character '{}'", *c as char),
            Tok::Str(_) => "string".into(),
            Tok::Comma => "','".into(),
            Tok::Colon => "':'".into(),
            Tok::LBracket => "'['".into(),
            Tok::RBracket => "']'".into(),
            Tok::Plus => "'+'".into(),
            Tok::Minus => "'-'".into(),
            Tok::Dollar => "'$'".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub tok: Tok,
    /// 1-based column.
    pub col: u32,
}

/// A lexing error: column and message.
pub type LexError = (u32, String);

pub fn lex_line(line: &str) -> Result<Vec<Token>, LexError> {
    let chars: Vec<char> = line.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        let col = i as u32 + 1;
        let single = |tok| Token { tok, col };

        match c {
            ';' => break,
            c if c.is_whitespace() => i += 1,
            ',' => {
                tokens.push(single(Tok::Comma));
                i += 1;
            }
            ':' => {
                tokens.push(single(Tok::Colon));
                i += 1;
            }
            '[' => {
                tokens.push(single(Tok::LBracket));
                i += 1;
            }
            ']' => {
                tokens.push(single(Tok::RBracket));
                i += 1;
            }
            '+' => {
                tokens.push(single(Tok::Plus));
                i += 1;
            }
            '-' => {
                tokens.push(single(Tok::Minus));
                i += 1;
            }
            '$' => {
                tokens.push(single(Tok::Dollar));
                i += 1;
            }
            '.' => {
                let start = i + 1;
                let end = scan(&chars, start, |c| c.is_ascii_alphanumeric() || c == '_');
                if end == start {
                    return Err((col, "expected a directive name after '.'".into()));
                }
                let name: String = chars[start..end].iter().collect();
                tokens.push(single(Tok::Directive(name.to_ascii_lowercase())));
                i = end;
            }
            c if c.is_ascii_digit() => {
                let end = scan(&chars, i, |c| c.is_ascii_alphanumeric() || c == '_');
                let text: String = chars[i..end].iter().collect();
                let value = parse_number(&text).map_err(|m| (col, m))?;
                tokens.push(single(Tok::Number(value)));
                i = end;
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let end = scan(&chars, i, |c| {
                    c.is_ascii_alphanumeric() || c == '_' || c == '.'
                });
                let text: String = chars[i..end].iter().collect();
                tokens.push(single(Tok::Ident(text)));
                i = end;
            }
            '\'' => {
                let (bytes, end) = scan_quoted(&chars, i, '\'')?;
                if bytes.len() != 1 {
                    return Err((
                        col,
                        "a character literal holds exactly one character".into(),
                    ));
                }
                tokens.push(single(Tok::Char(bytes[0])));
                i = end;
            }
            '"' => {
                let (bytes, end) = scan_quoted(&chars, i, '"')?;
                if bytes.is_empty() {
                    return Err((col, "empty string".into()));
                }
                tokens.push(single(Tok::Str(bytes)));
                i = end;
            }
            other => return Err((col, format!("unexpected character '{other}'"))),
        }
    }
    Ok(tokens)
}

/// Index of the first char from `start` that does not match.
fn scan(chars: &[char], start: usize, matches: impl Fn(char) -> bool) -> usize {
    let mut end = start;
    while end < chars.len() && matches(chars[end]) {
        end += 1;
    }
    end
}

/// Reads a quoted literal starting at `start` (the opening quote).
/// Returns its bytes and the index after the closing quote.
fn scan_quoted(chars: &[char], start: usize, quote: char) -> Result<(Vec<u8>, usize), LexError> {
    let mut bytes = Vec::new();
    let mut i = start + 1;
    loop {
        let Some(&c) = chars.get(i) else {
            return Err((start as u32 + 1, format!("missing closing {quote}")));
        };
        let col = i as u32 + 1;
        match c {
            c if c == quote => return Ok((bytes, i + 1)),
            '\\' => {
                let Some(&e) = chars.get(i + 1) else {
                    return Err((col, "unfinished escape sequence".into()));
                };
                i += 2;
                bytes.push(match e {
                    'n' => b'\n',
                    'r' => b'\r',
                    't' => b'\t',
                    '0' => 0,
                    '\\' => b'\\',
                    '\'' => b'\'',
                    '"' => b'"',
                    'x' => {
                        let hex: String = chars.get(i..i + 2).unwrap_or_default().iter().collect();
                        i += 2;
                        u8::from_str_radix(&hex, 16)
                            .ok()
                            .filter(|_| hex.len() == 2)
                            .ok_or((col, "'\\x' needs two hexadecimal digits".to_string()))?
                    }
                    other => return Err((col, format!("unknown escape sequence '\\{other}'"))),
                });
            }
            c if c.is_ascii() && !c.is_ascii_control() => {
                bytes.push(c as u8);
                i += 1;
            }
            other => return Err((col, format!("character '{other}' is not printable ASCII"))),
        }
    }
}

/// Decimal, `0x` hexadecimal or `0b` binary, with optional `_` separators.
fn parse_number(text: &str) -> Result<i64, String> {
    let lower = text.to_ascii_lowercase();
    let (digits, radix) = if let Some(rest) = lower.strip_prefix("0x") {
        (rest, 16)
    } else if let Some(rest) = lower.strip_prefix("0b") {
        (rest, 2)
    } else {
        (lower.as_str(), 10)
    };
    let digits: String = digits.chars().filter(|&c| c != '_').collect();
    if digits.is_empty() {
        return Err(format!("invalid number '{text}'"));
    }
    i64::from_str_radix(&digits, radix).map_err(|_| format!("invalid number '{text}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(line: &str) -> Vec<Tok> {
        lex_line(line).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn instruction_with_comment() {
        assert_eq!(
            toks("  mov a, 0x2A ; comment"),
            [
                Tok::Ident("mov".into()),
                Tok::Ident("a".into()),
                Tok::Comma,
                Tok::Number(42)
            ]
        );
    }

    #[test]
    fn numbers() {
        assert_eq!(
            toks("42 0x2a 0b0010_1010 0xF0_00"),
            [42, 42, 42, 0xF000].map(Tok::Number)
        );
        assert!(lex_line("0x").is_err());
        assert!(lex_line("12ab").is_err());
    }

    #[test]
    fn literals_and_escapes() {
        assert_eq!(
            toks(r"'A' '\n' '\x41'"),
            [Tok::Char(b'A'), Tok::Char(b'\n'), Tok::Char(0x41)]
        );
        assert_eq!(toks(r#""a;b\"c""#), [Tok::Str(b"a;b\"c".to_vec())]);
        assert!(lex_line("\"open").is_err());
        assert!(lex_line("'ab'").is_err());
    }

    #[test]
    fn labels_pairs_and_directives() {
        assert_eq!(
            toks("loop: str [x:y], b"),
            [
                Tok::Ident("loop".into()),
                Tok::Colon,
                Tok::Ident("str".into()),
                Tok::LBracket,
                Tok::Ident("x".into()),
                Tok::Colon,
                Tok::Ident("y".into()),
                Tok::RBracket,
                Tok::Comma,
                Tok::Ident("b".into()),
            ]
        );
        assert_eq!(
            toks(".DATA v.msb"),
            [Tok::Directive("data".into()), Tok::Ident("v.msb".into())]
        );
    }

    #[test]
    fn columns() {
        let t = lex_line("  jmp $0x0300").unwrap();
        assert_eq!(t.iter().map(|t| t.col).collect::<Vec<_>>(), [3, 7, 8]);
    }
}
