use crate::duckdb_ffi::*;
use crate::git::Repo;
use crate::kart::{self, gpkg_to_wkb, Dataset, DatasetInfo};
use crate::msgpack::{self, Value};
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::Arc;

struct KartBindData {
    repo: Arc<Repo>,
    dataset: Arc<Dataset>,
    col_types: Vec<duckdb_type>,
}

struct KartScanState {
    current_idx: usize,
}

unsafe extern "C" fn destroy_boxed<T>(ptr: *mut c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut T));
    }
}

pub unsafe extern "C" fn read_kart_bind(info: duckdb_bind_info) {
    let param_count = duckdb_bind_get_parameter_count(info);
    let (repo_path, dataset_name) = if param_count >= 2 {
        let p0 = duckdb_bind_get_parameter(info, 0);
        let p1 = duckdb_bind_get_parameter(info, 1);
        let s0 = CStr::from_ptr(duckdb_get_varchar(p0)).to_string_lossy().into_owned();
        let s1 = CStr::from_ptr(duckdb_get_varchar(p1)).to_string_lossy().into_owned();
        duckdb_destroy_value(&mut (p0 as *mut c_void));
        duckdb_destroy_value(&mut (p1 as *mut c_void));
        (s0, s1)
    } else if param_count == 1 {
        let p0 = duckdb_bind_get_parameter(info, 0);
        let s0 = CStr::from_ptr(duckdb_get_varchar(p0)).to_string_lossy().into_owned();
        duckdb_destroy_value(&mut (p0 as *mut c_void));

        // Split "repo/dataset"
        if let Some(pos) = s0.rfind('/') {
            let r = s0[..pos].to_string();
            let d = s0[pos + 1..].to_string();
            (r, d)
        } else {
            let err = CString::new("For single-argument read_kart, specify 'repo_path/dataset_name'").unwrap();
            duckdb_bind_set_error(info, err.as_ptr());
            return;
        }
    } else {
        let err = CString::new("read_kart requires at least one parameter (repo_path, dataset_name)").unwrap();
        duckdb_bind_set_error(info, err.as_ptr());
        return;
    };

    let repo = match Repo::open(&repo_path) {
        Ok(r) => Arc::new(r),
        Err(e) => {
            let err = CString::new(format!("Failed to open Kart repo at '{}': {}", repo_path, e)).unwrap();
            duckdb_bind_set_error(info, err.as_ptr());
            return;
        }
    };

    let dataset = match Dataset::load(&repo, &dataset_name) {
        Ok(d) => Arc::new(d),
        Err(e) => {
            let err = CString::new(format!("Failed to load dataset '{}': {}", dataset_name, e)).unwrap();
            duckdb_bind_set_error(info, err.as_ptr());
            return;
        }
    };

    let mut col_types = Vec::new();

    for col in dataset.schema.iter() {
        let dt = match col.data_type.as_str() {
            "integer" => match col.size {
                Some(16) => duckdb_type::DUCKDB_TYPE_SMALLINT,
                Some(32) => duckdb_type::DUCKDB_TYPE_INTEGER,
                _ => duckdb_type::DUCKDB_TYPE_BIGINT,
            },
            "float" => duckdb_type::DUCKDB_TYPE_DOUBLE,
            "boolean" => duckdb_type::DUCKDB_TYPE_BOOLEAN,
            "geometry" => duckdb_type::DUCKDB_TYPE_BLOB,
            "blob" => duckdb_type::DUCKDB_TYPE_BLOB,
            _ => duckdb_type::DUCKDB_TYPE_VARCHAR,
        };
        col_types.push(dt);

        let logical_type = duckdb_create_logical_type(dt);
        let col_name_c = CString::new(col.name.as_str()).unwrap();
        duckdb_bind_add_result_column(info, col_name_c.as_ptr(), logical_type);
        let mut lt = logical_type;
        duckdb_destroy_logical_type(&mut lt);
    }

    let bind_data = Box::new(KartBindData {
        repo,
        dataset,
        col_types,
    });

    duckdb_bind_set_bind_data(
        info,
        Box::into_raw(bind_data) as *mut c_void,
        Some(destroy_boxed::<KartBindData>),
    );
}

