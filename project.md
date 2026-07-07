# SuperApp Project Overview

This is a super app project consisting of three main components: **backend**, **frontend**, and **mobile**.

## Project Structure

The project is organized into three main application directories, each containing core functionality and dynamic modules:

```
project/
├── backend/                 # Backend application (Rust · loco.rs)
│   ├── core/                # Core backend functionalities
│   │   ├── src/             # Source code
│   │   ├── build/           # Build artifacts
│   │   ├── test/            # Test files
│   │   ├── README.md        # Core documentation
│   │   └── [config files]   # Cargo + loco config (Cargo.toml, config/*.yaml)
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


## Backend (Rust · loco.rs)

**Technology Stack**: Rust on the **loco.rs** framework (Axum HTTP + Tokio async runtime + **SeaORM** ORM), PostgreSQL, Redis, Kafka, Prometheus, Grafana, Prometheus Alertmanager

### Core Features
- **API Gateway**: Centralized request handling for mobile and frontend clients (loco controllers)
- **Authentication & Authorization**: SSO via **Rauthy** (OIDC IdP) using the `openidconnect` RP crate; policy-based authorization via **Cedar** (`cedar-policy`). loco's built-in JWT auth scaffolding is replaced by this stack.
  - **No Public Registration**: First user to access system becomes admin automatically (admin bootstrap)
  - **Admin-Only User Creation**: Only admin users can create new user accounts
  - **Role / policy-based access control**: Admin and regular user roles, enforced by Cedar policies
  - **Admin Role Management**: Admin can promote any user to admin status
  - **Token Management**: OIDC access/refresh tokens, with rotation on refresh
    - Short-lived access tokens for API requests
    - Long-lived refresh tokens for token renewal
    - Automatic token rotation on refresh
    - Secure refresh token storage with Redis
  - **API Security**: Protected endpoints with authorization middleware (Cedar)
  - **Service Authentication**: API key authentication for module-to-service communication
- **Database**: PostgreSQL as primary data store (via SeaORM entities + migrations)
- **Dynamic Module Loading**: Secure runtime plugin system (Rust dynamic-loading approach noted under Module Interface Contracts)
- **Real-time Communication**: HTTP2 SSE for notifications and updates
- **Queue System**: Asynchronous request processing via loco background workers / Kafka (available to modules)

### Security & Best Practices
- API endpoint documentation and validation
- Rate limiting implementation
- Security best practices enforcement
- Environment-based configuration with `SUPERAPP_BACKEND_` prefix (layered over loco `config/*.yaml`)

### Deployment
- Dockerized with multi-stage build
- Optimized caching strategies


## Frontend (React)

**Technology Stack**: React, ShadCN UI

### User Interface
- **Component Library**: ShadCN UI components with consistent theming
- **State Management**: React Context API with custom hooks
- **Routing**: React Router v6 for navigation with role-based route protection
- **Role-Based UI**: Interface adapts dynamically based on user role (Admin/User)
  - **Admin Interface**: User management, system configuration, module administration
  - **User Interface**: Standard application features with limited administrative access
- **Real-time Updates**: HTTP2 SSE integration for live data
- **Responsive Design**: Mobile-first approach with Tailwind CSS
- **Module Integration**: Dynamic component loading for frontend modules

### Deployment
- Dockerized with multi-stage build for production optimization
- Static asset optimization and bundling
- Environment-based configuration management
- CDN integration ready for static assets

## Mobile (React Native)

**Technology Stack**: React Native, Tamagui UI

### User Experience
- **UI Framework**: Tamagui-based responsive UI design
- **Role-Based Navigation**: App interface changes based on user role (Admin/User)
  - **Admin Experience**: User management screens, system settings, module controls
  - **User Experience**: Core app functionality with role-appropriate feature access
- **Authentication Flow**: Secure login with automatic role detection
- **Real-time Updates**: Live notifications and data synchronization
- **Module Integration**: Dynamic screen loading for mobile modules

# Technical Specifications

## API Patterns & Conventions

### RESTful Endpoint Structure
- **Base URL**: `/api/v1/`
- **Resource Naming**: Plural nouns (e.g., `/api/v1/users`, `/api/v1/modules`)
- **HTTP Methods**: GET (read), POST (create), PUT (update), DELETE (remove)
- **Nested Resources**: `/api/v1/users/{id}/roles`, `/api/v1/modules/{id}/config`

### Request/Response Formats
- **Content Type**: `application/json` for all requests/responses
- **Request Structure**:
  ```json
  {
    "data": { /* request payload */ },
    "metadata": { /* optional request metadata */ }
  }
  ```
- **Success Response Structure** (house envelope, `application/json`):
  ```json
  {
    "success": true,
    "data": { /* response payload */ },
    "message": "string",
    "pagination": { /* only for paginated responses */ }
  }
  ```
- **Error Response Structure** — **RFC 9457 Problem Details** (`Content-Type: application/problem+json`). The house envelope is **not** used for errors:
  ```json
  {
    "type": "https://superapp/errors/validation",
    "title": "Unprocessable Entity",
    "status": 422,
    "detail": "human-readable explanation specific to this occurrence",
    "instance": "/api/v1/users",
    "errors": [ { "pointer": "/email", "detail": "must be a valid email" } ]
  }
  ```
  - `type` is a URI identifying the problem class (defaults to `about:blank` when none applies); `title` is a stable human-readable summary of the `type`; `status` mirrors the HTTP status code; `detail`/`instance` are occurrence-specific.
  - `errors` is an RFC 9457 **extension member** carrying field-level validation failures (one entry per invalid field; `pointer` is a JSON Pointer into the request body). Extension members are permitted by the RFC.

### Error Handling Patterns
- **Error body format**: every non-2xx response is an **RFC 9457 Problem Details** document served as `application/problem+json` (see *Error Response Structure* above). Success responses use the house envelope; errors never do.
- **HTTP Status Codes**:
  - `200 OK`: Successful requests
  - `201 Created`: Successful resource creation
  - `400 Bad Request`: Invalid request data
  - `401 Unauthorized`: Authentication required
  - `403 Forbidden`: Insufficient permissions
  - `404 Not Found`: Resource not found
  - `422 Unprocessable Entity`: Validation errors
  - `500 Internal Server Error`: Server errors

### Authentication Headers
- **Access Token**: `Authorization: Bearer {access_token}`
- **API Key** (for modules): `X-API-Key: {api_key}`
- **User Role**: Included in JWT payload, validated server-side

## Database Schema Guidelines

### Table Naming Conventions
- **Tables**: Snake_case, plural (e.g., `users`, `user_roles`, `module_configs`)
- **Columns**: Snake_case (e.g., `user_id`, `created_at`, `is_active`)
- **Primary Keys**: `id` (UUID preferred) or `{table}_id`
- **Foreign Keys**: `{referenced_table}_id` (e.g., `user_id`, `module_id`)

### Standard Columns
- **Timestamps**: `created_at`, `updated_at` (automatically managed)
- **Soft Delete**: `deleted_at` (nullable timestamp)
- **Audit Trail**: `created_by`, `updated_by` (user IDs)
- **Status Fields**: `is_active`, `status` (enum types)

### Migration Strategies
- **Versioned Migrations**: Sequential numbering with timestamps
- **Rollback Support**: All migrations must be reversible
- **Module Migrations**: Isolated in module-specific migration paths
- **Data Seeding**: Separate seed files for initial data

## Inter-Service Communication

### Module-to-Core API Contracts
- **Module Registration**: POST `/api/v1/modules/register`
  ```json
  {
    "name": "module-name",
    "version": "1.0.0",
    "endpoints": ["array of exposed endpoints"],
    "permissions": ["required permissions"],
    "config_schema": { /* JSON schema for module config */ }
  }
  ```
- **Module Health Check**: GET `/api/v1/modules/{id}/health`
- **Module Configuration**: PUT `/api/v1/modules/{id}/config`

### Event System (Real-time Updates)
- **HTTP2 SSE Endpoints**: `/api/v1/events/stream`
- **Event Types**: `user.created`, `module.loaded`, `config.updated`
- **Event Format**:
  ```json
  {
    "type": "event.type",
    "data": { /* event payload */ },
    "timestamp": "ISO8601",
    "user_id": "target user or null for broadcast"
  }
  ```

### Message Queue Patterns (Kafka)
- **Topic Naming**: `superapp.{service}.{action}` (e.g., `superapp.user.created`)
- **Message Format**: JSON with metadata envelope
- **Consumer Groups**: One per service for load balancing
- **Error Handling**: Dead letter queue for failed messages

## Module Interface Contracts

### Required Module Contract (Backend)
Backend modules run as **out-of-process containers**; the core is a **gateway** that proxies to them. A module container exposes a service contract (HTTP/gRPC) built with the `superapp_module` SDK — it is *not* loaded in-process. Modules may be written in any language that fulfills the contract.
```rust
// The module container's entrypoint: build a module service with the SDK and serve it.
#[tokio::main]
async fn main() -> Result<(), ModuleError> {
    Module::builder()
        .name("my-module")
        .version("1.0.0")
        .permissions(["my-module:read"])            // registered as Cedar actions, enforced at the gateway
        .config_schema(include_str!("config.schema.json"))
        .route(Method::GET, "/my-module/items", list_items)  // proxied by the core gateway
        .on_init(|cfg| async move { /* setup */ Ok(()) })
        .on_shutdown(|| async move { /* cleanup */ Ok(()) })
        .health(|| async move { Health::Ok })        // GET /health, polled by the gateway
        .serve()                                     // listens on the module port; core proxies here
        .await
}
```
The SDK exposes a standard control surface the gateway relies on: `GET /health` (readiness/liveness), the declared routes (proxied), and a manifest (name/version/routes/permissions/config_schema, plus signatures).

### Required Module Exports (Frontend)
```javascript
export default {
  name: 'module-name',
  version: '1.0.0',
  routes: [/* React Router routes */],
  components: {/* Exported components */},
  permissions: [/* Required permissions */],
  initialize: (config) => Promise<void>,
  cleanup: () => Promise<void>
}
```

### Required Module Exports (Mobile)
```javascript
export default {
  name: 'module-name',
  version: '1.0.0',
  screens: {/* Navigation screens */},
  components: {/* Exported components */},
  permissions: [/* Required permissions */],
  initialize: (config) => Promise<void>,
  cleanup: () => Promise<void>
}
```

### Module Lifecycle (container)
- **Start**: the core launches the module container; the module runs `on_init(config)` and reports readiness
- **Ready**: the gateway proxies routes only after the container passes readiness
- **Stop**: the core stops the container; the module runs `on_shutdown` for cleanup
- **Health**: the gateway polls the container's `GET /health`; an unhealthy/crashed container is isolated without affecting the core (TR-05-008)
- *(Frontend/mobile modules remain in-process JS bundles loaded by their module hosts — `initialize`/`cleanup` hooks apply there.)*

### Configuration Interface
- **Schema Validation**: JSON Schema for module configuration
- **Environment Variables**: Module-specific env vars with `SUPERAPP_MODULE_{NAME}_` prefix
- **Runtime Config**: API endpoint for dynamic configuration updates

# Development Standards & Conventions

## Rust Backend Standards (loco.rs)

### Crate & Module Structure
- **Binary entry point**: `src/main.rs` + `src/app.rs` (loco `Hooks` impl wires the app)
- **Controllers**: `src/controllers/` — HTTP handlers (Axum), one module per resource
- **Models**: `src/models/` with SeaORM entities under `src/models/_entities/` (generated)
- **Migrations**: `migration/` sub-crate (SeaORM migrations)
- **Background work**: `src/workers/` (loco workers), `src/tasks/` (one-off tasks), `src/mailers/`
- **Shared library code**: `src/lib.rs` exposing reusable modules; cross-crate helpers live in a workspace crate
- **Module naming**: short, snake_case modules (e.g., `auth`, `user`, `config`)

### File Organization
```
backend/core/
├── src/
│   ├── main.rs              # Binary entry point
│   ├── app.rs               # loco App Hooks (routes, workers, boot)
│   ├── controllers/         # HTTP handlers (auth, user, module, ...)
│   ├── models/
│   │   └── _entities/       # SeaORM entities (generated)
│   ├── middleware/          # Tower/Axum middleware (authz, etc.)
│   ├── workers/             # Background workers
│   ├── tasks/               # CLI tasks (seed, maintenance)
│   ├── initializers/        # Custom boot initializers
│   └── common/              # Logging, validation, response helpers
├── migration/               # SeaORM migration crate
├── config/                  # loco config (development.yaml, production.yaml, test.yaml)
├── tests/                   # Integration tests
└── Cargo.toml
```

### Naming Conventions
- **Files / modules**: snake_case (e.g., `user_controller.rs`, `auth_middleware.rs`)
- **Types / traits / enums**: PascalCase (e.g., `UserService`, `AuthConfig`)
- **Functions / methods / variables**: snake_case (e.g., `user_id`, `config_path`)
- **Constants / statics**: SCREAMING_SNAKE_CASE (e.g., `DEFAULT_PORT`, `JWT_SECRET_KEY`)
- **Visibility**: `pub` only what must cross module boundaries; default to private

### Error Handling Patterns
```rust
// Domain error type with thiserror
#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("validation failed on {field}: {message}")]
    Validation { field: String, message: String },
    #[error(transparent)]
    Db(#[from] sea_orm::DbErr),
}

