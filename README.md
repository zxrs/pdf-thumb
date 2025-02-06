# pdf-thumb

This library is a thin wrapper of WinRT [PdfDocument Class](https://learn.microsoft.com/en-us/uwp/api/windows.data.pdf.pdfdocument?view=winrt-26100) to generate a thumbnail image for PDF.

## Example

```rust
use anyhow::Result;
use pdf_thumb::PdfDoc;

fn main() -> Result<()> {
    let pdf = PdfDoc::open("test.pdf")?;
    let thumb = pdf.thumb()?;
    std::fs::write("thumb.png", &thumb)?; // PNG is default.
    Ok(())
}
```

Some options are also available.

```rust
use anyhow::Result;
use pdf_thumb::{ImageFormat, Options, PdfDoc};

fn main() -> Result<()> {
    let pdf = PdfDoc::open("test.pdf")?;
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

- [crates.io](https://crates.io/crates/pdf-thumb)
- [Documentation](https://zxrs.github.io/pdf-thumb-docs/pdf_thumb/)
