use crate::doc::djvu_dir::{File, FileType};
use crate::iff::data_pool::DataPool;
use crate::iff::iff::{IffWriter, IffWriterExt};
use crate::utils::error::Result;
use std::io::Cursor;
use std::sync::Arc;

/// Builder for the shared dictionary (glyph table) in DjVu documents.
#[derive(Default)]
pub struct SharedDictBuilder {
    // Stub; real implementation would collect glyphs.
}

impl SharedDictBuilder {
    pub fn new() -> Self {
        SharedDictBuilder::default()
    }

    /// Finalize and produce a (Arc<File>, DataPool) for directory insertion.
    pub fn finish(&self) -> Result<(Arc<File>, DataPool)> {
        let mut buf = Vec::new();
        {
            let mut writer = IffWriter::new(Cursor::new(&mut buf));
            writer.write_chunk(*b"FORM", b"DJVI")?;
            // Real impl would write glyph table here.
            writer.close_chunk()?;
            writer.close_chunk()?;
        }
        let file = File::new(
            "shared_dict_0",
            "shared_dict.djvi",
            "Shared Dictionary",
            FileType::Include,
        );
        Ok((file, DataPool::from_vec(buf)))
    }
}
