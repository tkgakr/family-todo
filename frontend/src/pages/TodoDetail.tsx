import { useParams, useNavigate } from 'react-router-dom'
import { ArrowLeft, Clock, User } from 'lucide-react'
import { useTodo, useTodoHistory } from '../hooks/useTodos'
import { TodoStatus } from '../types/todo'

export default function TodoDetail() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  
  const { data: todo, isLoading: todoLoading } = useTodo(id!)
  const { data: history, isLoading: historyLoading } = useTodoHistory(id!)

  if (todoLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
      </div>
    )
  }

  if (!todo) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-md p-4">
        <p className="text-red-800">ToDoが見つかりませんでした。</p>
      </div>
    )
  }

  return (
    <div>
      <div className="mb-6">
        <button
          onClick={() => navigate('/')}
          className="flex items-center space-x-2 text-gray-600 hover:text-gray-900 transition-colors"
        >
          <ArrowLeft className="h-4 w-4" />
          <span>一覧に戻る</span>
        </button>
      </div>

      <div className="bg-white border border-gray-200 rounded-lg p-6 mb-6">
        <div className="flex items-start justify-between mb-4">
          <h1 className={`text-2xl font-bold ${todo.status === TodoStatus.Completed ? 'line-through text-gray-500' : 'text-gray-900'}`}>
            {todo.title}
          </h1>
          <span className={`px-3 py-1 rounded-full text-sm font-medium ${
            todo.status === TodoStatus.Active 
              ? 'bg-green-100 text-green-800'
              : 'bg-gray-100 text-gray-800'
          }`}>
            {todo.status === TodoStatus.Active ? 'アクティブ' : '完了済み'}
          </span>
        </div>

        {todo.description && (
          <div className="mb-4">
            <h2 className="text-sm font-medium text-gray-700 mb-2">説明</h2>
            <p className="text-gray-900 whitespace-pre-wrap">{todo.description}</p>
          </div>
        )}

        {todo.tags.length > 0 && (
          <div className="mb-4">
            <h2 className="text-sm font-medium text-gray-700 mb-2">タグ</h2>
            <div className="flex flex-wrap gap-1">
              {todo.tags.map((tag) => (
                <span
                  key={tag}
                  className="px-2 py-1 bg-gray-100 text-gray-700 text-sm rounded-full"
                >
                  {tag}
                </span>
              ))}
            </div>
          </div>
        )}

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 pt-4 border-t border-gray-200">
          <div>
            <h3 className="text-sm font-medium text-gray-700 mb-1">作成日</h3>
            <p className="text-gray-900 flex items-center space-x-1">
              <Clock className="h-4 w-4" />
              <span>{new Date(todo.created_at).toLocaleString('ja-JP')}</span>
            </p>
          </div>
          {todo.completed_at && (
            <div>
              <h3 className="text-sm font-medium text-gray-700 mb-1">完了日</h3>
              <p className="text-gray-900 flex items-center space-x-1">
                <Clock className="h-4 w-4" />
                <span>{new Date(todo.completed_at).toLocaleString('ja-JP')}</span>
              </p>
            </div>
          )}
          <div>
            <h3 className="text-sm font-medium text-gray-700 mb-1">作成者</h3>
            <p className="text-gray-900 flex items-center space-x-1">
              <User className="h-4 w-4" />
              <span>{todo.created_by}</span>
            </p>
          </div>
          <div>
            <h3 className="text-sm font-medium text-gray-700 mb-1">バージョン</h3>
            <p className="text-gray-900">v{todo.version}</p>
          </div>
        </div>
      </div>

      {/* History Section */}
      <div className="bg-white border border-gray-200 rounded-lg p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">変更履歴</h2>
        
        {historyLoading ? (
          <div className="flex items-center justify-center py-8">
            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-primary-600"></div>
          </div>
        ) : (
          <div className="space-y-4">
            {history?.events.map((event, index) => (
              <div key={event.event_id} className="flex items-start space-x-3">
                <div className="flex-shrink-0">
                  <div className="w-2 h-2 bg-primary-600 rounded-full mt-2"></div>
                </div>
                <div className="flex-1">
                  <div className="flex items-center space-x-2">
                    <span className="text-sm font-medium text-gray-900">
                      {event.event_type.replace('todo_', '').replace('_v1', '').replace('_v2', '')}
                    </span>
                    <span className="text-xs text-gray-500">
                      {new Date(event.timestamp).toLocaleString('ja-JP')}
                    </span>
                  </div>
                  <p className="text-sm text-gray-600 mt-1">
                    イベントID: {event.event_id}
                  </p>
                </div>
              </div>
            ))}
            
            {(!history?.events || history.events.length === 0) && (
              <p className="text-gray-500 text-center py-4">変更履歴がありません</p>
            )}
          </div>
        )}
      </div>
    </div>
  )
}