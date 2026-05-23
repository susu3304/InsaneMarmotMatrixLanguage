use crate::ast::*;
use crate::diagnostics::{Category, Diagnostic};
use crate::lexer::lex;
use crate::token::{Keyword, NumberLiteral, Token, TokenKind};

pub fn parse_source(file_id: usize, source: &str) -> Result<Program, Diagnostic> {
    let tokens = lex(file_id, source)?;
    Parser::new(tokens).parse()
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    last_lambda_body_was_block: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            last_lambda_body_was_block: false,
        }
    }

    pub fn parse(&mut self) -> Result<Program, Diagnostic> {
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() {
            items.push(self.item()?);
            self.skip_newlines();
        }
        Ok(Program { items })
    }

    fn item(&mut self) -> Result<Item, Diagnostic> {
        if self.check_keyword(Keyword::Insane) && self.check_next_keyword(Keyword::Howl) {
            self.advance();
            self.consume_keyword(Keyword::Howl, "expected howl after insane")?;
            return self.howl_item(true);
        }
        if self.match_keyword(Keyword::Howl) {
            return self.howl_item(false);
        }
        if self.check_keyword(Keyword::Insane) && self.check_next_keyword(Keyword::Marmot) {
            self.advance();
            return self.main_def(true);
        }
        if self.check_keyword(Keyword::Marmot) {
            return self.main_def(false);
        }
        if self.match_keyword(Keyword::Dig) {
            return Ok(Item::Function(self.function_def_after_dig()?));
        }
        if self.match_keyword(Keyword::Use) {
            return Ok(Item::Use(
                self.consume_name("expected module name after use")?,
            ));
        }
        if self.match_keyword(Keyword::Burrow) {
            return Ok(Item::Module(
                self.consume_name("expected module name after burrow")?,
            ));
        }
        if self.match_keyword(Keyword::Den) {
            return Ok(Item::Den(self.den_def()?));
        }
        if self.match_keyword(Keyword::Mask) {
            return Ok(Item::Mask(self.mask_def()?));
        }
        if self.match_keyword(Keyword::Probe) {
            return self.probe_def();
        }
        if self.match_keyword(Keyword::Pack) {
            return Ok(Item::Pack(self.pack_def()?));
        }
        Ok(Item::Stmt(self.statement()?))
    }

    fn howl_item(&mut self, insane: bool) -> Result<Item, Diagnostic> {
        if self.check_keyword(Keyword::Marmot) {
            return self.howl_main_def(insane);
        }
        if self.match_keyword(Keyword::Dig) {
            return Ok(Item::HowlFunction(self.function_def_after_dig()?));
        }
        Err(self.error_current("expected marmot main or dig after howl"))
    }

    fn main_def(&mut self, insane: bool) -> Result<Item, Diagnostic> {
        self.consume_keyword(Keyword::Marmot, "expected marmot")?;
        let name = self.consume_name("expected main after marmot")?;
        if name != "main" {
            return Err(self.error_previous("expected marmot main"));
        }
        Ok(Item::Main {
            body: self.block()?,
            insane,
        })
    }

    fn howl_main_def(&mut self, insane: bool) -> Result<Item, Diagnostic> {
        self.consume_keyword(Keyword::Marmot, "expected marmot")?;
        let name = self.consume_name("expected main after marmot")?;
        if name != "main" {
            return Err(self.error_previous("expected howl marmot main"));
        }
        Ok(Item::HowlMain {
            body: self.block()?,
            insane,
        })
    }

    fn function_def_after_dig(&mut self) -> Result<FunctionDef, Diagnostic> {
        let name = self.consume_identifier("expected function name")?;
        let (params, return_type) = self.function_signature_after_name()?;
        Ok(FunctionDef {
            name,
            params,
            return_type,
            body: self.block()?,
        })
    }

    fn function_signature_after_name(
        &mut self,
    ) -> Result<(Vec<Param>, Option<String>), Diagnostic> {
        self.consume_symbol("(", "expected ( after function name")?;
        let mut params = Vec::new();
        self.skip_newlines();
        if !self.check_symbol(")") {
            loop {
                let name = self.consume_identifier("expected parameter name")?;
                let type_name = if self.match_symbol(":") {
                    Some(self.type_until(&[",", ")"])?)
                } else {
                    None
                };
                params.push(Param { name, type_name });
                self.skip_newlines();
                if !self.match_symbol(",") {
                    break;
                }
                self.skip_newlines();
            }
        }
        self.consume_symbol(")", "expected ) after parameters")?;
        let return_type = if self.match_symbol("->") {
            Some(self.type_until(&["{"])?)
        } else {
            None
        };
        Ok((params, return_type))
    }

    fn probe_def(&mut self) -> Result<Item, Diagnostic> {
        let name = match self.peek().kind.clone() {
            TokenKind::String(value) => {
                self.advance();
                value
            }
            _ => return Err(self.error_current("expected probe name string")),
        };
        Ok(Item::Probe {
            name,
            body: self.block()?,
        })
    }

    fn pack_def(&mut self) -> Result<PackConfig, Diagnostic> {
        self.consume_symbol("{", "expected { after pack")?;
        let mut config = PackConfig::default();
        self.skip_newlines();
        while !self.check_symbol("}") && !self.is_at_end() {
            let key = self.consume_name("expected pack item")?;
            let value = match self.peek().kind.clone() {
                TokenKind::String(value) => {
                    self.advance();
                    value
                }
                _ => return Err(self.error_current(&format!("expected string after {key}"))),
            };
            match key.as_str() {
                "entry" if config.entry.is_none() => config.entry = Some(value),
                "crate" if config.crate_path.is_none() => config.crate_path = Some(value),
                "pelt" if config.pelt.is_none() => config.pelt = Some(value),
                "entry" | "crate" | "pelt" => {
                    return Err(self.error_previous(&format!("duplicate pack item {key}")));
                }
                _ => return Err(self.error_previous("expected entry, crate, or pelt in pack")),
            }
            self.skip_newlines();
        }
        self.consume_symbol("}", "expected } after pack")?;
        Ok(config)
    }

    fn den_def(&mut self) -> Result<DenDef, Diagnostic> {
        let name = self.consume_identifier("expected den name")?;
        let parent = if self.match_keyword(Keyword::Under) {
            Some(self.consume_identifier("expected parent den name after under")?)
        } else {
            None
        };
        let mut masks = Vec::new();
        if self.match_keyword(Keyword::Wear) {
            loop {
                masks.push(self.consume_identifier("expected mask name after wear")?);
                if !self.match_symbol(",") {
                    break;
                }
            }
        }
        self.consume_symbol("{", "expected { after den header")?;
        let mut members = Vec::new();
        self.skip_newlines();
        while !self.check_symbol("}") && !self.is_at_end() {
            members.push(self.den_member()?);
            self.skip_newlines();
        }
        self.consume_symbol("}", "expected } after den body")?;
        Ok(DenDef {
            name,
            parent,
            masks,
            members,
        })
    }

    fn den_member(&mut self) -> Result<DenMember, Diagnostic> {
        let mut access = Access::Fang;
        if self.match_keyword(Keyword::Fur) {
            access = Access::Fur;
        } else if self.match_keyword(Keyword::Fang) {
            access = Access::Fang;
        }
        if self.match_keyword(Keyword::Let) {
            let name = self.consume_identifier("expected field name")?;
            let type_name = if self.match_symbol(":") {
                Some(self.type_until(&["=", "}"])?)
            } else {
                None
            };
            let expr = if self.match_symbol("=") {
                self.skip_newlines();
                Some(self.expression()?)
            } else {
                None
            };
            return Ok(DenMember::Field(FieldDef {
                name,
                type_name,
                expr,
                access,
            }));
        }
        if self.match_keyword(Keyword::Dig) {
            let name = self.consume_identifier("expected method name")?;
            let (params, return_type) = self.function_signature_after_name()?;
            return Ok(DenMember::Method(MethodDef {
                name,
                params,
                return_type,
                body: self.block()?,
                access,
            }));
        }
        Err(self.error_current("expected field or method in den"))
    }

    fn mask_def(&mut self) -> Result<MaskDef, Diagnostic> {
        let name = self.consume_identifier("expected mask name")?;
        self.consume_symbol("{", "expected { after mask name")?;
        let mut methods = Vec::new();
        self.skip_newlines();
        while !self.check_symbol("}") && !self.is_at_end() {
            self.consume_keyword(Keyword::Dig, "mask members must be method signatures")?;
            let method_name = self.consume_identifier("expected mask method name")?;
            let (params, return_type) = self.function_signature_after_name()?;
            if self.check_symbol("{") {
                return Err(self.error_current("mask method cannot have a body"));
            }
            methods.push(MaskMethod {
                name: method_name,
                params,
                return_type,
            });
            self.skip_newlines();
        }
        self.consume_symbol("}", "expected } after mask body")?;
        Ok(MaskDef { name, methods })
    }

    fn statement(&mut self) -> Result<Stmt, Diagnostic> {
        if self.match_keyword(Keyword::Let) {
            return self.let_stmt(false);
        }
        if self.match_keyword(Keyword::Stash) {
            return self.let_stmt(true);
        }
        if self.match_keyword(Keyword::If) {
            return self.if_stmt();
        }
        if self.match_keyword(Keyword::For) {
            return self.for_stmt(false);
        }
        if self.match_keyword(Keyword::While) {
            return self.while_stmt();
        }
        if self.match_keyword(Keyword::Return) {
            if self.at_statement_end() {
                return Ok(Stmt::Return(None));
            }
            return Ok(Stmt::Return(Some(self.expression()?)));
        }
        if self.match_keyword(Keyword::Break) {
            return Ok(Stmt::Break);
        }
        if self.match_keyword(Keyword::Continue) {
            return Ok(Stmt::Continue);
        }
        if self.match_keyword(Keyword::Squeak) {
            return self.squeak_stmt();
        }
        if self.match_keyword(Keyword::Panic) {
            return Ok(Stmt::Panic(self.expression()?));
        }
        if self.match_keyword(Keyword::Expect) {
            return Ok(Stmt::Expect(self.expression()?));
        }
        if self.match_keyword(Keyword::Trace) {
            return self.trace_stmt();
        }
        if self.match_keyword(Keyword::Try) {
            return self.try_stmt(false);
        }
        if self.match_keyword(Keyword::Insane) {
            if self.match_keyword(Keyword::For) {
                return self.for_stmt(true);
            }
            if self.match_keyword(Keyword::Try) {
                return self.try_stmt(true);
            }
            if self.match_keyword(Keyword::Scatter) {
                return Ok(Stmt::Expr(Expr::Scatter {
                    expr: Box::new(self.expression()?),
                    insane: true,
                }));
            }
            if self.check_symbol("{") {
                return Ok(Stmt::InsaneBlock(self.block()?));
            }
            return Err(self.error_current("expected block, for, or try after insane"));
        }
        Ok(Stmt::Expr(self.expression()?))
    }

    fn let_stmt(&mut self, is_const: bool) -> Result<Stmt, Diagnostic> {
        let name = self.consume_identifier("expected variable name")?;
        let type_name = if self.match_symbol(":") {
            Some(self.type_until(&["="])?)
        } else {
            None
        };
        self.consume_symbol("=", "expected = in declaration")?;
        self.skip_newlines();
        Ok(Stmt::Let {
            name,
            expr: self.expression()?,
            type_name,
            is_const,
        })
    }

    fn if_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let condition = self.expression()?;
        let then_body = self.block()?;
        self.skip_newlines();
        let else_body = if self.match_keyword(Keyword::Else) {
            if self.match_keyword(Keyword::If) {
                Some(ElseBody::If(Box::new(self.if_stmt()?)))
            } else {
                Some(ElseBody::Block(self.block()?))
            }
        } else {
            None
        };
        Ok(Stmt::If {
            condition,
            then_body,
            else_body,
        })
    }

    fn for_stmt(&mut self, insane: bool) -> Result<Stmt, Diagnostic> {
        let name = self.consume_identifier("expected loop variable")?;
        self.consume_keyword(Keyword::In, "expected in after loop variable")?;
        let iterable = self.expression()?;
        let body = self.block()?;
        Ok(Stmt::For {
            name,
            iterable,
            body,
            insane,
        })
    }

    fn while_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let condition = self.expression()?;
        Ok(Stmt::While {
            condition,
            body: self.block()?,
        })
    }

    fn try_stmt(&mut self, insane: bool) -> Result<Stmt, Diagnostic> {
        let body = self.block()?;
        self.skip_newlines();
        let mut catch_name = None;
        let mut catch_body = None;
        if self.match_keyword(Keyword::Catch) {
            catch_name = Some(self.consume_name("expected catch variable")?);
            catch_body = Some(self.block()?);
        } else if !insane {
            return Err(self.error_current("try requires catch; use insane try to swallow errors"));
        }
        Ok(Stmt::Try {
            body,
            catch_name,
            catch_body,
            insane,
        })
    }

    fn squeak_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        Ok(Stmt::Squeak(self.expression_list()?))
    }

    fn trace_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        Ok(Stmt::Trace(self.expression_list()?))
    }

    fn expression_list(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        let mut exprs = Vec::new();
        if self.at_statement_end() {
            return Ok(exprs);
        }
        loop {
            exprs.push(self.expression()?);
            if !self.match_symbol(",") {
                break;
            }
        }
        Ok(exprs)
    }

    fn block(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        self.consume_symbol("{", "expected {")?;
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.check_symbol("}") && !self.is_at_end() {
            statements.push(self.statement()?);
            self.skip_newlines();
        }
        self.consume_symbol("}", "expected }")?;
        Ok(statements)
    }

    fn expression(&mut self) -> Result<Expr, Diagnostic> {
        self.lambda_or_assignment()
    }

    fn lambda_or_assignment(&mut self) -> Result<Expr, Diagnostic> {
        if self.check_ident() && self.peek_next_lexeme() == Some("=>") {
            let name = self.consume_identifier("expected lambda parameter")?;
            self.advance();
            let body = self.lambda_body()?;
            return Ok(Expr::Lambda {
                params: vec![name],
                body,
            });
        }
        if self.check_symbol("(") && self.looks_like_parenthesized_lambda() {
            let params = self.parse_lambda_params()?;
            self.consume_symbol("=>", "expected => after lambda parameters")?;
            let body = self.lambda_body()?;
            return Ok(Expr::Lambda { params, body });
        }
        let expr = self.tunnel()?;
        if self.match_symbol("=") {
            return Ok(Expr::Assign {
                target: Box::new(expr),
                value: Box::new(self.lambda_or_assignment()?),
            });
        }
        Ok(expr)
    }

    fn lambda_body(&mut self) -> Result<LambdaBody, Diagnostic> {
        if self.check_symbol("{") {
            self.last_lambda_body_was_block = true;
            return Ok(LambdaBody::Block(self.block()?));
        }
        self.last_lambda_body_was_block = false;
        Ok(LambdaBody::Expr(Box::new(self.lambda_or_assignment()?)))
    }

    fn parse_lambda_params(&mut self) -> Result<Vec<String>, Diagnostic> {
        let mut params = Vec::new();
        self.consume_symbol("(", "expected (")?;
        self.skip_newlines();
        if !self.check_symbol(")") {
            loop {
                params.push(self.consume_identifier("expected lambda parameter")?);
                self.skip_newlines();
                if !self.match_symbol(",") {
                    break;
                }
                self.skip_newlines();
            }
        }
        self.consume_symbol(")", "expected ) after lambda parameters")?;
        Ok(params)
    }

    fn looks_like_parenthesized_lambda(&self) -> bool {
        let mut i = self.current;
        let mut depth = 0_i32;
        while i < self.tokens.len() {
            let tok = &self.tokens[i];
            if matches!(tok.kind, TokenKind::Newline) {
                i += 1;
                continue;
            }
            if tok.lexeme == "(" {
                depth += 1;
            } else if tok.lexeme == ")" {
                depth -= 1;
                if depth == 0 {
                    let mut j = i + 1;
                    while j < self.tokens.len() && matches!(self.tokens[j].kind, TokenKind::Newline)
                    {
                        j += 1;
                    }
                    return j < self.tokens.len() && self.tokens[j].lexeme == "=>";
                }
            }
            i += 1;
        }
        false
    }

    fn tunnel(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.range()?;
        loop {
            let saved = self.current;
            self.skip_newlines();
            if self.match_keyword(Keyword::Tunnel) {
                expr = Expr::Tunnel {
                    left: Box::new(expr),
                    right: Box::new(self.range()?),
                };
            } else {
                self.current = saved;
                break;
            }
        }
        Ok(expr)
    }

    fn range(&mut self) -> Result<Expr, Diagnostic> {
        let expr = self.logic_or()?;
        if self.match_symbol("..") {
            return Ok(Expr::Range {
                start: Box::new(expr),
                end: Box::new(self.logic_or()?),
            });
        }
        Ok(expr)
    }

    fn logic_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.logic_and()?;
        while self.match_symbol("||") {
            expr = Expr::Binary {
                left: Box::new(expr),
                op: "||".to_string(),
                right: Box::new(self.logic_and()?),
            };
        }
        Ok(expr)
    }

    fn logic_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.equality()?;
        while self.match_symbol("&&") {
            expr = Expr::Binary {
                left: Box::new(expr),
                op: "&&".to_string(),
                right: Box::new(self.equality()?),
            };
        }
        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.comparison()?;
        while self.match_any_symbol(&["==", "!="]) {
            let op = self.previous().lexeme.clone();
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(self.comparison()?),
            };
        }
        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.term()?;
        while self.match_any_symbol(&["<", "<=", ">", ">="]) {
            let op = self.previous().lexeme.clone();
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(self.term()?),
            };
        }
        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.factor()?;
        while self.match_any_symbol(&["+", "-"]) {
            let op = self.previous().lexeme.clone();
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(self.factor()?),
            };
        }
        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.unary()?;
        while self.match_any_symbol(&["*", "/", "%"]) {
            let op = self.previous().lexeme.clone();
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(self.unary()?),
            };
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, Diagnostic> {
        if self.match_keyword(Keyword::Wait) {
            return Ok(Expr::Wait(Box::new(self.unary()?)));
        }
        if self.match_keyword(Keyword::Scatter) {
            return Ok(Expr::Scatter {
                expr: Box::new(self.unary()?),
                insane: false,
            });
        }
        if self.check_keyword(Keyword::Insane) && self.check_next_keyword(Keyword::Scatter) {
            self.advance();
            self.advance();
            return Ok(Expr::Scatter {
                expr: Box::new(self.unary()?),
                insane: true,
            });
        }
        if self.match_any_symbol(&["!", "-"]) {
            let op = self.previous().lexeme.clone();
            return Ok(Expr::Unary {
                op,
                expr: Box::new(self.unary()?),
            });
        }
        self.call()
    }

    fn call(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.primary()?;
        loop {
            if self.match_symbol("(") {
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args: self.arguments(")")?,
                };
            } else if self.match_symbol("[") {
                expr = Expr::Index {
                    target: Box::new(expr),
                    args: self.arguments("]")?,
                };
            } else if self.match_symbol(".") {
                let name = self.consume_name("expected member name after .")?;
                expr = Expr::Member {
                    target: Box::new(expr),
                    name,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn arguments(&mut self, closer: &str) -> Result<Vec<Expr>, Diagnostic> {
        let mut args = Vec::new();
        self.skip_newlines();
        if !self.check_symbol(closer) {
            loop {
                args.push(self.expression()?);
                self.skip_newlines();
                if !self.match_symbol(",") {
                    break;
                }
                self.skip_newlines();
                if self.check_symbol(closer) {
                    break;
                }
            }
        }
        self.consume_symbol(closer, &format!("expected {closer}"))?;
        Ok(args)
    }

    fn primary(&mut self) -> Result<Expr, Diagnostic> {
        match self.peek().kind.clone() {
            TokenKind::Number(NumberLiteral::Int(value)) => {
                self.advance();
                return Ok(Expr::Literal(Literal::Int(value)));
            }
            TokenKind::Number(NumberLiteral::Float(value)) => {
                self.advance();
                return Ok(Expr::Literal(Literal::Float(value)));
            }
            TokenKind::String(value) => {
                self.advance();
                return Ok(Expr::Literal(Literal::String(value)));
            }
            _ => {}
        }
        if self.match_keyword(Keyword::True) {
            return Ok(Expr::Literal(Literal::Bool(true)));
        }
        if self.match_keyword(Keyword::False) {
            return Ok(Expr::Literal(Literal::Bool(false)));
        }
        if self.match_keyword(Keyword::Null) {
            return Ok(Expr::Literal(Literal::Null));
        }
        if self.match_keyword(Keyword::Sniff) {
            return Ok(Expr::Sniff);
        }
        for (keyword, name) in [
            (Keyword::SelfValue, "self"),
            (Keyword::Under, "under"),
            (Keyword::Den, "den"),
            (Keyword::Burrow, "burrow"),
            (Keyword::Web, "web"),
            (Keyword::Tick, "tick"),
            (Keyword::Nap, "nap"),
            (Keyword::Law, "law"),
        ] {
            if self.match_keyword(keyword) {
                return Ok(Expr::Var(name.to_string()));
            }
        }
        if self.match_keyword(Keyword::Hatch) {
            let name = self.consume_identifier("expected den name after hatch")?;
            self.consume_symbol("(", "expected ( after hatch den name")?;
            return Ok(Expr::Hatch {
                name,
                args: self.arguments(")")?,
            });
        }
        if self.match_keyword(Keyword::Insane) {
            self.consume_keyword(
                Keyword::Choose,
                "expected choose after insane in expression",
            )?;
            return Ok(Expr::InsaneChoose(Box::new(self.expression()?)));
        }
        if self.match_keyword(Keyword::Matrix) {
            let rows = match self.array_literal(false)? {
                Expr::Array(rows) => rows,
                _ => unreachable!(),
            };
            return Ok(Expr::Matrix(rows));
        }
        if self.match_keyword(Keyword::Nest) {
            return self.nest_expr();
        }
        if self.match_symbol("@") {
            let name = self.consume_name("expected point after @")?;
            if name != "point" {
                return Err(self.error_previous("expected @point"));
            }
            self.consume_symbol("(", "expected ( after @point")?;
            let x = self.expression()?;
            self.consume_symbol(",", "expected , in @point")?;
            let y = self.expression()?;
            self.consume_symbol(")", "expected ) after @point")?;
            return Ok(Expr::Point {
                x: Box::new(x),
                y: Box::new(y),
            });
        }
        if self.match_symbol("[") {
            return self.array_literal(true);
        }
        if self.match_symbol("{") {
            return self.map_literal(true);
        }
        if self.match_symbol("(") {
            let expr = self.expression()?;
            self.consume_symbol(")", "expected ) after expression")?;
            return Ok(expr);
        }
        if let TokenKind::Identifier(name) = self.peek().kind.clone() {
            self.advance();
            return Ok(Expr::Var(name));
        }
        Err(self.error_current("expected expression"))
    }

    fn nest_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.consume_symbol("{", "expected { after nest")?;
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.check_symbol("}") && !self.is_at_end() {
            let insane = self.match_keyword(Keyword::Insane);
            self.consume_keyword(Keyword::Scatter, "nest only accepts scatter expressions")?;
            items.push(Expr::Scatter {
                expr: Box::new(self.expression()?),
                insane,
            });
            self.skip_newlines();
        }
        self.consume_symbol("}", "expected } after nest")?;
        Ok(Expr::Nest(items))
    }

    fn array_literal(&mut self, open_consumed: bool) -> Result<Expr, Diagnostic> {
        if !open_consumed {
            self.consume_symbol("[", "expected [")?;
        }
        let mut items = Vec::new();
        self.skip_newlines();
        if !self.check_symbol("]") {
            loop {
                items.push(self.expression()?);
                self.skip_newlines();
                if !self.match_symbol(",") {
                    break;
                }
                self.skip_newlines();
                if self.check_symbol("]") {
                    break;
                }
            }
        }
        self.consume_symbol("]", "expected ]")?;
        Ok(Expr::Array(items))
    }

    fn map_literal(&mut self, open_consumed: bool) -> Result<Expr, Diagnostic> {
        if !open_consumed {
            self.consume_symbol("{", "expected {")?;
        }
        let mut pairs = Vec::new();
        self.skip_newlines();
        if !self.check_symbol("}") {
            loop {
                let key = self.expression()?;
                self.consume_symbol(":", "expected : in map literal")?;
                self.skip_newlines();
                let value = self.expression()?;
                pairs.push((key, value));
                self.skip_newlines();
                if !self.match_symbol(",") {
                    break;
                }
                self.skip_newlines();
                if self.check_symbol("}") {
                    break;
                }
            }
        }
        self.consume_symbol("}", "expected }")?;
        Ok(Expr::Map(pairs))
    }

    fn type_until(&mut self, stop_symbols: &[&str]) -> Result<String, Diagnostic> {
        let mut pieces = Vec::new();
        let mut depth = 0_i32;
        self.skip_newlines();
        while !self.is_at_end() {
            let tok = self.peek();
            if matches!(tok.kind, TokenKind::Newline) {
                break;
            }
            if depth == 0
                && matches!(tok.kind, TokenKind::Symbol(_))
                && stop_symbols.contains(&tok.lexeme.as_str())
            {
                break;
            }
            if tok.lexeme == "<" {
                depth += 1;
            } else if tok.lexeme == ">" {
                depth -= 1;
            }
            pieces.push(tok.lexeme.clone());
            self.advance();
        }
        if pieces.is_empty() {
            return Err(self.error_current("expected type"));
        }
        Ok(pieces.join(""))
    }

    fn at_statement_end(&self) -> bool {
        self.check_kind_newline() || self.check_symbol("}") || self.check_kind_eof()
    }

    fn skip_newlines(&mut self) {
        while self.check_kind_newline() {
            self.advance();
        }
    }

    fn match_keyword(&mut self, keyword: Keyword) -> bool {
        if self.check_keyword(keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume_keyword(&mut self, keyword: Keyword, message: &str) -> Result<(), Diagnostic> {
        if self.match_keyword(keyword) {
            Ok(())
        } else {
            Err(self.error_current(message))
        }
    }

    fn check_keyword(&self, keyword: Keyword) -> bool {
        matches!(self.peek().kind, TokenKind::Keyword(k) if k == keyword)
    }

    fn check_next_keyword(&self, keyword: Keyword) -> bool {
        self.tokens
            .get(self.current + 1)
            .is_some_and(|token| matches!(token.kind, TokenKind::Keyword(k) if k == keyword))
    }

    fn match_symbol(&mut self, symbol: &str) -> bool {
        if self.check_symbol(symbol) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_any_symbol(&mut self, symbols: &[&str]) -> bool {
        if symbols.iter().any(|symbol| self.check_symbol(symbol)) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume_symbol(&mut self, symbol: &str, message: &str) -> Result<(), Diagnostic> {
        if self.match_symbol(symbol) {
            Ok(())
        } else {
            Err(self.error_current(message))
        }
    }

    fn check_symbol(&self, symbol: &str) -> bool {
        matches!(&self.peek().kind, TokenKind::Symbol(value) if value == symbol)
    }

    fn check_ident(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Identifier(_))
    }

    fn consume_identifier(&mut self, message: &str) -> Result<String, Diagnostic> {
        match self.peek().kind.clone() {
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(name)
            }
            TokenKind::Keyword(keyword) => {
                self.advance();
                Ok(keyword.as_str().to_string())
            }
            _ => Err(self.error_current(message)),
        }
    }

    fn consume_name(&mut self, message: &str) -> Result<String, Diagnostic> {
        match self.peek().kind.clone() {
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(name)
            }
            TokenKind::Keyword(keyword) => {
                self.advance();
                Ok(keyword.as_str().to_string())
            }
            _ => Err(self.error_current(message)),
        }
    }

    fn check_kind_newline(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Newline)
    }

    fn check_kind_eof(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn is_at_end(&self) -> bool {
        self.check_kind_eof()
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn peek_next_lexeme(&self) -> Option<&str> {
        self.tokens
            .get(self.current + 1)
            .map(|token| token.lexeme.as_str())
    }

    fn error_current(&self, message: &str) -> Diagnostic {
        Diagnostic::new(Category::Syntax, message).with_span(self.peek().span)
    }

    fn error_previous(&self, message: &str) -> Diagnostic {
        Diagnostic::new(Category::Syntax, message).with_span(self.previous().span)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_source;

    #[test]
    fn parses_hello_program() {
        let program = parse_source(
            0,
            r#"marmot main {
    squeak "hi"
}
"#,
        )
        .unwrap();
        assert_eq!(program.items.len(), 1);
    }
}
