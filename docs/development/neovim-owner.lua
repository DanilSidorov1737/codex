-- Optional owner config for official Neovim 0.12+. Copy to your own init.lua;
-- set CODEX_ORGANIZATION_NVIM_CONFIG to its absolute path before starting Codex.
-- Existing owner config/plugins can instead provide these settings themselves.
vim.o.number = true
vim.o.termguicolors = true
vim.cmd('packadd matchit') -- Plugin bundled with official Neovim; no downloads.
vim.diagnostic.config({ virtual_text = true, signs = true, underline = true })
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'rust',
  callback = function(event)
    if vim.fn.executable('rust-analyzer') ~= 1 then return end
    vim.lsp.start({
      name = 'rust-analyzer',
      cmd = { 'rust-analyzer' },
      root_dir = vim.fs.root(event.buf, { 'Cargo.toml', '.git' })
        or vim.fs.dirname(vim.api.nvim_buf_get_name(event.buf)),
    }, { bufnr = event.buf })
  end,
})
vim.api.nvim_create_autocmd('LspAttach', {
  callback = function(event)
    local client = vim.lsp.get_client_by_id(event.data.client_id)
    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, { buffer = event.buf })
    vim.bo[event.buf].omnifunc = 'v:lua.vim.lsp.omnifunc'
    if client and client:supports_method('textDocument/completion') then
      vim.lsp.completion.enable(true, client.id, event.buf, { autotrigger = true })
    end
  end,
})
