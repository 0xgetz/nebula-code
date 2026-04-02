# Nebula Code Marketplace

Web-based marketplace for discovering and sharing AI-powered development tools and skill cards.

## Getting Started

### Prerequisites

- Node.js 18+
- pnpm 9+

### Development

```bash
# Install dependencies
pnpm install

# Run development server
pnpm dev
```

The app will be available at http://localhost:3000

### Build

```bash
# Build for production
pnpm build

# Start production server
pnpm start
```

## Tech Stack

- **Framework**: Next.js 14 (App Router)
- **Language**: TypeScript
- **Styling**: Tailwind CSS
- **UI Components**: @nebula-code/ui-components

## Project Structure

```
apps/marketplace/
├── src/
│   ├── app/           # Next.js App Router
│   ├── components/    # React components
│   ├── lib/          # Utilities and helpers
│   └── styles/       # Global styles
├── public/           # Static assets
├── next.config.ts    # Next.js configuration
└── package.json      # Dependencies
```

## Features

- Browse and discover skill cards
- User authentication (coming soon)
- Skill card ratings and reviews
- Download and install skills directly
- Community contributions
