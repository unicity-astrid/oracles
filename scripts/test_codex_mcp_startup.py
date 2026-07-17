#!/usr/bin/env python3
"""Exercise the Codex MCP bootstrap handshake without timing guesses."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time


ROOT = Path(__file__).resolve().parent.parent
SERVER = json.loads((ROOT / "plugins/unicity-aos/.mcp.json").read_text())["mcpServers"]["aos"]


def wait_for(path: Path, process: subprocess.Popen[str], timeout: float = 5.0) -> None:
    deadline = time.monotonic() + timeout
    while not path.exists():
        if process.poll() is not None:
            stdout, stderr = process.communicate()
            raise AssertionError(
                f"MCP launcher exited before readiness marker: {stdout=} {stderr=}"
            )
        if time.monotonic() >= deadline:
            process.kill()
            process.wait()
            raise AssertionError(f"timed out waiting for {path}")
        time.sleep(0.01)


def write_executable(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)
    path.chmod(0o700)


def launch(workspace: Path, environment: dict[str, str]) -> subprocess.Popen[str]:
    return subprocess.Popen(
        [SERVER["command"], *SERVER["args"]],
        cwd=workspace,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def assert_success(process: subprocess.Popen[str]) -> None:
    try:
        stdout, stderr = process.communicate(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        stdout, stderr = process.communicate()
        raise AssertionError(f"MCP launcher did not exit: {stdout=} {stderr=}") from None
    if process.returncode != 0:
        raise AssertionError(f"MCP launcher failed: {stdout=} {stderr=}")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="aos-codex-mcp-") as raw:
        root = Path(raw)
        home = root / "home" / ".aos"
        workspace = root / "user-project"
        fake_bin = root / "fake-bin"
        wait_marker = root / "wait-observed"
        wait_gate = root / "release-waiter"
        cwd_log = root / "aos-cwd"
        args_log = root / "aos-args"
        workspace.mkdir(parents=True)
        fake_bin.mkdir()

        write_executable(
            fake_bin / "sleep",
            "#!/bin/sh\n"
            ': > "$TEST_WAIT_MARKER"\n'
            'while [ ! -e "$TEST_WAIT_GATE" ]; do /bin/sleep 0.01; done\n',
        )

        environment = {
            "HOME": str(root / "home"),
            "AOS_HOME": str(home),
            "PATH": f"{fake_bin}:/usr/bin:/bin",
            "TEST_WAIT_MARKER": str(wait_marker),
            "TEST_WAIT_GATE": str(wait_gate),
            "TEST_AOS_CWD": str(cwd_log),
            "TEST_AOS_ARGS": str(args_log),
        }

        write_executable(
            home / "bin/aos",
            "#!/bin/sh\n"
            'pwd -P > "$TEST_AOS_CWD"\n'
            'printf "%s\\n" "$*" > "$TEST_AOS_ARGS"\n',
        )

        process = launch(workspace, environment)
        wait_for(wait_marker, process)
        if process.poll() is not None:
            raise AssertionError("MCP launcher did not wait for the Codex pack receipt")

        receipt = home / "extensions/oracles/codex/Pack.lock"
        receipt.parent.mkdir(parents=True)
        receipt.write_text("ready\n")
        wait_gate.touch()
        assert_success(process)

        actual_cwd = cwd_log.read_text().strip()
        assert Path(actual_cwd) == workspace.resolve(), (actual_cwd, str(workspace.resolve()))
        assert args_log.read_text().strip() == "--principal codex-code mcp serve"

        (home / "bin/aos").unlink()
        wait_marker.unlink()
        wait_gate.unlink()
        process = launch(workspace, environment)
        wait_for(wait_marker, process)
        if process.poll() is not None:
            raise AssertionError("MCP launcher did not wait for the AOS executable")
        write_executable(
            home / "bin/aos",
            "#!/bin/sh\n"
            'pwd -P > "$TEST_AOS_CWD"\n'
            'printf "%s\\n" "$*" > "$TEST_AOS_ARGS"\n',
        )
        wait_gate.touch()
        assert_success(process)

        wait_marker.unlink()
        wait_gate.unlink()
        process = launch(workspace, environment)
        assert_success(process)
        assert not wait_marker.exists(), "ready startup unexpectedly entered the wait path"

        plugin_copy = root / "plugin-copy"
        shutil.copytree(ROOT / "plugins/unicity-aos", plugin_copy)
        subprocess.run(
            [
                "/bin/sh",
                str(plugin_copy / "install.sh"),
                "--bin-root",
                str(home / "bin"),
                "--skip-codex-install",
            ],
            cwd=workspace,
            env=environment,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        generated = json.loads((plugin_copy / ".mcp.json").read_text())["mcpServers"]["aos"]
        assert generated["command"] == SERVER["command"]
        assert generated["args"] == SERVER["args"]
        assert generated["startup_timeout_sec"] == SERVER["startup_timeout_sec"]
        assert "cwd" not in generated
        assert generated["env"] == {"AOS_BIN": str(home / "bin/aos")}


if __name__ == "__main__":
    main()
