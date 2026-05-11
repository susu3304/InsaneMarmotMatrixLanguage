class ImmError(Exception):
    pass


class ImmSyntaxError(ImmError):
    def __init__(self, message, line=None, column=None):
        self.line = line
        self.column = column
        where = ""
        if line is not None and column is not None:
            where = f" at {line}:{column}"
        super().__init__(f"syntax error{where}: {message}")


class ImmRuntimeError(ImmError):
    pass
