"""
Wasteland Project Test Runner
Run all test suites to verify the complete system
"""
import subprocess
import sys
import os
import json
import time

def run_test_script(script_name):
    """Run a test script and capture results."""
    script_path = os.path.join(os.path.dirname(__file__), script_name)
    
    if not os.path.exists(script_path):
        return {"script": script_name, "status": "NOT_FOUND", "output": "", "time": 0}
    
    start_time = time.time()
    result = subprocess.run(
        [sys.executable, script_path],
        capture_output=True,
        text=True,
        timeout=120
    )
    elapsed = time.time() - start_time
    
    return {
        "script": script_name,
        "status": "PASS" if result.returncode == 0 else "FAIL",
        "output": result.stdout + result.stderr,
        "time": round(elapsed, 2),
        "return_code": result.returncode
    }

def run_blender_verification():
    """Run Blender headless verification."""
    print("[RUNNING] Blender Plugin Verification")
    
    blender_installs = [
        r"E:\SteamLibrary\steamapps\common\Blender\blender.exe",
        r"C:\Program Files\Blender Foundation\Blender\blender.exe",
        r"E:\C盘迁移文件\游戏\建模与游戏引擎\New Folder\blender.exe",
    ]
    
    blender_exe = None
    for path in blender_installs:
        if os.path.exists(path):
            blender_exe = path
            break
    
    if not blender_exe:
        return {
            "script": "Blender Verification",
            "status": "SKIP",
            "output": "Blender not found",
            "time": 0
        }
    
    plugin_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "blender_plugin")
    verify_script = os.path.join(plugin_dir, "verify_wasteland.py")
    
    start_time = time.time()
    result = subprocess.run(
        [blender_exe, "--background", "--python", verify_script],
        capture_output=True,
        text=True,
        timeout=180
    )
    elapsed = time.time() - start_time
    
    return {
        "script": "Blender Verification",
        "status": "PASS" if result.returncode == 0 else "FAIL",
        "output": result.stdout + result.stderr,
        "time": round(elapsed, 2),
        "return_code": result.returncode
    }

def main():
    print("=" * 70)
    print("WASTELAND PROJECT COMPREHENSIVE TEST SUITE")
    print("=" * 70)
    print(f"Timestamp: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 70)
    
    test_results = []
    
    print("\n[1/3] Running Rust Compilation Tests...")
    test_results.append(run_test_script("test_rust_compilation.py"))
    
    print("\n[2/3] Running Blender Plugin Verification...")
    test_results.append(run_blender_verification())
    
    print("\n[3/3] Running System Integration Tests...")
    test_results.append(run_test_script("test_integration.py"))
    
    print("\n" + "=" * 70)
    print("TEST SUMMARY")
    print("=" * 70)
    
    passed = 0
    failed = 0
    skipped = 0
    total_time = 0
    
    for result in test_results:
        status = result["status"]
        if status == "PASS":
            passed += 1
        elif status == "FAIL":
            failed += 1
        else:
            skipped += 1
        total_time += result.get("time", 0)
    
    print(f"\nResults: {passed} passed, {failed} failed, {skipped} skipped")
    print(f"Total time: {total_time:.2f} seconds")
    
    print("\nDetailed Results:")
    for i, result in enumerate(test_results, 1):
        print(f"\n{i}. {result['script']}")
        print(f"   Status: {result['status']}")
        if "time" in result and result["time"] > 0:
            print(f"   Time: {result['time']:.2f}s")
        if result["status"] == "FAIL":
            output = result["output"]
            print(f"   Output:\n{output[-1500:]}" if len(output) > 1500 else f"   Output:\n{output}")
    
    print("\n" + "=" * 70)
    
    report = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "total_tests": len(test_results),
        "passed": passed,
        "failed": failed,
        "skipped": skipped,
        "total_time": total_time,
        "results": test_results
    }
    
    report_path = os.path.join(os.path.dirname(__file__), "test_report.json")
    with open(report_path, 'w') as f:
        json.dump(report, f, indent=2)
    print(f"\nReport saved to: {report_path}")
    
    print("\n" + "=" * 70)
    if failed == 0:
        print("ALL TESTS PASSED!")
        print("=" * 70)
        return 0
    else:
        print(f"FAILED: {failed} test(s)")
        print("=" * 70)
        return 1

if __name__ == "__main__":
    sys.exit(main())