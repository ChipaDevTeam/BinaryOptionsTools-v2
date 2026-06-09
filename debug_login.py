import requests, re

s = requests.Session()
s.headers.update({
    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36',
    'Accept-Language': 'en-GB,en;q=0.9',
    'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
})
r = s.get('https://pocketoption.com/en/login/', timeout=30)
html = r.text

# Find recaptcha sitekey
for m in re.finditer(r'(recaptcha|sitekey|grecaptcha|data-sitekey)[^"\'<>]{0,200}', html, re.IGNORECASE):
    print(m.group())

print()
# Also look for script tags referencing recaptcha
for m in re.finditer(r'<script[^>]{0,200}recaptcha[^>]{0,200}>', html, re.IGNORECASE):
    print(m.group())

# Find sitekey pattern directly
for m in re.finditer(r'["\']([0-9A-Za-z_-]{30,50})["\']', html):
    print("POTENTIAL KEY:", m.group(1))
