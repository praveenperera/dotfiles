---
name: tailwindplus
description: >
  TailwindPlus UI component catalog and selection guide. Use when building frontend interfaces
  with Tailwind CSS, selecting UI blocks for pages, recommending component layouts, or any task
  involving Tailwind Plus components (marketing sites, dashboards, ecommerce stores).
---

# TailwindPlus Component Catalog

Comprehensive catalog of 500+ UI components from Tailwind Plus (tailwindcss.com/plus), organized across three categories. Use this to recommend and select the right components when building interfaces.

## Setup Requirements

Projects using TailwindPlus components need:
- **Tailwind CSS v4.1+**
- **Inter font** (via Google Fonts or Fontsource)
- **@tailwindcss/forms**, **@headlessui/react** (or Vue), **@heroicons/react** (or Vue) depending on component
- Dark mode supported via `dark:` variants

## Available Formats

Every component is available in three formats:
- **HTML** — plain HTML with Tailwind classes
- **React** — JSX with Headless UI and Heroicons
- **Vue** — SFC with Headless UI and Heroicons

## URL Pattern

```
https://tailwindcss.com/plus/ui-blocks/{category}/{subcategory}/{component}
```

## Category Selection Guide

### Marketing (166 total)
**Use for:** Public-facing pages — landing pages, pricing, about, blog, contact, and any page meant to attract or convert visitors.

Key component types: Hero Sections, Feature Sections, CTA Sections, Pricing, Headers, Footers, Testimonials, Blog Sections, Newsletter, Stats, Team, FAQ, Contact, Logo Clouds, Content Sections, Bento Grids, Banners, 404 Pages

Full catalog: `references/marketing.md`

### Application UI (414 total)
**Use for:** Authenticated app interfaces — dashboards, settings, data tables, forms, navigation, and internal tools.

Key component types: Application Shells (Stacked/Sidebar/Multi-Column), Tables, Stacked Lists, Forms (Input Groups, Select Menus, Radio Groups, Checkboxes, Toggles), Navbars, Tabs, Command Palettes, Modal Dialogs, Drawers, Notifications, Alerts, Empty States, Calendars, Stats, Buttons, Badges, Avatars, Cards, Breadcrumbs, Progress Bars, Pagination

Full catalog: `references/application-ui.md`

### Ecommerce (118 total)
**Use for:** Online store pages — product listings, product details, shopping carts, checkout, order history, and store navigation.

Key component types: Product Overviews, Product Lists, Category Previews, Shopping Carts, Category Filters, Product Quickviews, Product Features, Store Navigation, Promo Sections, Checkout Forms, Reviews, Order Summaries, Order History, Incentives

Full catalog: `references/ecommerce.md`

## Component Selection Workflow

1. **Identify the page type** — Is this a marketing page, an app screen, or a store page?
2. **Load the reference file** for the matching category
3. **Find the component type** that matches what you need (e.g., "I need a pricing section" → Marketing > Pricing Sections)
4. **Pick the variant** that best fits the design requirements
5. **Construct the URL** using the base URL + slug pattern to view or copy the code

## Common Scenarios

| Building... | Start with |
|---|---|
| Landing page | Marketing: Hero + Features + CTA + Pricing + Footer |
| SaaS dashboard | App UI: Sidebar Layout + Stats + Tables + Cards |
| Settings page | App UI: Stacked Layout + Form Layouts + Toggles + Action Panels |
| Product page | Ecommerce: Product Overview + Product Features + Reviews |
| Storefront | Ecommerce: Store Navigation + Promo Sections + Category Previews + Product Lists |
| Blog | Marketing: Header Section + Blog Sections + Footer |
| Auth pages | App UI: Sign-in Forms |
| Admin panel | App UI: Sidebar Layout + Tables + Modal Dialogs + Alerts |
