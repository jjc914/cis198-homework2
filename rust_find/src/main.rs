use std::path::PathBuf;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;

use structopt::StructOpt;
use filesize::PathExt;
use regex::Regex;
use colored::Colorize;

#[derive(Debug, StructOpt)]
#[structopt(name = "rust-find 0.1.0", about = "a command line utility for searching for files")]
struct CLI {
    #[structopt(short, long)]
    dirs: Vec<PathBuf>,
    #[structopt(short, long)]
    patterns: Option<Vec<String>>,

    #[structopt(long)]
    size_min: Option<u64>,
    #[structopt(long)]
    size_max: Option<u64>,

    #[structopt(short, long)]
    output: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
struct MyFile {
    path: PathBuf,
    name: String,
    size_bytes: u64,
}

impl MyFile {
    fn from_path(p: &PathBuf) -> Option<Self> {
        let path = p.clone();
        let name = String::from(path.file_name()?.to_str()?);
        let size = path.as_path().size_on_disk().ok()?;
        Some(MyFile {
            path: path,
            name: name,
            size_bytes: size,
        })
    }
}

// gets all files
fn get_files(dirs: Vec<PathBuf>) -> Vec<MyFile> {
    // would be more efficient to skip files based on the regex, but i think this method is more
    // robust for future features
    fn rec_get_files(dir: PathBuf) -> Vec<MyFile> {
        let mut vec = Vec::new();
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                let file = match MyFile::from_path(&path) {
                    Some(f) => {
                        f
                    },
                    None => {
                        println!("{}{}{}", 
                                 "warning".bold().yellow(), 
                                 ": could not access file: ".bold(),
                                 path.display());
                        println!("skipping search in directory: {}", path.display());
                        continue;
                    },
                };
                vec.push(file);
                continue;
            }
            vec.append(&mut rec_get_files(path));
        }
        vec
    }

    let mut vec = Vec::new();
    for dir in dirs {
        if !dir.exists() {
            println!("{}{}{}", 
                     "warning".bold().yellow(), 
                     ": no such file or directory: ".bold(),
                     dir.clone().into_os_string().into_string().unwrap());
            println!("skipping search in directory: {}", dir.clone().into_os_string().into_string().unwrap());

            continue;
        }
        vec.append(&mut rec_get_files(dir));
    }
    vec
}

fn filter_files_regex<'a>(files: &'a Vec<&'a MyFile>, patterns: &Vec<String>) -> Vec<&'a MyFile> {
    let mut regexes = Vec::new();
    for pattern in patterns {
        let regex = match Regex::new(&pattern) {
            Ok(r) => {
                r
            },
            Err(e) => {
                println!("{}{}{}",
                         "warning".bold().yellow(),
                         ": invalid regex: ".bold(),
                         e);
                println!("skipping regex match: {}", pattern);
                continue;
            },
        };
        regexes.push(regex);
    }

    let filtered: Vec<&MyFile> = files.iter()
        .filter(|&&file| {
            regexes.iter().any(|regex| regex.is_match(&file.name))
        })
        .cloned()
        .collect();
    filtered
}

fn filter_files_size_min<'a>(files: &'a Vec<&'a MyFile>, min_size: &u64) -> Vec<&'a MyFile> {
    let filtered: Vec<&MyFile> = files.iter()
        .filter(|&&file| {
            file.size_bytes >= *min_size
        })
        .cloned()
        .collect();
    filtered
}

fn filter_files_size_max<'a>(files: &'a Vec<&'a MyFile>, max_size: &u64) -> Vec<&'a MyFile> {
    let filtered: Vec<&MyFile> = files.iter()
        .filter(|&&file| {
            file.size_bytes <= *max_size
        })
        .cloned()
        .collect();
    filtered
}

fn output_files(path: &PathBuf, files: &Vec<&MyFile>) -> std::io::Result<()> {
    File::create(path)?;

    let mut output = OpenOptions::new()
        .append(true)
        .open(path)
        .expect("cannot open file");

    // Write to a file
    for file in files {
        output.write(file.path.to_str()
                              .ok_or("conversion to string failed")
                              .unwrap()
                              .as_bytes())
              .expect("write failed");
        output.write(b"\n").expect("write failed");
    }
    Ok(())
}

