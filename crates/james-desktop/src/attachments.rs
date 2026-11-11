//! Files explicitly selected for a single chat question. They are never persisted.
use std::io::Read;
use std::path::Path;

const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 24 * 1024;
const MAX_PIXELS: u64 = 40_000_000;

#[derive(Debug, Clone)]
pub struct Attachment {
    pub name: String,
    pub content: Content,
}

#[derive(Debug, Clone)]
pub enum Content {
    Text(String),
    Image(Vec<u8>),
}

pub fn load(path: &Path) -> Result<Attachment, String> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Choose a file with a valid name.")?
        .to_owned();
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let file = std::fs::File::open(path).map_err(|e| format!("Could not open {name}: {e}"))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Choose a regular file.".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Could not read {name}: {e}"))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("Attachments must be 16 MiB or smaller.".into());
    }
    let content = match extension.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => {
            let reader = image::ImageReader::new(std::io::Cursor::new(&bytes))
                .with_guessed_format()
                .map_err(|e| format!("Could not read image: {e}"))?;
            let (width, height) = reader
                .into_dimensions()
                .map_err(|e| format!("Could not read image dimensions: {e}"))?;
            if u64::from(width) * u64::from(height) > MAX_PIXELS {
                return Err("Image exceeds 40 million pixels.".into());
            }
            let image = image::load_from_memory(&bytes)
                .map_err(|e| format!("Could not decode image: {e}"))?;
            let image = image.thumbnail(1600, 1600);
            let mut png = std::io::Cursor::new(Vec::new());
            image
                .write_to(&mut png, image::ImageFormat::Png)
                .map_err(|e| format!("Could not prepare image: {e}"))?;
            Content::Image(png.into_inner())
        }
        "pdf" => {
            let text = pdf_extract::extract_text_from_mem(&bytes)
                .map_err(|e| format!("Could not extract PDF text: {e}"))?;
            if text.trim().is_empty() {
                return Err("This PDF has no extractable text. Scanned pages need OCR.".into());
            }
            Content::Text(checked_text(text)?)
        }
        "txt" | "md" | "csv" | "tsv" | "json" | "xml" | "html" | "rs" | "py" | "js" | "ts"
        | "toml" | "yaml" | "yml" | "log" => {
            let text =
                String::from_utf8(bytes).map_err(|_| "The selected text file is not UTF-8.")?;
            Content::Text(checked_text(text)?)
        }
        _ => return Err("Choose a PDF, UTF-8 text file, PNG, JPEG, or WebP image.".into()),
    };
    Ok(Attachment { name, content })
}

fn checked_text(text: String) -> Result<String, String> {
    if text.len() > MAX_TEXT_BYTES {
        return Err("Extracted text exceeds 24 KiB; select a shorter document.".into());
    }
    if text.contains('\0') {
        return Err("This file contains binary data, not plain text.".into());
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_excessive_or_binary_text() {
        assert!(checked_text("x".repeat(MAX_TEXT_BYTES + 1)).is_err());
        assert!(checked_text("hello\0world".into()).is_err());
        assert_eq!(checked_text("hello".into()).unwrap(), "hello");
    }
    #[test]
    fn rejects_unknown_file_type() {
        let path =
            std::env::temp_dir().join(format!("james-attachment-{}.exe", std::process::id()));
        std::fs::write(&path, b"test").unwrap();
        assert!(load(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn extracts_text_from_a_real_pdf() {
        let stream = "BT /F1 18 Tf 72 700 Td (Hello from PDF) Tj ET";
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_string(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
            format!("<< /Length {} >>\nstream\n{}\nendstream", stream.len(), stream),
        ];
        let mut pdf = String::from("%PDF-1.4\n");
        let mut offsets = Vec::new();
        for (i, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.push_str(&format!("{} 0 obj\n{}\nendobj\n", i + 1, object));
        }
        let xref = pdf.len();
        pdf.push_str("xref\n0 6\n0000000000 65535 f \n");
        for offset in offsets {
            pdf.push_str(&format!("{offset:010} 00000 n \n"));
        }
        pdf.push_str(&format!(
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
        ));
        let path =
            std::env::temp_dir().join(format!("james-attachment-{}.pdf", std::process::id()));
        std::fs::write(&path, pdf).unwrap();
        let file = load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        let Content::Text(text) = file.content else {
            panic!("expected text")
        };
        assert!(text.contains("Hello from PDF"));
    }
}
