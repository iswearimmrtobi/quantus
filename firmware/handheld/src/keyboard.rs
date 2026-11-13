//! A USB keyboard in the dev kit's USB-A port (host mode): the ESP-IDF USB Host Library with a
//! small boot-protocol HID driver on top. The reports become keys in `quantus_ui::hid` (German
//! layout, the simulator's key table) and go into the same channel as the serial monitor's.
//!
//! Two threads: one runs the host library's events, the other is its only client. Every
//! callback of the library runs on the client thread, inside `usb_host_client_handle_events`, so
//! the driver's state is only ever touched from that thread.

use std::ffi::c_void;
use std::ptr;
use std::sync::mpsc::{self, Sender};
use std::time::Instant;

use esp_idf_svc::sys::{self, esp, EspError};
use quantus_ui::hid::{self, BootInterface, Keyboard, Report};
use quantus_ui::key::Key;

/// Room for a control request's setup packet; the HID requests here carry no data.
const CONTROL_SIZE: usize = 64;
/// How long the client waits for events before it looks at the held key again, in ms.
const POLL_MS: u32 = 10;

/// HID class requests (HID 1.11, §7.2).
const SET_IDLE: u8 = 0x0A;
const SET_PROTOCOL: u8 = 0x0B;

/// Build with `QUANTUS_USB_DEBUG=1` to log every report and every key it gives, with the time.
const DEBUG: bool = option_env!("QUANTUS_USB_DEBUG").is_some();

/// Starts the USB host and the keyboard driver; keys go to `keys`. Without a keyboard, or if the
/// USB host does not start, it only logs.
pub fn start(keys: Sender<Key>) {
    let (installed, wait) = mpsc::channel();
    let host = std::thread::Builder::new().name("usb host".into()).stack_size(4 * 1024).spawn(move || {
        let config = sys::usb_host_config_t {
            intr_flags: sys::ESP_INTR_FLAG_LEVEL1 as i32,
            // The default: the high-speed controller, the USB-A port.
            peripheral_map: 0,
            ..Default::default()
        };
        // SAFETY: the config is valid for the call; the library copies what it keeps.
        let result = esp!(unsafe { sys::usb_host_install(&config) });
        let ok = result.is_ok();
        let _ = installed.send(result);
        if !ok {
            return;
        }
        loop {
            let mut flags = 0;
            // SAFETY: the library is installed and only this thread handles its events; the
            // timeout is portMAX_DELAY, waiting for ever.
            unsafe { sys::usb_host_lib_handle_events(u32::MAX, &mut flags) };
            if flags & sys::USB_HOST_LIB_EVENT_FLAGS_NO_CLIENTS != 0 {
                // SAFETY: no client is left, so every device may be freed.
                unsafe { sys::usb_host_device_free_all() };
            }
        }
    });
    if let Err(error) = host {
        log::warn!("no USB host thread: {error}");
        return;
    }
    let client = std::thread::Builder::new().name("keyboard".into()).stack_size(8 * 1024).spawn(move || {
        match wait.recv() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => return log::warn!("USB host not started: {error}"),
            Err(_) => return,
        }
        if let Err(error) = run(keys) {
            log::warn!("USB keyboard driver stopped: {error}");
        }
    });
    if let Err(error) = client {
        log::warn!("no USB keyboard thread: {error}");
    }
}

/// The client's state, shared with the library's callbacks through a raw pointer.
struct Driver {
    client: sys::usb_host_client_handle_t,
    keys: Sender<Key>,
    keyboard: Keyboard,
    start: Instant,
    /// Devices that arrived or left, noted by the event callback, handled after it.
    arrived: Vec<u8>,
    left: Vec<sys::usb_device_handle_t>,
    device: Option<Device>,
}

/// The open keyboard.
struct Device {
    handle: sys::usb_device_handle_t,
    interface: BootInterface,
    control: *mut sys::usb_transfer_t,
    report: *mut sys::usb_transfer_t,
    stage: Stage,
    /// Transfers submitted and not yet called back; the device closes once none are left.
    busy: u32,
    closing: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    SetProtocol,
    SetIdle,
    Running,
}

