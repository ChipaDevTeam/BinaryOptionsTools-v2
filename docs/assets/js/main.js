// Main JavaScript functionality for BinaryOptionsToolsV2 Documentation
class DocumentationApp {
    constructor() {
        this.currentLanguage = 'python';
        this.isScrolling = false;
        this.init();
    }    init() {
        this.setupLanguageTabs();
        this.setupInstallationTabs();
        this.setupScrollAnimations();
        this.setupNavigation();
        this.setupCopyButtons();
        this.setupMobileMenu();
        this.setupTypewriter();
        this.setupParallaxEffects();
        this.setupThemeToggle();
        this.setupSmoothScrolling();
        this.trackPerformance();
    }

    // Language Tab Functionality
    setupLanguageTabs() {
        const tabButtons = document.querySelectorAll('[data-tab]');
        const tabContents = document.querySelectorAll('[data-tab-content]');

        tabButtons.forEach(button => {
            button.addEventListener('click', (e) => {
                const targetTab = e.target.dataset.tab;
                this.switchLanguageTab(targetTab, tabButtons, tabContents);
            });
        });

        // Initialize first tab
        if (tabButtons.length > 0) {
            this.switchLanguageTab(this.currentLanguage, tabButtons, tabContents);
        }
    }

    switchLanguageTab(targetLanguage, tabButtons, tabContents) {
        // Remove active class from all tabs
        tabButtons.forEach(btn => btn.classList.remove('active'));
        tabContents.forEach(content => {
            content.classList.remove('active');
            content.style.display = 'none';
        });

        // Add active class to selected tab
        const activeButton = document.querySelector(`[data-tab="${targetLanguage}"]`);
        const activeContent = document.querySelector(`[data-tab-content="${targetLanguage}"]`);

        if (activeButton && activeContent) {
            activeButton.classList.add('active');
            activeContent.classList.add('active');
            activeContent.style.display = 'block';
            this.currentLanguage = targetLanguage;

            // Trigger animation
            this.animateTabContent(activeContent);
        }
    }

    animateTabContent(content) {
        content.style.opacity = '0';
        content.style.transform = 'translateY(20px)';
        
        requestAnimationFrame(() => {
            content.style.transition = 'all 0.3s ease';
            content.style.opacity = '1';
            content.style.transform = 'translateY(0)';
        });
    }

