"""Regression coverage for the catalog's permission-denied startup failure."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('packaging', Path(__file__).resolve().parents[1] / 'scripts/appimage_packaging.py')
packaging = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(packaging)


class AppImagePermissionsTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.root.chmod(0o755)
        (self.root / 'usr/bin').mkdir(parents=True)
        for relative in ['AppRun', 'AppRun.wrapped', 'usr/bin/flaccompagnon-desktop']:
            path = self.root / relative
            path.write_bytes(b'launcher fixture')
            path.chmod(0o755)

    def test_wrapped_launcher_with_770_works_for_other_users_after_repair(self):
        wrapper = self.root / 'AppRun.wrapped'
        wrapper.chmod(0o770)
        with self.assertRaisesRegex(ValueError, 'inaccessible'):
            packaging.verify_permissions(self.root)
        packaging.repair_permissions(self.root)
        self.assertEqual(wrapper.stat().st_mode & 0o777, 0o755)
        self.assertEqual(wrapper.read_bytes(), b'launcher fixture')

    def test_resource_files_do_not_become_executable(self):
        resource = self.root / 'report.json'
        resource.write_text('{}')
        resource.chmod(0o644)
        packaging.repair_permissions(self.root)
        self.assertEqual(resource.stat().st_mode & 0o777, 0o644)

    def test_non_executable_main_launcher_is_repaired(self):
        launcher = self.root / 'usr/bin/flaccompagnon-desktop'
        launcher.chmod(0o644)
        packaging.repair_permissions(self.root)
        self.assertEqual(launcher.stat().st_mode & 0o777, 0o755)

    def test_missing_main_launcher_is_rejected(self):
        (self.root / 'usr/bin/flaccompagnon-desktop').unlink()
        with self.assertRaisesRegex(ValueError, 'Missing'):
            packaging.repair_permissions(self.root)

    def test_launcher_symlink_cannot_change_files_outside_appdir(self):
        with tempfile.TemporaryDirectory() as outside:
            target = Path(outside) / 'external'
            target.write_text('external')
            target.chmod(0o600)
            wrapper = self.root / 'AppRun.wrapped'
            wrapper.unlink()
            wrapper.symlink_to(target)
            with self.assertRaisesRegex(ValueError, 'Unsafe'):
                packaging.repair_permissions(self.root)
            self.assertEqual(target.stat().st_mode & 0o777, 0o600)

    def test_glibc_requirements_exclude_cpp_and_private_versions(self):
        versions = packaging.glibc_versions('Name: GLIBC_2.35 Name: GLIBCXX_3.4.30 Name: GLIBC_PRIVATE Name: GLIBC_2.38')
        self.assertEqual(versions, [(2, 35), (2, 38)])


if __name__ == '__main__':
    unittest.main()
