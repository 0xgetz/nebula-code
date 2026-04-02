# [Nebula Code Desktop](https://nebula.ai/code)

Desktop application built with Tauri, providing a native experience for Nebula Code.

## Getting Started

### Prerequisites

- Node.js 18+
- Rust 1.70+
- pnpm 9+

### Development

```bash
# Install dependencies
pnpm install

# Run development server
pnpm dev
```

### Build

```bash
# Build for production
pnpm build
```

## Tech Stack

- **Frontend**: React 18, TypeScript, Vite
- **Desktop**: Tauri 1.5
- **Styling**: CSS Modules

## Project Structure

```
apps/desktop/
├── src/
│   ├── components/    # React components
│   ├── lib/          # Utilities and helpers
│   ├── styles/       # Global styles
│   └── main.tsx      # Application entry point
├── public/           # Static assets
├── tauri.conf.json   # Tauri configuration
├── Cargo.toml        # Rust dependencies
└── package.json      # Node dependencies
```
