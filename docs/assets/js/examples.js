// Examples Page JavaScript
document.addEventListener('DOMContentLoaded', function() {
    // Filter functionality
    const filterButtons = document.querySelectorAll('.filter-btn');
    const exampleCategories = document.querySelectorAll('.examples-category');
    
    filterButtons.forEach(button => {
        button.addEventListener('click', () => {
            const category = button.dataset.category;
            
            // Update active button
            filterButtons.forEach(btn => btn.classList.remove('active'));
            button.classList.add('active');
            
            // Filter categories
            exampleCategories.forEach(categoryEl => {
                if (category === 'all' || categoryEl.id.includes(category)) {
                    categoryEl.style.display = 'block';
                    categoryEl.classList.add('fade-in-up');
                } else {
                    categoryEl.style.display = 'none';
                    categoryEl.classList.remove('fade-in-up');
                }
            });
        });
    });
    
    // Tab functionality for code examples
    const tabButtons = document.querySelectorAll('.tab-button');
    const tabContents = document.querySelectorAll('.tab-content');
    
    tabButtons.forEach(button => {
        button.addEventListener('click', () => {
            const tabId = button.dataset.tab;
            const tabGroup = button.closest('.example-block');
            
            // Update active button within the same tab group
            tabGroup.querySelectorAll('.tab-button').forEach(btn => btn.classList.remove('active'));
            button.classList.add('active');
            
            // Update active content within the same tab group
            tabGroup.querySelectorAll('.tab-content').forEach(content => {
                content.classList.remove('active');
                if (content.id === tabId) {
                    content.classList.add('active');
                }
            });
        });
    });
    
    // Smooth scrolling for navigation links
    const navLinks = document.querySelectorAll('.examples-nav .nav-link, .footer-nav-link');
    navLinks.forEach(link => {
        link.addEventListener('click', (e) => {
            const href = link.getAttribute('href');
            if (href && href.startsWith('#')) {
                e.preventDefault();
                const target = document.querySelector(href);
                if (target) {
                    target.scrollIntoView({
                        behavior: 'smooth',
                        block: 'start'
                    });
                    
                    // Update active nav link
                    document.querySelectorAll('.examples-nav .nav-link').forEach(navLink => {
                        navLink.classList.remove('active');
                    });
                    link.classList.add('active');
                }
            }
        });
    });
    
    // Sticky sidebar navigation
    const sidebar = document.querySelector('.examples-sidebar');
    const sidebarSticky = document.querySelector('.sidebar-sticky');
    
    if (sidebar && sidebarSticky) {
        const sidebarTop = sidebar.offsetTop;
        
        function updateSidebarPosition() {
            const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
            
            if (scrollTop > sidebarTop - 100) {
                sidebarSticky.style.position = 'fixed';
                sidebarSticky.style.top = '120px';
                sidebarSticky.style.width = sidebar.offsetWidth + 'px';
            } else {
                sidebarSticky.style.position = 'static';
                sidebarSticky.style.width = 'auto';
            }
        }
        
        window.addEventListener('scroll', updateSidebarPosition);
        window.addEventListener('resize', updateSidebarPosition);
    }
    
    // Highlight active section in navigation
    const sections = document.querySelectorAll('.example-card');
    const navLinksInternal = document.querySelectorAll('.examples-nav .nav-link');
    
    function updateActiveNavigation() {
        const scrollPosition = window.scrollY + 150;
        
        sections.forEach((section, index) => {
            const sectionTop = section.offsetTop;
            const sectionBottom = sectionTop + section.offsetHeight;
            const sectionId = section.id;
            
            if (scrollPosition >= sectionTop && scrollPosition < sectionBottom) {
                navLinksInternal.forEach(navLink => {
                    navLink.classList.remove('active');
                    if (navLink.getAttribute('href') === `#${sectionId}`) {
                        navLink.classList.add('active');
                    }
                });
            }
        });
    }
    
    window.addEventListener('scroll', updateActiveNavigation);
    
    // Search functionality
    const searchInput = document.getElementById('api-search');
    if (searchInput) {
        searchInput.addEventListener('input', (e) => {
            const searchTerm = e.target.value.toLowerCase();
            const exampleCards = document.querySelectorAll('.example-card');
            
            exampleCards.forEach(card => {
                const title = card.querySelector('h3').textContent.toLowerCase();
                const description = card.querySelector('.example-description p').textContent.toLowerCase();
                const badges = Array.from(card.querySelectorAll('.badge')).map(badge => badge.textContent.toLowerCase()).join(' ');
                
                if (title.includes(searchTerm) || description.includes(searchTerm) || badges.includes(searchTerm)) {
                    card.style.display = 'block';
                } else {
                    card.style.display = 'none';
                }
            });
        });
    }
    
    // Copy to clipboard functionality
    window.copyCode = function(elementId) {
        const codeElement = document.getElementById(elementId);
        if (!codeElement) return;
        
        const text = codeElement.textContent;
        
        if (navigator.clipboard) {
            navigator.clipboard.writeText(text).then(() => {
                showCopySuccess();
            }).catch(err => {
                console.error('Failed to copy: ', err);
                fallbackCopyTextToClipboard(text);
            });
        } else {
            fallbackCopyTextToClipboard(text);
        }
    };
    
    function fallbackCopyTextToClipboard(text) {
        const textArea = document.createElement("textarea");
        textArea.value = text;
        
        // Avoid scrolling to bottom
        textArea.style.top = "0";
        textArea.style.left = "0";
        textArea.style.position = "fixed";
        
        document.body.appendChild(textArea);
        textArea.focus();
        textArea.select();
        
        try {
            const successful = document.execCommand('copy');
            if (successful) {
                showCopySuccess();
            }
        } catch (err) {
            console.error('Fallback: Oops, unable to copy', err);
        }
        
        document.body.removeChild(textArea);
    }
    
    function showCopySuccess() {
        // Create and show a temporary success message
        const successMsg = document.createElement('div');
        successMsg.className = 'copy-success';
        successMsg.textContent = '✅ Code copied to clipboard!';
        successMsg.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            background: var(--primary-gradient);
            color: white;
            padding: 12px 20px;
            border-radius: 8px;
            font-weight: 500;
            z-index: 10000;
            animation: slideInRight 0.3s ease-out;
        `;
        
        document.body.appendChild(successMsg);
        
        setTimeout(() => {
            successMsg.style.animation = 'slideOutRight 0.3s ease-in';
            setTimeout(() => {
                document.body.removeChild(successMsg);
            }, 300);
        }, 2000);
    }
    
    // Animate cards on scroll
    const observerOptions = {
        threshold: 0.1,
        rootMargin: '0px 0px -50px 0px'
    };
    
    const observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.classList.add('fade-in-up');
            }
        });
    }, observerOptions);
    
    // Observe all example cards
    document.querySelectorAll('.example-card, .tip-card, .ssid-step').forEach(card => {
        observer.observe(card);
    });
    
    // Mobile responsive handling
    function handleMobileView() {
        const sidebar = document.querySelector('.examples-sidebar');
        const mainContent = document.querySelector('.examples-main-content');
        
        if (window.innerWidth <= 768) {
            // Mobile: stack sidebar below content
            if (sidebar && mainContent) {
                sidebar.style.order = '2';
                mainContent.style.order = '1';
            }
        } else {
            // Desktop: sidebar on left
            if (sidebar && mainContent) {
                sidebar.style.order = '1';
                mainContent.style.order = '2';
            }
        }
    }
    
    window.addEventListener('resize', handleMobileView);
    handleMobileView(); // Call on load
});

// Add CSS classes for animations
const style = document.createElement('style');
style.textContent = `
    .fade-in-up {
        animation: fadeInUp 0.6s ease forwards;
    }
    
    @keyframes fadeInUp {
        from {
            opacity: 0;
            transform: translateY(30px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }
      .copy-btn.copied {
        background: var(--accent-color) !important;
        color: white !important;
    }
`;
document.head.appendChild(style);

    // Initialize first tab content as visible in each example block
    const exampleBlocks = document.querySelectorAll('.example-block');
    exampleBlocks.forEach(block => {
        const firstTabContent = block.querySelector('.tab-content');
        if (firstTabContent && !firstTabContent.classList.contains('active')) {
            firstTabContent.classList.add('active');
        }
    });

});
