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
- **Authentication & Authorization**: JWT-based authentication with admin-controlled user management
  - **No Public Registration**: First user to access system becomes admin automatically
  - **Admin-Only User Creation**: Only admin users can create new user accounts
  - **Role-based Access Control (RBAC)**: Admin and regular user roles
  - **Admin Role Management**: Admin can promote any user to admin status
  - **Token Management**: Access/Refresh Token pattern with JWT
    - Short-lived access tokens for API requests
    - Long-lived refresh tokens for token renewal
    - Automatic token rotation on refresh
    - Secure refresh token storage with Redis
  - **API Security**: Protected endpoints with role-based middleware
  - **Service Authentication**: API key authentication for module-to-service communication
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
- **Response Structure**:
  ```json
  {
    "success": boolean,
    "data": { /* response payload */ },
    "message": "string",
    "errors": ["array of error messages"],
    "pagination": { /* for paginated responses */ }
  }
  ```

### Error Handling Patterns
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

### Required Module Exports (Backend)
```go
type Module interface {
    Name() string
    Version() string
    Initialize(config Config) error
    Shutdown() error
    Routes() []Route
    Permissions() []Permission
    HealthCheck() error
}
```

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

### Module Lifecycle Hooks
- **Load**: `initialize(config)` - Setup module with configuration
- **Unload**: `shutdown()` or `cleanup()` - Clean up resources
- **Health**: `healthCheck()` - Verify module status
- **Update**: Hot-reload capability for development

### Configuration Interface
- **Schema Validation**: JSON Schema for module configuration
- **Environment Variables**: Module-specific env vars with `SUPERAPP_MODULE_{NAME}_` prefix
- **Runtime Config**: API endpoint for dynamic configuration updates

# Development Standards & Conventions

## Go Backend Standards

### Package Structure
- **Main Package**: `cmd/` for application entry points
- **Internal Packages**: `internal/` for private application code
- **Shared Packages**: `pkg/` for reusable library code
- **Package Naming**: Short, lowercase, single word (e.g., `auth`, `user`, `config`)

### File Organization
```
backend/core/src/
├── cmd/
│   └── server/
│       └── main.go
├── internal/
│   ├── auth/          # Authentication logic
│   ├── user/          # User management
│   ├── module/        # Module system
│   ├── api/           # HTTP handlers
│   ├── middleware/    # HTTP middleware
│   ├── config/        # Configuration
│   └── database/      # Database operations
└── pkg/
    ├── logger/        # Logging utilities
    ├── validator/     # Validation helpers
    └── response/      # Response formatters
```

### Naming Conventions
- **Files**: Snake_case (e.g., `user_handler.go`, `auth_middleware.go`)
- **Types**: PascalCase (e.g., `UserService`, `AuthConfig`)
- **Functions**: PascalCase for exported, camelCase for private
- **Variables**: camelCase (e.g., `userID`, `configPath`)
- **Constants**: UPPER_SNAKE_CASE (e.g., `DEFAULT_PORT`, `JWT_SECRET_KEY`)

### Error Handling Patterns
```go
// Custom error types
type UserError struct {
    Code    string `json:"code"`
    Message string `json:"message"`
    Field   string `json:"field,omitempty"`
}

// Error wrapping
if err != nil {
    return fmt.Errorf("failed to create user: %w", err)
}

// HTTP error responses
func HandleError(c *gin.Context, err error) {
    // Structured error handling logic
}
```

### Testing Conventions
- **Test Files**: `*_test.go` in same package
- **Test Functions**: `TestFunctionName(t *testing.T)`
- **Benchmark Tests**: `BenchmarkFunctionName(b *testing.B)`
- **Table Tests**: Use subtests with `t.Run()`
- **Mocking**: Use interfaces for dependency injection
- **Coverage**: Minimum 80% code coverage for critical paths

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
- **Inline Documentation**: JSDoc for functions, GoDoc for Go packages

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
- **Go**: `gofmt` and `goimports` for automatic formatting
- **TypeScript/JavaScript**: Prettier with consistent configuration
- **Line Length**: Maximum 100 characters
- **Indentation**: 2 spaces for JS/TS, tabs for Go
- **Trailing Commas**: Always in multi-line structures

# Development Workflow

## Build Commands

### Backend (Go)
```bash
# Development
cd backend/core
go mod tidy                    # Install dependencies
go run cmd/server/main.go      # Run development server
go build -o bin/server cmd/server/main.go  # Build binary

# With hot reload (using air)
air                           # Auto-reload on file changes

# Production build
go build -ldflags="-s -w" -o bin/server cmd/server/main.go

# Module development
cd backend/modules/[module-name]
go mod tidy
go build -buildmode=plugin -o [module-name].so src/main.go
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
# Unit tests
go test ./...                 # Run all tests
go test -v ./internal/auth    # Run specific package tests
go test -cover ./...          # Run with coverage
go test -race ./...           # Run with race detection

# Integration tests
go test -tags=integration ./... # Run integration tests

# Benchmark tests
go test -bench=. ./...        # Run benchmarks

# Test coverage report
go test -coverprofile=coverage.out ./...
go tool cover -html=coverage.out -o coverage.html
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
go version          # Go 1.21+
node --version      # Node.js 18+
npm --version       # npm 9+
docker --version    # Docker 24+

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
go mod download
go run cmd/server/main.go

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

**Backend (using Air)**:
```yaml
# .air.toml
root = "."
tmp_dir = "tmp"

[build]
  cmd = "go build -o ./tmp/main ./cmd/server"
  bin = "tmp/main"
  full_bin = "./tmp/main"
  include_ext = ["go", "tpl", "tmpl", "html"]
  exclude_dir = ["assets", "tmp", "vendor", "node_modules"]
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
# Run migrations
cd backend/core
go run cmd/migrate/main.go up

# Seed initial data
go run cmd/seed/main.go

# Create first admin user (automatic on first access)
# First user to access the system becomes admin
```

## Module Development Guidelines

### Creating New Modules

**1. Generate Module Structure**
```bash
# Backend module
mkdir -p backend/modules/[module-name]/{src,build,test}
cd backend/modules/[module-name]

# Initialize Go module
go mod init superapp/modules/[module-name]

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
# Backend module structure
backend/modules/[module-name]/
├── src/
│   ├── main.go          # Module entry point
│   ├── handlers/        # HTTP handlers
│   ├── services/        # Business logic
│   ├── models/          # Data models
│   └── config/          # Module configuration
├── test/
│   └── *_test.go
├── go.mod
├── go.sum
└── README.md
```

**3. Module Implementation Requirements**

**Backend Module Interface**:
```go
// src/main.go
package main

import "superapp/pkg/module"

type MyModule struct {}

func (m *MyModule) Name() string { return "my-module" }
func (m *MyModule) Version() string { return "1.0.0" }

func (m *MyModule) Initialize(config module.Config) error {
    // Module initialization logic
    return nil
}

func (m *MyModule) Routes() []module.Route {
    return []module.Route{
        {Method: "GET", Path: "/my-module/health", Handler: m.HealthCheck},
    }
}

// Export for plugin loading
var Module MyModule
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
go test ./...

# Frontend/Mobile
npm test

# Integration testing with core
cd [platform]/core
# Load module and test integration
```

### Module Deployment
```bash
# Backend module build
cd backend/modules/[module-name]
go build -buildmode=plugin -o [module-name].so src/main.go

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
