#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

mod callbacks;
mod state;

#[cfg(windows)]
mod watcher;
#[cfg(windows)]
mod device;
#[cfg(windows)]
mod gatt;

use callbacks::*;
use state::STATE;

type WideChar = u16;

#[no_mangle]
pub extern "C" fn _open() -> i32 {
    #[cfg(windows)]
    {
        match state::initialize() {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
    #[cfg(not(windows))]
    {
        -1
    }
}

#[no_mangle]
pub extern "C" fn _close() -> i32 {
    state::cleanup();
    0
}

#[no_mangle]
pub extern "C" fn _release() -> i32 {
    state::cleanup();
    0
}

#[no_mangle]
pub extern "C" fn _startScan(_enable_filter: i32) -> i32 {
    #[cfg(windows)]
    {
        match watcher::start_scan() {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
    #[cfg(not(windows))]
    {
        -1
    }
}

#[no_mangle]
pub extern "C" fn _stopScan() -> i32 {
    #[cfg(windows)]
    {
        watcher::stop_scan();
    }
    0
}

#[no_mangle]
pub extern "C" fn _clearScan() -> i32 {
    let mut state = STATE.lock().unwrap();
    state.discovered_devices.clear();
    0
}

#[no_mangle]
pub unsafe extern "C" fn _connect(addr: *const WideChar, addr_len: i32) -> i32 {
    if addr.is_null() || addr_len <= 0 {
        return -1;
    }

    let name = unsafe {
        let slice = std::slice::from_raw_parts(addr, addr_len as usize);
        let actual_len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
        String::from_utf16_lossy(&slice[..actual_len])
    };

    #[cfg(windows)]
    {
        std::thread::spawn(move || {
            match device::connect_by_name(&name) {
                Ok(_) => {},
                Err(_) => {
                    device::disconnect();
                }
            }
        });
        0
    }
    #[cfg(not(windows))]
    {
        let _ = name;
        -1
    }
}

#[no_mangle]
pub extern "C" fn _disconnect() -> i32 {
    #[cfg(windows)]
    {
        device::disconnect();
    }
    0
}

#[no_mangle]
pub extern "C" fn _discoverServices() -> i32 {
    use std::sync::atomic::Ordering;
    use state::DISCOVERY_IN_PROGRESS;

    let was_already_running = DISCOVERY_IN_PROGRESS.swap(true, Ordering::SeqCst);

    if was_already_running {
        while DISCOVERY_IN_PROGRESS.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        return 0;
    }

    #[cfg(windows)]
    let result = {
        match gatt::discover_services() {
            Ok(_) => 0,
            Err(_) => -1,
        }
    };
    #[cfg(not(windows))]
    let result = -1;

    DISCOVERY_IN_PROGRESS.store(false, Ordering::SeqCst);
    result
}

#[no_mangle]
pub unsafe extern "C" fn _read(
    uuid: *const WideChar,
    uuid_len: i32,
    buffer: *mut u8,
    out_len: *mut i32,
) -> i32 {
    if uuid.is_null() || uuid_len <= 0 || out_len.is_null() {
        return -1;
    }

    let initial_buf_size = unsafe { *out_len } as usize;

    if buffer.is_null() && initial_buf_size > 0 {
        return -1;
    }

    let uuid_str = unsafe {
        let slice = std::slice::from_raw_parts(uuid, uuid_len as usize);
        String::from_utf16_lossy(slice)
    };

    #[cfg(windows)]
    {
        let uuid_lower = uuid_str.to_lowercase();

        if initial_buf_size == 0 {
            match gatt::read_characteristic(&uuid_lower) {
                Ok(data) => {
                    let data_len = data.len();
                    {
                        let mut state = STATE.lock().unwrap();
                        state.read_cache.insert(uuid_lower.clone(), data);
                        state.read_first_pass_buffer.insert(uuid_lower, buffer as usize);
                    }
                    unsafe { *out_len = data_len as i32; }
                    return data_len as i32;
                }
                Err(_) => return -1,
            }
        }

        let (cached_data, first_pass_buffer) = {
            let mut state = STATE.lock().unwrap();
            let data = state.read_cache.remove(&uuid_lower);
            let first_buf = state.read_first_pass_buffer.remove(&uuid_lower);
            (data, first_buf)
        };

        let data = match cached_data {
            Some(d) => d,
            None => {
                match gatt::read_characteristic(&uuid_lower) {
                    Ok(d) => d,
                    Err(_) => return -1,
                }
            }
        };

        let copy_len = data.len().min(initial_buf_size);

        let is_byte_ptr_ptr = first_pass_buffer.is_some()
            && first_pass_buffer.unwrap() == buffer as usize
            && !buffer.is_null();

        let actual_buffer = if is_byte_ptr_ptr {
            let inner = unsafe { *(buffer as *mut *mut u8) };
            if inner.is_null() {
                return -1;
            }
            inner
        } else {
            buffer
        };

        if actual_buffer.is_null() {
            return -1;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), actual_buffer, copy_len);
            *out_len = copy_len as i32;
        }

        copy_len as i32
    }
    #[cfg(not(windows))]
    {
        let _ = uuid_str;
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn _write(
    uuid: *const WideChar,
    uuid_len: i32,
    data: *const u8,
    data_len: i32,
) -> i32 {
    if uuid.is_null() || uuid_len <= 0 || data.is_null() || data_len <= 0 {
        return -1;
    }

    let uuid_str = unsafe {
        let slice = std::slice::from_raw_parts(uuid, uuid_len as usize);
        String::from_utf16_lossy(slice)
    };

    let data_vec = unsafe {
        std::slice::from_raw_parts(data, data_len as usize).to_vec()
    };

    #[cfg(windows)]
    {
        std::thread::spawn(move || {
            let _ = gatt::write_characteristic(&uuid_str, &data_vec);
        });
        0
    }
    #[cfg(not(windows))]
    {
        let _ = (uuid_str, data_vec);
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn _writeChunk(
    uuid: *const WideChar,
    uuid_len: i32,
    data: *const u8,
    data_len: i32,
    _chunk_size: i32,
) -> i32 {
    _write(uuid, uuid_len, data, data_len)
}

#[no_mangle]
pub unsafe extern "C" fn _enableNotification(uuid: *const WideChar, uuid_len: i32) -> i32 {
    if uuid.is_null() || uuid_len <= 0 {
        return -1;
    }

    let uuid_str = unsafe {
        let slice = std::slice::from_raw_parts(uuid, uuid_len as usize);
        String::from_utf16_lossy(slice)
    };

    #[cfg(windows)]
    {
        std::thread::spawn(move || {
            let _ = gatt::enable_notification(&uuid_str);
        });
        0
    }
    #[cfg(not(windows))]
    {
        let _ = uuid_str;
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn _disableNotification(uuid: *const WideChar, uuid_len: i32) -> i32 {
    if uuid.is_null() || uuid_len <= 0 {
        return -1;
    }

    let uuid_str = unsafe {
        let slice = std::slice::from_raw_parts(uuid, uuid_len as usize);
        String::from_utf16_lossy(slice)
    };

    #[cfg(windows)]
    {
        std::thread::spawn(move || {
            let _ = gatt::disable_notification(&uuid_str);
        });
        0
    }
    #[cfg(not(windows))]
    {
        let _ = uuid_str;
        -1
    }
}

#[no_mangle]
pub extern "C" fn _setScanResultHandler(handler: ScanResultHandler) -> i32 {
    let mut state = STATE.lock().unwrap();
    state.scan_result_handler = handler;
    0
}

#[no_mangle]
pub extern "C" fn _setScanStateChangedHandler(handler: ScanStateChangedHandler) -> i32 {
    let mut state = STATE.lock().unwrap();
    state.scan_state_changed_handler = handler;
    0
}

#[no_mangle]
pub extern "C" fn _setConnectionStateChangedHandler(handler: ConnectionStateHandler) -> i32 {
    let mut state = STATE.lock().unwrap();
    state.connection_state_handler = handler;
    0
}

#[no_mangle]
pub extern "C" fn _setServiceDiscoveredHandler(handler: ServiceDiscoveredHandler) -> i32 {
    let mut state = STATE.lock().unwrap();
    state.service_discovered_handler = handler;
    0
}

#[no_mangle]
pub extern "C" fn _setCharacteristicValueChangedHandler(handler: ValueChangedHandler) -> i32 {
    let mut state = STATE.lock().unwrap();
    state.value_changed_handler = handler;
    0
}

#[no_mangle]
pub extern "C" fn _setNotificationHandler(handler: NotificationHandler) -> i32 {
    let mut state = STATE.lock().unwrap();
    state.notification_handler = handler;
    0
}