// Propagate with `?`; convert into loco's Error at the controller boundary
async fn create_user(/* ... */) -> Result<Response, loco_rs::Error> {
    let user = service.create(payload).await?; // UserError -> loco Error via From
    format::json(user)
}
```

### Testing Conventions
- **Unit tests**: `#[cfg(test)] mod tests` in the same file; `#[test]` / `#[tokio::test]` for async
- **Integration tests**: under `tests/`, using loco's test helpers (`boot_test`, `request`)
- **Snapshots**: `insta` for response/serialization snapshots where useful
- **Fixtures / seeding**: loco fixtures + `serial_test` for DB-touching tests
- **Mocking**: depend on traits; inject fakes for external services
- **Coverage**: minimum 80% on critical paths (e.g., `cargo llvm-cov`)

## React Frontend Standards

### Component Organization
```
frontend/core/src/
├── components/
│   ├── ui/           # Reusable UI components
│   ├── forms/        # Form components
│   ├── layout/       # Layout components
│   └── modules/      # Module-specific components
├── pages/            # Route components
├── hooks/            # Custom hooks
├── contexts/         # React contexts
├── utils/            # Utility functions
├── types/            # TypeScript type definitions
└── constants/        # Application constants
```

### Naming Conventions
- **Components**: PascalCase (e.g., `UserProfile.tsx`, `LoginForm.tsx`)
- **Hooks**: camelCase starting with `use` (e.g., `useAuth.ts`, `useUserData.ts`)
- **Utilities**: camelCase (e.g., `formatDate.ts`, `validateEmail.ts`)
- **Constants**: UPPER_SNAKE_CASE (e.g., `API_BASE_URL`, `DEFAULT_THEME`)
- **Types**: PascalCase with descriptive suffixes (e.g., `UserData`, `ApiResponse`)

