"""Repair and verify the permissions inside unsigned Tauri AppImages."""
import json
import os
from pathlib import Path
import re
import stat
import shutil
import subprocess
import sys
import tempfile


def verify_permissions(root):
    """Require launchers and executable files to work for every user."""
    required = [root / 'AppRun', root / 'usr/bin/flaccompagnon-desktop']
    if (root / 'AppRun.wrapped').exists() or (root / 'AppRun.wrapped').is_symlink():
        required.append(root / 'AppRun.wrapped')
    for path in required:
        if not path.is_file() or not path.resolve().is_relative_to(root.resolve()):
            raise ValueError(f'Missing or unsafe launcher: {path}')
        if stat.S_IMODE(path.stat().st_mode) & 0o555 != 0o555:
            raise ValueError(f'Launcher is inaccessible to other users: {path}')
    for path in [root, *root.rglob('*')]:
        if path.is_symlink():
            continue
        mode = stat.S_IMODE(path.stat().st_mode)
        if path.is_dir() and mode & 0o555 != 0o555:
            raise ValueError(f'Inaccessible directory: {path}')
        if path.is_file() and mode & 0o111 and mode & 0o555 != 0o555:
            raise ValueError(f'Inaccessible executable: {path}')


def repair_permissions(root):
    """Normalize executable access without making resources executable."""
    root.chmod(0o755)
    # linuxdeploy can create its wrapped launcher with 770. Testing as its
    # owner hides the problem, so enforce access for other users explicitly.
    for path in root.rglob('*'):
        if path.is_symlink():
            continue
        mode = stat.S_IMODE(path.stat().st_mode)
        if path.is_dir():
            path.chmod(0o755)
        elif mode & 0o111:
            path.chmod(0o755)
    for relative in ['AppRun', 'AppRun.wrapped', 'usr/bin/flaccompagnon-desktop']:
        path = root / relative
        if path.exists() or path.is_symlink():
            if not path.resolve().is_relative_to(root.resolve()):
                raise ValueError(f'Unsafe launcher: {path}')
            path.chmod(0o755)
    verify_permissions(root)


def glibc_versions(text):
    """Read numeric glibc requirements, excluding GLIBCXX and private names."""
    return [tuple(map(int, version.split('.'))) for version in
            re.findall(r'Name: GLIBC_([0-9]+(?:\.[0-9]+)+)\b', text)]


def verify_glibc(root):
    """Keep the Ubuntu 22.04 compatibility baseline from drifting silently."""
    for path in root.rglob('*'):
        if path.is_symlink() or not path.is_file():
            continue
        with path.open('rb') as stream:
            if stream.read(4) != b'\x7fELF':
                continue
        result = subprocess.run(['readelf', '--version-info', '--wide', str(path)],
                                check=True, capture_output=True, text=True)
        if any(version > (2, 35) for version in glibc_versions(result.stdout)):
            raise ValueError(f'{path} requires glibc newer than 2.35')


def repair_image(image):
    """Rebuild the embedded filesystem, retaining the original ELF runtime."""
    image = image.resolve()
    image.chmod(0o755)
    offset = int(subprocess.check_output([str(image), '--appimage-offset'], text=True).strip())
    with image.open('rb') as stream:
        prefix = stream.read(offset)
    if len(prefix) != offset or offset < 16 or prefix[:4] != b'\x7fELF' or prefix[8:11] != b'AI\x02':
        raise ValueError('Expected a type-2 AppImage runtime')
    # This runs before publication/signing: rebuilding invalidates signatures.
    if Path(str(image) + '.sig').exists():
        raise ValueError('Repair the AppImage before signing it')
    with tempfile.TemporaryDirectory(dir=image.parent) as directory:
        temporary = Path(directory)
        root = temporary / 'AppDir'
        subprocess.run(['unsquashfs', '-no-progress', '-o', str(offset), '-d', str(root), str(image)], check=True)
        repair_permissions(root)
        verify_glibc(root)
        filesystem = temporary / 'filesystem.squashfs'
        subprocess.run(['mksquashfs', str(root), str(filesystem), '-noappend', '-comp', 'zstd', '-no-progress'], check=True)
        rebuilt = temporary / 'repaired.AppImage'
        with rebuilt.open('wb') as output:
            output.write(prefix)
            with filesystem.open('rb') as source:
                shutil.copyfileobj(source, output)
        rebuilt.chmod(0o755)
        verified = temporary / 'verified'
        subprocess.run(['unsquashfs', '-no-progress', '-o', str(offset), '-d', str(verified), str(rebuilt)], check=True)
        verify_permissions(verified)
        os.replace(rebuilt, image)


if __name__ == '__main__':
    images = [Path(path) for path in json.loads(Path(sys.argv[1]).read_text())
              if path.endswith('.AppImage')]
    if not images:
        sys.exit('No AppImage was produced')
    for image in images:
        repair_image(image)
