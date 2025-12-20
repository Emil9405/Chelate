[README.md](https://github.com/user-attachments/files/24272456/README.md)
# LIMS - Laboratory Information Management System

<div align="center">

![LIMS](https://img.shields.io/badge/LIMS-v0.1.0-blue)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![React](https://img.shields.io/badge/React-18+-61dafb)
![License](https://img.shields.io/badge/License-MIT-green)

**A modern, secure, and efficient Laboratory Information Management System**

[Features](#features) • [Quick Start](#quick-start) • [Documentation](#documentation) • [API](#api-documentation) • [Contributing](#contributing)

</div>

---

## 📋 Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Technology Stack](#technology-stack)
- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [API Documentation](#api-documentation)
- [Security](#security)
- [Development](#development)
- [Deployment](#deployment)
- [Contributing](#contributing)
- [License](#license)
- [Support](#support)

---

## 🔬 Overview

LIMS is a comprehensive Laboratory Information Management System designed for small to medium-sized chemical laboratories. It provides a complete solution for managing reagents, equipment, experiments, and laboratory operations with a focus on security, efficiency, and ease of use.

### Why LIMS?

- **🔒 Security First**: Built with security best practices, SQL injection protection, and JWT authentication
- **⚡ High Performance**: Rust backend ensures blazing-fast performance and memory safety
- **📱 Modern UI**: React-based responsive interface works seamlessly on desktop and mobile
- **🔍 Full-Text Search**: Fast search across reagents and experiments using SQLite FTS5
- **📊 Comprehensive Reports**: Generate detailed reports on inventory, experiments, and equipment
- **💾 Import/Export**: Support for Excel, CSV, and JSON data formats
- **🔐 Role-Based Access**: Fine-grained permissions for Admin, User, and Guest roles

---

## ✨ Features

### Core Functionality

#### 🧪 Reagent Management
- Track chemical reagents with detailed information (CAS number, formula, hazard class)
- Batch management with expiration tracking
- Automatic low-stock alerts
- Full audit trail of reagent usage
- Barcode/QR code support (future)

#### 🔬 Experiment Management
- Schedule and track laboratory experiments
- Link experiments to reagents and equipment
- Document experimental procedures and results
- Calendar view for experiment planning
- Collaborative experiment notes

#### 🛠️ Equipment Management
- Monitor laboratory equipment status
- Track maintenance schedules
- Equipment reservation system
- Calibration tracking
- Usage history and analytics

#### 👥 User Management
- Role-based access control (RBAC)
- Three user levels: Admin, User, Guest
- User activity logging
- Password management with bcrypt hashing
- JWT-based authentication with refresh tokens

#### 📈 Reporting & Analytics
- Reagent inventory reports
- Batch expiration reports
- Experiment history reports
- Equipment utilization reports
- Custom report generation
- Export to Excel, CSV, JSON

### Advanced Features

#### 🔍 Full-Text Search
- Fast search using SQLite FTS5
- Search across reagents, experiments, and equipment
- Autocomplete suggestions
- Advanced filtering

#### 📥 Import/Export
- Batch import from Excel/CSV
- Export data in multiple formats
- Template downloads for imports
- Data validation on import

#### 🔐 Security
- SQL injection protection through parameterized queries
- XSS protection
- CORS configuration
- Secure password hashing (bcrypt)
- JWT token rotation
- Field whitelisting
- Environment-based secrets

#### 🌐 API
- RESTful API
- Comprehensive endpoints
- JSON responses
- Authentication required
- Rate limiting (planned)

---

## 🛠️ Technology Stack

### Backend
- **Language**: Rust 1.70+
- **Framework**: Actix-web 4.4
- **Database**: SQLite with FTS5 support
- **ORM**: SQLx 0.7
- **Authentication**: JWT (jsonwebtoken 9.2)
- **Password Hashing**: bcrypt 0.15
- **Validation**: validator 0.18
- **Serialization**: serde 1.0

### Frontend
- **Framework**: React 18
- **Language**: JavaScript ES6+
- **Styling**: CSS3
- **HTTP Client**: Fetch API
- **State Management**: React Hooks

### Development Tools
- **Version Control**: Git
- **Package Manager**: Cargo (Rust), npm (Node.js)
- **Testing**: cargo test, Jest (planned)
- **Documentation**: Markdown

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    LIMS Application                     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌────────────────┐         ┌─────────────────────┐     │
│  │   Frontend     │ ◄─────► │      Backend        │     │
│  │   (React)      │  REST   │   (Actix-web)       │     │
│  │                │   API   │                     │     │
│  └────────────────┘         └─────────────────────┘     │
│                                      │                  │
│                                      ▼                  │
│                             ┌─────────────────┐         │
│                             │   SQLite DB     │         │
│                             │   + FTS5        │         │
│                             └─────────────────┘         │
│                                                         │
└─────────────────────────────────────────────────────────┘

Components:
┌──────────────────┐
│  Authentication  │ ── JWT tokens, bcrypt hashing
│  Module          │
└──────────────────┘

┌──────────────────┐
│  Query Builders  │ ── SQL injection protection
│  Module          │    Field whitelisting
└──────────────────┘

┌──────────────────┐
│  API Handlers    │ ── REST endpoints
│                  │    Request validation
└──────────────────┘

┌──────────────────┐
│  Data Models     │ ── Type-safe models
│                  │    Database schemas
└──────────────────┘
```

For detailed architecture documentation, see [docs/architecture/ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md)

---

## 🚀 Quick Start

### Prerequisites

- **Rust** 1.70 or higher ([Install Rust](https://rustup.rs/))
- **Node.js** 16 or higher ([Install Node.js](https://nodejs.org/))
- **SQLite** 3 ([Install SQLite](https://www.sqlite.org/download.html))

### Installation

1. **Clone the repository**
```bash
git clone https://github.com/Emil9405/LIMSgen.git
cd LIMSgen
```

2. **Set up backend**
```bash
# Create .env from template
cp .env.example .env

# Generate JWT secret
openssl rand -hex 32

# Edit .env and add the generated secret
nano .env  # or use any text editor

# Build and run
cargo build --release
cargo run
```

Backend will start on `http://127.0.0.1:8080`

3. **Set up frontend**
```bash
cd lims-frontend

# Install dependencies
npm install

# Start development server
npm start
```

Frontend will start on `http://localhost:3000`

4. **Login**
- Username: `admin`
- Password: Value from `DEFAULT_ADMIN_PASSWORD` in your `.env`

⚠️ **Important**: Change the admin password immediately after first login!

### Using Docker (Coming Soon)

```bash
docker-compose up -d
```

---

## ⚙️ Configuration

### Environment Variables

Create a `.env` file in the project root (copy from `.env.example`):

```env
# Database
DATABASE_URL=sqlite:lims.db

# Server
BIND_ADDRESS=127.0.0.1
LIMS_PORT=8080
LIMS_HOST=127.0.0.1

# Authentication
JWT_SECRET=<generate-with-openssl-rand-hex-32>
DEFAULT_ADMIN_PASSWORD=AdminPass123!

# CORS
ALLOWED_ORIGINS=*  # Use specific origins in production

# Logging
RUST_LOG=info
LOG_LEVEL=info

# Environment
LIMS_ENV=development

# Features
HOT_RELOAD_ENABLED=true
FRONTEND_BUILD_DIR=../lims-frontend/build
```

### Security Configuration

For production:
1. Use a strong JWT_SECRET (64+ characters)
2. Set specific ALLOWED_ORIGINS (not *)
3. Use HTTPS
4. Change DEFAULT_ADMIN_PASSWORD
5. Set LIMS_ENV=production

See [Security Guide](docs/guides/SECURITY.md) for details.

---

## 📖 Usage

### Basic Workflow

1. **Login** as admin
2. **Create users** with appropriate roles
3. **Add reagents** to the system
4. **Create batches** for reagents
5. **Add equipment** to track
6. **Schedule experiments**
7. **Generate reports**

### User Roles

- **Admin**: Full system access, user management
- **User**: Create/edit reagents, experiments, equipment
- **Guest**: Read-only access

### Common Tasks

#### Adding a Reagent
1. Navigate to "Reagents"
2. Click "Add Reagent"
3. Fill in required fields (name, CAS number, formula)
4. Specify hazard class and storage conditions
5. Save

#### Creating a Batch
1. Go to reagent details
2. Click "Add Batch"
3. Enter quantity, lot number, expiration date
4. Specify location and supplier
5. Save

#### Scheduling an Experiment
1. Navigate to "Experiments"
2. Click "Schedule Experiment"
3. Select date/time and room
4. Link required reagents and equipment
5. Add description and procedure
6. Save

For detailed usage instructions, see [User Guide](docs/guides/USER_GUIDE.md)

---

## 🔌 API Documentation

### Base URL
```
http://127.0.0.1:8080/api
```

### Authentication

All API endpoints (except login) require JWT authentication:

```bash
# Login
POST /api/auth/login
{
  "username": "admin",
  "password": "password"
}

# Response
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...",
  "user": { ... }
}

# Use token in subsequent requests
Authorization: Bearer <access_token>
```

### Main Endpoints

#### Authentication
- `POST /api/auth/login` - User login
- `POST /api/auth/refresh` - Refresh access token
- `POST /api/auth/logout` - User logout

#### Users (Admin only)
- `GET /api/users` - List all users
- `POST /api/users` - Create user
- `GET /api/users/:id` - Get user details
- `PUT /api/users/:id` - Update user
- `DELETE /api/users/:id` - Delete user

#### Reagents
- `GET /api/reagents` - List reagents (with pagination & filters)
- `POST /api/reagents` - Create reagent
- `GET /api/reagents/:id` - Get reagent details
- `PUT /api/reagents/:id` - Update reagent
- `DELETE /api/reagents/:id` - Delete reagent
- `GET /api/reagents/search` - Full-text search

#### Batches
- `GET /api/batches` - List batches
- `POST /api/batches` - Create batch
- `GET /api/batches/:id` - Get batch details
- `PUT /api/batches/:id` - Update batch
- `DELETE /api/batches/:id` - Delete batch
- `GET /api/batches/expiring` - Get expiring batches

#### Experiments
- `GET /api/experiments` - List experiments
- `POST /api/experiments` - Create experiment
- `GET /api/experiments/:id` - Get experiment details
- `PUT /api/experiments/:id` - Update experiment
- `DELETE /api/experiments/:id` - Delete experiment
- `GET /api/experiments/calendar` - Calendar view

#### Equipment
- `GET /api/equipment` - List equipment
- `POST /api/equipment` - Create equipment
- `GET /api/equipment/:id` - Get equipment details
- `PUT /api/equipment/:id` - Update equipment
- `DELETE /api/equipment/:id` - Delete equipment

#### Reports
- `GET /api/reports/reagents` - Reagents report
- `GET /api/reports/batches` - Batches report
- `GET /api/reports/experiments` - Experiments report
- `GET /api/reports/export` - Export data

For complete API documentation, see [API Reference](docs/api/API_REFERENCE.md)

---

## 🔒 Security

### Built-in Security Features

1. **SQL Injection Protection**
   - All queries use parameterized statements
   - Field whitelisting
   - Type-safe query builders

2. **Authentication & Authorization**
   - JWT-based authentication
   - Refresh token rotation
   - Role-based access control
   - bcrypt password hashing

3. **Input Validation**
   - All inputs validated
   - Type checking
   - Length restrictions
   - Sanitization

4. **CORS Protection**
   - Configurable allowed origins
   - Proper headers

5. **Environment Security**
   - Secrets in environment variables
   - No hardcoded credentials
   - .env excluded from git

### Security Best Practices

- Use HTTPS in production
- Rotate JWT secrets regularly
- Keep dependencies updated
- Monitor logs for suspicious activity
- Regular security audits

See [Security Guide](docs/guides/SECURITY.md) for details.

---

## 👨‍💻 Development

### Project Structure

```
lims/
├── src/                    # Rust backend source
│   ├── main.rs            # Entry point
│   ├── models.rs          # Data models
│   ├── db.rs              # Database
│   ├── auth.rs            # Authentication
│   ├── *_handlers.rs      # API handlers
│   └── query_builders/    # SQL query builders
├── lims-frontend/         # React frontend
│   ├── src/
│   │   ├── components/    # React components
│   │   ├── App.js        # Main component
│   │   └── api.js        # API client
│   └── public/
├── Cargo.toml             # Rust dependencies
├── .env.example           # Environment template
└── README.md             # This file
```

### Building from Source

```bash
# Backend
cargo build --release
cargo test

# Frontend
cd lims-frontend
npm install
npm run build
```

### Running Tests

```bash
# Backend tests
cargo test

# Frontend tests (if available)
cd lims-frontend
npm test
```

### Code Style

- **Rust**: Follow Rust conventions, use `cargo fmt` and `cargo clippy`
- **JavaScript**: Use ESLint, Prettier
- **Commits**: Follow conventional commits

See [Developer Guide](docs/guides/DEVELOPER_GUIDE.md) for details.

---

## 🚀 Deployment

### Production Build

```bash
# Backend
cargo build --release

# Frontend
cd lims-frontend
npm run build
```

### Systemd Service (Linux)

Create `/etc/systemd/system/lims.service`:

```ini
[Unit]
Description=LIMS Server
After=network.target

[Service]
Type=simple
User=lims
WorkingDirectory=/opt/lims
ExecStart=/opt/lims/target/release/lims
Restart=always

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable lims
sudo systemctl start lims
```

### Docker Deployment (Coming Soon)

```bash
docker build -t lims .
docker run -d -p 8080:8080 lims
```

See [Deployment Guide](docs/deployment/DEPLOYMENT.md) for details.

---

## 🤝 Contributing

We welcome contributions! Here's how you can help:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Contribution Guidelines

- Write tests for new features
- Update documentation
- Follow code style guidelines
- Keep commits focused and atomic

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 💬 Support

### Getting Help

- **Documentation**: Check the [docs](docs/) folder
- **Issues**: Open an issue on GitHub
- **Discussions**: Use GitHub Discussions

### Reporting Bugs

Please include:
- LIMS version
- Operating system
- Steps to reproduce
- Expected vs actual behavior
- Error messages/logs

### Feature Requests

Open an issue with:
- Clear description
- Use case
- Expected behavior

---

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- Powered by [Actix-web](https://actix.rs/)
- UI with [React](https://reactjs.org/)
- Database: [SQLite](https://www.sqlite.org/)

---

## 📚 Additional Documentation

- [User Guide](docs/guides/USER_GUIDE.md)
- [Developer Guide](docs/guides/DEVELOPER_GUIDE.md)
- [API Reference](docs/api/API_REFERENCE.md)
- [Architecture](docs/architecture/ARCHITECTURE.md)
- [Security Guide](docs/guides/SECURITY.md)
- [Deployment Guide](docs/deployment/DEPLOYMENT.md)
- [FAQ](docs/FAQ.md)

---

<div align="center">

**Made with ❤️ for laboratory efficiency**

[⬆ Back to Top](#lims---laboratory-information-management-system)

</div>
