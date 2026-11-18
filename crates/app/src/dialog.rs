//! File dialogs in a child process: rcnc starts itself again with `--pick-file`, which shows the dialog
//! and prints the chosen path. Windows' file dialog loads the shell's extensions (OneDrive, archivers,
//! preview handlers) into whatever process opens it, and one of them crashing takes that process down. With the
//! dialog in a child, it takes only the child: the controller, the motion stream and the listening hold carry
//! on. The window also stays live while the dialog is open (no modal loop on the UI thread), so Stop, the
//! keys and the hold keep working.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};

/// The argument that makes rcnc show one file dialog and exit.
pub const PICK_FILE: &str = "--pick-file";

/// A dialog open in a child process.
pub struct Pick {
    rx: Receiver<Result<Option<PathBuf>, String>>,
}

impl Pick {
    /// What it picked once it closed: `Ok(None)` cancelled, `Err` if the dialog's process failed.
    pub fn poll(&self) -> Option<Result<Option<PathBuf>, String>> {
        match self.rx.try_recv() {
            Ok(r) => Some(r),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err("the file dialog's process went away".into())),
        }
    }
}

/// Open a file dialog with these filters (name, extensions) in a child process.
pub fn pick_file(title: &str, filters: &[(&str, &[&str])], dir: Option<&Path>) -> Pick {
    let mut args = vec![PICK_FILE.to_string(), "--title".into(), title.into()];
    for (name, exts) in filters {
        args.push("--filter".into());
        args.push(format!("{name}|{}", exts.join(";")));
    }
    if let Some(d) = dir {
        args.push("--dir".into());
        args.push(d.display().to_string());
    }
    let (tx, rx) = channel();
    let _ = std::thread::Builder::new().name("file dialog".into()).spawn(move || {
        let _ = tx.send(run_child(&args));
    });
    Pick { rx }
}

fn run_child(args: &[String]) -> Result<Option<PathBuf>, String> {
    let exe = std::env::current_exe().map_err(|e| format!("can't find rcnc's own program to open the file dialog: {e}"))?;
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().map_err(|e| format!("could not open the file dialog: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "the file dialog crashed ({}); rcnc kept running. A shell extension (OneDrive, an archiver, a preview handler) loaded into the dialog is the usual cause",
            out.status
        ));
    }
    Ok(parse_output(&String::from_utf8_lossy(&out.stdout)))
}

fn parse_output(s: &str) -> Option<PathBuf> {
    let line = s.lines().next()?.trim_end_matches('\r');
    (!line.is_empty()).then(|| PathBuf::from(line))
}

/// The child: show the dialog the arguments describe, print the path (nothing when cancelled), exit.
pub fn child_main(args: &[String]) -> ! {
    let mut d = rfd::FileDialog::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match (a.as_str(), it.clone().next()) {
            ("--title", Some(v)) => {
                d = d.set_title(v);
                it.next();
            }
            ("--filter", Some(v)) => {
                let (name, exts) = v.split_once('|').unwrap_or((v, ""));
                let exts: Vec<&str> = exts.split(';').filter(|e| !e.is_empty()).collect();
                d = d.add_filter(name, &exts);
                it.next();
            }
            ("--dir", Some(v)) => {
                d = d.set_directory(v);
                it.next();
            }
            _ => {}
        }
    }
    if let Some(p) = d.pick_file() {
        println!("{}", p.display());
    }
    std::process::exit(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_childs_answer() {
        assert_eq!(parse_output("C:\\jobs\\op1-top.nc\r\n"), Some(PathBuf::from("C:\\jobs\\op1-top.nc")));
        assert_eq!(parse_output(""), None);
        assert_eq!(parse_output("\n"), None);
    }
}
