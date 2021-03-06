extern crate tar;
extern crate flate2;

use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::error::Error;
use std::time::UNIX_EPOCH;
use tar::Archive;
use flate2::read::GzDecoder;


const BASE_DIR: &str = "/var/log/";

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("No command line arguments supplied");

        std::process::exit(0);
    }

    let cmd = &args[1];

    if cmd == "list" {
        list_log_files()
    } else if cmd == "read" {

        if args.len() < 3 {
            println!("read needs a path supplied");

            std::process::exit(0);
        }

        let path: Vec<_> = args[2].split("/").map(|p| String::from(p)).collect();
        read_log_file(String::from(BASE_DIR), path.as_slice());

    }

}

fn list_log_files() {
    println!("{{");

    indent(1);
    list_dir_entries(BASE_DIR, 1);
    println!();
    println!("}}");

    std::process::exit(0);
}

fn read_log_file(basepath: String, path: &[String]) {

    if path.len() == 0 {
        print!("Error: path is empty");
        return;
    }

    let dir_iterator = match fs::read_dir(&basepath) {
        Ok(p) => p,
        Err(_) => {
            print!("Could not read path: {}", basepath);
            return;
        }
    };

    let dir_entries: Vec<_> = dir_iterator.collect();

    for i in 0..dir_entries.len() {

        let dir_entry = &dir_entries[i].as_ref().unwrap();
        let filetype = dir_entry.file_type().unwrap();
        let filename = dir_entry.file_name().into_string().unwrap();

        if filetype.is_dir() {

            if path.len() > 1 && filename == path[0] {

                read_log_file(basepath + &filename + "/", &path[1..]);
                return;

            }

        } else if filetype.is_file() {

            let fullpath = basepath.to_owned() + &filename;

            if path.len() == 1 && filename == path[0] && filename.to_lowercase().ends_with(".gz") {

                let binfile = File::open(&fullpath).unwrap();
                let mut gzfile = GzDecoder::new(binfile);

                let mut contents = String::new();
                gzfile.read_to_string(&mut contents).unwrap();

                println!("{}", contents);
                return;

            } else if path.len() == 1 && filename == path[0] {

                let mut f = match File::open(&fullpath) {
                    Ok(p) => p,
                    Err(_) => {
                        println!("file not found: {}", &fullpath);
                        return;
                    }
                };

                let mut contents = String::new();
                f.read_to_string(&mut contents).expect(&format!(
                    "Could not read file: {}",
                    &fullpath
                ));

                println!("{}", contents);
                return;

            } else if path.len() == 2 && filename == path[0] && filename.to_lowercase().ends_with(".tar.gz") {

                let targetfilename = &path[1];

                let binfile = File::open(&fullpath).unwrap();
                let gzfile = GzDecoder::new(binfile);
                let mut arch = Archive::new(gzfile);

                for file in arch.entries().unwrap() {

                    let mut file = file.unwrap();

                    if file.header()
                        .path()
                        .unwrap()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap() == targetfilename
                    {
                        let mut contents = String::new();
                        file.read_to_string(&mut contents).unwrap();
                        println!("{}", contents);
                        return; 
                    }
                }
            }
        }
    }

    println!("File not found in filesystem enumeration");
    return;
}

