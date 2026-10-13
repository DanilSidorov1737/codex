"""Drive an actual terminal through official Neovim; never replay submitted input.

Requires the existing development msgpack package and an actual nvim executable.
Neovim handles terminal queries which a bare PTY cannot answer. Call wait_for()
before send(); a captured welcome screen alone does not prove a model turn ran.
"""

import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import time

import msgpack


class Terminal:
    def __init__(self, nvim, evidence, env=None, width=120, height=36):
        self.evidence = Path(evidence)
        if self.evidence.is_symlink():
            raise ValueError("evidence must be a private real directory")
        self.evidence.mkdir(mode=0o700, parents=True, exist_ok=True)
        if self.evidence.stat().st_mode & 0o077:
            raise ValueError("evidence must be private")
        self.error = (self.evidence / "terminal.stderr").open("ab")
        self.process = subprocess.Popen(
            [str(nvim), "--embed", "-u", "NONE", "--noplugin", "-i", "NONE"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=self.error,
            env=env,
        )
        self.unpacker = msgpack.Unpacker(raw=False)
        self.request_id = 0
        self.grid = []
        self.highlights = []
        self.attributes = {}
        self.colors = {"foreground": 0xFFFFFF, "background": 0x000000}
        self.job = None
        self.rpc("nvim_ui_attach", width, height, {"rgb": True, "ext_linegrid": True})
        self.rpc(
            "nvim_command",
            "set laststatus=0 showtabline=0 cmdheight=0 noshowmode noruler",
        )

    def _event(self, message):
        if message[0] != 2 or message[1] != "redraw":
            return
        for batch in message[2]:
            for arg in batch[1:]:
                name = batch[0]
                if name == "default_colors_set":
                    self.colors.update(foreground=arg[0], background=arg[1])
                elif name == "hl_attr_define":
                    self.attributes[arg[0]] = arg[1]
                if name == "grid_resize" and arg[0] == 1:
                    self.grid = [[" "] * arg[1] for _ in range(arg[2])]
                    self.highlights = [[0] * arg[1] for _ in range(arg[2])]
                elif name == "grid_clear" and arg[0] == 1:
                    self.grid = [[" "] * len(row) for row in self.grid]
                    self.highlights = [[0] * len(row) for row in self.grid]
                elif name == "grid_line" and arg[0] == 1:
                    _, row, col, cells = arg[:4]
                    highlight = 0
                    for cell in cells:
                        if len(cell) > 1:
                            highlight = cell[1]
                        for _ in range(cell[2] if len(cell) > 2 else 1):
                            self.grid[row][col] = cell[0]
                            self.highlights[row][col] = highlight
                            col += 1
                elif name == "grid_scroll" and arg[0] == 1:
                    _, top, bottom, left, right, rows, cols = arg
                    old = [row[:] for row in self.grid]
                    old_highlights = [row[:] for row in self.highlights]
                    for row in range(top, bottom):
                        for col in range(left, right):
                            rr, cc = row + rows, col + cols
                            self.grid[row][col] = (
                                old[rr][cc]
                                if top <= rr < bottom and left <= cc < right
                                else " "
                            )
                            self.highlights[row][col] = (
                                old_highlights[rr][cc]
                                if top <= rr < bottom and left <= cc < right
                                else 0
                            )

    def _receive(self, timeout):
        readable, _, _ = select.select([self.process.stdout], [], [], timeout)
        if readable:
            data = os.read(self.process.stdout.fileno(), 65536)
            if not data:
                raise RuntimeError("official terminal renderer exited")
            self.unpacker.feed(data)

    def rpc(self, method, *args):
        self.request_id += 1
        request_id = self.request_id
        self.process.stdin.write(
            msgpack.packb([0, request_id, method, list(args)], use_bin_type=True)
        )
        self.process.stdin.flush()
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            for message in self.unpacker:
                if message[0] == 1 and message[1] == request_id:
                    if message[2] is not None:
                        raise RuntimeError((method, message[2]))
                    return message[3]
                self._event(message)
            self._receive(max(0, deadline - time.monotonic()))
        raise TimeoutError(method)

    def settle(self, seconds=0.1):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            for message in self.unpacker:
                self._event(message)
            self._receive(min(0.1, max(0, deadline - time.monotonic())))
        for message in self.unpacker:
            self._event(message)

    def start(self, argv, cwd):
        if self.job is not None or not argv or any("\0" in arg for arg in argv):
            raise ValueError("one finite argv job per terminal")
        self.job = self.rpc(
            "nvim_exec_lua",
            "local argv,cwd=...; vim.g.codex_terminal_exit=nil; "
            "return vim.fn.jobstart(argv,{term=true,cwd=cwd,on_exit=function(_,code) "
            "vim.g.codex_terminal_exit=code end})",
            [argv, str(Path(cwd).resolve(strict=True))],
        )
        if self.job <= 0:
            raise RuntimeError("terminal job did not start")
        self.rpc("nvim_command", "startinsert")

    def screen(self):
        return "\n".join("".join(cell or " " for cell in row) for row in self.grid)

    def wait_for(self, predicate, timeout=30):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            self.settle()
            if predicate(self.screen()):
                return self.screen()
        self.capture("timeout-" + str(time.time_ns()))
        raise TimeoutError("expected actual terminal screen was not observed")

    def send(self, text):
        if self.job is None:
            raise ValueError("terminal job is not started")
        self.rpc("nvim_chan_send", self.job, text)
        self.settle()

    def wait_exit(self, timeout=10):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            result = self.rpc("nvim_exec_lua", "return vim.g.codex_terminal_exit", [])
            if result is not None:
                return result
            self.settle()
        raise TimeoutError(
            "actual CLI did not exit; no forced exit is claimed as passing"
        )

    def submit(self, text):
        """Submit once after paste detection settles; text+Enter in one burst is paste."""
        self.send(text)
        self.settle(0.3)
        self.send("\r")

    def capture(self, name):
        if not name or Path(name).name != name or name in (".", ".."):
            raise ValueError("capture name must be a filename")
        data = (self.screen() + "\n").encode()
        with (self.evidence / (name + ".txt")).open("xb") as stream:
            stream.write(data)
        record = {
            "renderer": "official Neovim",
            "textSha256": hashlib.sha256(data).hexdigest(),
        }
        with (self.evidence / (name + ".json")).open("x") as stream:
            json.dump(record, stream)
        return record

    def capture_png(self, name):
        """Render captured native cells and RGB attributes with the installed font."""
        from PIL import Image, ImageDraw, ImageFont

        record = self.capture(name)
        font_root = "/usr/share/fonts/truetype/dejavu/"
        normal = ImageFont.truetype(font_root + "DejaVuSansMono.ttf", 20)
        bold = ImageFont.truetype(font_root + "DejaVuSansMono-Bold.ttf", 20)
        canvas = Image.new("RGB", (len(self.grid[0]) * 12, len(self.grid) * 25))
        draw = ImageDraw.Draw(canvas)
        for row, cells in enumerate(self.grid):
            for col, character in enumerate(cells):
                attr = self.attributes.get(self.highlights[row][col], {})
                foreground = attr.get("foreground", self.colors["foreground"])
                background = attr.get("background", self.colors["background"])
                if attr.get("reverse"):
                    foreground, background = background, foreground
                foreground = max(0, foreground)
                background = max(0, background)
                draw.rectangle(
                    (col * 12, row * 25, col * 12 + 11, row * 25 + 24),
                    fill=f"#{background:06x}",
                )
                if character:
                    draw.text(
                        (col * 12, row * 25),
                        character,
                        font=bold if attr.get("bold") else normal,
                        fill=f"#{foreground:06x}",
                    )
        with (self.evidence / (name + ".png")).open("xb") as stream:
            canvas.save(stream, format="PNG")
        return record

    def close(self):
        if self.process.poll() is None:
            if self.job is not None:
                self.rpc("nvim_exec_lua", "return vim.fn.jobstop(...)", [self.job])
            self.process.stdin.write(
                msgpack.packb([2, "nvim_command", ["qa!"]], use_bin_type=True)
            )
            self.process.stdin.flush()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.terminate()
                self.process.wait(timeout=5)
        self.process.stdin.close()
        self.process.stdout.close()
        self.error.close()
