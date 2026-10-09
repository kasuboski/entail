#!/usr/bin/env python3
"""Black-box framed-protocol and OS-process acceptance tests for entail worker.

Usage: python3 scripts/worker_acceptance.py /path/to/entail
Only the eventual real worker binary can pass these tests.
"""
from __future__ import annotations

import argparse
import json
import os
import select
import struct
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "author-engine-v0"
PROTOCOL = "author-worker/0.1"
MAX_DEFAULT = 8 * 1024 * 1024
IO_TIMEOUT_SECONDS = 10


class AcceptanceFailure(AssertionError):
    pass


def encode_frame(value: Any) -> bytes:
    payload = json.dumps(value, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return struct.pack(">I", len(payload)) + payload


def decode_payload(payload: bytes) -> Any:
    return json.loads(payload.decode("utf-8"))


def read_exact(stream: Any, length: int, timeout: float = IO_TIMEOUT_SECONDS) -> bytes:
    chunks = bytearray()
    deadline = time.monotonic() + timeout
    try:
        fd = stream.fileno()
    except (AttributeError, OSError):
        fd = None
    while len(chunks) < length:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise AcceptanceFailure(f"timed out reading worker frame ({len(chunks)} of {length} bytes)")
        if fd is None:
            part = stream.read(length - len(chunks))
        else:
            if not select.select([fd], [], [], remaining)[0]:
                raise AcceptanceFailure(f"timed out reading worker frame ({len(chunks)} of {length} bytes)")
            part = os.read(fd, length - len(chunks))
        if not part:
            raise EOFError(f"worker closed stream after {len(chunks)} of {length} bytes")
        chunks.extend(part)
    return bytes(chunks)


def read_frame(stream: Any, max_bytes: int = MAX_DEFAULT) -> Any:
    length = struct.unpack(">I", read_exact(stream, 4))[0]
    if length > max_bytes:
        raise AcceptanceFailure(f"worker response frame exceeds client limit ({length} > {max_bytes})")
    return decode_payload(read_exact(stream, length))


def write_fragmented(stream: Any, frame: bytes, fragment_size: int = 4096) -> None:
    """Write every byte, optionally splitting frames into tiny OS-pipe writes."""
    fd = stream.fileno()
    for offset in range(0, len(frame), fragment_size):
        data = memoryview(frame)[offset:offset + fragment_size]
        while data:
            written = os.write(fd, data)
            data = data[written:]
    stream.flush()


class Worker:
    def __init__(self, binary: Path):
        self.binary = binary
        self.process = subprocess.Popen([str(binary), "worker"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, bufsize=0)
        assert self.process.stdin is not None and self.process.stdout is not None
        self.next_id = 1

    def request(self, op: str, params: dict[str, Any] | None = None, fragmented: bool = False) -> dict[str, Any]:
        request_id = f"acceptance-{self.next_id}"
        self.next_id += 1
        request = {"protocol_version": PROTOCOL, "request_id": request_id, "op": op, "params": params or {}}
        frame = encode_frame(request)
        if fragmented:
            write_fragmented(self.process.stdin, frame, 1)
        else:
            write_fragmented(self.process.stdin, frame)
        response = read_frame(self.process.stdout)
        if not isinstance(response, dict) or response.get("protocol_version") != PROTOCOL or response.get("request_id") != request_id:
            raise AcceptanceFailure(f"invalid response envelope for {op}: {response!r}")
        if ("result" in response) == ("error" in response):
            raise AcceptanceFailure(f"response must contain exactly one of result/error: {response!r}")
        return response

    def close(self, timeout: float = 5.0) -> int:
        if self.process.poll() is None:
            self.process.kill()
        try:
            return self.process.wait(timeout=timeout)
        except subprocess.TimeoutExpired as exc:
            raise AcceptanceFailure("worker did not exit after kill") from exc


def _objects(value: Any):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from _objects(child)
    elif isinstance(value, list):
        for child in value:
            yield from _objects(child)


def expect_error(response: dict[str, Any], code: str, context: str) -> None:
    error = response.get("error")
    if not isinstance(error, dict) or str(error.get("code", "")).lower() != code.lower():
        raise AcceptanceFailure(f"{context}: expected error code {code}, got {response!r}")


def result(response: dict[str, Any], context: str) -> dict[str, Any]:
    value = response.get("result")
    if not isinstance(value, dict):
        raise AcceptanceFailure(f"{context}: expected result object, got {response!r}")
    return value


def compile_model(worker: Worker, fixture: str) -> dict[str, Any]:
    model = json.loads((BUNDLE / "fixtures" / "valid" / f"{fixture}.json").read_text(encoding="utf-8"))
    value = result(worker.request("compile", {"model": model}), "compile")
    if value.get("handle") is None:
        raise AcceptanceFailure(f"valid fixture {fixture} compiled without a handle: {value!r}")
    return value


def run_worker_acceptance(binary: Path) -> None:
    if not binary.exists():
        raise AcceptanceFailure(f"binary does not exist: {binary}")
    worker = Worker(binary)
    old_handle: Any = None
    max_frame = MAX_DEFAULT
    try:
        hello_response = worker.request("hello", fragmented=True)
        hello = result(hello_response, "hello")
        limits = hello.get("limits", {})
        max_frame = int(limits.get("max_frame_bytes", hello.get("configured_limits", {}).get("max_frame_bytes", MAX_DEFAULT)))
        max_handles = int(limits.get("max_handles", 64))
        if max_handles < 1:
            raise AcceptanceFailure(f"hello advertised invalid max_handles: {max_handles}")

        compiled = compile_model(worker, "unsafe-grant")
        handle = compiled["handle"]
        analyzed = result(worker.request("analyze", {"handle": handle, "checks": ["coverage", "constraint_compliance"], "assumption_ids": [], "budget": {}}), "analyze")
        if not isinstance(analyzed, dict) or not any(k in analyzed for k in ("obligations", "results", "checks")):
            raise AcceptanceFailure(f"analyze did not return ordered check results: {analyzed!r}")
        scenario = json.loads((BUNDLE / "scenarios" / "unsafe-expiry.json").read_text(encoding="utf-8"))
        evaluated = result(worker.request("evaluate", {"handle": handle, "inputs": scenario["inputs"], "assumption_ids": []}), "evaluate")
        evaluated_objects = list(_objects(evaluated))
        violations = [
            value
            for obj in evaluated_objects
            for key in ("violated_constraint_ids", "violated_constraints")
            for value in (obj.get(key, []) if isinstance(obj.get(key, []), list) else [])
        ]
        if "ExpiryWithinContract" not in violations:
            raise AcceptanceFailure(f"unsafe scenario evaluation did not report the violated expiry constraint: {evaluated!r}")

        # Compile validation failures are normal compile results (no handle), not transport errors.
        invalid_model = json.loads((BUNDLE / "fixtures/invalid/unknown-reference.json").read_text(encoding="utf-8"))
        invalid_response = worker.request("compile", {"model": invalid_model})
        invalid = result(invalid_response, "invalid compile")
        if invalid.get("handle") is not None or not any(k in invalid for k in ("diagnostics", "errors")):
            raise AcceptanceFailure(f"invalid compile must return diagnostics without a handle: {invalid!r}")

        released = result(worker.request("release", {"handle": handle}), "release")
        if released.get("released") is not True:
            raise AcceptanceFailure(f"release did not report successful release: {released!r}")
        missing_release = result(worker.request("release", {"handle": handle}), "repeat release")
        if missing_release.get("released") is not False:
            raise AcceptanceFailure(f"release of absent handle should report false: {missing_release!r}")

        # Capacity must be observable and must not evict a still-live compiled model.
        # Fill the advertised bound exactly; keep host resource use capped at 128.
        first = compile_model(worker, "partial")
        live = [first["handle"]]
        capacity_seen = False
        if max_handles > 128:
            raise AcceptanceFailure(f"advertised capacity {max_handles} exceeds safe acceptance bound 128")
        for _ in range(max_handles):
            response = worker.request("compile", {"model": json.loads((BUNDLE / "fixtures/valid/partial.json").read_text(encoding="utf-8"))})
            if "error" in response:
                code = response["error"].get("code")
                if code != "capacity_exceeded":
                    raise AcceptanceFailure(f"compile failed before advertised capacity: {response!r}")
                capacity_seen = True
                break
            value = result(response, "capacity compile")
            if value.get("handle") is None:
                raise AcceptanceFailure(f"capacity compile returned no handle and no capacity error: {value!r}")
            live.append(value["handle"])
        if not capacity_seen:
            raise AcceptanceFailure(f"worker accepted {len(live)} live models despite advertised capacity {max_handles}")
        # Prove full capacity didn't invalidate an existing handle when it is reached.
        if capacity_seen:
            response = worker.request("analyze", {"handle": live[0], "checks": [], "assumption_ids": [], "budget": {}})
            if "error" in response:
                raise AcceptanceFailure(f"worker evicted a live handle at capacity: {response!r}")
        for item in live:
            worker.request("release", {"handle": item})

        # Unknown operation and protocol version are stable protocol errors with request IDs preserved.
        unknown = worker.request("not-an-operation")
        expect_error(unknown, "unknown_operation", "unknown operation")
        bad_version = worker.request("hello")
        # Send a manually versioned request because Worker.request uses the valid version.
        bad_request = {"protocol_version": "author-worker/999", "request_id": "bad-version", "op": "hello", "params": {}}
        worker.process.stdin.write(encode_frame(bad_request)); worker.process.stdin.flush()
        bad_reply = read_frame(worker.process.stdout)
        if bad_reply.get("request_id") != "bad-version":
            raise AcceptanceFailure(f"protocol error lost recoverable request ID: {bad_reply!r}")
        expect_error(bad_reply, "version_mismatch", "unsupported protocol version")
        del bad_version

        # Malformed JSON frame is an error response when its ID can be recovered, otherwise worker may close.
        malformed = b'{"protocol_version":"author-worker/0.1","request_id":"malformed","op":'
        write_fragmented(worker.process.stdin, struct.pack(">I", len(malformed)) + malformed)
        malformed_reply = read_frame(worker.process.stdout)
        if malformed_reply.get("request_id") not in ("malformed", None):
            raise AcceptanceFailure(f"malformed frame request ID was not null or recoverable: {malformed_reply!r}")
        expect_error(malformed_reply, "malformed_json", "malformed JSON frame")

        # Invalid unsigned frame length is fatal/nonzero, unlike a request-level error.
        rc = worker.close()
        if rc == 0:
            # A clean terminate can race after all replies; invalid-length case is isolated below instead.
            pass
    finally:
        if worker.process.poll() is None:
            worker.close()

    # Isolated invalid-length process: length > protocol maximum must close nonzero.
    doomed = Worker(binary)
    try:
        hello = result(doomed.request("hello"), "hello limits")
        bound = int(hello.get("limits", {}).get("max_frame_bytes", hello.get("configured_limits", {}).get("max_frame_bytes", MAX_DEFAULT)))
        write_fragmented(doomed.process.stdin, struct.pack(">I", bound + 1))
        code = doomed.process.wait(timeout=5)
        if code == 0:
            raise AcceptanceFailure("invalid frame length did not close worker with nonzero exit")
    finally:
        if doomed.process.poll() is None:
            doomed.close()

    # Supervisor lifecycle: terminate and reap, restart, old handle unknown, compile again.
    first_worker = Worker(binary)
    try:
        old_handle = compile_model(first_worker, "partial")["handle"]
    finally:
        first_worker.close()
        if first_worker.process.poll() is None:
            raise AcceptanceFailure("terminated worker was not reaped")
    restarted = Worker(binary)
    try:
        unknown = restarted.request("analyze", {"handle": old_handle, "checks": [], "assumption_ids": [], "budget": {}})
        expect_error(unknown, "unknown_handle", "handle invalid after restart")
        new_handle = compile_model(restarted, "partial")["handle"]
        if new_handle == old_handle:
            # Opaque handle values may be reproducible, but the old handle was rejected above.
            pass
        result(restarted.request("release", {"handle": new_handle}), "release after restart")
    finally:
        restarted.close()
    print(f"PASS: real worker protocol/lifecycle acceptance against {binary}")


def self_tests() -> None:
    payload = {"protocol_version": PROTOCOL, "request_id": "roundtrip", "op": "hello", "params": {}}
    frame = encode_frame(payload)
    assert struct.unpack(">I", frame[:4])[0] == len(frame) - 4
    assert decode_payload(frame[4:]) == payload
    assert read_exact(__import__("io").BytesIO(b"abc"), 3) == b"abc"
    # Exercise terminate/wait/reap lifecycle using a child Python process, not an entail stand-in.
    child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
    child.terminate()
    child.wait(timeout=5)
    assert child.poll() is not None
    print("PASS: framing and subprocess lifecycle helper self-tests (not worker acceptance)")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", nargs="?", type=Path, help="path to the built entail executable")
    parser.add_argument("--self-test", action="store_true", help="test only local framing/process helpers")
    args = parser.parse_args()
    try:
        if args.self_test:
            self_tests(); return 0
        if args.binary is None:
            parser.error("provide the entail binary path")
        run_worker_acceptance(args.binary.resolve())
        return 0
    except (AcceptanceFailure, OSError, ValueError, subprocess.TimeoutExpired) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
