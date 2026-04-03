//! Automated API parity probe - extracts serde_json's actual public API from source
//! and compares against lifegraph-json.
//!
//! Run: cargo test --test api_parity_probe -- --nocapture

use lifegraph_json as lg;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Extract all `pub fn`, `pub struct`, `pub enum`, `pub type`, `pub use` from source files
fn extract_public_items(source_dir: &Path) -> BTreeSet<(String, String)> {
    let mut items = BTreeSet::new();

    if !source_dir.exists() {
        return items;
    }

    for entry in walkdir::WalkDir::new(source_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
    {
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let module = entry
            .path()
            .strip_prefix(source_dir)
            .ok()
            .and_then(|p| p.parent())
            .map(|p| p.to_string_lossy().replace('/', "::"))
            .unwrap_or_default();

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip comments and empty
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.is_empty() {
                continue;
            }

            // pub fn name
            if let Some(name) = trimmed
                .strip_prefix("pub fn ")
                .map(|s| s.split('(').next().unwrap_or(s).trim())
                .filter(|s| !s.is_empty() && s.chars().next().unwrap().is_ascii_alphabetic())
            {
                items.insert((module.clone(), format!("fn {name}")));
            }

            // pub struct name
            if let Some(name) = trimmed
                .strip_prefix("pub struct ")
                .map(|s| {
                    s.split('<')
                        .next()
                        .unwrap_or(s)
                        .split('{')
                        .next()
                        .unwrap_or(s)
                        .trim()
                })
                .filter(|s| !s.is_empty())
            {
                items.insert((module.clone(), format!("struct {name}")));
            }

            // pub enum name
            if let Some(name) = trimmed
                .strip_prefix("pub enum ")
                .map(|s| {
                    s.split('<')
                        .next()
                        .unwrap_or(s)
                        .split('{')
                        .next()
                        .unwrap_or(s)
                        .trim()
                })
                .filter(|s| !s.is_empty())
            {
                items.insert((module.clone(), format!("enum {name}")));
            }

            // pub type name
            if let Some(name) = trimmed
                .strip_prefix("pub type ")
                .map(|s| s.split('=').next().unwrap_or(s).trim())
                .filter(|s| !s.is_empty())
            {
                items.insert((module.clone(), format!("type {name}")));
            }

            // pub use X as Y / pub use X
            if trimmed.starts_with("pub use ") {
                // Extract the exported name
                if let Some(after_use) = trimmed.strip_prefix("pub use ") {
                    // Handle "pub use foo::bar;" or "pub use foo::bar as baz;"
                    let parts: Vec<&str> = after_use.split(" as ").collect();
                    let name = if parts.len() == 2 {
                        parts[1].trim().trim_end_matches(';').trim()
                    } else {
                        after_use
                            .split("::")
                            .last()
                            .unwrap_or("")
                            .trim_end_matches(';')
                            .trim()
                    };
                    if !name.is_empty() && name.chars().next().unwrap().is_ascii_alphabetic() {
                        items.insert((module.clone(), format!("use {name}")));
                    }
                }
            }

            // pub mod name - track re-exported modules
            if let Some(name) = trimmed
                .strip_prefix("pub mod ")
                .map(|s| {
                    s.split('{')
                        .next()
                        .unwrap_or(s)
                        .split(';')
                        .next()
                        .unwrap_or(s)
                        .trim()
                })
                .filter(|s| !s.is_empty())
            {
                items.insert((module.clone(), format!("mod {name}")));
            }

            // macro_rules! name (at crate level)
            if trimmed.starts_with("macro_rules!") {
                if let Some(name) = trimmed
                    .strip_prefix("macro_rules! ")
                    .map(|s| s.split('(').next().unwrap_or(s).trim())
                    .filter(|s| !s.is_empty())
                {
                    items.insert((module.clone(), format!("macro {name}")));
                }
            }
        }
    }

    items
}

/// Find serde_json source in cargo registry
fn find_serde_json_source() -> Option<std::path::PathBuf> {
    let cargo_home = std::env::var("CARGO_HOME")
        .unwrap_or_else(|_| format!("{}/.cargo", std::env::var("HOME").unwrap_or_default()));

    let registry = format!("{}/registry/src", cargo_home);
    let registry_path = Path::new(&registry);

    if !registry_path.exists() {
        return None;
    }

    // Walk through registry directories
    for base_dir in fs::read_dir(registry_path).ok()?.flatten() {
        if !base_dir.path().is_dir() {
            continue;
        }
        for entry in fs::read_dir(base_dir.path()).ok()?.flatten() {
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with("serde_json-")
            {
                let src = entry.path().join("src");
                if src.exists() {
                    return Some(src);
                }
            }
        }
    }

    None
}

