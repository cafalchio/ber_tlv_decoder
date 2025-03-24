from typing import Any


def tlv_from_gz_file(file: str) -> str: ...
"""Decode ber tlv from a single gz file
    parameters:
        file: str
    return: list
        list(class TlvObject:
            tag: BerTag
            length: int
            value: bytes
            offset: int
            children: list["TlvObject"]
        )
"""
