# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Project Dave is a desktop application that provides a user interface for interacting with the Autonomi network - a decentralized file storage system. It's built with Tauri v2 (Rust backend) and Nuxt 3 (Vue.js frontend).

## Essential Commands

### Development
```bash
# Start the full Tauri application in development mode
npm run tauri dev

# Start only the Nuxt frontend dev server (port 1420)
npm run dev

# For development: Skip wallet signing by providing vault signature via ENV
VITE_DEV_VAULT_SIGNATURE=0xYourSignatureHere npm run tauri dev
```

### Build
```bash
# Build the complete desktop application
npm run tauri build

# Build only the Nuxt frontend
npm run generate
```

### Important Notes
- No linting or formatting commands are configured - add them if needed
- No test framework is set up - implement testing infrastructure as required
- Always target the `development` branch for PRs, not `main`
- Follow conventional commits specification

## Architecture Overview

### Technology Stack
- **Backend**: Rust with Tauri v2 for native desktop capabilities
- **Frontend**: Vue 3 + Nuxt 3 (SSG mode) + TypeScript
- **UI**: PrimeVue 4 with custom Autonomi theme + Tailwind CSS
- **State Management**: Pinia stores
- **Blockchain**: Wagmi/Viem for Web3 wallet integration (Arbitrum One)
- **Network**: Autonomi client for decentralized storage

### Core Architecture Patterns

1. **Frontend-Backend Communication**:
   - Commands are invoked via `@tauri-apps/api/core`
   - Events are emitted from backend and listened to in frontend
   - All Tauri commands in `src-tauri/src/lib.rs`

2. **State Management**:
   - Backend: Thread-safe `Mutex<AppStateInner>` in Tauri State
   - Frontend: Pinia stores in `/stores/` directory
   - Two-phase file loading for UI responsiveness

3. **Payment Flow**:
   - Backend requests payment quotes from network
   - Emits payment order to frontend via events
   - Frontend handles wallet signing and transaction
   - Backend waits for confirmation before proceeding

4. **File Operations**:
   - Vault system for private encrypted storage
   - Public file sharing capabilities
   - Directory upload with structure preservation
   - Signature-based access control

### Key Tauri Commands

Located in `src-tauri/src/lib.rs`:
- `app_data` / `app_data_store` - Settings management
- `upload_files` - Upload to vault with encryption
- `get_vault_structure` - Fast metadata retrieval
- `get_files_from_vault` - Full file data retrieval
- `download_private_file` / `download_public_file` - File downloads
- `send_payment_order_message` - Payment status updates

### Project Structure

```
Frontend:
├── pages/          # Route components (index, upload, wallet, settings, nodes)
├── components/     # Reusable UI components
├── stores/         # Pinia state management
├── composables/    # Vue composition functions
└── types/          # TypeScript definitions

Backend:
└── src-tauri/
    ├── src/
    │   ├── lib.rs  # Tauri command definitions
    │   └── ant/    # Core business logic
    │       ├── client.rs    # Network client
    │       ├── files.rs     # File operations
    │       ├── payments.rs  # Payment handling
    │       └── app_data.rs  # Settings persistence
    └── capabilities/        # Security permissions
```

### Development Workflow

1. Frontend changes are hot-reloaded via Vite
2. Backend changes require restart of `npm run tauri dev`
3. Use `invoke()` for backend commands, `listen()` for events
4. Check browser DevTools and terminal for debugging
5. File permissions are set to user-only (0o600) on Unix systems

### Special Considerations

- The app uses static site generation (SSG) - no server-side rendering
- Auto-updater is configured via GitHub releases
- Settings persist in platform-specific directories
- Network client initialization happens on first use
- Payment orders have a timeout mechanism
- Vault operations require signature verification