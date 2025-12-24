//! Video frame types and processing

use crate::ffi::CPixelFormat;

/// Pixel format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb24,
    Mjpeg,
    H264,
    Yuyv,
    Nv12,
}

impl From<CPixelFormat> for PixelFormat {
    fn from(f: CPixelFormat) -> Self {
        match f {
            CPixelFormat::Rgb24 => PixelFormat::Rgb24,
            CPixelFormat::Mjpeg => PixelFormat::Mjpeg,
            CPixelFormat::H264 => PixelFormat::H264,
            CPixelFormat::Yuyv => PixelFormat::Yuyv,
            CPixelFormat::Nv12 => PixelFormat::Nv12,
        }
    }
}

/// Decoded RGB video frame
#[derive(Debug, Clone)]
pub struct VideoFrame {
    /// RGB pixel data (width * height * 3)
    pub data: Vec<u8>,
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
    /// Capture timestamp (nanoseconds)
    pub timestamp_ns: u64,
    /// Frame sequence number
    pub sequence: u32,
}

impl VideoFrame {
    /// Create a new video frame from raw RGB data
    pub fn new(data: Vec<u8>, width: u32, height: u32, timestamp_ns: u64, sequence: u32) -> Self {
        Self {
            data,
            width,
            height,
            timestamp_ns,
            sequence,
        }
    }

    /// Get pixel at (x, y)
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<[u8; 3]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y * self.width + x) * 3) as usize;
        Some([self.data[idx], self.data[idx + 1], self.data[idx + 2]])
    }

    /// Convert to grayscale
    pub fn to_grayscale(&self) -> Vec<u8> {
        let mut gray = Vec::with_capacity((self.width * self.height) as usize);
        for pixel in self.data.chunks(3) {
            // Luminance formula: 0.299*R + 0.587*G + 0.114*B
            let y = (pixel[0] as f32 * 0.299 
                   + pixel[1] as f32 * 0.587 
                   + pixel[2] as f32 * 0.114) as u8;
            gray.push(y);
        }
        gray
    }

    /// Crop a region of the frame
    pub fn crop(&self, x: u32, y: u32, w: u32, h: u32) -> Option<VideoFrame> {
        if x + w > self.width || y + h > self.height {
            return None;
        }

        let mut cropped = Vec::with_capacity((w * h * 3) as usize);
        for row in y..(y + h) {
            let start = ((row * self.width + x) * 3) as usize;
            let end = start + (w * 3) as usize;
            cropped.extend_from_slice(&self.data[start..end]);
        }

        Some(VideoFrame {
            data: cropped,
            width: w,
            height: h,
            timestamp_ns: self.timestamp_ns,
            sequence: self.sequence,
        })
    }

    /// Resize frame using bilinear interpolation
    pub fn resize(&self, new_width: u32, new_height: u32) -> VideoFrame {
        let mut resized = Vec::with_capacity((new_width * new_height * 3) as usize);
        
        let x_ratio = self.width as f32 / new_width as f32;
        let y_ratio = self.height as f32 / new_height as f32;

        for y in 0..new_height {
            for x in 0..new_width {
                let src_x = x as f32 * x_ratio;
                let src_y = y as f32 * y_ratio;
                
                let x0 = src_x.floor() as u32;
                let y0 = src_y.floor() as u32;
                
                // Simple nearest neighbor for now
                if let Some(pixel) = self.get_pixel(x0.min(self.width - 1), y0.min(self.height - 1)) {
                    resized.extend_from_slice(&pixel);
                } else {
                    resized.extend_from_slice(&[0, 0, 0]);
                }
            }
        }

        VideoFrame {
            data: resized,
            width: new_width,
            height: new_height,
            timestamp_ns: self.timestamp_ns,
            sequence: self.sequence,
        }
    }
}

/// Decode MJPEG frame to RGB
#[cfg(feature = "jpeg-decode")]
pub fn decode_mjpeg(mjpeg_data: &[u8]) -> Result<VideoFrame, image::ImageError> {
    use image::ImageFormat;
    
    let img = image::load_from_memory_with_format(mjpeg_data, ImageFormat::Jpeg)?;
    let rgb = img.to_rgb8();
    
    Ok(VideoFrame {
        data: rgb.into_raw(),
        width: img.width(),
        height: img.height(),
        timestamp_ns: 0,
        sequence: 0,
    })
}