pub unsafe extern "C" fn read_kart_init(info: duckdb_init_info) {
    let state = Box::new(KartScanState { current_idx: 0 });
    duckdb_init_set_init_data(
        info,
        Box::into_raw(state) as *mut c_void,
        Some(destroy_boxed::<KartScanState>),
    );
}

pub unsafe extern "C" fn read_kart_scan(info: duckdb_function_info, output: duckdb_data_chunk) {
    let bind_data = &*(duckdb_function_get_bind_data(info) as *const KartBindData);
    let state = &mut *(duckdb_function_get_init_data(info) as *mut KartScanState);

    let total_features = bind_data.dataset.features.len();
    if state.current_idx >= total_features {
        duckdb_data_chunk_set_size(output, 0);
        return;
    }

    let batch_size = 2048.min(total_features - state.current_idx);

    // Get vector pointers and validity masks for all columns
    let num_cols = bind_data.dataset.schema.len();
    let mut vectors = Vec::with_capacity(num_cols);
    let mut data_ptrs = Vec::with_capacity(num_cols);
    let mut validity_ptrs = Vec::with_capacity(num_cols);

    for c in 0..num_cols {
        let vec = duckdb_data_chunk_get_vector(output, c as idx_t);
        let ptr = duckdb_vector_get_data(vec);
        let val = duckdb_vector_get_validity(vec);
        vectors.push(vec);
        data_ptrs.push(ptr);
        validity_ptrs.push(val);
    }

    for row_offset in 0..batch_size {
        let feat_idx = state.current_idx + row_offset;
        let (feat_path, blob_oid) = &bind_data.dataset.features[feat_idx];

        // 1. Decode PK from filename
        let filename = feat_path.rsplit('/').next().unwrap_or(feat_path);
        let pk_values = kart::decode_pk_from_filename(filename).unwrap_or_default();

        // 2. Read feature blob
        let blob_bytes = match bind_data.repo.read_blob(blob_oid) {
            Ok(b) => b,
            Err(_) => {
                // Mark all columns as invalid for this row
                for c in 0..num_cols {
                    duckdb_validity_set_row_invalid(validity_ptrs[c], row_offset as idx_t);
                }
                continue;
            }
        };

        // 3. Decode MsgPack
        let unpacked = match msgpack::decode(&blob_bytes) {
            Ok(v) => v,
            Err(_) => {
                for c in 0..num_cols {
                    duckdb_validity_set_row_invalid(validity_ptrs[c], row_offset as idx_t);
                }
                continue;
            }
        };

        let arr = match unpacked.as_array() {
            Some(a) if a.len() == 2 => a,
            _ => {
                for c in 0..num_cols {
                    duckdb_validity_set_row_invalid(validity_ptrs[c], row_offset as idx_t);
                }
                continue;
            }
        };

        let legend_hash = arr[0].as_str().unwrap_or_default();
        let non_pk_values = arr[1].as_array().unwrap_or(&[]);

        let legend = match bind_data.dataset.legends.get(legend_hash) {
            Some(l) => l,
            None => {
                for c in 0..num_cols {
                    duckdb_validity_set_row_invalid(validity_ptrs[c], row_offset as idx_t);
                }
                continue;
            }
        };

        // Create mapping: col_id -> &Value
        let mut row_values: HashMap<&str, &Value> = HashMap::new();
        for (col_id, val) in legend.pk_columns.iter().zip(pk_values.iter()) {
            row_values.insert(col_id.as_str(), val);
        }
        for (col_id, val) in legend.non_pk_columns.iter().zip(non_pk_values.iter()) {
            row_values.insert(col_id.as_str(), val);
        }

        // Write each column value into DuckDB vector
        for col_idx in 0..num_cols {
            let col_schema = &bind_data.dataset.schema[col_idx];
            let val_opt = row_values.get(col_schema.id.as_str());

            match val_opt {
                None | Some(Value::Nil) => {
                    duckdb_validity_set_row_invalid(validity_ptrs[col_idx], row_offset as idx_t);
                }
                Some(val) => match bind_data.col_types[col_idx] {
                    duckdb_type::DUCKDB_TYPE_BIGINT => {
                        let i = val.as_i64().unwrap_or(0);
                        let ptr = data_ptrs[col_idx] as *mut i64;
                        *ptr.add(row_offset) = i;
                    }
                    duckdb_type::DUCKDB_TYPE_INTEGER => {
                        let i = val.as_i64().unwrap_or(0) as i32;
                        let ptr = data_ptrs[col_idx] as *mut i32;
                        *ptr.add(row_offset) = i;
                    }
                    duckdb_type::DUCKDB_TYPE_SMALLINT => {
                        let i = val.as_i64().unwrap_or(0) as i16;
                        let ptr = data_ptrs[col_idx] as *mut i16;
                        *ptr.add(row_offset) = i;
                    }
                    duckdb_type::DUCKDB_TYPE_DOUBLE => {
                        let f = val.as_f64().unwrap_or(0.0);
                        let ptr = data_ptrs[col_idx] as *mut f64;
                        *ptr.add(row_offset) = f;
                    }
                    duckdb_type::DUCKDB_TYPE_BOOLEAN => {
                        let b = val.as_bool().unwrap_or(false);
                        let ptr = data_ptrs[col_idx] as *mut bool;
                        *ptr.add(row_offset) = b;
                    }
                    duckdb_type::DUCKDB_TYPE_BLOB => {
                        // If geometry, extract WKB from GPKG binary
                        if col_schema.data_type == "geometry" {
                            if let Some(geom_bytes) = val.as_geom_bytes() {
                                if let Some(wkb) = gpkg_to_wkb(geom_bytes) {
                                    duckdb_vector_assign_string_element_len(
                                        vectors[col_idx],
                                        row_offset as idx_t,
                                        wkb.as_ptr() as *const c_char,
                                        wkb.len() as idx_t,
                                    );
                                } else {
                                    duckdb_validity_set_row_invalid(
                                        validity_ptrs[col_idx],
                                        row_offset as idx_t,
                                    );
                                }
                            } else {
                                duckdb_validity_set_row_invalid(
                                    validity_ptrs[col_idx],
                                    row_offset as idx_t,
                                );
                            }
                        } else if let Some(bytes) = val.as_bytes() {
                            duckdb_vector_assign_string_element_len(
                                vectors[col_idx],
                                row_offset as idx_t,
                                bytes.as_ptr() as *const c_char,
                                bytes.len() as idx_t,
                            );
                        } else {
                            duckdb_validity_set_row_invalid(
                                validity_ptrs[col_idx],
                                row_offset as idx_t,
                            );
                        }
                    }
                    duckdb_type::DUCKDB_TYPE_VARCHAR => {
                        let s = if let Some(str_val) = val.as_str() {
                            str_val.to_string()
                        } else if let Some(i_val) = val.as_i64() {
                            i_val.to_string()
                        } else if let Some(f_val) = val.as_f64() {
                            f_val.to_string()
                        } else if let Some(b_val) = val.as_bool() {
                            b_val.to_string()
                        } else {
                            format!("{:?}", val)
                        };
                        duckdb_vector_assign_string_element_len(
                            vectors[col_idx],
                            row_offset as idx_t,
                            s.as_ptr() as *const c_char,
                            s.len() as idx_t,
                        );
                    }
                    _ => {
                        duckdb_validity_set_row_invalid(validity_ptrs[col_idx], row_offset as idx_t);
                    }
                },
            }
        }
    }

    duckdb_data_chunk_set_size(output, batch_size as idx_t);
    state.current_idx += batch_size;
}

