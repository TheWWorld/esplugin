use std::path::Path;
use std::{f32, mem, panic, ptr};

use libc::{c_char, c_float, size_t, uint32_t, uint8_t};

use constants::*;
use esplugin::Plugin;
use helpers::*;

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_new(
    plugin_ptr_ptr: *mut *mut Plugin,
    game_id: uint32_t,
    path: *const c_char,
) -> uint32_t {
    panic::catch_unwind(|| {
        let rust_path = match to_str(path) {
            Ok(x) => Path::new(x),
            Err(x) => return x,
        };

        let mapped_game_id = match map_game_id(game_id) {
            Ok(x) => x,
            Err(x) => return x,
        };

        let plugin = Plugin::new(mapped_game_id, rust_path);
        *plugin_ptr_ptr = Box::into_raw(Box::new(plugin));

        ESP_OK
    }).unwrap_or(ESP_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_free(plugin_ptr: *mut Plugin) {
    if !plugin_ptr.is_null() {
        Box::from_raw(plugin_ptr);
    }
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_parse(
    plugin_ptr: *mut Plugin,
    load_header_only: bool,
) -> uint32_t {
    panic::catch_unwind(|| {
        if plugin_ptr.is_null() {
            ESP_ERROR_NULL_POINTER
        } else {
            let plugin = &mut *plugin_ptr;
            match plugin.parse_file(load_header_only) {
                Ok(_) => ESP_OK,
                Err(_) => ESP_ERROR_PARSE_ERROR,
            }
        }
    }).unwrap_or(ESP_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_filename(
    plugin_ptr: *const Plugin,
    filename: *mut *mut c_char,
) -> uint32_t {
    panic::catch_unwind(|| {
        if filename.is_null() || plugin_ptr.is_null() {
            ESP_ERROR_NULL_POINTER
        } else {
            let plugin = &*plugin_ptr;

            let c_string: *mut c_char = match plugin.filename().map(|s| to_c_string(&s)) {
                None => ptr::null_mut(),
                Some(x) => x,
            };

            *filename = c_string;

            ESP_OK
        }
    }).unwrap_or(ESP_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_masters(
    plugin_ptr: *const Plugin,
    plugin_masters: *mut *mut *mut c_char,
    plugin_masters_size: *mut uint8_t,
) -> uint32_t {
    panic::catch_unwind(|| {
        if plugin_masters.is_null() || plugin_ptr.is_null() {
            ESP_ERROR_NULL_POINTER
        } else {
            let plugin = &*plugin_ptr;

            let mut c_string_vec: Vec<*mut c_char> = match plugin.masters() {
                Ok(x) => x.iter().map(|m| to_c_string(m)).collect(),
                Err(_) => return ESP_ERROR_NOT_UTF8,
            };

            c_string_vec.shrink_to_fit();

            *plugin_masters = c_string_vec.as_mut_ptr();
            *plugin_masters_size = c_string_vec.len() as u8;

            mem::forget(c_string_vec);

            ESP_OK
        }
    }).unwrap_or(ESP_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_is_master(
    plugin_ptr: *const Plugin,
    is_master: *mut bool,
) -> uint32_t {
    panic::catch_unwind(|| {
        if plugin_ptr.is_null() || is_master.is_null() {
            ESP_ERROR_NULL_POINTER
        } else {
            let plugin = &*plugin_ptr;

            *is_master = plugin.is_master_file();

            ESP_OK
        }
    }).unwrap_or(ESP_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_is_light_master(
    plugin_ptr: *const Plugin,
    is_light_master: *mut bool,
) -> uint32_t {
    panic::catch_unwind(|| {
        if plugin_ptr.is_null() || is_light_master.is_null() {
            ESP_ERROR_NULL_POINTER
        } else {
            let plugin = &*plugin_ptr;

            *is_light_master = plugin.is_light_master_file();

            ESP_OK
        }
    }).unwrap_or(ESP_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_is_valid(
    game_id: uint32_t,
    path: *const c_char,
    load_header_only: bool,
    is_valid: *mut bool,
) -> uint32_t {
    panic::catch_unwind(|| {
        if path.is_null() || is_valid.is_null() {
            ESP_ERROR_NULL_POINTER
        } else {
            let rust_path = match to_str(path) {
                Ok(x) => Path::new(x),
                Err(x) => return x,
            };

            let mapped_game_id = match map_game_id(game_id) {
                Ok(x) => x,
                Err(x) => return x,
            };

            *is_valid = Plugin::is_valid(mapped_game_id, rust_path, load_header_only);

            ESP_OK
        }
    }).unwrap_or(ESP_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_description(
    plugin_ptr: *const Plugin,
    description: *mut *mut c_char,
) -> uint32_t {
    panic::catch_unwind(|| {
        if description.is_null() || plugin_ptr.is_null() {
            ESP_ERROR_NULL_POINTER
        } else {
            let plugin = &*plugin_ptr;

            let description_option = match plugin.description() {
                Ok(x) => x.map(|d| to_c_string(&d)),
                Err(_) => return ESP_ERROR_NOT_UTF8,
            };

            let c_string = match description_option {
                None => ptr::null_mut(),
                Some(x) => x,
            };

            *description = c_string;

            ESP_OK
        }
    }).unwrap_or(ESP_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_header_version(
    plugin_ptr: *const Plugin,
    header_version: *mut c_float,
) -> uint32_t {
    panic::catch_unwind(|| {
        if plugin_ptr.is_null() || header_version.is_null() {
            ESP_ERROR_NULL_POINTER
        } else {
            let plugin = &*plugin_ptr;

            match plugin.header_version() {
                Some(i) => *header_version = i,
                None => *header_version = f32::NAN,
            }

            ESP_OK
        }
    }).unwrap_or(ESP_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_is_empty(
    plugin_ptr: *const Plugin,
    is_empty: *mut bool,
) -> uint32_t {
    panic::catch_unwind(|| {
        if plugin_ptr.is_null() || is_empty.is_null() {
            ESP_ERROR_NULL_POINTER
        } else {
            let plugin = &*plugin_ptr;

            *is_empty = plugin.record_and_group_count().unwrap_or(0) == 0;

            ESP_OK
        }
    }).unwrap_or(ESP_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_count_override_records(
    plugin_ptr: *const Plugin,
    count: *mut size_t,
) -> uint32_t {
    panic::catch_unwind(|| {
        if plugin_ptr.is_null() || count.is_null() {
            ESP_ERROR_NULL_POINTER
        } else {
            let plugin = &*plugin_ptr;

            *count = plugin.count_override_records();

            ESP_OK
        }
    }).unwrap_or(ESP_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_do_records_overlap(
    plugin_ptr: *const Plugin,
    other_plugin_ptr: *const Plugin,
    overlap: *mut bool,
) -> uint32_t {
    panic::catch_unwind(|| {
        if plugin_ptr.is_null() || other_plugin_ptr.is_null() || overlap.is_null() {
            ESP_ERROR_NULL_POINTER
        } else {
            let plugin = &*plugin_ptr;
            let other_plugin = &*other_plugin_ptr;

            *overlap = plugin.overlaps_with(&other_plugin);

            ESP_OK
        }
    }).unwrap_or(ESP_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn esp_plugin_is_valid_as_light_master(
    plugin_ptr: *const Plugin,
    is_valid: *mut bool,
) -> uint32_t {
    panic::catch_unwind(|| {
        if plugin_ptr.is_null() || is_valid.is_null() {
            ESP_ERROR_NULL_POINTER
        } else {
            let plugin = &*plugin_ptr;

            *is_valid = plugin.is_valid_as_light_master();

            ESP_OK
        }
    }).unwrap_or(ESP_ERROR_PANICKED)
}
