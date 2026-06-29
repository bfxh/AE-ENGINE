#!/usr/bin/env python3
import os
import sys
import re
import json
from pathlib import Path
from datetime import datetime

VULN_PATTERNS = {
    "CRITICAL": [
        (r"OS\.execute\s*\(.*get_node", "GDSEC-001: Dynamic OS.execute with node input - potential RCE"),
        (r"FileAccess\.open\s*\(.*get_node", "GDSEC-002: FileAccess path from user input"),
        (r'OS\.execute\s*\(\s*"[^"]*cmd', "GDSEC-003: Direct cmd.exe execution"),
        (r"OS\.execute\s*\(.*str\s*\(.*\)", "GDSEC-004: OS.execute with string cast input"),
    ],
    "HIGH": [
        (r"JSON\.parse_string\s*\(.*HTTPRequest", "GDSEC-101: Unsafe JSON parsing from network"),
        (r"ResourceLoader\.load\s*\(.*get_node", "GDSEC-102: Dynamic resource loading from input"),
        (r'\.call\s*\(\s*[\"\']set', "GDSEC-103: Dynamic method call with set_ prefix"),
        (r"OS\.execute\s*\(.*\)", "GDSEC-104: OS.execute usage - review required"),
        (r"var\s+script\s*=\s*load\s*\(.*get_node", "GDSEC-105: Dynamic script loading"),
    ],
    "MEDIUM": [
        (r"randi\s*\(\s*\)\s*%\s*\d+", "GDSEC-201: randi()%N for security - use randi_range"),
        (r"str\s*\(\s*randi\s*\(\s*\)\s*%", "GDSEC-202: Random string generation for tokens"),
        (r"print\s*\(.*password", "GDSEC-203: Password printed to console"),
        (r"print\s*\(.*token", "GDSEC-204: Token printed to console"),
        (r"print\s*\(.*secret", "GDSEC-205: Secret printed to console"),
    ],
    "LOW": [
        (r"queue_free\s*\(\s*\)", "GDSEC-301: Direct queue_free without safety check"),
        (r"#\s*TODO", "GDSEC-302: TODO comment - check if implemented"),
        (r"#\s*FIXME", "GDSEC-303: FIXME comment - known issue"),
        (r"#\s*HACK", "GDSEC-304: HACK comment - temporary workaround"),
    ],
}

def scan_gdscript_files(directory):
    results = {
        "path": directory,
        "timestamp": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "files_scanned": 0,
        "lines_scanned": 0,
        "findings_count": 0,
        "by_severity": {"CRITICAL": 0, "HIGH": 0, "MEDIUM": 0, "LOW": 0},
        "findings": [],
    }

    for root, dirs, files in os.walk(directory):
        for file in files:
            if not file.endswith(".gd"):
                continue

            file_path = os.path.join(root, file)
            try:
                with open(file_path, 'r', encoding='utf-8') as f:
                    lines = f.readlines()
            except:
                continue

            results["files_scanned"] += 1
            results["lines_scanned"] += len(lines)

            for i, line in enumerate(lines, 1):
                for severity, patterns in VULN_PATTERNS.items():
                    for pattern, message in patterns:
                        if re.search(pattern, line, re.IGNORECASE):
                            results["findings_count"] += 1
                            results["by_severity"][severity] += 1
                            results["findings"].append({
                                "file": file_path,
                                "line": i,
                                "severity": severity,
                                "message": message,
                                "code": line.strip()[:120],
                            })

    return results

def main():
    if len(sys.argv) < 2:
        print("Usage: python gdscript_scanner.py <directory> [--json]")
        sys.exit(1)

    directory = sys.argv[1]
    use_json = "--json" in sys.argv

    results = scan_gdscript_files(directory)

    if use_json:
        print(json.dumps(results, indent=2, ensure_ascii=False))
    else:
        print(f"\n{'='*70}")
        print(f"  GODOT GDScript Security Scanner")
        print(f"  Target: {directory}")
        print(f"  Time: {results['timestamp']}")
        print(f"{'='*70}")
        print(f"  Files: {results['files_scanned']} | Lines: {results['lines_scanned']}")
        print(f"  Findings: {results['findings_count']}")
        print(f"  Critical: {results['by_severity']['CRITICAL']} | High: {results['by_severity']['HIGH']}")
        print(f"  Medium: {results['by_severity']['MEDIUM']} | Low: {results['by_severity']['LOW']}")
        print(f"{'='*70}")

        for finding in results["findings"]:
            prefix = {"CRITICAL": "🚨", "HIGH": "⚠️", "MEDIUM": "⚡", "LOW": "💡"}.get(finding["severity"], "?")
            print(f"\n{prefix} [{finding['severity']}] {finding['message']}")
            print(f"   File: {finding['file']}:{finding['line']}")
            print(f"   Code: {finding['code']}")

        print(f"\n{'='*70}")

if __name__ == "__main__":
    main()