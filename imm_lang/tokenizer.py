from dataclasses import dataclass

from .errors import ImmSyntaxError


KEYWORDS = {
    "marmot",
    "insane",
    "dig",
    "let",
    "stash",
    "return",
    "if",
    "else",
    "for",
    "in",
    "while",
    "break",
    "continue",
    "true",
    "false",
    "null",
    "matrix",
    "burrow",
    "use",
    "squeak",
    "sniff",
    "panic",
    "try",
    "catch",
    "tunnel",
    "choose",
    "den",
    "hatch",
    "self",
    "init",
    "fur",
    "fang",
    "mask",
    "wear",
    "under",
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


@dataclass(frozen=True)
class Token:
    kind: str
    lexeme: str
    literal: object
    line: int
    column: int

    def is_keyword(self, name):
        return self.kind == "KEYWORD" and self.lexeme == name


class Lexer:
    def __init__(self, source):
        self.source = source.replace("\r\n", "\n").replace("\r", "\n")
        self.tokens = []
        self.start = 0
        self.current = 0
        self.line = 1
        self.column = 1
        self.start_line = 1
        self.start_column = 1

    def tokenize(self):
        while not self._is_at_end():
            self.start = self.current
            self.start_line = self.line
            self.start_column = self.column
            self._scan_token()
        self.tokens.append(Token("EOF", "", None, self.line, self.column))
        return self.tokens

    def _scan_token(self):
        c = self._advance()
        if c in " \t":
            return
        if c == "\n":
            self._add("NEWLINE")
            return
        if c == "#":
            while self._peek() != "\n" and not self._is_at_end():
                self._advance()
            return
        if c == "/" and self._match("*"):
            self._block_comment()
            return

        if c == '"':
            self._string()
            return
        if c.isdigit():
            self._number()
            return
        if self._is_ident_start(c):
            self._identifier()
            return

        two_char = {
            "=": ("=", "=="),
            "!": ("=", "!="),
            "<": ("=", "<="),
            ">": ("=", ">="),
            "&": ("&", "&&"),
            "|": ("|", "||"),
            "-": (">", "->"),
            ".": (".", ".."),
        }
        if c in two_char:
            expected, op = two_char[c]
            if self._match(expected):
                self._add("SYMBOL", op)
                return

        if c == "=" and self._match(">"):
            self._add("SYMBOL", "=>")
            return

        if c in "{}()[],:;+-*/%!=<>.@":
            if c == ";":
                self._add("NEWLINE", ";")
            else:
                self._add("SYMBOL", c)
            return

        raise ImmSyntaxError(f"unexpected character {c!r}", self.start_line, self.start_column)

    def _block_comment(self):
        while not self._is_at_end():
            if self._peek() == "*" and self._peek_next() == "/":
                self._advance()
                self._advance()
                return
            self._advance()
        raise ImmSyntaxError("unterminated block comment", self.start_line, self.start_column)

    def _string(self):
        chars = []
        while not self._is_at_end():
            c = self._advance()
            if c == '"':
                self._add("STRING", "".join(chars))
                return
            if c == "\\":
                if self._is_at_end():
                    break
                esc = self._advance()
                escapes = {"n": "\n", "t": "\t", '"': '"', "\\": "\\"}
                if esc not in escapes:
                    raise ImmSyntaxError(f"unknown escape \\{esc}", self.line, self.column)
                chars.append(escapes[esc])
            else:
                if c == "\n":
                    raise ImmSyntaxError("unterminated string", self.start_line, self.start_column)
                chars.append(c)
        raise ImmSyntaxError("unterminated string", self.start_line, self.start_column)

    def _number(self):
        while self._peek().isdigit():
            self._advance()
        is_float = False
        if self._peek() == "." and self._peek_next().isdigit():
            is_float = True
            self._advance()
            while self._peek().isdigit():
                self._advance()
        text = self.source[self.start : self.current]
        self._add("NUMBER", float(text) if is_float else int(text))

    def _identifier(self):
        while self._is_ident_part(self._peek()):
            self._advance()
        text = self.source[self.start : self.current]
        kind = "KEYWORD" if text in KEYWORDS else "IDENT"
        self.tokens.append(Token(kind, text, text, self.start_line, self.start_column))

    def _add(self, kind, literal=None):
        text = self.source[self.start : self.current]
        if literal is None:
            literal = text
        self.tokens.append(Token(kind, text, literal, self.start_line, self.start_column))

    def _advance(self):
        c = self.source[self.current]
        self.current += 1
        if c == "\n":
            self.line += 1
            self.column = 1
        else:
            self.column += 1
        return c

    def _match(self, expected):
        if self._is_at_end() or self.source[self.current] != expected:
            return False
        self._advance()
        return True

    def _peek(self):
        if self._is_at_end():
            return "\0"
        return self.source[self.current]

    def _peek_next(self):
        if self.current + 1 >= len(self.source):
            return "\0"
        return self.source[self.current + 1]

    def _is_at_end(self):
        return self.current >= len(self.source)

    @staticmethod
    def _is_ident_start(c):
        return c == "_" or c.isalpha()

    @staticmethod
    def _is_ident_part(c):
        return c == "_" or c.isalnum()


def tokenize(source):
    return Lexer(source).tokenize()
