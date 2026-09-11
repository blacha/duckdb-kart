#![allow(non_camel_case_types, non_snake_case, dead_code)]
use std::ffi::{c_char, c_void};

pub type idx_t = u64;

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub enum duckdb_state {
    DuckDBSuccess = 0,
    DuckDBError = 1,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum duckdb_type {
    DUCKDB_TYPE_INVALID = 0,
    DUCKDB_TYPE_BOOLEAN = 1,
    DUCKDB_TYPE_TINYINT = 2,
    DUCKDB_TYPE_SMALLINT = 3,
    DUCKDB_TYPE_INTEGER = 4,
    DUCKDB_TYPE_BIGINT = 5,
    DUCKDB_TYPE_UTINYINT = 6,
    DUCKDB_TYPE_USMALLINT = 7,
    DUCKDB_TYPE_UINTEGER = 8,
    DUCKDB_TYPE_UBIGINT = 9,
    DUCKDB_TYPE_FLOAT = 10,
    DUCKDB_TYPE_DOUBLE = 11,
    DUCKDB_TYPE_TIMESTAMP = 12,
    DUCKDB_TYPE_DATE = 13,
    DUCKDB_TYPE_TIME = 14,
    DUCKDB_TYPE_INTERVAL = 15,
    DUCKDB_TYPE_HUGEINT = 16,
    DUCKDB_TYPE_VARCHAR = 17,
    DUCKDB_TYPE_BLOB = 18,
    DUCKDB_TYPE_DECIMAL = 19,
    DUCKDB_TYPE_TIMESTAMP_S = 20,
    DUCKDB_TYPE_TIMESTAMP_MS = 21,
    DUCKDB_TYPE_TIMESTAMP_NS = 22,
    DUCKDB_TYPE_ENUM = 23,
    DUCKDB_TYPE_LIST = 24,
    DUCKDB_TYPE_STRUCT = 25,
    DUCKDB_TYPE_MAP = 26,
    DUCKDB_TYPE_UUID = 27,
    DUCKDB_TYPE_UNION = 28,
    DUCKDB_TYPE_BIT = 29,
    DUCKDB_TYPE_TIME_TZ = 30,
    DUCKDB_TYPE_TIMESTAMP_TZ = 31,
    DUCKDB_TYPE_UHUGEINT = 32,
    DUCKDB_TYPE_ARRAY = 33,
    DUCKDB_TYPE_ANY = 34,
    DUCKDB_TYPE_VARINT = 35,
    DUCKDB_TYPE_SQLNULL = 36,
}

pub type duckdb_database = *mut c_void;
pub type duckdb_connection = *mut c_void;
pub type duckdb_logical_type = *mut c_void;
pub type duckdb_table_function = *mut c_void;
pub type duckdb_bind_info = *mut c_void;
pub type duckdb_init_info = *mut c_void;
pub type duckdb_function_info = *mut c_void;
pub type duckdb_data_chunk = *mut c_void;
pub type duckdb_vector = *mut c_void;
pub type duckdb_value = *mut c_void;
pub type duckdb_extension_info = *mut c_void;

pub type duckdb_table_function_bind_t = unsafe extern "C" fn(info: duckdb_bind_info);
pub type duckdb_table_function_init_t = unsafe extern "C" fn(info: duckdb_init_info);
pub type duckdb_table_function_t = unsafe extern "C" fn(info: duckdb_function_info, output: duckdb_data_chunk);

#[repr(C)]
pub struct duckdb_extension_access {
    pub set_error: Option<unsafe extern "C" fn(info: duckdb_extension_info, error: *const c_char)>,
    pub get_database: Option<unsafe extern "C" fn(info: duckdb_extension_info) -> *mut duckdb_database>,
    pub get_api: Option<unsafe extern "C" fn(info: duckdb_extension_info, version: *const c_char) -> *const c_void>,
}

static mut DUCKDB_API_STORAGE: std::mem::MaybeUninit<duckdb_ext_api_v1> = std::mem::MaybeUninit::uninit();
static DUCKDB_API: std::sync::atomic::AtomicPtr<duckdb_ext_api_v1> = std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

pub unsafe fn init_api(api: *const duckdb_ext_api_v1) {
    let storage_ptr = std::ptr::addr_of_mut!(DUCKDB_API_STORAGE) as *mut duckdb_ext_api_v1;
    std::ptr::copy_nonoverlapping(api, storage_ptr, 1);
    DUCKDB_API.store(storage_ptr, std::sync::atomic::Ordering::Release);
}

#[inline(always)]
pub unsafe fn get_api() -> *const duckdb_ext_api_v1 {
    DUCKDB_API.load(std::sync::atomic::Ordering::Relaxed)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct duckdb_ext_api_v1 {
    pub duckdb_open: *const c_void,
    pub duckdb_open_ext: *const c_void,
    pub duckdb_close: *const c_void,
    pub duckdb_connect: Option<unsafe extern "C" fn(database: duckdb_database, out_connection: *mut duckdb_connection) -> duckdb_state>,
    pub duckdb_interrupt: *const c_void,
    pub duckdb_query_progress: *const c_void,
    pub duckdb_disconnect: Option<unsafe extern "C" fn(connection: *mut duckdb_connection)>,
    pub duckdb_library_version: *const c_void,
    pub duckdb_create_config: *const c_void,
    pub duckdb_config_count: *const c_void,
    pub duckdb_get_config_flag: *const c_void,
    pub duckdb_set_config: *const c_void,
    pub duckdb_destroy_config: *const c_void,
    pub duckdb_query: *const c_void,
    pub duckdb_destroy_result: *const c_void,
    pub duckdb_column_name: *const c_void,
    pub duckdb_column_type: *const c_void,
    pub duckdb_result_statement_type: *const c_void,
    pub duckdb_column_logical_type: *const c_void,
    pub duckdb_column_count: *const c_void,
    pub duckdb_rows_changed: *const c_void,
    pub duckdb_result_error: *const c_void,
    pub duckdb_result_error_type: *const c_void,
    pub duckdb_result_return_type: *const c_void,
    pub duckdb_malloc: *const c_void,
    pub duckdb_free: Option<unsafe extern "C" fn(ptr: *mut c_void)>,
    pub duckdb_vector_size: *const c_void,
    pub duckdb_string_is_inlined: *const c_void,
    pub duckdb_string_t_length: *const c_void,
    pub duckdb_string_t_data: *const c_void,
    pub duckdb_from_date: *const c_void,
    pub duckdb_to_date: *const c_void,
    pub duckdb_is_finite_date: *const c_void,
    pub duckdb_from_time: *const c_void,
    pub duckdb_create_time_tz: *const c_void,
    pub duckdb_from_time_tz: *const c_void,
    pub duckdb_to_time: *const c_void,
    pub duckdb_from_timestamp: *const c_void,
    pub duckdb_to_timestamp: *const c_void,
    pub duckdb_is_finite_timestamp: *const c_void,
    pub duckdb_hugeint_to_double: *const c_void,
    pub duckdb_double_to_hugeint: *const c_void,
    pub duckdb_uhugeint_to_double: *const c_void,
    pub duckdb_double_to_uhugeint: *const c_void,
    pub duckdb_double_to_decimal: *const c_void,
    pub duckdb_decimal_to_double: *const c_void,
    pub duckdb_prepare: *const c_void,
    pub duckdb_destroy_prepare: *const c_void,
    pub duckdb_prepare_error: *const c_void,
    pub duckdb_nparams: *const c_void,
    pub duckdb_parameter_name: *const c_void,
    pub duckdb_param_type: *const c_void,
    pub duckdb_param_logical_type: *const c_void,
    pub duckdb_clear_bindings: *const c_void,
    pub duckdb_prepared_statement_type: *const c_void,
    pub duckdb_bind_value: *const c_void,
    pub duckdb_bind_parameter_index: *const c_void,
    pub duckdb_bind_boolean: *const c_void,
    pub duckdb_bind_int8: *const c_void,
    pub duckdb_bind_int16: *const c_void,
    pub duckdb_bind_int32: *const c_void,
    pub duckdb_bind_int64: *const c_void,
    pub duckdb_bind_hugeint: *const c_void,
    pub duckdb_bind_uhugeint: *const c_void,
    pub duckdb_bind_decimal: *const c_void,
    pub duckdb_bind_uint8: *const c_void,
    pub duckdb_bind_uint16: *const c_void,
    pub duckdb_bind_uint32: *const c_void,
    pub duckdb_bind_uint64: *const c_void,
    pub duckdb_bind_float: *const c_void,
    pub duckdb_bind_double: *const c_void,
    pub duckdb_bind_date: *const c_void,
    pub duckdb_bind_time: *const c_void,
    pub duckdb_bind_timestamp: *const c_void,
    pub duckdb_bind_timestamp_tz: *const c_void,
    pub duckdb_bind_interval: *const c_void,
    pub duckdb_bind_varchar: *const c_void,
    pub duckdb_bind_varchar_length: *const c_void,
    pub duckdb_bind_blob: *const c_void,
    pub duckdb_bind_null: *const c_void,
    pub duckdb_execute_prepared: *const c_void,
    pub duckdb_extract_statements: *const c_void,
    pub duckdb_prepare_extracted_statement: *const c_void,
    pub duckdb_extract_statements_error: *const c_void,
    pub duckdb_destroy_extracted: *const c_void,
    pub duckdb_pending_prepared: *const c_void,
    pub duckdb_destroy_pending: *const c_void,
    pub duckdb_pending_error: *const c_void,
    pub duckdb_pending_execute_task: *const c_void,
    pub duckdb_pending_execute_check_state: *const c_void,
    pub duckdb_execute_pending: *const c_void,
    pub duckdb_pending_execution_is_finished: *const c_void,
    pub duckdb_destroy_value: Option<unsafe extern "C" fn(val: *mut duckdb_value)>,
    pub duckdb_create_varchar: *const c_void,
    pub duckdb_create_varchar_length: *const c_void,
    pub duckdb_create_bool: *const c_void,
    pub duckdb_create_int8: *const c_void,
    pub duckdb_create_uint8: *const c_void,
    pub duckdb_create_int16: *const c_void,
    pub duckdb_create_uint16: *const c_void,
    pub duckdb_create_int32: *const c_void,
    pub duckdb_create_uint32: *const c_void,
    pub duckdb_create_uint64: *const c_void,
    pub duckdb_create_int64: *const c_void,
    pub duckdb_create_hugeint: *const c_void,
    pub duckdb_create_uhugeint: *const c_void,
    pub duckdb_create_float: *const c_void,
    pub duckdb_create_double: *const c_void,
    pub duckdb_create_date: *const c_void,
    pub duckdb_create_time: *const c_void,
    pub duckdb_create_time_tz_value: *const c_void,
    pub duckdb_create_timestamp: *const c_void,
    pub duckdb_create_interval: *const c_void,
    pub duckdb_create_blob: *const c_void,
    pub duckdb_create_varint: *const c_void,
    pub duckdb_create_decimal: *const c_void,
    pub duckdb_create_bit: *const c_void,
    pub duckdb_create_uuid: *const c_void,
    pub duckdb_get_bool: *const c_void,
    pub duckdb_get_int8: *const c_void,
    pub duckdb_get_uint8: *const c_void,
    pub duckdb_get_int16: *const c_void,
    pub duckdb_get_uint16: *const c_void,
    pub duckdb_get_int32: *const c_void,
    pub duckdb_get_uint32: *const c_void,
    pub duckdb_get_int64: *const c_void,
    pub duckdb_get_uint64: *const c_void,
    pub duckdb_get_hugeint: *const c_void,
    pub duckdb_get_uhugeint: *const c_void,
    pub duckdb_get_float: *const c_void,
    pub duckdb_get_double: *const c_void,
    pub duckdb_get_date: *const c_void,
    pub duckdb_get_time: *const c_void,
    pub duckdb_get_time_tz: *const c_void,
    pub duckdb_get_timestamp: *const c_void,
    pub duckdb_get_interval: *const c_void,
    pub duckdb_get_value_type: *const c_void,
    pub duckdb_get_blob: *const c_void,
    pub duckdb_get_varint: *const c_void,
    pub duckdb_get_decimal: *const c_void,
    pub duckdb_get_bit: *const c_void,
    pub duckdb_get_uuid: *const c_void,
    pub duckdb_get_varchar: Option<unsafe extern "C" fn(val: duckdb_value) -> *mut c_char>,
    pub duckdb_create_struct_value: *const c_void,
    pub duckdb_create_list_value: *const c_void,
    pub duckdb_create_array_value: *const c_void,
    pub duckdb_get_map_size: *const c_void,
    pub duckdb_get_map_key: *const c_void,
    pub duckdb_get_map_value: *const c_void,
    pub duckdb_is_null_value: *const c_void,
    pub duckdb_create_null_value: *const c_void,
    pub duckdb_get_list_size: *const c_void,
    pub duckdb_get_list_child: *const c_void,
    pub duckdb_create_enum_value: *const c_void,
    pub duckdb_get_enum_value: *const c_void,
    pub duckdb_get_struct_child: *const c_void,
    pub duckdb_create_logical_type: Option<unsafe extern "C" fn(type_: duckdb_type) -> duckdb_logical_type>,
    pub duckdb_logical_type_get_alias: *const c_void,
    pub duckdb_logical_type_set_alias: *const c_void,
    pub duckdb_create_list_type: *const c_void,
    pub duckdb_create_array_type: *const c_void,
    pub duckdb_create_map_type: *const c_void,
    pub duckdb_create_union_type: *const c_void,
    pub duckdb_create_struct_type: *const c_void,
    pub duckdb_create_enum_type: *const c_void,
    pub duckdb_create_decimal_type: *const c_void,
    pub duckdb_get_type_id: *const c_void,
    pub duckdb_decimal_width: *const c_void,
    pub duckdb_decimal_scale: *const c_void,
    pub duckdb_decimal_internal_type: *const c_void,
    pub duckdb_enum_internal_type: *const c_void,
    pub duckdb_enum_dictionary_size: *const c_void,
    pub duckdb_enum_dictionary_value: *const c_void,
    pub duckdb_list_type_child_type: *const c_void,
    pub duckdb_array_type_child_type: *const c_void,
    pub duckdb_array_type_array_size: *const c_void,
    pub duckdb_map_type_key_type: *const c_void,
    pub duckdb_map_type_value_type: *const c_void,
    pub duckdb_struct_type_child_count: *const c_void,
    pub duckdb_struct_type_child_name: *const c_void,
    pub duckdb_struct_type_child_type: *const c_void,
    pub duckdb_union_type_member_count: *const c_void,
    pub duckdb_union_type_member_name: *const c_void,
    pub duckdb_union_type_member_type: *const c_void,
    pub duckdb_destroy_logical_type: Option<unsafe extern "C" fn(type_: *mut duckdb_logical_type)>,
    pub duckdb_register_logical_type: *const c_void,
    pub duckdb_create_data_chunk: *const c_void,
    pub duckdb_destroy_data_chunk: *const c_void,
    pub duckdb_data_chunk_reset: *const c_void,
    pub duckdb_data_chunk_get_column_count: *const c_void,
    pub duckdb_data_chunk_get_vector: Option<unsafe extern "C" fn(chunk: duckdb_data_chunk, col_idx: idx_t) -> duckdb_vector>,
    pub duckdb_data_chunk_get_size: *const c_void,
    pub duckdb_data_chunk_set_size: Option<unsafe extern "C" fn(chunk: duckdb_data_chunk, size: idx_t)>,
    pub duckdb_vector_get_column_type: *const c_void,
    pub duckdb_vector_get_data: Option<unsafe extern "C" fn(vector: duckdb_vector) -> *mut c_void>,
    pub duckdb_vector_get_validity: Option<unsafe extern "C" fn(vector: duckdb_vector) -> *mut u64>,
    pub duckdb_vector_ensure_validity_writable: Option<unsafe extern "C" fn(vector: duckdb_vector)>,
    pub duckdb_vector_assign_string_element: *const c_void,
    pub duckdb_vector_assign_string_element_len: Option<unsafe extern "C" fn(vector: duckdb_vector, index: idx_t, str_: *const c_char, str_len: idx_t)>,
    pub duckdb_list_vector_get_child: *const c_void,
    pub duckdb_list_vector_get_size: *const c_void,
    pub duckdb_list_vector_set_size: *const c_void,
    pub duckdb_list_vector_reserve: *const c_void,
    pub duckdb_struct_vector_get_child: *const c_void,
    pub duckdb_array_vector_get_child: *const c_void,
    pub duckdb_validity_row_is_valid: *const c_void,
    pub duckdb_validity_set_row_validity: *const c_void,
    pub duckdb_validity_set_row_invalid: *const c_void,
    pub duckdb_validity_set_row_valid: *const c_void,
    pub duckdb_create_scalar_function: *const c_void,
    pub duckdb_destroy_scalar_function: *const c_void,
    pub duckdb_scalar_function_set_name: *const c_void,
    pub duckdb_scalar_function_set_varargs: *const c_void,
    pub duckdb_scalar_function_set_special_handling: *const c_void,
    pub duckdb_scalar_function_set_volatile: *const c_void,
    pub duckdb_scalar_function_add_parameter: *const c_void,
    pub duckdb_scalar_function_set_return_type: *const c_void,
    pub duckdb_scalar_function_set_extra_info: *const c_void,
    pub duckdb_scalar_function_set_function: *const c_void,
    pub duckdb_register_scalar_function: *const c_void,
    pub duckdb_scalar_function_get_extra_info: *const c_void,
    pub duckdb_scalar_function_set_error: *const c_void,
    pub duckdb_create_scalar_function_set: *const c_void,
    pub duckdb_destroy_scalar_function_set: *const c_void,
    pub duckdb_add_scalar_function_to_set: *const c_void,
    pub duckdb_register_scalar_function_set: *const c_void,
    pub duckdb_create_aggregate_function: *const c_void,
    pub duckdb_destroy_aggregate_function: *const c_void,
    pub duckdb_aggregate_function_set_name: *const c_void,
    pub duckdb_aggregate_function_add_parameter: *const c_void,
    pub duckdb_aggregate_function_set_return_type: *const c_void,
    pub duckdb_aggregate_function_set_functions: *const c_void,
    pub duckdb_aggregate_function_set_destructor: *const c_void,
    pub duckdb_register_aggregate_function: *const c_void,
    pub duckdb_aggregate_function_set_special_handling: *const c_void,
    pub duckdb_aggregate_function_set_extra_info: *const c_void,
    pub duckdb_aggregate_function_get_extra_info: *const c_void,
    pub duckdb_aggregate_function_set_error: *const c_void,
    pub duckdb_create_aggregate_function_set: *const c_void,
    pub duckdb_destroy_aggregate_function_set: *const c_void,
    pub duckdb_add_aggregate_function_to_set: *const c_void,
    pub duckdb_register_aggregate_function_set: *const c_void,
    pub duckdb_create_table_function: Option<unsafe extern "C" fn() -> duckdb_table_function>,
    pub duckdb_destroy_table_function: Option<unsafe extern "C" fn(table_function: *mut duckdb_table_function)>,
    pub duckdb_table_function_set_name: Option<unsafe extern "C" fn(table_function: duckdb_table_function, name: *const c_char)>,
    pub duckdb_table_function_add_parameter: Option<unsafe extern "C" fn(table_function: duckdb_table_function, type_: duckdb_logical_type)>,
    pub duckdb_table_function_add_named_parameter: Option<unsafe extern "C" fn(table_function: duckdb_table_function, name: *const c_char, type_: duckdb_logical_type)>,
    pub duckdb_table_function_set_extra_info: *const c_void,
    pub duckdb_table_function_set_bind: Option<unsafe extern "C" fn(table_function: duckdb_table_function, bind: duckdb_table_function_bind_t)>,
    pub duckdb_table_function_set_init: Option<unsafe extern "C" fn(table_function: duckdb_table_function, init: duckdb_table_function_init_t)>,
    pub duckdb_table_function_set_local_init: Option<unsafe extern "C" fn(table_function: duckdb_table_function, init: duckdb_table_function_init_t)>,
    pub duckdb_table_function_set_function: Option<unsafe extern "C" fn(table_function: duckdb_table_function, function: duckdb_table_function_t)>,
    pub duckdb_table_function_supports_projection_pushdown: *const c_void,
    pub duckdb_register_table_function: Option<unsafe extern "C" fn(con: duckdb_connection, function: duckdb_table_function) -> duckdb_state>,
    pub duckdb_bind_get_extra_info: *const c_void,
    pub duckdb_bind_add_result_column: Option<unsafe extern "C" fn(info: duckdb_bind_info, name: *const c_char, type_: duckdb_logical_type)>,
    pub duckdb_bind_get_parameter_count: Option<unsafe extern "C" fn(info: duckdb_bind_info) -> idx_t>,
    pub duckdb_bind_get_parameter: Option<unsafe extern "C" fn(info: duckdb_bind_info, index: idx_t) -> duckdb_value>,
    pub duckdb_bind_get_named_parameter: Option<unsafe extern "C" fn(info: duckdb_bind_info, name: *const c_char) -> duckdb_value>,
    pub duckdb_bind_set_bind_data: Option<unsafe extern "C" fn(info: duckdb_bind_info, extra_data: *mut c_void, destroy: Option<unsafe extern "C" fn(*mut c_void)>)>,
    pub duckdb_bind_set_cardinality: *const c_void,
    pub duckdb_bind_set_error: Option<unsafe extern "C" fn(info: duckdb_bind_info, error: *const c_char)>,
    pub duckdb_init_get_extra_info: *const c_void,
    pub duckdb_init_get_bind_data: Option<unsafe extern "C" fn(info: duckdb_init_info) -> *mut c_void>,
    pub duckdb_init_set_init_data: Option<unsafe extern "C" fn(info: duckdb_init_info, extra_data: *mut c_void, destroy: Option<unsafe extern "C" fn(*mut c_void)>)>,
    pub duckdb_init_get_column_count: *const c_void,
    pub duckdb_init_get_column_index: *const c_void,
    pub duckdb_init_set_max_threads: Option<unsafe extern "C" fn(info: duckdb_init_info, max_threads: idx_t)>,
    pub duckdb_init_set_error: Option<unsafe extern "C" fn(info: duckdb_init_info, error: *const c_char)>,
    pub duckdb_function_get_extra_info: *const c_void,
    pub duckdb_function_get_bind_data: Option<unsafe extern "C" fn(info: duckdb_function_info) -> *mut c_void>,
    pub duckdb_function_get_init_data: Option<unsafe extern "C" fn(info: duckdb_function_info) -> *mut c_void>,
    pub duckdb_function_get_local_init_data: Option<unsafe extern "C" fn(info: duckdb_function_info) -> *mut c_void>,
    pub duckdb_function_set_error: Option<unsafe extern "C" fn(info: duckdb_function_info, error: *const c_char)>,
    pub duckdb_add_replacement_scan: *const c_void,
    pub duckdb_replacement_scan_set_function_name: *const c_void,
    pub duckdb_replacement_scan_add_parameter: *const c_void,
    pub duckdb_replacement_scan_set_error: *const c_void,
    pub duckdb_profiling_info_get_metrics: *const c_void,
    pub duckdb_profiling_info_get_child_count: *const c_void,
    pub duckdb_profiling_info_get_child: *const c_void,
    pub duckdb_appender_create: *const c_void,
    pub duckdb_appender_create_ext: *const c_void,
    pub duckdb_appender_column_count: *const c_void,
    pub duckdb_appender_column_type: *const c_void,
    pub duckdb_appender_error: *const c_void,
    pub duckdb_appender_flush: *const c_void,
    pub duckdb_appender_close: *const c_void,
    pub duckdb_appender_destroy: *const c_void,
    pub duckdb_appender_add_column: *const c_void,
    pub duckdb_appender_clear_columns: *const c_void,
    pub duckdb_append_data_chunk: *const c_void,
    pub duckdb_table_description_create: *const c_void,
    pub duckdb_table_description_create_ext: *const c_void,
    pub duckdb_table_description_destroy: *const c_void,
    pub duckdb_table_description_error: *const c_void,
    pub duckdb_column_has_default: *const c_void,
    pub duckdb_table_description_get_column_name: *const c_void,
    pub duckdb_execute_tasks: *const c_void,
    pub duckdb_create_task_state: *const c_void,
    pub duckdb_execute_tasks_state: *const c_void,
    pub duckdb_execute_n_tasks_state: *const c_void,
    pub duckdb_finish_execution: *const c_void,
    pub duckdb_task_state_is_finished: *const c_void,
    pub duckdb_destroy_task_state: *const c_void,
    pub duckdb_execution_is_finished: *const c_void,
    pub duckdb_fetch_chunk: *const c_void,
    pub duckdb_create_cast_function: *const c_void,
    pub duckdb_cast_function_set_source_type: *const c_void,
    pub duckdb_cast_function_set_target_type: *const c_void,
    pub duckdb_cast_function_set_implicit_cast_cost: *const c_void,
    pub duckdb_cast_function_set_function: *const c_void,
    pub duckdb_cast_function_set_extra_info: *const c_void,
    pub duckdb_cast_function_get_extra_info: *const c_void,
    pub duckdb_cast_function_get_cast_mode: *const c_void,
    pub duckdb_cast_function_set_error: *const c_void,
    pub duckdb_cast_function_set_row_error: *const c_void,
    pub duckdb_register_cast_function: *const c_void,
    pub duckdb_destroy_cast_function: *const c_void,
    pub duckdb_is_finite_timestamp_s: *const c_void,
    pub duckdb_is_finite_timestamp_ms: *const c_void,
    pub duckdb_is_finite_timestamp_ns: *const c_void,
    pub duckdb_create_timestamp_tz: *const c_void,
    pub duckdb_create_timestamp_s: *const c_void,
    pub duckdb_create_timestamp_ms: *const c_void,
    pub duckdb_create_timestamp_ns: *const c_void,
    pub duckdb_get_timestamp_tz: *const c_void,
    pub duckdb_get_timestamp_s: *const c_void,
    pub duckdb_get_timestamp_ms: *const c_void,
    pub duckdb_get_timestamp_ns: *const c_void,
    pub duckdb_append_value: *const c_void,
    pub duckdb_get_profiling_info: *const c_void,
    pub duckdb_profiling_info_get_value: *const c_void,
    pub duckdb_appender_begin_row: *const c_void,
    pub duckdb_appender_end_row: *const c_void,
    pub duckdb_append_default: *const c_void,
    pub duckdb_append_bool: *const c_void,
    pub duckdb_append_int8: *const c_void,
    pub duckdb_append_int16: *const c_void,
    pub duckdb_append_int32: *const c_void,
    pub duckdb_append_int64: *const c_void,
    pub duckdb_append_hugeint: *const c_void,
    pub duckdb_append_uint8: *const c_void,
    pub duckdb_append_uint16: *const c_void,
    pub duckdb_append_uint32: *const c_void,
    pub duckdb_append_uint64: *const c_void,
    pub duckdb_append_uhugeint: *const c_void,
    pub duckdb_append_float: *const c_void,
    pub duckdb_append_double: *const c_void,
    pub duckdb_append_date: *const c_void,
    pub duckdb_append_time: *const c_void,
    pub duckdb_append_timestamp: *const c_void,
    pub duckdb_append_interval: *const c_void,
    pub duckdb_append_varchar: *const c_void,
    pub duckdb_append_varchar_length: *const c_void,
    pub duckdb_append_blob: *const c_void,
    pub duckdb_append_null: *const c_void,
    pub duckdb_row_count: *const c_void,
    pub duckdb_column_data: *const c_void,
    pub duckdb_nullmask_data: *const c_void,
    pub duckdb_result_get_chunk: *const c_void,
    pub duckdb_result_is_streaming: *const c_void,
    pub duckdb_result_chunk_count: *const c_void,
    pub duckdb_value_boolean: *const c_void,
    pub duckdb_value_int8: *const c_void,
    pub duckdb_value_int16: *const c_void,
    pub duckdb_value_int32: *const c_void,
    pub duckdb_value_int64: *const c_void,
    pub duckdb_value_hugeint: *const c_void,
    pub duckdb_value_uhugeint: *const c_void,
    pub duckdb_value_decimal: *const c_void,
    pub duckdb_value_uint8: *const c_void,
    pub duckdb_value_uint16: *const c_void,
    pub duckdb_value_uint32: *const c_void,
    pub duckdb_value_uint64: *const c_void,
    pub duckdb_value_float: *const c_void,
    pub duckdb_value_double: *const c_void,
    pub duckdb_value_date: *const c_void,
    pub duckdb_value_time: *const c_void,
    pub duckdb_value_timestamp: *const c_void,
    pub duckdb_value_interval: *const c_void,
    pub duckdb_value_varchar: *const c_void,
    pub duckdb_value_string: *const c_void,
    pub duckdb_value_varchar_internal: *const c_void,
    pub duckdb_value_string_internal: *const c_void,
    pub duckdb_value_blob: *const c_void,
    pub duckdb_value_is_null: *const c_void,
    pub duckdb_execute_prepared_streaming: *const c_void,
    pub duckdb_pending_prepared_streaming: *const c_void,
    pub duckdb_query_arrow: *const c_void,
    pub duckdb_query_arrow_schema: *const c_void,
    pub duckdb_prepared_arrow_schema: *const c_void,
    pub duckdb_result_arrow_array: *const c_void,
    pub duckdb_query_arrow_array: *const c_void,
    pub duckdb_arrow_column_count: *const c_void,
    pub duckdb_arrow_row_count: *const c_void,
    pub duckdb_arrow_rows_changed: *const c_void,
    pub duckdb_query_arrow_error: *const c_void,
    pub duckdb_destroy_arrow: *const c_void,
    pub duckdb_destroy_arrow_stream: *const c_void,
    pub duckdb_execute_prepared_arrow: *const c_void,
    pub duckdb_arrow_scan: *const c_void,
    pub duckdb_arrow_array_scan: *const c_void,
    pub duckdb_stream_fetch_chunk: *const c_void,
    pub duckdb_create_instance_cache: *const c_void,
    pub duckdb_get_or_create_from_cache: *const c_void,
    pub duckdb_destroy_instance_cache: *const c_void,
    pub duckdb_append_default_to_chunk: *const c_void,
}

#[inline(always)]
pub unsafe fn duckdb_connect(database: duckdb_database, out_connection: *mut duckdb_connection) -> duckdb_state {
    ((*get_api()).duckdb_connect.expect("DuckDB API function duckdb_connect is null"))(database, out_connection)
}

#[inline(always)]
pub unsafe fn duckdb_disconnect(connection: *mut duckdb_connection) {
    ((*get_api()).duckdb_disconnect.expect("DuckDB API function duckdb_disconnect is null"))(connection)
}

#[inline(always)]
pub unsafe fn duckdb_free(ptr: *mut c_void) {
    ((*get_api()).duckdb_free.expect("DuckDB API function duckdb_free is null"))(ptr)
}

#[inline(always)]
pub unsafe fn duckdb_destroy_value(val: *mut duckdb_value) {
    ((*get_api()).duckdb_destroy_value.expect("DuckDB API function duckdb_destroy_value is null"))(val)
}

#[inline(always)]
pub unsafe fn duckdb_get_varchar(val: duckdb_value) -> *mut c_char {
    ((*get_api()).duckdb_get_varchar.expect("DuckDB API function duckdb_get_varchar is null"))(val)
}

#[inline(always)]
pub unsafe fn duckdb_create_logical_type(type_: duckdb_type) -> duckdb_logical_type {
    ((*get_api()).duckdb_create_logical_type.expect("DuckDB API function duckdb_create_logical_type is null"))(type_)
}

#[inline(always)]
pub unsafe fn duckdb_destroy_logical_type(type_: *mut duckdb_logical_type) {
    ((*get_api()).duckdb_destroy_logical_type.expect("DuckDB API function duckdb_destroy_logical_type is null"))(type_)
}

#[inline(always)]
pub unsafe fn duckdb_data_chunk_get_vector(chunk: duckdb_data_chunk, col_idx: idx_t) -> duckdb_vector {
    ((*get_api()).duckdb_data_chunk_get_vector.expect("DuckDB API function duckdb_data_chunk_get_vector is null"))(chunk, col_idx)
}

#[inline(always)]
pub unsafe fn duckdb_data_chunk_set_size(chunk: duckdb_data_chunk, size: idx_t) {
    ((*get_api()).duckdb_data_chunk_set_size.expect("DuckDB API function duckdb_data_chunk_set_size is null"))(chunk, size)
}

#[inline(always)]
pub unsafe fn duckdb_vector_get_data(vector: duckdb_vector) -> *mut c_void {
    ((*get_api()).duckdb_vector_get_data.expect("DuckDB API function duckdb_vector_get_data is null"))(vector)
}

#[inline(always)]
pub unsafe fn duckdb_vector_get_validity(vector: duckdb_vector) -> *mut u64 {
    ((*get_api()).duckdb_vector_get_validity.expect("DuckDB API function duckdb_vector_get_validity is null"))(vector)
}

#[inline(always)]
pub unsafe fn duckdb_vector_ensure_validity_writable(vector: duckdb_vector) {
    ((*get_api()).duckdb_vector_ensure_validity_writable.expect("DuckDB API function duckdb_vector_ensure_validity_writable is null"))(vector)
}

#[inline(always)]
pub unsafe fn duckdb_vector_assign_string_element_len(vector: duckdb_vector, index: idx_t, str_: *const c_char, str_len: idx_t) {
    ((*get_api()).duckdb_vector_assign_string_element_len.expect("DuckDB API function duckdb_vector_assign_string_element_len is null"))(vector, index, str_, str_len)
}

#[inline(always)]
pub unsafe fn duckdb_create_table_function() -> duckdb_table_function {
    ((*get_api()).duckdb_create_table_function.expect("DuckDB API function duckdb_create_table_function is null"))()
}

#[inline(always)]
pub unsafe fn duckdb_destroy_table_function(table_function: *mut duckdb_table_function) {
    ((*get_api()).duckdb_destroy_table_function.expect("DuckDB API function duckdb_destroy_table_function is null"))(table_function)
}

#[inline(always)]
pub unsafe fn duckdb_table_function_set_name(table_function: duckdb_table_function, name: *const c_char) {
    ((*get_api()).duckdb_table_function_set_name.expect("DuckDB API function duckdb_table_function_set_name is null"))(table_function, name)
}

#[inline(always)]
pub unsafe fn duckdb_table_function_add_parameter(table_function: duckdb_table_function, type_: duckdb_logical_type) {
    ((*get_api()).duckdb_table_function_add_parameter.expect("DuckDB API function duckdb_table_function_add_parameter is null"))(table_function, type_)
}

#[inline(always)]
pub unsafe fn duckdb_table_function_add_named_parameter(table_function: duckdb_table_function, name: *const c_char, type_: duckdb_logical_type) {
    ((*get_api()).duckdb_table_function_add_named_parameter.expect("DuckDB API function duckdb_table_function_add_named_parameter is null"))(table_function, name, type_)
}

#[inline(always)]
pub unsafe fn duckdb_table_function_set_bind(table_function: duckdb_table_function, bind: duckdb_table_function_bind_t) {
    ((*get_api()).duckdb_table_function_set_bind.expect("DuckDB API function duckdb_table_function_set_bind is null"))(table_function, bind)
}

#[inline(always)]
pub unsafe fn duckdb_table_function_set_init(table_function: duckdb_table_function, init: duckdb_table_function_init_t) {
    ((*get_api()).duckdb_table_function_set_init.expect("DuckDB API function duckdb_table_function_set_init is null"))(table_function, init)
}

#[inline(always)]
pub unsafe fn duckdb_table_function_set_local_init(table_function: duckdb_table_function, init: duckdb_table_function_init_t) {
    ((*get_api()).duckdb_table_function_set_local_init.expect("DuckDB API function duckdb_table_function_set_local_init is null"))(table_function, init)
}

#[inline(always)]
pub unsafe fn duckdb_table_function_set_function(table_function: duckdb_table_function, function: duckdb_table_function_t) {
    ((*get_api()).duckdb_table_function_set_function.expect("DuckDB API function duckdb_table_function_set_function is null"))(table_function, function)
}

#[inline(always)]
pub unsafe fn duckdb_register_table_function(con: duckdb_connection, function: duckdb_table_function) -> duckdb_state {
    ((*get_api()).duckdb_register_table_function.expect("DuckDB API function duckdb_register_table_function is null"))(con, function)
}

#[inline(always)]
pub unsafe fn duckdb_bind_add_result_column(info: duckdb_bind_info, name: *const c_char, type_: duckdb_logical_type) {
    ((*get_api()).duckdb_bind_add_result_column.expect("DuckDB API function duckdb_bind_add_result_column is null"))(info, name, type_)
}

#[inline(always)]
pub unsafe fn duckdb_bind_get_parameter_count(info: duckdb_bind_info) -> idx_t {
    ((*get_api()).duckdb_bind_get_parameter_count.expect("DuckDB API function duckdb_bind_get_parameter_count is null"))(info)
}

#[inline(always)]
pub unsafe fn duckdb_bind_get_parameter(info: duckdb_bind_info, index: idx_t) -> duckdb_value {
    ((*get_api()).duckdb_bind_get_parameter.expect("DuckDB API function duckdb_bind_get_parameter is null"))(info, index)
}

#[inline(always)]
pub unsafe fn duckdb_bind_get_named_parameter(info: duckdb_bind_info, name: *const c_char) -> duckdb_value {
    ((*get_api()).duckdb_bind_get_named_parameter.expect("DuckDB API function duckdb_bind_get_named_parameter is null"))(info, name)
}

#[inline(always)]
pub unsafe fn duckdb_bind_set_bind_data(info: duckdb_bind_info, extra_data: *mut c_void, destroy: Option<unsafe extern "C" fn(*mut c_void)>) {
    ((*get_api()).duckdb_bind_set_bind_data.expect("DuckDB API function duckdb_bind_set_bind_data is null"))(info, extra_data, destroy)
}

#[inline(always)]
pub unsafe fn duckdb_bind_set_error(info: duckdb_bind_info, error: *const c_char) {
    ((*get_api()).duckdb_bind_set_error.expect("DuckDB API function duckdb_bind_set_error is null"))(info, error)
}

#[inline(always)]
pub unsafe fn duckdb_init_get_bind_data(info: duckdb_init_info) -> *mut c_void {
    ((*get_api()).duckdb_init_get_bind_data.expect("DuckDB API function duckdb_init_get_bind_data is null"))(info)
}

#[inline(always)]
pub unsafe fn duckdb_init_set_init_data(info: duckdb_init_info, extra_data: *mut c_void, destroy: Option<unsafe extern "C" fn(*mut c_void)>) {
    ((*get_api()).duckdb_init_set_init_data.expect("DuckDB API function duckdb_init_set_init_data is null"))(info, extra_data, destroy)
}

#[inline(always)]
pub unsafe fn duckdb_init_set_max_threads(info: duckdb_init_info, max_threads: idx_t) {
    ((*get_api()).duckdb_init_set_max_threads.expect("DuckDB API function duckdb_init_set_max_threads is null"))(info, max_threads)
}

#[inline(always)]
pub unsafe fn duckdb_init_set_error(info: duckdb_init_info, error: *const c_char) {
    ((*get_api()).duckdb_init_set_error.expect("DuckDB API function duckdb_init_set_error is null"))(info, error)
}

#[inline(always)]
pub unsafe fn duckdb_function_get_bind_data(info: duckdb_function_info) -> *mut c_void {
    ((*get_api()).duckdb_function_get_bind_data.expect("DuckDB API function duckdb_function_get_bind_data is null"))(info)
}

#[inline(always)]
pub unsafe fn duckdb_function_get_init_data(info: duckdb_function_info) -> *mut c_void {
    ((*get_api()).duckdb_function_get_init_data.expect("DuckDB API function duckdb_function_get_init_data is null"))(info)
}

#[inline(always)]
pub unsafe fn duckdb_function_get_local_init_data(info: duckdb_function_info) -> *mut c_void {
    ((*get_api()).duckdb_function_get_local_init_data.expect("DuckDB API function duckdb_function_get_local_init_data is null"))(info)
}

#[inline(always)]
pub unsafe fn duckdb_function_set_error(info: duckdb_function_info, error: *const c_char) {
    ((*get_api()).duckdb_function_set_error.expect("DuckDB API function duckdb_function_set_error is null"))(info, error)
}

#[inline(always)]
pub unsafe fn duckdb_validity_set_row_invalid(validity: *mut u64, row: idx_t) {
    if !validity.is_null() {
        let entry_idx = (row / 64) as usize;
        let bit_idx = (row % 64) as usize;
        *validity.add(entry_idx) &= !(1u64 << bit_idx);
    }
}

#[inline(always)]
pub unsafe fn duckdb_validity_set_row_valid(validity: *mut u64, row: idx_t) {
    if !validity.is_null() {
        let entry_idx = (row / 64) as usize;
        let bit_idx = (row % 64) as usize;
        *validity.add(entry_idx) |= 1u64 << bit_idx;
    }
}

#[inline(always)]
pub unsafe fn duckdb_validity_row_is_valid(validity: *const u64, row: idx_t) -> bool {
    if validity.is_null() {
        true
    } else {
        let entry_idx = (row / 64) as usize;
        let bit_idx = (row % 64) as usize;
        (*validity.add(entry_idx) & (1u64 << bit_idx)) != 0
    }
}
