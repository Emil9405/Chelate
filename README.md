# Laboratory Information Management System (LIMS)

A comprehensive, high-performance Laboratory Information Management System built with Rust and React, designed for managing chemical inventories, equipment, and laboratory operations at scale.

## 🎯 Overview

LIMS is a modern web-based laboratory management system optimized for handling large datasets (100,000+ records) with advanced features including full-text search, role-based access control, and comprehensive audit logging. Built specifically for academic and research laboratories with a focus on chemical inventory management and equipment tracking.

## ✨ Key Features

### Chemical Inventory Management
- **Advanced Search**: Full-text search with FTS5 implementation for instant results across large datasets
- **Batch Operations**: Efficient bulk import/export of chemical data
- **Smart Filtering**: Multi-criteria filtering with pagination for optimal performance
- **GHS Classification**: Integrated hazard pictogram and safety information management
- **Location Tracking**: Hierarchical storage location system

### Equipment Management
- **Asset Tracking**: Complete lifecycle management of laboratory equipment
- **Maintenance Scheduling**: Automated maintenance reminders and calibration tracking
- **Usage Logs**: Comprehensive equipment usage history with audit trails

### Security & Access Control
- **JWT Authentication**: Secure authentication with automatic token rotation
- **Role-Based Permissions**: Granular access control (Admin, Manager, User, Guest roles)
- **Audit Logging**: Complete activity tracking for compliance and security
- **SQL Injection Protection**: Parameterized queries throughout

### Performance Optimization
- **Efficient Pagination**: Cursor-based pagination for handling large result sets
- **Database Indexing**: Optimized SQLite indexes for sub-second query performance
- **Lazy Loading**: Frontend optimization for smooth UX with large datasets
- **Query Optimization**: Advanced SQL techniques including CTEs and window functions

## 🛠️ Technology Stack

### Backend
- **Framework**: Actix-web 4.x (Rust)
- **Database**: SQLite with FTS5 full-text search
- **Authentication**: JWT with RS256 signing
- **ORM**: SQLx for compile-time verified queries
- **Serialization**: Serde for JSON handling

### Frontend
- **Framework**: React 18.x with TypeScript
- **State Management**: React Context API / Redux
- **HTTP Client**: Axios
- **UI Components**: Custom component library
- **Build Tool**: Vite / Webpack

### Development Tools
- **Testing**: Rust's built-in test framework, Jest for frontend
- **Documentation**: rustdoc, JSDoc
- **Linting**: Clippy, ESLint
- **Formatting**: rustfmt, Prettier

## 📋 Prerequisites

- Rust 1.70+ (latest stable recommended)
- Node.js 18+ and npm/yarn
- SQLite 3.35+ (for FTS5 support)
- Git

## 🚀 Installation

### 1. Clone the Repository

```bash
git clone https://github.com/Emil9405/LIMSgen

```

### 2. Backend Setup

```bash
cd backend

# Install Rust dependencies
cargo build --release

# Set up environment variables
cp .env.example .env
# Edit .env with your configuration

# Initialize database
cargo run --bin init_db

# Generate JWT keys
cargo run --bin generate_keys
```

### 3. Frontend Setup

```bash
cd frontend

# Install Node dependencies
npm install

# Configure API endpoint
cp .env.example .env
# Edit .env with your backend URL
```

## 🎮 Running the Application

### Development Mode

**Terminal 1 - Backend:**
```bash
cd backend
cargo run
# Backend runs on http://localhost:8080
```

**Terminal 2 - Frontend:**
```bash
cd frontend
npm run dev
# Frontend runs on http://localhost:3000
```

### Production Build

**Backend:**
```bash
cd backend
cargo build --release
./target/release/lims-server
```

**Frontend:**
```bash
cd frontend
npm run build
# Serve the dist/ folder with your web server
```

## 📁 Project Structure

```
lims/
├── backend/
│   ├── src/
│   │   ├── main.rs              # Application entry point
│   │   ├── config.rs            # Configuration management
│   │   ├── models/              # Data models
│   │   │   ├── chemical.rs
│   │   │   ├── equipment.rs
│   │   │   └── user.rs
│   │   ├── handlers/            # HTTP request handlers
│   │   │   ├── auth.rs
│   │   │   ├── chemicals.rs
│   │   │   └── equipment.rs
│   │   ├── database/            # Database layer
│   │   │   ├── pool.rs
│   │   │   ├── migrations/
│   │   │   └── queries.rs
│   │   ├── middleware/          # Authentication, logging
│   │   │   ├── auth.rs
│   │   │   └── audit.rs
│   │   └── utils/               # Helper functions
│   ├── Cargo.toml
│   └── .env
├── frontend/
│   ├── src/
│   │   ├── App.tsx
│   │   ├── components/          # React components
│   │   │   ├── chemicals/
│   │   │   ├── equipment/
│   │   │   └── common/
│   │   ├── services/            # API services
│   │   │   ├── api.ts
│   │   │   └── auth.ts
│   │   ├── hooks/               # Custom React hooks
│   │   ├── types/               # TypeScript types
│   │   └── utils/
│   ├── package.json
│   └── .env
├── docs/                        # Additional documentation
├── tests/                       # Integration tests
└── README.md
```

## 🔌 API Documentation

### Authentication

#### Login
```http
POST /api/auth/login
Content-Type: application/json

{
  "username": "user@example.com",
  "password": "password"
}

Response:
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...",
  "expires_in": 3600
}
```

