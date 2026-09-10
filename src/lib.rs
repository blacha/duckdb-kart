pub mod duckdb_ffi;
pub mod git;
pub mod kart;
pub mod msgpack;
pub mod table_function;

use duckdb_ffi::*;
use std::ffi::{c_char, CString};

#[no_mangle]
pub extern "C" fn kart_version() -> *const c_char {
    static VERSION: &[u8] = b"v0.1.0\0";
    VERSION.as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn kart_init_c_api(
    info: duckdb_extension_info,
    access: *mut duckdb_extension_access,
) -> bool {
    // 1. Initialize libgit2
    git::git_libgit2_init();

    if access.is_null() {
        return false;
    }

    let get_db = match (*access).get_database {
        Some(f) => f,
        None => return false,
    };

    let db_ptr = get_db(info);
    if db_ptr.is_null() {
        return false;
    }

    let mut con: duckdb_connection = std::ptr::null_mut();
    if duckdb_connect(*db_ptr, &mut con) != duckdb_state::DuckDBSuccess {
        return false;
    }

    let varchar_type = duckdb_create_logical_type(duckdb_type::DUCKDB_TYPE_VARCHAR);

    // 1. Register read_kart(repo_path VARCHAR, dataset VARCHAR)
    {
        let func = duckdb_create_table_function();
        let name = CString::new("read_kart").unwrap();
        duckdb_table_function_set_name(func, name.as_ptr());
        duckdb_table_function_add_parameter(func, varchar_type);
        duckdb_table_function_add_parameter(func, varchar_type);
        duckdb_table_function_set_bind(func, table_function::read_kart_bind);
        duckdb_table_function_set_init(func, table_function::read_kart_init);
        duckdb_table_function_set_local_init(func, table_function::read_kart_local_init);
        duckdb_table_function_set_function(func, table_function::read_kart_scan);

        let res = duckdb_register_table_function(con, func);
        let mut f = func;
        duckdb_destroy_table_function(&mut f);
        if res != duckdb_state::DuckDBSuccess {
            duckdb_disconnect(&mut con);
            return false;
        }
    }

    // 2. Register read_kart_dataset(path VARCHAR) [single argument]
    {
        let func = duckdb_create_table_function();
        let name = CString::new("read_kart_dataset").unwrap();
        duckdb_table_function_set_name(func, name.as_ptr());
        duckdb_table_function_add_parameter(func, varchar_type);
        duckdb_table_function_set_bind(func, table_function::read_kart_bind);
        duckdb_table_function_set_init(func, table_function::read_kart_init);
        duckdb_table_function_set_local_init(func, table_function::read_kart_local_init);
        duckdb_table_function_set_function(func, table_function::read_kart_scan);

        let res = duckdb_register_table_function(con, func);
        let mut f = func;
        duckdb_destroy_table_function(&mut f);
        if res != duckdb_state::DuckDBSuccess {
            duckdb_disconnect(&mut con);
            return false;
        }
    }

    // 3. Register read_kart_datasets(repo_path VARCHAR)
    {
        let func = duckdb_create_table_function();
        let name = CString::new("read_kart_datasets").unwrap();
        duckdb_table_function_set_name(func, name.as_ptr());
        duckdb_table_function_add_parameter(func, varchar_type);
        duckdb_table_function_set_bind(func, table_function::read_kart_datasets_bind);
        duckdb_table_function_set_init(func, table_function::read_kart_datasets_init);
        duckdb_table_function_set_function(func, table_function::read_kart_datasets_scan);

        let res = duckdb_register_table_function(con, func);
        let mut f = func;
        duckdb_destroy_table_function(&mut f);
        if res != duckdb_state::DuckDBSuccess {
            duckdb_disconnect(&mut con);
            return false;
        }
    }

    let mut vt = varchar_type;
    duckdb_destroy_logical_type(&mut vt);

    duckdb_disconnect(&mut con);
    true
}
