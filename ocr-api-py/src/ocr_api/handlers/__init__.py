import importlib
import os
import shutil
from collections.abc import Callable
from tempfile import NamedTemporaryFile
from typing import TypeVar

from fastapi import UploadFile

from ocr_api.helpers import camel_to_snake


def parse_comma_list(s: str) -> set[str]:
    return {x for x in (x.strip() for x in s.split(",")) if x}


T = TypeVar("T")

_HANDLERS: dict[str, "Handler"] = {}

_DISABLED_HANDLERS = parse_comma_list(os.environ.get("HANDLER_DENYLIST", ""))
_ENABLED_HANDLERS = parse_comma_list(os.environ.get("HANDLER_ALLOWLIST", ""))


class Handler:
    @staticmethod
    def available_handlers():
        return {
            handler.name(): handler
            for handler in _HANDLERS.values()
            if handler.available()
        }

    @classmethod
    def name(cls):
        return camel_to_snake(cls.__name__)

    @staticmethod
    def handler_is_available(name: str):
        allow = False

        # If handler allowlist is set, only allow handlers in the allowlist
        if _ENABLED_HANDLERS and name in _ENABLED_HANDLERS:
            allow = True

        # If handler denylist is set, deny handlers in the denylist
        if _DISABLED_HANDLERS and name in _DISABLED_HANDLERS:
            allow = False

        # If neither denylist nor allowlist is set, allow all handlers
        if not _ENABLED_HANDLERS and not _DISABLED_HANDLERS:
            allow = True

        return allow

    def available(self):
        return self.handler_is_available(self.name())

    def handle(self, file: UploadFile):
        raise NotImplementedError

    def upload(self, *, file: UploadFile, after: Callable[[str], T]) -> T:
        with NamedTemporaryFile() as temp:
            shutil.copyfileobj(file.file, temp)
            return after(temp.name)

    def coords(self, c: list[int]):
        return {
            "x": int(c[0]),
            "y": int(c[1]),
        }

    def box(self, c: list[list[int]]):
        return {
            "tl": self.coords(c[0]),
            "tr": self.coords(c[1]),
            "br": self.coords(c[2]),
            "bl": self.coords(c[3]),
        }

    def __init_subclass__(cls) -> None:
        _HANDLERS[cls.name()] = cls()


# Dynamically load all files in the handlers directory to register them with the API
for file in os.listdir(os.path.dirname(__file__)):
    if not file.endswith(".py") or file.startswith("_"):
        continue

    name = file[:-3]

    if not Handler.handler_is_available(camel_to_snake(name)):
        print(f"|> Skipping {name}")
        continue

    print(f"|> Importing {name}")
    importlib.import_module(f".{name}", __package__)
