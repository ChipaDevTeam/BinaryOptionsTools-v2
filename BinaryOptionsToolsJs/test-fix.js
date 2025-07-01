#!/usr/bin/env node

// Comprehensive test to validate the BinaryToolsJS loading fix
console.log('🧪 Testing BinaryToolsJS Loading Fix');
console.log('=====================================\n');

const fs = require('fs');
const path = require('path');

console.log(`Platform: ${process.platform}`);
console.log(`Architecture: ${process.arch}`);

// Test 1: Check if local binary exists
const expectedBinary = (() => {
    const { platform, arch } = process;
    if (platform === 'win32') {
        if (arch === 'x64') return 'binary-options-tools.win32-x64-msvc.node';
        if (arch === 'ia32') return 'binary-options-tools.win32-ia32-msvc.node';
        if (arch === 'arm64') return 'binary-options-tools.win32-arm64-msvc.node';
    } else if (platform === 'darwin') {
        if (arch === 'x64') return 'binary-options-tools.darwin-x64.node';
        if (arch === 'arm64') return 'binary-options-tools.darwin-arm64.node';
        return 'binary-options-tools.darwin-universal.node';
    } else if (platform === 'linux') {
        if (arch === 'x64') return 'binary-options-tools.linux-x64-gnu.node';
        if (arch === 'arm64') return 'binary-options-tools.linux-arm64-gnu.node';
        if (arch === 'arm') return 'binary-options-tools.linux-arm-gnueabihf.node';
    }
    return null;
})();

console.log(`Expected binary: ${expectedBinary}`);

const localBinaryPath = path.join(__dirname, expectedBinary || 'unknown');
const localBinaryExists = fs.existsSync(localBinaryPath);

console.log(`Local binary exists: ${localBinaryExists ? '✅' : '❌'} (${localBinaryPath})`);

// Test 2: Check expected platform package
const expectedPlatformPackage = (() => {
    const { platform, arch } = process;
    if (platform === 'win32') {
        if (arch === 'x64') return '@rick-29/binary-options-tools-win32-x64-msvc';
        if (arch === 'ia32') return '@rick-29/binary-options-tools-win32-ia32-msvc';
        if (arch === 'arm64') return '@rick-29/binary-options-tools-win32-arm64-msvc';
    } else if (platform === 'darwin') {
        if (arch === 'x64') return '@rick-29/binary-options-tools-darwin-x64';
        if (arch === 'arm64') return '@rick-29/binary-options-tools-darwin-arm64';
        return '@rick-29/binary-options-tools-darwin-universal';
    } else if (platform === 'linux') {
        if (arch === 'x64') return '@rick-29/binary-options-tools-linux-x64-gnu';
        if (arch === 'arm64') return '@rick-29/binary-options-tools-linux-arm64-gnu';
        if (arch === 'arm') return '@rick-29/binary-options-tools-linux-arm-gnueabihf';
    }
    return null;
})();

console.log(`Expected platform package: ${expectedPlatformPackage}`);

// Test 3: Check if platform package is installed
let platformPackageInstalled = false;
if (expectedPlatformPackage) {
    try {
        require.resolve(expectedPlatformPackage);
        platformPackageInstalled = true;
    } catch (e) {
        // Not installed
    }
}

console.log(`Platform package installed: ${platformPackageInstalled ? '✅' : '❌'}`);

// Test 4: Try loading the main library
console.log('\n📦 Testing main library loading...');

let loadingResult = null;
let loadingError = null;

try {
    const lib = require('./index.js');
    loadingResult = {
        success: true,
        exports: Object.keys(lib),
        exportCount: Object.keys(lib).length
    };
} catch (error) {
    loadingError = error;
    loadingResult = {
        success: false,
        error: error.message,
        code: error.code
    };
}

if (loadingResult.success) {
    console.log('✅ Library loaded successfully!');
    console.log(`   Exports (${loadingResult.exportCount}): ${loadingResult.exports.join(', ')}`);
} else {
    console.log('❌ Library loading failed');
    console.log(`   Error: ${loadingResult.error}`);
    if (loadingError && loadingError.code) {
        console.log(`   Code: ${loadingResult.code}`);
    }
}

// Test 5: Summary and recommendations
console.log('\n📋 Summary and Recommendations');
console.log('================================');

if (loadingResult.success) {
    console.log('🎉 SUCCESS: The BinaryToolsJS loading issue has been resolved!');
    
    if (localBinaryExists) {
        console.log('✅ Using local binary (development mode)');
    } else if (platformPackageInstalled) {
        console.log('✅ Using platform package (production mode)');
    }
} else {
    console.log('⚠️  ISSUE: Library failed to load. Here\'s how to fix it:');
    
    if (!localBinaryExists && !platformPackageInstalled) {
        console.log('\n🔧 Recommended fixes:');
        console.log('   1. Build the binary: npm run build');
        console.log('   2. Set up development environment: npm run setup-dev');
        console.log('   3. Install the library with npm install');
    } else if (!platformPackageInstalled) {
        console.log('\n🔧 Recommended fix:');
        console.log('   Run: npm run setup-dev');
    }
    
    if (loadingError && loadingError.platform && loadingError.arch) {
        console.log(`\n📋 Platform Info: ${loadingError.platform}-${loadingError.arch}`);
    }
}

console.log('\n📚 For more help, see the Troubleshooting section in README.md');