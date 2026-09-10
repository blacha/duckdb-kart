use crate::git::{git_oid, Repo};
use crate::msgpack::{self, Value};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnSchema {
    pub id: String,
    pub name: String,
    pub data_type: String,
    pub primary_key_index: Option<usize>,
    pub size: Option<usize>,
    pub geometry_type: Option<String>,
    #[serde(rename = "geometryCRS")]
    pub geometry_crs: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Legend {
    pub pk_columns: Vec<String>,
    pub non_pk_columns: Vec<String>,
}

impl Legend {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let val = msgpack::decode(bytes)?;
        let arr = val.as_array().ok_or("Legend is not a msgpack array")?;
        if arr.len() != 2 {
            return Err(format!("Legend array expected 2 elements, got {}", arr.len()));
        }

        let parse_uuids = |v: &Value| -> Result<Vec<String>, String> {
            let items = v.as_array().ok_or("Expected array of UUIDs")?;
            let mut res = Vec::with_capacity(items.len());
            for item in items {
                res.push(item.as_str().ok_or("UUID is not a string")?.to_string());
            }
            Ok(res)
        };

        let pk_cols = parse_uuids(&arr[0])?;
        let non_pk_cols = parse_uuids(&arr[1])?;

        Ok(Self {
            pk_columns: pk_cols,
            non_pk_columns: non_pk_cols,
        })
    }
}

pub fn base64_urlsafe_decode(mut input: &str) -> Result<Vec<u8>, String> {
    if let Some(rest) = input.strip_prefix("base64:") {
        input = rest;
    }
    let input = input.trim_end_matches('=');
    let mut buf = 0u32;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity((input.len() * 3) / 4);

    for &b in input.as_bytes() {
        let val = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            b'+' => 62, // handle standard base64 fallback
            b'/' => 63,
            b' ' | b'\r' | b'\n' | b'\t' => continue,
            _ => return Err(format!("Invalid base64 character: {}", b as char)),
        } as u32;

        buf = (buf << 6) | val;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    Ok(out)
}

pub fn base64_urlsafe_decode_slice<'a>(mut input: &str, out: &'a mut [u8]) -> Result<&'a [u8], String> {
    if let Some(rest) = input.strip_prefix("base64:") {
        input = rest;
    }
    let input = input.trim_end_matches('=');
    let mut buf = 0u32;
    let mut bits = 0u32;
    let mut out_idx = 0;

    for &b in input.as_bytes() {
        let val = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            b' ' | b'\r' | b'\n' | b'\t' => continue,
            _ => return Err(format!("Invalid base64 character: {}", b as char)),
        } as u32;

        buf = (buf << 6) | val;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            if out_idx >= out.len() {
                return Err("Base64 output buffer overflow".to_string());
            }
            out[out_idx] = (buf >> bits) as u8;
            out_idx += 1;
            buf &= (1 << bits) - 1;
        }
    }

    Ok(&out[..out_idx])
}

#[inline]
pub fn decode_pk_into<'a>(
    filename: &str,
    byte_buf: &'a mut [u8],
    out_vals: &mut [Value<'a>; 4],
) -> Result<usize, String> {
    let bytes = base64_urlsafe_decode_slice(filename, byte_buf)?;
    let val = msgpack::decode(bytes)?;
    match val {
        Value::Array(arr) => {
            let n = arr.len().min(out_vals.len());
            for i in 0..n {
                out_vals[i] = arr[i].clone();
            }
            Ok(n)
        }
        single => {
            out_vals[0] = single;
            Ok(1)
        }
    }
}

pub fn decode_pk_from_filename(filename: &str) -> Result<Vec<Value<'static>>, String> {
    let bytes = base64_urlsafe_decode(filename)?;
    let boxed: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    let val = msgpack::decode(boxed)?;
    match val {
        Value::Array(arr) => Ok(arr),
        single => Ok(vec![single]),
    }
}

pub fn gpkg_to_wkb(gpkg_bytes: &[u8]) -> Option<&[u8]> {
    if gpkg_bytes.len() < 8 || &gpkg_bytes[0..2] != b"GP" {
        return None;
    }
    let flags = gpkg_bytes[3];
    let envelope_indicator = (flags >> 1) & 0b111;
    let envelope_size = match envelope_indicator {
        0 => 0,
        1 => 32,
        2 => 48,
        3 => 48,
        4 => 64,
        _ => 0,
    };
    let wkb_offset = 8 + envelope_size;
    if gpkg_bytes.len() >= wkb_offset {
        Some(&gpkg_bytes[wkb_offset..])
    } else {
        None
    }
}

