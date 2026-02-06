# GitHub Pages Deployment Guide

## Quick Deployment Steps

### 1. Enable GitHub Pages

1. Go to your repository on GitHub
2. Click on **Settings** tab
3. Scroll down to **Pages** section
4. Under **Source**, select **Deploy from a branch**
5. Choose **main** branch and **/docs** folder
6. Click **Save**

### 2. Update Configuration

Before deploying, update these values in the documentation:

#### In `_config.yml`

```yaml
url: "https://yourusername.github.io"
baseurl: "/your-repository-name"
```

#### Replace placeholders

- `yourusername` → Your GitHub username
- `your-repository-name` → Your actual repository name
- `your-google-site-verification-code` → Your Google Search Console verification code
- `your-bing-site-verification-code` → Your Bing Webmaster verification code

### 3. Test Locally (Optional)

```bash
cd docs
python -m http.server 8000
# Open http://localhost:8000 in your browser
```

### 4. Custom Domain (Optional)

1. Add a `CNAME` file to the docs folder with your domain:

   ```
   your-domain.com
   ```

2. Configure DNS settings with your domain provider

## Features Enabled

✅ **Purple Theme** - Modern glassmorphism design with purple color scheme
✅ **Multi-language Support** - Python, JavaScript, and Rust documentation
✅ **Interactive Examples** - Live code examples with syntax highlighting
✅ **Responsive Design** - Mobile-friendly navigation and layouts
✅ **SEO Optimized** - Complete sitemap.xml and meta tags
✅ **Performance Optimized** - GPU-accelerated animations and lazy loading
✅ **Bot Services Integration** - chipa.tech bot creation services
✅ **API Documentation** - Complete reference for all languages
✅ **Copy-to-clipboard** - Easy code copying functionality
✅ **Search Functionality** - Built-in documentation search

## File Structure

```
docs/
├── index.html              # Homepage
├── python.html            # Python documentation
├── javascript.html        # JavaScript documentation
├── rust.html             # Rust documentation
├── api.html              # API reference
├── examples.html         # Interactive examples
├── sitemap.xml           # SEO sitemap
├── favicon.svg           # Site icon
├── _config.yml           # GitHub Pages config
├── .nojekyll            # Skip Jekyll processing
├── README.md            # Documentation guide
└── assets/
    ├── css/
    │   ├── main.css         # Main styles
    │   ├── animations.css   # Animation library
    │   └── code-highlight.css # Syntax highlighting
    └── js/
        ├── main.js          # Core functionality
        ├── animations.js    # Animation controller
        └── code-highlight.js # Code highlighting
```

## Customization

### Colors

Edit the CSS custom properties in `assets/css/main.css`:

```css
:root {
  --primary-color: #8b5cf6; /* Main purple */
  --secondary-color: #a855f7; /* Secondary purple */
  --accent-color: #c084fc; /* Light purple */
}
```

### Content

- Edit HTML files directly for content changes
- Modify JavaScript files for functionality changes
- Update CSS files for styling changes

## Troubleshooting

### Site not loading?

1. Check if GitHub Pages is enabled in repository settings
2. Ensure the branch and folder are correctly selected
3. Wait 5-10 minutes for changes to propagate

### Styles not loading?

1. Check file paths in HTML files
2. Ensure all CSS files are in `assets/css/`
3. Verify `.nojekyll` file exists

### JavaScript not working?

1. Check browser console for errors
2. Ensure all JS files are in `assets/js/`
3. Verify file paths in HTML files

## Performance Tips

1. **Images**: Add images to `assets/images/` and optimize them
2. **Caching**: GitHub Pages automatically handles caching
3. **CDN**: Consider using a CDN for better global performance
4. **Minification**: Minify CSS/JS files for production

## Analytics Integration

Add Google Analytics by inserting this code before `</head>` in all HTML files:

```html
<!-- Google tag (gtag.js) -->
<script
  async
  src="https://www.googletagmanager.com/gtag/js?id=GA_MEASUREMENT_ID"
></script>
<script>
  window.dataLayer = window.dataLayer || [];
  function gtag() {
    dataLayer.push(arguments);
  }
  gtag("js", new Date());
  gtag("config", "GA_MEASUREMENT_ID");
</script>
```

Replace `GA_MEASUREMENT_ID` with your actual Google Analytics measurement ID.

## Support

For issues with the documentation site:

1. Check this deployment guide
2. Verify all file paths are correct
3. Test locally before deploying
4. Check GitHub Pages build logs in repository Actions tab

Your documentation site is now ready for deployment! 🚀
