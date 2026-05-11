def format_source(source):
    source = source.replace("\r\n", "\n").replace("\r", "\n")
    lines = source.split("\n")
    formatted = []
    depth = 0
    in_block_comment = False

    for raw in lines:
        stripped_right = raw.rstrip()
        stripped = stripped_right.strip()
        if stripped == "":
            formatted.append("")
            continue

        code_part = stripped
        if not in_block_comment and code_part.startswith("}"):
            depth = max(0, depth - 1)

        formatted.append((" " * 4 * depth) + stripped)

        delta, in_block_comment = brace_delta(stripped, in_block_comment)
        depth = max(0, depth + delta)

    while formatted and formatted[-1] == "":
        formatted.pop()
    return "\n".join(formatted) + "\n"


def brace_delta(line, in_block_comment=False):
    delta = 0
    in_string = False
    escaped = False
    i = 0
    while i < len(line):
        c = line[i]
        nxt = line[i + 1] if i + 1 < len(line) else ""
        if in_block_comment:
            if c == "*" and nxt == "/":
                in_block_comment = False
                i += 2
                continue
            i += 1
            continue
        if in_string:
            if escaped:
                escaped = False
            elif c == "\\":
                escaped = True
            elif c == '"':
                in_string = False
            i += 1
            continue
        if c == "/" and nxt == "*":
            in_block_comment = True
            i += 2
            continue
        if c == "#":
            break
        if c == '"':
            in_string = True
        elif c == "{":
            delta += 1
        elif c == "}":
            delta -= 1
        i += 1
    return delta, in_block_comment
