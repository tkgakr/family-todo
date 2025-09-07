import { Authenticator } from "@aws-amplify/ui-react"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { Amplify } from "aws-amplify"
import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { BrowserRouter } from "react-router-dom"
import "@aws-amplify/ui-react/styles.css"
import "./index.css"
import App from "./App"
import { amplifyConfig } from "./config/amplify"

Amplify.configure(amplifyConfig)

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 2,
      staleTime: 5 * 60 * 1000, // 5 minutes
    },
  },
})

const rootElement = document.getElementById("root")
if (!rootElement) throw new Error("Root element not found")

createRoot(rootElement).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Authenticator.Provider>
        <BrowserRouter>
          <App />
        </BrowserRouter>
      </Authenticator.Provider>
    </QueryClientProvider>
  </StrictMode>,
)