### Component Structure
```tsx
// Component with proper typing and organization
import React from 'react';
import { cn } from '@/lib/utils';

interface UserProfileProps {
  userId: string;
  className?: string;
}

export const UserProfile: React.FC<UserProfileProps> = ({ 
  userId, 
  className 
}) => {
  // Hooks first
  const { user, loading } = useUser(userId);
  
  // Early returns
  if (loading) return <LoadingSpinner />;
  if (!user) return <UserNotFound />;
  
  // Main render
  return (
    <div className={cn('user-profile', className)}>
      {/* Component content */}
    </div>
  );
};
```

### Hook Usage Patterns
- **Custom Hooks**: Extract reusable stateful logic
- **Context Usage**: Use `useContext` with proper error boundaries
- **Effect Cleanup**: Always clean up subscriptions and timeouts
- **Dependency Arrays**: Be explicit with useEffect dependencies

### Import/Export Conventions
```tsx
// External imports first
import React from 'react';
import { useQuery } from '@tanstack/react-query';

// Internal imports second
import { Button } from '@/components/ui/button';
import { useAuth } from '@/hooks/useAuth';
import { UserData } from '@/types/user';

// Named exports preferred
export { UserProfile } from './UserProfile';
export { UserList } from './UserList';
```

## React Native Mobile Standards

