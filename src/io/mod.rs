use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use jpeg_decoder;
use png::HasParameters;

use crate::error::{ImgIoError, ImgIoResult};
use crate::image::{Image, BaseImage};

/// Converts a `png::ColorType` to a tuple representing the number of channels in a png image
/// and if the image has an alpha channel or not
fn png_from_color_type(color_type: png::ColorType) -> ImgIoResult<(u8, bool)> {
    match color_type {
        png::ColorType::Grayscale => Ok((1, false)),
        png::ColorType::GrayscaleAlpha => Ok((2, true)),
        png::ColorType::RGB => Ok((3, false)),
        png::ColorType::RGBA => Ok((4, true)),
        png::ColorType::Indexed => Err(ImgIoError::UnsupportedImageFormat("png::ColorType::Indexed not supported".to_string())), // TODO: Add support for this
    }
}

/// Converts the number of channels in a png image to a `png::ColorType`
fn png_into_color_type(channels: u8) -> ImgIoResult<png::ColorType> {
    match channels {
        1 => Ok(png::ColorType::Grayscale),
        2 => Ok(png::ColorType::GrayscaleAlpha),
        3 => Ok(png::ColorType::RGB),
        4 => Ok(png::ColorType::RGBA),
        _ => Err(ImgIoError::UnsupportedImageFormat("invalid number of channels".to_string())), // TODO: Add png::ColorType::Indexed
    }
}

/// Decodes a png image
fn decode_png(filename: &str) -> ImgIoResult<Image<u8>> {
    let decoder = png::Decoder::new(File::open(filename)?);
    let (info, mut reader) = decoder.read_info()?;
    let mut buf = vec![0; info.buffer_size()];
    reader.next_frame(&mut buf)?;

    let (channels, alpha) = png_from_color_type(info.color_type)?;

    Ok(Image::new(info.width, info.height, channels, alpha, &buf))
}

/// Encodes a png image
fn encode_png(input: &Image<u8>, path: &Path) -> ImgIoResult<()> {
    let (width, height, channels) = input.info().whc();
    let file = File::create(path)?;
    let ref mut file_writer = BufWriter::new(file);

    let mut encoder = png::Encoder::new(file_writer, width, height);
    let color_type = png_into_color_type(channels)?;
    encoder.set(color_type).set(png::BitDepth::Eight);

    let mut png_writer = encoder.write_header()?;
    png_writer.write_image_data(input.data())?;

    Ok(())
}

/// Converts a `jpeg_decoder::PixelFormat` to the number of channels in a jpg image
pub fn jpg_pixel_format_to_channels(pixel_format: jpeg_decoder::PixelFormat) -> u8 {
    match pixel_format {
        jpeg_decoder::PixelFormat::L8 => 1,
        jpeg_decoder::PixelFormat::RGB24 => 3,
        jpeg_decoder::PixelFormat::CMYK32 => 4,
    }
}

/// Decodes a jpg image
fn decode_jpg(filename: &str) -> ImgIoResult<Image<u8>> {
    let file = File::open(filename)?;
    let mut decoder = jpeg_decoder::Decoder::new(BufReader::new(file));
    let pixels = decoder.decode()?;
    let info = decoder.info().ok_or_else(|| ImgIoError::Other("unable to read metadata".to_string()))?;
    let channels = jpg_pixel_format_to_channels(info.pixel_format);
    Ok(Image::new(info.width as u32, info.height as u32, channels, false, &pixels))
}

// TODO: Add support for jpg encoding
// fn encode_jpg(input: &Image<u8>, filename: &str) -> ImgIoResult<(), ImageError> {
//
// }

// TODO: Add support for more image file formats

/// Reads a png or jpg image file into an `Image<u8>`
pub fn read(filename: &str) -> ImgIoResult<Image<u8>> {
    let path = Path::new(filename);
    let ext = path.extension().ok_or_else(|| ImgIoError::Other("could not extract file extension".to_string()))?;
    let ext_str = ext.to_str().ok_or_else(|| ImgIoError::Other("invalid file extension".to_string()))?;

    match ext_str.to_ascii_lowercase().as_str() {
        "png" => Ok(decode_png(filename)?),
        "jpg" | "jpeg" => Ok(decode_jpg(filename)?),
        x => Err(ImgIoError::UnsupportedFileFormat(format!("{} is not supported", x))),
    }
}

/// Writes an `Image<u8>` into a png file
pub fn write(input: &Image<u8>, filename: &str) -> ImgIoResult<()> {
    let path = Path::new(filename);
    let ext = path.extension().ok_or_else(|| ImgIoError::Other("could not extract file extension".to_string()))?;
    let ext_str = ext.to_str().ok_or_else(|| ImgIoError::Other("invalid file extension".to_string()))?;

    match ext_str.to_ascii_lowercase().as_str() {
        "png" => Ok(encode_png(input, path)?),
        // "jpg" | "jpeg" => Ok(encode_jpg(input, filename)?),
        x => Err(ImgIoError::UnsupportedFileFormat(format!("{} is not supported", x))),
    }
}