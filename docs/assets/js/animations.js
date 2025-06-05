// Advanced Animation System for BinaryOptionsToolsV2 Documentation
class AnimationController {
    constructor() {
        this.animations = new Map();
        this.observers = new Map();
        this.isReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
        this.init();
    }

    init() {
        this.setupIntersectionObservers();
        this.setupScrollAnimations();
        this.setupHoverAnimations();
        this.setupLoadAnimations();
        this.setupParticleEffects();
        this.setupCounterAnimations();
    }

    // Intersection Observer for scroll-triggered animations
    setupIntersectionObservers() {
        const observerConfigs = [
            {
                selector: '.fade-in-up',
                options: { threshold: 0.1, rootMargin: '0px 0px -50px 0px' },
                animation: 'fadeInUp'
            },
            {
                selector: '.fade-in-left',
                options: { threshold: 0.1, rootMargin: '0px 0px -50px 0px' },
                animation: 'fadeInLeft'
            },
            {
                selector: '.fade-in-right',
                options: { threshold: 0.1, rootMargin: '0px 0px -50px 0px' },
                animation: 'fadeInRight'
            },
            {
                selector: '.scale-in',
                options: { threshold: 0.2 },
                animation: 'scaleIn'
            },
            {
                selector: '.slide-in-bottom',
                options: { threshold: 0.1 },
                animation: 'slideInBottom'
            },
            {
                selector: '.rotate-in',
                options: { threshold: 0.2 },
                animation: 'rotateIn'
            }
        ];

        observerConfigs.forEach(config => {
            const observer = new IntersectionObserver((entries) => {
                entries.forEach(entry => {
                    if (entry.isIntersecting) {
                        this.triggerAnimation(entry.target, config.animation);
                        observer.unobserve(entry.target);
                    }
                });
            }, config.options);

            document.querySelectorAll(config.selector).forEach(el => {
                observer.observe(el);
            });

            this.observers.set(config.selector, observer);
        });
    }

    // Scroll-based animations with throttling
    setupScrollAnimations() {
        let ticking = false;

        const updateAnimations = () => {
            const scrollY = window.pageYOffset;
            const windowHeight = window.innerHeight;

            // Parallax elements
            document.querySelectorAll('.parallax').forEach(element => {
                const rect = element.getBoundingClientRect();
                const speed = parseFloat(element.dataset.speed) || 0.5;
                const yPos = -(scrollY * speed);
                
                if (!this.isReducedMotion) {
                    element.style.transform = `translateY(${yPos}px)`;
                }
            });

            // Progress bars
            document.querySelectorAll('.progress-bar').forEach(bar => {
                const rect = bar.getBoundingClientRect();
                if (rect.top < windowHeight && rect.bottom > 0) {
                    const progress = Math.min(100, Math.max(0, 
                        ((windowHeight - rect.top) / windowHeight) * 100
                    ));
                    bar.style.setProperty('--progress', `${progress}%`);
                }
            });

            // Floating elements
            document.querySelectorAll('.float').forEach((element, index) => {
                if (!this.isReducedMotion) {
                    const offset = Math.sin(Date.now() * 0.001 + index) * 10;
                    element.style.transform = `translateY(${offset}px)`;
                }
            });

            ticking = false;
        };

        const requestTick = () => {
            if (!ticking) {
                requestAnimationFrame(updateAnimations);
                ticking = true;
            }
        };

        window.addEventListener('scroll', requestTick, { passive: true });
        
        // Initial call
        updateAnimations();
    }

    // Hover animations for interactive elements
    setupHoverAnimations() {
        // Card hover effects
        document.querySelectorAll('.card, .feature-card').forEach(card => {
            card.addEventListener('mouseenter', () => {
                if (!this.isReducedMotion) {
                    card.style.transform = 'translateY(-10px) scale(1.02)';
                    card.style.boxShadow = '0 20px 40px rgba(139, 69, 255, 0.3)';
                }
            });

            card.addEventListener('mouseleave', () => {
                card.style.transform = 'translateY(0) scale(1)';
                card.style.boxShadow = '';
            });
        });

        // Button hover effects
        document.querySelectorAll('.btn-primary, .btn-secondary').forEach(button => {
            button.addEventListener('mouseenter', () => {
                if (!this.isReducedMotion) {
                    button.style.transform = 'translateY(-2px)';
                    button.style.boxShadow = '0 10px 20px rgba(139, 69, 255, 0.4)';
                }
            });

            button.addEventListener('mouseleave', () => {
                button.style.transform = 'translateY(0)';
                button.style.boxShadow = '';
            });
        });

        // Logo spin on hover
        document.querySelectorAll('.logo').forEach(logo => {
            logo.addEventListener('mouseenter', () => {
                if (!this.isReducedMotion) {
                    logo.style.animation = 'spin 1s ease-in-out';
                }
            });

            logo.addEventListener('animationend', () => {
                logo.style.animation = '';
            });
        });
    }

