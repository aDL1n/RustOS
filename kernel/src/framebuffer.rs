use bootloader_api::info::{FrameBuffer, PixelFormat};
use core::fmt;
use lazy_static::lazy_static;
use noto_sans_mono_bitmap::{FontWeight, RasterHeight, get_raster};
use spin::Mutex;

const FONT_HEIGHT: RasterHeight = RasterHeight::Size16;
const FONT_WEIGHT: FontWeight = FontWeight::Light;

const CHAR_HEIGHT: usize = 16;

pub struct FrameBufferWriter {
    framebuffer: &'static mut FrameBuffer,
    x_pos: usize,
    y_pos: usize,
    pub(crate) color: [u8; 3],
    width: usize,
    height: usize,
    stride: usize,
    pixel_format: PixelFormat,
    bytes_per_pixel: usize,
}

impl FrameBufferWriter {
    pub fn new(framebuffer: &'static mut FrameBuffer) -> Self {
        let info = framebuffer.info();

        Self {
            framebuffer,
            x_pos: 0,
            y_pos: 0,
            color: [255, 255, 255],
            width: info.width,
            height: info.height,
            stride: info.stride,
            pixel_format: info.pixel_format,
            bytes_per_pixel: info.bytes_per_pixel,
        }
    }

    pub fn clear(&mut self) {
        self.framebuffer.buffer_mut().fill(0);

        self.x_pos = 0;
        self.y_pos = 0;
    }

    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.color = [r, g, b];
    }

    fn newline(&mut self) {
        self.x_pos = 0;
        self.y_pos += CHAR_HEIGHT;

        if self.y_pos + CHAR_HEIGHT >= self.height {
            self.scroll();
        }
    }

    fn scroll(&mut self) {
        let row_size = self.stride * self.bytes_per_pixel;
        let scroll_bytes = row_size * CHAR_HEIGHT;

        let buffer = self.framebuffer.buffer_mut();
        buffer.copy_within(scroll_bytes.., 0);

        let len = buffer.len();
        buffer[len - scroll_bytes..].fill(0);

        self.y_pos -= CHAR_HEIGHT;
    }

    fn write_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
        if x >= self.width || y >= self.height {
            return;
        }

        let offset = (y * self.stride + x) * self.bytes_per_pixel;

        let buffer = self.framebuffer.buffer_mut();

        match self.pixel_format {
            PixelFormat::Rgb => {
                buffer[offset] = r;
                buffer[offset + 1] = g;
                buffer[offset + 2] = b;
            }

            PixelFormat::Bgr => {
                buffer[offset] = b;
                buffer[offset + 1] = g;
                buffer[offset + 2] = r;
            }

            _ => {}
        }
    }

    fn draw_char(&mut self, c: char) {
        let glyph = get_raster(c, FONT_WEIGHT, FONT_HEIGHT)
            .or_else(|| get_raster('?', FONT_WEIGHT, FONT_HEIGHT))
            .unwrap();

        for (row, pixels) in glyph.raster().iter().enumerate() {
            for (col, intensity) in pixels.iter().enumerate() {
                if *intensity > 0 {
                    self.write_pixel(
                        self.x_pos + col,
                        self.y_pos + row,
                        self.color[0],
                        self.color[1],
                        self.color[2],
                    );
                }
            }
        }

        self.x_pos += glyph.width();

        if self.x_pos + glyph.width() >= self.width {
            self.newline();
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.newline(),

            0x20..=0x7e => {
                self.draw_char(byte as char);
            }

            _ => {
                self.draw_char('■');
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }
}

impl fmt::Write for FrameBufferWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Option<FrameBufferWriter>> = Mutex::new(None);
}

pub fn init_framebuffer(framebuffer: &'static mut FrameBuffer) {
    let mut writer = FrameBufferWriter::new(framebuffer);

    writer.clear();

    *WRITER.lock() = Some(writer);
}