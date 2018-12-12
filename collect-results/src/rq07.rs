use results;
use std::io::BufReader;
use std::io::BufRead;
use std::io::BufWriter;
use std::io::Write;
use std::fs::File;

pub fn process_rq(crates: &Vec<(String,String)>) {
    let mut writer_opt = BufWriter::new(::get_output_file("rq07_opt"));
    let mut writer_pes = BufWriter::new( ::get_output_file("rq07_pes"));
    for (crate_name, version) in crates {
        info!("Processing crate {:?}", crate_name);
        let dir_name = ::get_full_analysis_dir();
        let file_ops = results::FileOps::new( crate_name, &version, &dir_name );
        if let Some (files) = file_ops.open_files(results::IMPLICIT_RTA_OPTIMISTIC_FILENAME) {
            for file in files.iter() {
                process_file(file, &mut writer_opt, crate_name);
            }
        } else {
            error!("Implicit unsafe optimistic files missing for crate {:?}", crate_name);
        }
        if let Some (files) = file_ops.open_files(results::IMPLICIT_RTA_PESSIMISTIC_FILENAME) {
            for file in files.iter() {
                process_file(file, &mut writer_pes, crate_name);
            }
        } else {
            error!("Implicit unsafe pesimistic files missing for crate {:?}", crate_name);
        }
    }
}

fn process_file( input_file: &File, writer: &mut BufWriter<File>, crate_name: &String) {
    let mut reader = BufReader::new(input_file);
    //read line by line
    let mut total = 0;
    loop {
        let mut line = String::new();
        let len = reader.read_line(&mut line).expect("Error reading file");
        if len == 0 {
            //EOF reached
            break;
        } else {
            //process line
            let trimmed_line = line.trim_right();
            let res: results::implicit::UnsafeInBody = serde_json::from_str(&trimmed_line).unwrap();
            match res.fn_type {
                results::implicit::FnType::NormalNotSafe
                | results::implicit::FnType::Unsafe => {
                    total = total + 1;
                }
                _ => {}
            }
        }
    }
    writeln!(writer, "{}\t{}"
             , crate_name
             , total
    );
}