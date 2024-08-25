import importlib
import os
import shutil
from collections.abc import Callable
from tempfile import NamedTemporaryFile
from typing import TypeVar

from fastapi import UploadFile

from ocr_api.helpers import camel_to_snake

T = TypeVar("T")

_HANDLERS: dict[str, "Handler"] = {}


_DISABLED_HANDLERS = set(os.environ.get("HANDLER_DENYLIST", "").split(","))
_ENABLED_HANDLERS = set(os.environ.get("HANDLER_ALLOWLIST", "").split(","))


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

    def available(self):
        allow = False

        # If handler allowlist is set, only allow handlers in the allowlist
        if _ENABLED_HANDLERS and self.name() in _ENABLED_HANDLERS:
            allow = True

        # If handler denylist is set, deny handlers in the denylist
        if _DISABLED_HANDLERS and self.name() in _DISABLED_HANDLERS:
            allow = False

        # If neither denylist nor allowlist is set, allow all handlers
        if not _ENABLED_HANDLERS and not _DISABLED_HANDLERS:
            allow = True

        return allow

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

    def should_ignore(name: str):
        return name in _DISABLED_HANDLERS or (
            _ENABLED_HANDLERS and name not in _ENABLED_HANDLERS
        )

    if should_ignore(camel_to_snake(name)):
        continue

    importlib.import_module(f".{name}", __package__)
