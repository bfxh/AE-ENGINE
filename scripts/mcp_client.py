#!/usr/bin/env python3
"""Wasteland Editor MCP Client.

High-level Python client for driving the Wasteland Editor via the Model
Context Protocol (MCP) over the editor's HTTP bridge.

The editor writes its bound MCP HTTP bridge address to a well-known file
in the system temp dir:

    {temp_dir}/wasteland_editor_mcp_port.txt

This client reads that file to discover the port, then exposes all 15 MCP
tools as Python methods. Only the Python standard library is required.

Usage (as a library):

    from mcp_client import WastelandEditorClient

    with WastelandEditorClient.connect() as client:
        tree = client.get_scene_tree()
        node_id = client.create_node(parent_id=0, name="Cube", node_type="mesh")
        client.transform_node(node_id, translation=[1.0, 2.0, 3.0])
        client.save_scene("scene.wasteland")

Usage (CLI):

    python mcp_client.py status
    python mcp_client.py scene-tree
    python mcp_client.py create-node --name Cube --type mesh
    python mcp_client.py transform-node --id 1 --x 1 --y 2 --z 3
    python mcp_client.py tools
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile
import time
import urllib.error
import urllib.request
from typing import Any, Dict, List, Optional, Sequence

PORT_FILE_NAME = "wasteland_editor_mcp_port.txt"
DEFAULT_TIMEOUT = 5.0


def _port_file_path() -> str:
    """Return the path to the well-known port discovery file."""
    return os.path.join(tempfile.gettempdir(), PORT_FILE_NAME)


def discover_bridge_url(timeout: float = 0.0) -> Optional[str]:
    """Read the editor's MCP bridge URL from the well-known port file.

    Returns ``None`` if the file does not exist (editor not running or
    bridge not started). If ``timeout`` > 0, polls up to that many seconds
    waiting for the file to appear (useful when launching the editor and
    client in parallel).
    """
    deadline = time.monotonic() + timeout
    while True:
        try:
            with open(_port_file_path(), "r", encoding="utf-8") as f:
                addr = f.read().strip()
            if not addr:
                return None
            if not addr.startswith("http"):
                return f"http://{addr}/mcp"
            return addr if addr.endswith("/mcp") else f"{addr}/mcp"
        except FileNotFoundError:
            if time.monotonic() >= deadline:
                return None
            time.sleep(0.1)
        except OSError:
            return None


class McpError(Exception):
    """Raised when the MCP server returns a JSON-RPC error response."""

    def __init__(self, code: int, message: str, data: Any = None):
        super().__init__(f"[{code}] {message}")
        self.code = code
        self.message = message
        self.data = data


class BridgeUnavailableError(Exception):
    """Raised when the editor's HTTP bridge cannot be reached."""


