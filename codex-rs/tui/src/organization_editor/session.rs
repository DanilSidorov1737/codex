//! Data-only checkpoints and native buffer proposals. Never replay Ex scripts.
use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const MAX_SESSION: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Session {
    version: u32,
    size: (u16, u16),
    buffers: Vec<SavedBuffer>,
    windows: Vec<SavedWindow>,
    layout: Option<Layout>,
    #[serde(default)]
    tabs: Vec<Layout>,
    #[serde(default)]
    active_tab: usize,
    active: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedBuffer {
    path: String,
    base: String,
    lines: Option<Vec<String>>,
    endofline: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedWindow {
    buffer: usize,
    cursor: (usize, usize),
    width: u16,
    height: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
enum Layout {
    Leaf(usize),
    Row(Vec<Layout>),
    Col(Vec<Layout>),
}

pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn project_file(path: &Path, roots: &[PathBuf]) -> Result<Vec<u8>> {
    ensure!(path.is_absolute(), "editor file must be absolute");
    let root = roots
        .iter()
        .find(|root| path.starts_with(root) && path != root.as_path())
        .context("editor file is outside the current project roots")?;
    ensure!(
        std::fs::canonicalize(root)? == *root && std::fs::canonicalize(path)? == path,
        "aliased editor path refused"
    );
    let file = codex_utils_path::open_relative_regular_file(root, path.strip_prefix(root)?)?;
    ensure!(
        file.metadata()?.len() <= 1024 * 1024,
        "session file exceeds 1 MiB"
    );
    let mut bytes = Vec::new();
    file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 1024 * 1024, "session file grew beyond 1 MiB");
    Ok(bytes)
}
impl Session {
    pub(crate) fn has_unsaved(&self) -> bool {
        self.buffers.iter().any(|b| b.lines.is_some())
    }
    pub(crate) fn summary(&self) -> String {
        format!(
            "{} native tabs, {} windows, {} project files, {} unsaved buffers",
            if self.version == 1 {
                1
            } else {
                self.tabs.len()
            },
            self.windows.len(),
            self.buffers.len(),
            self.buffers.iter().filter(|b| b.lines.is_some()).count()
        )
    }
    fn validate(&self, roots: &[PathBuf]) -> Result<()> {
        ensure!(
            self.size.0 > 0
                && self.size.0 <= MAX_COLUMNS
                && self.size.1 > 0
                && self.size.1 <= MAX_ROWS
                && (self.version == 1 || self.version == 2)
                && !self.windows.is_empty()
                && self.windows.len() <= 16
                && self.buffers.len() <= 16
                && self.active < self.windows.len(),
            "invalid native session bounds"
        );
        let mut leaves = Vec::new();
        fn visit(layout: &Layout, depth: usize, leaves: &mut Vec<usize>) -> Result<()> {
            ensure!(
                depth <= 16 && leaves.len() < 16,
                "session layout exceeds bounds"
            );
            match layout {
                Layout::Leaf(i) => leaves.push(*i),
                Layout::Row(children) | Layout::Col(children) => {
                    ensure!((2..=16).contains(&children.len()), "invalid split layout");
                    for child in children {
                        visit(child, depth + 1, leaves)?;
                    }
                }
            }
            Ok(())
        }
        if self.version == 1 {
            ensure!(
                self.tabs.is_empty() && self.active_tab == 0,
                "invalid legacy session tabs"
            );
            visit(
                self.layout.as_ref().context("legacy layout missing")?,
                0,
                &mut leaves,
            )?;
        } else {
            ensure!(
                self.layout.is_none()
                    && (1..=8).contains(&self.tabs.len())
                    && self.active_tab < self.tabs.len(),
                "session supports 1–8 native tabs"
            );
            for (index, tab) in self.tabs.iter().enumerate() {
                let start = leaves.len();
                visit(tab, 0, &mut leaves)?;
                if index == self.active_tab {
                    ensure!(
                        leaves[start..].contains(&self.active),
                        "active window belongs to another tab"
                    );
                }
            }
        }
        leaves.sort_unstable();
        ensure!(
            leaves == (0..self.windows.len()).collect::<Vec<_>>(),
            "session windows must occur exactly once"
        );
        let mut paths = std::collections::BTreeSet::new();
        for buffer in &self.buffers {
            ensure!(paths.insert(&buffer.path), "duplicate session buffer");
            ensure!(
                hash(&project_file(Path::new(&buffer.path), roots)?) == buffer.base,
                "session original changed; checkpoint retained for manual reconciliation"
            );
            if let Some(lines) = &buffer.lines {
                ensure!(
                    !lines.is_empty()
                        && lines.iter().all(|line| !line.contains(['\n', '\0']))
                        && lines.iter().map(String::len).sum::<usize>() + lines.len()
                            <= 1024 * 1024,
                    "invalid unsaved checkpoint text"
                );
            }
        }
        for window in &self.windows {
            ensure!(
                window.buffer < self.buffers.len()
                    && window.cursor.0 > 0
                    && window.width > 0
                    && window.height > 0,
                "invalid session window"
            );
        }
        Ok(())
    }
    pub(crate) fn load(home: &Path, roots: &[PathBuf]) -> Result<Self> {
        let file = codex_utils_path::open_relative_regular_file(
            home,
            Path::new("organization-editor-session.json"),
        )?;
        let metadata = file.metadata()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            ensure!(
                metadata.uid() == unsafe { libc::geteuid() }
                    && metadata.mode() & 0o077 == 0
                    && metadata.nlink() == 1,
                "session checkpoint must be private and owner-controlled"
            );
        }
        ensure!(
            metadata.len() <= MAX_SESSION as u64,
            "session checkpoint exceeds 4 MiB"
        );
        let mut bytes = Vec::new();
        file.take(MAX_SESSION as u64 + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= MAX_SESSION,
            "session checkpoint grew beyond 4 MiB"
        );
        let session: Self = serde_json::from_slice(&bytes)?;
        session.validate(roots)?;
        Ok(session)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BufferPin {
    pub(crate) editor_id: String,
    pub(crate) buffer: i64,
    pub(crate) path: String,
    pub(crate) changedtick: i64,
    pub(crate) sha256: String,
    pub(crate) modified: bool,
}
#[derive(Clone, Debug)]
pub(crate) struct BufferProposal {
    pub(crate) pin: BufferPin,
    pub(crate) text: String,
    pub(crate) result_digest: String,
    pub(crate) artifact_sha256: String,
}

impl BufferProposal {
    pub(crate) fn verify_original(&self, roots: &[PathBuf]) -> Result<()> {
        ensure!(
            hash(&project_file(Path::new(&self.pin.path), roots)?) == self.pin.sha256,
            "original changed since proposal preview"
        );
        Ok(())
    }
}

impl Editor {
    pub(crate) fn save_session(&mut self, home: &Path, roots: &[PathBuf]) -> Result<Session> {
        ensure!(self.frozen.is_none(), "cannot checkpoint a frozen editor");
        let script = r#"
local pages=vim.api.nvim_list_tabpages()
if #pages>8 then error('Session exceeds 8 tabs') end
local wins={}
for _,tab in ipairs(pages) do for _,win in ipairs(vim.api.nvim_tabpage_list_wins(tab)) do table.insert(wins,win) end end
if #wins>16 then error('Session exceeds 16 windows') end
local buffers, windows, wi, bi, visible={},{},{},{},{}
for i,w in ipairs(wins) do wi[w]=i-1; visible[vim.api.nvim_win_get_buf(w)]=true end
for _,b in ipairs(vim.api.nvim_list_bufs()) do
  if vim.api.nvim_buf_is_loaded(b) and vim.bo[b].modified and not visible[b] then error('Hidden unsaved buffer: display it before checkpointing') end
end
local active=0
for i,w in ipairs(wins) do
 local b=vim.api.nvim_win_get_buf(w)
 if vim.bo[b].buftype~='' or vim.bo[b].readonly or not vim.bo[b].modifiable then error('Checkpoint original project files, outside Follow/diff mode') end
 if bi[b]==nil then
  local base=vim.b[b].codex_session_base
  if type(base)~='string' then error('Native file load base unavailable') end
  local lines=vim.bo[b].modified and vim.api.nvim_buf_get_lines(b,0,-1,true) or vim.NIL
  bi[b]=#buffers
  table.insert(buffers,{path=vim.api.nvim_buf_get_name(b),base=base,lines=lines,endofline=vim.bo[b].endofline})
 end
 table.insert(windows,{buffer=bi[b],cursor=vim.api.nvim_win_get_cursor(w),width=vim.api.nvim_win_get_width(w),height=vim.api.nvim_win_get_height(w)})
 if w==vim.api.nvim_get_current_win() then active=i-1 end
end
local function layout(x)
 if x[1]=='leaf' then return {kind='leaf',value=wi[x[2]]} end
 local children={}; for _,child in ipairs(x[2]) do table.insert(children,layout(child)) end
 return {kind=x[1],value=children}
end
local tabs,activeTab={},0
for i,tab in ipairs(pages) do
 table.insert(tabs,layout(vim.fn.winlayout(vim.api.nvim_tabpage_get_number(tab))))
 if tab==vim.api.nvim_get_current_tabpage() then activeTab=i-1 end
end
return vim.json.encode({version=2,size={vim.o.columns,vim.o.lines},buffers=buffers,windows=windows,layout=vim.NIL,tabs=tabs,activeTab=activeTab,active=active})
"#;
        let value = self.guarded_lua(script, vec![])?;
        let bytes = value
            .as_str()
            .context("invalid session response")?
            .as_bytes();
        ensure!(bytes.len() <= MAX_SESSION, "session exceeds 4 MiB");
        let session: Session = serde_json::from_slice(bytes)?;
        session.validate(roots)?;
        let mut file = tempfile::NamedTempFile::new_in(home)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(bytes)?;
        file.as_file().sync_all()?;
        file.persist(home.join("organization-editor-session.json"))?;
        std::fs::File::open(home)?.sync_all()?;
        Ok(session)
    }
    pub(crate) fn restore_session(
        &mut self,
        session: &Session,
        roots: &[PathBuf],
        recover_unsaved: bool,
    ) -> Result<()> {
        ensure!(
            self.frozen.is_none() && !self.dirty()?,
            "save or discard current buffers before restoring a session"
        );
        session.validate(roots)?;
        self.resize(session.size.0, session.size.1)?;
        let json = serde_json::to_string(session)?;
        self.guarded_lua(r#"
local encoded,recover=...; local s=vim.json.decode(encoded)
for _,b in ipairs(vim.api.nvim_list_bufs()) do if vim.api.nvim_buf_is_loaded(b) and vim.bo[b].modified then error('Current dirty buffers prevent restore') end end
local buffers={}
for i,b in ipairs(s.buffers) do
 if vim.fn.sha256(table.concat(vim.fn.readfile(b.path,'b'),'\n'))~=b.base then error('Session original changed during restore') end
 local id=vim.fn.bufadd(b.path); vim.fn.bufload(id); buffers[i]=id
 vim.api.nvim_buf_call(id,function() vim.cmd('silent checktime') end)
end
vim.cmd('tabonly'); vim.cmd('only')
local wins={}
local function build(x,win)
 vim.api.nvim_set_current_win(win)
 if x.kind=='leaf' then
  local n=x.value+1; local saved=s.windows[n]; wins[n]=win
  vim.api.nvim_win_set_buf(win,buffers[saved.buffer+1]); return
 end
 local created={win}
 for i=2,#x.value do
  vim.api.nvim_set_current_win(created[#created]); vim.cmd(x.kind=='row' and 'rightbelow vsplit' or 'rightbelow split'); table.insert(created,vim.api.nvim_get_current_win())
 end
 for i,child in ipairs(x.value) do build(child,created[i]) end
end
local tabs=s.version==1 and {s.layout} or s.tabs
local pages={}
for i,tab in ipairs(tabs) do
 if i>1 then vim.cmd('tabnew') end
 pages[i]=vim.api.nvim_get_current_tabpage()
 build(tab,vim.api.nvim_get_current_win())
end
if recover then
 for i,b in ipairs(s.buffers) do
  if b.lines~=vim.NIL then
   local id=buffers[i]; vim.api.nvim_buf_set_lines(id,0,-1,true,b.lines)
   vim.bo[id].endofline=b.endofline; vim.bo[id].modified=true
  end
 end
end
for i,w in ipairs(wins) do
 local saved=s.windows[i]; local count=vim.api.nvim_buf_line_count(vim.api.nvim_win_get_buf(w))
 vim.api.nvim_win_set_cursor(w,{math.min(saved.cursor[1],count),saved.cursor[2]})
 pcall(vim.api.nvim_win_set_width,w,saved.width); pcall(vim.api.nvim_win_set_height,w,saved.height)
end
vim.api.nvim_set_current_tabpage(pages[(s.activeTab or 0)+1]); vim.api.nvim_set_current_win(wins[s.active+1])
return true
"#,vec![json.into(),recover_unsaved.into()])?;
        self.focused = true;
        Ok(())
    }
    pub(crate) fn pin_buffer(&mut self) -> Result<BufferPin> {
        ensure!(self.frozen.is_none(), "editor is frozen during application");
        let value=self.guarded_lua(r#"
local b=vim.api.nvim_get_current_buf()
if vim.bo[b].buftype~='' or vim.bo[b].readonly or not vim.bo[b].modifiable then error('Select a writable original project buffer') end
local text=table.concat(vim.api.nvim_buf_get_lines(b,0,-1,true),'\n')..(vim.bo[b].endofline and '\n' or '')
if #text>1048576 then error('Buffer proposal exceeds 1 MiB') end
return vim.json.encode({editorId='',buffer=b,path=vim.api.nvim_buf_get_name(b),changedtick=vim.api.nvim_buf_get_changedtick(b),sha256=vim.fn.sha256(text),modified=vim.bo[b].modified})
"#,vec![])?;
        let mut pin: BufferPin =
            serde_json::from_str(value.as_str().context("invalid buffer pin")?)?;
        pin.editor_id = self.identity.clone();
        ensure!(
            !pin.modified,
            "save or discard dirty target buffer before proposing a change"
        );
        Ok(pin)
    }
    pub(crate) fn apply_buffer_proposal(&mut self, proposal: &BufferProposal) -> Result<()> {
        ensure!(
            self.frozen.is_none() && proposal.pin.editor_id == self.identity,
            "buffer proposal belongs to another editor session; recapture it"
        );
        ensure!(
            proposal.text.len() <= 1024 * 1024
                && !proposal.text.contains('\0')
                && hash(proposal.text.as_bytes()) == proposal.artifact_sha256,
            "invalid proposal text/hash"
        );
        let pin = &proposal.pin;
        self.guarded_lua(r#"
local b,t,path,sha,text=...
if not vim.api.nvim_buf_is_valid(b) or not vim.api.nvim_buf_is_loaded(b) then error('Proposal buffer closed') end
if vim.api.nvim_buf_get_name(b)~=path or vim.api.nvim_buf_get_changedtick(b)~=t or vim.bo[b].modified or vim.bo[b].readonly or not vim.bo[b].modifiable then error('Proposal buffer changed; recapture after reviewing current text') end
local old=table.concat(vim.api.nvim_buf_get_lines(b,0,-1,true),'\n')..(vim.bo[b].endofline and '\n' or '')
if vim.fn.sha256(old)~=sha then error('Proposal text pin changed') end
local eol=text:sub(-1)=='\n'; if eol then text=text:sub(1,-2) end
local lines=vim.split(text,'\n',{plain=true})
vim.api.nvim_buf_call(b,function()
 vim.api.nvim_buf_set_lines(b,0,-1,true,lines)
 vim.bo[b].endofline=eol; vim.bo[b].modified=true
end)
return true
"#,vec![pin.buffer.into(),pin.changedtick.into(),pin.path.clone().into(),pin.sha256.clone().into(),proposal.text.clone().into()])?;
        self.focused = true;
        Ok(())
    }
}
