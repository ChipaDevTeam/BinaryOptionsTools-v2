// Code Highlighting and Interactive Features for BinaryOptionsToolsV2 Documentation
class CodeHighlighter {
    constructor() {
        this.languages = {
            'python': {
                keywords: ['def', 'class', 'import', 'from', 'if', 'else', 'elif', 'for', 'while', 'try', 'except', 'finally', 'with', 'as', 'return', 'yield', 'lambda', 'and', 'or', 'not', 'in', 'is', 'True', 'False', 'None', 'async', 'await'],
                strings: [/'([^'\\]|\\.)*'/, /"([^"\\]|\\.)*"/, /'''[\s\S]*?'''/, /"""[\s\S]*?"""/],
                comments: [/#.*/],
                numbers: [/\b\d+\.?\d*\b/]
            },
            'javascript': {
                keywords: ['function', 'const', 'let', 'var', 'if', 'else', 'for', 'while', 'do', 'switch', 'case', 'default', 'break', 'continue', 'return', 'try', 'catch', 'finally', 'throw', 'new', 'this', 'class', 'extends', 'import', 'export', 'from', 'async', 'await', 'true', 'false', 'null', 'undefined'],
                strings: [/'([^'\\]|\\.)*'/, /"([^"\\]|\\.)*"/, /`([^`\\]|\\.)*`/],
                comments: [/\/\/.*/, /\/\*[\s\S]*?\*\//],
                numbers: [/\b\d+\.?\d*\b/]
            },
            'rust': {
                keywords: ['fn', 'let', 'mut', 'const', 'static', 'if', 'else', 'match', 'for', 'while', 'loop', 'break', 'continue', 'return', 'struct', 'enum', 'impl', 'trait', 'mod', 'use', 'pub', 'crate', 'self', 'super', 'async', 'await', 'true', 'false', 'Some', 'None', 'Ok', 'Err'],
                strings: [/"([^"\\]|\\.)*"/, /r#".*?"#/, /r".*?"/],
                comments: [/\/\/.*/, /\/\*[\s\S]*?\*\//],
                numbers: [/\b\d+\.?\d*\b/]
            }
        };
        this.init();
    }

    init() {
        this.highlightCode();
        this.setupCopyButtons();
        this.setupLineNumbers();
        this.setupCodeTabs();
        this.setupCodePlayground();
        this.setupSearchInCode();
    }

    // Syntax highlighting
    highlightCode() {
        document.querySelectorAll('pre code[class*="language-"]').forEach(block => {
            const language = this.getLanguageFromClass(block.className);
            if (this.languages[language]) {
                this.applyHighlighting(block, language);
            }
        });
    }

    getLanguageFromClass(className) {
        const match = className.match(/language-(\w+)/);
        return match ? match[1] : 'javascript';
    }

    applyHighlighting(block, language) {
        let code = block.textContent;
        const lang = this.languages[language];

        // Apply highlighting in order: comments, strings, keywords, numbers
        
        // Comments (preserve existing structure)
        lang.comments.forEach(regex => {
            code = code.replace(regex, '<span class="comment">$&</span>');
        });

        // Strings
        lang.strings.forEach(regex => {
            code = code.replace(regex, '<span class="string">$&</span>');
        });

        // Keywords
        lang.keywords.forEach(keyword => {
            const regex = new RegExp(`\\b${keyword}\\b`, 'g');
            code = code.replace(regex, `<span class="keyword">${keyword}</span>`);
        });

        // Numbers
        lang.numbers.forEach(regex => {
            code = code.replace(regex, '<span class="number">$&</span>');
        });

        // Functions (language-specific patterns)
        if (language === 'python') {
            code = code.replace(/\b(\w+)(?=\s*\()/g, '<span class="function">$1</span>');
        } else if (language === 'javascript') {
            code = code.replace(/\b(\w+)(?=\s*[=:]\s*(?:function|\(|async))/g, '<span class="function">$1</span>');
            code = code.replace(/\b(\w+)(?=\s*\()/g, '<span class="function">$1</span>');
        } else if (language === 'rust') {
            code = code.replace(/\bfn\s+(\w+)/g, 'fn <span class="function">$1</span>');
        }

        block.innerHTML = code;
    }

    // Copy to clipboard functionality
    setupCopyButtons() {
        document.querySelectorAll('pre').forEach(pre => {
            if (pre.querySelector('.copy-btn')) return; // Already has copy button

            const button = document.createElement('button');
            button.className = 'copy-btn';
            button.innerHTML = `
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                </svg>
                <span>Copy</span>
            `;

            pre.style.position = 'relative';
            pre.appendChild(button);

            button.addEventListener('click', async (e) => {
                e.preventDefault();
                const code = pre.querySelector('code').textContent;
                
                try {
                    await navigator.clipboard.writeText(code);
                    this.showCopySuccess(button);
                } catch (err) {
                    this.showCopyError(button);
                }
            });
        });
    }

    showCopySuccess(button) {
        const originalContent = button.innerHTML;
        button.innerHTML = `
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M20 6L9 17l-5-5"></path>
            </svg>
            <span>Copied!</span>
        `;
        button.classList.add('copied');

        setTimeout(() => {
            button.innerHTML = originalContent;
            button.classList.remove('copied');
        }, 2000);
    }

    showCopyError(button) {
        const originalContent = button.innerHTML;
        button.innerHTML = `
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="15" y1="9" x2="9" y2="15"></line>
                <line x1="9" y1="9" x2="15" y2="15"></line>
            </svg>
            <span>Failed</span>
        `;
        button.classList.add('error');

        setTimeout(() => {
            button.innerHTML = originalContent;
            button.classList.remove('error');
        }, 2000);
    }

    // Line numbers for code blocks
    setupLineNumbers() {
        document.querySelectorAll('pre code').forEach(block => {
            const pre = block.parentElement;
            if (pre.classList.contains('line-numbers')) {
                this.addLineNumbers(pre, block);
            }
        });
    }

    addLineNumbers(pre, code) {
        const lines = code.textContent.split('\n');
        const lineNumbersDiv = document.createElement('div');
        lineNumbersDiv.className = 'line-numbers-rows';
        
        lines.forEach((_, index) => {
            const span = document.createElement('span');
            span.textContent = index + 1;
            lineNumbersDiv.appendChild(span);
        });

        pre.appendChild(lineNumbersDiv);
        pre.classList.add('has-line-numbers');
    }

    // Code tabs for multi-language examples
    setupCodeTabs() {
        document.querySelectorAll('.code-tabs').forEach(container => {
            const tabs = container.querySelectorAll('.tab-button');
            const contents = container.querySelectorAll('.tab-content');

            tabs.forEach(tab => {
                tab.addEventListener('click', () => {
                    const target = tab.dataset.tab;
                    
                    // Remove active classes
                    tabs.forEach(t => t.classList.remove('active'));
                    contents.forEach(c => {
                        c.classList.remove('active');
                        c.style.display = 'none';
                    });

                    // Add active classes
                    tab.classList.add('active');
                    const targetContent = container.querySelector(`[data-tab-content="${target}"]`);
                    if (targetContent) {
                        targetContent.classList.add('active');
                        targetContent.style.display = 'block';
                        
                        // Re-highlight code in the new tab
                        targetContent.querySelectorAll('code[class*="language-"]').forEach(block => {
                            const language = this.getLanguageFromClass(block.className);
                            if (this.languages[language]) {
                                this.applyHighlighting(block, language);
                            }
                        });
                    }
                });
            });

            // Initialize first tab
            if (tabs.length > 0) {
                tabs[0].click();
            }
        });
    }

    // Interactive code playground
    setupCodePlayground() {
        document.querySelectorAll('.code-playground').forEach(playground => {
            const editor = playground.querySelector('.editor');
            const output = playground.querySelector('.output');
            const runButton = playground.querySelector('.run-code');
            
            if (editor && runButton) {
                runButton.addEventListener('click', () => {
                    this.runCode(editor.textContent, output);
                });

                // Make editor editable
                editor.contentEditable = true;
                editor.addEventListener('input', () => {
                    this.highlightEditableCode(editor);
                });
            }
        });
    }

    runCode(code, output) {
        // This is a simplified code runner - in production you'd use a proper sandbox
        try {
            // Only for demonstration - don't run arbitrary code in production
            const result = eval(code);
            output.innerHTML = `<div class="output-success">Output: ${result}</div>`;
        } catch (error) {
            output.innerHTML = `<div class="output-error">Error: ${error.message}</div>`;
        }
    }

    highlightEditableCode(editor) {
        const selection = window.getSelection();
        const range = selection.rangeCount > 0 ? selection.getRangeAt(0) : null;
        
        // Save cursor position
        let cursorPos = 0;
        if (range) {
            cursorPos = range.startOffset;
        }

        // Apply highlighting
        const language = editor.dataset.language || 'javascript';
        this.applyHighlighting(editor, language);

        // Restore cursor position
        if (range) {
            const newRange = document.createRange();
            const textNode = editor.firstChild;
            if (textNode && textNode.nodeType === Node.TEXT_NODE) {
                newRange.setStart(textNode, Math.min(cursorPos, textNode.length));
                newRange.collapse(true);
                selection.removeAllRanges();
                selection.addRange(newRange);
            }
        }
    }

    // Search within code blocks
    setupSearchInCode() {
        const searchInputs = document.querySelectorAll('.code-search');
        
        searchInputs.forEach(input => {
            const container = input.closest('.code-container');
            const codeBlocks = container.querySelectorAll('code');
            
            input.addEventListener('input', (e) => {
                const searchTerm = e.target.value.toLowerCase();
                this.highlightSearchResults(codeBlocks, searchTerm);
            });
        });
    }

    highlightSearchResults(codeBlocks, searchTerm) {
        codeBlocks.forEach(block => {
            let content = block.textContent;
            
            if (searchTerm) {
                const regex = new RegExp(`(${searchTerm})`, 'gi');
                content = content.replace(regex, '<mark class="search-highlight">$1</mark>');
            }
            
            // Re-apply syntax highlighting while preserving search highlights
            // This is a simplified version - in production you'd need more sophisticated logic
            block.innerHTML = content;
        });
    }

    // Code formatting
    formatCode(code, language) {
        // Basic code formatting - in production use a proper formatter like Prettier
        let formatted = code;
        
        if (language === 'javascript') {
            formatted = formatted
                .replace(/;/g, ';\n')
                .replace(/{/g, '{\n')
                .replace(/}/g, '\n}')
                .replace(/,/g, ',\n');
        }
        
        return formatted;
    }

    // Code validation
    validateCode(code, language) {
        // Basic validation - in production use proper linters
        const issues = [];
        
        if (language === 'javascript') {
            if (code.includes('eval(')) {
                issues.push({ line: 1, message: 'Avoid using eval()' });
            }
            if (!code.includes('use strict') && code.length > 100) {
                issues.push({ line: 1, message: 'Consider using strict mode' });
            }
        }
        
        return issues;
    }

    // Export code as different formats
    exportCode(code, format) {
        switch (format) {
            case 'html':
                return this.exportAsHTML(code);
            case 'pdf':
                return this.exportAsPDF(code);
            case 'image':
                return this.exportAsImage(code);
            default:
                return code;
        }
    }

    exportAsHTML(code) {
        return `
            <html>
            <head>
                <style>
                    body { font-family: 'Fira Code', monospace; }
                    .keyword { color: #8b45ff; }
                    .string { color: #00d4aa; }
                    .comment { color: #6c757d; }
                    .number { color: #ff6b6b; }
                    .function { color: #4ecdc4; }
                </style>
            </head>
            <body>
                <pre><code>${code}</code></pre>
            </body>
            </html>
        `;
    }

    // Utility methods
    debounce(func, wait) {
        let timeout;
        return function executedFunction(...args) {
            const later = () => {
                clearTimeout(timeout);
                func(...args);
            };
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
        };
    }
}

// Language-specific code templates
const CodeTemplates = {
    python: {
        basic: `# Basic Python example
def main():
    print("Hello, BinaryOptionsToolsV2!")
    
if __name__ == "__main__":
    main()`,
        
        trading: `from binary_options_tools import BinaryOptionsAPI

# Initialize API
api = BinaryOptionsAPI()

# Connect to broker
api.connect()

# Place a trade
result = api.place_trade(
    asset="EURUSD",
    amount=10,
    direction="call",
    expiry=60
)

print(f"Trade result: {result}")`
    },
    
    javascript: {
        basic: `// Basic JavaScript example
function main() {
    console.log("Hello, BinaryOptionsToolsV2!");
}

main();`,
        
        trading: `const BinaryOptionsAPI = require('binary-options-tools');

// Initialize API
const api = new BinaryOptionsAPI();

// Connect and trade
async function trade() {
    await api.connect();
    
    const result = await api.placeTrade({
        asset: 'EURUSD',
        amount: 10,
        direction: 'call',
        expiry: 60
    });
    
    console.log('Trade result:', result);
}

trade();`
    },
    
    rust: {
        basic: `// Basic Rust example
fn main() {
    println!("Hello, BinaryOptionsToolsV2!");
}`,
        
        trading: `use binary_options_tools::BinaryOptionsAPI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut api = BinaryOptionsAPI::new();
    
    api.connect().await?;
    
    let result = api.place_trade(
        "EURUSD",
        10.0,
        "call",
        60
    ).await?;
    
    println!("Trade result: {:?}", result);
    Ok(())
}`
    }
};

// Initialize code highlighter when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    if (!window.codeHighlighter) {
        window.codeHighlighter = new CodeHighlighter();
    }
});

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { CodeHighlighter, CodeTemplates };
}
