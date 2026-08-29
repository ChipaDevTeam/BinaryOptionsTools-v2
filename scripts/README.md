# CloseOption SSID Export Scripts

This directory contains scripts to help extract and manage CloseOption session credentials.

## Overview

CloseOption requires a Session ID (SSID) in the format:
```
token|sid|demo|public_code|hidden_code
```

These scripts help you:
1. Extract credentials from your browser
2. Save sessions for reuse
3. Export as environment variables

## Scripts

### Python Script (Cross-Platform)
```bash
# Show help
python scripts/export_closeoption_ssid.py --help

# Input SSID directly
python scripts/export_closeoption_ssid.py --input "token|sid|true|pub|hid"

# Print manual extraction instructions
python scripts/export_closeoption_ssid.py --manual

# Read from saved JSON file
python scripts/export_closeoption_ssid.py --file ~/.closeoption_session.json

# Save extracted SSID
python scripts/export_closeoption_ssid.py --input "token|sid|true|pub|hid" --save
```

### Bash Script (Linux/macOS)
```bash
# Show help
./scripts/export_closeoption_ssid.sh --help

# Check for saved session
./scripts/export_closeoption_ssid.sh

# Save new session (interactive)
./scripts/export_closeoption_ssid.sh --save
```

### Batch Script (Windows)
```cmd
REM Show help
scripts\export_closeoption_ssid.bat --help

REM Check for saved session
scripts\export_closeoption_ssid.bat

REM Save new session (interactive)
scripts\export_closeoption_ssid.bat --save
```

## SSID Format

The SSID contains 5 pipe-delimited components:

| Component | Description | Example |
|-----------|-------------|---------|
| `token` | Authentication token | `abc123xyz` |
| `sid` | Socket.IO session ID | `4f8a9b2c1d` |
| `demo` | Account type (`true`/`false`) | `true` |
| `public_code` | Public asset code | `pub_12345` |
| `hidden_code` | Hidden asset code | `hid_67890` |

**Example:**
```
abc123token|4f8a9b2c1d|true|pub_12345|hig_67890
```

## Manual Extraction Steps

### Method 1: Browser DevTools

1. Open [CloseOption](https://www.closeoption.com) in your browser
2. Press `F12` to open Developer Tools
3. Go to **Application** (Chrome) or **Storage** (Firefox) tab
4. Expand **Cookies** and select `closeoption.com`
5. Look for these values:
   - `token` - Authentication token
   - `sid` - Session ID
   - `publicCode` - Public asset code
   - `hiddenCode` - Hidden asset code
   - `isDemo` - Boolean (true/false)

### Method 2: Network Tab

1. Open CloseOption in browser
2. Press `F12` → Network tab
3. Refresh page (`Ctrl+R` or `Cmd+R`)
4. Filter by **WS** (WebSocket)
5. Click on any WebSocket connection
6. Check **Request Headers** for authorization tokens

### Method 3: Console Commands

Open browser console (`F12` → Console) and run:

```javascript
// Get all cookies
document.cookie

// Check localStorage
localStorage.getItem('session')

// Check sessionStorage
sessionStorage.getItem('session')
```

## Saving Sessions

Sessions can be saved to `~/.closeoption_session.json`:

```json
{
  "token": "abc123token",
  "sid": "4f8a9b2c1d",
  "demo": true,
  "public_code": "pub_12345",
  "hidden_code": "hid_67890"
}
```

## Environment Variables

After extracting your SSID, export it:

### Linux/macOS
```bash
export CLOSEOPTION_SSID="token|sid|true|public_code|hidden_code"
```

### Windows (Command Prompt)
```cmd
set CLOSEOPTION_SSID=token|sid|true|public_code|hidden_code
```

### Windows (PowerShell)
```powershell
$env:CLOSEOPTION_SSID="token|sid|true|public_code|hidden_code"
```

## Using with Examples

```bash
# Set environment variable
export CLOSEOPTION_SSID="your_token|your_sid|true|pub|hid"

# Run example
python examples/async/closeoption_basic.py
```

## Security Notes

- **Never commit** session credentials to version control
- Session tokens expire and should be refreshed regularly
- Use demo account (`demo=true`) for testing when possible
- Keep your `~/.closeoption_session.json` file secure

## Troubleshooting

### "SSID not found" error
- Ensure you've exported the environment variable
- Check format: `token|sid|demo|public_code|hidden_code`
- Verify all 5 components are present

### "Connection failed" error
- Session may have expired
- Re-extract credentials from browser
- Check if you're using demo or real account credentials

### "Invalid token" error
- Token may be incorrect or expired
- Re-login to CloseOption website
- Extract fresh credentials using the scripts