fn list_dir_entries(p: &str, depth: i32) {

    let dir_iterator = match fs::read_dir(p) {
        Ok(p) => p,
        Err(e) => {
            print!("\"entries\": [ ], \"error\": true, \"errortext\": \"{}\"", e.description());
            return;
        }
    };

    let mut dir_entries: Vec<_> = dir_iterator.collect();

    if dir_entries.len() == 0 {
        print!("\"entries\": [ ]");
        return;
    }

    dir_entries.sort_by_key(|p| {
        p.as_ref()
            .map(|q| q.file_name().into_string().unwrap_or("".to_string()))
            .unwrap_or("".to_string())
            .to_lowercase()
    });

    dir_entries.sort_by_key(|p| {
        p.as_ref()
            .map(|q| q.file_type().map(|w| w.is_dir()).unwrap_or(false))
            .unwrap_or(false)
    });

    println!("\"entries\":");
    indent(depth);
    println!("[");

    for i in 0..dir_entries.len() {

        let dir_entry = &dir_entries[i].as_ref().unwrap();
        let last = i == dir_entries.len() - 1;
        let de_path = dir_entry.path();

        let entrypath = de_path.to_str().unwrap();
        let filename = dir_entry.file_name().into_string().unwrap();
        let filetype = dir_entry.file_type().unwrap();
        let metadata = dir_entry.metadata().unwrap();

        if filetype.is_dir() {
            indent(depth + 1);
            print!("{{");
            print!("\"name\": \"{0}\", ", filename);
            print!("\"type\": \"dir\", ");
            list_dir_entries(entrypath, depth + 1);

            print!("}}");

            if !last {
                print!(",");
            }
            println!();

        } else if filetype.is_file() {

            if filename.to_lowercase().ends_with(".tar.gz") {

                indent(depth + 1);
                print!("{{");
                print!("\"name\": \"{0}\", ", filename);
                print!("\"type\": \"compressed_dir\", ");
                list_compressed_dir_entries(entrypath, depth + 1);

                print!("}}");

                if !last {
                    print!(",");
                }
                println!();

            } else if filename.to_lowercase().ends_with(".gz") {

                indent(depth + 1);
                print!("{{");
                print!("\"name\": \"{0}\", ", filename);
                print!("\"type\": \"compressed_file\", ");
                print!("\"size\": {}, ", metadata.len());
                print!(
                    "\"ctime\": {}, ",
                    metadata.created().ok().map_or(0, |p| {
                        p.duration_since(UNIX_EPOCH).unwrap().as_secs()
                    })
                );
                print!(
                    "\"mtime\": {} ",
                    metadata.modified().ok().map_or(0, |p| {
                        p.duration_since(UNIX_EPOCH).unwrap().as_secs()
                    })
                );
                print!("}}");

                if !last {
                    print!(",");
                }
                println!();

            } else {

                indent(depth + 1);
                print!("{{");
                print!("\"name\": \"{0}\", ", filename);
                print!("\"type\": \"file\", ");
                print!("\"size\": {}, ", metadata.len());
                print!(
                    "\"ctime\": {}, ",
                    metadata.created().ok().map_or(0, |p| {
                        p.duration_since(UNIX_EPOCH).unwrap().as_secs()
                    })
                );
                print!(
                    "\"mtime\": {} ",
                    metadata.modified().ok().map_or(0, |p| {
                        p.duration_since(UNIX_EPOCH).unwrap().as_secs()
                    })
                );
                print!("}}");

                if !last {
                    print!(",");
                }
                println!();

            }


        } else if filetype.is_symlink() {
            indent(depth + 1);
            print!("{{");
            print!("\"name\": \"{0}\", ", filename);
            print!("\"type\": \"symlink\" ");
            print!("}}");

            if !last {
                print!(",");
            }
            println!();
        }
    }

    indent(depth);
    print!("]");
}

fn list_compressed_dir_entries(p: &str, depth: i32) {

    let binfile = File::open(p).unwrap();
    let gzfile = GzDecoder::new(binfile);
    let mut arch = Archive::new(gzfile);

    let dir_entries: Vec<_> = arch.entries().unwrap().collect();

    if dir_entries.len() == 0 {
        print!("\"entries\": [ ]");
        return;
    }

    println!("\"entries\":");
    indent(depth);
    println!("[");

    for i in 0..dir_entries.len() {
        let file = &dir_entries[i].as_ref().unwrap();
        let last = i == dir_entries.len() - 1;

        indent(depth + 1);
        print!("{{");
        print!(
            "\"name\": \"{0}\", ",
            file.header()
                .path()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        );
        print!("\"type\": \"file\", ");
        print!("\"size\": {}, ", file.header().size().unwrap());
        print!("\"ctime\": {}, ", 0);
        print!("\"mtime\": {} ", file.header().mtime().unwrap());
        print!("}}");

        if !last {
            print!(",");
        }
        println!();
    }

    indent(depth);
    print!("]");
}

fn indent(d: i32) {
    for _ in 0..d {
        print!("\t");
    }
}
