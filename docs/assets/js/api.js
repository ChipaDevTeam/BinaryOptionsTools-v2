// API Documentation Interactive Functionality
class APIDocumentation {
    constructor() {
        this.currentLanguage = 'python';
        this.init();
    }

    init() {
        this.setupLanguageSelector();
        this.setupDetailTabs();
        this.setupCodeTabs();
        this.setupSearch();
        this.setupCopyButtons();
        this.setupSidebarNavigation();
        this.setupMobileMenu();
    }

    // Language Selector for the entire API page
    setupLanguageSelector() {
        const langButtons = document.querySelectorAll('.lang-btn');
        
        langButtons.forEach(button => {
            button.addEventListener('click', () => {
                const language = button.dataset.tab;
                this.switchLanguage(language, langButtons);
            });
        });
    }

    switchLanguage(language, langButtons) {
        // Update language selector
        langButtons.forEach(btn => btn.classList.remove('active'));
        document.querySelector(`[data-tab="${language}"]`).classList.add('active');
        
        // Update all code examples to show the selected language
        const codeExamples = document.querySelectorAll('.code-example');
        codeExamples.forEach(example => {
            example.classList.remove('active');
            if (example.dataset.lang === language) {
                example.classList.add('active');
            }
        });

        // Update code tabs
        const codeTabs = document.querySelectorAll('.code-tab');
        codeTabs.forEach(tab => {
            tab.classList.remove('active');
            if (tab.dataset.lang === language) {
                tab.classList.add('active');
            }
        });
        
        this.currentLanguage = language;
    }

    // Detail Tabs (Parameters, Response, Example)
    setupDetailTabs() {
        const detailTabs = document.querySelectorAll('.detail-tab');
        
        detailTabs.forEach(tab => {
            tab.addEventListener('click', () => {
                const detail = tab.dataset.detail;
                const container = tab.closest('.endpoint-details');
                this.switchDetailTab(detail, container);
            });
        });
    }

    switchDetailTab(detail, container) {
        // Update tab buttons
        container.querySelectorAll('.detail-tab').forEach(tab => {
            tab.classList.remove('active');
        });
        container.querySelector(`[data-detail="${detail}"]`).classList.add('active');

        // Update content panes
        container.querySelectorAll('.detail-pane').forEach(pane => {
            pane.classList.remove('active');
        });
        container.querySelector(`[data-detail="${detail}"].detail-pane`).classList.add('active');
    }

    // Code Tabs within examples
    setupCodeTabs() {
        const codeTabContainers = document.querySelectorAll('.code-tabs');
        
        codeTabContainers.forEach(container => {
            const tabs = container.querySelectorAll('.code-tab');
            
            tabs.forEach(tab => {
                tab.addEventListener('click', () => {
                    const language = tab.dataset.lang;
                    const exampleContainer = container.closest('.detail-pane');
                    this.switchCodeTab(language, exampleContainer);
                });
            });
        });
    }

    switchCodeTab(language, container) {
        // Update code tab buttons
        container.querySelectorAll('.code-tab').forEach(tab => {
            tab.classList.remove('active');
        });
        container.querySelector(`[data-lang="${language}"].code-tab`).classList.add('active');

        // Update code examples
        container.querySelectorAll('.code-example').forEach(example => {
            example.classList.remove('active');
        });
        container.querySelector(`[data-lang="${language}"].code-example`).classList.add('active');
    }

    // Search functionality
    setupSearch() {
        const searchInput = document.getElementById('api-search');
        if (!searchInput) return;

        searchInput.addEventListener('input', (e) => {
            const query = e.target.value.toLowerCase().trim();
            this.filterAPIethods(query);
        });
    }

    filterAPIMethods(query) {
        const methodLinks = document.querySelectorAll('.api-method');
        const sections = document.querySelectorAll('.nav-section');

        methodLinks.forEach(link => {
            const text = link.textContent.toLowerCase();
            const listItem = link.closest('li');
            
            if (text.includes(query)) {
                listItem.style.display = 'block';
            } else {
                listItem.style.display = 'none';
            }
        });

        // Hide sections with no visible methods
        sections.forEach(section => {
            const visibleMethods = section.querySelectorAll('li[style="display: block"], li:not([style])');
            const allMethods = section.querySelectorAll('li');
            
            if (query && visibleMethods.length === 0) {
                section.style.display = 'none';
            } else {
                section.style.display = 'block';
            }
        });
    }

