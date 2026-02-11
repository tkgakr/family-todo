import type { AuthUser } from "aws-amplify/auth";
import { TodoList } from "./components/TodoList";

interface AppProps {
	user?: AuthUser;
	signOut?: () => void;
}

export default function App({ user, signOut }: AppProps) {
	return (
		<div className="app">
			<header>
				<h1>Family Todo</h1>
				<div className="user-info">
					<span>{user?.signInDetails?.loginId}</span>
					<button type="button" onClick={signOut}>
						サインアウト
					</button>
				</div>
			</header>
			<main>
				<TodoList />
			</main>
		</div>
	);
}
