extern crate tempdir;

use self::tempdir::TempDir;
use super::mpd;
use std::fs::{File, create_dir};
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Child};

struct MpdConfig {
    db_file: PathBuf,
    music_directory: PathBuf,
    playlist_directory: PathBuf,
    config_path: PathBuf,
    sock_path: PathBuf,
}

impl MpdConfig {
    pub fn new<P>(base: P) -> MpdConfig
        where P: AsRef<Path>
    {
        let base = base.as_ref();
        MpdConfig {
            db_file: base.join("db"),
            music_directory: base.join("music"),
            playlist_directory: base.join("playlists"),
            config_path: base.join("config"),
            sock_path: base.join("sock"),
        }
    }

    fn config_text(&self) -> String {
        format!(r#"
db_file "{db_file}"
log_file "/dev/null"
music_directory "{music_directory}"
playlist_directory "{playlist_directory}"
bind_to_address "{sock_path}"
audio_output {{
    type "null"
    name "null"
}}
"#,
            db_file=self.db_file.display(),
            music_directory=self.music_directory.display(),
            playlist_directory=self.playlist_directory.display(),
            sock_path=self.sock_path.display(),
        )
    }

    fn generate(&self) {
        create_dir(&self.music_directory).expect("Could not create music directory.");
        create_dir(&self.playlist_directory).expect("Could not create playlist directory.");
        let mut file = File::create(&self.config_path).expect("Could not create config file.");
        file.write_all(self.config_text().as_bytes()).expect("Could not write config file.");
    }
}

pub struct Daemon {
    // Saved here so it gets dropped when this does.
    _temp_dir: TempDir,
    config: MpdConfig,
    process: Child,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        self.process.kill().expect("Could not kill mpd daemon.");
        self.process.wait().expect("Could not wait for mpd daemon to shutdown.");
    }
}

fn sleep() {
    use std::{thread, time};
    let ten_millis = time::Duration::from_millis(100);
    thread::sleep(ten_millis);
}

impl Daemon {
    pub fn start() -> Daemon {
        let temp_dir = TempDir::new("mpd-test").unwrap();
        let config = MpdConfig::new(&temp_dir);
        config.generate();
        let process = Command::new("mpd")
            .arg("--no-daemon")
            .arg(&config.config_path)
            .spawn()
            .expect("Could not create mpd daemon.");
        while !config.sock_path.exists() {}

        // FIXME: Wait for mpd to finish updating the database.
        sleep();

        Daemon {
            _temp_dir: temp_dir,
            config: config,
            process: process,
        }
    }

    pub fn connect(&self) -> mpd::Client<UnixStream> {
        let stream = UnixStream::connect(&self.config.sock_path).unwrap();
        mpd::Client::new(stream).unwrap()
    }
}
