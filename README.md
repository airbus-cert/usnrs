# usnrs - USN Journal parser

This project is a parser for [`$UsnJrnl:$J`](https://en.wikipedia.org/wiki/USN_Journal) files, which tracks file system changes at the file level. It only handles [USN_RECORD_V2](https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ns-winioctl-usn_record_v2) records, which are the most common in our experience.

The tool can also optionally use an [MFT](https://en.wikipedia.org/wiki/NTFS#Master_File_Table) file to try to reconstruct full paths to files mentioned in the journal.

Other similar projects:

- [PoorBillionaire/USN-Journal-Parser](https://github.com/PoorBillionaire/USN-Journal-Parser) (Python)
- [jschicht/UsnJrnl2Csv](https://github.com/jschicht/UsnJrnl2Csv) (AutoIt)

This one is written in Rust, and exposes both a binary (`usnrs-cli`) to process USN Journal files, and a library (`usnrs`) which can be used in other applications.

## Installation

You will need a [Rust](https://rust-lang.org) compiler and the [Cargo](https://github.com/rust-lang/cargo) package manager to build this project. Both can be automatically installed by [rustup](https://rustup.rs).

Alternatively, you can use `nix-shell` to automatically setup all this.

Building the project is as simple as running:

```
$ cargo build --features=usnrs-cli --release
```

## Usage

The `usnrs-cli` binary can be used to parse `$UsnJrnl:$J` files and output the extracted USN records. The USN Journal file is a [sparse file](https://en.wikipedia.org/wiki/Sparse_file), which means that it is usually mostly empty (filled with `0x00` bytes).

Some forensics acquisition tools are able to only extract the non-empty data, while others acquire the full, mostly empty file. `usnrs-cli` supports both types of files.

### Basic usage

```
$ usnrs-cli PATH-TO-USNJRNL-J
```

Outputs the file in a format similar to [USN-Journal-Parser](https://github.com/PoorBillionaire/USN-Journal-Parser), `Timestamp | Filename | Attributes | Reasons`.

### Bodyfile format output

```
$ usnrs-cli -f bodyfile PATH-TO-USNJRNL-J
```

Outputs the file in [Body file format (version 3.X)](https://wiki.sleuthkit.org/index.php?title=Body_file), for use with `mactime` or other tools.

### Full path reconstruction

If you also have the Master File Table file for the disk from which you extracted the USN Journal, you can give it as an option to reconstruct the full path to each file.

```
$ usnrs-cli --mft PATH-TO-MFT PATH-TO-USNJRNL-J
```

Path reconstruction is based on the MFT entry number stored in the USN record. Simple checks are in place to prevent the tool from giving out false paths when dealing with reallocated entry numbers (when dealing with deleted files for example).

### Specifying the start offset manually

In order to deal with full sparse files more quickly, `usnrs-cli` starts scanning for the beginning of the list of records from the end of the given file. While this seemed pretty robust during our tests, this may give out false start offsets, resulting in wrongly parsed entries.

If you observe this behavior, you can specify the start offset of the first record using the `--start` argument. This offset can be found by looking at the file in a hex editor and manually searching for the first record.

```
$ usnrs-cli --start OFFSET PATH-TO-USNJRNL-J
```