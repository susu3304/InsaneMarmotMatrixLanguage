from . import nodes as n
from .errors import ImmSyntaxError


NEW_RESERVED_WORDS = {
    "web",
    "fetch",
    "grab",
    "howl",
    "wait",
    "scatter",
    "nest",
    "nap",
    "tick",
    "pack",
    "crate",
    "pelt",
    "probe",
    "law",
    "expect",
    "trace",
}


class Parser:
    def __init__(self, tokens):
        self.tokens = tokens
        self.current = 0

    def parse(self):
        items = []
        self._skip_newlines()
        while not self._is_at_end():
            items.append(self._item())
            self._skip_newlines()
        return n.Program(items)

    def _item(self):
        if self._check_keyword("insane") and self._check_next_keyword("howl"):
            self._advance()
            self._consume_keyword("howl", "expected howl after insane")
            return self._howl_item(insane=True)
        if self._match_keyword("howl"):
            return self._howl_item(insane=False)
        if self._check_keyword("insane") and self._check_next_keyword("marmot"):
            self._advance()
            return self._main_def(insane=True)
        if self._check_keyword("marmot"):
            return self._main_def(insane=False)
        if self._match_keyword("dig"):
            return self._function_def()
        if self._match_keyword("use"):
            name = self._consume_name("expected module name after use")
            return n.UseStmt(name)
        if self._match_keyword("burrow"):
            name = self._consume_name("expected module name after burrow")
            return n.ModuleDef(name)
        if self._match_keyword("den"):
            return self._den_def()
        if self._match_keyword("mask"):
            return self._mask_def()
        if self._match_keyword("probe"):
            return self._probe_def()
        if self._match_keyword("pack"):
            return self._pack_def()
        return self._statement()

    def _howl_item(self, insane):
        if self._check_keyword("marmot"):
            return self._howl_main_def(insane)
        if self._match_keyword("dig"):
            return self._howl_function_def()
        self._error_current("expected marmot main or dig after howl")

    def _main_def(self, insane):
        self._consume_keyword("marmot", "expected marmot")
        name = self._consume_name("expected main after marmot")
        if name != "main":
            self._error_previous("expected marmot main")
        return n.MainDef(self._block(), insane=insane)

    def _howl_main_def(self, insane):
        self._consume_keyword("marmot", "expected marmot")
        name = self._consume_name("expected main after marmot")
        if name != "main":
            self._error_previous("expected howl marmot main")
        return n.HowlMainDef(self._block(), insane=insane)

    def _function_def(self):
        name = self._consume_identifier("expected function name")
        params, return_type = self._function_signature_after_name()
        return n.FunctionDef(name, params, return_type, self._block())

    def _howl_function_def(self):
        name = self._consume_identifier("expected howl function name")
        params, return_type = self._function_signature_after_name()
        return n.HowlFunctionDef(name, params, return_type, self._block())

    def _function_signature_after_name(self):
        self._consume_symbol("(", "expected ( after function name")
        params = []
        self._skip_newlines()
        if not self._check_symbol(")"):
            while True:
                param_name = self._consume_identifier("expected parameter name")
                param_type = None
                if self._match_symbol(":"):
                    param_type = self._type_until({",", ")"})
                params.append(n.Param(param_name, param_type))
                self._skip_newlines()
                if not self._match_symbol(","):
                    break
                self._skip_newlines()
        self._consume_symbol(")", "expected ) after parameters")
        return_type = None
        if self._match_symbol("->"):
            return_type = self._type_until({"{"})
        return params, return_type

    def _probe_def(self):
        if not self._match("STRING"):
            self._error_current("expected probe name string")
        name = self._previous().literal
        return n.ProbeDef(name, self._block())

    def _pack_def(self):
        self._consume_symbol("{", "expected { after pack")
        values = {"entry": None, "crate": None, "pelt": None}
        self._skip_newlines()
        while not self._check_symbol("}") and not self._is_at_end():
            key = self._consume_name("expected pack item")
            if key not in values:
                self._error_previous("expected entry, crate, or pelt in pack")
            if values[key] is not None:
                self._error_previous(f"duplicate pack item {key}")
            if not self._match("STRING"):
                self._error_current(f"expected string after {key}")
            values[key] = self._previous().literal
            self._skip_newlines()
        self._consume_symbol("}", "expected } after pack")
        return n.PackDef(values["entry"], values["crate"], values["pelt"])

    def _den_def(self):
        name = self._consume_identifier("expected den name")
        parent = None
        masks = []
        if self._match_keyword("under"):
            parent = self._consume_identifier("expected parent den name after under")
        if self._match_keyword("wear"):
            while True:
                masks.append(self._consume_identifier("expected mask name after wear"))
                if not self._match_symbol(","):
                    break
        self._consume_symbol("{", "expected { after den header")
        members = []
        self._skip_newlines()
        while not self._check_symbol("}") and not self._is_at_end():
            members.append(self._den_member())
            self._skip_newlines()
        self._consume_symbol("}", "expected } after den body")
        return n.DenDef(name, parent, masks, members)

    def _den_member(self):
        access = "fang"
        if self._match_keyword("fur"):
            access = "fur"
        elif self._match_keyword("fang"):
            access = "fang"
        if self._match_keyword("let"):
            name = self._consume_identifier("expected field name")
            type_name = None
            if self._match_symbol(":"):
                type_name = self._type_until({"=", "}"})
            expr = None
            if self._match_symbol("="):
                self._skip_newlines()
                expr = self._expression()
            return n.FieldDef(name, type_name, expr, access)
        if self._match_keyword("dig"):
            name = self._consume_identifier("expected method name")
            params, return_type = self._function_signature_after_name()
            body = self._block()
            return n.MethodDef(name, params, return_type, body, access)
        self._error_current("expected field or method in den")

    def _mask_def(self):
        name = self._consume_identifier("expected mask name")
        self._consume_symbol("{", "expected { after mask name")
        methods = []
        self._skip_newlines()
        while not self._check_symbol("}") and not self._is_at_end():
            if not self._match_keyword("dig"):
                self._error_current("mask members must be method signatures")
            method_name = self._consume_identifier("expected mask method name")
            params, return_type = self._function_signature_after_name()
            if self._check_symbol("{"):
                self._error_current("mask method cannot have a body")
            methods.append(n.MaskMethod(method_name, params, return_type))
            self._skip_newlines()
        self._consume_symbol("}", "expected } after mask body")
        return n.MaskDef(name, methods)

    def _statement(self):
        if self._match_keyword("let"):
            return self._let_stmt(const=False)
        if self._match_keyword("stash"):
            return self._let_stmt(const=True)
        if self._match_keyword("if"):
            return self._if_stmt()
        if self._match_keyword("for"):
            return self._for_stmt(insane=False)
        if self._match_keyword("while"):
            return self._while_stmt()
        if self._match_keyword("return"):
            if self._at_statement_end():
                return n.ReturnStmt(None)
            return n.ReturnStmt(self._expression())
        if self._match_keyword("break"):
            return n.BreakStmt()
        if self._match_keyword("continue"):
            return n.ContinueStmt()
        if self._match_keyword("squeak"):
            return self._squeak_stmt()
        if self._match_keyword("panic"):
            return n.PanicStmt(self._expression())
        if self._match_keyword("expect"):
            return n.ExpectStmt(self._expression())
        if self._match_keyword("trace"):
            return self._trace_stmt()
        if self._match_keyword("try"):
            return self._try_stmt(insane=False)
        if self._match_keyword("insane"):
            if self._match_keyword("for"):
                return self._for_stmt(insane=True)
            if self._match_keyword("try"):
                return self._try_stmt(insane=True)
            if self._match_keyword("scatter"):
                return n.ExprStmt(n.ScatterExpr(self._expression(), insane=True))
            if self._check_symbol("{"):
                return n.InsaneBlock(self._block())
            self._error_current("expected block, for, or try after insane")
        return n.ExprStmt(self._expression())

    def _let_stmt(self, const):
        name = self._consume_identifier("expected variable name")
        type_name = None
        if self._match_symbol(":"):
            type_name = self._type_until({"="})
        self._consume_symbol("=", "expected = in declaration")
        self._skip_newlines()
        return n.LetStmt(name, self._expression(), type_name, const)

    def _if_stmt(self):
        condition = self._expression()
        then_body = self._block()
        self._skip_newlines()
        else_body = None
        if self._match_keyword("else"):
            if self._match_keyword("if"):
                else_body = self._if_stmt()
            else:
                else_body = self._block()
        return n.IfStmt(condition, then_body, else_body)

    def _for_stmt(self, insane):
        name = self._consume_identifier("expected loop variable")
        self._consume_keyword("in", "expected in after loop variable")
        iterable = self._expression()
        body = self._block()
        return n.ForStmt(name, iterable, body, insane)

    def _while_stmt(self):
        condition = self._expression()
        return n.WhileStmt(condition, self._block())

    def _try_stmt(self, insane):
        body = self._block()
        self._skip_newlines()
        catch_name = None
        catch_body = None
        if self._match_keyword("catch"):
            catch_name = self._consume_name("expected catch variable")
            catch_body = self._block()
        elif not insane:
            self._error_current("try requires catch; use insane try to swallow errors")
        return n.TryStmt(body, catch_name, catch_body, insane)

    def _squeak_stmt(self):
        exprs = []
        if self._at_statement_end():
            return n.SqueakStmt(exprs)
        while True:
            exprs.append(self._expression())
            if not self._match_symbol(","):
                break
        return n.SqueakStmt(exprs)

    def _trace_stmt(self):
        exprs = []
        if self._at_statement_end():
            return n.TraceStmt(exprs)
        while True:
            exprs.append(self._expression())
            if not self._match_symbol(","):
                break
        return n.TraceStmt(exprs)

    def _block(self):
        self._consume_symbol("{", "expected {")
        statements = []
        self._skip_newlines()
        while not self._check_symbol("}") and not self._is_at_end():
            statements.append(self._statement())
            self._skip_newlines()
        self._consume_symbol("}", "expected }")
        return statements

    def _expression(self):
        return self._lambda_or_assignment()

    def _lambda_or_assignment(self):
        if self._check("IDENT") and self._peek_next().lexeme == "=>":
            name = self._consume_identifier("expected lambda parameter")
            self._advance()
            return n.LambdaExpr([name], self._lambda_body(), self._last_lambda_body_was_block)
        if self._check_symbol("(") and self._looks_like_parenthesized_lambda():
            params = self._parse_lambda_params()
            self._consume_symbol("=>", "expected => after lambda parameters")
            return n.LambdaExpr(params, self._lambda_body(), self._last_lambda_body_was_block)

        expr = self._tunnel()
        if self._match_symbol("="):
            return n.Assign(expr, self._lambda_or_assignment())
        return expr

    def _lambda_body(self):
        if self._check_symbol("{"):
            self._last_lambda_body_was_block = True
            return self._block()
        self._last_lambda_body_was_block = False
        return self._lambda_or_assignment()

    def _parse_lambda_params(self):
        params = []
        self._consume_symbol("(", "expected (")
        self._skip_newlines()
        if not self._check_symbol(")"):
            while True:
                params.append(self._consume_identifier("expected lambda parameter"))
                self._skip_newlines()
                if not self._match_symbol(","):
                    break
                self._skip_newlines()
        self._consume_symbol(")", "expected ) after lambda parameters")
        return params

    def _looks_like_parenthesized_lambda(self):
        i = self.current
        depth = 0
        while i < len(self.tokens):
            tok = self.tokens[i]
            if tok.kind == "NEWLINE":
                i += 1
                continue
            if tok.lexeme == "(":
                depth += 1
            elif tok.lexeme == ")":
                depth -= 1
                if depth == 0:
                    j = i + 1
                    while j < len(self.tokens) and self.tokens[j].kind == "NEWLINE":
                        j += 1
                    return j < len(self.tokens) and self.tokens[j].lexeme == "=>"
            i += 1
        return False

    def _tunnel(self):
        expr = self._range()
        while True:
            saved = self.current
            self._skip_newlines()
            if self._match_keyword("tunnel"):
                expr = n.TunnelExpr(expr, self._range())
            else:
                self.current = saved
                break
        return expr

    def _range(self):
        expr = self._logic_or()
        if self._match_symbol(".."):
            expr = n.RangeExpr(expr, self._logic_or())
        return expr

    def _logic_or(self):
        expr = self._logic_and()
        while self._match_symbol("||"):
            expr = n.Binary(expr, "||", self._logic_and())
        return expr

    def _logic_and(self):
        expr = self._equality()
        while self._match_symbol("&&"):
            expr = n.Binary(expr, "&&", self._equality())
        return expr

    def _equality(self):
        expr = self._comparison()
        while self._match_any_symbol("==", "!="):
            op = self._previous().lexeme
            expr = n.Binary(expr, op, self._comparison())
        return expr

    def _comparison(self):
        expr = self._term()
        while self._match_any_symbol("<", "<=", ">", ">="):
            op = self._previous().lexeme
            expr = n.Binary(expr, op, self._term())
        return expr

    def _term(self):
        expr = self._factor()
        while self._match_any_symbol("+", "-"):
            op = self._previous().lexeme
            expr = n.Binary(expr, op, self._factor())
        return expr

    def _factor(self):
        expr = self._unary()
        while self._match_any_symbol("*", "/", "%"):
            op = self._previous().lexeme
            expr = n.Binary(expr, op, self._unary())
        return expr

    def _unary(self):
        if self._match_keyword("wait"):
            return n.WaitExpr(self._unary())
        if self._match_keyword("scatter"):
            return n.ScatterExpr(self._unary(), insane=False)
        if self._check_keyword("insane") and self._check_next_keyword("scatter"):
            self._advance()
            self._advance()
            return n.ScatterExpr(self._unary(), insane=True)
        if self._match_any_symbol("!", "-"):
            return n.Unary(self._previous().lexeme, self._unary())
        return self._call()

    def _call(self):
        expr = self._primary()
        while True:
            if self._match_symbol("("):
                args = self._arguments(")")
                expr = n.Call(expr, args)
            elif self._match_symbol("["):
                args = self._arguments("]")
                expr = n.Index(expr, args)
            elif self._match_symbol("."):
                name = self._consume_name("expected member name after .")
                expr = n.Member(expr, name)
            else:
                break
        return expr

    def _arguments(self, closer):
        args = []
        self._skip_newlines()
        if not self._check_symbol(closer):
            while True:
                args.append(self._expression())
                self._skip_newlines()
                if not self._match_symbol(","):
                    break
                self._skip_newlines()
                if self._check_symbol(closer):
                    break
        self._consume_symbol(closer, f"expected {closer}")
        return args

    def _primary(self):
        if self._match("NUMBER"):
            return n.Literal(self._previous().literal)
        if self._match("STRING"):
            return n.Literal(self._previous().literal)
        if self._match_keyword("true"):
            return n.Literal(True)
        if self._match_keyword("false"):
            return n.Literal(False)
        if self._match_keyword("null"):
            return n.Literal(None)
        if self._match_keyword("sniff"):
            return n.SniffExpr()
        if self._match_keyword("self"):
            return n.Var("self")
        if self._match_keyword("under"):
            return n.Var("under")
        if self._match_keyword("web"):
            return n.Var("web")
        if self._match_keyword("tick"):
            return n.Var("tick")
        if self._match_keyword("nap"):
            return n.Var("nap")
        if self._match_keyword("law"):
            return n.Var("law")
        if self._match_keyword("hatch"):
            name = self._consume_identifier("expected den name after hatch")
            self._consume_symbol("(", "expected ( after hatch den name")
            args = self._arguments(")")
            return n.HatchExpr(name, args)
        if self._match_keyword("insane"):
            self._consume_keyword("choose", "expected choose after insane in expression")
            return n.InsaneChoose(self._expression())
        if self._match_keyword("matrix"):
            rows = self._array_literal().items
            return n.MatrixLiteral(rows)
        if self._match_keyword("nest"):
            return self._nest_expr()
        if self._match_symbol("@"):
            name = self._consume_name("expected point after @")
            if name != "point":
                self._error_previous("expected @point")
            self._consume_symbol("(", "expected ( after @point")
            x = self._expression()
            self._consume_symbol(",", "expected , in @point")
            y = self._expression()
            self._consume_symbol(")", "expected ) after @point")
            return n.PointLiteral(x, y)
        if self._match_symbol("["):
            return self._array_literal(open_consumed=True)
        if self._match_symbol("{"):
            return self._map_literal(open_consumed=True)
        if self._match_symbol("("):
            expr = self._expression()
            self._consume_symbol(")", "expected ) after expression")
            return expr
        if self._match("IDENT"):
            return n.Var(self._previous().lexeme)
        self._error_current("expected expression")

    def _nest_expr(self):
        self._consume_symbol("{", "expected { after nest")
        items = []
        self._skip_newlines()
        while not self._check_symbol("}") and not self._is_at_end():
            insane = False
            if self._match_keyword("insane"):
                insane = True
            self._consume_keyword("scatter", "nest only accepts scatter expressions")
            items.append(n.ScatterExpr(self._expression(), insane=insane))
            self._skip_newlines()
        self._consume_symbol("}", "expected } after nest")
        return n.NestExpr(items)

    def _array_literal(self, open_consumed=False):
        if not open_consumed:
            self._consume_symbol("[", "expected [")
        items = []
        self._skip_newlines()
        if not self._check_symbol("]"):
            while True:
                items.append(self._expression())
                self._skip_newlines()
                if not self._match_symbol(","):
                    break
                self._skip_newlines()
                if self._check_symbol("]"):
                    break
        self._consume_symbol("]", "expected ]")
        return n.ArrayLiteral(items)

    def _map_literal(self, open_consumed=False):
        if not open_consumed:
            self._consume_symbol("{", "expected {")
        pairs = []
        self._skip_newlines()
        if not self._check_symbol("}"):
            while True:
                key = self._expression()
                self._consume_symbol(":", "expected : in map literal")
                self._skip_newlines()
                value = self._expression()
                pairs.append((key, value))
                self._skip_newlines()
                if not self._match_symbol(","):
                    break
                self._skip_newlines()
                if self._check_symbol("}"):
                    break
        self._consume_symbol("}", "expected }")
        return n.MapLiteral(pairs)

    def _type_until(self, stop_symbols):
        pieces = []
        depth = 0
        self._skip_newlines()
        while not self._is_at_end():
            tok = self._peek()
            if tok.kind == "NEWLINE":
                break
            if depth == 0 and tok.kind == "SYMBOL" and tok.lexeme in stop_symbols:
                break
            if tok.lexeme == "<":
                depth += 1
            elif tok.lexeme == ">":
                depth -= 1
            pieces.append(tok.lexeme)
            self._advance()
        if not pieces:
            self._error_current("expected type")
        return "".join(pieces)

    def _at_statement_end(self):
        return self._check("NEWLINE") or self._check_symbol("}") or self._check("EOF")

    def _skip_newlines(self):
        while self._match("NEWLINE"):
            pass

    def _match(self, kind):
        if self._check(kind):
            self._advance()
            return True
        return False

    def _match_keyword(self, name):
        if self._check_keyword(name):
            self._advance()
            return True
        return False

    def _match_symbol(self, symbol):
        if self._check_symbol(symbol):
            self._advance()
            return True
        return False

    def _match_any_symbol(self, *symbols):
        for symbol in symbols:
            if self._check_symbol(symbol):
                self._advance()
                return True
        return False

    def _consume_keyword(self, name, message):
        if self._match_keyword(name):
            return self._previous()
        self._error_current(message)

    def _consume_symbol(self, symbol, message):
        if self._match_symbol(symbol):
            return self._previous()
        self._error_current(message)

    def _consume_name(self, message):
        if self._match("IDENT") or self._match("KEYWORD"):
            return self._previous().lexeme
        self._error_current(message)

    def _consume_identifier(self, message):
        if self._match("IDENT"):
            return self._previous().lexeme
        if self._check("KEYWORD"):
            if self._peek().lexeme in NEW_RESERVED_WORDS:
                self._error_current(f"{self._peek().lexeme} is a reserved keyword")
            return self._advance().lexeme
        self._error_current(message)

    def _check(self, kind):
        if self._is_at_end() and kind != "EOF":
            return False
        return self._peek().kind == kind

    def _check_keyword(self, name):
        return self._peek().is_keyword(name)

    def _check_next_keyword(self, name):
        if self.current + 1 >= len(self.tokens):
            return False
        return self.tokens[self.current + 1].is_keyword(name)

    def _check_symbol(self, symbol):
        return self._peek().kind == "SYMBOL" and self._peek().lexeme == symbol

    def _advance(self):
        if not self._is_at_end():
            self.current += 1
        return self._previous()

    def _is_at_end(self):
        return self._peek().kind == "EOF"

    def _peek(self):
        return self.tokens[self.current]

    def _peek_next(self):
        if self.current + 1 >= len(self.tokens):
            return self.tokens[-1]
        return self.tokens[self.current + 1]

    def _previous(self):
        return self.tokens[self.current - 1]

    def _error_current(self, message):
        tok = self._peek()
        raise ImmSyntaxError(message, tok.line, tok.column)

    def _error_previous(self, message):
        tok = self._previous()
        raise ImmSyntaxError(message, tok.line, tok.column)


def parse(tokens):
    return Parser(tokens).parse()