### Screen Organization
```
mobile/core/src/
├── screens/          # Screen components
│   ├── auth/         # Authentication screens
│   ├── user/         # User management screens
│   └── settings/     # Settings screens
├── components/       # Reusable components
├── navigation/       # Navigation configuration
├── hooks/           # Custom hooks
├── contexts/        # React contexts
├── utils/           # Utility functions
├── types/           # TypeScript definitions
└── constants/       # Application constants
```

### Navigation Patterns
```tsx
// Stack navigator structure
const AppStack = () => {
  return (
    <Stack.Navigator screenOptions={defaultScreenOptions}>
      <Stack.Screen 
        name="Home" 
        component={HomeScreen}
        options={{ title: 'Dashboard' }}
      />
    </Stack.Navigator>
  );
};

// Screen component structure
const HomeScreen: React.FC<StackScreenProps<'Home'>> = ({ 
  navigation, 
  route 
}) => {
  // Screen logic
};
```

### Platform-Specific Code
```tsx
// Platform-specific styling
const styles = StyleSheet.create({
  container: {
    flex: 1,
    ...Platform.select({
      ios: {
        paddingTop: 20,
      },
      android: {
        paddingTop: 0,
      },
    }),
  },
});

// Platform-specific file extensions
// Button.ios.tsx, Button.android.tsx, Button.tsx (fallback)
```

