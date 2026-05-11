# IMM Object Model

This document records the implemented first slice of the IMM object proposal.

## Implemented

- `den Name { ... }` defines an object type.
- `hatch Name(args...)` creates an instance.
- `self` is available inside methods and `init`.
- `init` is a constructor and cannot declare a return type.
- `fur` members are public.
- `fang` members are private. Omitted access defaults to `fang`.
- `mask` defines method signatures.
- `den Name wear MaskA, MaskB { ... }` validates that required methods exist and signatures match.
- `den Child under Parent { ... }` provides single inheritance.
- Child methods can override parent methods when signatures match.
- `under.init(...)` calls the parent constructor.
- `under.method(...)` calls the parent method implementation.
- `den` and `mask` names can be used in runtime type annotations.
- Values annotated as a `mask` expose only that mask's methods at runtime.
- `imm check` also reports common object errors before execution, including private access from the wrong `den` and mask view violations.

## Example

```imm
mask Movable {
    dig move(dir: String) -> Void
}

den Player wear Movable {
    fur let name: String
    fang let hp: Int

    fur dig init(name: String) {
        self.name = name
        self.hp = 100
    }

    fur dig move(dir: String) {
        squeak self.name + " digs " + dir
    }
}

marmot main {
    let p: Movable = hatch Player("marmot")
    p.move("UP")
}
```

## Still Planned

- Static object type checking in `imm check`.
- Static mask-view checking in `imm check`.
- Full initialization analysis before execution.
- Better diagnostics with source spans.
- Module export/import rules for object types.
- Optional stricter `insane` object behavior controls.
