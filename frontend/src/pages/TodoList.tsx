import { useState } from 'react'
import { Plus, Check, Clock, Trash2, Edit } from 'lucide-react'
import { useTodos, useCreateTodo, useCompleteTodo, useDeleteTodo } from '../hooks/useTodos'
import { TodoStatus, type CreateTodoRequest } from '../types/todo'
import TodoForm from '../components/TodoForm'

export default function TodoList() {
  const [showForm, setShowForm] = useState(false)
  const [activeTab, setActiveTab] = useState<TodoStatus>(TodoStatus.Active)
  
  const { data: todosData, isLoading, error } = useTodos(activeTab)
  const createTodoMutation = useCreateTodo()
  const completeTodoMutation = useCompleteTodo()
  const deleteTodoMutation = useDeleteTodo()

  const handleCreateTodo = async (data: CreateTodoRequest) => {
    try {
      await createTodoMutation.mutateAsync(data)
      setShowForm(false)
    } catch (error) {
      console.error('Failed to create todo:', error)
    }
  }

  const handleCompleteTodo = async (id: string) => {
    try {
      await completeTodoMutation.mutateAsync(id)
    } catch (error) {
      console.error('Failed to complete todo:', error)
    }
  }

  const handleDeleteTodo = async (id: string) => {
    if (window.confirm('このToDoを削除しますか？')) {
      try {
        await deleteTodoMutation.mutateAsync({ id })
      } catch (error) {
        console.error('Failed to delete todo:', error)
      }
    }
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600" />
      </div>
    )
  }

  if (error) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-md p-4">
        <p className="text-red-800">エラーが発生しました。ページを再読み込みしてください。</p>
      </div>
    )
  }

  const todos = todosData?.todos || []

  return (
    <div>
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold text-gray-900">ToDo一覧</h1>
        <button
          type="button"
          onClick={() => setShowForm(true)}
          className="bg-primary-600 text-white px-4 py-2 rounded-md flex items-center space-x-2 hover:bg-primary-700 transition-colors"
        >
          <Plus className="h-4 w-4" />
          <span>新規作成</span>
        </button>
      </div>

      {/* Tabs */}
      <div className="flex space-x-1 mb-6">
        {[
          { status: TodoStatus.Active, label: 'アクティブ', icon: Clock },
          { status: TodoStatus.Completed, label: '完了済み', icon: Check },
        ].map(({ status, label, icon: Icon }) => (
          <button
            type="button"
            key={status}
            onClick={() => setActiveTab(status)}
            className={`flex items-center space-x-2 px-4 py-2 rounded-md font-medium transition-colors ${
              activeTab === status
                ? 'bg-primary-100 text-primary-700 border border-primary-200'
                : 'bg-white text-gray-600 border border-gray-200 hover:bg-gray-50'
            }`}
          >
            <Icon className="h-4 w-4" />
            <span>{label}</span>
          </button>
        ))}
      </div>

      {/* Todo List */}
      <div className="space-y-4">
        {todos.length === 0 ? (
          <div className="text-center py-12 bg-white rounded-lg border border-gray-200">
            <p className="text-gray-500">
              {activeTab === TodoStatus.Active ? 'アクティブなToDoはありません' : '完了したToDoはありません'}
            </p>
          </div>
        ) : (
          todos.map((todo) => (
            <div
              key={todo.id}
              className="bg-white border border-gray-200 rounded-lg p-4 hover:shadow-md transition-shadow"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <h3 className={`font-medium ${todo.status === TodoStatus.Completed ? 'line-through text-gray-500' : 'text-gray-900'}`}>
                    {todo.title}
                  </h3>
                  {todo.description && (
                    <p className="text-gray-600 text-sm mt-1">{todo.description}</p>
                  )}
                  {todo.tags.length > 0 && (
                    <div className="flex flex-wrap gap-1 mt-2">
                      {todo.tags.map((tag) => (
                        <span
                          key={tag}
                          className="px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded-full"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  )}
                  <p className="text-xs text-gray-500 mt-2">
                    作成日: {new Date(todo.created_at).toLocaleDateString('ja-JP')}
                    {todo.completed_at && (
                      <span className="ml-4">
                        完了日: {new Date(todo.completed_at).toLocaleDateString('ja-JP')}
                      </span>
                    )}
                  </p>
                </div>
                
                <div className="flex items-center space-x-2 ml-4">
                  {todo.status === TodoStatus.Active && (
                    <>
                      <button
                        type="button"
                        onClick={() => handleCompleteTodo(todo.id)}
                        disabled={completeTodoMutation.isPending}
                        className="p-2 text-green-600 hover:bg-green-50 rounded-md transition-colors"
                        title="完了"
                      >
                        <Check className="h-4 w-4" />
                      </button>
                      <button
                        type="button"
                        className="p-2 text-blue-600 hover:bg-blue-50 rounded-md transition-colors"
                        title="編集"
                      >
                        <Edit className="h-4 w-4" />
                      </button>
                    </>
                  )}
                  <button
                    type="button"
                    onClick={() => handleDeleteTodo(todo.id)}
                    disabled={deleteTodoMutation.isPending}
                    className="p-2 text-red-600 hover:bg-red-50 rounded-md transition-colors"
                    title="削除"
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>
            </div>
          ))
        )}
      </div>

      {/* Create Todo Modal */}
      {showForm && (
        <TodoForm
          onSubmit={handleCreateTodo}
          onCancel={() => setShowForm(false)}
          isLoading={createTodoMutation.isPending}
        />
      )}
    </div>
  )
}