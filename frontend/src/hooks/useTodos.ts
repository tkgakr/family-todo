import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { todosApi } from "../api/todos"
import { type CreateTodoRequest, TodoStatus, type UpdateTodoRequest } from "../types/todo"

export const useTodos = (status: TodoStatus = TodoStatus.Active) => {
  return useQuery({
    queryKey: ["todos", status],
    queryFn: () => todosApi.getTodos(status),
  })
}

export const useTodo = (id: string) => {
  return useQuery({
    queryKey: ["todo", id],
    queryFn: () => todosApi.getTodo(id),
    enabled: !!id,
  })
}

export const useTodoHistory = (id: string) => {
  return useQuery({
    queryKey: ["todo-history", id],
    queryFn: () => todosApi.getTodoHistory(id),
    enabled: !!id,
  })
}

export const useCreateTodo = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (data: CreateTodoRequest) => todosApi.createTodo(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["todos"] })
    },
  })
}

export const useUpdateTodo = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateTodoRequest }) =>
      todosApi.updateTodo(id, data),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: ["todos"] })
      queryClient.invalidateQueries({ queryKey: ["todo", id] })
      queryClient.invalidateQueries({ queryKey: ["todo-history", id] })
    },
  })
}

export const useCompleteTodo = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (id: string) => todosApi.completeTodo(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: ["todos"] })
      queryClient.invalidateQueries({ queryKey: ["todo", id] })
    },
  })
}

export const useDeleteTodo = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({ id, reason }: { id: string; reason?: string }) =>
      todosApi.deleteTodo(id, reason),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: ["todos"] })
      queryClient.removeQueries({ queryKey: ["todo", id] })
    },
  })
}