fn main() {
    let cli = CLI::from_args();

    let files: Vec<MyFile> = get_files(cli.dirs);

    let ffiles = files.iter().collect();
    let ffiles: Vec<&MyFile> = match cli.patterns {
        None => {
            files.iter().collect()
        },
        Some(pat) => {
            filter_files_regex(&ffiles, &pat)
        }
    };

    let ffiles: Vec<&MyFile> = match cli.size_min {
        None => {
            ffiles
        },
        Some(min) => {
            filter_files_size_min(&ffiles, &min)
        }
    };

    let ffiles: Vec<&MyFile> = match cli.size_max {
        None => {
            ffiles
        },
        Some(max) => {
            filter_files_size_max(&ffiles, &max)
        }
    };

    match cli.output {
        None => {
            for file in ffiles {
                println!("{}", file.path.display());
            }
        }, 
        Some(path) => {
            output_files(&path, &ffiles).expect("output failed");
        },
    };
}

#[test]
fn test_filter_files_regex() {
    let file1 = MyFile { 
        path: PathBuf::from("/path/to/file1.txt"), 
        name: "file1.txt".to_string(), 
        size_bytes: 1024 
    };
    let file2 = MyFile { 
        path: PathBuf::from("/path/to/file2.jpg"), 
        name: "file2.jpg".to_string(), 
        size_bytes: 2048 
    };
    let file3 = MyFile { 
        path: PathBuf::from("/path/to/file3.txt"), 
        name: "file3.txt".to_string(), 
        size_bytes: 4096 
    };
    let file4 = MyFile { 
        path: PathBuf::from("/path/to/file4.png"), 
        name: "file4.png".to_string(), 
        size_bytes: 1024
    };
    let files = vec![&file1, &file2, &file3, &file4];

    let patterns = vec![
        "\\w+\\.txt".to_string(),
        "\\w+\\.jpg".to_string()
    ];

    let result = filter_files_regex(&files, &patterns);

    assert_eq!(result.len(), 3);
    assert!(result.contains(&&file1));
    assert!(result.contains(&&file2));
    assert!(result.contains(&&file3));
    assert!(!result.contains(&&file4));
}

#[test]
fn filter_files_size_min_test() {
    let file1 = MyFile { 
        path: PathBuf::from("/path/to/file1.txt"), 
        name: "file1.txt".to_string(), 
        size_bytes: 1024 
    };
    let file2 = MyFile { 
        path: PathBuf::from("/path/to/file2.jpg"), 
        name: "file2.jpg".to_string(), 
        size_bytes: 2048 
    };
    let file3 = MyFile { 
        path: PathBuf::from("/path/to/file3.txt"), 
        name: "file3.txt".to_string(), 
        size_bytes: 4096 
    };
    let file4 = MyFile { 
        path: PathBuf::from("/path/to/file4.png"), 
        name: "file4.png".to_string(), 
        size_bytes: 1024
    };
    let files = vec![&file1, &file2, &file3, &file4];

    let min = 2048;

    let result = filter_files_size_min(&files, &min);

    assert_eq!(result.len(), 2);
    assert!(!result.contains(&&file1));
    assert!(result.contains(&&file2));
    assert!(result.contains(&&file3));
    assert!(!result.contains(&&file4));
}

#[test]
fn filter_files_size_max_test() {
    let file1 = MyFile { 
        path: PathBuf::from("/path/to/file1.txt"), 
        name: "file1.txt".to_string(), 
        size_bytes: 1024 
    };
    let file2 = MyFile { 
        path: PathBuf::from("/path/to/file2.jpg"), 
        name: "file2.jpg".to_string(), 
        size_bytes: 2048 
    };
    let file3 = MyFile { 
        path: PathBuf::from("/path/to/file3.txt"), 
        name: "file3.txt".to_string(), 
        size_bytes: 4096 
    };
    let file4 = MyFile { 
        path: PathBuf::from("/path/to/file4.png"), 
        name: "file4.png".to_string(), 
        size_bytes: 1024
    };
    let files = vec![&file1, &file2, &file3, &file4];

    let max = 2048;

    let result = filter_files_size_max(&files, &max);

    assert_eq!(result.len(), 3);
    assert!(result.contains(&&file1));
    assert!(result.contains(&&file2));
    assert!(!result.contains(&&file3));
    assert!(result.contains(&&file4));
}
