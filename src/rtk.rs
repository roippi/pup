//! Token-efficient JSON compression for LLM agent mode.
//!
//! Schema extraction ported from rtk-ai/rtk (MIT License).
//! Source: https://github.com/rtk-ai/rtk/blob/main/src/json_cmd.rs
//! Copyright: Patrick Szymkowiak
//!
//! `compress_value` is a new addition: keeps real values but strips the fat
//! (nulls, long strings truncated, large arrays sampled) so the LLM sees
//! actionable data rather than type descriptors.

use serde_json::Value;

// Compress: keep real values, just strip the fat.
const STRING_TRUNC: usize = 200;
const ARRAY_ITEMS_TOP: usize = 20; // top-level list response
const ARRAY_ITEMS_NESTED: usize = 10; // nested arrays (e.g. tags)

/// Compress a JSON string: strip nulls, truncate long strings, sample large arrays.
/// Returns compact (non-pretty) JSON so the caller controls formatting.
pub fn compress_json_string(json_str: &str) -> anyhow::Result<String> {
    let value: Value = serde_json::from_str(json_str)?;
    let compressed = compress_value(&value, 0);
    Ok(serde_json::to_string(&compressed)?)
}

fn compress_value(value: &Value, depth: u8) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::Bool(b) => Value::Bool(*b),
        Value::Number(n) => Value::Number(n.clone()),
        Value::String(s) => {
            if s.len() > STRING_TRUNC {
                Value::String(format!("{}...[{} chars]", &s[..STRING_TRUNC], s.len()))
            } else {
                Value::String(s.clone())
            }
        }
        Value::Array(arr) => {
            let limit = if depth == 0 {
                ARRAY_ITEMS_TOP
            } else {
                ARRAY_ITEMS_NESTED
            };
            let mut items: Vec<Value> = arr
                .iter()
                .take(limit)
                .map(|v| compress_value(v, depth + 1))
                .filter(|v| !v.is_null())
                .collect();
            if arr.len() > limit {
                items.push(Value::String(format!("... +{} more", arr.len() - limit)));
            }
            Value::Array(items)
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if v.is_null() {
                    continue;
                }
                let c = compress_value(v, depth + 1);
                if !c.is_null() {
                    out.insert(k.clone(), c);
                }
            }
            Value::Object(out)
        }
    }
}

// Schema extraction — ported verbatim from rtk-ai/rtk json_cmd.rs.
// Kept for potential future use (e.g. `pup --schema` flag).
#[allow(dead_code)]
pub fn filter_json_string(json_str: &str) -> anyhow::Result<String> {
    let value: Value = serde_json::from_str(json_str)?;
    Ok(extract_schema(&value, 0, 5))
}

