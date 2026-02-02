/*!
    Hardware acceleration support.
*/

use std::ptr;

use ffmpeg_next::{ffi, util::frame::video::Video as VideoFrameFFmpeg};

use crate::config::HwDevice;

/**
    Hardware device context wrapper.
*/
pub(crate) struct HwDeviceContext {
    ctx: *mut ffi::AVBufferRef,
}

impl HwDeviceContext {
    /**
        Try to create a hardware device context.

        Returns None if hardware acceleration is not available.
    */
    #[cfg(target_os = "macos")]
    pub fn try_create(device: Option<HwDevice>) -> Option<Self> {
        // On macOS, default to VideoToolbox
        let device_type = match device {
            Some(HwDevice::VideoToolbox) | None => {
                ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_VIDEOTOOLBOX
            }
            _ => return None, // Other devices not supported on macOS
        };

        unsafe {
            let mut hw_device_ctx: *mut ffi::AVBufferRef = ptr::null_mut();
            let ret = ffi::av_hwdevice_ctx_create(
                &mut hw_device_ctx,
                device_type,
                ptr::null(),
                ptr::null_mut(),
                0,
            );

            if ret < 0 || hw_device_ctx.is_null() {
                return None;
            }

            Some(Self { ctx: hw_device_ctx })
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn try_create(_device: Option<HwDevice>) -> Option<Self> {
        // TODO: Implement VAAPI, CUDA, QSV support on other platforms
        None
    }

    /**
        Get the raw context pointer.
    */
    pub fn as_ptr(&self) -> *mut ffi::AVBufferRef {
        self.ctx
    }

    /**
        Create a reference to the context for use in a decoder.
    */
    pub fn create_ref(&self) -> *mut ffi::AVBufferRef {
        unsafe { ffi::av_buffer_ref(self.ctx) }
    }
}

impl Drop for HwDeviceContext {
    fn drop(&mut self) {
        if !self.ctx.is_null() {
            unsafe {
                ffi::av_buffer_unref(&mut self.ctx);
            }
        }
    }
}

// SAFETY: The FFmpeg buffer reference is internally reference-counted
// and thread-safe for the operations we perform.
unsafe impl Send for HwDeviceContext {}

/**
    Check if a frame is a hardware frame that needs transfer.
*/
pub(crate) fn is_hw_frame(frame: &VideoFrameFFmpeg) -> bool {
    let format = unsafe { (*frame.as_ptr()).format };
    // VideoToolbox format
    format == ffi::AVPixelFormat::AV_PIX_FMT_VIDEOTOOLBOX as i32
}

/**
    Transfer a hardware frame to a software frame.

    Returns an error if the transfer fails.
*/
pub(crate) fn transfer_hw_frame(
    hw_frame: &VideoFrameFFmpeg,
) -> Result<VideoFrameFFmpeg, ffmpeg_next::Error> {
    unsafe {
        let mut sw_frame = VideoFrameFFmpeg::empty();
        let ret = ffi::av_hwframe_transfer_data(sw_frame.as_mut_ptr(), hw_frame.as_ptr(), 0);

        if ret < 0 {
            return Err(ffmpeg_next::Error::from(ret));
        }

        // Copy PTS from hardware frame
        (*sw_frame.as_mut_ptr()).pts = (*hw_frame.as_ptr()).pts;

        Ok(sw_frame)
    }
}
