# @nebula-code/ui-components

Reusable React UI components for Nebula Code applications.

## Installation

```bash
pnpm add @nebula-code/ui-components
```

## Components

### SkillCard
Display skill card information with category badge and tags.

```tsx
import { SkillCard } from '@nebula-code/ui-components';

<SkillCard skill={skillData} onSelect={handleSelect} />
```

### Button
Customizable button with variants and sizes.

```tsx
import { Button } from '@nebula-code/ui-components';

<Button variant="primary" size="lg">Click me</Button>
```

### Card
Container component with optional title and footer.

```tsx
import { Card } from '@nebula-code/ui-components';

<Card title="My Card">
  Content goes here
</Card>
```

### Badge
Small highlighted label for status or categorization.

```tsx
import { Badge } from '@nebula-code/ui-components';

<Badge variant="success">Active</Badge>
```

## Styling

Components use Tailwind CSS classes. Make sure Tailwind is configured in your project.
