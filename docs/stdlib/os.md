# os Module

Process-level helpers.

## Functions
- `args() -> [str]`: Returns script arguments passed after `--` in `nimble run script.nmb -- arg1 arg2`.
- `exit(code int)`: Exits the process with `code`.

## Example
```nimble
load os
if len(os.args()) == 0:
    out("missing arg")
    os.exit(1)
```
