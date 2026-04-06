# json Module

JSON parsing and serialization for maps.

## Functions
- `parse(s str) -> {str: any} | error`: Parses a JSON object into a map, preserving nested lists, maps, numbers, booleans, and `null`.
- `stringify(data {str: T}) -> str | error`: Serializes a Nimble value tree to JSON.
- `pretty(data {str: T}) -> str | error`: Pretty-prints JSON.

## Example
```nimble
load json
cfg = json.parse('{"env":"dev","port":8080}')?
out(cfg["env"])
```
