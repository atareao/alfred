# TravelPrepWorker Specification

## Purpose
Analyzes upcoming events with a location and generates travel preparation suggestions.

## Requirements

### Requirement: TravelPrepWorker SHALL find upcoming trips
**Given** a database with events  
**When** `find_upcoming_trips(profile_id)` is called  
**Then** it SHALL query events in the next 30 days with a non-null location  
**And** SHALL return a list of formatted trip descriptions

#### Scenario: No trips when no events have location
**Given** no events with location exist  
**When** `find_upcoming_trips` is called  
**Then** the result SHALL be an empty list

#### Scenario: Finds events with location, ignores those without
**Given** one event with location and one without  
**When** `find_upcoming_trips` is called  
**Then** only the event with location SHALL be returned

### Requirement: TravelPrepWorker SHALL generate trip preparation markdown
**Given** an event title and location  
**When** `prepare_for_trip(title, location)` is called  
**Then** it SHALL return markdown with destination, weather section, and suggestions

#### Scenario: Returns valid markdown
**Given** title "Viaje a Paris" and location "Paris, Francia"  
**When** `prepare_for_trip` is called  
**Then** the result SHALL contain the event title  
**And** SHALL contain the destination  
**And** SHALL contain a weather section  
**And** SHALL contain a suggestions section