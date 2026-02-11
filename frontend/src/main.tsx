import { Authenticator } from "@aws-amplify/ui-react";
import "@aws-amplify/ui-react/styles.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { configureAmplify } from "./config/amplify";
import "./index.css";

configureAmplify();

const root = document.getElementById("root");
if (root) {
	createRoot(root).render(
		<StrictMode>
			<Authenticator>
				{({ signOut, user }) => <App user={user} signOut={signOut} />}
			</Authenticator>
		</StrictMode>,
	);
}
