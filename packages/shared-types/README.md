# @nebula-code/shared-types

Shared TypeScript type definitions for Nebula Code projects.

## Installation

```bash
pnpm add @nebula-code/shared-types
```

## Usage

```typescript
import { SkillCard, Project, SkillCategory } from '@nebula-code/shared-types';

// Use the types
const skill: SkillCard = {
  id: 'my-skill',
  name: 'My Skill',
  description: 'A custom skill',
  version: '1.0.0',
  // ... other fields
};
```

## Types

- `SkillCard` - Complete skill card data model
- `Project` - Project structure and configuration
- `SkillAuthor` - Author information
- `SkillDependency` - Skill dependencies
- And more...

See the [API documentation](./src/index.ts) for full type definitions.
