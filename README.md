# pdf-thumb

This library is a thin wrapper of WinRT [PdfDocument Class](https://learn.microsoft.com/en-us/uwp/api/windows.data.pdf.pdfdocument?view=winrt-26100) to generate a thumbnail image for PDF.

# Example

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

Some options and async operation are also available.

```rust
use anyhow::Result;
use pdf_thumb::{ImageFormat, Options, PdfDoc};

#[tokio::main]
fn main() -> Result<()> {
    let pdf = PdfDoc::open_async("test.pdf").await?;
    let options = Options {
        width: 320,                // Set thumbnail image width.
        format: ImageFormat::Jpeg, // Set thumbnail image format.
        ..Default::default()
    };
    let thumb = pdf.thumb_with_options_async(options).await?;
    tokio::fs::write("thumb.jpg", &thumb).await?;
    Ok(())
}
```

- [crates.io](https://crates.io/crates/pdf-thumb)
- [Repository](https://github.com/zxrs/pdf-thumb)
