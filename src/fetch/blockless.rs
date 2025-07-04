use crate::fetch::FetchOptions;
use serde_json::{json, Value as SerdeValue};
use std::cmp::Ordering;
use std::collections::HashMap;

pub type Handle = u32;

pub type CodeStatus = u32;

#[allow(dead_code)]
pub struct BlocklessHttp {
    inner: Handle,
    code: CodeStatus,
}

pub struct HttpOptions {
    method: String,
    connect_timeout: u32,
    read_timeout: u32,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
}

impl HttpOptions {
    pub fn new(
        method: &str,
        connect_timeout: u32,
        read_timeout: u32,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    ) -> Self {
        HttpOptions {
            method: method.into(),
            connect_timeout,
            read_timeout,
            headers,
            body,
        }
    }

    pub fn to_json(&self) -> SerdeValue {
        let mut obj = serde_json::Map::new();
        obj.insert("method".to_string(), SerdeValue::String(self.method.clone()));
        obj.insert("connectTimeout".to_string(), json!(self.connect_timeout));
        obj.insert("readTimeout".to_string(), json!(self.read_timeout));
        
        match &self.headers {
            Some(h) if !h.is_empty() => {
                let headers_str = serde_json::to_string(h).unwrap_or_else(|_| "{}".to_string());
                obj.insert("headers".to_string(), SerdeValue::String(headers_str));
            }
            _ => {
                obj.insert("headers".to_string(), SerdeValue::String("{}".to_string())); 
            }
        }

        obj.insert(
            "body".to_string(),
            self.body.as_ref().map_or(SerdeValue::Null, |b| {
                SerdeValue::String(b.clone())
            }),
        );
        SerdeValue::Object(obj)
    }
}

#[allow(dead_code)]
impl BlocklessHttp {
    pub fn open(url: &str, opts: &FetchOptions) -> Result<Self, HttpErrorKind> {
        let http_opts = HttpOptions::new(
            &opts.method,
            30,
            10,
            opts.headers.clone(),
            opts.body.clone(),
        );
        let http_opts_value = http_opts.to_json();
        let http_opts_str = serde_json::to_string(&http_opts_value).unwrap();

        let mut fd = 0;
        let mut status = 0;
        let rs = unsafe {
            http_req(
                url.as_ptr(),
                url.len() as _,
                http_opts_str.as_ptr(),
                http_opts_str.len() as _,
                &mut fd,
                &mut status,
            )
        };
        if rs != 0 {
            return Err(HttpErrorKind::from(rs));
        }
        Ok(Self {
            inner: fd,
            code: status,
        })
    }

    #[allow(dead_code)]
    pub fn get_code(&self) -> CodeStatus {
        self.code
    }

    pub fn get_all_body(&self) -> Result<Vec<u8>, HttpErrorKind> {
        let mut vec = Vec::new();
        loop {
            let mut buf = [0u8; 1024];
            let mut num: u32 = 0;
            let rs =
                unsafe { http_read_body(self.inner, buf.as_mut_ptr(), buf.len() as _, &mut num) };

            if rs == u32::MAX {
                continue;
            } else if rs != 0 {
                return Err(HttpErrorKind::from(rs));
            } else {
                match num.cmp(&0) {
                    Ordering::Greater => vec.extend_from_slice(&buf[0..num as _]),
                    _ => break,
                }
            }
        }
        Ok(vec)
    }

    pub fn get_header(&self, header: &str) -> Result<String, HttpErrorKind> {
        let mut vec = Vec::new();
        loop {
            let mut buf = [0u8; 1024];
            let mut num: u32 = 0;
            let rs = unsafe {
                http_read_header(
                    self.inner,
                    header.as_ptr(),
                    header.len() as _,
                    buf.as_mut_ptr(),
                    buf.len() as _,
                    &mut num,
                )
            };

            if rs == u32::MAX {
                continue;
            } else if rs != 0 {
                return Err(HttpErrorKind::from(rs));
            } else {
                match num.cmp(&0) {
                    Ordering::Greater => vec.extend_from_slice(&buf[0..num as _]),
                    _ => break,
                }
            }
        }
        String::from_utf8(vec).map_err(|_| HttpErrorKind::Utf8Error)
    }

