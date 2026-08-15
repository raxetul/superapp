//! Minimal JSON-Schema validation for module configuration — the same
//! practical subset the core's `modules::config_schema` enforces at `PUT
//! /modules/{id}/config` (TR-05-006), so a module author can validate a
//! config document identically before shipping it.

use serde_json::Value;

use crate::manifest::FieldError;

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
        "number" => matches!(instance, Value::Number(_)),
        "integer" => instance.as_i64().is_some() || instance.as_u64().is_some(),
        other => type_name(instance) == other,
    }
}

fn validate_at(schema: &Value, instance: &Value, ptr: &str, errs: &mut Vec<FieldError>) {
    let Some(schema) = schema.as_object() else {
        return;
    };

    if let Some(Value::String(expected)) = schema.get("type") {
        if !type_matches(expected, instance) {
            errs.push(FieldError::new(
                pointer_or_root(ptr),
                format!("expected type `{expected}`, got `{}`", type_name(instance)),
            ));
            return;
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
            "required": ["greeting"],
            "properties": { "greeting": { "type": "string", "minLength": 1 } }
        })
    }

    #[test]
    fn valid_config_passes() {
        assert!(validate(&schema(), &json!({"greeting": "hi"})).is_empty());
    }

    #[test]
    fn missing_required_is_reported() {
        let errs = validate(&schema(), &json!({}));
        assert!(errs.iter().any(|e| e.pointer == "/greeting"));
    }
}
