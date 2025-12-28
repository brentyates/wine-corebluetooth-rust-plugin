use crate::callbacks::invoke_connection_state;
use crate::state::STATE;
use crate::watcher::get_address_for_name;
use windows::Devices::Bluetooth::BluetoothLEDevice;
use windows::Devices::Bluetooth::BluetoothConnectionStatus;

pub fn connect_by_name(name: &str) -> Result<(), windows::core::Error> {
    let address = match get_address_for_name(name) {
        Some(addr) => addr,
        None => {
            let state = STATE.lock().unwrap();
            let mut found_addr: Option<u64> = None;
            for (stored_name, &addr) in state.discovered_devices.iter() {
                if name.starts_with(stored_name) {
                    found_addr = Some(addr);
                    break;
                }
            }
            drop(state);
            match found_addr {
                Some(addr) => addr,
                None => {
                    return Err(windows::core::Error::new(
                        windows::core::HRESULT(-1),
                        "Device not found in scan results",
                    ));
                }
            }
        }
    };

    notify_connection_state(1);

    let async_op = BluetoothLEDevice::FromBluetoothAddressAsync(address)?;
    let device = async_op.get()?;

    {
        let mut state = STATE.lock().unwrap();
        state.device = Some(device.clone());
        state.device_name = Some(name.to_string());
        state.device_address = address;
        state.is_connected = true;
    }

    setup_connection_status_handler(&device)?;

    notify_connection_state(2);

    Ok(())
}

fn setup_connection_status_handler(device: &BluetoothLEDevice) -> Result<(), windows::core::Error> {
    use windows::Foundation::TypedEventHandler;

    {
        let state = STATE.lock().unwrap();
        if state.connection_status_handler.is_some() {
            return Ok(());
        }
    }

    let handler = TypedEventHandler::new(
        |_sender: &Option<BluetoothLEDevice>, _args: &Option<windows::core::IInspectable>| {
            let state = STATE.lock().unwrap();
            if let Some(ref dev) = state.device {
                if let Ok(status) = dev.ConnectionStatus() {
                    let state_val = match status {
                        BluetoothConnectionStatus::Disconnected => 0,
                        BluetoothConnectionStatus::Connected => 2,
                        _ => 1,
                    };
                    drop(state);
                    notify_connection_state(state_val);
                }
            }
            Ok(())
        },
    );

    device.ConnectionStatusChanged(&handler)?;

    {
        let mut state = STATE.lock().unwrap();
        state.connection_status_handler = Some(send_wrapper::SendWrapper::new(handler));
    }

    Ok(())
}

pub fn disconnect() {
    let handler;
    {
        let mut state = STATE.lock().unwrap();
        handler = state.connection_state_handler;
        state.device = None;
        state.is_connected = false;
        state.services.clear();
        state.characteristics.clear();
        state.notification_handlers.clear();
        state.connection_status_handler = None;
    }

    invoke_connection_state(handler, 0);
}

fn notify_connection_state(state: i32) {
    let handler = {
        let s = STATE.lock().unwrap();
        s.connection_state_handler
    };
    invoke_connection_state(handler, state);
}
