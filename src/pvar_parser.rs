use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::{self, BufRead};

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "pvar_parser.pest"]
pub struct PvarParser;

impl PvarParser {
    fn get_substring_from_index(input: &str, start_index: usize) -> Option<&str> {
        if start_index < input.len() {
            Some(&input[start_index..])
        } else {
            None
        }
    }
    
    fn get_meta_idname(input: &str) -> Option<&str> {
        let pairs = PvarParser::parse(Rule::infoheader, input).ok().unwrap();
        for pair in pairs {
            for inner_pair in pair.into_inner() {
                if inner_pair.as_rule() == Rule::idname {
                    return Some(inner_pair.as_str())
                }
            }
        }
        return None
    }

    fn get_meta_descs(input: &str) -> (String, String) {
        // let mut kv_pairs = HashMap::new();
        let pairs_opt = PvarParser::parse(Rule::infoheader, input).ok();
        match pairs_opt {
            Some (pairs) => {
                let mut idname = "";
                let mut desc = "a";
                for pair in pairs {
                    for inner_pair in pair.into_inner() {
                        if inner_pair.as_rule() == Rule::idname {
                            idname = inner_pair.as_str();
                        }
                        if inner_pair.as_rule() == Rule::desc {
                            desc = inner_pair.as_str();
            
                        }
                    }
                }
                return (idname.to_string(), desc.to_string())
            },
            None => return ("".to_string(), "".to_string()),
        };
    }

    pub fn format_descriptions(filepath: &str) -> io::Result<Vec<String>> {
        let file_path = filepath;
        let file = File::open(file_path)?;
        let reader = io::BufReader::new(file);
        let mut parsed_descriptions = Vec::new();
    
        for line in reader.lines() {
            let line = line?;
            if line.starts_with("##INFO=") {
                let (name, desc) = Self::get_meta_descs(&line);
                parsed_descriptions.push(format!("- {}: {}", name, desc));
            } else if line.starts_with("#CHROM") {
                break;
            }
        }
        Ok(parsed_descriptions)
    }
    
    pub fn get_meta_idnames(filepath: &str) -> io::Result<Vec<String>> {
        let file_path = filepath;
        let file = File::open(file_path)?;
        let reader = io::BufReader::new(file);
        let mut continue_parsing = true;
        let mut parsed_idnames = Vec::new();
    
        for line in reader.lines() {
            let line = line?;
            match Self::get_meta_idname(&line) {
                Some(substr) => parsed_idnames.push(substr.to_string()),
                None => {
                    if line.contains("#CHROM") { // stop once we get to the real header
                        continue_parsing = false;
                        break;
                    }
                },
            }
            if !continue_parsing {
                break;
            }
        }
        Ok(parsed_idnames)
    }

    fn get_info_line(input: &str) -> Option<&str> {
        let pairs = PvarParser::parse(Rule::line, input).ok()?;
        for pair in pairs {
            for inner_pair in pair.into_inner() {
                if inner_pair.as_rule() == Rule::INFO {
                    return Some(inner_pair.as_str());
                }
            }
        }
        None
    }

    pub fn get_info_kv_pairs(input: &str) -> HashMap<String, String> {
        let mut kv_pairs = HashMap::new();
        let pairs_opt = PvarParser::parse(Rule::INFO, input).ok();
        match pairs_opt {
            Some (pairs) => {
                for pair in pairs {
                    for inner_pair in pair.into_inner() {
                        if inner_pair.as_rule() == Rule::key_val_pair {
                            let mut keyname = "";
                            let mut val = "";
                            for kv in inner_pair.into_inner() {
                                if kv.as_rule() == Rule::key_name {
                                    keyname = kv.as_str();
                                }
                                if kv.as_rule() == Rule::value {
                                    val = kv.as_str();
                                }
                            }
                            kv_pairs.insert(keyname.to_string(), val.to_string());
                        } else if inner_pair.as_rule() == Rule::key_name {
                            kv_pairs.insert(inner_pair.as_str().to_string(), "".to_string());
                        }
                    }
                }
            },
            None => (),
        }
        return kv_pairs
    }

