#!/usr/bin/env python3
"""
Wasteland 全面漏洞扫描器
扫描 Rust/Python/GDScript + 依赖审计 + Cargo audit
"""

import os
import sys
import re
import json
import subprocess
from pathlib import Path
from datetime import datetime

IS_WINDOWS = os.name == 'nt'
PROJECT_ROOT = Path("d:/rj/wasteland_project") if IS_WINDOWS else Path.home() / "wasteland_project"

RUST_PATTERNS = {
    "CRITICAL": [
        (r"unsafe\s*\{", "RUST-001: unsafe block detected - review required"),
        (r"std::mem::transmute", "RUST-002: transmute usage - potential UB"),
        (r"std::ptr::null\s*\(\s*\)", "RUST-003: null pointer usage"),
    ],
    "HIGH": [
        (r"unwrap\s*\(\s*\)", "RUST-101: unwrap() will panic on None/Err"),
        (r"expect\s*\(\s*\"", "RUST-102: expect with message - better than unwrap but still panic"),
        (r"\.clone\s*\(\s*\)", "RUST-103: clone() may cause unnecessary allocation"),
    ],
    "MEDIUM": [
        (r"#\[allow\(dead_code\)\]", "RUST-201: dead_code allowed - review if needed"),
        (r"#\[allow\(unused\)\]", "RUST-202: unused allowed"),
        (r"TODO", "RUST-203: TODO marker"),
        (r"FIXME", "RUST-204: FIXME marker"),
        (r"HACK", "RUST-205: HACK marker"),
    ],
}

PYTHON_PATTERNS = {
    "CRITICAL": [
        (r"os\.system\s*\(.*\)", "PY-001: os.system() - command injection risk"),
        (r"subprocess\.call\s*\(.*shell\s*=\s*True", "PY-002: subprocess with shell=True"),
        (r"eval\s*\(.*\)", "PY-003: eval() usage"),
        (r"exec\s*\(.*\)", "PY-004: exec() usage"),
        (r"pickle\.load", "PY-005: pickle.load - deserialization risk"),
    ],
    "HIGH": [
        (r"except\s*:", "PY-101: bare except"),
        (r"except\s*Exception", "PY-102: broad exception catch"),
        (r"assert\s.*==.*password", "PY-103: assert with password"),
        (r"print\s*\(.*password", "PY-104: printing password"),
        (r"print\s*\(.*token", "PY-105: printing token"),
    ],
    "MEDIUM": [
        (r"yaml\.load\s*\(.*\)", "PY-201: use yaml.safe_load instead"),
        (r"open\s*\(.*,\s*[\'\"]w", "PY-202: file write - check path"),
        (r"os\.remove\s*\(.*\)", "PY-203: file deletion - confirm safe"),
        (r"shutil\.rmtree", "PY-204: recursive deletion"),
    ],
}

GDSCRIPT_PATTERNS = {
    "CRITICAL": [
        (r"OS\.execute\s*\(.*get_node", "GD-001: Dynamic OS.execute - RCE risk"),
        (r"FileAccess\.open\s*\(.*get_node", "GD-002: FileAccess from user input"),
    ],
    "HIGH": [
        (r"OS\.execute\s*\(.*\)", "GD-101: OS.execute usage"),
        (r"JSON\.parse_string\s*\(.*HTTPRequest", "GD-102: Unsafe JSON from network"),
        (r"\.call\s*\(\s*[\'\"]set", "GD-103: Dynamic method call"),
    ],
    "MEDIUM": [
        (r"randi\s*\(\s*\)\s*%\s*\d+", "GD-201: Use randi_range instead of randi()%N"),
        (r"print\s*\(.*password", "GD-202: Password in print"),
        (r"print\s*\(.*secret", "GD-203: Secret in print"),
    ],
    "LOW": [
        (r"queue_free\s*\(\s*\)", "GD-301: Direct queue_free"),
        (r"#\s*TODO", "GD-302: TODO check"),
        (r"#\s*FIXME", "GD-303: FIXME check"),
    ],
}

def scan_files(directory, patterns, extensions):
    findings = []
    files_scanned = 0
    lines_scanned = 0

    if not os.path.exists(directory):
        return findings, 0, 0

    for root, dirs, files in os.walk(directory):
        dirs[:] = [d for d in dirs if d not in ('target', '.git', 'node_modules', '__pycache__', '.trae', '.venv', '.vscode')]
        for f in files:
            if "comprehensive_scanner" in f.lower():
                continue
            if any(f.endswith(ext) for ext in extensions):
                file_path = os.path.join(root, f)
                try:
                    with open(file_path, 'r', encoding='utf-8', errors='ignore') as fh:
                        lines = fh.readlines()
                except:
                    continue
                files_scanned += 1
                lines_scanned += len(lines)
                for i, line in enumerate(lines, 1):
                    if line.strip().startswith('#') or line.strip().startswith('//'):
                        continue
                    for severity, pats in patterns.items():
                        for pattern, message in pats:
                            if re.search(pattern, line, re.IGNORECASE):
                                findings.append({
                                    "file": file_path,
                                    "line": i,
                                    "severity": severity,
                                    "message": message,
                                    "code": line.strip()[:150],
                                })
    return findings, files_scanned, lines_scanned

