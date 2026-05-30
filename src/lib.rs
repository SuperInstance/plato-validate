use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Expected types for Type validation rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedType {
    Integer,
    Float,
    Boolean,
    String,
}

/// The kind of validation rule to apply.
#[derive(Debug, Clone)]
pub enum RuleType {
    Range(f64, f64),
    NotNull,
    Type(ExpectedType),
    Regex(String),
    Custom(fn(&str) -> bool),
    Enum(Vec<String>),
}

/// A single validation rule targeting a named field.
#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub field: String,
    pub rule_type: RuleType,
}

impl ValidationRule {
    pub fn new(field: impl Into<String>, rule_type: RuleType) -> Self {
        Self {
            field: field.into(),
            rule_type,
        }
    }
}

/// One validation failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub value: String,
}

/// The outcome of running validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    pub fn with_errors(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: errors.is_empty(),
            errors,
        }
    }
}

/// Accumulates rules and validates `HashMap<String, String>` data against them.
#[derive(Debug, Clone, Default)]
pub struct Validator {
    rules: Vec<ValidationRule>,
}

impl Validator {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: ValidationRule) -> &mut Self {
        self.rules.push(rule);
        self
    }

    pub fn validate(&self, data: &HashMap<String, String>) -> ValidationResult {
        let mut errors: Vec<ValidationError> = Vec::new();

        for rule in &self.rules {
            let raw = data.get(&rule.field).map(|s| s.as_str()).unwrap_or("");

            match &rule.rule_type {
                RuleType::NotNull => {
                    if raw.is_empty() {
                        errors.push(ValidationError {
                            field: rule.field.clone(),
                            message: "field must not be null or empty".into(),
                            value: String::new(),
                        });
                    }
                }
                RuleType::Range(min, max) => {
                    if raw.is_empty() {
                        continue; // NotNull handles missing
                    }
                    if let Ok(v) = raw.parse::<f64>() {
                        if v < *min || v > *max {
                            errors.push(ValidationError {
                                field: rule.field.clone(),
                                message: format!("value must be in range [{}, {}]", min, max),
                                value: raw.to_string(),
                            });
                        }
                    } else {
                        errors.push(ValidationError {
                            field: rule.field.clone(),
                            message: "value is not a valid number".into(),
                            value: raw.to_string(),
                        });
                    }
                }
                RuleType::Type(expected) => {
                    if raw.is_empty() {
                        continue;
                    }
                    let ok = match expected {
                        ExpectedType::Integer => raw.parse::<i64>().is_ok(),
                        ExpectedType::Float => raw.parse::<f64>().is_ok(),
                        ExpectedType::Boolean => raw == "true" || raw == "false",
                        ExpectedType::String => true,
                    };
                    if !ok {
                        errors.push(ValidationError {
                            field: rule.field.clone(),
                            message: format!("expected type {:?}", expected).to_lowercase(),
                            value: raw.to_string(),
                        });
                    }
                }
                RuleType::Regex(pattern) => {
                    if raw.is_empty() {
                        continue;
                    }
                    // Lightweight check: try regex crate if available, otherwise basic contains
                    if let Ok(re) = regex::Regex::new(pattern) {
                        if !re.is_match(raw) {
                            errors.push(ValidationError {
                                field: rule.field.clone(),
                                message: format!("does not match pattern {}", pattern),
                                value: raw.to_string(),
                            });
                        }
                    } else {
                        errors.push(ValidationError {
                            field: rule.field.clone(),
                            message: format!("invalid regex pattern: {}", pattern),
                            value: raw.to_string(),
                        });
                    }
                }
                RuleType::Custom(check) => {
                    if raw.is_empty() {
                        continue;
                    }
                    if !check(raw) {
                        errors.push(ValidationError {
                            field: rule.field.clone(),
                            message: "custom validation failed".into(),
                            value: raw.to_string(),
                        });
                    }
                }
                RuleType::Enum(allowed) => {
                    if raw.is_empty() {
                        continue;
                    }
                    if !allowed.iter().any(|a| a == raw) {
                        errors.push(ValidationError {
                            field: rule.field.clone(),
                            message: format!("must be one of {:?}", allowed),
                            value: raw.to_string(),
                        });
                    }
                }
            }
        }

        ValidationResult::with_errors(errors)
    }
}

// ---------------------------------------------------------------------------
// Standalone validation helpers
// ---------------------------------------------------------------------------

