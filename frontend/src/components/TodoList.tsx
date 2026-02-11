import { useCallback, useEffect, useState } from "react";
import * as api from "../api/todos";
import type { Todo } from "../types/todo";
import { AddTodo } from "./AddTodo";
import { TodoItem } from "./TodoItem";

export function TodoList() {
	const [todos, setTodos] = useState<Todo[]>([]);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);

	const fetchTodos = useCallback(async () => {
		try {
			setError(null);
			const data = await api.listTodos();
			setTodos(data);
		} catch (e) {
			setError(e instanceof Error ? e.message : "Failed to load todos");
		} finally {
			setLoading(false);
		}
	}, []);

	useEffect(() => {
		fetchTodos();
	}, [fetchTodos]);

	const handleAdd = async (title: string) => {
		const todo = await api.createTodo({ title });
		setTodos((prev) => [todo, ...prev]);
	};

	const handleToggle = async (id: string, completed: boolean) => {
		const updated = await api.updateTodo(id, { completed });
		setTodos((prev) => prev.map((t) => (t.id === id ? updated : t)));
	};

	const handleDelete = async (id: string) => {
		await api.deleteTodo(id);
		setTodos((prev) => prev.filter((t) => t.id !== id));
	};

	if (loading) {
		return <div className="loading">読み込み中...</div>;
	}

	return (
		<div className="todo-list">
			<AddTodo onAdd={handleAdd} />
			{error && <div className="error">{error}</div>}
			{todos.length === 0 ? (
				<p className="empty">ToDoはまだありません</p>
			) : (
				<ul>
					{todos.map((todo) => (
						<TodoItem
							key={todo.id}
							todo={todo}
							onToggle={handleToggle}
							onDelete={handleDelete}
						/>
					))}
				</ul>
			)}
		</div>
	);
}