    // Page load animations
    setupLoadAnimations() {
        window.addEventListener('load', () => {
            // Stagger animation for navigation items
            document.querySelectorAll('.navbar .nav-link').forEach((link, index) => {
                setTimeout(() => {
                    link.classList.add('animate-in');
                }, index * 100);
            });

            // Hero section animations
            const heroElements = document.querySelectorAll('.hero .fade-in-up');
            heroElements.forEach((element, index) => {
                setTimeout(() => {
                    element.classList.add('animate-in');
                }, index * 200 + 500);
            });

            // Loading screen fade out
            const loadingScreen = document.querySelector('.loading-screen');
            if (loadingScreen) {
                setTimeout(() => {
                    loadingScreen.style.opacity = '0';
                    setTimeout(() => {
                        loadingScreen.style.display = 'none';
                    }, 500);
                }, 1000);
            }
        });
    }

    // Particle effects for backgrounds
    setupParticleEffects() {
        const particleContainers = document.querySelectorAll('.particles');
        
        particleContainers.forEach(container => {
            this.createParticles(container);
        });
    }

    createParticles(container) {
        if (this.isReducedMotion) return;

        const particleCount = 50;
        const particles = [];

        for (let i = 0; i < particleCount; i++) {
            const particle = document.createElement('div');
            particle.className = 'particle';
            particle.style.cssText = `
                position: absolute;
                width: ${Math.random() * 4 + 1}px;
                height: ${Math.random() * 4 + 1}px;
                background: rgba(139, 69, 255, ${Math.random() * 0.5 + 0.1});
                border-radius: 50%;
                pointer-events: none;
            `;
            
            container.appendChild(particle);
            particles.push({
                element: particle,
                x: Math.random() * container.offsetWidth,
                y: Math.random() * container.offsetHeight,
                vx: (Math.random() - 0.5) * 0.5,
                vy: (Math.random() - 0.5) * 0.5,
                size: Math.random() * 4 + 1
            });
        }

        const animateParticles = () => {
            particles.forEach(particle => {
                particle.x += particle.vx;
                particle.y += particle.vy;

                // Wrap around edges
                if (particle.x < 0) particle.x = container.offsetWidth;
                if (particle.x > container.offsetWidth) particle.x = 0;
                if (particle.y < 0) particle.y = container.offsetHeight;
                if (particle.y > container.offsetHeight) particle.y = 0;

                particle.element.style.left = particle.x + 'px';
                particle.element.style.top = particle.y + 'px';
            });

            requestAnimationFrame(animateParticles);
        };

        animateParticles();
    }

