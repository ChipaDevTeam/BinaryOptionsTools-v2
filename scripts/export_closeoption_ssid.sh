#!/bin/bash
#
# CloseOption SSID Export Script
#
# Extracts and exports CloseOption session credentials.
#
# Usage:
#   ./scripts/export_closeoption_ssid.sh [--browser chrome] [--save]
#
# Environment:
#   CLOSEOPTION_SSID  - Session ID in format: token|sid|demo|public_code|hidden_code

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default browser
BROWSER="chrome"
SAVE_SESSION=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --browser|-b)
            BROWSER="$2"
            shift 2
            ;;
        --save|-s)
            SAVE_SESSION=true
            shift
            ;;
        --help|-h)
            echo "CloseOption SSID Export Script"
            echo ""
            echo "Usage:"
            echo "  ./scripts/export_closeoption_ssid.sh [--browser chrome] [--save]"
            echo ""
            echo "Environment:"
            echo "  CLOSEOPTION_SSID  - Session ID in format: token|sid|demo|public_code|hidden_code"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Detect platform
detect_platform() {
    case "$OSTYPE" in
        linux*) echo "linux" ;;
        darwin*) echo "mac" ;;
        msys*|cygwin*) echo "windows" ;;
        *) echo "unknown" ;;
    esac
}

PLATFORM=$(detect_platform)

# Get browser path
get_browser_path() {
    local browser=$1
    local home="$HOME"
    
    case "$browser" in
        chrome)
            case "$PLATFORM" in
                windows) echo "$home/AppData/Local/Google/Chrome/User Data" ;;
                mac) echo "$home/Library/Application Support/Google/Chrome" ;;
                linux) echo "$home/.config/chrome" ;;
            esac
            ;;
        firefox)
            case "$PLATFORM" in
                windows) echo "$home/AppData/Roaming/Mozilla/Firefox/Profiles" ;;
                mac) echo "$home/Library/Application Support/Firefox/Profiles" ;;
                linux) echo "$home/.mozilla/firefox" ;;
            esac
            ;;
        edge)
            case "$PLATFORM" in
                windows) echo "$home/AppData/Local/Microsoft/Edge/User Data" ;;
                mac) echo "$home/Library/Application Support/Microsoft Edge" ;;
                linux) echo "$home/.config/microsoft-edge" ;;
            esac
            ;;
        brave)
            case "$PLATFORM" in
                windows) echo "$home/AppData/Local/BraveSoftware/Brave-Browser/User Data" ;;
                mac) echo "$home/Library/Application Support/BraveSoftware/Brave-Browser" ;;
                linux) echo "$home/.config/brave-browser" ;;
            esac
            ;;
    esac
}

# Check if session file exists
check_saved_session() {
    local session_file="$HOME/.closeoption_session.json"
    
    if [[ -f "$session_file" ]]; then
        echo -e "${GREEN}Found saved session: $session_file${NC}"
        return 0
    fi
    return 1
}

# Show manual extraction instructions
show_instructions() {
    cat << 'EOF'

=== CloseOption SSID Export Instructions ===

Method 1: Using Browser DevTools
---------------------------------
1. Open https://www.closeoption.com in your browser
2. Press F12 to open Developer Tools
3. Go to Application (Chrome) or Storage (Firefox) tab
4. Expand Cookies and find closeoption.com
5. Look for these values:
   - token: Your authentication token
   - sid: Session ID from Socket.IO
   - publicCode: Public asset code
   - hiddenCode: Hidden asset code
   - isDemo: Boolean (true/false) for demo account

Method 2: Using Network Tab
----------------------------
1. Open CloseOption in browser
2. Press F12 → Network tab
3. Refresh page (Cmd/Ctrl + R)
4. Filter by WS (WebSocket)
5. Click on any WebSocket connection
6. Check Request Headers for authorization

SSID Format:
------------
token|sid|demo|public_code|hidden_code

Example:
abc123token|xyz789sid|true|pub_code123|hid_code456

EOF
}

# Main function
main() {
    echo -e "${YELLOW}CloseOption SSID Export Tool${NC}"
    echo "Platform: $PLATFORM"
    echo "Browser: $BROWSER"
    echo ""
    
    # Check for saved session
    if check_saved_session; then
        echo ""
        echo -e "${GREEN}To export:${NC}"
        if [[ "$PLATFORM" == "windows" ]]; then
            SSID=$(python3 -c "import json; d=json.load(open('$HOME/.closeoption_session.json')); print(f\"{d['token']}|{d['sid']}|{str(d['demo']).lower()}|{d['public_code']}|{d['hidden_code']}\")" 2>/dev/null || echo 'LOAD_ERROR')
            echo "  set CLOSEOPTION_SSID=$SSID"
        else
            SSID=$(python3 -c "import json; d=json.load(open('$HOME/.closeoption_session.json')); print(f\"{d['token']}|{d['sid']}|{str(d['demo']).lower()}|{d['public_code']}|{d['hidden_code']}\")" 2>/dev/null || echo 'LOAD_ERROR')
            echo "  export CLOSEOPTION_SSID='$SSID'"
        fi
        return 0
    fi
    
    # Check browser path
    BROWSER_PATH=$(get_browser_path "$BROWSER")
    if [[ -n "$BROWSER_PATH" && -d "$BROWSER_PATH" ]]; then
        echo -e "${YELLOW}Browser found: $BROWSER_PATH${NC}"
        echo ""
        echo -e "${YELLOW}Note:${NC} Direct browser extraction requires Python script with DPAPI support."
        echo "Please use the Python script instead:"
        echo ""
        echo "  python3 $SCRIPT_DIR/export_closeoption_ssid.py --browser $BROWSER"
        echo ""
    else
        echo -e "${YELLOW}Browser not found at expected location${NC}"
    fi
    
    echo ""
    show_instructions
    
    # Offer to save if user provides SSID
    if [[ "$SAVE_SESSION" == true ]]; then
        echo -e "${YELLOW}Enter SSID (token|sid|demo|public_code|hidden_code):${NC}"
        read -r SSID
        
        if [[ -n "$SSID" ]]; then
            # Create directory if needed
            mkdir -p "$HOME/.closeoption_tools" 2>/dev/null || true
            
            # Parse and save
            TOKEN=$(echo "$SSID" | cut -d'|' -f1)
            SID=$(echo "$SSID" | cut -d'|' -f2)
            DEMO=$(echo "$SSID" | cut -d'|' -f3)
            PUB=$(echo "$SSID" | cut -d'|' -f4)
            HID=$(echo "$SSID" | cut -d'|' -f5)
            
            umask 077
            cat > "$HOME/.closeoption_session.json" << EOF
{
  "token": "$TOKEN",
  "sid": "$SID",
  "demo": $DEMO,
  "public_code": "$PUB",
  "hidden_code": "$HID"
}
EOF
            chmod 600 "$HOME/.closeoption_session.json"
            
            echo -e "${GREEN}Session saved to: $HOME/.closeoption_session.json${NC}"
        fi
    fi

main "$@"
