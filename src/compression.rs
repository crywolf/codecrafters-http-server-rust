pub mod gzip {
    use anyhow::Context;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    pub fn compress(content: &[u8]) -> anyhow::Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(content)
            .context("writing data to gzip encoder")?;
        encoder.finish().context("finishing encoding the stream")
    }
}
