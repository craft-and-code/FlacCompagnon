"""Check that a packaged AppImage opens a window and stays running under Xvfb."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time


def smoke(image):
    """Exercise the actual packaged launch chain without requiring FUSE."""
    with tempfile.TemporaryFile(mode='w+') as log:
        process = subprocess.Popen([str(image.resolve()), '--appimage-extract-and-run'],
                                   stdout=log, stderr=log, start_new_session=True)
        try:
            deadline = time.monotonic() + 45
            visible_since = None
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    raise RuntimeError(f'AppImage exited early ({process.returncode})')
                window = subprocess.run(['xdotool', 'search', '--onlyvisible', '--name', '^FlacCompagnon$'],
                                        capture_output=True, text=True, check=False)
                if window.returncode == 0 and window.stdout.strip():
                    if visible_since is None:
                        visible_since = time.monotonic()
                    if time.monotonic() - visible_since >= 10:
                        print(f'Window opened and remained visible: {image.name}')
                        return
                else:
                    visible_since = None
                time.sleep(0.5)
            raise RuntimeError('AppImage did not keep a visible window for 10 seconds')
        except Exception:
            log.seek(0)
            print(log.read(), file=sys.stderr)
            raise
        finally:
            # WebKit creates child processes; clean up the entire process group.
            try:
                os.killpg(process.pid, signal.SIGTERM)
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            except ProcessLookupError:
                pass


if __name__ == '__main__':
    for path in json.loads(Path(sys.argv[1]).read_text()):
        if path.endswith('.AppImage'):
            smoke(Path(path))