// -------------------------------------------------------------------------
// read_kart_datasets table function
// -------------------------------------------------------------------------

struct DatasetsBindData {
    datasets: Vec<DatasetInfo>,
}

struct DatasetsScanState {
    current_idx: usize,
}

pub unsafe extern "C" fn read_kart_datasets_bind(info: duckdb_bind_info) {
    let p0 = duckdb_bind_get_parameter(info, 0);
    let repo_path = CStr::from_ptr(duckdb_get_varchar(p0)).to_string_lossy().into_owned();
    duckdb_destroy_value(&mut (p0 as *mut c_void));

    let repo = match Repo::open(&repo_path) {
        Ok(r) => r,
        Err(e) => {
            let err = CString::new(format!("Failed to open Kart repo at '{}': {}", repo_path, e)).unwrap();
            duckdb_bind_set_error(info, err.as_ptr());
            return;
        }
    };

    let datasets = match kart::list_datasets(&repo) {
        Ok(d) => d,
        Err(e) => {
            let err = CString::new(format!("Failed to list datasets in '{}': {}", repo_path, e)).unwrap();
            duckdb_bind_set_error(info, err.as_ptr());
            return;
        }
    };

    let add_col = |name: &str, dt: duckdb_type| {
        let logical_type = duckdb_create_logical_type(dt);
        let name_c = CString::new(name).unwrap();
        duckdb_bind_add_result_column(info, name_c.as_ptr(), logical_type);
        let mut lt = logical_type;
        duckdb_destroy_logical_type(&mut lt);
    };

    add_col("dataset_name", duckdb_type::DUCKDB_TYPE_VARCHAR);
    add_col("dataset_type", duckdb_type::DUCKDB_TYPE_VARCHAR);
    add_col("feature_count", duckdb_type::DUCKDB_TYPE_BIGINT);
    add_col("geometry_crs", duckdb_type::DUCKDB_TYPE_VARCHAR);
    add_col("geometry_type", duckdb_type::DUCKDB_TYPE_VARCHAR);

    let bind_data = Box::new(DatasetsBindData { datasets });
    duckdb_bind_set_bind_data(
        info,
        Box::into_raw(bind_data) as *mut c_void,
        Some(destroy_boxed::<DatasetsBindData>),
    );
}

