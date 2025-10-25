# SuperApp Project Overview

This is a super app project consisting of three main components: **backend**, **frontend**, and **mobile**.

## Project Structure

The project is organized into three main application directories, each containing core functionality and dynamic modules:

```
project/
├── backend/                 # Backend application (Go)
│   ├── core/                # Core backend functionalities
│   │   ├── src/             # Source code
│   │   ├── build/           # Build artifacts
│   │   ├── test/            # Test files
│   │   ├── README.md        # Core documentation
│   │   └── [config files]   # Go-specific configurations
│   └── modules/             # Backend modules (plugins)
│       └── [module-name]/   # Individual module folders
│           ├── src/
│           ├── build/
│           ├── test/
│           └── README.md
├── frontend/                # Frontend application (React)
│   ├── core/                # Core frontend functionalities
│   │   ├── src/             # Source code
│   │   ├── build/           # Build artifacts
│   │   ├── test/            # Test files
│   │   ├── README.md        # Core documentation
│   │   └── [config files]   # React-specific configurations
│   └── modules/             # Frontend modules (plugins)
│       └── [module-name]/   # Individual module folders
│           ├── src/
│           ├── build/
│           ├── test/
│           └── README.md
├── mobile/                  # Mobile application (React Native)
│   ├── core/                # Core mobile functionalities
│   │   ├── src/             # Source code
│   │   ├── build/           # Build artifacts
│   │   ├── test/            # Test files
│   │   ├── README.md        # Core documentation
│   │   └── [config files]   # React Native-specific configurations
│   └── modules/             # Mobile modules (plugins)
│       └── [module-name]/   # Individual module folders
│           ├── src/
│           ├── build/
│           ├── test/
│           └── README.md
└── doc/                     # Shared documentation
```

### Module System
- Modules are dynamically loadable plugins that can be added/removed at runtime
- Each module must have secure verification for dynamic loading
- Modules follow the same structure as their corresponding core applications
- New modules are added as folders under the respective `modules/` directory:
  - Backend modules: `backend/modules/[module-name]/`
  - Frontend modules: `frontend/modules/[module-name]/`
  - Mobile modules: `mobile/modules/[module-name]/`
- Cross-platform modules will have components in all three directories
- Each module component follows its platform's technology stack and conventions

# Core Applications


## Backend (Go)

**Technology Stack**: Go, PostgreSQL, Redis, Kafka, Prometheus, Grafana, Prometheus Alertmanager

### Core Features
- **API Gateway**: Centralized request handling for mobile and frontend clients
- **Authentication & Authorization**: Impl
- **Database**: PostgreSQL as primary data store
- **Dynamic Module Loading**: Secure runtime plugin system
- **Real-time Communication**: HTTP2 SSE for notifications and updates
- **Queue System**: Asynchronous request processing (available to modules)

### Security & Best Practices
- API endpoint documentation and validation
- Rate limiting implementation
- Security best practices enforcement
- Environment-based configuration with `SUPERAPP_BACKEND_` prefix

### Deployment
- Dockerized with multi-stage build
- Optimized caching strategies


## Frontend (React)

**Technology Stack**: React, ShadCN UI

### User Interface

### Deployment
- Dockerized for containerized deployment

## Mobile (React Native)

**Technology Stack**: React Native, Tamagui UI

### User Experience
- Tamagui-based responsive UI design
- Intuitive alert creation workflow
- Real-time feedback on alert submission

# Deployment & Infrastructure

## Docker Configuration
- Individual Dockerfiles for backend and frontend
- Multi-stage builds for optimization
- Development and production configurations
- Services for infra like DB, Queue and all other third party container will use an existing project: helvetia-compose. Don't try to do anything for these, just inform me and get connection information

## Docker Compose
- Orchestrated deployment of backend, frontend, and database
- Environment variable management
- Service networking and dependencies
- Volume management for persistent data

## Environment Variables
- All backend configurations prefixed with `SUPERAPP_BACKEND_`
- Secure credential management
- Environment-specific settings

# Modules (Plugin System)

*Dynamic modules can be added here as the project evolves. Each module will follow the same structure as core applications and provide specific functionality that can be loaded/unloaded at runtime.*