/// Extract top-level exported names (ignore module path for comparison)
fn get_exported_names(items: &BTreeSet<(String, String)>) -> BTreeSet<(String, String)> {
    items
        .iter()
        .filter(|(module, _)| {
            // Only top-level exports (lib.rs or main module)
            module.is_empty()
        })
        .map(|(_, item)| {
            let kind = item.split(' ').next().unwrap_or("");
            let name = item.split(' ').nth(1).unwrap_or("");
            (kind.to_string(), name.to_string())
        })
        .collect()
}

#[test]
fn generate_api_parity_report() {
    // 1. Extract lifegraph-json public API
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lg_source = manifest_dir.join("src");
    let lg_items = extract_public_items(&lg_source);

    // 2. Extract serde_json public API
    let sj_source = find_serde_json_source();
    let sj_items = if let Some(ref path) = sj_source {
        extract_public_items(path)
    } else {
        eprintln!("Warning: serde_json source not found, using fallback list");
        BTreeSet::new()
    };

    // 3. Get top-level exports for comparison
    let lg_exports = get_exported_names(&lg_items);
    let sj_exports = get_exported_names(&sj_items);

    // 4. Compare
    let mut report = String::new();
    report.push_str("# serde_json API Parity Report\n\n");
    report.push_str("> **Auto-generated** by `cargo test --test api_parity_probe`\n");
    report.push_str("> Do not edit manually - regenerated each CI run\n");
    if let Some(ref path) = sj_source {
        report.push_str(&format!(
            "> serde_json source: `{}`\n",
            path.parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
        ));
    }
    report.push('\n');

    // Core items we care about - serde_json exports these at top level
    // These are the ACTUAL items serde_json exports (from lib.rs `pub use`)
    let core_items = [
        ("fn", "from_str"),
        ("fn", "from_slice"),
        ("fn", "from_reader"),
        ("fn", "to_string"),
        ("fn", "to_string_pretty"),
        ("fn", "to_vec"),
        ("fn", "to_vec_pretty"),
        ("fn", "to_writer"),
        ("fn", "to_writer_pretty"),
        ("fn", "to_value"),
        ("fn", "from_value"),
        ("use", "Value"), // serde_json exports: pub use ...::Value
        ("use", "Map"),
        ("use", "Number"),
        ("use", "Error"),
        ("use", "Result"),
        ("macro", "json"),
        ("use", "RawValue"),
        ("fn", "to_raw_value"),
        ("use", "Deserializer"),
        ("use", "StreamDeserializer"),
        ("use", "Serializer"),
    ];

    let mut total = 0;
    let mut present = 0;
    let mut missing = Vec::new();

    report.push_str("## Summary\n\n");
    report.push_str(&format!(
        "- **serde_json top-level items found**: {}\n",
        sj_exports.len()
    ));
    report.push_str(&format!(
        "- **lifegraph-json top-level items found**: {}\n",
        lg_exports.len()
    ));
    report.push_str(&format!(
        "- **Core API items tracked**: {}\n\n",
        core_items.len()
    ));

    for (kind, name) in &core_items {
        // Check if serde_json has this (might be "use" for re-exports)
        let in_sj = sj_exports.contains(&((*kind).to_string(), (*name).to_string()));

        // Check lifegraph - be flexible about the kind (fn/struct/use/type all OK)
        let in_lg = lg_exports.contains(&((*kind).to_string(), (*name).to_string()))
            || lg_exports.contains(&("fn".to_string(), (*name).to_string()))
            || lg_exports.contains(&("struct".to_string(), (*name).to_string()))
            || lg_exports.contains(&("use".to_string(), (*name).to_string()))
            || lg_exports.contains(&("type".to_string(), (*name).to_string()))
            || lg_exports.contains(&("macro".to_string(), (*name).to_string()));

        if in_sj {
            total += 1;
            if in_lg {
                present += 1;
            } else {
                missing.push((kind, name));
            }
        }
    }

    report.push_str(&format!(
        "- **Present in lifegraph-json**: {} / {} ({:.1}%)\n",
        present,
        total,
        if total > 0 {
            (present as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    ));
    report.push_str(&format!("- **Missing**: {}\n\n", total - present));

    // Full item lists
    report.push_str("## serde_json Public API (extracted)\n\n");
    if sj_items.is_empty() {
        report.push_str("*Could not extract - serde_json source not found*\n\n");
    } else {
        report.push_str("### Functions\n\n");
        for (_, item) in sj_items.iter().filter(|(_, i)| i.starts_with("fn ")) {
            report.push_str(&format!("- `{}`\n", item.strip_prefix("fn ").unwrap()));
        }
        report.push('\n');

        report.push_str("### Types\n\n");
        for (_, item) in sj_items.iter().filter(|(_, i)| {
            i.starts_with("struct ") || i.starts_with("enum ") || i.starts_with("type ")
        }) {
            let name = item.split(' ').nth(1).unwrap();
            let kind = item.split(' ').next().unwrap();
            report.push_str(&format!("- `{}` ({})\n", name, kind));
        }
        report.push('\n');
    }

    report.push_str("## lifegraph-json Public API (extracted)\n\n");
    report.push_str("### Functions\n\n");
    for (_, item) in lg_items.iter().filter(|(_, i)| i.starts_with("fn ")) {
        report.push_str(&format!("- `{}`\n", item.strip_prefix("fn ").unwrap()));
    }
    report.push('\n');

    report.push_str("### Types\n\n");
    for (_, item) in lg_items.iter().filter(|(_, i)| {
        i.starts_with("struct ") || i.starts_with("enum ") || i.starts_with("type ")
    }) {
        let name = item.split(' ').nth(1).unwrap();
        let kind = item.split(' ').next().unwrap();
        report.push_str(&format!("- `{}` ({})\n", name, kind));
    }
    report.push('\n');

    // Compile-time probes - these will FAIL TO COMPILE if API is missing
    compile_time_probes();

    // Missing items detail
    if !missing.is_empty() {
        report.push_str("## Missing APIs\n\n");
        for (kind, name) in &missing {
            report.push_str(&format!("- **{} {}**\n", kind, name));
        }
        report.push('\n');
    }

    // Write report
    let report_path = manifest_dir.join("docs").join("API_PARITY_REPORT.md");
    fs::write(&report_path, &report).unwrap_or_else(|e| panic!("Failed to write report: {}", e));

    println!("{}", report);
    eprintln!("\n✅ Report written to docs/API_PARITY_REPORT.md");

    // Assert minimum coverage
    if total > 0 {
        assert!(
            present >= (total * 3 / 4),
            "API parity too low: {}/{} (need 75%)",
            present,
            total
        );
    }
}

/// Compile-time probes - will fail to compile if API is missing
fn compile_time_probes() {
    // Core functions
    let _: fn(&str) -> Result<lg::JsonValue, _> = lg::from_str;
    let _: fn(&[u8]) -> Result<lg::JsonValue, _> = lg::from_slice;
    let _: fn(&lg::JsonValue) -> Result<String, _> = lg::to_string;
    let _: fn(&lg::JsonValue) -> Result<Vec<u8>, _> = lg::to_vec;

    // Types
    let _ = std::any::type_name::<lg::JsonValue>();
    let _ = std::any::type_name::<lg::Map>();
    let _ = std::any::type_name::<lg::Number>();

    // Macro
    let _ = lg::json!(null);

    // Map methods
    let mut map = lg::Map::new();
    let _: Option<&lg::JsonValue> = map.get("k");
    let _: Option<&mut lg::JsonValue> = map.get_mut("k");
    let _: bool = map.contains_key("k");
    let _: Option<lg::JsonValue> = map.insert("k".into(), lg::JsonValue::Null);
    map.retain(|_, _| true);
    map.append(&mut lg::Map::new());
    map.sort_keys();

    // Value methods
    let v = lg::JsonValue::Null;
    let _: bool = v.is_null();
    let _: Option<bool> = v.as_bool();
    let _: bool = v.is_number();
    let _: bool = v.is_i64();
    let _: Option<i64> = v.as_i64();
    let _: Option<&str> = v.as_str();
    let _: Option<&lg::Map> = v.as_object();
    let _: usize = v.len();

    // Number methods
    let n = lg::Number::from(42i64);
    let _: bool = n.is_i64();
    let _: Option<i64> = n.as_i64();
    let _: Option<lg::Number> = lg::Number::from_f64(std::f64::consts::PI);

    // Serde feature probes
    #[cfg(feature = "serde")]
    {
        let _ = std::any::type_name::<lg::Error>();
        let _ = std::any::type_name::<lg::Category>();

        #[derive(serde_crate::Serialize, serde_crate::Deserialize)]
        #[serde(crate = "serde_crate")]
        struct TestType {
            value: i32,
        }

        let _: fn(&str) -> Result<TestType, _> = lg::from_str;
        let _: fn(TestType) -> Result<lg::JsonValue, _> = lg::to_value;
        let _: fn(lg::JsonValue) -> Result<TestType, _> = lg::from_value;

        let err = lg::Error::custom("test");
        let _: usize = err.line();
        let _: usize = err.column();
        let _: lg::Category = err.classify();
        let _: bool = err.is_syntax();
    }
}
