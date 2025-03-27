//! This library is a thin wrapper of WinRT [PdfDocument Class](https://learn.microsoft.com/en-us/uwp/api/windows.data.pdf.pdfdocument?view=winrt-26100) to generate a thumbnail image for PDF.
//!
//! # Example
//!
//! ```rust
//! use anyhow::Result;
//! use pdf_thumb::PdfDoc;
//!
//! fn main() -> Result<()> {
//!     let pdf = PdfDoc::open("test.pdf")?;
//!     let thumb = pdf.thumb()?;
//!     std::fs::write("thumb.png", &thumb)?; // PNG is default.
//!     Ok(())
//! }
//! ```
//!
//! Some options and async operation are also available.
//!
//! ```rust
//! use anyhow::Result;
//! use pdf_thumb::{ImageFormat, Options, PdfDoc};
//!
//! #[tokio::main]
//! fn main() -> Result<()> {
//!     let pdf = PdfDoc::open_async("test.pdf").await?;
//!     let options = Options {
//!         width: 320,                // Set thumbnail image width.
//!         format: ImageFormat::Jpeg, // Set thumbnail image format.
//!         ..Default::default()
//!     };
//!     let thumb = pdf.thumb_with_options_async(options).await?;
//!     tokio::fs::write("thumb.jpg", &thumb).await?;
//!     Ok(())
//! }
//! ```
//!
//! - [crates.io](https://crates.io/crates/pdf-thumb)
//! - [Repository](https://github.com/zxrs/pdf-thumb)

#![cfg(target_os = "windows")]

use std::{
    ops::{Deref, DivAssign},
    path::Path,
};
use thiserror::Error;
use windows::{
    core::{GUID, HSTRING},
    Data::Pdf::{PdfDocument as PdfDocument_, PdfPage as PdfPage_, PdfPageRenderOptions},
    Foundation,
    Storage::{
        StorageFile,
        Streams::{DataReader, DataWriter, InMemoryRandomAccessStream},
    },
};
use windows_future::{IAsyncAction, IAsyncOperation};

mod guid;
use guid::*;

#[derive(Debug, Error)]
pub enum PdfThumbError {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("windows error")]
    Windows(#[from] windows::core::Error),
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

unsafe impl Send for Options {}
unsafe impl Sync for Options {}

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
pub struct PdfDocument {
    doc: PdfDocument_,
}

unsafe impl Send for PdfDocument {}
unsafe impl Sync for PdfDocument {}

impl PdfDocument {
    /// Load a PDF document from memory.
    pub fn load(pdf: &[u8]) -> Result<Self, PdfThumbError> {
        let stream = InMemoryRandomAccessStream::new()?;
        let writer = DataWriter::CreateDataWriter(&stream)?;
        writer.WriteBytes(pdf)?;
        writer.StoreAsync()?.get()?;
        writer.FlushAsync()?.get()?;
        writer.DetachStream()?;
        let doc = PdfDocument_::LoadFromStreamAsync(&stream)?.get()?;
        Ok(Self { doc })
    }

    /// Open a PDF document from a path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, PdfThumbError> {
        let file = get_file(path)?.get()?;
        let doc = open(&file)?.get()?;
        Ok(Self { doc })
    }

    /// Open a PDF document from a path asynchronously.
    pub async fn open_async<P: AsRef<Path>>(path: P) -> Result<Self, PdfThumbError> {
        let file = get_file(path)?.await?;
        let doc = open(&file)?.await?;
        Ok(Self { doc })
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

    /// Generate a thumbnail image with default options asynchronously.
    pub async fn thumb_async(&self) -> Result<Vec<u8>, PdfThumbError> {
        let options = Options::default();
        self.thumb_with_options_async(options).await
    }

    /// Generate a thumbnail image with the specified options.
    pub fn thumb_with_options(&self, options: Options) -> Result<Vec<u8>, PdfThumbError> {
        let page = self.get_page(options.page)?;
        let output = InMemoryRandomAccessStream::new()?;
        render(page, &output, options)?.get()?;
        read_bytes(output)
    }

    /// Generate a thumbnail image with the specified options asynchronously.
    pub async fn thumb_with_options_async(
        &self,
        options: Options,
    ) -> Result<Vec<u8>, PdfThumbError> {
        let page = self.get_page(options.page)?;
        let output = InMemoryRandomAccessStream::new()?;
        render(page, &output, options)?.await?;
        read_bytes(output)
    }

    pub fn get_page(&self, page_index: u32) -> Result<PdfPage, PdfThumbError> {
        let page = self.doc.GetPage(page_index)?;
        Ok(PdfPage::new(page))
    }
}

fn get_file<P: AsRef<Path>>(path: P) -> Result<IAsyncOperation<StorageFile>, PdfThumbError> {
    let path = HSTRING::from(path.as_ref());
    StorageFile::GetFileFromPathAsync(&path).map_err(Into::into)
}

fn open(file: &StorageFile) -> Result<IAsyncOperation<PdfDocument_>, PdfThumbError> {
    PdfDocument_::LoadFromFileAsync(file).map_err(Into::into)
}

fn render(
    page: PdfPage,
    output: &InMemoryRandomAccessStream,
    options: Options,
) -> Result<IAsyncAction, PdfThumbError> {
    page.RenderWithOptionsToStreamAsync(output, options.try_into().as_ref().ok())
        .map_err(Into::into)
}

fn read_bytes(output: InMemoryRandomAccessStream) -> Result<Vec<u8>, PdfThumbError> {
    let input = output.GetInputStreamAt(0)?;
    let reader = DataReader::CreateDataReader(&input)?;
    let size = output.Size()?;
    reader.LoadAsync(size as u32)?.get()?;
    let mut buf = vec![0; size as usize];
    reader.ReadBytes(&mut buf)?;
    Ok(buf)
}

#[derive(Debug)]
pub struct PdfPage {
    page: PdfPage_,
}

unsafe impl Sync for PdfPage {}
unsafe impl Send for PdfPage {}

impl Deref for PdfPage {
    type Target = PdfPage_;

    fn deref(&self) -> &Self::Target {
        &self.page
    }
}

impl PdfPage {
    pub fn new(page: PdfPage_) -> Self {
        Self { page }
    }

    pub fn size(&self) -> Result<Size, PdfThumbError> {
        Ok(self.page.Size()?.into())
    }
}

impl Drop for PdfPage {
    fn drop(&mut self) {
        self.page.Close().ok();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width() / self.height()
    }
}

impl From<Foundation::Size> for Size {
    fn from(value: Foundation::Size) -> Self {
        Self {
            width: value.Width,
            height: value.Height,
        }
    }
}
