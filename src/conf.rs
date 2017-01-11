use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process;
use std::os::unix::io::{RawFd, IntoRawFd};
use std::env;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use std::collections::HashSet;

use exec;

#[derive(Clone)]
pub struct Log {
    pub seed_count: u32,
    pub crash_count: u32,
    pub uniq_crash_count: u32,
    pub crash_paths: HashSet<u64>,
    pub start_time: SystemTime,
    pub exec_count: u64,
    pub total_node: u32,
}

pub struct Conf {
    pub args: Vec<String>,
    pub input_path: PathBuf,
    pub output_dir: PathBuf,
    pub path_base: PathBuf,
    pub stdin_fd: RawFd,
    pub timeout: u64,
    pub log: Arc<RwLock<Log>>,
}

impl Log {
    pub fn new() -> Self {
        Log {
            seed_count: 0,
            crash_count: 0,
            uniq_crash_count: 0,
            crash_paths: HashSet::new(),
            start_time: SystemTime::now(),
            exec_count: 0,
            total_node: 0,
        }
    }
}

impl Conf {
    pub fn new(args: Vec<&str>, output_dir: &str, t: u64, input_path: &str) -> Conf {
        if Path::new(&output_dir).exists() {
            println!("Error: {} already exists", output_dir);
            process::exit(1)
        }

        fs::create_dir(&output_dir).unwrap();
        fs::create_dir(format!("{}/queue", output_dir)).unwrap();
        fs::create_dir(format!("{}/crash", output_dir)).unwrap();
        let stdin_fd = fs::File::create(format!("{}/.stdin", output_dir)).unwrap().into_raw_fd();

        let input_path = match args.iter().find(|&&s| s == "@@") {
            Some(_) => String::from(input_path),
            None => format!("{}/.stdin", output_dir),
        };
        let mut args = args.iter()
            .map(|&s| if s == "@@" {
                input_path.clone()
            } else {
                String::from(s)
            })
            .collect::<Vec<String>>();
        let path_base = match Path::new(&env::args().nth(0).unwrap()).parent() {
            Some(p) => String::from(p.to_str().unwrap()),
            None => "".into(),
        };
        let qemu_path = format!("{}/qemu-trace-coverage", path_base);
        args.insert(0, qemu_path);

        let log = Arc::new(RwLock::new(Log::new()));

        Conf {
            args: args,
            output_dir: PathBuf::from(output_dir),
            input_path: PathBuf::from(input_path),
            path_base: PathBuf::from(path_base),
            stdin_fd: stdin_fd,
            timeout: t,
            log: log,
        }
    }

    pub fn new_without_filename(args: Vec<&str>, output_dir: &str, t: u64) -> Conf {
        let filepath = format!("{}/.input", output_dir);
        Conf::new(args, output_dir, t, &filepath)
    }

    pub fn save_crash(&self, buf: &Vec<u8>, feedback: &exec::Feedback) {
        let crash_num = {
            let mut log = self.log.write().unwrap();
            log.crash_count += 1;
            if log.crash_paths.insert(feedback.subpath) {
                log.uniq_crash_count += 1;
                log.uniq_crash_count
            } else {
                return;
            }
        };
        let path = self.output_dir.join(format!("crash/tc-{}", crash_num));
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(buf).unwrap();
    }
}
