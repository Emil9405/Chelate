# LIMS - Laboratory Information Management System

[![Rust](https://img.shields.io/badge/Rust-1.75+-blue.svg)](https://www.rust-lang.org/)
[![Actix Web](https://img.shields.io/badge/Actix%20Web-4.0-red.svg)](https://actix.rs/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

LIMS is a lightweight, secure web-based Laboratory Information Management System built with Rust. It allows lab managers and researchers to track reagents, manage inventory batches, generate reports, and handle user permissions. The system uses SQLite for persistence, JWT for authentication, and a simple HTML/JS frontend for the UI.

## Features

- **User Management**: Role-based access (Admin, Researcher, Viewer) with JWT authentication, registration, and password management.
- **Reagent Tracking**: CRUD operations for reagents with details like formula, CAS number, manufacturer, and status (Active/Inactive/Discontinued).
- **Batch Management**: Track inventory batches with quantity, expiry dates, locations, and usage logging.
- **Reports & Analytics**: Dashboard stats, low-stock alerts, and expiring items reports.
- **Audit Logging**: Automatic tracking of changes for compliance.
- **Security**: Rate limiting, CORS, input validation, and HTTPS support.
- **Monitoring**: Health checks, metrics endpoint, and background maintenance tasks.

## Tech Stack

- **Backend**: Rust with Actix-web (web framework), SQLx (SQLite ORM), Serde/Validator (serialization/validation).
- **Auth**: JWT (jsonwebtoken), Bcrypt for passwords.
- **Frontend**: Static HTML/JS (included in `web_interface.html`).
- **Database**: SQLite (with WAL mode for concurrency).
- **Other**: Chrono (dates), Uuid (IDs), Tracing (logging).

## Quick Start

### Prerequisites

- Rust (1.75+): Install via [rustup](https://rustup.rs/).
- Git: For cloning the repo.

### Installation

1. Clone the repository:
   ```
   git clone https://github.com/your-username/lims-rust.git
   cd lims-rust
   ```

2. Build and run:
   ```
   cargo build
   cargo run
   ```

   The server will start on `http://127.0.0.1:8080`. Open `http://localhost:8080` in your browser.

3. **First-time setup**: A default admin user is created automatically:
   - Username: `admin`
   - Password: `admin123456`
   - **Change this immediately!**

   Access the UI to log in and manage users/reagents.

### Environment Configuration

Copy `.env.example` to `.env` and customize:
```
DATABASE_URL=sqlite:lims.db
JWT_SECRET=your-super-secret-key-here  # At least 32 chars
LIMS_PORT=8080
ALLOW_SELF_REGISTRATION=false
```

## Usage

- **Login**: POST `/auth/login` with JSON `{ "username": "...", "password": "..." }`.
- **Dashboard**: GET `/api/v1/dashboard/stats` (requires auth).
- **Reagents**: 
  - List: GET `/api/v1/reagents?page=1&per_page=20`
  - Create: POST `/api/v1/reagents` with JSON body (e.g., `{ "name": "Sodium Chloride", "formula": "NaCl" }`).
- **Batches**: 
  - Create for reagent: POST `/api/v1/reagents/{id}/batches`
  - Use: POST `/api/v1/batches/{reagent_id}/{batch_id}/use`
- **Reports**:
  - Low Stock: GET `/api/v1/batches/low-stock?threshold=10`
  - Expiring: GET `/api/v1/batches/expiring?days=30`

Full API docs: Check the code in `src/handlers.rs` or use Swagger (add later).

## Project Structure

```
lims-rust/
├── Cargo.toml          # Dependencies
├── web_interface.html  # Frontend UI
├── src/
│   ├── main.rs         # Entry point
│   ├── config.rs       # App config
│   ├── auth.rs         # Auth logic
│   ├── auth_handlers.rs # Auth routes
│   ├── db.rs           # DB migrations
│   ├── error.rs        # Error handling
│   ├── handlers.rs     # API handlers
│   ├── models.rs       # Data models
│   └── monitoring.rs   # Health/metrics
└── README.md           # This file
```

## Contributing

1. Fork the repo and create a feature branch: `git checkout -b feature/amazing-feature`.
2. Commit changes: `git commit -m 'Add amazing feature'`.
3. Push: `git push origin feature/amazing-feature`.
4. Open a Pull Request.

Report bugs or suggest features via [Issues](https://github.com/your-username/lims-rust/issues).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with love using Rust's ecosystem.
- Inspired by modern lab management needs.

---

*Questions? Open an issue or reach out!*