class WastelandEditorClient:
    """Synchronous client for the Wasteland Editor MCP HTTP bridge.

    Each tool method maps 1:1 to an MCP tool exposed by the editor's
    JSON-RPC server. Methods return the parsed ``result`` field of the
    JSON-RPC response, or raise :class:`McpError` on a server-reported
    error, or :class:`BridgeUnavailableError` if the editor is not
    reachable.
    """

    def __init__(self, url: str, timeout: float = DEFAULT_TIMEOUT):
        self._url = url
        self._timeout = timeout
        self._next_id = 1

    # ------------------------------------------------------------------
    # Connection helpers
    # ------------------------------------------------------------------

    @classmethod
    def connect(
        cls,
        url: Optional[str] = None,
        *,
        timeout: float = DEFAULT_TIMEOUT,
        wait_for_port: float = 0.0,
    ) -> "WastelandEditorClient":
        """Create a client, auto-discovering the bridge URL if not given.

        If ``url`` is ``None``, reads the port file. If ``wait_for_port``
        > 0, polls that many seconds for the file to appear.
        """
        if url is None:
            url = discover_bridge_url(timeout=wait_for_port)
        if url is None:
            raise BridgeUnavailableError(
                f"Editor MCP bridge port file not found at {_port_file_path()!r}. "
                "Is the editor running?"
            )
        return cls(url, timeout=timeout)

    def __enter__(self) -> "WastelandEditorClient":
        return self

    def __exit__(self, *exc_info) -> None:
        pass

    # ------------------------------------------------------------------
    # Low-level transport
    # ------------------------------------------------------------------

    def _next_request_id(self) -> int:
        rid = self._next_id
        self._next_id += 1
        return rid

    def _post(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """POST a JSON-RPC request to /mcp and return the parsed response."""
        body = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            self._url,
            data=body,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=self._timeout) as resp:
                raw = resp.read().decode("utf-8")
        except urllib.error.URLError as e:
            raise BridgeUnavailableError(
                f"Cannot reach editor MCP bridge at {self._url}: {e}"
            ) from e
        except TimeoutError as e:
            raise BridgeUnavailableError(
                f"Editor MCP bridge timed out after {self._timeout}s"
            ) from e

        if not raw:
            raise BridgeUnavailableError(
                "Editor returned an empty response (504 gateway timeout likely)."
            )
        try:
            return json.loads(raw)
        except json.JSONDecodeError as e:
            raise BridgeUnavailableError(
                f"Editor returned non-JSON response: {raw!r}"
            ) from e

    def call_method(
        self,
        method: str,
        params: Optional[Dict[str, Any]] = None,
    ) -> Any:
        """Send a low-level JSON-RPC 2.0 request and return ``result``."""
        payload: Dict[str, Any] = {
            "jsonrpc": "2.0",
            "id": self._next_request_id(),
            "method": method,
        }
        if params is not None:
            payload["params"] = params
        resp = self._post(payload)
        if "error" in resp and resp["error"] is not None:
            err = resp["error"]
            raise McpError(
                err.get("code", -1),
                err.get("message", "unknown error"),
                err.get("data"),
            )
        return resp.get("result")

    def call_tool(
        self,
        name: str,
        arguments: Optional[Dict[str, Any]] = None,
    ) -> Any:
        """Invoke an MCP tool by name and return its result."""
        params: Dict[str, Any] = {"name": name}
        if arguments is not None:
            params["arguments"] = arguments
        return self.call_method("tools/call", params)

    # ------------------------------------------------------------------
    # Bridge status / raw transport
    # ------------------------------------------------------------------

    def status(self) -> Dict[str, int]:
        """GET /mcp/status — pending request/response counts."""
        url = self._url.replace("/mcp", "/mcp/status")
        try:
            with urllib.request.urlopen(url, timeout=self._timeout) as resp:
                return json.loads(resp.read().decode("utf-8"))
        except urllib.error.URLError as e:
            raise BridgeUnavailableError(f"status failed: {e}") from e

    def drain_responses(self) -> List[str]:
        """GET /mcp/responses — drain all pending responses (debugging)."""
        url = self._url.replace("/mcp", "/mcp/responses")
        try:
            with urllib.request.urlopen(url, timeout=self._timeout) as resp:
                return json.loads(resp.read().decode("utf-8"))
        except urllib.error.URLError as e:
            raise BridgeUnavailableError(f"drain failed: {e}") from e

    # ------------------------------------------------------------------
    # MCP tools (15 total)
    # ------------------------------------------------------------------

    # --- Scene tree (1) ---
    def get_scene_tree(self) -> Dict[str, Any]:
        return self.call_tool("get_scene_tree")

    # --- Node lifecycle (2-3) ---
    def create_node(
        self,
        parent_id: int,
        name: str,
        node_type: str = "empty",
        *,
        path: Optional[str] = None,
        light_type: Optional[str] = None,
        color: Optional[Sequence[float]] = None,
        intensity: Optional[float] = None,
        fov: Optional[float] = None,
        near: Optional[float] = None,
        far: Optional[float] = None,
    ) -> Dict[str, Any]:
        """Create a node. ``node_type`` in {empty, mesh, light, camera}."""
        args: Dict[str, Any] = {
            "parent_id": parent_id,
            "name": name,
            "node_type": node_type,
        }
        if node_type == "mesh" and path is not None:
            args["path"] = path
        if node_type == "light":
            if light_type is not None:
                args["light_type"] = light_type
            if color is not None:
                args["color"] = list(color)
            if intensity is not None:
                args["intensity"] = intensity
        if node_type == "camera":
            if fov is not None:
                args["fov"] = fov
            if near is not None:
                args["near"] = near
            if far is not None:
                args["far"] = far
        return self.call_tool("create_node", args)

    def delete_node(self, node_id: int) -> Dict[str, Any]:
        return self.call_tool("delete_node", {"node_id": node_id})

    # --- Node properties (4-5) ---
    def set_node_property(
        self,
        node_id: int,
        property: str,
        value: Any,
    ) -> Dict[str, Any]:
        """Set a property on a node.

        Supported ``property`` values:
        name, translation, scale, rotation, path, intensity, fov
        """
        return self.call_tool(
            "set_node_property",
            {"node_id": node_id, "property": property, "value": value},
        )

    def get_node_properties(self, node_id: int) -> Dict[str, Any]:
        return self.call_tool("get_node_properties", {"node_id": node_id})

    # --- Transform (6) ---
    def transform_node(
        self,
        node_id: int,
        *,
        translation: Optional[Sequence[float]] = None,
        rotation: Optional[Sequence[float]] = None,
        scale: Optional[Sequence[float]] = None,
    ) -> Dict[str, Any]:
        """Set the transform of a node. Each component is [x, y, z]."""
        args: Dict[str, Any] = {"node_id": node_id}
        if translation is not None:
            args["translation"] = list(translation)
        if rotation is not None:
            args["rotation"] = list(rotation)
        if scale is not None:
            args["scale"] = list(scale)
        return self.call_tool("transform_node", args)

    # --- Selection (7-8) ---
    def select_node(self, node_id: int) -> Dict[str, Any]:
        return self.call_tool("select_node", {"node_id": node_id})

    def get_selection(self) -> Dict[str, Any]:
        return self.call_tool("get_selection")

    # --- Scene I/O (9-11) ---
    def save_scene(self, path: str) -> Dict[str, Any]:
        return self.call_tool("save_scene", {"path": path})

    def load_scene(self, path: str) -> Dict[str, Any]:
        return self.call_tool("load_scene", {"path": path})

    def new_scene(self) -> Dict[str, Any]:
        return self.call_tool("new_scene")

    # --- Validation & batch (12-13) ---
    def validate_scene(self) -> Dict[str, Any]:
        return self.call_tool("validate_scene")

    def batch_execute(
        self,
        commands: List[Dict[str, Any]],
    ) -> Dict[str, Any]:
        """Execute a batch of tool calls atomically.

        Each command is a dict with ``name`` and ``arguments`` keys.
        """
        return self.call_tool("batch_execute", {"commands": commands})

    # --- Editor state & camera (14-15) ---
    def get_editor_state(self) -> Dict[str, Any]:
        return self.call_tool("get_editor_state")

    def set_camera_view(
        self,
        *,
        position: Optional[Sequence[float]] = None,
        target: Optional[Sequence[float]] = None,
        fov: Optional[float] = None,
    ) -> Dict[str, Any]:
        args: Dict[str, Any] = {}
        if position is not None:
            args["position"] = list(position)
        if target is not None:
            args["target"] = list(target)
        if fov is not None:
            args["fov"] = fov
        return self.call_tool("set_camera_view", args)

    # ------------------------------------------------------------------
    # Convenience
    # ------------------------------------------------------------------

    def list_tools(self) -> Any:
        """Return the editor's advertised tool list (JSON-RPC tools/list)."""
        return self.call_method("tools/list")

    def initialize(self) -> Any:
        """Send the JSON-RPC initialize handshake."""
        return self.call_method("initialize")


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------


