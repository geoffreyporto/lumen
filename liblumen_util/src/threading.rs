use std::panic;
use std::thread;

pub fn with_default_thread_pool<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    // the 1 here is duplicating code in config.opts.debugging_opts.threads
    // which also defaults to 1; it ultimately doesn't matter as the default
    // isn't threaded, and just ignores this parameter
    spawn_thread_pool(1, f)
}

pub fn spawn_thread_pool<F, R>(_threads: usize, f: F) -> R
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    let builder = thread::Builder::new().name("lumen".to_string());

    scoped_thread(builder, || f())
}

pub fn scoped_thread<F, R>(builder: thread::Builder, f: F) -> R
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    struct Ptr(*mut ());
    unsafe impl Send for Ptr {}
    unsafe impl Sync for Ptr {}

    let mut f = Some(f);
    let run = Ptr(&mut f as *mut _ as *mut ());
    let mut result = None;
    let result_ptr = Ptr(&mut result as *mut _ as *mut ());

    let thread = builder.spawn(move || {
        let run = unsafe { (*(run.0 as *mut Option<F>)).take().unwrap() };
        let result = unsafe { &mut *(result_ptr.0 as *mut Option<R>) };
        *result = Some(run());
    });

    match thread.unwrap().join() {
        Ok(()) => result.unwrap(),
        Err(p) => panic::resume_unwind(p),
    }
}
