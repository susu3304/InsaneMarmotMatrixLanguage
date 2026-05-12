use crate::diagnostics::{Category, Diagnostic};
use crate::source::Span;
use crate::token::{Keyword, NumberLiteral, Token, TokenKind};

pub fn lex(file_id: usize, source: &str) -> Result<Vec<Token>, Diagnostic> {
    Lexer::new(file_id, source).tokenize()
}

struct Lexer {
    file_id: usize,
    source: Vec<char>,
    current: usize,
    byte_current: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
}

impl Lexer {
    fn new(file_id: usize, source: &str) -> Self {
        let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
        Self {
            file_id,
            source: normalized.chars().collect(),
            current: 0,
            byte_current: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, Diagnostic> {
        while !self.is_at_end() {
            let start = self.mark();
            self.scan_token(start)?;
        }
        let span = Span::new(
            self.file_id,
            self.byte_current,
            self.byte_current,
            self.line,
            self.column,
        );
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            span,
        });
        Ok(self.tokens)
    }

    fn scan_token(&mut self, start: Mark) -> Result<(), Diagnostic> {
        let c = self.advance();
        match c {
            ' ' | '\t' => Ok(()),
            '\n' => {
                self.push(TokenKind::Newline, "\n".to_string(), start);
                Ok(())
            }
            '#' => {
                while self.peek() != '\n' && !self.is_at_end() {
                    self.advance();
                }
                Ok(())
            }
            '/' if self.match_char('*') => self.block_comment(start),
            '"' => self.string(start),
            c if c.is_ascii_digit() => self.number(start),
            c if is_ident_start(c) => {
                self.identifier(start);
                Ok(())
            }
            '=' if self.match_char('=') => {
                self.push_symbol("==", start);
                Ok(())
            }
            '=' if self.match_char('>') => {
                self.push_symbol("=>", start);
                Ok(())
            }
            '!' if self.match_char('=') => {
                self.push_symbol("!=", start);
                Ok(())
            }
            '<' if self.match_char('=') => {
                self.push_symbol("<=", start);
                Ok(())
            }
            '>' if self.match_char('=') => {
                self.push_symbol(">=", start);
                Ok(())
            }
            '&' if self.match_char('&') => {
                self.push_symbol("&&", start);
                Ok(())
            }
            '|' if self.match_char('|') => {
                self.push_symbol("||", start);
                Ok(())
            }
            '-' if self.match_char('>') => {
                self.push_symbol("->", start);
                Ok(())
            }
            '.' if self.match_char('.') => {
                self.push_symbol("..", start);
                Ok(())
            }
            ';' => {
                self.push(TokenKind::Newline, ";".to_string(), start);
                Ok(())
            }
            c if "{}()[],:+-*/%!=<>.@".contains(c) => {
                self.push_symbol(&c.to_string(), start);
                Ok(())
            }
            _ => Err(
                Diagnostic::new(Category::Syntax, format!("unexpected character {c:?}"))
                    .with_span(start.span(self.file_id, self.byte_current)),
            ),
        }
    }

    fn block_comment(&mut self, start: Mark) -> Result<(), Diagnostic> {
        while !self.is_at_end() {
            if self.peek() == '*' && self.peek_next() == '/' {
                self.advance();
                self.advance();
                return Ok(());
            }
            self.advance();
        }
        Err(
            Diagnostic::new(Category::Syntax, "unterminated block comment")
                .with_span(start.span(self.file_id, self.byte_current)),
        )
    }