def _print_json(obj: Any) -> None:
    print(json.dumps(obj, indent=2, ensure_ascii=False))


def _cli_status(args: argparse.Namespace) -> int:
    try:
        client = WastelandEditorClient.connect()
    except BridgeUnavailableError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    _print_json(client.status())
    return 0


def _cli_scene_tree(args: argparse.Namespace) -> int:
    with WastelandEditorClient.connect() as c:
        _print_json(c.get_scene_tree())
    return 0


def _cli_create_node(args: argparse.Namespace) -> int:
    with WastelandEditorClient.connect() as c:
        result = c.create_node(
            parent_id=args.parent_id,
            name=args.name,
            node_type=args.type,
            path=args.path,
        )
    _print_json(result)
    return 0


def _cli_transform_node(args: argparse.Namespace) -> int:
    with WastelandEditorClient.connect() as c:
        result = c.transform_node(
            args.id,
            translation=[args.x, args.y, args.z] if args.x is not None else None,
        )
    _print_json(result)
    return 0


def _cli_save_scene(args: argparse.Namespace) -> int:
    with WastelandEditorClient.connect() as c:
        _print_json(c.save_scene(args.path))
    return 0


def _cli_load_scene(args: argparse.Namespace) -> int:
    with WastelandEditorClient.connect() as c:
        _print_json(c.load_scene(args.path))
    return 0


