use std::io::{BufRead, BufReader};
use std::io::{Lines, Write};
use std::os::windows::process::CommandExt;
use std::process::Stdio;
use std::process::{ChildStdin, ChildStdout, Command};

use anyhow::{Context, Result};
use itertools::Itertools;
use log::{debug, info};
use rusty_tesseract::Image;
pub fn run_manga_ocr(images: &Vec<Image>, manga_ocr: &mut MangaOcrInstance) -> Result<Vec<String>> {
    let paths = images
        .iter()
        .filter_map(|x| x.get_image_path().ok())
        .join(",");

    manga_ocr.run_ocr(&paths)
}

pub struct MangaOcrInstance {
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
}
const CREATE_NO_WINDOW: u32 = 0x08000000;

impl MangaOcrInstance {
    pub fn init() -> Result<Self> {
        let mut child = Command::new("cmd")
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .args(["/c", "python -X utf8 -m manga_ocr cli cli"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;

        let stdin = child.stdin.take().context("no stdin.take()")?;
        let stdout = child.stdout.take().context("no stdout.take()")?;
        let mut stdout = BufReader::new(stdout).lines();

        let mut option: String = String::new();
        while !option.contains("Enter image path:") {
            option = stdout.next().context("no stdout.next()")??;
            debug!("---{}", option);
        }

        info!("manga ocr init done");

        Ok(Self { stdin, stdout })
    }

    pub fn run_ocr(&mut self, input: &str) -> Result<Vec<String>> {
        self.stdin.write_all(format!("{}\n", input).as_bytes())?;
        let mut output_vec: Vec<String> = Vec::new();
        let mut stdout_text = self.stdout.next().context("no next stdout output")??;

        while !stdout_text.contains("Enter image path:") {
            output_vec.push(stdout_text);
            stdout_text = self.stdout.next().context("no next stdout_text")??;
        }

        Ok(output_vec)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::ocr::manga_ocr::MangaOcrInstance;

    #[test]
    fn test2() {
        let mut ocr = MangaOcrInstance::init().unwrap();
        let cargo_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("input/input.jpg")
            .to_str()
            .unwrap()
            .to_string();

        let _string = ocr.run_ocr(&cargo_dir);
        let _string = ocr.run_ocr(&cargo_dir);
        let _string = ocr.run_ocr(&cargo_dir);
        let _string = ocr.run_ocr(&cargo_dir);
        // let _ = test_cmd();
    }

    // #[test]
    // fn test_manga_ocr() {
    //     let mut child = Command::new("cmd")
    //         .args(["/c", "python -X utf8 -m manga_ocr cli cli"])
    //         .stdout(Stdio::piped()) // set up stdout so we can read it
    //         .stderr(Stdio::piped()) // set up stdout so we can read it
    //         .stdin(Stdio::piped()) // set up stdin so we can write on it
    //         .spawn()
    //         .expect("Could not run the command");
    //
    //     let mut child_stdout = child.stdout.take().unwrap();
    //     let mut child_stdin = child.stdin.take().unwrap();
    //
    //     thread::spawn(move || {
    //         let mut s: [u8; 500] = [0; 500];
    //         loop {
    //             child_stdout.read(&mut s);
    //             let output = String::from_utf8_lossy(&s);
    //             info!("???? {}", output);
    //             sleep(Duration::from_secs(1))
    //         }
    //     });
    //
    //     loop {
    //         child_stdin.write_all("C:/Users/bluek/Pictures/Ocr/Unbenannt.PNG".as_bytes());
    //         sleep(Duration::from_secs(1))
    //     }
    //
    //     // let mut stdout = child.stdout.take().expect("Failed to open stdout");
    //     // let mut stdin = child.stdin.take().expect("Failed to open stdin");
    //     // let mut s = String::new();
    //
    //     let stdin = child.stdin.as_mut().unwrap();
    //     let mut stdout = BufReader::new(child.stdout.as_mut().unwrap()).lines();
    //     for i in 10..20 {
    //         info!("????{}", stdout.next().unwrap().unwrap());
    //         stdin.write_all(format!("{}!\n", i).as_bytes()).unwrap();
    //         info!("????{}", stdout.next().unwrap().unwrap());
    //     }
    //
    //     // thread::spawn(move || {
    //     //     let reader = BufReader::new(stdout);
    //     //     for line in reader.lines() {
    //     //         print!("wc responded with:\n{}", line.unwrap());
    //     //     }
    //     //
    //     //     // match stdout.read_to_string(&mut s) {
    //     //     //     Err(why) => panic!("couldn't read wc stdout: {}", why),
    //     //     //     Ok(_) => print!("wc responded with:\n{}", s),
    //     //     // }
    //     // });
    //
    //     sleep(Duration::from_secs(10))
    //
    //     // info!("Obtained: {}", &output);
    //
    //     // let mut stdin = child.stdin.take().expect("Failed to open stdin");
    //     //
    //     // for a in 0..1000 {
    //     //     use std::io::Write;
    //     //     writeln!(&mut stdin, "{}", a).unwrap();
    //     // }
    //     //
    //     // stdin
    //     //     .write_all("C:/Users/bluek/Pictures/Ocr/Unbenannt.PNG".as_bytes())
    //     //     .expect("Failed to write to stdin");
    //     //
    //     // let output = child.wait_with_output().expect("Failed to read stdout");
    //     // assert_eq!(String::from_utf8_lossy(&output.stdout), "!dlrow ,olleH")
    // }

    // #[test]
    // fn interactive_test() {
    //     use interactive_process::InteractiveProcess;
    //     use std::process::Command;
    //
    //     let mut command = Command::new("cmd");
    //     let mut cmd = command.args(["/c", "python -X utf8 -m manga_ocr cli cli"]);
    //     let mut proc = InteractiveProcess::new_with_exit_callback(
    //         &mut cmd,
    //         |line| {
    //             info!("Got: {}", line.unwrap());
    //         },
    //         || info!("Child exited."),
    //     )
    //     .unwrap();
    //
    //     proc.send("data1").unwrap();
    //     sleep(Duration::from_secs(1));
    //     proc.send("data2").unwrap();
    //     sleep(Duration::from_secs(1));
    //     proc.send("data3").unwrap();
    //
    //     info!("{}", proc.wait().unwrap());
    // }

    // #[test]
    // fn test_cmd() -> Result<(), ()> {
    //     use std::io::{BufRead, BufReader};
    //
    //     let mut child = Command::new("cmd")
    //         .args(["/c", "python -X utf8 -m manga_ocr cli cli"])
    //         .stdout(Stdio::piped())
    //         .stdin(Stdio::piped())
    //         .spawn()
    //         .unwrap();
    //     let stdout = child.stdout.take().unwrap();
    //
    //     let mut stdin = child.stdin.take().unwrap();
    //
    //     stdin
    //         .write_all(b"C:/Users/bluek/Pictures/Ocr/Unbenannt.PNG\n")
    //         .unwrap();
    //
    //     // Close stdin to finish and avoid indefinite blocking
    //     // drop(stdin);
    //
    //     // let mut reader = BufReader::new(stdout);
    //
    //     // for line in BufReader::new(stdout).lines() {
    //     //     info!("??{}", line.unwrap());
    //     // }
    //
    //     thread::spawn(move || {
    //         for line in BufReader::new(stdout).lines() {
    //             info!("??{}", line.unwrap());
    //         }
    //         // exit_callback();
    //     });
    //
    //     // let mut buf = String::new();
    //     // let mut iter = stdout.read_to_string(&mut buf);
    //
    //     // stdout.read_to_string()
    //
    //     // info!("??{:?}", buf);
    //
    //     stdin
    //         .write_all(b"C:/Users/bluek/Pictures/Ocr/Unbenannt.PNG\n")
    //         .unwrap();
    //     // iter
    //     //     .take_while(|line| !line.contains("cli"))
    //     //     .for_each(|line| {
    //     //         info!("??{}", line);
    //     //         let _ = dbg!(stdin.write_all(b"Hello, world!\n"));
    //     //     });
    //
    //     Ok(())
    // }
}