#[allow(dead_code)]
fn extract_schema(value: &Value, depth: usize, max_depth: usize) -> String {
    let indent = "  ".repeat(depth);

    if depth > max_depth {
        return format!("{}...", indent);
    }

    match value {
        Value::Null => format!("{}null", indent),
        Value::Bool(_) => format!("{}bool", indent),
        Value::Number(n) => {
            if n.is_i64() {
                format!("{}int", indent)
            } else {
                format!("{}float", indent)
            }
        }
        Value::String(s) => {
            if s.len() > 50 {
                format!("{}string[{}]", indent, s.len())
            } else if s.is_empty() {
                format!("{}string", indent)
            } else if s.starts_with("http") {
                format!("{}url", indent)
            } else if s.contains('-') && s.len() == 10 {
                format!("{}date?", indent)
            } else {
                format!("{}string", indent)
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                format!("{}[]", indent)
            } else {
                let first_schema = extract_schema(&arr[0], depth + 1, max_depth);
                let trimmed = first_schema.trim();
                if arr.len() == 1 {
                    format!("{}[\n{}\n{}]", indent, first_schema, indent)
                } else {
                    format!("{}[{}] ({})", indent, trimmed, arr.len())
                }
            }
        }
        Value::Object(map) => {
            if map.is_empty() {
                return format!("{}{{}}", indent);
            }
            let mut lines = vec![format!("{}{{", indent)];
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();

            for (i, key) in keys.iter().enumerate() {
                let val = &map[*key];
                let val_schema = extract_schema(val, depth + 1, max_depth);
                let val_trimmed = val_schema.trim();

                let is_simple = matches!(
                    val,
                    Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
                );

                if is_simple {
                    if i < keys.len() - 1 {
                        lines.push(format!("{}  {}: {},", indent, key, val_trimmed));
                    } else {
                        lines.push(format!("{}  {}: {}", indent, key, val_trimmed));
                    }
                } else {
                    lines.push(format!("{}  {}:", indent, key));
                    lines.push(val_schema);
                }

                if i >= 15 {
                    lines.push(format!(
                        "{}  ... +{} more keys",
                        indent,
                        keys.len() - i - 1
                    ));
                    break;
                }
            }
            lines.push(format!("{}}}", indent));
            lines.join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- compress_value tests ---

    #[test]
    fn test_compress_drops_nulls() {
        let obj = serde_json::json!({"id": 1, "deleted": null, "name": "foo"});
        let c = compress_value(&obj, 0);
        let m = c.as_object().unwrap();
        assert!(m.contains_key("id"));
        assert!(m.contains_key("name"));
        assert!(!m.contains_key("deleted"));
    }

    #[test]
    fn test_compress_truncates_long_string() {
        let long = "x".repeat(300);
        let c = compress_value(&Value::String(long), 0);
        let s = c.as_str().unwrap();
        assert!(s.contains("...[300 chars]"));
        assert!(s.len() < 300);
    }

    #[test]
    fn test_compress_keeps_short_string() {
        let c = compress_value(&serde_json::json!("hello"), 0);
        assert_eq!(c.as_str().unwrap(), "hello");
    }

    #[test]
    fn test_compress_array_top_level_sampled() {
        let arr: Vec<Value> = (0..30).map(|i| serde_json::json!(i)).collect();
        let c = compress_value(&Value::Array(arr), 0);
        let items = c.as_array().unwrap();
        // 20 real items + 1 "+10 more" sentinel
        assert_eq!(items.len(), 21);
        assert_eq!(items.last().unwrap().as_str().unwrap(), "... +10 more");
    }

    #[test]
    fn test_compress_array_nested_sampled() {
        let arr: Vec<Value> = (0..15).map(|i| serde_json::json!(i)).collect();
        let c = compress_value(&Value::Array(arr), 1); // depth=1 → nested limit
        let items = c.as_array().unwrap();
        assert_eq!(items.len(), 11); // 10 + sentinel
    }

    #[test]
    fn test_compress_array_within_limit() {
        let arr = serde_json::json!(["env:prod", "team:api"]);
        let c = compress_value(&arr, 1);
        assert_eq!(c.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_compress_preserves_real_values() {
        let obj = serde_json::json!({
            "id": 123,
            "name": "High CPU",
            "status": "Alert",
            "tags": ["env:prod", "team:api"],
        });
        let c = compress_value(&obj, 0);
        let m = c.as_object().unwrap();
        assert_eq!(m["id"], serde_json::json!(123));
        assert_eq!(m["name"].as_str().unwrap(), "High CPU");
        assert_eq!(m["status"].as_str().unwrap(), "Alert");
        assert_eq!(m["tags"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_compress_smaller_than_original() {
        // Lots of null fields + a long message → should compress well
        let json = r#"[{"id":1,"name":"High CPU","message":"CPU above 90% on host for 5 minutes. Check runaway processes or traffic spikes immediately. Alert will resolve when CPU drops below threshold for 3 consecutive minutes.","tags":["env:prod","team:api"],"status":"Alert","deleted":null,"draft":null,"restricted_roles":null,"creator":null,"priority":null,"org_id":null}]"#;
        let compressed = compress_json_string(json).unwrap();
        assert!(
            compressed.len() < json.len(),
            "compressed ({}) should be smaller than original ({})",
            compressed.len(),
            json.len()
        );
    }

    // --- extract_schema tests (RTK port) ---

    #[test]
    fn test_extract_schema_primitives() {
        assert_eq!(extract_schema(&serde_json::json!(null), 0, 5), "null");
        assert_eq!(extract_schema(&serde_json::json!(true), 0, 5), "bool");
        assert_eq!(extract_schema(&serde_json::json!(42), 0, 5), "int");
        assert_eq!(extract_schema(&serde_json::json!(3.7), 0, 5), "float");
    }

    #[test]
    fn test_extract_schema_string_short() {
        assert_eq!(extract_schema(&serde_json::json!("hello"), 0, 5), "string");
    }

    #[test]
    fn test_extract_schema_string_long() {
        let long = "a".repeat(60);
        assert_eq!(
            extract_schema(&serde_json::json!(long), 0, 5),
            "string[60]"
        );
    }

    #[test]
    fn test_extract_schema_string_url() {
        assert_eq!(
            extract_schema(&serde_json::json!("https://app.datadoghq.com"), 0, 5),
            "url"
        );
    }

    #[test]
    fn test_extract_schema_string_date() {
        assert_eq!(
            extract_schema(&serde_json::json!("2024-03-11"), 0, 5),
            "date?"
        );
    }

    #[test]
    fn test_extract_schema_empty_array() {
        assert_eq!(extract_schema(&serde_json::json!([]), 0, 5), "[]");
    }

    #[test]
    fn test_extract_schema_array_single() {
        let arr = serde_json::json!([42]);
        let result = extract_schema(&arr, 0, 5);
        assert!(result.contains("int"));
        assert!(result.starts_with('['));
    }

    #[test]
    fn test_extract_schema_array_multi() {
        let arr = serde_json::json!(["env:prod", "team:api", "service:web"]);
        assert_eq!(extract_schema(&arr, 0, 5), "[string] (3)");
    }

    #[test]
    fn test_extract_schema_object() {
        let obj = serde_json::json!({"id": 42, "name": "monitor"});
        let result = extract_schema(&obj, 0, 5);
        assert!(result.contains("id: int"));
        assert!(result.contains("name: string"));
    }

    #[test]
    fn test_extract_schema_object_nested() {
        let obj = serde_json::json!({
            "id": 1,
            "creator": {"email": "a@b.com", "handle": "alice"}
        });
        let result = extract_schema(&obj, 0, 5);
        assert!(result.contains("creator:"));
        assert!(result.contains("email: string"));
    }

    #[test]
    fn test_extract_schema_depth_limit() {
        let deep = serde_json::json!({"a": 1});
        let result = extract_schema(&deep, 0, 0);
        assert!(result.contains("a: ..."), "got: {result}");
    }

    #[test]
    fn test_filter_json_string_roundtrip() {
        let json = r#"{"name": "test", "count": 42, "tags": ["a", "b"]}"#;
        let result = filter_json_string(json).unwrap();
        assert!(result.contains("name: string"));
        assert!(result.contains("count: int"));
        assert!(result.contains("[string] (2)"));
    }

    #[test]
    fn test_filter_json_string_invalid() {
        assert!(filter_json_string("not json").is_err());
    }

    #[test]
    fn test_schema_smaller_than_original() {
        let json = r#"[{"id":123456,"name":"High CPU on prod","message":"CPU above 90% on host for 5 minutes. Please check for runaway processes or traffic spikes immediately.","tags":["env:production","team:platform","severity:high","service:web"],"status":"Alert","created":"2024-01-15","type":"metric alert"}]"#;
        let schema = filter_json_string(json).unwrap();
        assert!(
            schema.len() < json.len(),
            "schema ({}) should be smaller than json ({})",
            schema.len(),
            json.len()
        );
    }
}
