import re


def camel_to_snake(word: str) -> str:
    word = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1-\2", word)
    word = re.sub(r"([a-z\d])([A-Z])", r"\1-\2", word)
    word = word.replace("_", "-")
    word = word.replace(" ", "-")
    word = word.replace(r"\-+", "-")
    word = word.lower()

    return word
