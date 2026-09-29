# frontend/agenda Specification

## Purpose
Interfaz de agenda de Valet: componente de calendario con vistas de día, semana y mes, creación, edición y eliminación de eventos, categorías con colores y filtrado por categoría.

## Requirements

### Requirement: Calendar view component

El componente SHALL mostrar un calendario de Ant Design con vistas mensual, semanal y diaria, permitiendo crear, editar y eliminar eventos y filtrarlos por categoría.

New React component for agenda visualization using Ant Design Calendar with day/week/month views.

**Contracts:**

```typescript
// Types
interface CalendarEvent {
  id: string;
  title: string;
  description?: string;
  start: string;          // ISO 8601
  end: string;
  location?: string;
  scope: 'shared' | 'personal';
  category: 'default' | 'work' | 'personal' | 'health' | 'birthday' | 'holiday';
  allDay: boolean;
  rrule?: string;
}

// Category color mapping
const CATEGORY_COLORS: Record<string, string> = {
  default: '#1677ff',     // blue
  work: '#52c41a',        // green
  personal: '#fa8c16',    // orange
  health: '#f5222d',      // red
  birthday: '#eb2f96',    // pink
  holiday: '#722ed1',     // purple
};

// Hooks
function useEvents(dateRange: [string, string]): {
  events: CalendarEvent[];
  loading: boolean;
  error: Error | null;
  refetch: () => void;
}

function useCreateEvent(): {
  create: (event: Partial<CalendarEvent>) => Promise<CalendarEvent>;
  loading: boolean;
}

function useUpdateEvent(): {
  update: (id: string, changes: Partial<CalendarEvent>) => Promise<void>;
  loading: boolean;
}

function useDeleteEvent(): {
  delete: (id: string) => Promise<void>;
  loading: boolean;
}
```

**Scenarios:**

#### Scenario: Calendar view shows monthly grid
Given the user navigates to the agenda section
When the CalendarView component renders
Then it shows a monthly calendar grid with Ant Design Calendar
And events are displayed as colored badges on their dates

#### Scenario: Click on date shows daily events
Given the CalendarView is in monthly mode
When the user clicks on a specific day
Then the view switches to daily detail showing that day's events with times

#### Scenario: Create event from calendar
Given the CalendarView is open
When the user clicks "New Event" button
Then a modal/form opens with fields: title, start, end, allDay, category, scope, rrule, reminder
And on submit the event is created and appears in the calendar

#### Scenario: Edit event from calendar
Given an existing event displayed in the calendar
When the user clicks on the event
Then a modal opens with pre-filled fields
And on save the event is updated

#### Scenario: Delete event from calendar
Given an existing event
When the user clicks delete in the event modal
Then a confirmation dialog appears
And on confirm the event is removed from the calendar

#### Scenario: Filter events by category
Given the calendar has events of multiple categories
When the user selects a filter for "work" category
Then only work events are displayed
