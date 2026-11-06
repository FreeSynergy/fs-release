#!/usr/bin/env python3
"""Write a plain 256x256 PNG (solid FreeSynergy blue) for AppImages without their own icon."""
import struct
import sys
import zlib


def chunk(kind, data):
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)


def main(path):
    size, row = 256, b"\x00" + bytes((0x1F, 0x4E, 0x8C)) * 256
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(row * size, 9)) + chunk(b"IEND", b"")
    with open(path, "wb") as fh:
        fh.write(png)


if __name__ == "__main__":
    main(sys.argv[1])