    // Enhanced copy buttons for code examples
    setupCopyButtons() {
        const codeBlocks = document.querySelectorAll('.code-example pre');
        
        codeBlocks.forEach(block => {
            // Check if copy button already exists
            if (block.querySelector('.copy-btn')) return;
            
            const button = document.createElement('button');
            button.className = 'copy-btn';
            button.innerHTML = `
                <i class="fas fa-copy"></i>
                Copy
            `;
            
            block.style.position = 'relative';
            block.appendChild(button);

            button.addEventListener('click', async () => {
                const code = block.querySelector('code').textContent;
                try {
                    await navigator.clipboard.writeText(code);
                    button.innerHTML = `
                        <i class="fas fa-check"></i>
                        Copied!
                    `;
                    button.classList.add('copied');
                    
                    setTimeout(() => {
                        button.innerHTML = `
                            <i class="fas fa-copy"></i>
                            Copy
                        `;
                        button.classList.remove('copied');
                    }, 2000);
                } catch (err) {
                    console.error('Failed to copy:', err);
                    button.innerHTML = `
                        <i class="fas fa-times"></i>
                        Failed
                    `;
                    
                    setTimeout(() => {
                        button.innerHTML = `
                            <i class="fas fa-copy"></i>
                            Copy
                        `;
                    }, 2000);
                }
            });
        });
    }

    // Sidebar navigation with smooth scrolling
    setupSidebarNavigation() {
        const navLinks = document.querySelectorAll('.api-method');
        
        navLinks.forEach(link => {
            link.addEventListener('click', (e) => {
                e.preventDefault();
                const targetId = link.getAttribute('href');
                const targetElement = document.querySelector(targetId);
                
                if (targetElement) {
                    // Update active states
                    navLinks.forEach(l => l.classList.remove('active'));
                    link.classList.add('active');
                    
                    // Smooth scroll to target
                    targetElement.scrollIntoView({
                        behavior: 'smooth',
                        block: 'start'
                    });
                    
                    // Highlight the target briefly
                    targetElement.classList.add('highlighted');
                    setTimeout(() => {
                        targetElement.classList.remove('highlighted');
                    }, 2000);
                }
            });
        });

        // Update active navigation based on scroll position
        this.setupScrollSpy();
    }

    setupScrollSpy() {
        const sections = document.querySelectorAll('.endpoint-card[id]');
        const navLinks = document.querySelectorAll('.api-method');

        const observerOptions = {
            rootMargin: '-20% 0px -80% 0px',
            threshold: 0
        };

        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    const id = entry.target.id;
                    
                    // Update active navigation
                    navLinks.forEach(link => {
                        link.classList.remove('active');
                        if (link.getAttribute('href') === `#${id}`) {
                            link.classList.add('active');
                        }
                    });
                }
            });
        }, observerOptions);

        sections.forEach(section => {
            observer.observe(section);
        });
    }

    // Mobile menu functionality
    setupMobileMenu() {
        const mobileToggle = document.querySelector('.mobile-menu-toggle');
        const mobileMenu = document.querySelector('.mobile-menu');
        const overlay = document.querySelector('.mobile-overlay');

        if (mobileToggle && mobileMenu) {
            mobileToggle.addEventListener('click', () => {
                mobileMenu.classList.toggle('active');
                overlay?.classList.toggle('active');
                document.body.classList.toggle('menu-open');
            });

            // Close mobile menu when clicking overlay
            overlay?.addEventListener('click', () => {
                this.closeMobileMenu();
            });

            // Close mobile menu when clicking links
            mobileMenu.querySelectorAll('a').forEach(link => {
                link.addEventListener('click', () => {
                    this.closeMobileMenu();
                });
            });

            // Close mobile menu on window resize
            window.addEventListener('resize', () => {
                if (window.innerWidth > 768) {
                    this.closeMobileMenu();
                }
            });
        }
    }

    closeMobileMenu() {
        const mobileMenu = document.querySelector('.mobile-menu');
        const overlay = document.querySelector('.mobile-overlay');
        
        mobileMenu?.classList.remove('active');
        overlay?.classList.remove('active');
        document.body.classList.remove('menu-open');
    }

    // Utility method to add syntax highlighting
    addSyntaxHighlighting() {
        if (typeof Prism !== 'undefined') {
            Prism.highlightAll();
        }
    }

    // Add endpoint card animations
    setupCardAnimations() {
        const cards = document.querySelectorAll('.endpoint-card');
        
        const observerOptions = {
            threshold: 0.1,
            rootMargin: '0px 0px -50px 0px'
        };

        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    entry.target.classList.add('animate-in');
                }
            });
        }, observerOptions);

        cards.forEach(card => {
            observer.observe(card);
        });
    }
}

// Initialize API Documentation functionality
document.addEventListener('DOMContentLoaded', () => {
    // Only initialize on API pages
    if (document.querySelector('.api-content')) {
        new APIDocumentation();
    }
});

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
    module.exports = APIDocumentation;
}