    // Counter animations for statistics
    setupCounterAnimations() {
        const counters = document.querySelectorAll('.counter');
        
        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    this.animateCounter(entry.target);
                    observer.unobserve(entry.target);
                }
            });
        }, { threshold: 0.5 });

        counters.forEach(counter => observer.observe(counter));
    }

    animateCounter(element) {
        const target = parseInt(element.dataset.target) || 0;
        const duration = parseInt(element.dataset.duration) || 2000;
        const increment = target / (duration / 16);
        let current = 0;

        const updateCounter = () => {
            current += increment;
            if (current < target) {
                element.textContent = Math.floor(current).toLocaleString();
                requestAnimationFrame(updateCounter);
            } else {
                element.textContent = target.toLocaleString();
            }
        };

        updateCounter();
    }

    // Trigger specific animations
    triggerAnimation(element, animationType) {
        if (this.isReducedMotion) {
            element.style.opacity = '1';
            return;
        }

        element.classList.add('animate-in');

        // Add specific animation classes
        switch (animationType) {
            case 'fadeInUp':
                element.style.animation = 'fadeInUp 0.8s ease-out forwards';
                break;
            case 'fadeInLeft':
                element.style.animation = 'fadeInLeft 0.8s ease-out forwards';
                break;
            case 'fadeInRight':
                element.style.animation = 'fadeInRight 0.8s ease-out forwards';
                break;
            case 'scaleIn':
                element.style.animation = 'scaleIn 0.6s ease-out forwards';
                break;
            case 'slideInBottom':
                element.style.animation = 'slideInBottom 0.8s ease-out forwards';
                break;
            case 'rotateIn':
                element.style.animation = 'rotateIn 0.8s ease-out forwards';
                break;
        }

        // Stagger child animations
        const staggerChildren = element.querySelectorAll('.stagger-child');
        staggerChildren.forEach((child, index) => {
            setTimeout(() => {
                child.classList.add('animate-in');
                child.style.animation = `fadeInUp 0.6s ease-out forwards`;
            }, index * 100);
        });
    }

    // Text reveal animation
    setupTextReveal() {
        const textElements = document.querySelectorAll('.text-reveal');
        
        textElements.forEach(element => {
            const text = element.textContent;
            element.innerHTML = '';
            
            // Split text into spans
            text.split('').forEach((char, index) => {
                const span = document.createElement('span');
                span.textContent = char === ' ' ? '\u00A0' : char;
                span.style.animationDelay = `${index * 50}ms`;
                span.className = 'char';
                element.appendChild(span);
            });
        });
    }

    // Morphing shapes animation
    setupMorphingShapes() {
        const shapes = document.querySelectorAll('.morphing-shape');
        
        shapes.forEach(shape => {
            const paths = shape.dataset.paths?.split('|') || [];
            let current = 0;
            
            if (paths.length > 1) {
                setInterval(() => {
                    current = (current + 1) % paths.length;
                    const path = shape.querySelector('path');
                    if (path) {
                        path.style.transition = 'd 0.8s ease-in-out';
                        path.setAttribute('d', paths[current]);
                    }
                }, 3000);
            }
        });
    }

    // Clean up observers and animations
    destroy() {
        this.observers.forEach(observer => observer.disconnect());
        this.animations.clear();
        this.observers.clear();
    }
}

// Utility functions for common animations
const AnimationUtils = {
    // Smooth element transitions
    fadeIn(element, duration = 300) {
        element.style.opacity = '0';
        element.style.display = 'block';
        
        requestAnimationFrame(() => {
            element.style.transition = `opacity ${duration}ms ease-in-out`;
            element.style.opacity = '1';
        });
    },

    fadeOut(element, duration = 300) {
        element.style.transition = `opacity ${duration}ms ease-in-out`;
        element.style.opacity = '0';
        
        setTimeout(() => {
            element.style.display = 'none';
        }, duration);
    },

    // Slide animations
    slideDown(element, duration = 300) {
        element.style.height = '0';
        element.style.overflow = 'hidden';
        element.style.display = 'block';
        
        const height = element.scrollHeight;
        element.style.transition = `height ${duration}ms ease-in-out`;
        
        requestAnimationFrame(() => {
            element.style.height = height + 'px';
        });
        
        setTimeout(() => {
            element.style.height = 'auto';
        }, duration);
    },

    slideUp(element, duration = 300) {
        const height = element.scrollHeight;
        element.style.height = height + 'px';
        element.style.overflow = 'hidden';
        element.style.transition = `height ${duration}ms ease-in-out`;
        
        requestAnimationFrame(() => {
            element.style.height = '0';
        });
        
        setTimeout(() => {
            element.style.display = 'none';
        }, duration);
    },

    // Pulse effect
    pulse(element, color = 'rgba(139, 69, 255, 0.4)') {
        const ripple = document.createElement('div');
        ripple.style.cssText = `
            position: absolute;
            top: 50%;
            left: 50%;
            width: 0;
            height: 0;
            border-radius: 50%;
            background: ${color};
            transform: translate(-50%, -50%);
            animation: pulse 0.6s ease-out;
            pointer-events: none;
        `;
        
        element.style.position = 'relative';
        element.appendChild(ripple);
        
        setTimeout(() => {
            ripple.remove();
        }, 600);
    }
};

// Initialize animation controller when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    if (!window.animationController) {
        window.animationController = new AnimationController();
    }
});

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { AnimationController, AnimationUtils };
}
