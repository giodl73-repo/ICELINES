//! Raw shape/coverage receipts. Field names and event tags do not establish task semantics.
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
const MAX_BYTES: usize = 32 * 1024 * 1024;
#[derive(Debug, Serialize, PartialEq)]
pub struct PathCount {
    pub path: String,
    pub occurrences: usize,
    pub null_values: usize,
    pub kinds: BTreeMap<String, usize>,
}
#[derive(Debug, Serialize)]
pub struct Column {
    pub name: String,
    pub nonempty: usize,
    pub empty: usize,
}
#[derive(Debug, Serialize)]
pub struct EventTags {
    pub rows: usize,
    pub type_counts: BTreeMap<String, usize>,
    pub missing_or_nonstring_type: usize,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub schema: String,
    pub format: String,
    pub input_sha256: String,
    pub bytes: usize,
    pub json_paths: Vec<PathCount>,
    pub observed_plays_tags: Option<EventTags>,
    pub csv_rows: Option<usize>,
    pub csv_columns: Vec<Column>,
    pub limit: String,
}
pub fn inventory(format: &str, raw: &str) -> Result<Inventory, String> {
    if raw.len() > MAX_BYTES {
        return Err("Raw input exceeds32MiB".into());
    }
    let mut out=Inventory{schema:"icelines.raw-source-inventory.v1".into(),format:format.into(),input_sha256:format!("{:x}",Sha256::digest(raw.as_bytes())),bytes:raw.len(),json_paths:vec![],observed_plays_tags:None,csv_rows:None,csv_columns:vec![],limit:"Observed shapes/tags only, not field semantics, complete opportunity coverage, source authenticity or player ability. Absent paths do not occur; explicit nulls are counted separately. Arrays use[*]; object keys use JSON-quoted brackets. CSV empty strings are not interpreted as numeric zero or JSON null.".into()};
    match format {
        "json" => {
            let value: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
            fn visit(
                v: &Value,
                path: String,
                depth: usize,
                nodes: &mut usize,
                paths: &mut BTreeMap<String, PathCount>,
            ) -> Result<(), String> {
                *nodes += 1;
                if depth > 64
                    || *nodes > 2_000_000
                    || (!paths.contains_key(&path) && paths.len() >= 10_000)
                {
                    return Err("JSON inventory depth/node/path budget exceeded".into());
                }
                let kind = match v {
                    Value::Null => "null",
                    Value::Bool(_) => "bool",
                    Value::Number(_) => "number",
                    Value::String(_) => "string",
                    Value::Array(_) => "array",
                    Value::Object(_) => "object",
                };
                let p = paths.entry(path.clone()).or_insert_with(|| PathCount {
                    path: path.clone(),
                    occurrences: 0,
                    null_values: 0,
                    kinds: BTreeMap::new(),
                });
                p.occurrences += 1;
                p.null_values += usize::from(v.is_null());
                *p.kinds.entry(kind.into()).or_default() += 1;
                match v {
                    Value::Object(o) => {
                        for (k, v) in o {
                            visit(
                                v,
                                format!(
                                    "{path}[{}]",
                                    serde_json::to_string(k).map_err(|e| e.to_string())?
                                ),
                                depth + 1,
                                nodes,
                                paths,
                            )?;
                        }
                    }
                    Value::Array(a) => {
                        for v in a {
                            visit(v, format!("{path}[*]"), depth + 1, nodes, paths)?;
                        }
                    }
                    _ => {}
                }
                Ok(())
            }
            let mut paths = BTreeMap::new();
            visit(&value, "$".into(), 0, &mut 0, &mut paths)?;
            out.json_paths = paths.into_values().collect();
            if let Some(plays) = value.get("plays").and_then(Value::as_array) {
                let mut tags = EventTags {
                    rows: plays.len(),
                    type_counts: BTreeMap::new(),
                    missing_or_nonstring_type: 0,
                };
                for p in plays {
                    if let Some(t) = p.get("typeDescKey").and_then(Value::as_str) {
                        *tags.type_counts.entry(t.into()).or_default() += 1;
                    } else {
                        tags.missing_or_nonstring_type += 1;
                    }
                }
                out.observed_plays_tags = Some(tags);
            }
        }
        "csv" => {
            let mut reader = csv::Reader::from_reader(raw.as_bytes());
            let headers = reader.headers().map_err(|e| e.to_string())?.clone();
            let unique = headers.iter().collect::<BTreeSet<_>>();
            if headers.is_empty()
                || headers.iter().any(|s| s.trim().is_empty())
                || unique.len() != headers.len()
            {
                return Err("Empty or duplicate CSV headers".into());
            }
            out.csv_columns = headers
                .iter()
                .map(|name| Column {
                    name: name.into(),
                    nonempty: 0,
                    empty: 0,
                })
                .collect();
            let mut rows = 0;
            for row in reader.records() {
                let row = row.map_err(|e| e.to_string())?;
                rows += 1;
                for (i, v) in row.iter().enumerate() {
                    if v.is_empty() {
                        out.csv_columns[i].empty += 1;
                    } else {
                        out.csv_columns[i].nonempty += 1;
                    }
                }
            }
            out.csv_rows = Some(rows);
        }
        _ => return Err("Format must be json or csv; no inferred format".into()),
    }
    Ok(out)
}

#[derive(Debug, Serialize)]
pub struct CategoryGroup {
    pub value: String,
    pub rows: usize,
    pub columns: Vec<Column>,
}
#[derive(Debug, Serialize)]
pub struct CategoryInventory {
    pub schema: String,
    pub input_sha256: String,
    pub bytes: usize,
    pub group_column: String,
    pub total_rows: usize,
    pub groups: Vec<CategoryGroup>,
    pub limit: String,
}
/// Explicit-column literal census, not provider-label normalization or a task importer.
pub fn csv_categories(raw: &str, group_column: &str) -> Result<CategoryInventory, String> {
    let base = inventory("csv", raw)?;
    let Some(index) = base.csv_columns.iter().position(|c| c.name == group_column) else {
        return Err("Grouping column absent; no alias inference".into());
    };
    let mut groups = BTreeMap::<String, CategoryGroup>::new();
    let mut reader = csv::Reader::from_reader(raw.as_bytes());
    for record in reader.records() {
        let record = record.map_err(|e| e.to_string())?;
        let value = record[index].to_string();
        if !groups.contains_key(&value)
            && (groups.len() >= 512 || (groups.len() + 1) * base.csv_columns.len() > 100_000)
        {
            return Err("CSV category/column budget exceeded; no partial receipt".into());
        }
        let g = groups
            .entry(value.clone())
            .or_insert_with(|| CategoryGroup {
                value,
                rows: 0,
                columns: base
                    .csv_columns
                    .iter()
                    .map(|c| Column {
                        name: c.name.clone(),
                        empty: 0,
                        nonempty: 0,
                    })
                    .collect(),
            });
        g.rows += 1;
        for (i, v) in record.iter().enumerate() {
            if v.is_empty() {
                g.columns[i].empty += 1;
            } else {
                g.columns[i].nonempty += 1;
            }
        }
    }
    Ok(CategoryInventory{schema:"icelines.csv-category-inventory.v1".into(),input_sha256:base.input_sha256,bytes:base.bytes,group_column:group_column.into(),total_rows:base.csv_rows.expect("CSV row count"),groups:groups.into_values().collect(),limit:"Literal group/field-presence counts only. Empty labels remain distinct; nonempty is not verified identity or observability. Provider success/incomplete labels are not Rinkcraft execution/choice/receiver outcomes. No completion-rate, completeness, goalie/league/role, capability or population-transfer inference.".into()})
}
