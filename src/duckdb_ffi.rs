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

extern "C" {
    pub fn duckdb_connect(database: duckdb_database, out_connection: *mut duckdb_connection) -> duckdb_state;
    pub fn duckdb_disconnect(connection: *mut duckdb_connection);

    pub fn duckdb_create_logical_type(type_: duckdb_type) -> duckdb_logical_type;
    pub fn duckdb_destroy_logical_type(type_: *mut duckdb_logical_type);

    pub fn duckdb_create_table_function() -> duckdb_table_function;
    pub fn duckdb_destroy_table_function(table_function: *mut duckdb_table_function);
    pub fn duckdb_table_function_set_name(table_function: duckdb_table_function, name: *const c_char);
    pub fn duckdb_table_function_add_parameter(table_function: duckdb_table_function, type_: duckdb_logical_type);
    pub fn duckdb_table_function_add_named_parameter(
        table_function: duckdb_table_function,
        name: *const c_char,
        type_: duckdb_logical_type,
    );
    pub fn duckdb_table_function_set_bind(table_function: duckdb_table_function, bind: duckdb_table_function_bind_t);
    pub fn duckdb_table_function_set_init(table_function: duckdb_table_function, init: duckdb_table_function_init_t);
    pub fn duckdb_table_function_set_function(table_function: duckdb_table_function, function: duckdb_table_function_t);

    pub fn duckdb_register_table_function(con: duckdb_connection, function: duckdb_table_function) -> duckdb_state;

    pub fn duckdb_bind_get_parameter_count(info: duckdb_bind_info) -> idx_t;
    pub fn duckdb_bind_get_parameter(info: duckdb_bind_info, index: idx_t) -> duckdb_value;
    pub fn duckdb_bind_get_named_parameter(info: duckdb_bind_info, name: *const c_char) -> duckdb_value;
    pub fn duckdb_bind_add_result_column(info: duckdb_bind_info, name: *const c_char, type_: duckdb_logical_type);
    pub fn duckdb_bind_set_bind_data(
        info: duckdb_bind_info,
        extra_data: *mut c_void,
        destroy: Option<unsafe extern "C" fn(*mut c_void)>,
    );
    pub fn duckdb_bind_set_error(info: duckdb_bind_info, error: *const c_char);

    pub fn duckdb_init_get_bind_data(info: duckdb_init_info) -> *mut c_void;
    pub fn duckdb_init_set_init_data(
        info: duckdb_init_info,
        extra_data: *mut c_void,
        destroy: Option<unsafe extern "C" fn(*mut c_void)>,
    );
    pub fn duckdb_init_set_error(info: duckdb_init_info, error: *const c_char);

    pub fn duckdb_function_get_bind_data(info: duckdb_function_info) -> *mut c_void;
    pub fn duckdb_function_get_init_data(info: duckdb_function_info) -> *mut c_void;
    pub fn duckdb_function_set_error(info: duckdb_function_info, error: *const c_char);

    pub fn duckdb_data_chunk_get_vector(chunk: duckdb_data_chunk, col_idx: idx_t) -> duckdb_vector;
    pub fn duckdb_data_chunk_set_size(chunk: duckdb_data_chunk, size: idx_t);

    pub fn duckdb_vector_get_data(vector: duckdb_vector) -> *mut c_void;
    pub fn duckdb_vector_ensure_validity_writable(vector: duckdb_vector);
    pub fn duckdb_vector_get_validity(vector: duckdb_vector) -> *mut u64;
    pub fn duckdb_validity_set_row_validity(validity: *mut u64, row: idx_t, valid: bool);
    pub fn duckdb_validity_set_row_invalid(validity: *mut u64, row: idx_t);
    pub fn duckdb_validity_set_row_valid(validity: *mut u64, row: idx_t);
    pub fn duckdb_vector_assign_string_element_len(
        vector: duckdb_vector,
        index: idx_t,
        str_: *const c_char,
        str_len: idx_t,
    );

    pub fn duckdb_get_varchar(val: duckdb_value) -> *mut c_char;
    pub fn duckdb_destroy_value(val: *mut duckdb_value);
    pub fn duckdb_free(ptr: *mut c_void);
}
