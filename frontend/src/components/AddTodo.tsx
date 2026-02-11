import { type FormEvent, useState } from "react";

interface AddTodoProps {
	onAdd: (title: string) => Promise<void>;
}

export function AddTodo({ onAdd }: AddTodoProps) {
	const [title, setTitle] = useState("");
	const [loading, setLoading] = useState(false);

	const handleSubmit = async (e: FormEvent) => {
		e.preventDefault();
		const trimmed = title.trim();
		if (!trimmed) return;

		setLoading(true);
		try {
			await onAdd(trimmed);
			setTitle("");
		} finally {
			setLoading(false);
		}
	};

	return (
		<form onSubmit={handleSubmit} className="add-todo">
			<input
				type="text"
				value={title}
				onChange={(e) => setTitle(e.target.value)}
				placeholder="新しいToDo..."
				disabled={loading}
			/>
			<button type="submit" disabled={loading || !title.trim()}>
				{loading ? "追加中..." : "追加"}
			</button>
		</form>
	);
}
