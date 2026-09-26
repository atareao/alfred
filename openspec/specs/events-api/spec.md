# events-api Specification

## Purpose
TBD - created by archiving change events-rest-api. Update Purpose after archive.

## Requirements

### Requirement: Events REST API CRUD endpoints

#### Scenario: List events in date range
**Given** un evento existe con start_time="2026-09-25T10:00:00Z" en la BD  
**When** se hace GET `/api/events?start=2026-09-25T00:00:00Z&end=2026-09-25T23:59:59Z`  
**Then** devuelve 200 con un array que contiene ese evento

#### Scenario: List events — empty range
**Given** no hay eventos en la BD  
**When** se hace GET `/api/events?start=2026-09-25T00:00:00Z&end=2026-09-25T23:59:59Z`  
**Then** devuelve 200 con un array vacío

#### Scenario: List events — missing query params
**When** se hace GET `/api/events`  
**Then** devuelve 422 Unprocessable Entity

#### Scenario: Create event
**Given** un payload JSON válido con title, start_time, end_time  
**When** se hace POST `/api/events`  
**Then** devuelve 201 con el evento creado (incluyendo id, created_at, updated_at)

#### Scenario: Create event — missing required fields
**When** se hace POST `/api/events` con body `{}`  
**Then** devuelve 422 Unprocessable Entity

#### Scenario: Update event
**Given** un evento existe con id="evt-1"  
**When** se hace PUT `/api/events/evt-1` con body `{"title": "Updated"}`  
**Then** devuelve 200 con el evento actualizado

#### Scenario: Update event — not found
**When** se hace PUT `/api/events/nonexistent` con body `{"title": "X"}`  
**Then** devuelve 404

#### Scenario: Delete event
**Given** un evento existe con id="evt-1"  
**When** se hace DELETE `/api/events/evt-1`  
**Then** devuelve 204 No Content

#### Scenario: Delete event — not found (idempotent)
**When** se hace DELETE `/api/events/nonexistent`  
**Then** devuelve 204 No Content (idempotent)
