import { useState, useCallback, useEffect } from "react";
import type { CalendarEvent } from "../types";
import { api } from "../api/client";

export function useEvents(start: string, end: string) {
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refetch = useCallback(() => {
    setLoading(true);
    setError(null);
    api
      .listEvents(start, end)
      .then((data) => setEvents(data))
      .catch((err) => setError(err.message))
      .finally(() => setLoading(false));
  }, [start, end]);

  useEffect(() => {
    refetch();
  }, [refetch]);

  // Listen for custom event from useMainChat when LLM creates an event
  useEffect(() => {
    const handler = () => refetch();
    window.addEventListener("events-changed", handler);
    return () => window.removeEventListener("events-changed", handler);
  }, [refetch]);

  return { events, loading, error, refetch };
}

export function useCreateEvent() {
  const [loading, setLoading] = useState(false);

  const create = useCallback(async (data: Partial<CalendarEvent>) => {
    setLoading(true);
    try {
      const event = await api.createEvent(data);
      return event;
    } finally {
      setLoading(false);
    }
  }, []);

  return { create, loading };
}

export function useUpdateEvent() {
  const [loading, setLoading] = useState(false);

  const update = useCallback(
    async (id: string, data: Partial<CalendarEvent>) => {
      setLoading(true);
      try {
        const event = await api.updateEvent(id, data);
        return event;
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  return { update, loading };
}

export function useDeleteEvent() {
  const [loading, setLoading] = useState(false);

  const del = useCallback(async (id: string) => {
    setLoading(true);
    try {
      await api.deleteEvent(id);
    } finally {
      setLoading(false);
    }
  }, []);

  return { delete: del, loading };
}
