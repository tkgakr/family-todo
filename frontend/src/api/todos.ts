import {
  type CreateTodoRequest,
  type Todo,
  type TodoHistoryResponse,
  type TodoListResponse,
  TodoStatus,
  type UpdateTodoRequest,
} from "../types/todo"
import { apiClient } from "./client"

export const todosApi = {
  // Get all todos
  getTodos: async (status: TodoStatus = TodoStatus.Active): Promise<TodoListResponse> => {
    const response = await apiClient.get("/todos", {
      params: { status: status.toLowerCase() },
    })
    return response.data
  },

  // Get a specific todo
  getTodo: async (id: string): Promise<Todo> => {
    const response = await apiClient.get(`/todos/${id}`)
    return response.data
  },

  // Create a new todo
  createTodo: async (data: CreateTodoRequest): Promise<Todo> => {
    const response = await apiClient.post("/todos", data)
    return response.data
  },

  // Update a todo
  updateTodo: async (id: string, data: UpdateTodoRequest): Promise<Todo> => {
    const response = await apiClient.put(`/todos/${id}`, data)
    return response.data
  },

  // Complete a todo
  completeTodo: async (id: string): Promise<Todo> => {
    const response = await apiClient.post(`/todos/${id}/complete`)
    return response.data
  },

  // Delete a todo
  deleteTodo: async (id: string, reason?: string): Promise<void> => {
    await apiClient.delete(`/todos/${id}`, {
      data: { reason },
    })
  },

  // Get todo history
  getTodoHistory: async (id: string): Promise<TodoHistoryResponse> => {
    const response = await apiClient.get(`/todos/${id}/history`)
    return response.data
  },
}
