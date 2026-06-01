# plato-validate

> Input validation for PLATO tile data — range checks, type checks, regex, custom rules

## What This Does

plato-validate provides a rule-based validation system for tile data. Define rules (range, not-null, type, regex, enum, custom function), apply them to field-value maps, and get structured validation results with per-field error messages.

## The Key Idea

Garbage in, garbage out. Before tile data enters the pipeline, validate it: temperature must be a number between -50 and 150, sensor_id must not be null, status must be one of ["healthy", "degraded", "faulted"]. plato-validate makes these rules declarative and composable.

## Install

```bash
cargo add plato-validate
```

## Quick Start

```rust
use plato_validate::*;

let mut validator = Validator::new();
validator.add_rule(ValidationRule::new("temperature", RuleType::Range(-50.0, 150.0)));
validator.add_rule(ValidationRule::new("sensor_id", RuleType::NotNull));
validator.add_rule(ValidationRule::new("status", RuleType::Enum(vec![
    "healthy".into(), "degraded".into(), "faulted".into()
])));

let mut data = HashMap::new();
data.insert("temperature", "22.5");
data.insert("sensor_id", "temp-001");
data.insert("status", "healthy");

let result = validator.validate(&data);
assert!(result.valid);
```

## API Reference

| Type | Description |
|---|---|
| `RuleType` | `Range(min, max)` / `NotNull` / `Type(ExpectedType)` / `Regex(pattern)` / `Custom(fn)` / `Enum(values)` |
| `ExpectedType` | `Integer` / `Float` / `Boolean` / `String` |
| `ValidationRule { field, rule_type }` | A rule targeting a named field |
| `ValidationError { field, message, value }` | One failure |
| `ValidationResult { valid, errors }` | Outcome of validation |
| `Validator` | Rule accumulator. `add_rule()`, `validate(data)` |

## Testing

29 tests: range checks (in/out/boundary), not-null, type checking (int/float/bool/string), regex patterns, enum validation, custom functions, multiple rules, nested validation, error messages.

## License

Apache-2.0
