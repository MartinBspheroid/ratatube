/// Decode artwork under strict dimensions and allocation limits.
pub fn decode_thumbnail(bytes: &[u8]) -> image::ImageResult<image::DynamicImage> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    reader.decode()
}

#[cfg(test)]
mod tests {
    use super::decode_thumbnail;

    #[test]
    fn thumbnail_decoder_rejects_dimensions_above_limit() {
        let image = image::DynamicImage::new_rgba8(4097, 1);
        let mut encoded = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut encoded, image::ImageFormat::Png)
            .expect("encode png");

        assert!(decode_thumbnail(encoded.get_ref()).is_err());
    }
}
