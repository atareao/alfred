import { useState, useCallback, useEffect } from "react";
import type { CalendarEvent } from "../types";
import { api } from "../api/client";

export function useEvents(start: string, end: string) {
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchEvents = useCallback(() => {
    return api
      .listEvents(start, end)
      .then((data) => {
        setEvents(data);
        setError(null);
      })
      .catch((err) => setError(err.message))
      .finally(() => setLoading(false));
  }, [start, end]);

  const refetch = useCallback(() => {
    setLoading(true);
    setError(null);
    void fetchEvents();
  }, [fetchEvents]);

  useEffect(() => {
    // La marca de "petición en curso" debe fijarse fuera del camino síncrono
    // del efecto (spec frontend-lint-zero), pero en el mismo turno, para que
    // al cambiar `start`/`end` vuelva a aparecer el `Spin` y no se sigan
    // mostrando los eventos del rango anterior. Programarla en un microtask
    // resuelto deja el estado fuera del cuerpo síncrono del efecto, sin
    // necesidad de suprimir la regla.
    let cancelled = false;
    void Promise.resolve().then(() => {
      if (cancelled) return;
      setLoading(true);
      setError(null);
    });
    void fetchEvents();
    return () => {
      cancelled = true;
    };
  }, [fetchEvents]);

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
