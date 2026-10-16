#!/usr/bin/env python3
"""Prints the command frames the GSUI sends on a serial port.

Frame (PROTOCOL.md): FE CA | CMD | LEN | payload[LEN] | BE

    python tools/cmd_sniffer.py --list
    python tools/cmd_sniffer.py COM7 --baud 115200
    python tools/cmd_sniffer.py --selftest
"""

import argparse
import sys
import time

SYNC = b"\xFE\xCA"
FOOTER = 0xBE

COMMANDS = {
    0x00: "COMMAND_NONE",
    0x01: "COMMAND_RESET",
    0x02: "COMMAND_GROUND_ABORT",
    0x03: "COMMAND_CALIBRATION",
    0x04: "COMMAND_DROGUE",
    0x05: "COMMAND_LANDED",
    0x06: "COMMAND_DEEP_CALIBRATION",
    0x07: "COMMAND_REQUEST_TELEM_A",
    0x08: "COMMAND_REQUEST_TELEM_B",
    0x10: "COMMAND_HIL_DATA",
    0x20: "COMMAND_GPS_DATA",
    0x80: "COMMAND_RESPONSE_TELEM_A",
    0x81: "COMMAND_RESPONSE_TELEM_B",
}

FACES = {1: "+X up", 2: "-X up", 3: "+Y up", 4: "-Y up", 5: "+Z up", 6: "-Z up"}


def decode_payload(cmd, payload):
    """Human-readable payload, per the payload tables in PROTOCOL.md."""
    if cmd == 0x03 and len(payload) == 4:
        import struct
        rad = struct.unpack("<f", payload)[0]
        return f"pitch={rad:.4f} rad ({rad * 57.2957795:.1f} deg)"
    if cmd == 0x06 and len(payload) == 1:
        return f"face={payload[0]} ({FACES.get(payload[0], '?')})"
    return payload.hex(" ") if payload else ""


def take_frames(buf):
    """Pulls complete frames off the front of bytearray `buf`.

    Returns a list of (cmd, payload). Garbage before a sync is dropped; a frame
    with a bad footer costs 2 bytes and resyncs, same as the Rust parser.
    """
    frames = []
    while True:
        pos = buf.find(SYNC)
        if pos < 0:
            del buf[: max(0, len(buf) - 1)]  # keep a trailing half sync
            return frames
        del buf[:pos]
        if len(buf) < 4:
            return frames
        cmd, length = buf[2], buf[3]
        total = 5 + length
        if len(buf) < total:
            return frames
        if buf[total - 1] != FOOTER:
            del buf[:2]
            continue
        frames.append((cmd, bytes(buf[4 : 4 + length])))
        del buf[:total]


def sniff(port, baud):
    import serial

    buf = bytearray()
    with serial.Serial(port, baud, timeout=0.1) as ser:
        print(f"listening on {port} @ {baud} (ctrl-c to stop)")
        while True:
            chunk = ser.read(256)
            if chunk:
                buf += chunk
                for cmd, payload in take_frames(buf):
                    name = COMMANDS.get(cmd, "UNKNOWN")
                    extra = decode_payload(cmd, payload)
                    stamp = time.strftime("%H:%M:%S")
                    print(f"{stamp}  0x{cmd:02X} {name:<26} {extra}")
                    sys.stdout.flush()


def list_ports():
    from serial.tools import list_ports as lp

    for p in lp.comports():
        print(f"{p.device}\t{p.description}")


def selftest():
    def frame(cmd, payload=b""):
        return bytes(SYNC) + bytes([cmd, len(payload)]) + payload + bytes([FOOTER])

    import struct

    buf = bytearray(b"\xAA\xBB")  # leading garbage
    buf += frame(0x07)
    buf += frame(0x03, struct.pack("<f", 1.5707964))
    buf += frame(0x06, b"\x04")
    got = take_frames(buf)
    assert [c for c, _ in got] == [0x07, 0x03, 0x06], got
    assert not buf, buf
    assert "90.0 deg" in decode_payload(0x03, got[1][1])
    assert decode_payload(0x06, got[2][1]) == "face=4 (-Y up)"

    # bad footer is skipped, the good frame behind it still lands
    bad = bytearray(bytes(SYNC) + bytes([0x01, 0x00, 0x00]) + frame(0x02))
    assert [c for c, _ in take_frames(bad)] == [0x02]

    # a frame split across reads is held until complete
    split = bytearray(frame(0x06, b"\x02")[:3])
    assert take_frames(split) == []
    split += frame(0x06, b"\x02")[3:]
    assert take_frames(split) == [(0x06, b"\x02")]

    print("selftest ok")


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("port", nargs="?", help="serial port, e.g. COM7 or /dev/ttyUSB0")
    ap.add_argument("--baud", type=int, default=115200)
    ap.add_argument("--list", action="store_true", help="list serial ports and exit")
    ap.add_argument("--selftest", action="store_true", help="run parser checks and exit")
    args = ap.parse_args()

    if args.selftest:
        selftest()
    elif args.list:
        list_ports()
    elif args.port:
        try:
            sniff(args.port, args.baud)
        except KeyboardInterrupt:
            print()
    else:
        ap.error("give a port, or --list / --selftest")
