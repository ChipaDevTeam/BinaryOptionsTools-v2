"""Test if pocketoption blocks headless browser at network level vs app level."""
import socket, requests

# Basic TCP connectivity check
host = "pocketoption.com"
try:
    ip = socket.gethostbyname(host)
    print(f"DNS resolves to: {ip}")
    s = socket.create_connection((ip, 443), timeout=10)
    s.close()
    print("TCP port 443: OPEN")
except Exception as e:
    print(f"TCP check failed: {e}")

# requests works?
try:
    r = requests.get("https://pocketoption.com/en/login/", timeout=15,
                     headers={"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36"})
    print(f"requests GET status: {r.status_code}, length: {len(r.text)}")
except Exception as e:
    print(f"requests failed: {e}")