### Component Naming
- **Screens**: PascalCase with "Screen" suffix (e.g., `LoginScreen.tsx`)
- **Components**: PascalCase (e.g., `UserCard.tsx`, `LoadingSpinner.tsx`)
- **Navigation**: PascalCase with context (e.g., `AuthNavigator.tsx`)

## Cross-Platform Standards

### File Naming Conventions
- **Configuration**: kebab-case (e.g., `docker-compose.yml`, `tsconfig.json`)
- **Documentation**: kebab-case (e.g., `api-reference.md`, `deployment-guide.md`)
- **Scripts**: kebab-case (e.g., `build-docker.sh`, `run-tests.sh`)
- **Environment Files**: `.env`, `.env.local`, `.env.production`

### Documentation Standards
- **README Files**: Include purpose, setup, usage, and contribution guidelines
- **Code Comments**: Explain "why" not "what"
- **API Documentation**: Use OpenAPI/Swagger for backend APIs
- **Inline Documentation**: JSDoc for JS/TS functions, rustdoc (`///`) for Rust items

### Git Commit Conventions
- **Format**: `type(scope): description`
- **Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`
- **Examples**:
  - `feat(auth): add refresh token rotation`
  - `fix(api): handle null user data in response`
  - `docs(readme): update installation instructions`

### Environment Variable Standards
- **Backend**: `SUPERAPP_BACKEND_*` (e.g., `SUPERAPP_BACKEND_DB_URL`)
- **Frontend**: `VITE_*` (e.g., `VITE_API_BASE_URL`)
- **Mobile**: `EXPO_PUBLIC_*` (e.g., `EXPO_PUBLIC_API_URL`)
- **Modules**: `SUPERAPP_MODULE_{NAME}_*` (e.g., `SUPERAPP_MODULE_AUTH_SECRET`)

### Code Formatting
- **Rust**: `cargo fmt` (rustfmt) for formatting, `cargo clippy` (deny warnings) for linting
- **TypeScript/JavaScript**: Prettier with consistent configuration
- **Line Length**: Maximum 100 characters
- **Indentation**: 2 spaces for JS/TS, 4 spaces for Rust (rustfmt default)
- **Trailing Commas**: Always in multi-line structures

# Development Workflow

## Build Commands

### Backend (Rust · loco.rs)
```bash
# Development
cd backend/core
cargo fetch                    # Fetch dependencies
cargo loco start               # Run development server
cargo build                    # Build binary

# With hot reload (cargo-watch)
cargo watch -x "loco start"   # Auto-reload on file changes

