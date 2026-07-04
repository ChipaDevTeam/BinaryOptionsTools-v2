---
sidebar_position: 1
slug: /project/deployment
---

# GitLab Pages Deployment Guide

This documentation site is deployed to GitLab Pages using Docusaurus.

## Quick Deployment Steps

### 1. Enable GitHub Pages

1. Go to your repository on GitLab
2. Click on **Settings** tab
3. Scroll down to **Pages** section
4. Under **Source**, select **Deploy from a branch**
5. Choose `master` branch and `/` folder
6. Click **Save**

### 2. Configure Docusaurus

Update `docusaurus.config.js` with your project details:

```javascript
url: 'https://chipatrade.gitlab.io',
baseUrl: '/chipadevorg/BinaryOptionsTools-v2/',
organizationName: 'chipadevorg',
projectName: 'BinaryOptionsTools-v2',
```

### 3. Deploy
The site is automatically deployed via GitLab CI on push to master branch.

Manual deployment:
```bash
npm run build
npm run deploy
```

### 4. Custom Domain (Optional)

1. Add a `CNAME` file to `static/` folder with your domain:
   ```
   your-domain.com
   ```
2. Configure DNS settings with your domain provider

## GitLab CI Workflow

The deployment is handled by `.gitlab-ci.yml`:

```yaml
stages:
  - build

pages:
  stage: build
  image: node:20-bookworm
  script:
    - npm ci
    - npm run build
  artifacts:
    paths:
      - public
  only:
    - master
```

## Features Enabled

- ✅ **Responsive Design** - Works on desktop, tablet, and mobile
- ✅ **Dark/Light Mode** - Choose your preferred viewing theme
- ✅ **Search** - Full-text search via Algolia DocSearch
- ✅ **Versioned Docs** - Maintain documentation for multiple versions
- ✅ **Internationalization** - Ready for multi-language support
- ✅ **SEO Optimized** - Sitemap, meta tags, Open Graph

## File Structure

```
docs/
├── intro.md              # Homepage
├── overview.md           # Documentation overview
├── api/                  # API Reference
├── guides/               # Trading guides
├── architecture/         # Architecture docs
├── examples/             # Code examples
├── tutorials/            # Step-by-step tutorials
├── project/              # Project info
├── docusaurus.config.js  # Docusaurus configuration
├── sidebars.js           # Sidebar navigation
├── package.json          # Dependencies
├── src/
│   └── css/
│       └── custom.css    # Custom styles
└── static/
    ├── img/              # Images
    └── CNAME             # Custom domain (optional)
```

## Customization

### Colors

Edit CSS custom properties in `src/css/custom.css`:

```css
:root {
  --ifm-color-primary: #4f46e5;
  --ifm-color-primary-dark: #4338ca;
  --ifm-color-primary-darker: #3730a3;
  --ifm-color-primary-darkest: #312e81;
  --ifm-color-primary-light: #6366f1;
  --ifm-color-primary-lighter: #818cf8;
  --ifm-color-primary-lightest: #a5b4fc;
}
```

### Content

- Edit markdown files in `docs/` for content changes
- Modify `sidebars.js` for navigation structure
- Update `docusaurus.config.js` for site configuration

## Troubleshooting

### Site not loading?

1. Check if GitLab Pages is enabled in repository settings
2. Ensure the branch and folder are correctly selected (master / public)
3. Wait 5-10 minutes for changes to propagate
4. Check GitLab CI build logs

### Styles not loading?

1. Check file paths in HTML files
2. Ensure all CSS files are properly imported
3. Verify baseUrl in docusaurus.config.js

### Build failing?

1. Check Node.js version (requires 18+)
2. Run `npm ci` to reinstall dependencies
3. Check for markdown syntax errors

## Performance Tips

1. **Images**: Add optimized images to `static/img/`
2. **Caching**: GitHub Pages automatically handles caching
3. **CDN**: GitHub Pages uses a global CDN
4. **Minification**: Docusaurus minifies CSS/JS in production

## Analytics Integration

Add Google Analytics by updating `docusaurus.config.js`:

```javascript
themeConfig: {
  // ...
  gtag: {
    trackingID: 'GA_MEASUREMENT_ID',
    anonymizeIP: true,
  },
}
```

## Support

For issues with the documentation site:
1. Check this deployment guide
2. Verify all file paths are correct
3. Test locally with `npm run start`
4. Check GitHub Actions build logs in repository Actions tab