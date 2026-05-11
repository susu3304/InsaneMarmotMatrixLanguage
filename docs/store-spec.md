# IMM Store Persistence

IMM `store` is the built-in persistence layer for `den` objects. It is intentionally SQLite-like in daily use, but it does not require the SQLite CLI, an external database server, or third-party packages. The reference interpreter stores data in a deterministic JSON file.

## Goals

- Persist `den` instances without external tools.
- Keep the API small enough for game AI and simulations.
- Preserve private fields, because persistence belongs to the runtime, not public object access.
- Load objects back as real IMM objects with methods and type checks.
- Use a portable file format that can be inspected and backed up.

## File Model

A store file has extension `.immstore` by convention.

```text
game.immstore
players.immstore
```

The file is a UTF-8 JSON document with:

- a format marker,
- per-type next ids,
- records grouped by `den` name,
- serialized field values.

The format is considered a runtime detail, but it is stable enough for tests and simple inspection.

## API

`store` is a built-in namespace.

```imm
use store
```

Available functions:

| Function | Meaning |
| --- | --- |
| `store.open(path)` | Open or create a store file. |
| `store.save(db, object)` | Insert or update a `den` object. Returns its integer id. |
| `store.load(db, Type, id)` | Load one object by id. Returns object or `null`. |
| `store.all(db, Type)` | Load all objects of a den type. |
| `store.find(db, Type, field, value)` | Load objects whose stored field equals value. |
| `store.get(db, Type, field, value)` | Return first matching object or `null`. |
| `store.delete(db, Type, id)` | Delete by id. Returns `true` if deleted. |
| `store.count(db, Type)` | Count stored objects for a den type. |
| `store.clear(db, Type)` | Delete all objects of a den type. Returns deleted count. |

## Example

```imm
use store

den Player {
    fur let name: String
    fang let hp: Int = 100

    fur dig init(name: String) {
        self.name = name
    }

    fur dig status() {
        squeak self.name + ": " + str(self.hp)
    }
}

marmot main {
    let db = store.open("players.immstore")
    let p = hatch Player("susu")
    let id = store.save(db, p)

    let loaded: Player = store.load(db, Player, id)
    loaded.status()
}
```

## Serialization

Supported field values:

- `null`
- `Bool`
- `Int`
- `Float`
- `String`
- `Array`
- `Point`
- `Matrix`
- `den` objects as embedded snapshots

Embedded object snapshots are intended for small object graphs. Cyclic object graphs are rejected.

## Identity

`store.save` returns an integer id. If the same object is saved again to the same store file, the existing record is updated. An object loaded from a store keeps its store id, so saving it updates the original record.

## Type Safety

`store.load`, `store.all`, `store.find`, and `store.get` require a `den` type value, for example `Player`.

Loaded objects are checked against the current `den` definition. Missing fields or incompatible field values are runtime errors.

## Non-Goals For The First Version

- SQL query language.
- Joins.
- Indexes.
- Transactions across multiple files.
- Concurrent writer guarantees.

Those can be added later without changing the basic object persistence API.