pub struct DatasetInfo {
    pub name: String,
    pub dataset_type: String,
    pub feature_count: usize,
    pub geometry_crs: Option<String>,
    pub geometry_type: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum ColumnSource {
    Pk(usize),
    NonPk(usize),
    NotFound,
}

#[derive(Debug, Clone)]
pub struct CompiledLegend {
    pub sources: Vec<ColumnSource>,
}

pub struct Dataset {
    pub name: String,
    pub schema: Vec<ColumnSchema>,
    pub legends: HashMap<String, CompiledLegend>,
    pub single_legend: Option<CompiledLegend>,
    pub features: Vec<(String, git_oid)>, // (filename, blob_oid)
}

impl Dataset {
    pub fn load(repo: &Repo, dataset_name: &str) -> Result<Self, String> {
        let head_tree = repo.get_head_tree()?;
        let meta_prefix = format!("{}/.table-dataset/meta", dataset_name);

        // 1. Read schema.json
        let schema_path = format!("{}/schema.json", meta_prefix);
        let (schema_oid, _) = head_tree.get_entry_bypath(&schema_path)?;
        let schema_raw = repo.read_blob(&schema_oid)?;
        let schema: Vec<ColumnSchema> = serde_json::from_slice(&schema_raw)
            .map_err(|e| format!("Failed to parse schema.json: {}", e))?;

        // 2. Read legends and precompile column sources
        let legend_dir_path = format!("{}/legend", meta_prefix);
        let mut legends = HashMap::new();
        if let Ok((legend_dir_oid, _)) = head_tree.get_entry_bypath(&legend_dir_path) {
            if let Ok(legend_tree) = repo.lookup_tree(&legend_dir_oid) {
                let count = legend_tree.entry_count();
                for i in 0..count {
                    if let Some((hash, oid, _)) = legend_tree.get_entry_by_index(i) {
                        if let Ok(blob_bytes) = repo.read_blob(&oid) {
                            if let Ok(legend) = Legend::parse(&blob_bytes) {
                                let sources: Vec<ColumnSource> = schema
                                    .iter()
                                    .map(|col| {
                                        if let Some(pos) =
                                            legend.pk_columns.iter().position(|id| id == &col.id)
                                        {
                                            ColumnSource::Pk(pos)
                                        } else if let Some(pos) =
                                            legend.non_pk_columns.iter().position(|id| id == &col.id)
                                        {
                                            ColumnSource::NonPk(pos)
                                        } else {
                                            ColumnSource::NotFound
                                        }
                                    })
                                    .collect();
                                legends.insert(hash, CompiledLegend { sources });
                            }
                        }
                    }
                }
            }
        }

        let single_legend = if legends.len() == 1 {
            legends.values().next().cloned()
        } else {
            None
        };

        // 3. Collect features
        let feature_dir_path = format!("{}/.table-dataset/feature", dataset_name);
        let mut features = Vec::new();
        if let Ok((feature_dir_oid, _)) = head_tree.get_entry_bypath(&feature_dir_path) {
            if let Ok(feature_tree) = repo.lookup_tree(&feature_dir_oid) {
                feature_tree.walk_blobs(|filename, oid| {
                    features.push((filename.to_string(), *oid));
                    Ok(())
                })?;
            }
        }

        Ok(Self {
            name: dataset_name.to_string(),
            schema,
            legends,
            single_legend,
            features,
        })
    }
}

pub fn list_datasets(repo: &Repo) -> Result<Vec<DatasetInfo>, String> {
    let head_tree = repo.get_head_tree()?;
    let count = head_tree.entry_count();
    let mut datasets = Vec::new();

    for i in 0..count {
        if let Some((name, _, _)) = head_tree.get_entry_by_index(i) {
            let table_path = format!("{}/.table-dataset/meta/schema.json", name);
            if let Ok((schema_oid, _)) = head_tree.get_entry_bypath(&table_path) {
                let schema_raw = repo.read_blob(&schema_oid).unwrap_or_default();
                let schema: Result<Vec<ColumnSchema>, _> = serde_json::from_slice(&schema_raw);

                let (geom_crs, geom_type) = if let Ok(cols) = &schema {
                    let geom_col = cols.iter().find(|c| c.data_type == "geometry");
                    (
                        geom_col.and_then(|c| c.geometry_crs.clone()),
                        geom_col.and_then(|c| c.geometry_type.clone()),
                    )
                } else {
                    (None, None)
                };

                let feature_path = format!("{}/.table-dataset/feature", name);
                let mut feat_count = 0;
                if let Ok((feat_oid, _)) = head_tree.get_entry_bypath(&feature_path) {
                    if let Ok(feat_tree) = repo.lookup_tree(&feat_oid) {
                        let _ = feat_tree.walk_blobs(|_, _| {
                            feat_count += 1;
                            Ok(())
                        });
                    }
                }

                datasets.push(DatasetInfo {
                    name,
                    dataset_type: "table".to_string(),
                    feature_count: feat_count,
                    geometry_crs: geom_crs,
                    geometry_type: geom_type,
                });
            }
        }
    }

    Ok(datasets)
}