def _cli_tools(args: argparse.Namespace) -> int:
    with WastelandEditorClient.connect() as c:
        _print_json(c.list_tools())
    return 0


def _cli_editor_state(args: argparse.Namespace) -> int:
    with WastelandEditorClient.connect() as c:
        _print_json(c.get_editor_state())
    return 0


def _cli_selection(args: argparse.Namespace) -> int:
    with WastelandEditorClient.connect() as c:
        if args.select is not None:
            _print_json(c.select_node(args.select))
        else:
            _print_json(c.get_selection())
    return 0


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="mcp_client",
        description="Wasteland Editor MCP client (stdlib only).",
    )
    sub = p.add_subparsers(dest="cmd", required=True)

    sub.add_parser("status", help="Show bridge pending counts.").set_defaults(
        func=_cli_status
    )
    sub.add_parser("scene-tree", help="Dump the scene tree as JSON.").set_defaults(
        func=_cli_scene_tree
    )
    sub.add_parser("tools", help="List MCP tools advertised by the editor.").set_defaults(
        func=_cli_tools
    )
    sub.add_parser("state", help="Get editor state.").set_defaults(
        func=_cli_editor_state
    )

    sel = sub.add_parser("selection", help="Get or set the current selection.")
    sel.add_argument("--select", type=int, default=None, help="Node ID to select.")
    sel.set_defaults(func=_cli_selection)

    cn = sub.add_parser("create-node", help="Create a new scene node.")
    cn.add_argument("--parent-id", type=int, default=0, help="Parent node ID (default 0=root).")
    cn.add_argument("--name", required=True, help="Node name.")
    cn.add_argument("--type", default="empty", choices=["empty", "mesh", "light", "camera"])
    cn.add_argument("--path", default=None, help="Mesh asset path (for --type mesh).")
    cn.set_defaults(func=_cli_create_node)

    tn = sub.add_parser("transform-node", help="Set node translation.")
    tn.add_argument("--id", type=int, required=True, help="Node ID.")
    tn.add_argument("--x", type=float, default=None)
    tn.add_argument("--y", type=float, default=None)
    tn.add_argument("--z", type=float, default=None)
    tn.set_defaults(func=_cli_transform_node)

    ss = sub.add_parser("save-scene", help="Save the current scene.")
    ss.add_argument("--path", required=True, help="Output file path.")
    ss.set_defaults(func=_cli_save_scene)

    ls = sub.add_parser("load-scene", help="Load a scene from a file.")
    ls.add_argument("--path", required=True, help="Input file path.")
    ls.set_defaults(func=_cli_load_scene)

    return p


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    try:
        return args.func(args)
    except BridgeUnavailableError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    except McpError as e:
        print(f"mcp error: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
