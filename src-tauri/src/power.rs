#[cfg(target_os = "macos")]
mod macos {
    use crate::timer::engine::TimerEngine;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};
    use tauri::{AppHandle, Emitter};

    type IoObjectT = u32;
    type IoServiceT = u32;
    type IoConnectT = u32;
    type NaturalT = u32;
    type IONotificationPortRef = *mut c_void;
    type CFRunLoopRef = *mut c_void;
    type CFRunLoopSourceRef = *mut c_void;
    type CFStringRef = *const c_void;
    type IOReturn = i32;

    const K_IO_MESSAGE_CAN_SYSTEM_SLEEP: NaturalT = 0xE000_0270;
    const K_IO_MESSAGE_SYSTEM_WILL_SLEEP: NaturalT = 0xE000_0280;
    const K_IO_MESSAGE_SYSTEM_HAS_POWERED_ON: NaturalT = 0xE000_0300;

    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IORegisterForSystemPower(
            refcon: *mut c_void,
            the_port_ref: *mut IONotificationPortRef,
            callback: extern "C" fn(*mut c_void, IoServiceT, NaturalT, *mut c_void),
            notifier: *mut IoObjectT,
        ) -> IoConnectT;

        fn IOAllowPowerChange(root_port: IoConnectT, notification_id: usize) -> IOReturn;
        fn IONotificationPortGetRunLoopSource(port: IONotificationPortRef) -> CFRunLoopSourceRef;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFRunLoopCommonModes: CFStringRef;
        fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
        fn CFRunLoopRun();
    }

    struct PowerContext {
        app_handle: AppHandle,
        engine: Arc<Mutex<TimerEngine>>,
        root_port: AtomicU32,
    }

    extern "C" fn power_callback(
        refcon: *mut c_void,
        _service: IoServiceT,
        message_type: NaturalT,
        message_argument: *mut c_void,
    ) {
        let context = unsafe { (refcon as *mut PowerContext).as_ref() };
        let Some(context) = context else {
            return;
        };

        match message_type {
            K_IO_MESSAGE_CAN_SYSTEM_SLEEP => {
                let root_port = context.root_port.load(Ordering::SeqCst);
                if root_port != 0 {
                    let _ = unsafe { IOAllowPowerChange(root_port, message_argument as usize) };
                }
            }
            K_IO_MESSAGE_SYSTEM_WILL_SLEEP => {
                {
                    let mut engine = context.engine.lock().unwrap();
                    engine.on_system_will_sleep();
                    crate::timer::engine::update_tray_now(&context.app_handle, &engine);
                }

                let root_port = context.root_port.load(Ordering::SeqCst);
                if root_port != 0 {
                    let _ = unsafe { IOAllowPowerChange(root_port, message_argument as usize) };
                }
            }
            K_IO_MESSAGE_SYSTEM_HAS_POWERED_ON => {
                let payload = {
                    let mut engine = context.engine.lock().unwrap();
                    if let Some(excluded_secs) = engine.on_system_did_wake() {
                        eprintln!(
                            "[CT] macOS power wake: excluded {}s of sleep time",
                            excluded_secs
                        );
                    }
                    crate::timer::engine::update_tray_now(&context.app_handle, &engine);
                    engine.get_tick_payload()
                };
                let _ = context.app_handle.emit("timer-tick", &payload);
            }
            _ => {}
        }
    }

    pub fn register_power_notifications(app_handle: AppHandle, engine: Arc<Mutex<TimerEngine>>) {
        let _ = std::thread::Builder::new()
            .name("ct-macos-power-notify".to_string())
            .spawn(move || {
                let context = Box::new(PowerContext {
                    app_handle,
                    engine,
                    root_port: AtomicU32::new(0),
                });
                let context_ptr = Box::into_raw(context);

                let mut notification_port: IONotificationPortRef = std::ptr::null_mut();
                let mut notifier: IoObjectT = 0;

                let root_port = unsafe {
                    IORegisterForSystemPower(
                        context_ptr.cast(),
                        &mut notification_port,
                        power_callback,
                        &mut notifier,
                    )
                };

                if root_port == 0 || notification_port.is_null() {
                    eprintln!(
                        "[CT] macOS power: failed to register sleep/wake notifications"
                    );
                    unsafe {
                        let _ = Box::from_raw(context_ptr);
                    }
                    return;
                }

                unsafe {
                    (*context_ptr).root_port.store(root_port, Ordering::SeqCst);
                }

                let source = unsafe { IONotificationPortGetRunLoopSource(notification_port) };
                if source.is_null() {
                    eprintln!(
                        "[CT] macOS power: missing run loop source for sleep/wake notifications"
                    );
                    unsafe {
                        let _ = Box::from_raw(context_ptr);
                    }
                    return;
                }

                unsafe {
                    CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
                    CFRunLoopRun();
                }
            });
    }
}

#[cfg(target_os = "macos")]
pub use macos::register_power_notifications;

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
pub fn register_power_notifications(
    _app_handle: tauri::AppHandle,
    _engine: std::sync::Arc<std::sync::Mutex<crate::timer::engine::TimerEngine>>,
) {
}
