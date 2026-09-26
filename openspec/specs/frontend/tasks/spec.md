# frontend/tasks Specification

## Purpose
Interfaz de usuario para gestión de tareas GTD con dos vistas: Kanban (flujo activo: inbox → todo → doing → done) y Lista (todas las tareas incluyendo waiting y someday).

## Requirements

### Requirement: Task type and API client

Añadir tipos y métodos de API para tareas en el frontend.

**Contracts:**

```typescript
// types/index.ts
export interface Task {
  id: string;
  profile_id: string;
  content: string;
  status: 'inbox' | 'todo' | 'doing' | 'waiting' | 'someday' | 'done';
  priority: 'low' | 'medium' | 'high';
  project?: string;
  due_date?: string;
  scope: 'shared' | 'personal';
  created_at: string;
  updated_at: string;
}

// api/client.ts — nuevos métodos
export const api = {
  // ... existing methods
  listTasks: (params?: { status?: string; priority?: string; project?: string }) =>
    request<Task[]>(`/tasks?${new URLSearchParams(params as Record<string, string>).toString()}`),

  createTask: (data: Partial<Task>) =>
    request<Task>('/tasks', { method: 'POST', body: JSON.stringify(data) }),

  updateTask: (id: string, data: Partial<Task>) =>
    request<Task>(`/tasks/${id}`, { method: 'PUT', body: JSON.stringify(data) }),

  deleteTask: (id: string) =>
    request<Task>(`/tasks/${id}`, { method: 'DELETE' }),
};
```

**Scenarios:**

#### Scenario: Task type has all GTD fields
When a Task object is created
Then it has id, content, status (6 values), priority, project, due_date, scope, created_at, updated_at

#### Scenario: API client can list tasks with filters
When `api.listTasks({ status: 'todo', priority: 'high' })` is called
Then it returns an array of Task objects matching the filters

### Requirement: Task hooks

Hooks React para gestionar el estado de las tareas.

**Contracts:**

```typescript
// hooks/useTasks.ts
function useTasks(filters?: { status?: string; priority?: string; project?: string }): {
  tasks: Task[];
  loading: boolean;
  error: string | null;
  refetch: () => void;
}

function useCreateTask(): {
  create: (data: Partial<Task>) => Promise<Task>;
  loading: boolean;
}

function useUpdateTask(): {
  update: (id: string, data: Partial<Task>) => Promise<Task>;
  loading: boolean;
}

function useDeleteTask(): {
  delete: (id: string) => Promise<Task>;
  loading: boolean;
}
```

**Scenarios:**

#### Scenario: useTasks fetches and returns tasks
When useTasks is called with filters
Then it returns tasks matching those filters
And loading state transitions from true to false

#### Scenario: useCreateTask creates and returns task
When useCreateTask.create() is called with task data
Then a new task is created
And the full task object is returned

### Requirement: TaskView component with Kanban and List views

Componente principal con selector de vista (Kanban / Lista), botón "New Task", y filtros.

**Contracts:**

```typescript
interface TaskViewProps {
  onClose?: () => void;
}

// Colores por estado
const STATUS_COLORS: Record<string, string> = {
  inbox: '#8c8c8c',     // grey
  todo: '#1677ff',      // blue
  doing: '#faad14',     // gold/amber
  waiting: '#722ed1',   // purple
  someday: '#13c2c2',   // cyan
  done: '#52c41a',      // green
};

const STATUS_LABELS: Record<string, string> = {
  inbox: '📥 Inbox',
  todo: '📋 Todo',
  doing: '🏗️ Doing',
  waiting: '⏳ Waiting',
  someday: '📆 Someday',
  done: '✅ Done',
};
```

**Kanban view:**
- 4 columnas del flujo activo: `inbox` | `todo` | `doing` | `done`
- Cada columna muestra tarjetas con content, priority badge, project tag, due_date
- Drag & drop entre columnas (cambia status)
- Botón "+" en cada columna para crear tarea directamente en ese estado
- Las tareas `waiting` y `someday` NO aparecen en el Kanban

**List view:**
- Ant Design Table con columnas: content, status, priority, project, due_date, actions
- Filtros rápidos: etiquetas clickeables "⏳ Waiting (n)" y "📆 Someday (n)" con contador
- Filtro general por proyecto, prioridad, estado
- Ordenable por fecha de creación

**Scenarios:**

#### Scenario: TaskView shows view toggle
Given the TaskView component renders
Then it shows a toggle/segmented control between "Kanban" and "List" views
And the default view is "Kanban"

#### Scenario: Kanban shows 4 active-flow columns
Given the Kanban view is active
Then it shows exactly 4 columns: inbox, todo, doing, done
And tasks with status waiting/someday are NOT shown

#### Scenario: Kanban drag changes status
Given a task with status "todo"
When the user drags it to the "doing" column
Then the task's status is updated to "doing"
And the task appears in the doing column

#### Scenario: List view shows all tasks including waiting/someday
Given tasks exist with statuses inbox, todo, doing, waiting, someday, done
When the List view is active
Then ALL tasks are displayed in the table
And quick-filter badges show counts for waiting and someday

#### Scenario: Quick filter for waiting tasks
Given the List view is active
When the user clicks "⏳ Waiting (3)" badge
Then only tasks with status "waiting" are shown

#### Scenario: Create task from TaskView
Given the TaskView is open
When the user clicks "New Task"
Then a modal/form opens with fields: content, status, priority, project, due_date, scope
And on submit the task is created and appears in the view

#### Scenario: Edit task from TaskView
Given an existing task
When the user clicks edit on a task
Then a modal opens with pre-filled fields
And on save the task is updated

#### Scenario: Delete task from TaskView
Given an existing task
When the user clicks delete on a task
Then a confirmation dialog appears
And on confirm the task is removed

### Requirement: TaskView integrated in AppLayout

Añadir botón de Tasks en el header de AppLayout, junto al de calendario.

**Contracts:**

```typescript
// AppLayout.tsx — nuevo estado y botón
const [tasksVisible, setTasksVisible] = useState(false);

// Nuevo botón en el header
<Button type="text" icon={<CheckSquareOutlined />} onClick={() => setTasksVisible(true)} />

// Nuevo modal
<Modal title="✅ Tasks" open={tasksVisible} onCancel={() => setTasksVisible(false)} footer={null} width={1000}>
  <TaskView />
</Modal>
```

**Scenarios:**

#### Scenario: Tasks button visible in header
Given the AppLayout renders
Then there is a button with a checkbox icon in the header
When clicked, it opens the TaskView modal

#### Scenario: LLM task operations trigger refetch
When the LLM completes a tasks operation (add_task, update_task, complete_task, delete_task)
Then a custom event 'tasks-changed' is dispatched
And the TaskView refetches tasks automatically