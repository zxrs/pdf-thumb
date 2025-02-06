//! This library is a thin wrapper of WinRT [PdfDocument Class](https://learn.microsoft.com/en-us/uwp/api/windows.data.pdf.pdfdocument?view=winrt-26100) to generate a thumbnail image for PDF.
//!
//! # Example
//!
//! ```
//! use anyhow::Result;
//! use pdf_thumb::PdfThumb;
//!
//! fn main() -> Result<()> {
//!     let pdf = PdfThumb::open("test.pdf")?;
//!     let thumb = pdf.thumb()?;
//!     std::fs::write("thumb.png", &thumb)?; // PNG is default.
//!     Ok(())
//! }
//! ```
//!
//! Some options are also available.
//!
//! ```
//! use anyhow::Result;
//! use pdf_thumb::{ImageFormat, Options, PdfThumb};
//!
//! fn main() -> Result<()> {
//!     let pdf = PdfThumb::open("test.pdf")?;
//!     let options = Options {
//!         width: 320,                // Set thumbnail image width.
//!         format: ImageFormat::Jpeg, // Set thumbnail image format.
//!         ..Default::default()
//!     };
//!     let thumb = pdf.thumb_with_options(options)?;
//!     std::fs::write("thumb.jpg", &thumb)?;
//!     Ok(())
//! }
//! ```

use std::fs;
use std::path::Path;
use thiserror::Error;
use windows::{
    core::GUID,
    Data::Pdf::{PdfDocument, PdfPageRenderOptions},
    Foundation,
    Storage::Streams::{DataReader, DataWriter, InMemoryRandomAccessStream},
};

mod guid;
use guid::*;

#[derive(Debug, Error)]
pub enum PdfThumbError {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("windows error")]
    WindowsError(#[from] windows::core::Error),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl From<Rect> for Foundation::Rect {
    fn from(r: Rect) -> Self {
        Self {
            X: r.x as _,
            Y: r.y as _,
            Width: r.width as _,
            Height: r.height as _,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Options {
    /// The destination width of the rendered page. If `width` is not specified, the page's aspect ratio is maintained relative to the destination height.
    pub width: u32,
    /// The destination height of the rendered page. If `height` is not specified, the page's aspect ratio is maintained relative to the destination width.
    pub height: u32,
    /// The portion of the PDF page to be rendered. If `rect` is not specified, the whole page is rendered.
    pub rect: Rect,
    /// The page index to be rendered. If `page` is not specified, the first page is rendered.
    pub page: u32,
    /// The image format of thumbnail. If `format` is not specified, PNG format is used.
    pub format: ImageFormat,
}

impl TryFrom<Options> for PdfPageRenderOptions {
    type Error = PdfThumbError;
    fn try_from(options: Options) -> Result<Self, Self::Error> {
        let op = PdfPageRenderOptions::new()?;
        if options.width > 0 {
            op.SetDestinationWidth(options.width)?;
        }
        if options.height > 0 {
            op.SetDestinationHeight(options.height)?;
        }
        if options.rect.ne(&Rect::default()) {
            op.SetSourceRect(options.rect.into())?;
        }
        op.SetBitmapEncoderId(options.format.guid())?;
        Ok(op)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Png,
    Bmp,
    Jpeg,
    Tiff,
    Gif,
}

impl Default for ImageFormat {
    fn default() -> Self {
        Self::Png
    }
}

impl ImageFormat {
    const fn guid(&self) -> GUID {
        use ImageFormat::*;
        match self {
            Png => PNG_ENCORDER_ID,
            Bmp => BITMAP_ENCODER_ID,
            Jpeg => JPEG_ENCORDER_ID,
            Tiff => TIFF_ENCODER_ID,
            Gif => GIF_ENCODER_ID,
        }
    }
}

#[derive(Debug)]
pub struct PdfThumb {
    doc: PdfDocument,
}

impl PdfThumb {
    /// Load a PDF document from memory.
    pub fn load(pdf: &[u8]) -> Result<Self, PdfThumbError> {
        let stream = InMemoryRandomAccessStream::new()?;
        let writer = DataWriter::CreateDataWriter(&stream)?;
        writer.WriteBytes(pdf)?;
        writer.StoreAsync()?.get()?;
        writer.FlushAsync()?.get()?;
        writer.DetachStream()?;
        let doc = PdfDocument::LoadFromStreamAsync(&stream)?.get()?;
        Ok(Self { doc })
    }

    /// Open a PDF document from a path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, PdfThumbError> {
        let file = fs::read(path)?;
        Self::load(&file)
    }

    /// Get the number of PDF document.
    pub fn page_count(&self) -> Result<u32, PdfThumbError> {
        Ok(self.doc.PageCount()?)
    }

    /// Generate a thumbnail image with default options.
    pub fn thumb(&self) -> Result<Vec<u8>, PdfThumbError> {
        let options = Options::default();
        self.thumb_with_options(options)
    }

    /// Generate a thumbnail image with the specified options.
    pub fn thumb_with_options(&self, options: Options) -> Result<Vec<u8>, PdfThumbError> {
        let page = self.doc.GetPage(options.page)?;
        let output = InMemoryRandomAccessStream::new()?;
        page.RenderWithOptionsToStreamAsync(&output, options.try_into().as_ref().ok())?
            .get()?;
        let input = output.GetInputStreamAt(0)?;
        let reader = DataReader::CreateDataReader(&input)?;
        let size = output.Size()?;
        reader.LoadAsync(size as u32)?.get()?;
        let mut buf = vec![0; size as usize];
        reader.ReadBytes(&mut buf)?;
        Ok(buf)
    }
}