# Production build
cargo build --release          # Optimized binary at target/release/

# Module development (module = a containerized service)
cd backend/modules/[module-name]
cargo run                      # run the module service locally
docker build -t [module-name] . # build the module's OCI image (see Module Interface Contracts)
```

### Frontend (React)
```bash
# Development
cd frontend/core
npm install                   # Install dependencies
npm run dev                   # Start development server
npm run build                 # Production build
npm run preview              # Preview production build

# Linting and formatting
npm run lint                  # ESLint check
npm run lint:fix             # Fix ESLint issues
npm run format               # Prettier formatting

# Module development
cd frontend/modules/[module-name]
npm install
npm run build                # Build module for integration
```

### Mobile (React Native)
```bash
# Development setup
cd mobile/core
npm install                   # Install dependencies
npx expo install             # Install Expo dependencies

# Development servers
npx expo start               # Start Expo development server
npx expo start --ios         # iOS simulator
npx expo start --android     # Android emulator
npx expo start --web         # Web development

# Build for production
npx expo build:ios           # iOS build
npx expo build:android       # Android build
eas build --platform all     # Using EAS Build

# Module development
cd mobile/modules/[module-name]
npm install
npm run build
```

## Testing Strategies

### Backend Testing
```bash
# Unit + integration tests
cargo test                    # Run all tests
cargo test auth::             # Run a specific module's tests
cargo test --test integration # Run an integration test target

# Benchmarks (criterion)
cargo bench

# Coverage report
cargo llvm-cov --html         # HTML coverage report
```

### Frontend Testing
```bash
# Unit and component tests
npm run test                  # Run Jest tests
npm run test:watch           # Watch mode
npm run test:coverage        # Coverage report

# E2E tests with Playwright
npm run test:e2e             # Run E2E tests
npm run test:e2e:ui          # Run with UI

# Component testing
npm run test:component       # Isolated component tests
```

### Mobile Testing
```bash
# Unit tests
npm run test                  # Run Jest tests
npm run test:watch           # Watch mode

# Device testing
npx expo start --ios         # Test on iOS simulator
npx expo start --android     # Test on Android emulator

# Component testing
npm run test:component       # Component unit tests
```

### Module Testing
```bash
# Test module in isolation
cd [platform]/modules/[module-name]
npm run test

# Test module integration
cd [platform]/core
# Load module and run integration tests
npm run test:modules
```

## Development Server Setup

### Prerequisites
```bash
# Required tools
rustc --version      # Rust 1.75+ (stable)
cargo --version      # Cargo
cargo loco --version # loco-cli (cargo install loco-cli)
node --version       # Node.js 18+
npm --version        # npm 9+
docker --version     # Docker 24+

# Mobile development
npx expo --version  # Expo CLI
# Android Studio (for Android development)
# Xcode (for iOS development on macOS)
```

### Local Environment Setup
```bash
# Clone repository
git clone <repository-url>
cd superapp

# Setup environment variables
cp .env.example .env.local
# Edit .env.local with your local configuration

# Start infrastructure services (using helvetia-compose)
# Connect to existing helvetia-compose project for:
# - PostgreSQL database
# - Redis cache
# - Kafka message broker
# - Prometheus monitoring
# - Grafana dashboards

# Backend setup
cd backend/core
cp .env.example .env
cargo loco start

# Frontend setup (new terminal)
cd frontend/core
npm install
npm run dev

# Mobile setup (new terminal)
cd mobile/core
npm install
npx expo start
```

### Hot Reload Configuration

**Backend (cargo-watch)**:
```bash
# Install once: cargo install cargo-watch
# Rebuild + restart the loco server on source changes
cargo watch -w src -w config -x "loco start"
```

**Frontend (Vite)**:
```javascript
// vite.config.ts
export default {
  server: {
    hmr: {
      overlay: false
    },
    watch: {
      usePolling: true
    }
  }
}
```

**Mobile (Expo)**:
```json
// metro.config.js
module.exports = {
  watchFolders: ['../modules'],
  resolver: {
    alias: {
      '@modules': '../modules'
    }
  }
}
```

### Database Setup
```bash
# Using helvetia-compose infrastructure
# Database will be available at connection details provided
# Run migrations (SeaORM via loco)
cd backend/core
cargo loco db migrate

