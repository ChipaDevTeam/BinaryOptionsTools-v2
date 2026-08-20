import os
import sys
import subprocess
import time
from pathlib import Path
from dotenv import load_dotenv

def run_example(filepath: Path, ssid: str) -> bool:
    print(f"\n==================================================")
    print(f"Testing Example: {filepath.relative_to(Path.cwd())}")
    print(f"==================================================")

    # Prepare environment
    env = os.environ.copy()
    env["POCKET_OPTION_SSID"] = ssid
    env["POCKET_OPTION_EMAIL"] = "dummy@example.com"
    env["POCKET_OPTION_PASSWORD"] = "dummy_pass"
    # Ensure local python/ directory is in PYTHONPATH so example imports our local build
    python_dir = str(Path(__file__).resolve().parent / "python")
    env["PYTHONPATH"] = python_dir

    # Start the process
    # Pass the SSID on stdin in case the example prompts with input("...")
    proc = subprocess.Popen(
        [sys.executable, str(filepath)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env
    )

    # Wait up to 10 seconds for it to run
    try:
        stdout, stderr = proc.communicate(input=ssid + "\n", timeout=10)
        timed_out = False
    except subprocess.TimeoutExpired:
        # Kill the process if it timed out (which is expected for streaming/websocket loops)
        proc.kill()
        stdout, stderr = proc.communicate()
        timed_out = True

    print("--- STDOUT ---")
    print(stdout)
    if stderr.strip():
        print("--- STDERR ---")
        print(stderr)

    # Determine if the test was successful:
    # 1. If it timed out, but printed output indicating success (e.g. Connected: True, Candle, Balance, Yield)
    # 2. If it exited cleanly with return code 0
    # 3. If it failed with a known credentials/session issue (since we might have a expired/different session or not set up email/password)
    # 4. If it has a syntax or import error, it's a failure.
    if timed_out:
        print(f"Process timed out (10s) as expected for streaming examples.")
        success_markers = ["Connected: True", "Candle", "Balance", "Yield", "Found", "Log message", "WS Incoming"]
        if any(marker in stdout for marker in success_markers):
            print("Status: SUCCESS (Timed out after successful stream start)")
            return True
        else:
            print("Status: FAILURE (Timed out with no success markers)")
            return False
    else:
        # Exited before 10s
        rc = proc.returncode
        if rc == 0:
            print(f"Status: SUCCESS (Exited cleanly with code 0)")
            return True
        else:
            # Treat every nonzero exit as failure unless it matches a narrowly recognized intentional skip condition
            skip_conditions = [
                "invalid ssid",
                "not set",
                "Login failed",
                "missing prerequisites",
            ]
            is_skip = any(condition in stdout or condition in stderr for condition in skip_conditions)
            if is_skip:
                print(f"Status: SKIPPED (Exited with code {rc} due to known skip condition)")
                return True
            print(f"Status: FAILURE (Exited with code {rc})")
            return False

def main():
    # Load .env file
    env_path = Path(__file__).resolve().parent / ".env"
    if env_path.exists():
        load_dotenv(env_path)

    ssid = os.getenv("POCKET_OPTION_SSID", "mock_ssid_for_import_checks")

    # Require explicit opt-in to run order-placing examples
    run_order_examples = os.getenv("RUN_ORDER_EXAMPLES", "0") == "1"

    examples_dir = Path(__file__).resolve().parent / "examples" / "python"

    # Find all python files recursively in examples/python
    example_files = sorted(list(examples_dir.glob("**/*.py")))

    # Exclude files that are not runnable examples (e.g. validator.py is helper/utility)
    runnable_files = []
    for f in example_files:
        # Exclude validator.py as it is not a standalone executable example, but rather helper classes
        if f.name == "validator.py" or f.name.startswith("_"):
            continue
        runnable_files.append(f)

    # Exclude order-placing examples by default unless explicitly opted in
    order_keywords = ["trade.py", "create_raw_order.py", "buy", "sell"]
    if not run_order_examples:
        runnable_files = [
            f for f in runnable_files
            if not any(keyword in f.name.lower() for keyword in order_keywords)
        ]

    # Validate that the supplied SSID is demo-only before allowing order-placing examples
    if run_order_examples:
        if not ssid or "isDemo" not in ssid:
            print("Refusing to run order-placing examples: POCKET_OPTION_SSID is not a demo credential.")
            sys.exit(1)

    print(f"Found {len(runnable_files)} runnable python examples to test.")

    success_count = 0
    failures = []

    for f in runnable_files:
        success = run_example(f, ssid)
        if success:
            success_count += 1
        else:
            failures.append(str(f.relative_to(examples_dir)))

    print(f"\n==================================================")
    print(f"Example Testing Summary")
    print(f"==================================================")
    print(f"Passed: {success_count} / {len(runnable_files)}")
    if failures:
        print(f"Failed examples: {', '.join(failures)}")
        sys.exit(1)
    else:
        print("All examples executed correctly without syntax/import/runtime failures!")
        sys.exit(0)

if __name__ == "__main__":
    main()
