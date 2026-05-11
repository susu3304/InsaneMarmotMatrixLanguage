use crate::ast::{Item, PackConfig, Program};
use crate::diagnostics::{Category, Diagnostic};
use crate::token::{Keyword, Token, TokenKind};

pub fn parse_items(tokens: &[Token]) -> Result<Program, Diagnostic> {
    let mut items = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        match &tokens[index].kind {
            TokenKind::Eof => break,
            TokenKind::Newline => index += 1,
            TokenKind::Keyword(Keyword::Marmot) => {
                items.push(Item::Main);
                index = skip_balanced_block(tokens, index)?;
            }
            TokenKind::Keyword(Keyword::Howl) => {
                items.push(Item::HowlMain);
                index = skip_balanced_block(tokens, index)?;
            }
            TokenKind::Keyword(Keyword::Insane) => {
                index += 1;
            }
            TokenKind::Keyword(Keyword::Dig) => {
                items.push(Item::Function(name_after(tokens, index, "function")?));
                index = skip_balanced_block(tokens, index)?;
            }
            TokenKind::Keyword(Keyword::Den) => {
                items.push(Item::Den(name_after(tokens, index, "den")?));
                index = skip_balanced_block(tokens, index)?;
            }
            TokenKind::Keyword(Keyword::Mask) => {
                items.push(Item::Mask(name_after(tokens, index, "mask")?));
                index = skip_balanced_block(tokens, index)?;
            }
            TokenKind::Keyword(Keyword::Use) => {
                items.push(Item::Use(name_after(tokens, index, "module")?));
                index = skip_to_line(tokens, index + 1);
            }
            TokenKind::Keyword(Keyword::Burrow) => {
                items.push(Item::Module(name_after(tokens, index, "module")?));
                index = skip_to_line(tokens, index + 1);
            }
            TokenKind::Keyword(Keyword::Probe) => {
                let name = string_after(tokens, index, "probe")?;
                items.push(Item::Probe(name));
                index = skip_balanced_block(tokens, index)?;
            }
            TokenKind::Keyword(Keyword::Pack) => {
                items.push(Item::Pack(pack_config(tokens, index)?));
                index = skip_balanced_block(tokens, index)?;
            }
            _ => {
                items.push(Item::Statement);
                index = skip_to_line(tokens, index + 1);
            }
        }
    }
    Ok(Program { items })
}

fn name_after(tokens: &[Token], index: usize, label: &str) -> Result<String, Diagnostic> {
    tokens
        .get(index + 1)
        .and_then(|token| match &token.kind {
            TokenKind::Identifier(name) => Some(name.clone()),
            TokenKind::Keyword(keyword) => Some(keyword.as_str().to_string()),
            _ => None,
        })
        .ok_or_else(|| Diagnostic::new(Category::Syntax, format!("expected {label} name")))
}

fn string_after(tokens: &[Token], index: usize, label: &str) -> Result<String, Diagnostic> {
    tokens
        .get(index + 1)
        .and_then(|token| match &token.kind {
            TokenKind::String(value) => Some(value.clone()),
            _ => None,
        })
        .ok_or_else(|| Diagnostic::new(Category::Syntax, format!("expected {label} string")))
}

fn pack_config(tokens: &[Token], index: usize) -> Result<PackConfig, Diagnostic> {
    let mut config = PackConfig::default();
    let mut cursor = index + 1;
    while cursor < tokens.len() {
        match &tokens[cursor].kind {
            TokenKind::Keyword(Keyword::Crate) => {
                config.crate_path = tokens.get(cursor + 1).and_then(string_value);
                cursor += 2;
            }
            TokenKind::Keyword(Keyword::Pelt) => {
                config.pelt = tokens.get(cursor + 1).and_then(string_value);
                cursor += 2;
            }
            TokenKind::Identifier(name) if name == "entry" => {
                config.entry = tokens.get(cursor + 1).and_then(string_value);
                cursor += 2;
            }
            TokenKind::Symbol(symbol) if symbol == "}" => return Ok(config),
            TokenKind::Eof => break,
            _ => cursor += 1,
        }
    }
    Ok(config)
}

fn string_value(token: &Token) -> Option<String> {
    match &token.kind {
        TokenKind::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn skip_to_line(tokens: &[Token], mut index: usize) -> usize {
    while index < tokens.len() {
        if matches!(tokens[index].kind, TokenKind::Newline | TokenKind::Eof) {
            return index + 1;
        }
        index += 1;
    }
    index
}

fn skip_balanced_block(tokens: &[Token], mut index: usize) -> Result<usize, Diagnostic> {
    while index < tokens.len() {
        if matches!(&tokens[index].kind, TokenKind::Symbol(symbol) if symbol == "{") {
            let mut depth = 1;
            index += 1;
            while index < tokens.len() {
                match &tokens[index].kind {
                    TokenKind::Symbol(symbol) if symbol == "{" => depth += 1,
                    TokenKind::Symbol(symbol) if symbol == "}" => {
                        depth -= 1;
                        if depth == 0 {
                            return Ok(index + 1);
                        }
                    }
                    TokenKind::Eof => break,
                    _ => {}
                }
                index += 1;
            }
            return Err(Diagnostic::new(Category::Syntax, "expected } after block"));
        }
        if matches!(tokens[index].kind, TokenKind::Eof) {
            break;
        }
        index += 1;
    }
    Ok(index)
}
