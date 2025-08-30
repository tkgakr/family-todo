import { Routes, Route } from 'react-router-dom'
import { Authenticator } from '@aws-amplify/ui-react'
import Layout from './components/Layout'
import TodoList from './pages/TodoList'
import TodoDetail from './pages/TodoDetail'
import Settings from './pages/Settings'

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