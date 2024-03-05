use std::fs::File;
use std::io;
use std::io::{Read, Write, BufRead, BufReader, Seek, SeekFrom};
use csv::{ReaderBuilder, StringRecord, Reader};
use evalexpr::{eval_boolean_with_context, HashMapContext, ContextWithMutableVariables, Value};

// use polars_core::prelude::*;
// use polars_io::prelude::*;

// use polars::prelude::*;
// use polars::prelude::{CsvReader, DataFrame, SerReader};
// use polars_io::RowIndex;
// use polars::prelude::DataFrame;

pub struct Pfile {
    pub pfile_prefix: String,
    pub num_variants: u32,
    pub num_samples: u32,
}

impl Pfile {
    pub fn pgen_path(&self) -> String {
        format!("{}.pgen", self.pfile_prefix)
    }

    pub fn psam_path(&self) -> String {
        format!("{}.psam", self.pfile_prefix)
    }

    pub fn pvar_path(&self) -> String {
        format!("{}.pvar", self.pfile_prefix)
    }

    pub fn from_prefix(pfile_prefix: String) -> Pfile {
        let pgen_path = format!("{}.pgen", pfile_prefix);

        let pgen = File::open(&pgen_path).unwrap();
        let mut pgen_reader = BufReader::new(pgen);

        let mut buf = [0u8; 2];
        pgen_reader.read_exact(&mut buf).unwrap();
        // check the magic number
        assert_eq!(buf, [0x6C, 0x1B]);

        let mut buf = [0u8; 1];
        pgen_reader.read_exact(&mut buf).unwrap();
        let storage_mode = buf[0];
        // for now we only deal with the simplest fixed-width storage mode
        assert!(storage_mode == 0x02);
        
        let mut buf = [0u8; 4];
        pgen_reader.read_exact(&mut buf).unwrap();
        let num_variants = u32::from_le_bytes(buf);
        println!("variant_count: {}", num_variants);

        let mut buf = [0u8; 4];
        pgen_reader.read_exact(&mut buf).unwrap();
        let num_samples = u32::from_le_bytes(buf);
        println!("sample_count: {}", num_samples);
        
        let mut buf = [0u8; 1];
        pgen_reader.read_exact(&mut buf).unwrap();
        // print this bit in binary
        // println!("flags: {:08b}", buf[0]);
        assert_eq!(buf, [0x40]);

        Pfile {
            pfile_prefix,
            num_variants,
            num_samples,
        }
    }

    pub fn output_vcf(&self, sample_ids: Vec<String>, variant_ids: Vec<String>) {
        let (pvar_header, pvar_column_names) = self.read_pvar_header();
        let (variant_ids, variant_idxs, variant_lines) = self.filter_pvar(variant_ids);
        let (sample_ids, sample_idxs) = self.filter_psam(sample_ids);

        let output_vcf_path = format!("{}.pgen-rs.vcf", self.pfile_prefix);
        let mut vcf = File::create(output_vcf_path).unwrap();
        // write the header
        write!(vcf, "##fileformat=VCFv4.2\n").unwrap();
        write!(vcf, "##source=pgen-rs\n").unwrap();
        write!(vcf, "{}", pvar_header).unwrap();
    
        let mut pvar_column_names = pvar_column_names.trim().to_string();
        // pvar_column_names.push_str("\tQUAL\tFILTER\tINFO\tFORMAT\t");
        pvar_column_names.push_str("\tFORMAT\t");
        // join the sample ids with tabs and add a newline
        pvar_column_names.push_str(&sample_ids.join("\t"));
        pvar_column_names.push_str("\n");
        write!(vcf, "{}", pvar_column_names).unwrap();

        // now the fun part, write the actual data
        let pgen = File::open(&self.pgen_path()).unwrap();
        let mut pgen_reader = BufReader::new(pgen);
        for (i, variant_idx) in variant_idxs.iter().enumerate() {
            let mut variant_line = variant_lines[i].trim().to_string();
            variant_line.push_str("\tGT");
            // variant_line.push_str("\t.\t.\t.\tGT");

            let record_offset = 12 + (variant_idx * self.variant_record_size()) as u64;
            for (j, sample_idx) in sample_idxs.iter().enumerate() {
                // first we obtain the byte in which the sample is located
                let sample_offset = sample_idx / 4;
                let byte_offest = record_offset + sample_offset as u64;
                // then we read the byte
                pgen_reader.seek(SeekFrom::Start(byte_offest)).unwrap();
                let mut buf = [0u8; 1];
                pgen_reader.read_exact(&mut buf).unwrap();
                // then we obtain the 2 bits that represent the sample
                let in_byte_offset = sample_idx % 4;
                // let encoded_genotype = (buf[0] >> ((3 - in_byte_offset) * 2)) & 0b11;
                let encoded_genotype = (buf[0] >> (in_byte_offset * 2)) & 0b11;
                let genotype = match encoded_genotype {
                    0b00 => "0/0",
                    0b01 => "0/1",
                    0b10 => "1/1",
                    0b11 => "./.",
                    _ => panic!("unexpected genotype"),
                };
                variant_line.push_str("\t");
                variant_line.push_str(genotype);
            }
            variant_line.push_str("\n");
            write!(vcf, "{}", variant_line).unwrap();
        }        
    }

