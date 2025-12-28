type WideChar = u16;

pub type ScanResultHandler = Option<
    unsafe extern "C" fn(
        addr: *const WideChar,
        addr_len: i32,
        name: *const WideChar,
        name_len: i32,
        rssi: i32,
    ),
>;

pub type ConnectionStateHandler = Option<unsafe extern "C" fn(state: i32)>;

pub type ServiceDiscoveredHandler = Option<
    unsafe extern "C" fn(
        service_type: i32,
        uuid: *const WideChar,
        uuid_len: i32,
    ),
>;

pub type ValueChangedHandler = Option<
    unsafe extern "C" fn(
        uuid: *const WideChar,
        uuid_len: i32,
        value: *const WideChar,
        value_len: i32,
    ),
>;

pub type ScanStateChangedHandler = Option<unsafe extern "C" fn(state: i32)>;

pub type NotificationHandler = Option<
    unsafe extern "C" fn(
        uuid: *const WideChar,
        uuid_len: i32,
        value: *const u8,
        value_len: i32,
    ),
>;

pub fn invoke_scan_result(
    handler: ScanResultHandler,
    addr: &str,
    name: &str,
    rssi: i32,
) {
    if let Some(cb) = handler {
        let mut addr_utf16: Vec<u16> = addr.encode_utf16().collect();
        addr_utf16.push(0);
        let mut name_utf16: Vec<u16> = name.encode_utf16().collect();
        name_utf16.push(0);
        unsafe {
            cb(
                addr_utf16.as_ptr(),
                (addr_utf16.len() - 1) as i32,
                name_utf16.as_ptr(),
                (name_utf16.len() - 1) as i32,
                rssi,
            );
        }
    }
}

pub fn invoke_connection_state(handler: ConnectionStateHandler, state: i32) {
    if let Some(cb) = handler {
        unsafe {
            cb(state);
        }
    }
}

pub fn invoke_service_discovered(
    handler: ServiceDiscoveredHandler,
    service_type: i32,
    uuid: &str,
) {
    if let Some(cb) = handler {
        let mut uuid_utf16: Vec<u16> = uuid.encode_utf16().collect();
        uuid_utf16.push(0);
        unsafe {
            cb(service_type, uuid_utf16.as_ptr(), (uuid_utf16.len() - 1) as i32);
        }
    }
}

pub fn invoke_value_changed(
    handler: ValueChangedHandler,
    uuid: &str,
    value: &[u8],
) {
    if let Some(cb) = handler {
        let mut uuid_utf16: Vec<u16> = uuid.encode_utf16().collect();
        uuid_utf16.push(0);
        let value_hex: String = value.iter().map(|b| format!("{:02x}", b)).collect();
        let mut value_utf16: Vec<u16> = value_hex.encode_utf16().collect();
        value_utf16.push(0);
        unsafe {
            cb(
                uuid_utf16.as_ptr(),
                (uuid_utf16.len() - 1) as i32,
                value_utf16.as_ptr(),
                (value_utf16.len() - 1) as i32,
            );
        }
    }
}
