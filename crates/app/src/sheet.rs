//! Setup sheets: the HTML page a CAM makes for the operator (stock, work
//! zero, fixtures, tools), shown beside the program it belongs to.
//!
//! A sheet comes with a program sent over the CAM link, or sits next to an
//! opened G-code file (`part.html` or `part setup sheet.html` for
//! `part.nc`). On Windows it is shown inside rcnc by WebView2, the Chromium
//! engine that ships with Windows; elsewhere it opens in the browser.
//!
//! The web view lives on the UI thread but renders in its own browser
//! processes, and the machine is polled on the controller's thread, so
//! nothing a page does can hold up motion. The sheet never keeps the
//! keyboard: a click in it takes the focus, and rcnc takes it straight back,
//! so Esc, Space and the jog keys always reach rcnc and its buttons keep
//! working.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Sheets sent with programs over the CAM link.
pub struct Sheets {
    dir: PathBuf,
    /// The last program sent and its sheet, until the UI sees it loaded.
    sent: Mutex<Option<(String, Option<PathBuf>)>>,
}

impl Sheets {
    pub fn new(dir: PathBuf) -> Self {
        Sheets { dir, sent: Mutex::new(None) }
    }

    /// Note the sheet for a program about to be loaded: save it, or note that it has none.
    pub fn sent(&self, program: &str, html: Option<&str>) -> std::io::Result<()> {
        let path = match html.filter(|h| !h.trim().is_empty()) {
            Some(html) => {
                std::fs::create_dir_all(&self.dir)?;
                let stem: String = Path::new(program)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
                    .chars()
                    .map(|c| if c.is_alphanumeric() || "-_ ".contains(c) { c } else { '_' })
                    .collect();
                let path = self.dir.join(format!("{}.html", if stem.is_empty() { "program" } else { &stem }));
                std::fs::write(&path, html)?;
                Some(path)
            }
            None => None,
        };
        *self.sent.lock().unwrap() = Some((program.to_string(), path));
        Ok(())
    }

    /// The sheet sent with `program`, if that is the program just sent.
    /// The outer None means it didn't come over the link.
    pub fn take(&self, program: &str) -> Option<Option<PathBuf>> {
        let mut sent = self.sent.lock().unwrap();
        match sent.as_ref() {
            Some((name, _)) if name == program => sent.take().map(|(_, p)| p),
            _ => None,
        }
    }
}

/// A sheet saved next to a G-code file.
pub fn beside(program: &Path) -> Option<PathBuf> {
    let stem = program.file_stem()?.to_string_lossy().into_owned();
    let dir = program.parent()?;
    [
        format!("{stem}.html"),
        format!("{stem}.htm"),
        format!("{stem} setup sheet.html"),
        format!("{stem}_setup_sheet.html"),
        format!("{stem}-setup-sheet.html"),
    ]
    .into_iter()
    .map(|n| dir.join(n))
    .find(|p| p.is_file())
}

/// Open a sheet in the system's browser.
pub fn open_in_browser(path: &Path) {
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(path).spawn();
    #[cfg(not(any(windows, target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(path).spawn();
    if let Err(e) = r {
        eprintln!("could not open {}: {e}", path.display());
    }
}

/// Esc and Space pressed inside the page in the moment before rcnc takes the
/// keyboard back, sent on as they would be from rcnc's own window.
#[cfg_attr(not(windows), allow(dead_code))]
const KEYS_SCRIPT: &str = r#"
window.addEventListener('keydown', function (e) {
  if (e.key === 'Escape') { window.ipc.postMessage('stop'); }
  else if (e.key === ' ' && !e.repeat) { window.ipc.postMessage('hold'); }
}, true);
"#;

#[cfg(windows)]
mod view {
    use std::path::{Path, PathBuf};

    use eframe::egui;
    use machine::{Command, ControllerHandle, Mode};
    use wry::dpi::{PhysicalPosition, PhysicalSize};
    use wry::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use wry::{NewWindowResponse, Rect, WebContext, WebView, WebViewBuilder};

