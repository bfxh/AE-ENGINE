import json
import os
import sys
import time
import urllib.request
import urllib.error
import base64
from pathlib import Path

IS_WINDOWS = os.name == "nt"
COMFYUI_URL = os.environ.get("COMFYUI_URL", "http://127.0.0.1:8188")
OUTPUT_DIR = Path(os.environ.get("COMFYUI_OUTPUT", str(Path.home() / "comfyui_output" if not IS_WINDOWS else "D:/AI/comfyui_output")))


def queue_prompt(workflow_path: str, prompt_overrides: dict = None) -> str:
    with open(workflow_path, "r", encoding="utf-8") as f:
        workflow = json.load(f)

    if prompt_overrides:
        for node_id, overrides in prompt_overrides.items():
            if node_id in workflow.get("nodes", {}):
                node = workflow["nodes"][node_id]
                if "widgets_values" in node and isinstance(overrides, list):
                    for i, val in enumerate(overrides):
                        if i < len(node["widgets_values"]):
                            node["widgets_values"][i] = val

    prompt_data = {"prompt": _extract_prompt_from_workflow(workflow)}

    req = urllib.request.Request(
        f"{COMFYUI_URL}/prompt",
        data=json.dumps(prompt_data).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            result = json.loads(resp.read().decode("utf-8"))
            prompt_id = result.get("prompt_id", "")
            print(f"[ComfyUI] Queued prompt: {prompt_id}")
            return prompt_id
    except urllib.error.URLError as e:
        print(f"[ComfyUI] Error queueing prompt: {e}")
        return ""


def _extract_prompt_from_workflow(workflow: dict) -> dict:
    prompt = {}
    for node in workflow.get("nodes", []):
        node_id = str(node.get("id", ""))
        node_type = node.get("type", "")
        widgets_values = node.get("widgets_values", [])

        prompt[node_id] = {
            "class_type": node_type,
            "inputs": _widgets_to_inputs(node_type, widgets_values, workflow, node_id),
        }

    for link in workflow.get("links", []):
        link_id, from_node, from_slot, to_node, to_slot, link_type = link[:6]
        from_id = str(from_node)
        to_id = str(to_node)

        if to_id in prompt and from_id in prompt:
            input_name = _get_input_name(prompt[to_id]["class_type"], to_slot)
            if input_name:
                prompt[to_id]["inputs"][input_name] = [from_id, from_slot]

    return prompt


def _widgets_to_inputs(node_type: str, widgets_values: list, workflow: dict, node_id: str) -> dict:
    inputs = {}
    if node_type == "CheckpointLoaderSimple":
        if widgets_values:
            inputs["ckpt_name"] = widgets_values[0]
    elif node_type == "CLIPTextEncode":
        if widgets_values:
            inputs["text"] = widgets_values[0]
    elif node_type == "EmptyLatentImage":
        if len(widgets_values) >= 3:
            inputs["width"] = widgets_values[0]
            inputs["height"] = widgets_values[1]
            inputs["batch_size"] = widgets_values[2]
    elif node_type == "KSampler":
        if len(widgets_values) >= 7:
            inputs["seed"] = widgets_values[0]
            inputs["sampler_name"] = widgets_values[4]
            inputs["scheduler"] = widgets_values[5]
            inputs["steps"] = widgets_values[2]
            inputs["cfg"] = widgets_values[3]
            inputs["denoise"] = widgets_values[6]
    elif node_type == "SaveImage":
        if widgets_values:
            inputs["filename_prefix"] = widgets_values[0]
    return inputs


def _get_input_name(class_type: str, slot: int) -> str:
    input_map = {
        "KSampler": {0: "model", 1: "positive", 2: "negative", 3: "latent_image"},
        "VAEDecode": {0: "samples", 1: "vae"},
        "SaveImage": {0: "images"},
        "CLIPTextEncode": {0: "clip"},
    }
    return input_map.get(class_type, {}).get(slot, f"input_{slot}")


def wait_for_completion(prompt_id: str, timeout: float = 300.0) -> bool:
    start = time.time()
    while time.time() - start < timeout:
        try:
            req = urllib.request.Request(f"{COMFYUI_URL}/history/{prompt_id}")
            with urllib.request.urlopen(req, timeout=10) as resp:
                history = json.loads(resp.read().decode("utf-8"))
                if prompt_id in history:
                    status = history[prompt_id].get("status", {})
                    if status.get("completed", False) or status.get("status_str") == "success":
                        print(f"[ComfyUI] Prompt {prompt_id} completed")
                        return True
                    if status.get("status_str") == "error":
                        print(f"[ComfyUI] Prompt {prompt_id} failed")
                        return False
        except urllib.error.URLError:
            pass

        time.sleep(2.0)

    print(f"[ComfyUI] Timeout waiting for {prompt_id}")
    return False


def get_output_images(prompt_id: str) -> list:
    try:
        req = urllib.request.Request(f"{COMFYUI_URL}/history/{prompt_id}")
        with urllib.request.urlopen(req, timeout=10) as resp:
            history = json.loads(resp.read().decode("utf-8"))

        if prompt_id not in history:
            return []

        images = []
        outputs = history[prompt_id].get("outputs", {})
        for node_id, node_output in outputs.items():
            for img_info in node_output.get("images", []):
                filename = img_info.get("filename", "")
                subfolder = img_info.get("subfolder", "")
                img_type = img_info.get("type", "output")

                url = f"{COMFYUI_URL}/view?filename={filename}&subfolder={subfolder}&type={img_type}"
                images.append({"url": url, "filename": filename, "subfolder": subfolder})

        return images
    except urllib.error.URLError as e:
        print(f"[ComfyUI] Error getting output: {e}")
        return []


def download_images(prompt_id: str, output_dir: str = None) -> list:
    out = Path(output_dir) if output_dir else OUTPUT_DIR
    out.mkdir(parents=True, exist_ok=True)

    images = get_output_images(prompt_id)
    downloaded = []

    for img in images:
        try:
            req = urllib.request.Request(img["url"])
            with urllib.request.urlopen(req, timeout=30) as resp:
                data = resp.read()

            dest = out / img["filename"]
            with open(dest, "wb") as f:
                f.write(data)

            downloaded.append(str(dest))
            print(f"[ComfyUI] Downloaded: {dest}")
        except urllib.error.URLError as e:
            print(f"[ComfyUI] Error downloading {img['filename']}: {e}")

    return downloaded


def generate_6view(object_description: str, style: str = "wasteland post-apocalyptic") -> list:
    workflow_path = Path(__file__).parent / "comfyui_workflows" / "text_to_6view.json"

    if not workflow_path.exists():
        print(f"[ComfyUI] Workflow not found: {workflow_path}")
        return []

    views = ["front", "right side", "back", "left side", "top-down aerial", "bottom-up"]
    all_images = []

    for i, view in enumerate(views):
        prompt_text = f"{style} {object_description}, {view} view, photorealistic, detailed, 8k"

        overrides = {
            "2": [prompt_text],
            "8": [prompt_text.replace("front", view).replace("side", view)],
        }

        prompt_id = queue_prompt(str(workflow_path), overrides)
        if not prompt_id:
            continue

        if wait_for_completion(prompt_id):
            images = download_images(prompt_id)
            all_images.extend(images)

    print(f"[ComfyUI] Generated {len(all_images)} images for 6 views")
    return all_images


def check_server() -> bool:
    try:
        req = urllib.request.Request(f"{COMFYUI_URL}/system_stats")
        with urllib.request.urlopen(req, timeout=5) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            print(f"[ComfyUI] Server online: {data.get('system', {}).get('devices', [{}])[0].get('name', 'unknown')}")
            return True
    except urllib.error.URLError:
        print(f"[ComfyUI] Server not reachable at {COMFYUI_URL}")
        return False


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python comfyui_api.py [check|generate <description>|queue <workflow.json>]")
        sys.exit(1)

    cmd = sys.argv[1]

    if cmd == "check":
        check_server()
    elif cmd == "generate":
        desc = " ".join(sys.argv[2:]) if len(sys.argv) > 2 else "ruined building"
        generate_6view(desc)
    elif cmd == "queue":
        wf = sys.argv[2] if len(sys.argv) > 2 else ""
        if wf and os.path.exists(wf):
            pid = queue_prompt(wf)
            if pid:
                wait_for_completion(pid)
                download_images(pid)
        else:
            print(f"Workflow file not found: {wf}")
    else:
        print(f"Unknown command: {cmd}")
