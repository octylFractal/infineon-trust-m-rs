use crate::cbindings;
use naive_timer::Timer;

// Can't be a none value because sometimes a null callback is passed through, but a reference
// to a valid event is always required.
static mut pal_os_event_0: Option<cbindings::pal_os_event_t> = Some(cbindings::pal_os_event {
    is_event_triggered: false as u8,
    callback_registered: None,
    callback_ctx: core::ptr::null_mut(),
    os_timer: core::ptr::null_mut(),
    sync_flag: 0,
    timeout_us: 0,
});
static mut pal_os_event_cback_timer: Option<Timer> = None;

// handle the callback stack
#[no_mangle]
pub unsafe extern "C" fn pal_os_event_destroy(event: *mut cbindings::pal_os_event_t) {
    // pal_os_event_0 = None;
}

#[no_mangle]
pub unsafe extern "C" fn pal_os_event_create(
    callback: cbindings::register_callback,
    callback_args: *mut cty::c_void,
) -> *mut cbindings::pal_os_event_t {
    #[cfg(not(any(test, feature = "tester")))]
    defmt::trace!(
        "callback: {}, callback_args: {}",
        !callback.is_some(),
        !callback_args.is_null()
    );

    if !callback.is_some() && !callback_args.is_null() {
        pal_os_event_start(
            &mut pal_os_event_0.unwrap() as *mut cbindings::pal_os_event_t,
            callback,
            callback_args,
        );
    }

    return &mut pal_os_event_0.unwrap() as *mut cbindings::pal_os_event_t;
}

#[no_mangle]
pub unsafe extern "C" fn pal_os_event_trigger_registered_callback() {
    if let Some(event) = pal_os_event_0 {
        if event.is_event_triggered != 0 {
            let mut event = pal_os_event_0.take().unwrap();
            event.is_event_triggered = true as u8;
            let callback = event.callback_registered.take().unwrap();
            callback(event.callback_ctx);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pal_os_event_register_callback_oneshot(
    p_pal_os_event: *mut cbindings::pal_os_event_t,
    callback: cbindings::register_callback,
    callback_args: *mut cty::c_void,
    time_us: u32,
) {
    let os_event: &mut cbindings::pal_os_event_t = p_pal_os_event.as_mut().unwrap();

    *os_event = cbindings::pal_os_event {
        is_event_triggered: false as u8,
        callback_registered: callback,
        callback_ctx: callback_args,
        os_timer: core::ptr::null_mut(),
        sync_flag: 0,
        timeout_us: 0,
    };

    struct CallbackCtx(*mut cty::c_void);
    unsafe impl Send for CallbackCtx {}
    unsafe impl Sync for CallbackCtx {}
    impl CallbackCtx {
        unsafe fn callfunc(self, callback: cbindings::register_callback) {
            if let Some(callback) = callback {
                let CallbackCtx(context) = self;
                callback(context);
            }
        }
    }

    let context = CallbackCtx(os_event.callback_ctx);

    let timer: &mut _ = pal_os_event_cback_timer.get_or_insert(Timer::default());

    timer.add(
        core::time::Duration::from_micros(time_us as u64 + systick::micros()),
        |_| {
            context.callfunc(os_event.callback_registered);
        },
    );
}

#[no_mangle]
pub unsafe extern "C" fn pal_os_event_start(
    p_pal_os_event: *mut cbindings::pal_os_event_t,
    callback: cbindings::register_callback,
    callback_args: *mut cty::c_void,
) {
    if let Some(ref mut os_event) = p_pal_os_event.as_mut() {
        if os_event.is_event_triggered == false as u8 {
            os_event.is_event_triggered = true as u8;
            pal_os_event_register_callback_oneshot(p_pal_os_event, callback, callback_args, 1000);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pal_os_event_stop(p_pal_os_event: *mut cbindings::pal_os_event_t) {
    if let Some(ref mut os_event) = p_pal_os_event.as_mut() {
        os_event.is_event_triggered = false as u8;
    }
}

#[no_mangle]
pub unsafe extern "C" fn pal_os_event_process() {
    let timer: &mut _ = pal_os_event_cback_timer.get_or_insert(Timer::default());

    timer.expire({
        #[cfg(not(feature = "tester"))]
        {
            core::time::Duration::from_micros(systick::micros())
        }

        #[cfg(feature = "tester")]
        {
            crate::since_started.elapsed()
        }
    });
}
