# pdf-thumb

This library is a thin wrapper of WinRT [PdfDocumet Class](https://learn.microsoft.com/en-us/uwp/api/windows.data.pdf.pdfdocument?view=winrt-26100) to generate a thumbnail image for PDF.

## Example

```
use anyhow::Result;
use pdf_thumb::PdfThumb;

fn main() -> Result<()> {
    let pdf = PdfThumb::open("test.pdf")?;
    let thumb = pdf.thumb()?;
    std::fs::write("thumb.png", &thumb)?; // PNG is default.
    Ok(())
}
```

Some options are also available.

```
use anyhow::Result;
use pdf_thumb::{ImageFormat, Options, PdfThumb};

fn main() -> Result<()> {
    let pdf = PdfThumb::open("test.pdf")?;
    let options = Options {
        width: 320,                // Set thumbnail image width.
        format: ImageFormat::Jpeg, // Set thumbnail image format.
        ..Default::default()
    };
    let thumb = pdf.thumb_with_options(options)?;
    std::fs::write("thumb.jpg", &thumb)?;
    Ok(())
}
```