    fn string(&mut self, start: Mark) -> Result<(), Diagnostic> {
        let mut value = String::new();
        while !self.is_at_end() {
            let c = self.advance();
            match c {
                '"' => {
                    let lexeme = self.lexeme(start);
                    self.push(TokenKind::String(value), lexeme, start);
                    return Ok(());
                }
                '\\' => {
                    if self.is_at_end() {
                        break;
                    }
                    let escaped = match self.advance() {
                        'n' => '\n',
                        't' => '\t',
                        '"' => '"',
                        '\\' => '\\',
                        other => {
                            return Err(Diagnostic::new(
                                Category::Syntax,
                                format!("unknown escape \\{other}"),
                            )
                            .with_span(start.span(self.file_id, self.byte_current)));
                        }
                    };
                    value.push(escaped);
                }
                '\n' => {
                    return Err(Diagnostic::new(Category::Syntax, "unterminated string")
                        .with_span(start.span(self.file_id, self.byte_current)));
                }
                other => value.push(other),
            }
        }
        Err(Diagnostic::new(Category::Syntax, "unterminated string")
            .with_span(start.span(self.file_id, self.byte_current)))
    }

    fn number(&mut self, start: Mark) -> Result<(), Diagnostic> {
        while self.peek().is_ascii_digit() {
            self.advance();
        }
        let mut is_float = false;
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            is_float = true;
            self.advance();
            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }
        let text = self.lexeme(start);
        let literal = if is_float {
            NumberLiteral::Float(text.parse::<f64>().map_err(|_| {
                Diagnostic::new(Category::Syntax, format!("invalid float literal {text}"))
                    .with_span(start.span(self.file_id, self.byte_current))
            })?)
        } else {
            NumberLiteral::Int(text.parse::<i64>().map_err(|_| {
                Diagnostic::new(Category::Syntax, format!("invalid integer literal {text}"))
                    .with_span(start.span(self.file_id, self.byte_current))
            })?)
        };
        self.push(TokenKind::Number(literal), text, start);
        Ok(())
    }

    fn identifier(&mut self, start: Mark) {
        while is_ident_part(self.peek()) {
            self.advance();
        }
        let text = self.lexeme(start);
        let kind = Keyword::parse(&text)
            .map(TokenKind::Keyword)
            .unwrap_or_else(|| TokenKind::Identifier(text.clone()));
        self.push(kind, text, start);
    }

    fn push_symbol(&mut self, symbol: &str, start: Mark) {
        self.push(
            TokenKind::Symbol(symbol.to_string()),
            symbol.to_string(),
            start,
        );
    }

    fn push(&mut self, kind: TokenKind, lexeme: String, start: Mark) {
        let span = start.span(self.file_id, self.byte_current);
        self.tokens.push(Token { kind, lexeme, span });
    }

    fn lexeme(&self, start: Mark) -> String {
        self.source[start.current..self.current].iter().collect()
    }

    fn mark(&self) -> Mark {
        Mark {
            current: self.current,
            byte_current: self.byte_current,
            line: self.line,
            column: self.column,
        }
    }

    fn advance(&mut self) -> char {
        let c = self.source[self.current];
        self.current += 1;
        self.byte_current += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        c
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.source[self.current] != expected {
            return false;
        }
        self.advance();
        true
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source[self.current]
        }
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            '\0'
        } else {
            self.source[self.current + 1]
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }
}

#[derive(Clone, Copy)]
struct Mark {
    current: usize,
    byte_current: usize,
    line: usize,
    column: usize,
}

impl Mark {
    fn span(self, file_id: usize, byte_end: usize) -> Span {
        Span::new(file_id, self.byte_current, byte_end, self.line, self.column)
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_part(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use crate::token::{Keyword, NumberLiteral, TokenKind};

    use super::lex;

    #[test]
    fn tokenizes_keywords_strings_numbers_and_spans() {
        let tokens = lex(0, "marmot main {\r\n  squeak \"hi\\n\"\n  let x = 42\n}\n").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Keyword(Keyword::Marmot));
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.column, 1);
        assert!(tokens
            .iter()
            .any(|token| token.kind == TokenKind::String("hi\n".to_string())));
        assert!(tokens
            .iter()
            .any(|token| token.kind == TokenKind::Number(NumberLiteral::Int(42))));
    }

    #[test]
    fn reports_bad_block_comment() {
        let err = lex(0, "/* nope").unwrap_err();
        assert_eq!(err.message, "unterminated block comment");
    }
}
