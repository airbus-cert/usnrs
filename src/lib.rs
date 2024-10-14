extern crate binrw;
extern crate mft;

use binrw::io::Read;
use binrw::{binrw, BinRead};
use bitflags::bitflags;
use chrono::NaiveDateTime;

use mft::MftParser;
use std::fmt;
use std::io;
use Error::BadVersion;

#[binrw(little)]
#[derive(Debug)]
pub struct Entry {
    entry_size: u32,

    major: u16,
    minor: u16,

    file_ref: u64,
    parent_file_ref: u64,
    pub usn: u64,
    pub timestamp: u64,
    reason: u32,
    source_info: u32,
    security_id: u32,
    file_attributes: u32,

    filename_length: u16, // number of bytes
    filename_offset: u16,

    #[br(count = filename_length/2, pad_size_to = entry_size - 60)]
    filename: Vec<u16>,
}

bitflags! {
    pub struct Reason: u32 {
        const DataOverwrite = 0x1;
        const DataExtend = 0x2;
        const DataTruncation = 0x4;
        const NamedDataOverwrite = 0x10;
        const NamedDataExtend = 0x20;
        const NamedDataTruncation = 0x40;
        const FileCreate = 0x100;
        const FileDelete = 0x200;
        const EaChange = 0x400;
        const SecurityChange = 0x800;
        const RenameOldName = 0x1000;
        const RenameNewName = 0x2000;
        const IndexableChange = 0x4000;
        const BasicInfoChange = 0x8000;
        const HardLinkChange = 0x10000;
        const CompressionChange = 0x20000;
        const EncryptionChange = 0x40000;
        const ObjectIdChange = 0x80000;
        const ReparsePointChange = 0x100000;
        const StreamChange = 0x200000;
        const Close = 0x80000000;
    }
}

bitflags! {
    pub struct Attributes: u32 {
        const ReadOnly = 0x1;
        const Hidden = 0x2;
        const System = 0x4;
        const Directory = 0x10;
        const Archive = 0x20;
        const Device = 0x40;
        const Normal = 0x80;
        const Temporary = 0x100;
        const SparseFile = 0x200;
        const ReparsePoint = 0x400;
        const Compressed = 0x800;
        const Offline = 0x1000;
        const NotContentIndexed = 0x2000;
        const Encrypted = 0x4000;
        const IntegrityStream = 0x8000;
        const Virtual = 0x10000;
        const NoScrubData = 0x20000;
    }
}

#[derive(Debug)]
pub enum Error {
    Mft(mft::err::Error),
    Io(std::io::Error),
    BinRw(binrw::Error),
    BadVersion(String),
}

impl From<mft::err::Error> for Error {
    fn from(value: mft::err::Error) -> Self {
        Error::Mft(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<binrw::Error> for Error {
    fn from(value: binrw::Error) -> Self {
        Error::BinRw(value)
    }
}

impl Entry {
    pub fn new<R: Read + io::Seek>(reader: &mut R) -> Result<Self, Error> {
        let e = Entry::read_le(reader)?;
        if e.major != 2 || e.minor != 0 {
            return Err(BadVersion(format!(
                "Entry version mismatch: expected 2.0, got {}.{}",
                e.major, e.minor
            )));
        }

        return Ok(e);
    }

    pub fn filename(&self) -> String {
        String::from_utf16_lossy(&self.filename[..])
    }

    pub fn reasons(&self) -> Reason {
        Reason::from_bits_truncate(self.reason)
    }

    pub fn attributes(&self) -> Attributes {
        Attributes::from_bits_truncate(self.file_attributes)
    }

    pub fn unix_timestamp(&self) -> i64 {
        (self.timestamp as i64) / 10000000 - 11644473600
    }

    pub fn time(&self) -> NaiveDateTime {
        let unix = self.unix_timestamp();
        NaiveDateTime::from_timestamp_opt(unix, 0)
            .unwrap_or_else(|| panic!("timestamp: could not parse {}", unix))
    }

    pub fn mft_entry_num(&self) -> u64 {
        self.file_ref & 0xFFFFFFFFFFFF
    }

    pub fn sequence_num(&self) -> u64 {
        (self.file_ref >> 48) & 0xFFFF
    }

    pub fn parent_mft_entry_num(&self) -> u64 {
        self.parent_file_ref & 0xFFFFFFFFFFFF
    }

    pub fn parent_sequence_num(&self) -> u64 {
        (self.parent_file_ref >> 48) & 0xFFFF
    }
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<String> = self.iter_names().map(|(s, _)| String::from(s)).collect();

        f.write_str(names.join(" ").as_str())?;

        Ok(())
    }
}

impl fmt::Display for Attributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<String> = self.iter_names().map(|(s, _)| String::from(s)).collect();

        f.write_str(names.join(" ").as_str())?;

        Ok(())
    }
}