fn run(keys: Sender<Key>) -> Result<(), EspError> {
    // The driver lives as long as the firmware; the library keeps a pointer to it.
    let driver: *mut Driver = Box::into_raw(Box::new(Driver {
        client: ptr::null_mut(),
        keys,
        keyboard: Keyboard::default(),
        start: Instant::now(),
        arrived: Vec::new(),
        left: Vec::new(),
        device: None,
    }));
    let config = sys::usb_host_client_config_t {
        is_synchronous: false,
        max_num_event_msg: 5,
        __bindgen_anon_1: sys::usb_host_client_config_t__bindgen_ty_1 {
            async_: sys::usb_host_client_config_t__bindgen_ty_1__bindgen_ty_1 {
                client_event_callback: Some(client_event),
                callback_arg: driver.cast(),
            },
        },
    };
    let mut client = ptr::null_mut();
    // SAFETY: the config is valid for the call; `driver` stays valid for ever.
    esp!(unsafe { sys::usb_host_client_register(&config, &mut client) })?;
    // SAFETY: no callback can run yet, they only run inside `usb_host_client_handle_events`.
    unsafe { (*driver).client = client };
    log::info!("USB host ready for a keyboard");
    let ticks = (POLL_MS * sys::CONFIG_FREERTOS_HZ / 1000).max(1);
    loop {
        // SAFETY: the callbacks it runs reach `driver` only through their own pointer, and no
        // reference to it is alive across this call.
        unsafe { sys::usb_host_client_handle_events(client, ticks) };
        // SAFETY: the callbacks have returned; until the next call this is the only access.
        let driver = unsafe { &mut *driver };
        for address in std::mem::take(&mut driver.arrived) {
            driver.open(address);
        }
        for handle in std::mem::take(&mut driver.left) {
            driver.leave(handle);
        }
        driver.close_when_idle();
        let now = driver.now();
        if let Some(key) = driver.keyboard.tick(now) {
            if DEBUG {
                log::info!("usb {now} ms: repeat {key:?}");
            }
            let _ = driver.keys.send(key);
        }
    }
}

impl Driver {
    fn now(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// Opens a new device and starts it if it is a keyboard.
    fn open(&mut self, address: u8) {
        if self.device.is_some() {
            log::info!("USB device {address} ignored, a keyboard is already in use");
            return;
        }
        let mut handle = ptr::null_mut();
        // SAFETY: `client` is registered; `handle` receives the device.
        if let Err(error) = esp!(unsafe { sys::usb_host_device_open(self.client, address, &mut handle) }) {
            return log::warn!("USB device {address} not opened: {error}");
        }
        describe(address, handle);
        match self.set_up(handle) {
            Ok(Some(device)) => {
                log::info!("USB keyboard on interface {}", device.interface.number);
                self.device = Some(device);
            }
            Ok(None) => {
                log::info!("USB device {address} is no keyboard");
                // SAFETY: opened above, nothing of it claimed.
                unsafe { sys::usb_host_device_close(self.client, handle) };
            }
            Err(error) => {
                log::warn!("USB keyboard not started: {error}");
                // SAFETY: as above; `set_up` undid what it had done.
                unsafe { sys::usb_host_device_close(self.client, handle) };
            }
        }
    }

    /// Claims the keyboard interface of an open device and sends it the first request; `None`
    /// if the device has no boot keyboard interface.
    fn set_up(&mut self, handle: sys::usb_device_handle_t) -> Result<Option<Device>, EspError> {
        let mut config: *const sys::usb_config_desc_t = ptr::null();
        // SAFETY: `handle` is open; the descriptor stays valid while it is.
        esp!(unsafe { sys::usb_host_get_active_config_descriptor(handle, &mut config) })?;
        // SAFETY: the descriptor's first bytes hold its total length, the whole of it follows.
        let bytes = unsafe {
            let start = config.cast::<u8>();
            let length = u16::from_le_bytes([*start.add(2), *start.add(3)]);
            std::slice::from_raw_parts(start, usize::from(length))
        };
        let Some(interface) = hid::boot_keyboard(bytes) else { return Ok(None) };
        // SAFETY: `client` and `handle` are valid; the interface is in the active configuration.
        esp!(unsafe { sys::usb_host_interface_claim(self.client, handle, interface.number, interface.alternate) })?;
        let mut control = ptr::null_mut();
        let mut report = ptr::null_mut();
        let size = usize::from(interface.max_packet.max(8));
        // SAFETY: each call fills its pointer on success.
        let allocated = esp!(unsafe { sys::usb_host_transfer_alloc(CONTROL_SIZE, 0, &mut control) })
            .and_then(|()| esp!(unsafe { sys::usb_host_transfer_alloc(size, 0, &mut report) }));
        if let Err(error) = allocated {
            // SAFETY: freeing null is allowed; the interface was claimed above.
            unsafe {
                sys::usb_host_transfer_free(control);
                sys::usb_host_interface_release(self.client, handle, interface.number);
            }
            return Err(error);
        }
        let context: *mut c_void = (self as *mut Driver).cast();
        // SAFETY: both transfers were just allocated with these sizes and are not in flight.
        unsafe {
            (*control).device_handle = handle;
            (*control).bEndpointAddress = 0;
            (*control).callback = Some(control_done);
            (*control).context = context;
            (*report).device_handle = handle;
            (*report).bEndpointAddress = interface.endpoint;
            // An IN transfer's length is a whole number of packets.
            (*report).num_bytes = size as i32;
            (*report).callback = Some(report_done);
            (*report).context = context;
        }
        let mut device = Device { handle, interface, control, report, stage: Stage::SetProtocol, busy: 0, closing: false };
        // The boot protocol: the reports `Report::parse` reads, whatever the report descriptor.
        if let Err(error) = device.request(self.client, SET_PROTOCOL, 0) {
            // SAFETY: nothing is in flight; the interface was claimed above.
            unsafe {
                sys::usb_host_transfer_free(control);
                sys::usb_host_transfer_free(report);
                sys::usb_host_interface_release(self.client, handle, interface.number);
            }
            return Err(error);
        }
        Ok(Some(device))
    }

    /// A device left: the keyboard stops, and closes once its transfers are back.
    fn leave(&mut self, handle: sys::usb_device_handle_t) {
        let Some(device) = self.device.as_mut().filter(|device| device.handle == handle) else { return };
        log::info!("USB keyboard unplugged");
        device.closing = true;
        // SAFETY: the device handle is still open; halting and flushing returns the report
        // transfer through its callback.
        unsafe {
            sys::usb_host_endpoint_halt(handle, device.interface.endpoint);
            sys::usb_host_endpoint_flush(handle, device.interface.endpoint);
        }
        self.keyboard.reset();
    }

    /// Closes a leaving keyboard once no transfer of it is in flight.
    fn close_when_idle(&mut self) {
        if !self.device.as_ref().is_some_and(|device| device.closing && device.busy == 0) {
            return;
        }
        let Some(device) = self.device.take() else { return };
        // SAFETY: no transfer is in flight, so they may be freed and the interface released.
        unsafe {
            sys::usb_host_endpoint_clear(device.handle, device.interface.endpoint);
            sys::usb_host_interface_release(self.client, device.handle, device.interface.number);
            sys::usb_host_transfer_free(device.control);
            sys::usb_host_transfer_free(device.report);
            sys::usb_host_device_close(self.client, device.handle);
        }
    }
}

impl Device {
    /// Sends a HID class request without data to the keyboard's interface.
    fn request(&mut self, client: sys::usb_host_client_handle_t, request: u8, value: u16) -> Result<(), EspError> {
        let [value_low, value_high] = value.to_le_bytes();
        // Host to device, class, interface.
        let setup = [0x21, request, value_low, value_high, self.interface.number, 0, 0, 0];
        // SAFETY: the control transfer is not in flight (one request at a time) and its buffer
        // holds `CONTROL_SIZE` bytes.
        unsafe {
            ptr::copy_nonoverlapping(setup.as_ptr(), (*self.control).data_buffer, setup.len());
            (*self.control).num_bytes = setup.len() as i32;
        }
        // SAFETY: as above; the callback finds the driver through the transfer's context.
        esp!(unsafe { sys::usb_host_transfer_submit_control(client, self.control) })?;
        self.busy += 1;
        Ok(())
    }