    pub fn close(self) {
        unsafe {
            http_close(self.inner);
        }
    }

    pub fn read_body(&self, buf: &mut [u8]) -> Result<u32, HttpErrorKind> {
        let mut num: u32 = 0;
        let rs = unsafe { http_read_body(self.inner, buf.as_mut_ptr(), buf.len() as _, &mut num) };
        if rs != 0 {
            return Err(HttpErrorKind::from(rs));
        }
        Ok(num)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum HttpErrorKind {
    InvalidDriver,
    InvalidHandle,
    MemoryAccessError,
    BufferTooSmall,
    HeaderNotFound,
    Utf8Error,
    DestinationNotAllowed,
    InvalidMethod,
    InvalidEncoding,
    InvalidUrl,
    RequestError,
    RuntimeError,
    TooManySessions,
    PermissionDeny,
}

impl std::error::Error for HttpErrorKind {}

impl std::fmt::Display for HttpErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::InvalidDriver => write!(f, "Invalid Driver"),
            Self::InvalidHandle => write!(f, "Invalid Error"),
            Self::MemoryAccessError => write!(f, "Memory Access Error"),
            Self::BufferTooSmall => write!(f, "Buffer too small"),
            Self::HeaderNotFound => write!(f, "Header not found"),
            Self::Utf8Error => write!(f, "Utf8 error"),
            Self::DestinationNotAllowed => write!(f, "Destination not allowed"),
            Self::InvalidMethod => write!(f, "Invalid method"),
            Self::InvalidEncoding => write!(f, "Invalid encoding"),
            Self::InvalidUrl => write!(f, "Invalid url"),
            Self::RequestError => write!(f, "Request url"),
            Self::RuntimeError => write!(f, "Runtime error"),
            Self::TooManySessions => write!(f, "Too many sessions"),
            Self::PermissionDeny => write!(f, "Permission deny."),
        }
    }
}

impl From<u32> for HttpErrorKind {
    fn from(i: u32) -> HttpErrorKind {
        match i {
            1 => HttpErrorKind::InvalidDriver,
            2 => HttpErrorKind::InvalidHandle,
            3 => HttpErrorKind::MemoryAccessError,
            4 => HttpErrorKind::BufferTooSmall,
            5 => HttpErrorKind::HeaderNotFound,
            6 => HttpErrorKind::Utf8Error,
            7 => HttpErrorKind::DestinationNotAllowed,
            8 => HttpErrorKind::InvalidMethod,
            9 => HttpErrorKind::InvalidEncoding,
            10 => HttpErrorKind::InvalidUrl,
            11 => HttpErrorKind::RequestError,
            12 => HttpErrorKind::RuntimeError,
            13 => HttpErrorKind::TooManySessions,
            14 => HttpErrorKind::PermissionDeny,
            _ => HttpErrorKind::RuntimeError,
        }
    }
}

#[link(wasm_import_module = "blockless_http")]
extern "C" {
    #[link_name = "http_req"]
    pub(crate) fn http_req(
        url: *const u8,
        url_len: u32,
        opts: *const u8,
        opts_len: u32,
        fd: *mut u32,
        status: *mut u32,
    ) -> u32;

    #[allow(dead_code)]
    #[link_name = "http_read_header"]
    pub(crate) fn http_read_header(
        handle: u32,
        header: *const u8,
        header_len: u32,
        buf: *mut u8,
        buf_len: u32,
        num: *mut u32,
    ) -> u32;

    #[link_name = "http_read_body"]
    pub(crate) fn http_read_body(handle: u32, buf: *mut u8, buf_len: u32, num: *mut u32) -> u32;

    #[link_name = "http_close"]
    pub(crate) fn http_close(handle: u32) -> u32;
}