    fn variant_record_size(&self) -> u32 {
        let bit_size = self.num_samples * 2;
        let byte_size = (bit_size / 8) + if bit_size % 8 == 0 { 0 } else { 1 };
        byte_size
    }

    fn read_pvar_header(&self) -> (String, String) {
        let pvar = File::open(&self.pvar_path()).unwrap();
        let mut pvar_reader = BufReader::new(pvar);
        // read all lines that start with # and store them in a vector
        let mut header_lines = Vec::new();
        loop {
            let mut buf = String::new();
            pvar_reader.read_line(&mut buf).unwrap();
            if buf.starts_with("#") {
                header_lines.push(buf);
            } else {
                break;
            }
        }
        // the last line must be the column names
        let header = header_lines.pop().unwrap();
        // return the header comments and the column names
        (header_lines.join(""), header)
    }

    /// Gives the offset to the start of the headers without the headers'
    /// comment prefix.
    ///
    /// The pvar file will look like the following
    /// (psams look the same with different column types)
    ///
    ///     ## Some headers
    ///     ## ...
    ///     ## Other headers
    ///     ## Other headers
    ///     #CHROM ID POS ...
    ///     12 id pos ...
    ///     11 id pos ...
    ///
    /// Per the spec, the last header line will indicate the headers for
    /// the rest of the pvar file, in the case of the example this would be
    ///
    ///     #CHROM ID POS ...
    ///
    /// We want to seek to just after the # in that line so that we can
    /// give the file reader to csv and it'll handle parsing everything.
    ///
    ///     #CHROM ID POS ...
    ///      ^
    ///      |
    ///      start here
    fn find_metadata_file_header_start(meta_file: String) -> io::Result<u64> {
        let meta_file = File::open(meta_file)?;
        let mut meta_raw_reader = BufReader::new(meta_file);
        let mut prev_buf = String::new();
        let mut buf = String::new();
        loop {
            prev_buf = buf;
            buf = String::new();
            meta_raw_reader.read_line(&mut buf)?;
            // We are reading the data now
            if !buf.starts_with("#") {
                let current_pos = meta_raw_reader.seek(SeekFrom::Current(0))?;
                // The current line is not what we're looking for.
                // The header is the previous line, but it is forced to start
                // with a #.
                // We want to rewind to just after that #.
                let offset = (buf.len() + prev_buf.len()) as u64 - 1;
                return Ok(current_pos - offset);
            }
        }
    }

    fn metadata_file_reader(file: String, num_rows: usize) -> io::Result<Reader<File>> {
        let header_start = Pfile::find_metadata_file_header_start(file.clone())?;
        let mut meta_file = File::open(file)?;
        meta_file.seek(SeekFrom::Start(header_start))?;

        let meta_reader = ReaderBuilder::new()
            .delimiter(b'\t')
            // per the spec, there are no comments
            .buffer_capacity(num_rows)
            // we seek to exactly where the headers start
            .has_headers(true)
            .from_reader(meta_file);
        return Ok(meta_reader);
    }