    fn get_line_entry(input: &str, rule: Rule) -> Option<&str> {
        let pairs = PvarParser::parse(Rule::line, input).ok()?;
        for pair in pairs {
            for inner_pair in pair.into_inner() {
                if inner_pair.as_rule() == rule {
                    return Some(inner_pair.as_str());
                }
            }
        }
        None
    }

    // fn get_alt_info(altinput: &str) -> Vec<String> {
    //     let alleles = altinput.split(",").collect();
    //     return alleles;
    // }
    
    fn get_info_lines(filepath: &str) -> io::Result<Vec<String>> {
        let file_path = filepath;
        let file = File::open(file_path)?;
        let reader = io::BufReader::new(file);
        let mut parsed_kvpairs = Vec::new();
    
        for line in reader.lines() {
            let line = line?;
            if !line.starts_with("#") {
                if let Some(kvpairs) = Self::get_info_line(&line) {
                    parsed_kvpairs.push(kvpairs.to_string());
                }
            }
        }
        Ok(parsed_kvpairs)
    }
    
    fn find_kv_pair<'a>(input: &'a str, keyname: &str) -> Option<&'a str> {
        if let Some(start_index) = input.find(keyname) {
            let start_index = start_index + keyname.len();
            let end_index = input[start_index..].find(';')
                .map(|index| start_index + index)
                .unwrap_or_else(|| input.len());
            return Some(&input[start_index..end_index]);
        }
        None
    }
    
    fn find_key<'a>(input: &'a str, keyname: &str) -> bool {
        if let Some(start_index) = input.find(keyname) {
            let start_index = start_index + keyname.len();
            let end_index = input[start_index..].find(';')
                .map(|index| start_index + index)
                .unwrap_or_else(|| input.len());
            let substr = &input[start_index..end_index];
            // if we've consumed the prefix and there's nothing else then we've found a single key
            return substr == "";
        } else {
            return false;
        }
    }

}


fn main() -> io::Result<()> {
    // let parsed_idnames = get_idnames("data/basic0.pvar");

    // for idname in &parsed_idnames {
    //     println!("{:?}", idname);
    // }

    // let ids = PvarParser::get_meta_descs("##INFO=<ID=AA,Number=1,Type=String,Description=\"Ancestral Allele. Format: AA|REF|ALT|IndelType. AA: Ancestral allele, REF:Reference Allele, ALT:Alternate Allele, IndelType:Type of Indel (REF, ALT and IndelType are only defined for indels)\">");
    // println!("{:?}", ids);

    // let info: HashMap<String, String> = PvarParser::get_info_kv_pairs("AC=2731;AF=0.545327;AN=5008;NS=2504;DP=19168;EAS_AF=0.124;EX_TARGET;AMR_AF=0.5072;AFR_AF=0.8835;EUR_AF=0.6252;SAS_AF=0.4673;AA=.|||;VT=SNP");
    // let info = PvarParser::get_info_kv_pairs("AC=2731;AF=0.545327");
    // println!("{:?}", info);


    // let info = get_info_info("data/basic0.pvar");
    // for inf in &info {
    //     for kv in inf {
    //         println!("{:?}", kv);
    //     }
    //     // println!("{:?}", inf)

    // }

    // if let Some(info) = PvarParser::get_info_lines("data/basic1/basic1.pvar").ok() {
    //     let mut i = 0;
    //     for ln in info {
    //         // println!("{:?}", ln);
    //         // let prefix = format!("{}{}", "EAS_AF", "=");
    //         let prefix = "EX_TARGET";
    //         // if let Some(pair) = PvarParser::find_kv_pair(&ln, &prefix) {
    //         if PvarParser::find_key(&ln, &prefix) {
    //             println!("{}: {:?}", i, ln);
    //             // println!("{:?}", has);
    //             println!();
    //         }
    //         i += 1;
    //         // if i > 10 {
    //         //     break;
    //         // }
    //     }
    // }


    Ok(())
}