use crate::callbacks::{invoke_service_discovered, invoke_value_changed};
use crate::state::STATE;
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattClientCharacteristicConfigurationDescriptorValue,
    GattCommunicationStatus, GattDeviceService, GattValueChangedEventArgs,
};
use windows::Foundation::TypedEventHandler;
use windows::Storage::Streams::DataReader;
use windows::Security::Cryptography::CryptographicBuffer;

pub fn discover_services() -> Result<(), windows::core::Error> {
    let device = {
        let state = STATE.lock().unwrap();
        state.device.clone()
    };

    let device = match device {
        Some(d) => d,
        None => {
            return Err(windows::core::Error::new(windows::core::HRESULT(-1), "Not connected"));
        }
    };

    let async_op = device.GetGattServicesAsync()?;
    let result = async_op.get()?;

    if result.Status()? != GattCommunicationStatus::Success {
        return Err(windows::core::Error::new(
            windows::core::HRESULT(-1),
            "Failed to get services",
        ));
    }

    let services = result.Services()?;
    let service_handler;
    {
        let state = STATE.lock().unwrap();
        service_handler = state.service_discovered_handler;
    }

    for service in services {
        let guid = service.Uuid()?;
        let uuid_clean = format!("{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            guid.data1, guid.data2, guid.data3,
            guid.data4[0], guid.data4[1], guid.data4[2], guid.data4[3],
            guid.data4[4], guid.data4[5], guid.data4[6], guid.data4[7]);

        {
            let mut state = STATE.lock().unwrap();
            state.services.insert(uuid_clean.clone(), service.clone());
        }

        invoke_service_discovered(service_handler, 0, &uuid_clean);

        discover_characteristics(&service, service_handler)?;
    }

    Ok(())
}

fn discover_characteristics(
    service: &GattDeviceService,
    handler: crate::callbacks::ServiceDiscoveredHandler,
) -> Result<(), windows::core::Error> {
    let async_op = service.GetCharacteristicsAsync()?;
    let result = async_op.get()?;

    if result.Status()? != GattCommunicationStatus::Success {
        return Ok(());
    }

    let characteristics = result.Characteristics()?;

    for characteristic in characteristics {
        let guid = characteristic.Uuid()?;
        let uuid_clean = format!("{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            guid.data1, guid.data2, guid.data3,
            guid.data4[0], guid.data4[1], guid.data4[2], guid.data4[3],
            guid.data4[4], guid.data4[5], guid.data4[6], guid.data4[7]);

        {
            let mut state = STATE.lock().unwrap();
            state.characteristics.insert(uuid_clean.clone(), characteristic);
        }

        invoke_service_discovered(handler, 1, &uuid_clean);

        if uuid_clean.to_lowercase() == "86602102-6b7e-439a-bdd1-489a3213e9bb" {
            let _ = enable_notification(&uuid_clean);
        }
    }

    Ok(())
}

pub fn read_characteristic(uuid: &str) -> Result<Vec<u8>, windows::core::Error> {
    let uuid_lower = uuid.to_lowercase();

    let characteristic = {
        let state = STATE.lock().unwrap();
        state.characteristics.get(&uuid_lower).cloned()
    };

    let characteristic = characteristic.ok_or_else(|| {
        windows::core::Error::new(windows::core::HRESULT(-1), "Characteristic not found")
    })?;

    let async_op = characteristic.ReadValueAsync()?;
    let result = async_op.get()?;

    if result.Status()? != GattCommunicationStatus::Success {
        return Err(windows::core::Error::new(
            windows::core::HRESULT(-1),
            "Read failed",
        ));
    }

    let buffer = result.Value()?;
    let reader = DataReader::FromBuffer(&buffer)?;
    let len = reader.UnconsumedBufferLength()? as usize;

    let mut data = vec![0u8; len];
    reader.ReadBytes(&mut data)?;

    Ok(data)
}