def run_cargo_audit():
    print("\n[Cargo Audit] Scanning for known vulnerabilities...")
    try:
        result = subprocess.run(
            ["cargo", "audit"],
            cwd=str(PROJECT_ROOT),
            capture_output=True, text=True, timeout=120
        )
        return {
            "tool": "cargo-audit",
            "exit_code": result.returncode,
            "output": result.stdout[-2000:] + result.stderr[-500:],
        }
    except FileNotFoundError:
        return {"tool": "cargo-audit", "error": "Not installed. Run: cargo install cargo-audit"}
    except subprocess.TimeoutExpired:
        return {"tool": "cargo-audit", "error": "Timed out"}
    except Exception as e:
        return {"tool": "cargo-audit", "error": str(e)}

def run_pip_audit(script_dirs):
    print("\n[Pip Audit] Scanning Python packages...")
    try:
        result = subprocess.run(
            [sys.executable, "-m", "pip_audit"],
            capture_output=True, text=True, timeout=60
        )
        return {
            "tool": "pip-audit",
            "exit_code": result.returncode,
            "output": result.stdout[-2000:] + result.stderr[-500:],
        }
    except:
        return {"tool": "pip-audit", "error": "Not installed. Run: pip install pip-audit"}

def run_bandit(script_dir):
    print(f"\n[Bandit] Scanning {script_dir}...")
    try:
        result = subprocess.run(
            [sys.executable, "-m", "bandit", "-r", script_dir, "-f", "json", "-q"],
            capture_output=True, text=True, timeout=60
        )
        if result.stdout.strip():
            data = json.loads(result.stdout)
            return {
                "tool": "bandit",
                "path": script_dir,
                "issues": len(data.get("results", [])),
                "high": sum(1 for r in data.get("results", []) if r.get("issue_severity") == "HIGH"),
                "medium": sum(1 for r in data.get("results", []) if r.get("issue_severity") == "MEDIUM"),
            }
        return {"tool": "bandit", "path": script_dir, "issues": 0}
    except:
        return {"tool": "bandit", "path": script_dir, "error": "Failed or not installed"}

def main():
    print("=" * 70)
    print("  WASTELAND COMPREHENSIVE VULNERABILITY SCANNER")
    print(f"  Time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 70)

    all_findings = {
        "timestamp": datetime.now().isoformat(),
        "project": str(PROJECT_ROOT),
        "scans": {},
        "findings": [],
        "summary": {},
    }

    scan_targets = [
        ("Rust", str(PROJECT_ROOT / "wasteland_engine" / "src"), RUST_PATTERNS, [".rs"]),
        ("Rust-GDExt", str(PROJECT_ROOT / "gdextension" / "src"), RUST_PATTERNS, [".rs"]),
        ("Python", str(PROJECT_ROOT / "scripts"), PYTHON_PATTERNS, [".py"]),
        ("GDScript", str(PROJECT_ROOT / "godot_project" / "scripts"), GDSCRIPT_PATTERNS, [".gd"]),
    ]

    for lang, directory, patterns, exts in scan_targets:
        findings, files, lines = scan_files(directory, patterns, exts)
        all_findings["scans"][lang] = {
            "files": files,
            "lines": lines,
            "findings": len(findings),
        }
        all_findings["findings"].extend(findings)

        by_sev = {}
        for f in findings:
            by_sev[f["severity"]] = by_sev.get(f["severity"], 0) + 1
        print(f"\n[{lang}] {files} files, {lines} lines, {len(findings)} findings")
        for sev in ["CRITICAL", "HIGH", "MEDIUM", "LOW"]:
            if by_sev.get(sev, 0) > 0:
                print(f"  {sev}: {by_sev[sev]}")

    cargo_result = run_cargo_audit()
    all_findings["scans"]["cargo-audit"] = cargo_result

    pip_result = run_pip_audit(str(PROJECT_ROOT / "scripts"))
    all_findings["scans"]["pip-audit"] = pip_result

    bandit_result = run_bandit(str(PROJECT_ROOT / "scripts"))
    all_findings["scans"]["bandit"] = bandit_result

    total = len(all_findings["findings"])
    critical = sum(1 for f in all_findings["findings"] if f["severity"] == "CRITICAL")
    high = sum(1 for f in all_findings["findings"] if f["severity"] == "HIGH")

    all_findings["summary"] = {
        "total_findings": total,
        "critical": critical,
        "high": high,
        "pass": total == 0,
    }

    report_path = PROJECT_ROOT / "reports" / "vulnerability_report.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    with open(report_path, 'w') as f:
        json.dump(all_findings, f, indent=2, ensure_ascii=False)

    print(f"\n{'='*70}")
    print(f"SUMMARY: {total} findings ({critical} CRITICAL, {high} HIGH)")
    print(f"Report saved: {report_path}")

    if critical > 0:
        print("\n!!! CRITICAL FINDINGS !!!")
        for f in all_findings["findings"]:
            if f["severity"] == "CRITICAL":
                print(f"  [{f['file']}:{f['line']}] {f['message']}")

    print("=" * 70)

if __name__ == "__main__":
    main()