use std::{
    fs::{create_dir_all, File},
    path::Path,
};

pub struct LogTarget {
    file: File,
}

impl LogTarget {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        create_dir_all(path.parent().unwrap()).unwrap();
        Self {
            file: File::create(path).unwrap(),
        }
    }
}

impl std::io::Write for LogTarget {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

pub fn init_logger<P: AsRef<Path>>(path: P) {
    let log_target = LogTarget::new(path);
    env_logger::builder()
        .target(env_logger::Target::Pipe(Box::new(log_target)))
        .init();
}