    /// Asks for the next report.
    fn listen(&mut self) -> Result<(), EspError> {
        // SAFETY: the report transfer is not in flight, its callback has returned or not run.
        esp!(unsafe { sys::usb_host_transfer_submit(self.report) })?;
        self.busy += 1;
        Ok(())
    }
}

/// The client's events: only noted here, handled in `run` after the call returns.
unsafe extern "C" fn client_event(message: *const sys::usb_host_client_event_msg_t, arg: *mut c_void) {
    // SAFETY: `arg` is the driver from `run`, not borrowed while events are handled; the
    // message is valid for the call.
    let (driver, message) = unsafe { (&mut *arg.cast::<Driver>(), &*message) };
    match message.event {
        sys::usb_host_client_event_t_USB_HOST_CLIENT_EVENT_NEW_DEV => {
            // SAFETY: the union holds `new_dev` for this event.
            driver.arrived.push(unsafe { message.__bindgen_anon_1.new_dev.address });
        }
        sys::usb_host_client_event_t_USB_HOST_CLIENT_EVENT_DEV_GONE => {
            // SAFETY: the union holds `dev_gone` for this event.
            driver.left.push(unsafe { message.__bindgen_anon_1.dev_gone.dev_hdl });
        }
        _ => {}
    }
}

/// A HID request is done: SET_PROTOCOL, then SET_IDLE, then the reports. A keyboard may stall
/// SET_IDLE (it is optional), which only means it repeats reports by itself; both go on anyway.
unsafe extern "C" fn control_done(transfer: *mut sys::usb_transfer_t) {
    // SAFETY: the context is the driver from `run`, not borrowed while callbacks run.
    let driver = unsafe { &mut *(*transfer).context.cast::<Driver>() };
    let client = driver.client;
    let Some(device) = driver.device.as_mut() else { return };
    device.busy -= 1;
    if device.closing {
        return;
    }
    // SAFETY: the transfer is back and valid.
    let status = unsafe { (*transfer).status };
    if status != sys::usb_transfer_status_t_USB_TRANSFER_STATUS_COMPLETED {
        log::info!("USB keyboard: request answered with status {status}");
    }
    let next = match device.stage {
        Stage::SetProtocol => {
            device.stage = Stage::SetIdle;
            // Reports only when something changes; repeating is `Keyboard::tick`'s job.
            device.request(client, SET_IDLE, 0)
        }
        Stage::SetIdle => {
            device.stage = Stage::Running;
            device.listen()
        }
        Stage::Running => Ok(()),
    };
    if let Err(error) = next {
        log::warn!("USB keyboard stopped: {error}");
    }
}

/// A report arrived: its keys go out, and the next one is asked for.
unsafe extern "C" fn report_done(transfer: *mut sys::usb_transfer_t) {
    // SAFETY: the context is the driver from `run`, not borrowed while callbacks run.
    let driver = unsafe { &mut *(*transfer).context.cast::<Driver>() };
    let now = driver.now();
    let Some(device) = driver.device.as_mut() else { return };
    device.busy -= 1;
    // SAFETY: the transfer is back; its buffer holds `actual_num_bytes` received bytes.
    let (status, bytes) = unsafe {
        let length = usize::try_from((*transfer).actual_num_bytes).unwrap_or(0);
        ((*transfer).status, std::slice::from_raw_parts((*transfer).data_buffer, length))
    };
    if device.closing {
        return;
    }
    match status {
        sys::usb_transfer_status_t_USB_TRANSFER_STATUS_COMPLETED => {
            if DEBUG {
                log::info!("usb {now} ms: report {bytes:02x?}");
            }
            if let Some(report) = Report::parse(bytes) {
                for key in driver.keyboard.report(report, now) {
                    if DEBUG {
                        log::info!("usb {now} ms: key {key:?}");
                    }
                    let _ = driver.keys.send(key);
                }
            }
        }
        sys::usb_transfer_status_t_USB_TRANSFER_STATUS_TIMED_OUT | sys::usb_transfer_status_t_USB_TRANSFER_STATUS_SKIPPED => {}
        // Unplugged or cancelled: `leave` takes over.
        sys::usb_transfer_status_t_USB_TRANSFER_STATUS_NO_DEVICE | sys::usb_transfer_status_t_USB_TRANSFER_STATUS_CANCELED => return,
        status => return log::warn!("USB keyboard stopped: report status {status}; plug it in again"),
    }
    let Some(device) = driver.device.as_mut() else { return };
    if let Err(error) = device.listen() {
        log::warn!("USB keyboard stopped: {error}");
    }
}

/// Logs what a device says it is: vendor and product id, speed, and its name strings.
fn describe(address: u8, handle: sys::usb_device_handle_t) {
    let mut descriptor: *const sys::usb_device_desc_t = ptr::null();
    let mut info: sys::usb_device_info_t = Default::default();
    // SAFETY: `handle` is open; both out values are valid while it is.
    let ok = unsafe {
        esp!(sys::usb_host_get_device_descriptor(handle, &mut descriptor)).is_ok()
            && esp!(sys::usb_host_device_info(handle, &mut info)).is_ok()
    };
    if !ok || descriptor.is_null() {
        return log::info!("USB device {address}: no descriptor");
    }
    // SAFETY: a string descriptor is its length, its type, then UTF-16LE code units.
    let text = |string: *const sys::usb_str_desc_t| -> String {
        if string.is_null() {
            return String::new();
        }
        unsafe {
            let start = string.cast::<u8>();
            let length = usize::from(*start);
            let units: Vec<u16> = (2..length).step_by(2).map(|i| u16::from_le_bytes([*start.add(i), *start.add(i + 1)])).collect();
            String::from_utf16_lossy(&units)
        }
    };
    // SAFETY: checked for null above; valid while the device is open.
    let (vendor, product) = unsafe { ((*descriptor).__bindgen_anon_1.idVendor, (*descriptor).__bindgen_anon_1.idProduct) };
    log::info!(
        "USB device {address}: {vendor:04x}:{product:04x}, speed {}, parent hub {:?} port {}, {:?} {:?} {:?}",
        info.speed,
        info.parent.dev_hdl,
        info.parent.port_num,
        text(info.str_desc_manufacturer),
        text(info.str_desc_product),
        text(info.str_desc_serial_num),
    );
}
