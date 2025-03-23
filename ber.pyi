from typing import Any


def decode_from_gz_file() -> str: ...
"""Decode ber tlv from a single gz file
    class TlvObject:
        tag: BerTag
        length: int
        value: bytes
        offset: int
        children: list["TlvObject"] = None
"""

def decode_from_gz_path() -> str: ...
