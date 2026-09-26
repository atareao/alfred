import { useState, useEffect, useCallback } from "react";
import type { Task } from "../types";
import { api } from "../api/client";

export function useTasks(filters?: Record<string, string>) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const serializedFilters = filters ? JSON.stringify(filters) : "";

  const refetch = useCallback(() => {
    setLoading(true);
    setError(null);
    api
      .listTasks(filters)
      .then((data) => setTasks(data))
      .catch((err) => setError(err.message))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [serializedFilters]);

  useEffect(() => {
    refetch();
  }, [refetch]);

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
