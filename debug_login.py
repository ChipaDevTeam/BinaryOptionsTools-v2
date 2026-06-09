"""Test connectivity to both PO domains and attempt login via po.trade"""
import re
import requests

UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36"

for url in ["https://po.trade/en/login/", "https://pocketoption.com/en/login/"]:
    try:
        r = requests.get(url, headers={"User-Agent": UA}, timeout=15)
        print(f"GET {url} -> {r.status_code}, len={len(r.text)}")
        # print input fields
        for m in re.finditer(r'<input[^>]{0,200}>', r.text, re.IGNORECASE):
            print(" ", m.group()[:100])
    except Exception as e:
        print(f"GET {url} -> FAILED: {e}")
    print()