    // Scroll Animations
    setupScrollAnimations() {
        const animatedElements = document.querySelectorAll('.animate-on-scroll, .feature-card, .api-feature');
        
        const observerOptions = {
            threshold: 0.1,
            rootMargin: '0px 0px -50px 0px'
        };

        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    const animation = entry.target.dataset.animation || 'fade-in-up';
                    entry.target.classList.add('animate', animation);
                }
            });
        }, observerOptions);

        animatedElements.forEach(element => {
            observer.observe(element);
        });
    }

    // Navigation Functionality
    setupNavigation() {
        const nav = document.querySelector('.navbar');
        const dropdowns = document.querySelectorAll('.dropdown');

        // Sticky navigation
        window.addEventListener('scroll', () => {
            if (window.scrollY > 100) {
                nav.classList.add('scrolled');
            } else {
                nav.classList.remove('scrolled');
            }
        });

        // Dropdown menus
        dropdowns.forEach(dropdown => {
            const toggle = dropdown.querySelector('.dropdown-toggle');
            const menu = dropdown.querySelector('.dropdown-menu');

            toggle.addEventListener('click', (e) => {
                e.preventDefault();
                dropdown.classList.toggle('active');
            });

            // Close dropdown when clicking outside
            document.addEventListener('click', (e) => {
                if (!dropdown.contains(e.target)) {
                    dropdown.classList.remove('active');
                }
            });
        });

        // Smooth scrolling for anchor links
        document.querySelectorAll('a[href^="#"]').forEach(anchor => {
            anchor.addEventListener('click', (e) => {
                e.preventDefault();
                const target = document.querySelector(anchor.getAttribute('href'));
                if (target) {
                    target.scrollIntoView({
                        behavior: 'smooth',
                        block: 'start'
                    });
                }
            });
        });
    }

    // Enhanced Copy Button Functionality
    setupCopyButtons() {
        const copyButtons = document.querySelectorAll('.copy-btn');
        
        copyButtons.forEach(button => {
            button.addEventListener('click', async (e) => {
                e.preventDefault();
                const textToCopy = button.dataset.copy || button.parentElement.querySelector('code')?.textContent;
                
                if (textToCopy) {
                    try {
                        await navigator.clipboard.writeText(textToCopy);
                        
                        // Visual feedback
                        const originalIcon = button.innerHTML;
                        button.innerHTML = '<i class="fas fa-check"></i>';
                        button.classList.add('copied');
                        
                        setTimeout(() => {
                            button.innerHTML = originalIcon;
                            button.classList.remove('copied');
                        }, 2000);
                        
                        // Show toast notification
                        this.showToast('Code copied to clipboard!', 'success');
                    } catch (err) {
                        console.error('Failed to copy text: ', err);
                        this.showToast('Failed to copy code', 'error');
                    }
                }
            });
        });
    }

    // Installation Tab Functionality
    setupInstallationTabs() {
        const tabButtons = document.querySelectorAll('.tab-btn');
        const tabPanes = document.querySelectorAll('.tab-pane');

        tabButtons.forEach(button => {
            button.addEventListener('click', (e) => {
                e.preventDefault();
                const targetTab = button.dataset.tab;
                
                // Remove active class from all buttons and panes
                tabButtons.forEach(btn => btn.classList.remove('active'));
                tabPanes.forEach(pane => pane.classList.remove('active'));
                
                // Add active class to selected button and pane
                button.classList.add('active');
                const targetPane = document.querySelector(`.tab-pane[data-tab="${targetTab}"]`);
                if (targetPane) {
                    targetPane.classList.add('active');
                }
            });
        });
    }

    // Toast Notification System
    showToast(message, type = 'info') {
        const toast = document.createElement('div');
        toast.className = `toast toast-${type}`;
        toast.innerHTML = `
            <div class="toast-content">
                <i class="fas fa-${type === 'success' ? 'check-circle' : type === 'error' ? 'exclamation-circle' : 'info-circle'}"></i>
                <span>${message}</span>
            </div>
        `;
        
        document.body.appendChild(toast);
        
        // Trigger animation
        setTimeout(() => toast.classList.add('show'), 100);
        
        // Auto remove
        setTimeout(() => {
            toast.classList.remove('show');
            setTimeout(() => document.body.removeChild(toast), 300);
        }, 3000);
    }

    // Mobile Menu
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
                mobileMenu.classList.remove('active');
                overlay.classList.remove('active');
                document.body.classList.remove('menu-open');
            });

            // Close mobile menu when clicking links
            mobileMenu.querySelectorAll('a').forEach(link => {
                link.addEventListener('click', () => {
                    mobileMenu.classList.remove('active');
                    overlay?.classList.remove('active');
                    document.body.classList.remove('menu-open');
                });
            });
        }
    }

    // Typewriter Effect
    setupTypewriter() {
        const typewriterElements = document.querySelectorAll('.typewriter');
        
        typewriterElements.forEach(element => {
            const text = element.textContent;
            element.textContent = '';
            element.style.borderRight = '2px solid var(--primary-color)';
            
            let i = 0;
            const typeSpeed = 50;
            
            function type() {
                if (i < text.length) {
                    element.textContent += text.charAt(i);
                    i++;
                    setTimeout(type, typeSpeed);
                } else {
                    // Blinking cursor effect
                    setInterval(() => {
                        element.style.borderRight = element.style.borderRight === '2px solid transparent' 
                            ? '2px solid var(--primary-color)' 
                            : '2px solid transparent';
                    }, 500);
                }
            }
            
            // Start typing when element is visible
            const observer = new IntersectionObserver((entries) => {
                entries.forEach(entry => {
                    if (entry.isIntersecting) {
                        setTimeout(type, 500);
                        observer.unobserve(entry.target);
                    }
                });
            });
            
            observer.observe(element);
        });
    }

    // Parallax Effects
    setupParallaxEffects() {
        const parallaxElements = document.querySelectorAll('.parallax');
        
        window.addEventListener('scroll', () => {
            if (this.isScrolling) return;
            
            this.isScrolling = true;
            requestAnimationFrame(() => {
                const scrollY = window.pageYOffset;
                
                parallaxElements.forEach(element => {
                    const speed = element.dataset.speed || 0.5;
                    const yPos = -(scrollY * speed);
                    element.style.transform = `translateY(${yPos}px)`;
                });
                
                this.isScrolling = false;
            });
        });
    }

    // Theme Toggle (for future dark mode support)
    setupThemeToggle() {
        const themeToggle = document.querySelector('.theme-toggle');
        
        if (themeToggle) {
            themeToggle.addEventListener('click', () => {
                document.body.classList.toggle('dark-theme');
                localStorage.setItem('theme', 
                    document.body.classList.contains('dark-theme') ? 'dark' : 'light'
                );
            });

            // Load saved theme
            const savedTheme = localStorage.getItem('theme');
            if (savedTheme === 'dark') {
                document.body.classList.add('dark-theme');
            }
        }
    }

    // Performance Monitoring
    trackPerformance() {
        // Track page load time
        window.addEventListener('load', () => {
            const loadTime = performance.now();
            console.log(`Documentation loaded in ${loadTime.toFixed(2)}ms`);
        });

        // Track user interactions
        document.addEventListener('click', (e) => {
            if (e.target.matches('.btn, .nav-link, .feature-card')) {
                console.log(`User clicked: ${e.target.textContent?.trim() || e.target.className}`);
            }
        });
    }

    // Utility Methods
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

    throttle(func, limit) {
        let inThrottle;
        return function() {
            const args = arguments;
            const context = this;
            if (!inThrottle) {
                func.apply(context, args);
                inThrottle = true;
                setTimeout(() => inThrottle = false, limit);
            }
        };
    }
}

// Initialize the app when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    new DocumentationApp();
});

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
    module.exports = DocumentationApp;
}
