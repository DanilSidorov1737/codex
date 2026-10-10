//! Actual Neovim's pipe RPC and external line grid. Neovim owns all editing.
//! Deliberately one child/one grid, with bounded transport and no project config.

use crate::tui::FrameRequester;
use anyhow::{Context, Result, bail, ensure};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};
use rmpv::Value;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const MAX_MESSAGE: usize = 16 * 1024 * 1024;
const MAX_COLUMNS: u16 = 512;
const MAX_ROWS: u16 = 256;

struct Limited<R> {
    inner: R,
    remaining: usize,
}
impl<R: Read> Read for Limited<R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(std::io::Error::other("Neovim message exceeds 16 MiB"));
        }
        let count = bytes.len().min(self.remaining);
        let read = self.inner.read(&mut bytes[..count])?;
        self.remaining -= read;
        Ok(read)
    }
}

#[derive(Clone, Default)]
struct Grid {
    width: u16,
    height: u16,
    cells: Vec<(String, u64)>,
    cursor: (u16, u16),
}

pub(crate) struct Editor {
    child: Child,
    writer: Option<mpsc::SyncSender<Value>>,
    reader: Option<mpsc::Receiver<Result<Value, String>>>,
    threads: Vec<thread::JoinHandle<()>>,
    next_id: u64,
    pending: Grid,
    visible: Grid,
    highlights: BTreeMap<u64, Style>,
    default_style: Style,
    size: (u16, u16),
    pub(crate) focused: bool,
    pub(crate) error: Option<String>,
    pub(crate) mode: String,
    frozen: Option<Value>,
}
impl Drop for Editor {
    fn drop(&mut self) {
        // Free a reader blocked on a full queue before joining its thread.
        self.reader.take();
        self.writer.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}
impl Editor {
    pub(crate) fn start(frame: FrameRequester) -> Result<Self> {
        let mut child = Command::new("nvim")
            .args([
                "--embed",
                "-u",
                "NONE",
                "--noplugin",
                "-i",
                "NONE",
                "--cmd",
                "set nomodeline noexrc noswapfile",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("launch official Neovim: install a compatible nvim on PATH")?;
        let mut stdin = child.stdin.take().context("Neovim stdin missing")?;
        let stdout = child.stdout.take().context("Neovim stdout missing")?;
        let (writer, outgoing) = mpsc::sync_channel(8);
        let (incoming, reader) = mpsc::sync_channel(8);
        let read_thread = thread::spawn(move || {
            let mut stream = Limited {
                inner: stdout,
                remaining: MAX_MESSAGE,
            };
            loop {
                stream.remaining = MAX_MESSAGE;
                let value = rmpv::decode::read_value_with_max_depth(&mut stream, 64)
                    .map_err(|e| format!("Neovim RPC closed or invalid: {e}"));
                let failed = value.is_err();
                if incoming.send(value).is_err() {
                    break;
                }
                frame.schedule_frame();
                if failed {
                    break;
                }
            }
        });
        let write_thread = thread::spawn(move || {
            while let Ok(value) = outgoing.recv() {
                if rmpv::encode::write_value(&mut stdin, &value).is_err() || stdin.flush().is_err()
                {
                    break;
                }
            }
        });
        let mut editor = Self {
            child,
            writer: Some(writer),
            reader: Some(reader),
            threads: vec![read_thread, write_thread],
            next_id: 0,
            pending: Grid::default(),
            visible: Grid::default(),
            highlights: BTreeMap::new(),
            default_style: Style::default(),
            size: (80, 20),
            focused: true,
            error: None,
            mode: "normal".into(),
            frozen: None,
        };
        let info = editor.rpc("nvim_get_api_info", vec![])?;
        let metadata = array(&info)?
            .get(1)
            .context("missing Neovim API metadata")?;
        let version = map_get(metadata, "version").context("missing Neovim version")?;
        ensure!(
            map_get(version, "major")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
                || map_get(version, "minor")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    >= 10,
            "Neovim 0.10 or newer required"
        );
        editor.rpc(
            "nvim_ui_attach",
            vec![
                80.into(),
                20.into(),
                Value::Map(vec![
                    ("rgb".into(), true.into()),
                    ("ext_linegrid".into(), true.into()),
                ]),
            ],
        )?;
        editor.rpc(
            "nvim_set_option_value",
            vec!["mouse".into(), "a".into(), Value::Map(vec![])],
        )?;
        Ok(editor)
    }
    fn send(&self, value: Value) -> Result<()> {
        self.writer
            .as_ref()
            .context("Neovim writer closed")?
            .try_send(value)
            .context("Neovim input queue full or closed; input was not sent")
    }
    pub(crate) fn rpc(&mut self, method: &str, params: Vec<Value>) -> Result<Value> {
        ensure!(
            self.error.is_none(),
            "Neovim unavailable; reopen editor before retrying"
        );
        let result = self.rpc_inner(method, params);
        if let Err(error) = &result {
            self.error = Some(error.to_string());
        }
        result
    }
    fn rpc_inner(&mut self, method: &str, params: Vec<Value>) -> Result<Value> {
        ensure!(
            self.error.is_none(),
            "Neovim is unavailable: {:?}",
            self.error
        );
        self.next_id += 1;
        let id = self.next_id;
        self.send(Value::Array(vec![
            0.into(),
            id.into(),
            method.into(),
            Value::Array(params),
        ]))?;
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            ensure!(
                !remaining.is_zero(),
                "Neovim RPC timed out; outcome unknown, reopen editor before retrying"
            );
            let value = self
                .reader
                .as_ref()
                .context("Neovim reader closed")?
                .recv_timeout(remaining)
                .context("Neovim RPC timed out or closed")?
                .map_err(anyhow::Error::msg)?;
            let message = array(&value)?;
            if message.first().and_then(Value::as_u64) == Some(1) {
                ensure!(
                    message.get(1).and_then(Value::as_u64) == Some(id),
                    "unexpected Neovim response ID"
                );
                ensure!(
                    message.get(2) == Some(&Value::Nil),
                    "Neovim rejected {method}: {:?}",
                    message.get(2)
                );
                return message
                    .get(3)
                    .cloned()
                    .context("Neovim response missing result");
            }
            self.notification(&value)?;
        }
    }
    pub(crate) fn poll(&mut self) {
        for _ in 0..64 {
            let next = self.reader.as_ref().map(mpsc::Receiver::try_recv);
            match next {
                Some(Ok(Ok(value))) => {
                    if let Err(e) = self.notification(&value) {
                        self.error = Some(e.to_string());
                        break;
                    }
                }
                Some(Ok(Err(e))) => {
                    self.error = Some(e);
                    break;
                }
                Some(Err(mpsc::TryRecvError::Disconnected)) => {
                    self.error =
                        Some("Neovim exited; saved files remain, unsaved buffers are lost".into());
                    break;
                }
                _ => break,
            }
        }
    }
    fn notification(&mut self, value: &Value) -> Result<()> {
        let message = array(value)?;
        match message.first().and_then(Value::as_u64) {
            Some(0) => {
                // No editor request may invoke daemon/owner commands.
                let id = message
                    .get(1)
                    .context("invalid nested Neovim request")?
                    .clone();
                self.send(Value::Array(vec![
                    1.into(),
                    id,
                    Value::Array(vec![0.into(), "host method unsupported".into()]),
                    Value::Nil,
                ]))?;
            }
            Some(2) if message.get(1).and_then(Value::as_str) == Some("redraw") => {
                for event in array(message.get(2).context("missing redraw")?)? {
                    let event = array(event)?;
                    let name = event
                        .first()
                        .and_then(Value::as_str)
                        .context("invalid redraw name")?;
                    for params in event.iter().skip(1) {
                        self.redraw(name, array(params)?)?;
                    }
                }
            }
            Some(2) => {}
            _ => bail!("unexpected Neovim message"),
        }
        Ok(())
    }
    fn redraw(&mut self, event: &str, p: &[Value]) -> Result<()> {
        let number = |i: usize| -> Result<u64> {
            p.get(i)
                .and_then(Value::as_u64)
                .context("invalid Neovim grid coordinate")
        };
        match event {
            "grid_resize" => {
                ensure!(number(0)? == 1, "unsupported Neovim multigrid");
                let (w, h) = (number(1)?, number(2)?);
                ensure!(
                    w > 0 && w <= u64::from(MAX_COLUMNS) && h > 0 && h <= u64::from(MAX_ROWS),
                    "Neovim grid exceeds 512×256"
                );
                self.pending.width = w as u16;
                self.pending.height = h as u16;
                self.pending.cells = vec![(" ".into(), 0); (w * h) as usize];
            }
            "grid_clear" => {
                ensure!(number(0)? == 1, "unsupported grid");
                self.pending.cells.fill((" ".into(), 0));
            }
            "grid_cursor_goto" => {
                ensure!(number(0)? == 1, "unsupported grid");
                self.pending.cursor = (number(2)? as u16, number(1)? as u16);
            }
            "grid_line" => {
                ensure!(number(0)? == 1, "unsupported grid");
                let row = number(1)? as usize;
                let mut col = number(2)? as usize;
                let mut highlight = 0;
                ensure!(
                    row < usize::from(self.pending.height),
                    "Neovim row out of bounds"
                );
                for cell in array(p.get(3).context("missing grid cells")?)? {
                    let cell = array(cell)?;
                    let text = cell
                        .first()
                        .and_then(Value::as_str)
                        .context("missing cell text")?;
                    if let Some(id) = cell.get(1) {
                        highlight = id.as_u64().context("invalid highlight ID")?;
                    }
                    let repeat = cell.get(2).and_then(Value::as_u64).unwrap_or(1);
                    ensure!(
                        repeat <= u64::from(self.pending.width),
                        "invalid grid repeat"
                    );
                    for _ in 0..repeat {
                        ensure!(
                            col < usize::from(self.pending.width),
                            "Neovim column out of bounds"
                        );
                        self.pending.cells[row * usize::from(self.pending.width) + col] =
                            (text.into(), highlight);
                        col += 1;
                    }
                }
            }
            "grid_scroll" => {
                ensure!(number(0)? == 1, "unsupported grid");
                let (top, bottom, left, right) = (
                    number(1)? as usize,
                    number(2)? as usize,
                    number(3)? as usize,
                    number(4)? as usize,
                );
                ensure!(
                    bottom <= usize::from(self.pending.height)
                        && right <= usize::from(self.pending.width)
                        && top <= bottom
                        && left <= right,
                    "invalid scroll region"
                );
                let rows = p
                    .get(5)
                    .and_then(Value::as_i64)
                    .context("invalid scroll delta")?;
                let cols = p
                    .get(6)
                    .and_then(Value::as_i64)
                    .context("invalid scroll delta")?;
                let old = self.pending.cells.clone();
                let width = usize::from(self.pending.width);
                for y in top..bottom {
                    for x in left..right {
                        let (sy, sx) = (y as i64 + rows, x as i64 + cols);
                        self.pending.cells[y * width + x] = if sy >= top as i64
                            && sy < bottom as i64
                            && sx >= left as i64
                            && sx < right as i64
                        {
                            old[sy as usize * width + sx as usize].clone()
                        } else {
                            (" ".into(), 0)
                        };
                    }
                }
            }
            "hl_attr_define" => {
                let id = number(0)?;
                ensure!(id < 65536, "too many Neovim highlights");
                self.highlights.insert(
                    id,
                    highlight(p.get(1).context("missing highlight")?, self.default_style),
                );
            }
            "default_colors_set" => {
                self.default_style = Style::default().fg(color(p.first())).bg(color(p.get(1)));
            }
            "mode_change" => {
                self.mode = p.first().and_then(Value::as_str).unwrap_or("normal").into();
            }
            "flush" => {
                self.visible = self.pending.clone();
            }
            // Documented compatible notifications: native Neovim draws its own
            // command line, messages, menus and splits because no other UI extensions attach.
            "mode_info_set" | "option_set" | "mouse_on" | "mouse_off" | "busy_start"
            | "busy_stop" | "bell" | "visual_bell" | "set_title" | "set_icon" | "suspend"
            | "update_menu" | "chdir" => {}
            _ => {}
        }
        Ok(())
    }
    pub(crate) fn resize(&mut self, width: u16, height: u16) -> Result<()> {
        let size = (width.clamp(1, MAX_COLUMNS), height.clamp(1, MAX_ROWS));
        if size != self.size {
            self.rpc("nvim_ui_try_resize", vec![size.0.into(), size.1.into()])?;
            self.size = size;
        }
        Ok(())
    }
    pub(crate) fn key(&mut self, key: KeyEvent) -> Result<()> {
        ensure!(
            self.frozen.is_none(),
            "editor input frozen during application"
        );
        let input = key_text(key);
        if input.is_empty() {
            return Ok(());
        }
        let mut offset = 0;
        // nvim_input reports a byte count; preserve partial acceptance exactly.
        for _ in 0..8 {
            let accepted = self
                .rpc("nvim_input", vec![input[offset..].into()])?
                .as_u64()
                .context("invalid input receipt")? as usize;
            ensure!(
                accepted > 0
                    && input.is_char_boundary(offset + accepted)
                    && offset + accepted <= input.len(),
                "Neovim input stalled; remaining input not delivered"
            );
            offset += accepted;
            if offset == input.len() {
                return Ok(());
            }
        }
        bail!("Neovim input remained backpressured")
    }
    pub(crate) fn paste(&mut self, text: &str) -> Result<()> {
        ensure!(
            self.frozen.is_none(),
            "editor input frozen during application"
        );
        ensure!(text.len() <= 1024 * 1024, "paste exceeds 1 MiB");
        ensure!(
            self.rpc("nvim_paste", vec![text.into(), false.into(), (-1).into()])?
                .as_bool()
                == Some(true),
            "Neovim cancelled paste"
        );
        Ok(())
    }
    pub(crate) fn mouse(&mut self, event: MouseEvent, area: Rect) -> Result<()> {
        if self.frozen.is_some()
            || !self.focused
            || !area.contains((event.column, event.row).into())
        {
            return Ok(());
        }
        let (button, action) = match event.kind {
            MouseEventKind::Down(b) => (format!("{b:?}").to_lowercase(), "press"),
            MouseEventKind::Up(b) => (format!("{b:?}").to_lowercase(), "release"),
            MouseEventKind::Drag(b) => (format!("{b:?}").to_lowercase(), "drag"),
            MouseEventKind::ScrollUp => ("wheel".into(), "up"),
            MouseEventKind::ScrollDown => ("wheel".into(), "down"),
            MouseEventKind::ScrollLeft => ("wheel".into(), "left"),
            MouseEventKind::ScrollRight => ("wheel".into(), "right"),
            _ => return Ok(()),
        };
        self.rpc(
            "nvim_input_mouse",
            vec![
                button.into(),
                action.into(),
                modifiers(event.modifiers).into(),
                0.into(),
                (event.row - area.y).into(),
                (event.column - area.x).into(),
            ],
        )?;
        Ok(())
    }
    pub(crate) fn dirty(&mut self) -> Result<bool> {
        self.rpc("nvim_exec_lua",vec!["for _,b in ipairs(vim.api.nvim_list_bufs()) do if vim.api.nvim_buf_is_loaded(b) and vim.bo[b].modified then return true end end return false".into(),Value::Array(vec![])])?.as_bool().context("invalid dirty-buffer response")
    }
    pub(crate) fn freeze(&mut self) -> Result<()> {
        ensure!(self.frozen.is_none(), "editor already frozen");
        // Check all dirty flags/capture changedticks and freeze text mutation in
        // one Neovim main-loop operation. No model-supplied script is executed.
        let state=self.rpc("nvim_exec_lua",vec!["local r={}; for _,b in ipairs(vim.api.nvim_list_bufs()) do if vim.api.nvim_buf_is_loaded(b) then if vim.bo[b].modified then return false end; table.insert(r,{b,vim.api.nvim_buf_get_changedtick(b),vim.bo[b].modifiable}) end end; for _,s in ipairs(r) do vim.bo[s[1]].modifiable=false end; return r".into(),Value::Array(vec![])])?;
        ensure!(
            state.as_bool() != Some(false),
            "save or discard modified Neovim buffers before applying"
        );
        self.frozen = Some(state);
        Ok(())
    }
    pub(crate) fn thaw(&mut self) -> Result<()> {
        if let Some(state) = self.frozen.take() {
            let unchanged=self.rpc("nvim_exec_lua",vec!["local ok=true; for _,s in ipairs(...) do if vim.api.nvim_buf_is_valid(s[1]) then if vim.api.nvim_buf_get_changedtick(s[1])~=s[2] then ok=false end; vim.bo[s[1]].modifiable=s[3] end end; return ok".into(),Value::Array(vec![state])])?;
            ensure!(
                unchanged.as_bool() == Some(true),
                "editor revision changed during application; inspect buffers before reloading"
            );
        }
        Ok(())
    }
    fn command(&mut self, command: &str, args: Vec<Value>) -> Result<()> {
        self.rpc(
            "nvim_cmd",
            vec![
                Value::Map(vec![
                    ("cmd".into(), command.into()),
                    ("args".into(), Value::Array(args)),
                    (
                        "magic".into(),
                        Value::Map(vec![
                            ("bar".into(), false.into()),
                            ("file".into(), false.into()),
                        ]),
                    ),
                ]),
                Value::Map(vec![]),
            ],
        )?;
        Ok(())
    }
    pub(crate) fn open(&mut self, path: &Path, readonly: bool) -> Result<()> {
        ensure!(
            !self.dirty()?,
            "save or discard modified Neovim buffers before changing host file mode"
        );
        self.command(
            "edit",
            vec![path.to_str().context("non-UTF8 file path")?.into()],
        )?;
        self.rpc(
            "nvim_set_option_value",
            vec![
                "readonly".into(),
                readonly.into(),
                Value::Map(vec![("buf".into(), 0.into())]),
            ],
        )?;
        self.rpc(
            "nvim_set_option_value",
            vec![
                "modifiable".into(),
                (!readonly).into(),
                Value::Map(vec![("buf".into(), 0.into())]),
            ],
        )?;
        self.focused = true;
        Ok(())
    }
    pub(crate) fn diff(&mut self, base: &Path, artifact: &Path) -> Result<()> {
        ensure!(
            !self.dirty()?,
            "modified Neovim buffers block changing diff mode"
        );
        self.command("only", vec![])?;
        self.open(base, true)?;
        self.command("diffthis", vec![])?;
        self.command(
            "vsplit",
            vec![artifact.to_str().context("non-UTF8 artifact")?.into()],
        )?;
        self.command("diffthis", vec![])?;
        for option in ["readonly", "modifiable"] {
            self.rpc(
                "nvim_set_option_value",
                vec![
                    option.into(),
                    (option == "readonly").into(),
                    Value::Map(vec![("buf".into(), 0.into())]),
                ],
            )?;
        }
        self.focused = true;
        Ok(())
    }
    pub(crate) fn reload_follow(&mut self) -> Result<()> {
        if self.focused || self.dirty()? {
            return Ok(());
        }
        // One native main-loop operation retains views and never touches dirty buffers.
        self.rpc(
            "nvim_exec_lua",
            vec![
                "local v=vim.fn.winsaveview(); vim.api.nvim_cmd({cmd='checktime',args={tostring(vim.api.nvim_get_current_buf())}},{}); vim.fn.winrestview(v)".into(),
                Value::Array(vec![]),
            ],
        )?;
        Ok(())
    }
    pub(crate) fn render(&self, area: Rect, buffer: &mut Buffer) -> Option<(u16, u16)> {
        for y in 0..area.height.min(self.visible.height) {
            for x in 0..area.width.min(self.visible.width) {
                let (text, id) = &self.visible.cells
                    [usize::from(y) * usize::from(self.visible.width) + usize::from(x)];
                let cell = &mut buffer[(area.x + x, area.y + y)];
                // Empty cells are native double-width continuations.
                cell.set_symbol(if text.is_empty() { " " } else { text });
                cell.set_diff_option(if text.is_empty() {
                    ratatui::buffer::CellDiffOption::Skip
                } else {
                    ratatui::buffer::CellDiffOption::None
                });
                cell.set_style(
                    self.highlights
                        .get(id)
                        .copied()
                        .unwrap_or(self.default_style),
                );
            }
        }
        let (x, y) = self.visible.cursor;
        (self.focused && x < area.width && y < area.height).then_some((area.x + x, area.y + y))
    }
}
fn array(value: &Value) -> Result<&[Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .context("expected Neovim RPC array")
}
fn map_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value
        .as_map()?
        .iter()
        .find(|(k, _)| k.as_str() == Some(key))
        .map(|(_, v)| v)
}
#[expect(
    clippy::disallowed_methods,
    reason = "preserve actual Neovim external UI RGB highlights rather than substituting a host palette"
)]
fn color(value: Option<&Value>) -> Color {
    value
        .and_then(Value::as_i64)
        .filter(|v| *v >= 0)
        .map(|v| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
        .unwrap_or(Color::Reset)
}
fn highlight(value: &Value, default: Style) -> Style {
    let mut style = default;
    if let Some(v) = map_get(value, "foreground") {
        style = style.fg(color(Some(v)));
    }
    if let Some(v) = map_get(value, "background") {
        style = style.bg(color(Some(v)));
    }
    for (key, modifier) in [
        ("bold", Modifier::BOLD),
        ("italic", Modifier::ITALIC),
        ("underline", Modifier::UNDERLINED),
        ("undercurl", Modifier::UNDERLINED),
        ("reverse", Modifier::REVERSED),
        ("strikethrough", Modifier::CROSSED_OUT),
    ] {
        if map_get(value, key).and_then(Value::as_bool) == Some(true) {
            style = style.add_modifier(modifier);
        }
    }
    style
}
fn modifiers(m: KeyModifiers) -> String {
    let mut s = String::new();
    if m.contains(KeyModifiers::CONTROL) {
        s.push_str("C-");
    }
    if m.contains(KeyModifiers::ALT) {
        s.push_str("M-");
    }
    if m.contains(KeyModifiers::SHIFT) {
        s.push_str("S-");
    }
    s
}
fn key_text(key: KeyEvent) -> String {
    if key.kind == crossterm::event::KeyEventKind::Release {
        return String::new();
    }
    if let KeyCode::Char(c) = key.code {
        if !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return if c == '<' {
                "<lt>".into()
            } else {
                c.to_string()
            };
        }
        return format!("<{}{c}>", modifiers(key.modifiers));
    }
    let name = match key.code {
        KeyCode::Enter => "CR".into(),
        KeyCode::Esc => "Esc".into(),
        KeyCode::Backspace => "BS".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::BackTab => return "<S-Tab>".into(),
        KeyCode::Left => "Left".into(),
        KeyCode::Right => "Right".into(),
        KeyCode::Up => "Up".into(),
        KeyCode::Down => "Down".into(),
        KeyCode::Home => "Home".into(),
        KeyCode::End => "End".into(),
        KeyCode::PageUp => "PageUp".into(),
        KeyCode::PageDown => "PageDown".into(),
        KeyCode::Delete => "Del".into(),
        KeyCode::Insert => "Insert".into(),
        KeyCode::F(n) => format!("F{n}"),
        _ => return String::new(),
    };
    format!("<{}{name}>", modifiers(key.modifiers))
}

#[cfg(test)]
#[path = "organization_editor_tests.rs"]
mod tests;