# Generate entities from the schema (after migrations)
cargo loco db entities

# Seed initial data
cargo loco task seed

# Create first admin user (automatic on first access)
# First user to access the system becomes admin
```

## Module Development Guidelines

### Creating New Modules

**1. Generate Module Structure**
```bash
# Backend module
mkdir -p backend/modules/[module-name]/{src,tests}
cd backend/modules/[module-name]

# Initialize a Cargo binary crate (the module service) + a Dockerfile
cargo init --bin --name superapp_module_[module-name]

# Frontend module
mkdir -p frontend/modules/[module-name]/{src,build,test}
cd frontend/modules/[module-name]
npm init -y

# Mobile module
mkdir -p mobile/modules/[module-name]/{src,build,test}
cd mobile/modules/[module-name]
npm init -y
```

**2. Module Template Structure**
```bash
# Backend module structure (a containerized service)
backend/modules/[module-name]/
├── src/
│   ├── main.rs          # Service entry point (builds + serves the module via the SDK)
│   ├── controllers/     # HTTP/gRPC handlers
│   ├── services/        # Business logic
│   ├── models/          # SeaORM entities / data models
│   └── config.rs        # Module configuration
├── tests/
│   └── integration.rs
├── Dockerfile           # builds the module's OCI image
├── Cargo.toml
└── README.md
```

**3. Module Implementation Requirements**

**Backend Module Interface** (the container's entrypoint — built with the SDK, served over HTTP/gRPC; no in-process linking):
```rust
// src/main.rs
use superapp_module::{Health, Method, Module, ModuleError};

#[tokio::main]
async fn main() -> Result<(), ModuleError> {
    Module::builder()
        .name("my-module")
        .version("1.0.0")
        .permissions(["my-module:read"])              // Cedar actions, enforced at the gateway
        .route(Method::GET, "/my-module/items", list_items)
        .on_init(|_cfg| async move { Ok(()) })
        .on_shutdown(|| async move { Ok(()) })
        .health(|| async move { Health::Ok })
        .serve()                                       // listens on the module port; core proxies here
        .await
}
```

**Frontend Module Interface**:
```typescript
// src/index.ts
import { ModuleInterface } from '@/types/module';

const MyModule: ModuleInterface = {
  name: 'my-module',
  version: '1.0.0',
  
  routes: [
    {
      path: '/my-module',
      component: lazy(() => import('./components/ModuleMain')),
      permissions: ['user']
    }
  ],
  
  components: {
    ModuleMain: lazy(() => import('./components/ModuleMain'))
  },
  
  async initialize(config) {
    // Module initialization
  },
  
  async cleanup() {
    // Cleanup resources
  }
};

export default MyModule;
```

### Module Testing
```bash
# Test module in isolation
cd [platform]/modules/[module-name]

# Backend
cargo test

# Frontend/Mobile
npm test

# Integration testing with core
cd [platform]/core
# Load module and test integration
```

### Module Deployment
```bash
# Backend module build (OCI image, pushed to the private registry)
cd backend/modules/[module-name]
docker build -t <private-registry>/[module-name]:1.0.0 .
docker push <private-registry>/[module-name]:1.0.0   # core resolves/runs it as a container

# Frontend module build
cd frontend/modules/[module-name]
npm run build

# Mobile module build
cd mobile/modules/[module-name]
npm run build

# Module registration via API
curl -X POST http://localhost:8080/api/v1/modules/register \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "name": "my-module",
    "version": "1.0.0",
    "platform": "backend"
  }'
```

### Development Best Practices

1. **Incremental Development**: Start with one platform, then extend to others
2. **Mock Dependencies**: Use mocks for external services during development
3. **Hot Reload**: Leverage platform-specific hot reload for faster development
4. **Logging**: Use structured logging for better debugging
5. **Error Boundaries**: Implement proper error handling at module boundaries
6. **Performance**: Monitor module performance impact on core application
7. **Security**: Always validate module permissions and data access

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