/// Known PLATO tile types.
pub const TILE_TYPES: &[&str] = &[
    "thermal",
    "pressure",
    "vibration",
    "rpm",
    "humidity",
    "acoustic",
    "magnetic",
    "flow",
    "voltage",
    "current",
];

pub fn validate_tile_type(s: &str) -> bool {
    TILE_TYPES.contains(&s)
}

/// Physical-plausibility bounds for sensor readings.
pub fn sensor_range(sensor_type: &str) -> Option<(f64, f64)> {
    match sensor_type {
        "temperature" => Some((-273.15, 1000.0)),
        "pressure" => Some((0.0, 10000.0)),
        "rpm" => Some((0.0, 100000.0)),
        "vibration" => Some((0.0, 50.0)),
        "humidity" => Some((0.0, 100.0)),
        _ => None,
    }
}

pub fn validate_sensor_value(value: f64, sensor_type: &str) -> bool {
    sensor_range(sensor_type)
        .map(|(lo, hi)| value >= lo && value <= hi)
        .unwrap_or(false)
}

pub fn validate_confidence(value: f64) -> bool {
    (0.0..=1.0).contains(&value)
}

/// Timestamp must not be in the future and not older than 30 days.
pub fn validate_timestamp(ts: u64) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let thirty_days: u64 = 30 * 24 * 3600;
    ts <= now && ts >= now.saturating_sub(thirty_days)
}

/// Trim whitespace and strip ASCII control characters (except newline/tab).
pub fn sanitize_string(s: &str) -> String {
    s.trim()
        .chars()
        .filter(|c| !c.is_ascii_control() || *c == '\n' || *c == '\t')
        .collect()
}

