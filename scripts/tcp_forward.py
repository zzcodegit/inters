#!/usr/bin/env python3
import argparse
import asyncio
import json
import time
from contextlib import suppress
from pathlib import Path


def now_unix_ms() -> int:
    return int(time.time() * 1000)


def now_mono_ns() -> int:
    return time.monotonic_ns()


def ms_since(start_ns: int, end_ns: int | None = None) -> float:
    end_ns = now_mono_ns() if end_ns is None else end_ns
    return (end_ns - start_ns) / 1_000_000.0


def extract_host(request_prefix: bytes) -> str:
    marker = b"Host:"
    pos = request_prefix.find(marker)
    if pos < 0:
        return "unknown"
    idx = pos + len(marker)
    while idx < len(request_prefix) and request_prefix[idx] in b" \t":
        idx += 1
    end = idx
    while end < len(request_prefix) and request_prefix[end] not in b"\r\n":
        end += 1
    host = request_prefix[idx:end].decode("utf-8", errors="ignore").strip()
    if not host:
        return "unknown"
    if host.startswith("[") and "]" in host:
        return host[1 : host.index("]")]
    if ":" in host:
        candidate, port = host.rsplit(":", 1)
        if port.isdigit():
            return candidate
    return host


def append_jsonl(path: str | None, payload: dict) -> None:
    if not path:
        return
    out = Path(path)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(payload, sort_keys=True))
        handle.write("\n")


async def pipe_client_to_target(
    client_reader: asyncio.StreamReader,
    target_writer: asyncio.StreamWriter,
    state: dict,
) -> None:
    try:
        while True:
            chunk = await client_reader.read(65536)
            if not chunk:
                break
            if state["request_host"] == "unknown" and len(state["request_prefix"]) < 4096:
                state["request_prefix"].extend(chunk[: 4096 - len(state["request_prefix"])])
                state["request_host"] = extract_host(bytes(state["request_prefix"]))
            state["bytes_from_client"] += len(chunk)
            target_writer.write(chunk)
            await target_writer.drain()
    finally:
        with suppress(Exception):
            target_writer.write_eof()


async def pipe_target_to_client(
    target_reader: asyncio.StreamReader,
    client_writer: asyncio.StreamWriter,
    state: dict,
) -> None:
    try:
        while True:
            chunk = await target_reader.read(65536)
            if not chunk:
                break
            if state["first_target_byte_ns"] is None:
                state["first_target_byte_ns"] = now_mono_ns()
            state["bytes_to_client"] += len(chunk)
            client_writer.write(chunk)
            await client_writer.drain()
    finally:
        with suppress(Exception):
            client_writer.close()
            await client_writer.wait_closed()


async def handle_client(
    client_reader: asyncio.StreamReader,
    client_writer: asyncio.StreamWriter,
    target_host: str,
    target_port: int,
    stage_log_path: str | None,
) -> None:
    accept_ns = now_mono_ns()
    client_peer = client_writer.get_extra_info("peername")
    state = {
        "request_prefix": bytearray(),
        "request_host": "unknown",
        "bytes_from_client": 0,
        "bytes_to_client": 0,
        "first_target_byte_ns": None,
    }

    connect_start_ns = now_mono_ns()
    try:
        target_reader, target_writer = await asyncio.open_connection(target_host, target_port)
    except Exception as exc:
        append_jsonl(
            stage_log_path,
            {
                "component": "direct_forward",
                "stage": "connection_failed",
                "timestamp_unix_ms": now_unix_ms(),
                "client_peer": str(client_peer),
                "target_addr": f"{target_host}:{target_port}",
                "target_connect_ms": ms_since(connect_start_ns),
                "error": str(exc),
            },
        )
        with suppress(Exception):
            client_writer.close()
            await client_writer.wait_closed()
        return

    connect_done_ns = now_mono_ns()
    await asyncio.gather(
        pipe_client_to_target(client_reader, target_writer, state),
        pipe_target_to_client(target_reader, client_writer, state),
        return_exceptions=True,
    )
    total_done_ns = now_mono_ns()

    append_jsonl(
        stage_log_path,
        {
            "component": "direct_forward",
            "stage": "connection_complete",
            "timestamp_unix_ms": now_unix_ms(),
            "request_host": state["request_host"],
            "client_peer": str(client_peer),
            "target_addr": f"{target_host}:{target_port}",
            "accept_to_target_connect_ms": ms_since(accept_ns, connect_done_ns),
            "target_connect_ms": ms_since(connect_start_ns, connect_done_ns),
            "accept_to_first_target_byte_ms": (
                ms_since(accept_ns, state["first_target_byte_ns"])
                if state["first_target_byte_ns"] is not None
                else None
            ),
            "first_target_byte_after_connect_ms": (
                ms_since(connect_done_ns, state["first_target_byte_ns"])
                if state["first_target_byte_ns"] is not None
                else None
            ),
            "total_ms": ms_since(accept_ns, total_done_ns),
            "bytes_from_client": state["bytes_from_client"],
            "bytes_to_client": state["bytes_to_client"],
        },
    )

    with suppress(Exception):
        target_writer.close()
        await target_writer.wait_closed()


async def main() -> None:
    parser = argparse.ArgumentParser(description="Simple TCP forwarder")
    parser.add_argument("--listen-host", required=True)
    parser.add_argument("--listen-port", required=True, type=int)
    parser.add_argument("--target-host", required=True)
    parser.add_argument("--target-port", required=True, type=int)
    parser.add_argument("--stage-log-path")
    args = parser.parse_args()

    server = await asyncio.start_server(
        lambda r, w: handle_client(
            r,
            w,
            args.target_host,
            args.target_port,
            args.stage_log_path,
        ),
        args.listen_host,
        args.listen_port,
    )
    async with server:
        await server.serve_forever()


if __name__ == "__main__":
    asyncio.run(main())
