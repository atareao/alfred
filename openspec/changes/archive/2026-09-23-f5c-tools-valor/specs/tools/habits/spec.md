# Habits Tool

## Contracts

```rust
pub struct HabitsTool {
    db: Arc<Mutex<Connection>>,
}

pub struct Habit {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub frequency: String,       // "daily" | "weekly"
    pub target: Option<u32>,     // target per period (e.g. 30 min)
    pub created_at: String,
}

pub struct HabitLog {
    pub habit_id: String,
    pub date: String,            // ISO date
    pub completed: bool,
}

pub struct HabitStreak {
    pub habit_id: String,
    pub name: String,
    pub current_streak: u32,     // consecutive days/weeks
    pub longest_streak: u32,
    pub total_count: u32,
}

pub struct HabitStats {
    pub habit_id: String,
    pub name: String,
    pub period: String,          // "week" | "month" | "year"
    pub completion_rate: f64,    // 0.0 - 1.0
    pub total_expected: u32,
    pub total_completed: u32,
}
```

## Scenarios

### Happy path: create_habit
**Given** a valid profile  
**When** the user calls `create_habit` with `{ "name": "Leer 20 min", "frequency": "daily", "target": 20 }`  
**Then** a new habit is created and stored in `habits`

### Happy path: log_habit
**Given** a habit exists  
**When** the user calls `log_habit` with `{ "habit_id": "<id>" }`  
**Then** a log entry is created for today with `completed: true`

### Happy path: habit_streaks
**Given** habits exist with some logs  
**When** the user calls `habit_streaks`  
**Then** the tool returns current and longest streaks for all habits

### Happy path: habit_stats
**Given** a habit exists with logs  
**When** the user calls `habit_stats` with `{ "habit_id": "<id>", "period": "month" }`  
**Then** the tool returns completion stats for the specified period

### Error: create_habit missing name
**When** the user calls `create_habit` without `name`  
**Then** the tool returns `ToolError::InvalidArguments`

### Error: log_habit for non-existent habit
**When** the user calls `log_habit` with an invalid `habit_id`  
**Then** the tool returns `ToolError::NotFound`

### Error: duplicate log for same day
**Given** a habit was already logged today  
**When** the user calls `log_habit` again for the same habit  
**Then** the tool returns success (idempotent — updates existing log)

### Permission: create_habit
**Given** the `create_habit` operation  
**Then** its permission level is `Notify`

### Permission: other operations
**Given** `log_habit`, `habit_streaks`, `habit_stats`  
**Then** their permission level is `NoConfirm`