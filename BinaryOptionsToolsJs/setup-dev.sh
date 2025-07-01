#!/bin/bash

# Script to set up platform packages for local development
# This installs the platform packages locally so they can be found by require()

echo "Setting up platform packages for local development..."

cd /home/runner/work/BinaryOptionsTools-v2/BinaryOptionsTools-v2/BinaryOptionsToolsJs

# Install all platform packages that exist
for pkg_dir in npm/*/; do
    if [ -f "$pkg_dir/package.json" ]; then
        pkg_name=$(basename "$pkg_dir")
        echo "Installing platform package: $pkg_name"
        npm install "./$pkg_dir" --save-optional 2>/dev/null || echo "Failed to install $pkg_name (this may be normal if the binary doesn't exist)"
    fi
done

echo "Platform packages setup complete!"
echo ""
echo "Note: For full functionality, you need to:"
echo "1. Build the binaries for your target platforms: npm run build"
echo "2. Copy the built binaries to the respective npm/<platform>/ directories"
echo "3. Re-run this script to install packages with binaries"