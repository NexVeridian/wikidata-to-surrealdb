use anyhow::{Error, Result};
use bzip2::read::MultiBzDecoder;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[allow(non_camel_case_types)]
pub enum File_Format {
    json,
    bz2,
}
impl File_Format {
    pub async fn new(file: &str) -> Self {
        match file {
            "json" => Self::json,
            "bz2" => Self::bz2,
            _ => panic!("Unknown file format"),
        }
    }
    pub async fn reader(self, file: &str) -> Result<Box<dyn BufRead>, Error> {
        let file = File::open(file)?;
        match self {
            File_Format::json => Ok(Box::new(BufReader::new(file))),
            File_Format::bz2 => Ok(Box::new(BufReader::new(MultiBzDecoder::new(file)))),
        }
    }
}
