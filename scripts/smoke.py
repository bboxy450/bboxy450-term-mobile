# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term-mobile — https://github.com/bboxy450/bboxy450-term-mobile
# Credit copied, adapted and translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term-mobile/blob/main/ATTRIBUTION.md

import json
import os
from pathlib import Path
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "target/smoke"
NAME = "bboxy450-smoke"
APP = "com.bboxy450.term.mobile"
READY = "BBOXY_INBOX_READY"
SUSPEND = "BBOXY_INBOX_SUSPENDED"
IOS = ("xcrun", "simctl")
BUTTON = (38, 58, 88)
CARD = (21, 29, 42)
INK = (242, 238, 230)
ANSWERED = (142, 219, 187)


def run(*argv, timeout=30, **options):
    return subprocess.run(argv, check=True, timeout=timeout, **options)


def output(*argv, timeout=30, **options):
    return run(*argv, timeout=timeout, stdout=subprocess.PIPE, **options).stdout


def until(what, condition, seconds=120, process=None):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if process is not None and process.poll() is not None:
            raise RuntimeError(f"{what}: child exited with {process.returncode}")
        value = condition()
        if value:
            return value
        time.sleep(0.5)
    raise RuntimeError(f"timed out waiting for {what}")


def marker(log, token, after=None):
    lines = log.read_text(errors="replace").splitlines()
    error = next((line for line in lines if "BBOXY_INBOX_ERROR" in line), None)
    if error:
        raise RuntimeError(error)
    if after is not None:
        last = next((i for i in range(len(lines) - 1, -1, -1) if after in lines[i]), None)
        if last is None:
            return None
        lines = lines[last + 1:]
    return next((line for line in reversed(lines) if token in line), None)


