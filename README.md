# Project Dave

Project Dave is a web application built with Rust, Tauri, Vue.js, and Nuxt.js that demonstrates the core functionality of the Autonomi network. It provides a user-friendly interface for interacting with the Autonomi network to upload data, and view and download uploaded data.

## Getting Started

### Prerequisites

- Node.js 16.x or later
- npm package manager
- Rust 1.70.0 or later
- rustc (Rust compiler)
- cargo (Rust package manager)

### Installation

1. Clone the repository: `git clone https://github.com/maidsafe/project-dave.git`

2. Navigate into the top level of the repo: `cd project-dave`

3. Install Node.js dependencies: `npm install`

## Development Server

Start the application for development `npm run tauri dev`

## Contributing

We welcome contributions! Here's how you can help:

1. Fork the repository
2. Create your feature branch (git checkout -b feat/amazing-feature)
3. Commit your changes (git commit -m 'adds amazing feature'). <br>Please ensure that your commit messages clearly describe the changes you have made and use the [Conventional Commits](https://www.conventionalcommits.org/) specification.
4. Push to the branch (git push origin feat/amazing-feature)
5. Open a Pull Request to the `development` branch instead of the `main` branch

You can also visit the [Autonomi Website](https://autonomi.com/) if you would like to learn more and please join our
online community through our [Discord server](https://discord.com/invite/autonomi) or [forum](https://forum.autonomi.community/).

## Development Guidelines

1. Follow the existing code style and conventions
2. Write meaningful commit messages
3. Add tests for new features
4. Update documentation as needed

## Project Structure

```
project-dave
├─ README.md
├─ app.vue
├─ assets
│  ├─ abi
│  └─ css
├─ components
│  ├─ Common
│  ├─ Icon
├─ config
├─ lib
├─ nuxt.config.ts
├─ package-lock.json
├─ package.json
├─ pages
├─ plugins
├─ public
├─ src-tauri
│  ├─ Cargo.lock
│  ├─ Cargo.toml
│  ├─ capabilities
│  ├─ gen
│  │  └─ schemas
│  ├─ icons
│  │  ├─ android
│  │  └─ ios
│  ├─ src
│  │  ├─ ant
│  ├─ target
│  └─ tauri.conf.json
├─ stores
├─ types
├─ utils
└─ vite.config.ts
```

## License

This Safe Network repository is licensed under the General Public License (GPL), version
3 ([LICENSE](http://www.gnu.org/licenses/gpl-3.0.en.html)).