pub unsafe extern "C" fn read_kart_datasets_init(info: duckdb_init_info) {
    let state = Box::new(DatasetsScanState { current_idx: 0 });
    duckdb_init_set_init_data(
        info,
        Box::into_raw(state) as *mut c_void,
        Some(destroy_boxed::<DatasetsScanState>),
    );
}

pub unsafe extern "C" fn read_kart_datasets_scan(info: duckdb_function_info, output: duckdb_data_chunk) {
    let bind_data = &*(duckdb_function_get_bind_data(info) as *const DatasetsBindData);
    let state = &mut *(duckdb_function_get_init_data(info) as *mut DatasetsScanState);

    let total = bind_data.datasets.len();
    if state.current_idx >= total {
        duckdb_data_chunk_set_size(output, 0);
        return;
    }

    let batch_size = 2048.min(total - state.current_idx);

    let vec_name = duckdb_data_chunk_get_vector(output, 0);
    let vec_type = duckdb_data_chunk_get_vector(output, 1);
    let vec_count = duckdb_data_chunk_get_vector(output, 2);
    let vec_crs = duckdb_data_chunk_get_vector(output, 3);
    let vec_geom_type = duckdb_data_chunk_get_vector(output, 4);

    let val_crs = duckdb_vector_get_validity(vec_crs);
    let val_geom_type = duckdb_vector_get_validity(vec_geom_type);
    let count_ptr = duckdb_vector_get_data(vec_count) as *mut i64;

    for i in 0..batch_size {
        let row_idx = state.current_idx + i;
        let ds = &bind_data.datasets[row_idx];

        duckdb_vector_assign_string_element_len(
            vec_name,
            i as idx_t,
            ds.name.as_ptr() as *const c_char,
            ds.name.len() as idx_t,
        );
        duckdb_vector_assign_string_element_len(
            vec_type,
            i as idx_t,
            ds.dataset_type.as_ptr() as *const c_char,
            ds.dataset_type.len() as idx_t,
        );
        *count_ptr.add(i) = ds.feature_count as i64;

        if let Some(crs) = &ds.geometry_crs {
            duckdb_vector_assign_string_element_len(
                vec_crs,
                i as idx_t,
                crs.as_ptr() as *const c_char,
                crs.len() as idx_t,
            );
        } else {
            duckdb_validity_set_row_invalid(val_crs, i as idx_t);
        }

        if let Some(gt) = &ds.geometry_type {
            duckdb_vector_assign_string_element_len(
                vec_geom_type,
                i as idx_t,
                gt.as_ptr() as *const c_char,
                gt.len() as idx_t,
            );
        } else {
            duckdb_validity_set_row_invalid(val_geom_type, i as idx_t);
        }
    }

    duckdb_data_chunk_set_size(output, batch_size as idx_t);
    state.current_idx += batch_size;
}