    pub fn pvar_reader(&self) -> io::Result<Reader<File>> {
        Pfile::metadata_file_reader(self.pvar_path(), self.num_variants as usize)
    }

    pub fn filter_test(&self) -> io::Result<()> {
        let mut reader = self.pvar_reader()?;
        let headers: StringRecord = reader.headers()?.clone();
        let mut count = 0;
        for record in reader.records() {
            let mut context = HashMapContext::new();
            for (var, val) in std::iter::zip(&headers, &record?) {
                context.set_value(var.to_string(), Value::String(val.to_string())).unwrap();
            }
            if eval_boolean_with_context("ID == \"rs8100066\"", &context).unwrap() {
                count += 1;
            }
        }
        println!("Keeping {} rows", count);
        Ok(())
    }

    pub fn psam_reader(&self) -> io::Result<Reader<File>> {
        Pfile::metadata_file_reader(self.psam_path(), self.num_samples as usize)
    }

    // TODO: refactor to use pvar_reader()
    pub fn filter_pvar(&self, variant_ids: Vec<String>) -> (Vec<String>, Vec<u32>, Vec<String>) {
        let pvar = File::open(&self.pvar_path()).unwrap();
        let mut pvar_reader = BufReader::new(pvar);
        // skip all lines that start with #
        loop {
            
            let mut buf = String::new();
            pvar_reader.read_line(&mut buf).unwrap();
            if !buf.starts_with("#") {
                // seek back to the start of the line
                pvar_reader.seek(SeekFrom::Current(-(buf.len() as i64))).unwrap();
                break;
            }
        }
        // read the rest of the file line by line, keeping the row idxs of the variants we want
        // as well as the whole line, so we can write it to a new file
        let mut cur_idx = 0;
        let mut sorted_variant_ids = Vec::new();
        let mut variant_idxs = Vec::new();
        let mut variant_lines = Vec::new();
        loop {
            let mut line = String::new();
            let bytes_read = pvar_reader.read_line(&mut line).unwrap();
            if bytes_read == 0 {
                break;
            }
            let variant_row: Vec<_> = line.split("\t").collect();
            let variant_id = variant_row[2].to_string();
            // the third item is the variant id
            if variant_ids.contains(&variant_id) {
                sorted_variant_ids.push(variant_id);
                variant_idxs.push(cur_idx);
                variant_lines.push(line);
            }
            cur_idx += 1;
        }
        assert_eq!(variant_idxs.len(), variant_ids.len());
        assert_eq!(cur_idx, self.num_variants);

        (sorted_variant_ids, variant_idxs, variant_lines)
    }

    // TODO: refactor to use psam_reader()
    pub fn filter_psam(&self, sample_ids: Vec<String>) -> (Vec<String>, Vec<u32>) {
        let psam = File::open(&self.psam_path()).unwrap();
        let mut psam_reader = BufReader::new(psam);
        // read the first line, which is the header, and split it by tabs
        // if the first column is not #IID, panic
        let mut header = String::new();
        psam_reader.read_line(&mut header).unwrap();
        let header: Vec<_> = header.split("\t").collect();
        assert_eq!(header[0], "#IID");

        // read the rest of the file line by line, keeping the row idxs of the samples we want
        let mut cur_idx = 0;
        let mut sorted_sample_ids = Vec::new();
        let mut sample_idxs = Vec::new();
        loop {
            let mut line = String::new();
            let bytes_read = psam_reader.read_line(&mut line).unwrap();
            if bytes_read == 0 {
                break;
            }
            let sample_row: Vec<_> = line.split("\t").collect();
            let sample_id = sample_row[0].to_string();
            // the first item is the sample id
            if sample_ids.contains(&sample_id) {
                sorted_sample_ids.push(sample_id);
                sample_idxs.push(cur_idx);
            }
            cur_idx += 1;
        }
        assert_eq!(cur_idx, self.num_samples);

        (sorted_sample_ids, sample_idxs)
    }
}
