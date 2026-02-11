import { fetchAuthSession } from "aws-amplify/auth";
import type { CreateTodoRequest, Todo, UpdateTodoRequest } from "../types/todo";

const API_URL = import.meta.env.VITE_API_ENDPOINT;

async function authFetch(
	path: string,
	options: RequestInit = {},
): Promise<Response> {
	const session = await fetchAuthSession();
	const token = session.tokens?.idToken?.toString();

	if (!token) {
		throw new Error("Not authenticated");
	}

	const response = await fetch(`${API_URL}${path}`, {
		...options,
		headers: {
			"Content-Type": "application/json",
			Authorization: `Bearer ${token}`,
			...options.headers,
		},
	});

	if (!response.ok) {
		const error = await response.json().catch(() => ({}));
		throw new Error(
			(error as { error?: string }).error ||
				`Request failed: ${response.status}`,
		);
	}

	return response;
}

export async function listTodos(): Promise<Todo[]> {
	const response = await authFetch("/todos");
	return response.json();
}

export async function createTodo(data: CreateTodoRequest): Promise<Todo> {
	const response = await authFetch("/todos", {
		method: "POST",
		body: JSON.stringify(data),
	});
	return response.json();
}

export async function updateTodo(
	id: string,
	data: UpdateTodoRequest,
): Promise<Todo> {
	const response = await authFetch(`/todos/${id}`, {
		method: "PATCH",
		body: JSON.stringify(data),
	});
	return response.json();
}

export async function deleteTodo(id: string): Promise<void> {
	await authFetch(`/todos/${id}`, {
		method: "DELETE",
	});
}
