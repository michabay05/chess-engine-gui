use std::process::{Command, Child, ChildStdin, ChildStdout, Stdio};
use std::io::{Write, Read};
use std::time::Duration;

pub struct EngineComm {
    process: Child,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,

    name: String,
    search_time_left: Option<Duration>,
    searching: bool,
}

impl EngineComm {
    const MAX_RE_READ_COUNT: usize = 4;
    pub fn new(file_path: &str) -> Result<Self, ()> {
        let mut process = Command::new(file_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start child process");

        // Take ownership of stdin and stdout
        let stdin = process.stdin.take().expect("Failed to open stdin");
        let stdout = process.stdout.take().expect("Failed to open stdout");
        let mut this = Self {
            process, 
            stdin: Some(stdin), 
            stdout: Some(stdout),
            name: String::new(),
            search_time_left: None,
            searching: false,
        };
        if !this.uci() {
            return Err(());
        }
        Ok(this)
    }

    fn read(&mut self, buf: &mut String) {
        assert!(self.stdout.is_some());
        let stdout = self.stdout.as_mut().unwrap();

        let mut buffer = [0; 1024 * 64];
        match stdout.read(&mut buffer) {
            Ok(_) => {
                buf.clear();
                *buf = String::from_utf8_lossy(&buffer).into_owned();
                // *buf = String::from_utf8((&buffer).to_vec()).unwrap();
            }
            Err(e) => eprintln!("[ERROR] {e}"),
        };
    }

    fn read_until_rmatch(&mut self, pat: &str, buf: &mut String) -> Option<usize> {
        let mut temp = String::new();
        let mut loop_count = 0;
        // Note: Loop count needed to prevent the current thread from being
        //       infinitely blocked.
        while loop_count <= Self::MAX_RE_READ_COUNT {
            self.read(&mut temp);
            buf.push_str(&temp);
            let found_pat = buf.rfind(pat);
            if found_pat.is_some() { return found_pat; }
            loop_count += 1;
        }
        None
    }

    fn send(&mut self, cmd: &str) {
        assert!(self.stdin.is_some());
        let stdin = self.stdin.as_mut().unwrap();

        // Note: newline needed in order to simulate <ENTER> key press
        let message = format!("{}\n", cmd.trim());
        if let Err(_) = stdin.write(message.as_bytes()) {
            eprintln!("[ERROR] Failed to send message to child stdin");
        }
        if let Err(_) = stdin.flush() {
            eprintln!("[ERROR] Failed to flush to child");
        }
        // println!("[SEND] {}", cmd.trim());
    }

    fn uci(&mut self) -> bool {
        let mut buf = String::new();
        self.send("uci");
        if self.read_until_rmatch("uciok", &mut buf).is_none() {
            return false;
        }
        for line in buf.lines() {
            let mut words = line.split_whitespace();
            if let Some(word) = words.next() {
                if &word[word.len() - 2..] != "id" { continue; }
            }
            if let Some(word) = words.next() {
                match word {
                    "name" => self.name = words.next().unwrap_or("No name").to_string(),
                    _ => {}
                };
            }
        }
        self.send("isready");
        buf.clear();
        if self.read_until_rmatch("readyok", &mut buf).is_none() {
            return false;
        }
        true
    }

    pub fn fen(&mut self, fen: &str) {
        self.send(&format!("position fen {}", fen));
    }

    pub fn stop(&mut self) {
        self.searching = false;
        self.search_time_left = None;
        self.send("stop");
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn search_movetime(&mut self, time_ms: u64) {
        self.send(&format!("go movetime {}", time_ms));
        self.search_time_left = Some(Duration::from_millis(time_ms));
        self.searching = true;
    }

    pub fn is_searching(&mut self) -> bool {
        self.searching
    }

    pub fn update_time_left(&mut self, time_s: f32) {
        if let Some(stl) = self.search_time_left.take() {
            let frame_dur = Duration::from_secs_f32(time_s);
            self.search_time_left = stl.checked_sub(frame_dur);
        }
    }

    pub fn search_time_over(&mut self) -> bool {
        let result = self.search_time_left.is_none();
        if result { self.searching = false; }
        result
    }

    pub fn best_move(&mut self) -> Option<String> {
        let mut buf = String::new();
        if let Some(ind) = self.read_until_rmatch("bestmove", &mut buf) {
            // TODO: try to parse the last evaluation from the output produced by the engine

            let best_move = &buf[(ind+8)..].trim_start();
            let mut i = 0;
            loop {
                let mut iter = best_move.chars();
                if let Some(ch) = iter.nth(i) {
                    if ch.is_ascii_alphanumeric() {
                        i += 1;
                    } else {
                        break;
                    }
                }
            }
            Some(best_move[0..i].to_string())
        } else {
            None
        }
    }
}

impl Drop for EngineComm {
    fn drop(&mut self) {
        self.send("quit");
        drop(self.stdin.take());
        let _ = self.process.wait().expect("Failed to wait for child process");
    }
}
