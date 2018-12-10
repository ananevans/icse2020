use results;
use std::io::BufReader;
use std::io::BufRead;
use std::io::BufWriter;
use std::io::Write;
use results::unsafety_sources::SourceKind;

struct SourceSummary {
    pub unsafe_fn_calls: usize,
    pub raw_ptr: usize,
    pub asm: usize,
    pub static_access: usize,
    pub borrow_packed: usize,
    pub assignment_union: usize,
    pub union: usize,
    pub extern_static: usize,
    pub argument: bool,
    pub from_trait: bool,
}

impl SourceSummary {
    pub fn new() -> Self {
        Self{ unsafe_fn_calls: 0
            , raw_ptr: 0
            , asm: 0
            , static_access: 0
            , borrow_packed: 0
            , assignment_union: 0
            , union: 0
            , extern_static: 0
            , argument : false
            , from_trait: false
        }
    }

}


pub fn process_rq(crates: &Vec<(String,String)>) {
    let output_file = ::get_output_file("rq05");
    let mut writer = BufWriter::new(output_file);
    for (crate_name, version) in crates {
        let dir_name = ::get_full_analysis_dir();
        let file_ops = results::FileOps::new( crate_name, &version, &dir_name );
        if let Some (files) = file_ops.open_files(results::FN_UNSAFETY_SOURCES_FILE_NAME) {
            for file in files.iter() {
                let mut reader = BufReader::new(file);
                //read line by line
                loop {
                    let mut line = String::new();
                    let len = reader.read_line(&mut line).expect("Error reading file");
                    if len == 0 {
                        //EOF reached
                        break;
                    } else {
                        //process line
                        let trimmed_line = line.trim_right();
                        let res1: serde_json::Result<results::functions::UnsafeFnUsafetySources> = serde_json::from_str(&trimmed_line);

                        if let Ok(res) = res1 {
                            let mut fn_summary = SourceSummary::new();
                            for src in res.sources() {
                                match src.kind {
                                    SourceKind::UnsafeFnCall(_) => { fn_summary.unsafe_fn_calls += 1; },
                                    SourceKind::DerefRawPointer => { fn_summary.raw_ptr += 1; },
                                    SourceKind::Asm => { fn_summary.asm += 1; },
                                    SourceKind::Static => { fn_summary.static_access += 1; },
                                    SourceKind::BorrowPacked => { fn_summary.borrow_packed += 1; },
                                    SourceKind::AssignmentToNonCopyUnionField => { fn_summary.assignment_union += 1; },
                                    SourceKind::AccessToUnionField => { fn_summary.union += 1; },
                                    SourceKind::ExternStatic => { fn_summary.extern_static += 1; },
                                }
                            }
                            fn_summary.from_trait = res.from_trait();
                            fn_summary.argument = res.arguments().len() > 0;
                            writeln!(writer, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}"
                                     , fn_summary.unsafe_fn_calls
                                     , fn_summary.raw_ptr
                                     , fn_summary.asm
                                     , fn_summary.static_access
                                     , fn_summary.borrow_packed
                                     , fn_summary.assignment_union
                                     , fn_summary.union
                                     , fn_summary.extern_static
                                     , (if fn_summary.argument { 1 } else { 0 })
                                     , (if fn_summary.from_trait { 1 } else { 0 })
                                     , crate_name
                                     //, res.name()
                            );
                        } else {
                            error!("Could not process {:?} line: {:?}", crate_name, trimmed_line);
                        }
                    }
                }
            }
        } else {
            error!("Function unsafety sources files missing for crate {:?}", crate_name);
        }
    }
}