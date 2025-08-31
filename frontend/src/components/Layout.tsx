import type { ReactNode } from 'react'
import { Link, useLocation } from 'react-router-dom'
import type { AuthUser } from 'aws-amplify/auth'
import { LogOut, Settings, CheckSquare, User } from 'lucide-react'

interface LayoutProps {
  children: ReactNode
  user?: AuthUser
  signOut?: () => void
}

export default function Layout({ children, user, signOut }: LayoutProps) {
  const location = useLocation()

  const isActive = (path: string) => location.pathname === path

  return (
    <div className="min-h-screen bg-gray-50">
      <nav className="bg-white shadow-sm border-b">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between h-16">
            <div className="flex items-center">
              <Link to="/" className="flex items-center space-x-2">
                <CheckSquare className="h-8 w-8 text-primary-600" />
                <span className="text-xl font-semibold text-gray-900">
                  Family Todo
                </span>
              </Link>
            </div>
            
            <div className="flex items-center space-x-4">
              <Link
                to="/"
                className={`px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                  isActive('/') 
                    ? 'text-primary-600 bg-primary-50' 
                    : 'text-gray-600 hover:text-gray-900'
                }`}
              >
                ToDo一覧
              </Link>
              
              <Link
                to="/settings"
                className={`px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                  isActive('/settings') 
                    ? 'text-primary-600 bg-primary-50' 
                    : 'text-gray-600 hover:text-gray-900'
                }`}
              >
                <Settings className="h-4 w-4" />
              </Link>
              
              <div className="flex items-center space-x-2 pl-4 border-l border-gray-200">
                <div className="flex items-center space-x-2">
                  <User className="h-4 w-4 text-gray-500" />
                  <span className="text-sm text-gray-700">
                    {user?.signInDetails?.loginId || 'ユーザー'}
                  </span>
                </div>
                
                <button
                  type="button"
                  onClick={signOut}
                  className="p-2 text-gray-500 hover:text-gray-700 transition-colors"
                  title="ログアウト"
                >
                  <LogOut className="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </nav>
      
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {children}
      </main>
    </div>
  )
}