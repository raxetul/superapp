//! Minimal JSON-Schema validation for module configuration (TR-05-006).
//!
//! Rather than pull in a full JSON-Schema crate (MSRV-sensitive on rustc
//! 1.85), this validates the practical subset module authors use: `type`,
//! `properties`, `required`, `enum`, numeric `minimum`/`maximum`, and string
//! `minLength`/`maxLength`. Unknown keywords are ignored (permissive), and
//! failures are returned as RFC 9457 field errors (JSON Pointer + message).

use serde_json::Value;

use crate::response::FieldError;

/// Validate `instance` against `schema`. An empty result means valid.
#[must_use]
pub fn validate(schema: &Value, instance: &Value) -> Vec<FieldError> {
    let mut errs = Vec::new();
    validate_at(schema, instance, "", &mut errs);
    errs
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer"
            } else {
                "number"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn type_matches(expected: &str, instance: &Value) -> bool {
    match expected {
        // JSON has no separate integer type; accept integers as numbers too.
        "number" => matches!(instance, Value::Number(_)),
        "integer" => instance.as_i64().is_some() || instance.as_u64().is_some(),
        other => type_name(instance) == other,
    }
}

fn validate_at(schema: &Value, instance: &Value, ptr: &str, errs: &mut Vec<FieldError>) {
    let Some(schema) = schema.as_object() else {
        return; // non-object schema ⇒ nothing to enforce
    };

    if let Some(Value::String(expected)) = schema.get("type") {
        if !type_matches(expected, instance) {
            errs.push(FieldError::new(
                pointer_or_root(ptr),
                format!("expected type `{expected}`, got `{}`", type_name(instance)),
            ));
            return; // a type mismatch makes deeper checks meaningless
        }
    }

    if let Some(Value::Array(allowed)) = schema.get("enum") {
        if !allowed.contains(instance) {
            errs.push(FieldError::new(
                pointer_or_root(ptr),
                "value is not one of the allowed values",
            ));
        }
    }

    match instance {
        Value::Object(obj) => {
            if let Some(Value::Array(required)) = schema.get("required") {
                for req in required.iter().filter_map(Value::as_str) {
                    if !obj.contains_key(req) {
                        errs.push(FieldError::new(
                            format!("{ptr}/{req}"),
                            format!("missing required property `{req}`"),
                        ));
                    }
                }
            }
            if let Some(Value::Object(props)) = schema.get("properties") {
                for (key, subschema) in props {
                    if let Some(child) = obj.get(key) {
                        validate_at(subschema, child, &format!("{ptr}/{key}"), errs);
                    }
                }
            }
        }
        Value::Number(n) => {
            if let (Some(min), Some(v)) =
                (schema.get("minimum").and_then(Value::as_f64), n.as_f64())
            {
                if v < min {
                    errs.push(FieldError::new(
                        pointer_or_root(ptr),
                        format!("must be >= {min}"),
                    ));
                }
            }
            if let (Some(max), Some(v)) =
                (schema.get("maximum").and_then(Value::as_f64), n.as_f64())
            {
                if v > max {
                    errs.push(FieldError::new(
                        pointer_or_root(ptr),
                        format!("must be <= {max}"),
                    ));
                }
            }
        }
        Value::String(s) => {
            if let Some(min) = schema.get("minLength").and_then(Value::as_u64) {
                if (s.chars().count() as u64) < min {
                    errs.push(FieldError::new(
                        pointer_or_root(ptr),
                        format!("must be at least {min} characters"),
                    ));
                }
            }
            if let Some(max) = schema.get("maxLength").and_then(Value::as_u64) {
                if (s.chars().count() as u64) > max {
                    errs.push(FieldError::new(
                        pointer_or_root(ptr),
                        format!("must be at most {max} characters"),
                    ));
                }
            }
        }
        _ => {}
    }
}

fn pointer_or_root(ptr: &str) -> String {
    if ptr.is_empty() {
        "/".to_string()
    } else {
        ptr.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> Value {
        json!({
            "type": "object",
            "required": ["currency"],
            "properties": {
                "currency": { "type": "string", "enum": ["USD", "EUR", "TRY"] },
                "retries": { "type": "integer", "minimum": 0, "maximum": 10 },
                "label": { "type": "string", "minLength": 2 }
            }
        })
    }

    #[test]
    fn valid_config_passes() {
        let errs = validate(
            &schema(),
            &json!({"currency":"EUR","retries":3,"label":"eu"}),
        );
        assert!(errs.is_empty(), "expected valid, got {errs:?}");
    }

    #[test]
    fn missing_required_is_reported() {
        let errs = validate(&schema(), &json!({"retries": 1}));
        assert!(errs.iter().any(|e| e.pointer == "/currency"));
    }

    #[test]
    fn wrong_type_is_reported() {
        let errs = validate(&schema(), &json!({"currency": "USD", "retries": "lots"}));
        assert!(errs.iter().any(|e| e.pointer == "/retries"));
    }

    #[test]
    fn enum_and_bounds_are_enforced() {
        let errs = validate(&schema(), &json!({"currency":"GBP","retries":99}));
        let ptrs: Vec<&str> = errs.iter().map(|e| e.pointer.as_str()).collect();
        assert!(ptrs.contains(&"/currency")); // not in enum
        assert!(ptrs.contains(&"/retries")); // > maximum
    }

    #[test]
    fn string_min_length_enforced() {
        let errs = validate(&schema(), &json!({"currency":"USD","label":"x"}));
        assert!(errs.iter().any(|e| e.pointer == "/label"));
    }
}
