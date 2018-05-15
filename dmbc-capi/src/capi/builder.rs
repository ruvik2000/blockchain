use std::ptr; 
use std::mem;

use libc::{c_void, size_t};
use exonum::messages::Message;

use transactions::builders::transaction::{Builder, AddAssetBuilder};
use transactions::add_assets::ADD_ASSETS_ID;
use capi::common::hex_string;

use error::{Error, ErrorKind};

#[derive(Debug, Clone)]
pub struct BuilderContext {
    pub context_ptr: *mut c_void,
    pub message_type: u16,
}

impl BuilderContext {
    pub fn from_ptr<'a>(context: *mut BuilderContext) -> Result<&'a BuilderContext, Error> {
        if context.is_null() {
            return Err(
                Error::new(
                    ErrorKind::Text("context isn't initialized".to_string())
                )
            );
        }
        Ok( unsafe { &*context } )
    }

    pub fn guard(&self, message_type: u16) -> Result<(), Error> {
        if self.message_type != message_type {
            return Err(
                Error::new(ErrorKind::Text("Different builder type".to_string()))
            );
        }
        Ok(())
    }
}

ffi_fn! {
    fn dmbc_builder_create(
        network_id: u8,
        protocol_version: u8,
        service_id: u16,
        message_type: u16,
        error: *mut Error,
    ) -> *mut BuilderContext {

        let context_ptr: *mut c_void = match message_type {
            ADD_ASSETS_ID => {
                let builder = Builder::new(network_id, protocol_version, service_id)
                    .tx_add_asset();
                unsafe { mem::transmute(Box::new(builder)) }
            },
            _ => {
                unsafe {
                    if !error.is_null() {
                        *error = Error::new(ErrorKind::Text(format!("Unknown message type '{}'", message_type)));
                    }
                    return ptr::null_mut();
                }
            }
        };

        Box::into_raw(
            Box::new(
                BuilderContext {
                context_ptr,
                message_type
            })
        )
    }
}

ffi_fn! {
    fn dmbc_builder_free(context: *const BuilderContext) {
        if !context.is_null() {
            unsafe { Box::from_raw(context as *mut BuilderContext); }
        }
    }
}

ffi_fn! {
    fn dmbc_builder_tx_create(
        context: *mut BuilderContext,
        length: *mut size_t,
        error: *mut Error,
    ) -> *const u8 {
        let context = match BuilderContext::from_ptr(context) {
            Ok(context) => context,
            Err(err) => {
                unsafe {
                    if !error.is_null() {
                        *error = err;
                    }
                    return ptr::null();
                }
            }
        };

        let mut bytes = match context.message_type {
            ADD_ASSETS_ID => {
                let builder: &mut AddAssetBuilder = unsafe { mem::transmute(context.context_ptr) };
                match builder.build() {
                    Ok(tx) => { tx.raw().body().to_vec()},
                    Err(err) => {
                        unsafe {
                            if !error.is_null() {
                                *error = err;
                            }
                            return ptr::null();
                        }
                    }
                }
            }
            _ => {
                unsafe {
                    if !error.is_null() {
                        *error = Error::new(ErrorKind::Text("Unknown context, not implemented".to_string()));
                    }
                    return ptr::null();
                }
            }
        };

        if length.is_null() {
            unsafe {
                if !error.is_null() {
                    *error = Error::new(ErrorKind::Text("length argument is null".to_string()));
                }
                return ptr::null();
            }
        }
        
        bytes.shrink_to_fit();
        let to_print = hex_string(bytes.clone());
        println!("{}", to_print);

        assert!(bytes.len() == bytes.capacity());
        let ptr = bytes.as_ptr();
        let length = unsafe { &mut *length };
        let len = bytes.len() as size_t;
        *length = len;

        mem::forget(bytes);
        ptr
    }
}

ffi_fn! {
    fn dmbc_builder_tx_free(ptr: *mut u8, len: size_t) {
        let len = len as usize;
        unsafe {
            drop(Vec::from_raw_parts(ptr, len, len));
        }
    }
}