#### Refresh Token
```http
POST /api/auth/refresh
Authorization: Bearer {refresh_token}

Response:
{
  "access_token": "eyJ...",
  "expires_in": 3600
}
```

### Chemicals

#### Search Chemicals
```http
GET /api/chemicals/search?q=acetone&page=1&limit=50
Authorization: Bearer {access_token}

Response:
{
  "results": [...],
  "total": 1234,
  "page": 1,
  "total_pages": 25
}
```

#### Get Chemical by ID
```http
GET /api/chemicals/{id}
Authorization: Bearer {access_token}
```

#### Create Chemical
```http
POST /api/chemicals
Authorization: Bearer {access_token}
Content-Type: application/json

{
  "name": "Acetone",
  "cas": "67-64-1",
  "formula": "C3H6O",
  "quantity": 1.0,
  "unit": "L",
  "location": "Cabinet A, Shelf 2",
  "supplier": "Sigma-Aldrich",
  "catalog_number": "179124"
}
```

#### Update Chemical
```http
PUT /api/chemicals/{id}
Authorization: Bearer {access_token}
Content-Type: application/json
```

#### Delete Chemical
```http
DELETE /api/chemicals/{id}
Authorization: Bearer {access_token}
```

### Equipment

Similar endpoints structure for equipment management:
- `GET /api/equipment`
- `GET /api/equipment/{id}`
- `POST /api/equipment`
- `PUT /api/equipment/{id}`
- `DELETE /api/equipment/{id}`

## 🔐 Security Features

### Authentication
- JWT tokens with RS256 asymmetric signing
- Automatic token rotation and refresh
- Secure password hashing with Argon2

### Authorization
- Role-based access control (RBAC)
- Granular permissions per resource
- Audit logging of all sensitive operations

### Data Protection
- SQL injection prevention through parameterized queries
- XSS protection in frontend
- HTTPS enforcement in production
- Rate limiting on API endpoints

## 🗄️ Database Schema

### Key Tables

**chemicals**
- `id`: Primary key
- `name`: Chemical name
- `cas`: CAS Registry Number
- `formula`: Molecular formula
- `quantity`: Available quantity
- `unit`: Unit of measurement
- `location`: Storage location
- `created_at`, `updated_at`: Timestamps

**equipment**
- `id`: Primary key
- `name`: Equipment name
- `type`: Equipment category
- `serial_number`: Unique identifier
- `location`: Current location
- `status`: Operational status
- `last_maintenance`: Maintenance date
- `created_at`, `updated_at`: Timestamps

**users**
- `id`: Primary key
- `username`: Email/username
- `password_hash`: Argon2 hash
- `role`: User role
- `created_at`, `updated_at`: Timestamps

**audit_log**
- `id`: Primary key
- `user_id`: Foreign key to users
- `action`: Type of action
- `resource`: Affected resource
- `timestamp`: When action occurred
- `details`: JSON details

## 🧪 Testing

### Backend Tests
```bash
cd backend
cargo test
cargo test -- --nocapture  # With output
cargo test --release       # Release mode
```

### Frontend Tests
```bash
cd frontend
npm test
npm run test:coverage
```

### Integration Tests
```bash
# Run full test suite
cargo test --workspace
```

## 📊 Performance Considerations

### Database Optimization
- Indexed columns: `cas`, `name`, `location` for chemicals
- FTS5 virtual table for full-text search
- Query optimization with EXPLAIN QUERY PLAN
- Connection pooling (recommended: 5-10 connections)

### Frontend Optimization
- Virtual scrolling for large lists
- Debounced search inputs
- Lazy loading of images and heavy components
- Code splitting for faster initial load

### Recommended Limits
- Pagination: 50-100 items per page
- Search results: Up to 1000 results
- Batch operations: 500 records per batch
- File uploads: 10MB max per file

## 🐛 Troubleshooting

### Common Issues

**Database locked error**
```bash
# Increase busy timeout in config
database.busy_timeout = 5000
```

**JWT verification failed**
```bash
# Regenerate keys
cargo run --bin generate_keys
# Update .env with new keys
```

**Search not working**
```bash
# Rebuild FTS5 index
cargo run --bin rebuild_fts
```

**Frontend can't connect to backend**
```bash
# Check CORS settings in backend config
# Verify API_URL in frontend .env
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Style
- Follow Rust standard conventions (rustfmt)
- Use meaningful variable names
- Add comments for complex logic
- Write tests for new features

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 👨‍🔬 Author

**Emil** - Laboratory Chemist & Developer
- 🏫 French-Azerbaijani University (UFAZ)
- 🔬 Synthetic Organic Chemistry
- 💻 Rust, Python, JavaScript

## 🙏 Acknowledgments

- Actix-web team for excellent async web framework
- SQLite team for FTS5 implementation
- React team for powerful UI library
- All contributors and users of this system

## 📚 Additional Resources

- [API Documentation](docs/API.md) - Detailed API reference
- [Database Schema](docs/DATABASE.md) - Complete schema documentation
- [Development Guide](docs/DEVELOPMENT.md) - Setup for contributors
- [Deployment Guide](docs/DEPLOYMENT.md) - Production deployment instructions

## 🗺️ Roadmap

- [ ] Mobile app (React Native)
- [ ] Advanced reporting and analytics
- [ ] Integration with chemical suppliers APIs
- [ ] Barcode/QR code scanning
- [ ] Multi-language support
- [ ] Cloud backup and sync
- [ ] Advanced equipment maintenance scheduling
- [ ] Chemical expiration tracking and alerts

---

**Version**: 1.0.0  
**Last Updated**: January 2025  
**Status**: Active Development