/// Clamp a float to [min, max].
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Range ---
    #[test]
    fn range_in_bounds() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("temp", RuleType::Range(-273.15, 1000.0)));
        let mut data = HashMap::new();
        data.insert("temp".into(), "25.0".into());
        assert!(v.validate(&data).valid);
    }

    #[test]
    fn range_out_of_bounds() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("temp", RuleType::Range(-273.15, 1000.0)));
        let mut data = HashMap::new();
        data.insert("temp".into(), "2000.0".into());
        let res = v.validate(&data);
        assert!(!res.valid);
        assert!(res.errors[0].message.contains("range"));
    }

    // --- NotNull ---
    #[test]
    fn not_null_passes() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("name", RuleType::NotNull));
        let mut data = HashMap::new();
        data.insert("name".into(), "sensor-1".into());
        assert!(v.validate(&data).valid);
    }

    #[test]
    fn not_null_missing_field() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("name", RuleType::NotNull));
        let data = HashMap::new();
        assert!(!v.validate(&data).valid);
    }

    // --- Type ---
    #[test]
    fn type_check_integer_pass() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("count", RuleType::Type(ExpectedType::Integer)));
        let mut data = HashMap::new();
        data.insert("count".into(), "42".into());
        assert!(v.validate(&data).valid);
    }

    #[test]
    fn type_check_integer_fail() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("count", RuleType::Type(ExpectedType::Integer)));
        let mut data = HashMap::new();
        data.insert("count".into(), "hello".into());
        assert!(!v.validate(&data).valid);
    }

    // --- Regex ---
    #[test]
    fn regex_pass() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("id", RuleType::Regex(r"^[A-Z]{3}-\d{4}$".into())));
        let mut data = HashMap::new();
        data.insert("id".into(), "ABC-1234".into());
        assert!(v.validate(&data).valid);
    }

    #[test]
    fn regex_fail() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("id", RuleType::Regex(r"^[A-Z]{3}-\d{4}$".into())));
        let mut data = HashMap::new();
        data.insert("id".into(), "bad-id".into());
        assert!(!v.validate(&data).valid);
    }

    // --- Custom ---
    #[test]
    fn custom_pass() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("hex", RuleType::Custom(|s| s.chars().all(|c| c.is_ascii_hexdigit()))));
        let mut data = HashMap::new();
        data.insert("hex".into(), "deadbeef".into());
        assert!(v.validate(&data).valid);
    }

    #[test]
    fn custom_fail() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("hex", RuleType::Custom(|s| s.chars().all(|c| c.is_ascii_hexdigit()))));
        let mut data = HashMap::new();
        data.insert("hex".into(), "ZZZZ".into());
        assert!(!v.validate(&data).valid);
    }

    // --- Tile type ---
    #[test]
    fn tile_type_valid() {
        assert!(validate_tile_type("thermal"));
        assert!(validate_tile_type("rpm"));
    }

    #[test]
    fn tile_type_invalid() {
        assert!(!validate_tile_type("unknown"));
        assert!(!validate_tile_type(""));
    }

    // --- Sensor value ---
    #[test]
    fn sensor_plausible() {
        assert!(validate_sensor_value(25.0, "temperature"));
        assert!(validate_sensor_value(0.0, "pressure"));
    }

    #[test]
    fn sensor_implausible() {
        assert!(!validate_sensor_value(-300.0, "temperature"));
        assert!(!validate_sensor_value(99999.0, "humidity"));
    }

    // --- Confidence ---
    #[test]
    fn confidence_bounds() {
        assert!(validate_confidence(0.0));
        assert!(validate_confidence(1.0));
        assert!(validate_confidence(0.5));
        assert!(!validate_confidence(-0.1));
        assert!(!validate_confidence(1.1));
    }

    // --- Timestamp ---
    #[test]
    fn timestamp_valid() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(validate_timestamp(now - 60)); // 1 min ago
    }

    #[test]
    fn timestamp_future_invalid() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(!validate_timestamp(now + 3600));
    }

    #[test]
    fn timestamp_too_old_invalid() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(!validate_timestamp(now - 31 * 24 * 3600));
    }

    // --- Sanitize ---
    #[test]
    fn sanitize_trims_and_strips_control() {
        assert_eq!(sanitize_string("  hello\x00world  "), "helloworld");
    }

    // --- Clamp ---
    #[test]
    fn clamp_within() {
        assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
    }

    #[test]
    fn clamp_below() {
        assert_eq!(clamp(-1.0, 0.0, 10.0), 0.0);
    }

    #[test]
    fn clamp_above() {
        assert_eq!(clamp(11.0, 0.0, 10.0), 10.0);
    }

    // --- Multiple rules ---
    #[test]
    fn multiple_rules_same_field() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("val", RuleType::NotNull));
        v.add_rule(ValidationRule::new("val", RuleType::Range(0.0, 100.0)));
        let mut data = HashMap::new();
        data.insert("val".into(), "50".into());
        assert!(v.validate(&data).valid);
    }

    #[test]
    fn multiple_rules_one_fails() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("val", RuleType::NotNull));
        v.add_rule(ValidationRule::new("val", RuleType::Range(0.0, 100.0)));
        let mut data = HashMap::new();
        data.insert("val".into(), "200".into());
        assert!(!v.validate(&data).valid);
        assert_eq!(v.validate(&data).errors.len(), 1);
    }

    // --- Empty / missing ---
    #[test]
    fn empty_field_with_range_skipped() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new("val", RuleType::Range(0.0, 100.0)));
        let mut data = HashMap::new();
        data.insert("val".into(), "".into());
        let res = v.validate(&data);
        // Range skips empty values; NotNull would catch it
        assert!(res.valid);
    }

    // --- Enum ---
    #[test]
    fn enum_pass() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new(
            "status",
            RuleType::Enum(vec!["active".into(), "inactive".into(), "error".into()]),
        ));
        let mut data = HashMap::new();
        data.insert("status".into(), "active".into());
        assert!(v.validate(&data).valid);
    }

    #[test]
    fn enum_fail() {
        let mut v = Validator::new();
        v.add_rule(ValidationRule::new(
            "status",
            RuleType::Enum(vec!["active".into(), "inactive".into()]),
        ));
        let mut data = HashMap::new();
        data.insert("status".into(), "unknown".into());
        assert!(!v.validate(&data).valid);
    }

    // --- Serialization ---
    #[test]
    fn serialize_validation_result() {
        let res = ValidationResult::with_errors(vec![ValidationError {
            field: "x".into(),
            message: "bad".into(),
            value: "42".into(),
        }]);
        let json = serde_json::to_string(&res).unwrap();
        assert!(json.contains("\"valid\":false"));
        assert!(json.contains("\"field\":\"x\""));
    }

    #[test]
    fn deserialize_validation_error() {
        let json = r#"{"field":"y","message":"oops","value":"null"}"#;
        let err: ValidationError = serde_json::from_str(json).unwrap();
        assert_eq!(err.field, "y");
    }
}
