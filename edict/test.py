import json
import os
import unittest
from datetime import datetime, timezone
from difflib import unified_diff
from sys import stderr
from tempfile import NamedTemporaryFile

from edict.subedict import create_subedict, create_subenamdict, save_subedict

input_path = os.path.join(os.path.dirname(__file__), 'test-input.json')
output_edict_path = os.path.join(os.path.dirname(__file__), 'test-output-edict')
output_enamdict_path = os.path.join(os.path.dirname(__file__), 'test-output-enamdict')


class TestSubedict(unittest.TestCase):
    def diff(self, fromfile: str, tofile: str) -> None:
        with open(fromfile) as f, open(tofile) as g:
            a = f.readlines()
            b = g.readlines()
            if a != b:
                stderr.writelines(unified_diff(a, b, fromfile, tofile))
                raise AssertionError('the outputs are different')

    def test_subedict(self) -> None:
        with open(input_path) as f:
            stories = json.load(f)
        content = ''.join(story['content'] for story in stories)
        with NamedTemporaryFile() as f:
            start = datetime.now(tz=timezone.utc)
            save_subedict(create_subedict(content), f.name)
            elapsed = datetime.now(tz=timezone.utc) - start
            print()
            print(elapsed)
            self.diff(output_edict_path, f.name)

    def test_subenamdict(self) -> None:
        with open(input_path) as f:
            stories = json.load(f)
        content = ''.join(story['content'] for story in stories)
        with NamedTemporaryFile() as f:
            start = datetime.now(tz=timezone.utc)
            save_subedict(create_subenamdict(content), f.name)
            elapsed = datetime.now(tz=timezone.utc) - start
            print()
            print(elapsed)
            self.diff(output_enamdict_path, f.name)