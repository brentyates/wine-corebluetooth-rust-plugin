use crate::callbacks::invoke_scan_result;
use crate::state::STATE;
use windows::Devices::Bluetooth::Advertisement::{
    BluetoothLEAdvertisementReceivedEventArgs,
    BluetoothLEAdvertisementWatcher,
};
use windows::Foundation::TypedEventHandler;

pub fn start_scan() -> Result<(), windows::core::Error> {
    let mut state = STATE.lock().unwrap();

    if state.watcher_handler.is_some() {
        return Ok(());
    }

    let watcher = state.watcher.as_ref().ok_or_else(|| {
        windows::core::Error::new(
            windows::core::HRESULT(-1),
            "Watcher not initialized",
        )
    })?;

    let handler = TypedEventHandler::new(
        |_sender: &Option<BluetoothLEAdvertisementWatcher>,
         args: &Option<BluetoothLEAdvertisementReceivedEventArgs>| {
            if let Some(args) = args {
                handle_advertisement_received(args);
            }
            Ok(())
        },
    );

    watcher.Received(&handler)?;
    watcher.Start()?;

    state.watcher_handler = Some(send_wrapper::SendWrapper::new(handler));

    Ok(())
}

pub fn stop_scan() {
    let mut state = STATE.lock().unwrap();
    if let Some(ref watcher) = state.watcher {
        let _ = watcher.Stop();
    }
    state.watcher_handler = None;
}

fn handle_advertisement_received(args: &BluetoothLEAdvertisementReceivedEventArgs) {
    let address = args.BluetoothAddress().unwrap_or(0);
    let rssi = args.RawSignalStrengthInDBm().unwrap_or(-100) as i32;

    let name = if let Ok(adv) = args.Advertisement() {
        adv.LocalName()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default()
    } else {
        String::new()
    };

    if name.is_empty() {
        return;
    }

    {
        let mut state = STATE.lock().unwrap();
        state.discovered_devices.insert(name.clone(), address);
    }

    let addr_str = format!("{:012X}", address);

    let handler = {
        let state = STATE.lock().unwrap();
        state.scan_result_handler
    };

    invoke_scan_result(handler, &addr_str, &name, rssi);
}

pub fn get_address_for_name(name: &str) -> Option<u64> {
    let state = STATE.lock().unwrap();
    state.discovered_devices.get(name).copied()
}