    /// The web view showing the sheet, made the first time a sheet is shown.
    pub struct SheetView {
        web: Option<(WebView, WebContext)>,
        data: PathBuf,
        shown: Option<PathBuf>,
        bounds: Option<Rect>,
        visible: bool,
        error: Option<String>,
    }

    impl SheetView {
        pub fn new(data: PathBuf) -> Self {
            SheetView { web: None, data, shown: None, bounds: None, visible: false, error: None }
        }

        /// Why the sheet can't be shown here, if it can't.
        pub fn error(&self) -> Option<&str> {
            self.error.as_deref()
        }

        /// Show `path` over `rect` (in egui points).
        pub fn show(&mut self, frame: &eframe::Frame, ctx: &egui::Context, rect: egui::Rect, path: &Path, machine: &ControllerHandle) {
            if self.error.is_some() {
                return;
            }
            let ppp = ctx.pixels_per_point();
            let bounds = Rect {
                position: PhysicalPosition::new((rect.min.x * ppp).round() as i32, (rect.min.y * ppp).round() as i32).into(),
                size: PhysicalSize::new((rect.width() * ppp).round().max(1.0) as u32, (rect.height() * ppp).round().max(1.0) as u32).into(),
            };
            let Some(url) = file_url(path) else {
                self.error = Some(format!("can't show {}", path.display()));
                return;
            };
            if self.web.is_none() {
                match self.create(frame, bounds, &url, machine) {
                    Ok(w) => {
                        self.web = Some(w);
                        self.shown = Some(path.to_path_buf());
                        self.bounds = Some(bounds);
                        self.visible = true;
                    }
                    Err(e) => self.error = Some(format!("The web view (WebView2) couldn't start: {e}")),
                }
                return;
            }
            let Some((web, _)) = &self.web else { return };
            if self.shown.as_deref() != Some(path) {
                let _ = web.load_url(&url);
                self.shown = Some(path.to_path_buf());
            }
            if self.bounds != Some(bounds) {
                let _ = web.set_bounds(bounds);
                self.bounds = Some(bounds);
            }
            if !self.visible {
                let _ = web.set_visible(true);
                self.visible = true;
            }
        }

        /// Take the keyboard back from the page. Called every frame: a click
        /// in the page gives it the focus, and while it has it, Esc wouldn't
        /// reach rcnc and egui would ignore clicks on its own buttons.
        pub fn keep_keyboard(&self, frame: &eframe::Frame) {
            let Some((web, _)) = &self.web else { return };
            let Ok(handle) = frame.window_handle() else { return };
            let RawWindowHandle::Win32(h) = handle.as_raw() else { return };
            let ours = h.hwnd.get();
            // Only while rcnc is the active window, so another program never loses the keyboard to it.
            let (active, focus) = unsafe { (GetForegroundWindow(), GetFocus()) };
            if active == ours && focus != ours {
                let _ = web.focus_parent();
            }
        }

        pub fn hide(&mut self) {
            if let (Some((web, _)), true) = (&self.web, self.visible) {
                let _ = web.set_visible(false);
                // Give the keyboard back to rcnc's window, for the jog keys.
                let _ = web.focus_parent();
                self.visible = false;
            }
        }

        /// Reload the sheet, after its file changed.
        pub fn reload(&mut self) {
            if let Some((web, _)) = &self.web {
                if let Some(url) = self.shown.as_deref().and_then(file_url) {
                    let _ = web.load_url(&url);
                }
            }
        }

        pub fn print(&self) {
            if let Some((web, _)) = &self.web {
                let _ = web.print();
            }
        }

