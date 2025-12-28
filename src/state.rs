use std::sync::Mutex;
use crate::callbacks::*;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::AtomicBool;
use send_wrapper::SendWrapper;

pub static DISCOVERY_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
use windows::Devices::Bluetooth::Advertisement::{BluetoothLEAdvertisementWatcher, BluetoothLEAdvertisementReceivedEventArgs};
#[cfg(windows)]
use windows::Devices::Bluetooth::BluetoothLEDevice;
#[cfg(windows)]
use windows::Devices::Bluetooth::GenericAttributeProfile::{GattDeviceService, GattCharacteristic, GattValueChangedEventArgs};
#[cfg(windows)]
use windows::Foundation::TypedEventHandler;
#[cfg(windows)]
use windows::core::IInspectable;

pub static STATE: LazyLock<Mutex<PluginState>> = LazyLock::new(|| Mutex::new(PluginState::new()));

pub struct PluginState {
    pub initialized: bool,
    pub is_connected: bool,
    pub device_name: Option<String>,
    pub device_address: u64,
    pub battery_level: i32,
    pub signal_strength: i32,

    pub scan_result_handler: ScanResultHandler,
    pub scan_state_changed_handler: ScanStateChangedHandler,
    pub connection_state_handler: ConnectionStateHandler,
    pub service_discovered_handler: ServiceDiscoveredHandler,
    pub value_changed_handler: ValueChangedHandler,
    pub notification_handler: NotificationHandler,

    #[cfg(windows)]
    pub watcher: Option<BluetoothLEAdvertisementWatcher>,
    #[cfg(windows)]
    pub device: Option<BluetoothLEDevice>,
    #[cfg(windows)]
    pub services: HashMap<String, GattDeviceService>,
    #[cfg(windows)]
    pub characteristics: HashMap<String, GattCharacteristic>,
    #[cfg(windows)]
    pub notification_handlers: HashMap<String, SendWrapper<TypedEventHandler<GattCharacteristic, GattValueChangedEventArgs>>>,
    #[cfg(windows)]
    pub watcher_handler: Option<SendWrapper<TypedEventHandler<BluetoothLEAdvertisementWatcher, BluetoothLEAdvertisementReceivedEventArgs>>>,
    #[cfg(windows)]
    pub connection_status_handler: Option<SendWrapper<TypedEventHandler<BluetoothLEDevice, IInspectable>>>,

    pub discovered_devices: HashMap<String, u64>,
    pub read_cache: HashMap<String, Vec<u8>>,
    pub read_first_pass_buffer: HashMap<String, usize>,
}

impl PluginState {
    pub fn new() -> Self {
        Self {
            initialized: false,
            is_connected: false,
            device_name: None,
            device_address: 0,
            battery_level: -1,
            signal_strength: -100,

            scan_result_handler: None,
            scan_state_changed_handler: None,
            connection_state_handler: None,
            service_discovered_handler: None,
            value_changed_handler: None,
            notification_handler: None,

            #[cfg(windows)]
            watcher: None,
            #[cfg(windows)]
            device: None,
            #[cfg(windows)]
            services: HashMap::new(),
            #[cfg(windows)]
            characteristics: HashMap::new(),
            #[cfg(windows)]
            notification_handlers: HashMap::new(),
            #[cfg(windows)]
            watcher_handler: None,
            #[cfg(windows)]
            connection_status_handler: None,

            discovered_devices: HashMap::new(),
            read_cache: HashMap::new(),
            read_first_pass_buffer: HashMap::new(),
        }
    }

    pub fn reset(&mut self) {
        self.is_connected = false;
        self.device_name = None;
        self.device_address = 0;
        self.battery_level = -1;
        self.signal_strength = -100;

        #[cfg(windows)]
        {
            self.device = None;
            self.services.clear();
            self.characteristics.clear();
            self.notification_handlers.clear();
            self.connection_status_handler = None;
        }

        self.discovered_devices.clear();
        self.read_cache.clear();
        self.read_first_pass_buffer.clear();
    }
}

#[cfg(windows)]
pub fn initialize() -> Result<(), windows::core::Error> {
    use windows::Devices::Bluetooth::Advertisement::{
        BluetoothLEAdvertisementWatcher,
        BluetoothLEAdvertisementFilter,
        BluetoothLEAdvertisement,
    };
    use windows::core::HSTRING;

    let mut state = STATE.lock().unwrap();
    if state.initialized {
        return Ok(());
    }

    let watcher = BluetoothLEAdvertisementWatcher::new()?;

    let filter = BluetoothLEAdvertisementFilter::new()?;
    let adv = BluetoothLEAdvertisement::new()?;
    adv.SetLocalName(&HSTRING::from("SquareGolf"))?;
    filter.SetAdvertisement(&adv)?;
    watcher.SetAdvertisementFilter(&filter)?;

    state.watcher = Some(watcher);
    state.initialized = true;

    Ok(())
}

#[cfg(not(windows))]
pub fn initialize() -> Result<(), ()> {
    Err(())
}

pub fn cleanup() {
    let mut state = STATE.lock().unwrap();

    #[cfg(windows)]
    {
        if let Some(ref watcher) = state.watcher {
            let _ = watcher.Stop();
        }
        state.watcher = None;
        state.watcher_handler = None;
        state.device = None;
        state.services = HashMap::new();
        state.characteristics = HashMap::new();
        state.notification_handlers = HashMap::new();
        state.connection_status_handler = None;
    }

    state.reset();
    state.initialized = false;
}
