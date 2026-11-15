//! Files explicitly selected for a single chat question. They are never persisted.
//! Selected contents are frozen once loaded; the file on disk is never reread.
use std::io::Read;
use std::path::Path;

const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 24 * 1024;
const MAX_PIXELS: u64 = 40_000_000;

/// At most this many files may be selected for one question.
pub const MAX_FILES: usize = 8;
/// The extracted text of all selected files together.
pub const MAX_TOTAL_TEXT_BYTES: usize = 96 * 1024;
/// All prepared images together.
pub const MAX_TOTAL_IMAGE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Attachment {
    /// Unique within one collection, so two files with the same name can be
    /// removed independently.
    pub id: u32,
    pub name: String,
    pub content: Content,
}

#[derive(Debug, Clone)]
pub enum Content {
    Text(String),
    Image(Vec<u8>),
}

impl Content {
    /// The number of bytes this item counts toward its aggregate limit.
    fn weight(&self) -> usize {
        match self {
            Content::Text(text) => text.len(),
            Content::Image(png) => png.len(),
        }
    }
}

pub fn load(path: &Path) -> Result<Attachment, String> {
    load_with_id(path, 0)
}

/// Loads one file and stamps it with `id`. Loading never touches an existing
/// collection; callers append the result only on success.
fn load_with_id(path: &Path, id: u32) -> Result<Attachment, String> {
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
    if let Content::Text(text) = &content
        && text.trim().is_empty()
    {
        return Err("This file has no text content.".into());
    }
    Ok(Attachment { id, name, content })
}

/// The ordered files selected for one question. Contents are frozen at load
/// time; a rejected file never disturbs the items already selected.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    items: Vec<Attachment>,
    next_id: u32,
}

impl Selection {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Attachment> {
        self.items.iter()
    }

    /// Removes the item with this ID, whichever position it occupies. Items
    /// sharing a name are removed one at a time because their IDs differ.
    pub fn remove(&mut self, id: u32) -> Option<Attachment> {
        let index = self.items.iter().position(|item| item.id == id)?;
        Some(self.items.remove(index))
    }

    /// Loads `path` and appends it, or explains why it was refused. On
    /// refusal the selection is unchanged, including earlier accepted files.
    pub fn append(&mut self, path: &Path) -> Result<(), String> {
        self.add(load(path)?)
    }

    /// Appends an already loaded file, re-stamping its ID so IDs stay unique
    /// and ordered within the selection. The same limits as [`Selection::append`]
    /// apply; on refusal the selection is unchanged.
    pub fn add(&mut self, attachment: Attachment) -> Result<(), String> {
        if self.items.len() >= MAX_FILES {
            return Err(format!(
                "At most {MAX_FILES} files can be selected for one question; remove one first."
            ));
        }
        let total = |kind: fn(&Content) -> bool| {
            self.items
                .iter()
                .filter(|item| kind(&item.content))
                .map(|item| item.content.weight())
                .sum::<usize>()
        };
        match &attachment.content {
            Content::Text(text) => {
                if total(|c| matches!(c, Content::Text(_))) + text.len() > MAX_TOTAL_TEXT_BYTES {
                    return Err("The selected text files together exceed 96 KiB. Remove one or choose shorter files.".into());
                }
            }
            Content::Image(png) => {
                if total(|c| matches!(c, Content::Image(_))) + png.len() > MAX_TOTAL_IMAGE_BYTES {
                    return Err("The selected images together exceed 16 MiB. Remove one or use smaller images.".into());
                }
            }
        }
        let mut attachment = attachment;
        attachment.id = self.next_id;
        self.items.push(attachment);
        self.next_id += 1;
        Ok(())
    }

    /// All text items, in selection order.
    pub fn texts(&self) -> impl Iterator<Item = &Attachment> {
        self.items
            .iter()
            .filter(|item| matches!(item.content, Content::Text(_)))
    }

    /// All image items, in selection order.
    pub fn images(&self) -> impl Iterator<Item = &Attachment> {
        self.items
            .iter()
            .filter(|item| matches!(item.content, Content::Image(_)))
    }

