import { useState, useEffect, useCallback, useMemo } from "react";
import type { Task } from "../types";
import { api } from "../api/client";

export function useTasks(filters?: Record<string, string>) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const serializedFilters = filters ? JSON.stringify(filters) : "";

  const derivedFilters = useMemo<Record<string, string> | undefined>(() => {
    if (!serializedFilters) return undefined;
    return JSON.parse(serializedFilters) as Record<string, string>;
  }, [serializedFilters]);

  const fetchTasks = useCallback(() => {
    return api
      .listTasks(derivedFilters)
      .then((data) => {
        setTasks(data);
        setError(null);
      })
      .catch((err) => setError(err.message))
      .finally(() => setLoading(false));
  }, [derivedFilters]);

  const refetch = useCallback(() => {
    setLoading(true);
    setError(null);
    void fetchTasks();
  }, [fetchTasks]);

  useEffect(() => {
    // La marca de "petición en curso" debe fijarse fuera del camino síncrono
    // del efecto (spec frontend-lint-zero), pero en el mismo turno, para que
    // al cambiar los filtros vuelva a aparecer el indicador de carga.
    // Programarla en un microtask resuelto deja el estado fuera del cuerpo
    // síncrono del efecto, sin necesidad de suprimir la regla.
    let cancelled = false;
    void Promise.resolve().then(() => {
      if (cancelled) return;
      setLoading(true);
      setError(null);
    });
    void fetchTasks();
    return () => {
      cancelled = true;
    };
  }, [fetchTasks]);

  // Listen for custom event from LLM task operations
  useEffect(() => {
    const handler = () => refetch();
    window.addEventListener("tasks-changed", handler);
    return () => window.removeEventListener("tasks-changed", handler);
  }, [refetch]);

  return { tasks, loading, error, refetch };
}

export function useCreateTask() {
  const [loading, setLoading] = useState(false);

  const create = useCallback(async (data: Partial<Task>) => {
    setLoading(true);
    try {
      return await api.createTask(data);
    } finally {
      setLoading(false);
    }
  }, []);

  return { create, loading };
}

export function useUpdateTask() {
  const [loading, setLoading] = useState(false);

  const update = useCallback(async (id: string, data: Partial<Task>) => {
    setLoading(true);
    try {
      return await api.updateTask(id, data);
    } finally {
      setLoading(false);
    }
  }, []);

  return { update, loading };
}

export function useDeleteTask() {
  const [loading, setLoading] = useState(false);

  const del = useCallback(async (id: string) => {
    setLoading(true);
    try {
      return await api.deleteTask(id);
    } finally {
      setLoading(false);
    }
  }, []);

  return { delete: del, loading };
}
