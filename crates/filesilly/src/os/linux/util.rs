use libc::c_int;

pub fn mark_errno(value: c_int) {
    unsafe {
        *libc::__errno_location() = value;
    }
}
