#![allow(clippy::doc_overindented_list_items)]

pub use drm_core as core;

mod constants;
mod crypto;
mod device;
mod error;
mod pssh_ext;
mod session;
mod types;

pub mod proto {
    pub use drm_widevine_proto::prost::Message;
    pub use drm_widevine_proto::*;
}

#[cfg(feature = "static-devices")]
pub mod static_devices;

pub use self::device::Device;
pub use self::error::{CdmError, CdmResult};
pub use self::pssh_ext::WidevineExt;
pub use self::session::Session;
pub use self::types::{DeviceType, LicenseType, SecurityLevel};
