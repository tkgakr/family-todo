import { Authenticator } from "@aws-amplify/ui-react"
import { Route, Routes } from "react-router-dom"
import Layout from "./components/Layout"
import Settings from "./pages/Settings"
import TodoDetail from "./pages/TodoDetail"
import TodoList from "./pages/TodoList"

function App() {
  return (
    <Authenticator>
      {({ signOut, user }) => (
        <Layout user={user} signOut={signOut}>
          <Routes>
            <Route path="/" element={<TodoList />} />
            <Route path="/todos/:id" element={<TodoDetail />} />
            <Route path="/settings" element={<Settings />} />
          </Routes>
        </Layout>
      )}
    </Authenticator>
  )
}

export default App
