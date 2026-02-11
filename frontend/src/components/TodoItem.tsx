import type { Todo } from "../types/todo";

interface TodoItemProps {
	todo: Todo;
	onToggle: (id: string, completed: boolean) => Promise<void>;
	onDelete: (id: string) => Promise<void>;
}

export function TodoItem({ todo, onToggle, onDelete }: TodoItemProps) {
	return (
		<li className={`todo-item ${todo.completed ? "completed" : ""}`}>
			<label>
				<input
					type="checkbox"
					checked={todo.completed}
					onChange={() => onToggle(todo.id, !todo.completed)}
				/>
				<span className="todo-title">{todo.title}</span>
			</label>
			<button
				type="button"
				className="delete-btn"
				onClick={() => onDelete(todo.id)}
				aria-label="削除"
			>
				✕
			</button>
		</li>
	);
}
