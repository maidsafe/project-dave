# Dave

Dave is a client app for managing your Vault on the Autonomi network.

## Features

- **File Uploading**: Upload files and folders publicly or privately to Autonomi.
- **Vault Management**: Manage your local and network Vault.
- **Web3 Wallet Integration**: Secure access and payments via Web3 wallet.

## Installation

Download the latest version from the [releases page](https://github.com/maidsafe/project-dave/releases):

- Windows (.msi installer)
- macOS (.dmg)
- Linux (.AppImage or .deb)

## Using Dave with Your Wallet

Dave uses Web3 wallets for secure Vault access and upload payments on the Arbitrum One network.

### Setting Up Your Wallet

1. **Install a Web3 Wallet**: Dave supports mobile wallets through WalletConnect
    - Popular options include MetaMask Mobile, Rainbow, or Trust Wallet

2. **Connect in Dave**:
    - Click the wallet button in Dave
    - Scan the QR code with your mobile wallet
    - Approve the connection

3. **Fund Your Wallet**: You'll need ETH and Autonomi tokens on Arbitrum One for upload payments

### Basic Usage

#### Uploading Files

1. Click the plus (+) button on the Files page
2. Select files or folders from your computer
3. Choose upload type:
    - **Private**: Encrypted files with data map kept locally (can be shared via datamap hex)
    - **Public**: Encrypted files with data map uploaded publicly for sharing via data address
4. Sign with your wallet when prompted
5. Confirm payment if required

## Developer Setup

For developers who want to run Dave from source:

### Prerequisites

- [Node.js](https://nodejs.org/) (v18 or later)
- [Rust](https://rustup.rs/) (latest stable)
- [pnpm](https://pnpm.io/) or npm

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/maidsafe/project-dave.git
   cd project-dave
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Set up environment variables:
   ```bash
   cp .env.example .env
   ```

4. Edit `.env` and configure the required variables:
   ```bash
   # Optional: Skip wallet signing during development
   VITE_DEV_VAULT_SIGNATURE=0x...

   # Required if using paymaster functionality
   VITE_PIMLICO_API_KEY=your_pimlico_api_key_here
   ```

### Running the App

```bash
# Development mode (starts both frontend and Tauri backend)
npm run tauri dev

# Frontend only (for UI development)
npm run dev
```

### Building

```bash
# Build the complete desktop application
npm run tauri build

# Build only the frontend
npm run generate
```

## Support

- Learn more at [autonomi.com](https://autonomi.com/)
- Join our [Discord](https://discord.com/invite/autonomi)
- Visit the [Forum](https://forum.autonomi.community/)