    pub fn has_images(&self) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item.content, Content::Image(_)))
    }

    /// Rebuilds a selection from already loaded items, in order, re-stamping
    /// their IDs. The same limits as [`Selection::append`] apply; on refusal
    /// the selection is unchanged.
    pub fn from_parts(items: Vec<Attachment>) -> Result<Self, String> {
        let mut selection = Selection::default();
        for item in items {
            selection.add(item)?;
        }
        Ok(selection)
    }
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

    /// One directory shared by this test run, so parallel tests with distinct
    /// file names never collide while the names stay exactly as asserted.
    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("james-selection-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn temp_text(name: &str, text: &str) -> std::path::PathBuf {
        let path = temp_dir().join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    fn png_bytes(color: [u8; 4]) -> Vec<u8> {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(4, 3, image::Rgba(color)))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        png.into_inner()
    }

    fn temp_png(name: &str, color: [u8; 4]) -> std::path::PathBuf {
        let path = temp_dir().join(name);
        std::fs::write(&path, png_bytes(color)).unwrap();
        path
    }

    #[test]
    fn selection_accepts_the_bound_and_rejects_one_more() {
        let mut selection = Selection::default();
        for i in 0..MAX_FILES {
            let path = temp_text(&format!("a{i}.txt"), "hello");
            selection.append(&path).unwrap();
            std::fs::remove_file(path).unwrap();
        }
        assert_eq!(selection.len(), MAX_FILES);
        let path = temp_text("extra.txt", "hi");
        assert!(selection.append(&path).is_err());
        std::fs::remove_file(path).unwrap();
        assert_eq!(selection.len(), MAX_FILES, "the rejected extra is not kept");
    }

    #[test]
    fn a_rejected_sibling_preserves_earlier_files() {
        let mut selection = Selection::default();
        let good = temp_text("good.txt", "kept");
        selection.append(&good).unwrap();
        let empty = temp_text("empty.txt", "");
        let error = selection.append(&empty).unwrap_err();
        std::fs::remove_file(good).unwrap();
        std::fs::remove_file(empty).unwrap();
        assert!(error.contains("no text content"), "{error}");
        assert_eq!(selection.len(), 1);
        assert!(selection.texts().next().unwrap().name == "good.txt");
    }

    #[test]
    fn equal_basenames_are_removed_independently() {
        let first = temp_png("one.png", [1, 2, 3, 255]);
        let second = temp_png("two.png", [7, 8, 9, 255]);
        let mut selection = Selection::default();
        selection.append(&first).unwrap();
        selection.append(&second).unwrap();
        std::fs::rename(&first, second.parent().unwrap().join("same.png")).unwrap();
        // The loaded bytes are frozen: the rename does not affect anything.
        // Small imports are upscaled toward 1600 pixels for the model, so the
        // stored PNG is checked by decoding it, not by comparing raw bytes.
        let ids: Vec<u32> = selection.iter().map(|item| item.id).collect();
        assert_eq!(ids.len(), 2);
        let removed = selection.remove(ids[1]).unwrap();
        let Content::Image(bytes) = removed.content else {
            panic!("expected image")
        };
        let decoded = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (1600, 1200));
        assert_eq!(decoded.get_pixel(0, 0).0, [7, 8, 9, 255]);
        assert_eq!(selection.len(), 1);
        assert_eq!(
            selection.iter().next().unwrap().name,
            "one.png",
            "the loaded name is frozen; the disk rename does not reach it"
        );
    }

    #[test]
    fn aggregate_text_bound_is_enforced_across_files() {
        let mut selection = Selection::default();
        // Each file alone is under its own 24 KiB cap; four of them fill the
        // 96 KiB total, and one more byte anywhere is refused.
        let mut paths = Vec::new();
        for name in ["big.txt", "more.txt", "even-more.txt", "last.txt"] {
            let path = temp_text(name, &"x".repeat(MAX_TEXT_BYTES));
            selection.append(&path).unwrap();
            paths.push(path);
        }
        let over = temp_text("over.txt", "z");
        assert!(selection.append(&over).is_err());
        assert_eq!(
            selection.iter().map(|i| i.content.weight()).sum::<usize>(),
            MAX_TOTAL_TEXT_BYTES
        );
        for path in paths {
            std::fs::remove_file(path).unwrap();
        }
        std::fs::remove_file(over).unwrap();
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