        fn create(&mut self, frame: &eframe::Frame, bounds: Rect, url: &str, machine: &ControllerHandle) -> wry::Result<(WebView, WebContext)> {
            let mut context = WebContext::new(Some(self.data.clone()));
            let machine = machine.clone();
            let web = WebViewBuilder::new_with_web_context(&mut context)
                .with_bounds(bounds)
                .with_url(url)
                .with_focused(false)
                .with_background_color((0xff, 0xff, 0xff, 0xff))
                .with_initialization_script(super::KEYS_SCRIPT)
                .with_ipc_handler(move |req| match req.body().as_str() {
                    "stop" => machine.send(Command::Stop),
                    "hold" if machine.snapshot().mode == Mode::Running => machine.send(Command::FeedHold),
                    _ => {}
                })
                // A sheet is a local page: links don't take it elsewhere or open windows.
                .with_navigation_handler(|url| url.starts_with("file:") || url.starts_with("about:") || url.starts_with("data:"))
                .with_new_window_req_handler(|_, _| NewWindowResponse::Deny)
                .build_as_child(frame)?;
            Ok((web, context))
        }
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> isize;
        fn GetFocus() -> isize;
    }

    fn file_url(path: &Path) -> Option<String> {
        let path = std::path::absolute(path).ok()?;
        let mut url = String::from("file:///");
        let text = path.to_string_lossy().replace('\\', "/");
        let text = text.strip_prefix("//?/").unwrap_or(&text);
        for b in text.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b':' | b'-' | b'_' | b'.' | b'~' => url.push(b as char),
                _ => url.push_str(&format!("%{b:02X}")),
            }
        }
        Some(url)
    }
}

#[cfg(not(windows))]
mod view {
    use std::path::{Path, PathBuf};

    use eframe::egui;
    use machine::ControllerHandle;

    /// Without WebView2 the sheet opens in the browser instead.
    pub struct SheetView;

    impl SheetView {
        pub fn new(_data: PathBuf) -> Self {
            SheetView
        }

        pub fn error(&self) -> Option<&str> {
            Some("Setup sheets show inside rcnc on Windows; here they open in the browser.")
        }

        pub fn show(&mut self, _: &eframe::Frame, _: &egui::Context, _: egui::Rect, _: &Path, _: &ControllerHandle) {}

        /// Take the keyboard back from the page. Called every frame: a click
        /// in the page gives it the focus, and while it has it, Esc wouldn't
        /// reach rcnc and egui would ignore clicks on its own buttons.
        pub fn keep_keyboard(&self, frame: &eframe::Frame) {
            let Some((web, _)) = &self.web else { return };
            let Ok(handle) = frame.window_handle() else { return };
            let RawWindowHandle::Win32(h) = handle.as_raw() else { return };
            let ours = h.hwnd.get();
            // Only while rcnc is the active window, so another program never loses the keyboard to it.
            let (active, focus) = unsafe { (GetForegroundWindow(), GetFocus()) };
            if active == ours && focus != ours {
                let _ = web.focus_parent();
            }
        }

        pub fn keep_keyboard(&self, _: &eframe::Frame) {}

        pub fn hide(&mut self) {}

        pub fn reload(&mut self) {}

        pub fn print(&self) {}
    }
}

pub use view::SheetView;

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rcnc-sheet-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn sent_sheets_belong_to_their_program() {
        let d = temp("sent");
        let sheets = Sheets::new(d.join("sheets"));
        sheets.sent("bracket.nc", Some("<h1>Bracket</h1>")).unwrap();
        assert_eq!(sheets.take("other.nc"), None);
        let path = sheets.take("bracket.nc").unwrap().unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "<h1>Bracket</h1>");
        assert_eq!(sheets.take("bracket.nc"), None, "taken once");
        sheets.sent("plate.nc", None).unwrap();
        assert_eq!(sheets.take("plate.nc"), Some(None));
    }

    #[test]
    fn finds_sheets_beside_programs() {
        let d = temp("beside");
        let nc = d.join("part.nc");
        std::fs::write(&nc, "M30").unwrap();
        assert_eq!(beside(&nc), None);
        std::fs::write(d.join("part setup sheet.html"), "x").unwrap();
        assert_eq!(beside(&nc), Some(d.join("part setup sheet.html")));
        std::fs::write(d.join("part.html"), "x").unwrap();
        assert_eq!(beside(&nc), Some(d.join("part.html")));
    }
}