pub trait Skip {
    fn find_first_record(&mut self) -> io::Result<bool>;
    fn find_next_record(&mut self) -> io::Result<bool>;
}

impl<T: Read + io::Seek> Skip for T {
    fn find_first_record(&mut self) -> io::Result<bool> {
        let mut buf = vec![0_u8; 65535];
        loop {
            let n = self.read(&mut buf)?;
            let idx = buf[0..n].iter().position(|x| *x != 0);

            if let Some(x) = idx {
                let off = (n as i64) - (x as i64);
                self.seek(io::SeekFrom::Current(-off))?;
                return Ok(true);
            }
        }
    }

    fn find_next_record(&mut self) -> io::Result<bool> {
        let mut buf: [u8; 4] = [0; 4];
        loop {
            let n = self.read(&mut buf)?;
            if n != 4 {
                return Ok(false);
            } else if u32::from_le_bytes(buf) != 0 {
                self.seek(io::SeekFrom::Current(-4))?;
                return Ok(true);
            }
        }
    }
}

pub struct Usn<T, U>
where
    T: io::Read + io::Seek,
    U: io::Read + io::Seek,
{
    mft: Option<MftParser<T>>,
    usn: U,
}

impl<T, U> Usn<T, U>
where
    T: io::Read + io::Seek,
    U: io::Read + io::Seek,
{
    pub fn new(mft: Option<MftParser<T>>, mut usn: U, offset: Option<u64>) -> Result<Self, Error> {
        if let Some(off) = offset {
            usn.seek(io::SeekFrom::Start(off))?;
        }

        usn.find_first_record()?;
        println!("Found first record at offset {}", usn.stream_position()?);

        Ok(Self { mft, usn })
    }
}

impl Usn<std::io::BufReader<std::fs::File>, std::fs::File> {
    pub fn from_usn_with_mft(
        usn_path: &str,
        offset: Option<u64>,
        mft_path: &str,
    ) -> Result<Self, Error> {
        let mft = MftParser::from_path(mft_path)?;
        let usn = std::fs::File::open(usn_path)?;
        Usn::new(Some(mft), usn, offset)
    }
}

impl<T> Usn<T, std::fs::File>
where
    T: io::Read + io::Seek,
{
    pub fn from_usn(usn_path: &str, offset: Option<u64>) -> Result<Self, Error> {
        let usn = std::fs::File::open(usn_path)?;
        Usn::new(None, usn, offset)
    }
}

impl<T, U> Iterator for Usn<T, U>
where
    T: io::Read + io::Seek,
    U: io::Read + io::Seek,
{
    type Item = (String, Entry);
    fn next(&mut self) -> Option<Self::Item> {
        if !self.usn.find_next_record().ok()? {
            println!("no more records");
            return None;
        }

        let entry = Entry::new(&mut self.usn).unwrap_or_else(|err| panic!("error building entry: {:?}", err));
        let mut filename = entry.filename();

        if let Some(mft) = &mut self.mft {
            let e = mft.get_entry(entry.mft_entry_num()).ok()?;
            let name_in_mft = match e.find_best_name_attribute() {
                Some(attr) => attr.name,
                None => String::from(""),
            };

            if name_in_mft == entry.filename() {
                if let Ok(Some(fp)) = mft.get_full_path_for_entry(&e) {
                    filename = fp
                        .into_os_string()
                        .into_string()
                        .unwrap_or(entry.filename())
                }
            }
        }

        Some((filename, entry))
    }
}
