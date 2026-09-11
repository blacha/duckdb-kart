use crate::duckdb_ffi::*;
use crate::git::Repo;
use crate::kart::{self, gpkg_to_wkb, ColumnSource, Dataset, DatasetInfo};
use crate::msgpack::{self, Value};
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct KartBindData {
    repo_path: String,
    repo: Arc<Repo>,
    dataset: Arc<Dataset>,
    col_types: Vec<duckdb_type>,
}

struct KartScanGlobalState {
    current_idx: AtomicUsize,
}

struct KartScanLocalState {
    repo: Repo,
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

    let dataset = match Dataset::load(&repo, &repo_path, &dataset_name) {
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
        repo_path,
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
    let bind_data = &*(duckdb_init_get_bind_data(info) as *const KartBindData);
    let total_features = bind_data.dataset.features.len();

    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min((total_features + 2047) / 2048)
        .min(8)
        .max(1);

    duckdb_init_set_max_threads(info, threads as idx_t);

    let state = Box::new(KartScanGlobalState {
        current_idx: AtomicUsize::new(0),
    });
    duckdb_init_set_init_data(
        info,
        Box::into_raw(state) as *mut c_void,
        Some(destroy_boxed::<KartScanGlobalState>),
    );
}

pub unsafe extern "C" fn read_kart_local_init(info: duckdb_init_info) {
    let bind_data = &*(duckdb_init_get_bind_data(info) as *const KartBindData);
    let repo = match Repo::open(&bind_data.repo_path) {
        Ok(r) => r,
        Err(e) => {
            let err = CString::new(format!("Failed to open repo for worker thread: {}", e)).unwrap();
            duckdb_init_set_error(info, err.as_ptr());
            return;
        }
    };
    let local_state = Box::new(KartScanLocalState { repo });
    duckdb_init_set_init_data(
        info,
        Box::into_raw(local_state) as *mut c_void,
        Some(destroy_boxed::<KartScanLocalState>),
    );
}

#[inline]
fn format_i64_to_buf(n: i64, buf: &mut [u8; 32]) -> &str {
    if n == 0 {
        return "0";
    }
    let is_neg = n < 0;
    let mut val = if is_neg {
        n.unsigned_abs()
    } else {
        n as u64
    };
    let mut idx = buf.len();
    while val > 0 {
        idx -= 1;
        buf[idx] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    if is_neg {
        idx -= 1;
        buf[idx] = b'-';
    }
    unsafe { std::str::from_utf8_unchecked(&buf[idx..]) }
}

pub unsafe extern "C" fn read_kart_scan(info: duckdb_function_info, output: duckdb_data_chunk) {
    let bind_data = &*(duckdb_function_get_bind_data(info) as *const KartBindData);
    let global_state = &*(duckdb_function_get_init_data(info) as *const KartScanGlobalState);
    let local_state = duckdb_function_get_local_init_data(info) as *mut KartScanLocalState;

    let total_features = bind_data.dataset.features.len();
    let batch_size = 2048;

    let start_idx = global_state.current_idx.fetch_add(batch_size, Ordering::Relaxed);
    if start_idx >= total_features {
        duckdb_data_chunk_set_size(output, 0);
        return;
    }

    let this_batch = batch_size.min(total_features - start_idx);

    let repo = if !local_state.is_null() {
        &(*local_state).repo
    } else {
        &bind_data.repo
    };

    // Get vector pointers and validity masks for all columns
    let num_cols = bind_data.dataset.schema.len();
    let mut vectors = Vec::with_capacity(num_cols);
    let mut data_ptrs = Vec::with_capacity(num_cols);
    let mut validity_ptrs = Vec::with_capacity(num_cols);

    for c in 0..num_cols {
        let vec = duckdb_data_chunk_get_vector(output, c as idx_t);
        let ptr = duckdb_vector_get_data(vec);
        duckdb_vector_ensure_validity_writable(vec);
        let val = duckdb_vector_get_validity(vec);
        vectors.push(vec);
        data_ptrs.push(ptr);
        validity_ptrs.push(val);
    }

    let single_legend = bind_data.dataset.single_legend.as_ref();

    for row_offset in 0..this_batch {
        let feat_idx = start_idx + row_offset;
        let (filename, blob_oid) = &bind_data.dataset.features[feat_idx];

        // 1. Decode PK from filename using stack buffer (no heap allocations!)
        let mut pk_buf = [0u8; 128];
        let mut pk_vals = [Value::Nil, Value::Nil, Value::Nil, Value::Nil];
        let pk_count = kart::decode_pk_into(filename, &mut pk_buf, &mut pk_vals).unwrap_or(0);

        // 2. Read feature blob with zero copy
        let res = repo.with_blob(blob_oid, |blob_bytes| {
            // 3. Decode MsgPack directly from blob_bytes
            let unpacked = match msgpack::decode(blob_bytes) {
                Ok(v) => v,
                Err(_) => return false,
            };

            let arr = match unpacked.as_array() {
                Some(a) if a.len() == 2 => a,
                _ => return false,
            };

            let non_pk_values = match arr[1].as_array() {
                Some(a) => a,
                None => return false,
            };

            let compiled_legend = if let Some(sl) = single_legend {
                sl
            } else {
                let legend_hash = arr[0].as_str().unwrap_or_default();
                match bind_data.dataset.legends.get(legend_hash) {
                    Some(l) => l,
                    None => return false,
                }
            };

            // Write each column value into DuckDB vector
            for col_idx in 0..num_cols {
                let val_opt = match compiled_legend.sources[col_idx] {
                    ColumnSource::Pk(idx) if idx < pk_count => Some(&pk_vals[idx]),
                    ColumnSource::NonPk(idx) => non_pk_values.get(idx),
                    _ => None,
                };

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
                            let col_schema = &bind_data.dataset.schema[col_idx];
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
                            if let Some(str_val) = val.as_str() {
                                duckdb_vector_assign_string_element_len(
                                    vectors[col_idx],
                                    row_offset as idx_t,
                                    str_val.as_ptr() as *const c_char,
                                    str_val.len() as idx_t,
                                );
                            } else if let Some(i_val) = val.as_i64() {
                                let mut num_buf = [0u8; 32];
                                let s = format_i64_to_buf(i_val, &mut num_buf);
                                duckdb_vector_assign_string_element_len(
                                    vectors[col_idx],
                                    row_offset as idx_t,
                                    s.as_ptr() as *const c_char,
                                    s.len() as idx_t,
                                );
                            } else if let Some(f_val) = val.as_f64() {
                                let s = f_val.to_string();
                                duckdb_vector_assign_string_element_len(
                                    vectors[col_idx],
                                    row_offset as idx_t,
                                    s.as_ptr() as *const c_char,
                                    s.len() as idx_t,
                                );
                            } else if let Some(b_val) = val.as_bool() {
                                let s = if b_val { "true" } else { "false" };
                                duckdb_vector_assign_string_element_len(
                                    vectors[col_idx],
                                    row_offset as idx_t,
                                    s.as_ptr() as *const c_char,
                                    s.len() as idx_t,
                                );
                            } else {
                                duckdb_validity_set_row_invalid(
                                    validity_ptrs[col_idx],
                                    row_offset as idx_t,
                                );
                            }
                        }
                        _ => {
                            duckdb_validity_set_row_invalid(validity_ptrs[col_idx], row_offset as idx_t);
                        }
                    },
                }
            }
            true
        });

        if res != Ok(true) {
            for c in 0..num_cols {
                duckdb_validity_set_row_invalid(validity_ptrs[c], row_offset as idx_t);
            }
        }
    }

    duckdb_data_chunk_set_size(output, this_batch as idx_t);
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

    let datasets = match kart::list_datasets(&repo, &repo_path) {
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

    duckdb_vector_ensure_validity_writable(vec_crs);
    duckdb_vector_ensure_validity_writable(vec_geom_type);
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
