extern crate wee_alloc;

use std::mem;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[no_mangle]
pub fn handle() {
    trace(&format!("Handling {:?} {}", get_request_method(), get_request_url()));
    set_response_status(200);
    set_response_header("My-Custom-Header", "hello2");
    set_response_body(&format!("Hi!\nYou requested {}\n", get_request_url()));
}


















#[derive(Debug)]
#[repr(C)]
pub enum Method {
    Get = 1,
    Post = 2,
    Put = 3,
    Delete = 4
}

extern "C" {
    pub fn _get_request_method() -> Method;
    pub fn _get_request_url(ptr: *mut u8, len: usize) -> usize;
    pub fn _get_request_url_len() -> usize;
    pub fn _get_request_header(hdr_ptr: *const u8, hdr_len: usize, dst_ptr: *mut u8, dst_len: usize) -> usize;
    pub fn _get_request_header_len(hdr_ptr: *const u8, hdr_len: usize) -> usize;
    pub fn _get_request_body(ptr: *mut u8, len: usize) -> usize;
    pub fn _get_request_body_len() -> usize;
    pub fn _set_response_status(rc: u32);
    pub fn _set_response_header(hdr_ptr: *const u8, hdr_len: usize, val_ptr: *const u8, val_len: usize);
    pub fn _set_response_body(ptr: *const u8, len: usize);
    pub fn _trace(ptr: *const u8, len: usize);
}

fn get_str<S: Fn(*mut u8, usize) -> usize, T: Fn() -> usize>(get: S, get_len: T) -> String {
    unsafe {
        let mut vec = Vec::<u8>::with_capacity(get_len());
        let ptr = vec.as_mut_ptr();
        let len = vec.capacity();
        mem::forget(vec);
        get(ptr, len);
        let vec = Vec::from_raw_parts(ptr, len, len);
        String::from_utf8(vec).unwrap()
    }
}

fn get_request_method() -> Method {
    unsafe {
        _get_request_method()
    }
}

fn get_request_url() -> String {
    get_str(|p, l| unsafe { _get_request_url(p, l) }, || unsafe { _get_request_url_len()} )
}

fn get_request_header(hdr: &str) -> String {
    get_str(|p, l| unsafe { _get_request_header(hdr.as_ptr(), hdr.len(), p, l) }, || unsafe { _get_request_header_len(hdr.as_ptr(), hdr.len() )})
}

fn get_request_body() -> String {
    get_str(|p, l| unsafe { _get_request_body(p, l) }, || unsafe { _get_request_body_len()} )
}

fn set_response_status(rc: u32) {
    unsafe {
        _set_response_status(rc);
    }
}

fn set_response_header(hdr: &str, val: &str) {
    unsafe {
        _set_response_header(hdr.as_ptr(), hdr.len(), val.as_ptr(), val.len());
    }
}

fn set_response_body(body: &str) {
    unsafe {
        _set_response_body(body.as_ptr(), body.len());
    }
}

fn trace(msg: &str) {
    unsafe {
        _trace(msg.as_ptr(), msg.len());
    }
}