pub fn write_characteristic(uuid: &str, data: &[u8]) -> Result<(), windows::core::Error> {
    let uuid_lower = uuid.to_lowercase();

    let characteristic = {
        let state = STATE.lock().unwrap();
        state.characteristics.get(&uuid_lower).cloned()
    };

    let characteristic = characteristic.ok_or_else(|| {
        windows::core::Error::new(windows::core::HRESULT(-1), "Characteristic not found")
    })?;

    let buffer = CryptographicBuffer::CreateFromByteArray(data)?;

    match characteristic.WriteValueAsync(&buffer) {
        Ok(_op) => {}
        Err(e) => return Err(e),
    };

    Ok(())
}

pub fn enable_notification(uuid: &str) -> Result<(), windows::core::Error> {
    let uuid_lower = uuid.to_lowercase();

    {
        let state = STATE.lock().unwrap();
        if state.notification_handlers.contains_key(&uuid_lower) {
            return Ok(());
        }
    }

    let characteristic = {
        let state = STATE.lock().unwrap();
        state.characteristics.get(&uuid_lower).cloned()
    };

    let characteristic = characteristic.ok_or_else(|| {
        windows::core::Error::new(windows::core::HRESULT(-1), "Characteristic not found")
    })?;

    let cccd_value = GattClientCharacteristicConfigurationDescriptorValue::Notify;
    let async_op = characteristic.WriteClientCharacteristicConfigurationDescriptorAsync(cccd_value)?;
    let status = async_op.get()?;

    if status != GattCommunicationStatus::Success {
        return Err(windows::core::Error::new(
            windows::core::HRESULT(-1),
            "Enable notification failed",
        ));
    }

    let uuid_clone = uuid_lower.clone();
    let handler = TypedEventHandler::new(
        move |_sender: &Option<GattCharacteristic>,
              args: &Option<GattValueChangedEventArgs>| {
            if let Some(args) = args {
                handle_value_changed(&uuid_clone, args);
            }
            Ok(())
        },
    );

    characteristic.ValueChanged(&handler)?;

    {
        let mut state = STATE.lock().unwrap();
        state.notification_handlers.insert(uuid_lower, send_wrapper::SendWrapper::new(handler));
    }

    Ok(())
}

pub fn disable_notification(uuid: &str) -> Result<(), windows::core::Error> {
    let uuid_lower = uuid.to_lowercase();

    let characteristic = {
        let state = STATE.lock().unwrap();
        state.characteristics.get(&uuid_lower).cloned()
    };

    let characteristic = characteristic.ok_or_else(|| {
        windows::core::Error::new(windows::core::HRESULT(-1), "Characteristic not found")
    })?;

    let cccd_value = GattClientCharacteristicConfigurationDescriptorValue::None;
    let async_op = characteristic.WriteClientCharacteristicConfigurationDescriptorAsync(cccd_value)?;
    let status = async_op.get()?;

    if status != GattCommunicationStatus::Success {
        return Err(windows::core::Error::new(
            windows::core::HRESULT(-1),
            "Disable notification failed",
        ));
    }

    {
        let mut state = STATE.lock().unwrap();
        state.notification_handlers.remove(&uuid_lower);
    }

    Ok(())
}

fn handle_value_changed(uuid: &str, args: &GattValueChangedEventArgs) {
    let buffer = match args.CharacteristicValue() {
        Ok(b) => b,
        Err(_) => return,
    };

    let reader = match DataReader::FromBuffer(&buffer) {
        Ok(r) => r,
        Err(_) => return,
    };

    let len = match reader.UnconsumedBufferLength() {
        Ok(l) => l as usize,
        Err(_) => return,
    };

    let mut data = vec![0u8; len];
    if reader.ReadBytes(&mut data).is_err() {
        return;
    }

    let handler = {
        let state = STATE.lock().unwrap();
        state.value_changed_handler
    };

    invoke_value_changed(handler, uuid, &data);
}
