#![feature(seek_stream_len)]

mod header;
mod error;

use header::{Header};
use std::path::{Path, PathBuf};

use error::Result;
use std::fs::OpenOptions;
use std::io::{Seek, Read, SeekFrom, Write};

use clap::{Arg, App, crate_authors, crate_version, crate_description};

fn bin_to_bmp<P: AsRef<Path>>(path: P, rename: bool) -> Result<()> {

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path.as_ref())?;

    let file_size = file.stream_len()?;

    //Create the bitmap and b2b headers
    let header = Header::new(file_size);

    // If the file is smaller than the combined bmp and b2b headers, then expand it
    if file_size < Header::total_header_size() as u64 {
        file.set_len(Header::total_header_size() as u64)?
    }

    // Make a copy of the beginning of the file
    let mut buffer: [u8; Header::total_header_size() as usize] = [0u8; Header::total_header_size() as usize];

    file.read(& mut buffer)?;

    // Add these copied bytes to the end of the file
    file.seek(SeekFrom::End(0))?;

    file.write_all(& buffer)?;

    //Copy the header to the beginning
    file.seek(SeekFrom::Start(0))?;

    bincode::serialize_into(& mut file, & header)?;

    //Resize to add padding
    file.set_len((header.pixmap_size() + Header::bitmap_header_size()) as u64)?;

    if rename {
        let renamed = PathBuf::from(format!("{}.bmp", path.as_ref().to_str().unwrap()));

        std::fs::rename(path.as_ref(), renamed)?;
    }

    Ok(())
}

fn bmp_to_bin<P: AsRef<Path>>(path: P, rename: bool) -> Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path.as_ref())?;

    // Load combined bitmap and b2b header
    let header: Header = bincode::deserialize_from(& file)?;

    header.check_id()?;

    header.check_padding_size()?;

    header.check_signature()?;

    //Create a buffer for the data at the end of the file (i.e. beginning of original file)
    let mut buffer: [u8; Header::total_header_size() as usize] = [0u8; Header::total_header_size() as usize];

    file.seek(SeekFrom::End(-(Header::total_header_size() as i64) - header.padding_size() as i64))?;

    file.read(& mut buffer)?;

    //Copy this buffer to the beginning
    file.seek(SeekFrom::Start(0))?;

    file.write_all(& buffer)?;

    //Resize the file back to its original size
    file.set_len(header.original_file_size() as u64)?;

    if rename {
        let path_str = path.as_ref().to_str().unwrap();

        let renamed = PathBuf::from(&path_str[..path_str.len() - 4]);

        std::fs::rename(path.as_ref(), renamed)?;
    }

    Ok(())
}

fn main() {
    let matches = App::new("B2B")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(Arg::new("path")
            .about("Path to a binary or bitmap file to convert. Converts non-bitmaps into bitmaps, and bitmaps back into non-bitmaps")
            .takes_value(true)
            .required(true)
            .validator(|path| {
                let path = Path::new(path);

                if path.exists() {
                    if OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(path)
                        .is_ok() {
                        Ok(())
                    } else {
                        Err(String::from("Cannot read or write to specified file."))
                    }
                } else {
                    Err(String::from("The specified path does not exist."))
                }
            }))
        .get_matches();

    let path = matches.value_of("path").unwrap();

    let extension = &path[path.len() - 4..];

    if extension == ".bmp" {
        bmp_to_bin(path, true).unwrap();
    } else {
        bin_to_bmp(path, true).unwrap();
    }
}