def stop(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


class Shot:
    def __init__(self, png):
        with tempfile.TemporaryDirectory(dir=OUT) as temp:
            bmp = Path(temp) / "shot.bmp"
            output("sips", "-s", "format", "bmp", png, "--out", bmp)
            data = bmp.read_bytes()
        if data[:2] != b"BM":
            raise ValueError("sips did not produce a BMP")
        offset, = struct.unpack_from("<I", data, 10)
        self.width, height, planes, bits, compression = struct.unpack_from("<iiHHI", data, 18)
        if planes != 1 or bits not in (24, 32) or compression not in (0, 3):
            raise ValueError(f"unexpected BMP encoding ({bits} bits, compression {compression})")
        self.height = abs(height)
        size = bits // 8
        stride = (self.width * size + 3) & ~3
        self.rows = [data[i:i + self.width * size] for i in
                     range(offset, offset + self.height * stride, stride)]
        if height > 0:
            self.rows.reverse()
        self.pixel_size = size

    def pixel(self, x, y):
        i = x * self.pixel_size
        return tuple(reversed(self.rows[y][i:i + 3]))

    def first_button(self):
        x = self.width * 9 // 25
        for y in range(self.height // 12, self.height // 2):
            if near(self.pixel(x, y), BUTTON) and near(self.pixel(x, y + 24), BUTTON):
                return x, y + 24
        return None

    def ink_samples(self, color, top, bottom):
        return sum(
            near(self.pixel(x, y), color)
            for x in range(self.width // 12, self.width * 2 // 3, 2)
            for y in range(max(0, top), min(self.height, bottom), 2)
        )

    def inbox(self):
        card_samples = sum(
            near(self.pixel(x, y), CARD)
            for x in range(0, self.width, 8)
            for y in range(0, self.height, 8)
        )
        return self.height > self.width and card_samples > 200 and self.first_button()


def near(pixel, expected):
    return all(abs(a - b) <= 5 for a, b in zip(pixel, expected))


def capture(platform, shot):
    path = OUT / f"{platform}.png"
    def frame():
        shot(path)
        image = Shot(path)
        button = image.inbox()
        if not button:
            return None
        _, y = button
        if image.ink_samples(INK, y - image.width // 9, y - 24) >= 25:
            return button
        return None
    first_button = until(f"{platform} Inbox pixels", frame, seconds=25)
    print(f"{platform}: verified portrait Inbox cards, controls and question ink in {path}")
    return first_button


def ios():
    runtimes = json.loads(output(*IOS, "list", "-j", "runtimes"))["runtimes"]
    candidates = [
        r for r in runtimes
        if r["isAvailable"] and r["identifier"].rsplit(".", 1)[-1].startswith("iOS-")
        # winit 0.30 uses UIApplicationDelegate. iOS 27 traps without scene adoption.
        and 17 <= int(r["version"].split(".")[0]) < 27
    ]
    if not candidates:
        raise RuntimeError("winit 0.30 requires an iOS Simulator runtime from 17 through 26")
    runtime = max(candidates, key=lambda r: tuple(int(n) for n in r["version"].split(".")))
    devices = json.loads(output(*IOS, "list", "-j", "devices", "available"))["devices"]
    matches = [d for d in devices.get(runtime["identifier"], []) if d["name"] == NAME]
    if len(matches) > 1:
        raise ValueError(f"multiple simulators on {runtime['identifier']} are named {NAME}")
    if matches:
        udid = matches[0]["udid"]
        already_booted = matches[0]["state"] == "Booted"
    else:
        phones = [d for d in runtime["supportedDeviceTypes"] if d["name"].startswith("iPhone")]
        phone = next((d for d in phones if d["name"] == "iPhone 17e"), phones[0])
        udid = output(*IOS, "create", NAME, phone["identifier"], runtime["identifier"]).decode().strip()
        already_booted = False

    if not already_booted:
        run(*IOS, "boot", udid)
    try:
        run(*IOS, "bootstatus", udid, timeout=240)
        run(*IOS, "install", udid, ROOT / "apps/ios/bboxy450-term-mobile.app", timeout=150)
        log = OUT / "ios.log"
        with log.open("wb") as writer:
            app = subprocess.Popen(
                [*IOS, "launch", "--console", "--terminate-running-process", udid, APP],
                stdout=writer, stderr=subprocess.STDOUT,
            )
            try:
                line = until(f"iOS {READY} (see {log})", lambda: marker(log, READY),
                             seconds=60, process=app)
                print(line)
                capture("ios", lambda path: output(*IOS, "io", udid, "screenshot", path))
            finally:
                stop(app)
    finally:
        if not already_booted:
            run(*IOS, "shutdown", udid)


def emulator_port():
    for port in range(5560, 5682, 2):
        with socket.socket() as console, socket.socket() as adb:
            try:
                console.bind(("127.0.0.1", port))
                adb.bind(("127.0.0.1", port + 1))
                return port
            except OSError:
                continue
    raise RuntimeError("no free emulator console/ADB port pair")


def android():
    sdk = Path(os.environ["ANDROID_HOME"])
    avd = ROOT / "target/android/avd"
    avd.mkdir(parents=True, exist_ok=True)
    env = {**os.environ, "ANDROID_AVD_HOME": str(avd)}
    if not (avd / f"{NAME}.ini").is_file():
        run(sdk / "cmdline-tools/latest/bin/avdmanager", "create", "avd", "--name", NAME,
            "--package", os.environ["ANDROID_IMAGE"], "--device", "pixel_6", env=env, input=b"no\n")

    port = emulator_port()
    adb_bin = sdk / "platform-tools/adb"
    run(adb_bin, "start-server")
    def adb(*args, timeout=20, **options):
        return output(adb_bin, "-s", f"emulator-{port}", *args, timeout=timeout, **options)

    emulator_log = OUT / "android-emulator.log"
    with emulator_log.open("wb") as writer:
        emulator = subprocess.Popen(
            [sdk / "emulator/emulator", "-avd", NAME, "-port", str(port),
             "-no-window", "-no-audio", "-no-snapshot", "-gpu", "host"],
            stdout=writer, stderr=subprocess.STDOUT, env=env,
        )
        try:
            def booted():
                try:
                    return adb("shell", "getprop", "sys.boot_completed", timeout=5, stderr=subprocess.DEVNULL).strip() == b"1"
                except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
                    return False
            until(f"Android boot (see {emulator_log})", booted, seconds=240, process=emulator)
            adb("shell", "input", "keyevent", "KEYCODE_WAKEUP")
            adb("shell", "wm", "dismiss-keyguard")
            adb("install", "-r", ROOT / "target/android/bboxy450-term-mobile.apk", timeout=60)
            adb("shell", "am", "force-stop", APP)
            adb("logcat", "-c")
            component = f"{APP}/android.app.NativeActivity"
            adb("shell", "am", "start", "-W", "-n", component)
            log = OUT / "android.log"

            def messages(token, after=None):
                data = adb("logcat", "-d", "-s", "bboxy450-mobile", "AndroidRuntime")
                log.write_bytes(data)
                return marker(log, token, after)

            def shot(path):
                path.write_bytes(adb("exec-out", "screencap", "-p"))

            ready = until(f"Android {READY} (see {log})", lambda: messages(READY), process=emulator)
            print(ready)
            x, y = capture("android", shot)
            adb("shell", "input", "tap", str(x), str(y))
            answered = OUT / "android-answered.png"

            def answer_visible():
                shot(answered)
                frame = Shot(answered)
                return near(frame.pixel(x, y), CARD) and (
                    frame.ink_samples(ANSWERED, y - 24, y + 32) > 8
                )

            until("Android tap removed the first answer button", answer_visible, seconds=15, process=emulator)
            adb("shell", "input", "keyevent", "KEYCODE_HOME")
            until("Android surface suspension", lambda: messages(SUSPEND), process=emulator)
            adb("shell", "am", "start", "-W", "-n", component)
            until("Android surface recreation", lambda: messages(READY, after=SUSPEND), process=emulator)
            until("Android answer retained through resume", answer_visible, seconds=15, process=emulator)
            print(f"android: tap changed the first row, and the change survived surface recreation ({answered})")
        finally:
            stop(emulator)


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    signal.signal(signal.SIGTERM, lambda signum, _: sys.exit(128 + signum))
    {"ios": ios, "android": android}[sys.argv[1]]()
