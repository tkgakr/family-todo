import { User, Users, Bell, Shield } from 'lucide-react'

export default function Settings() {
  return (
    <div>
      <h1 className="text-2xl font-bold text-gray-900 mb-6">設定</h1>
      
      <div className="space-y-6">
        {/* Profile Section */}
        <div className="bg-white border border-gray-200 rounded-lg p-6">
          <div className="flex items-center space-x-3 mb-4">
            <User className="h-5 w-5 text-gray-600" />
            <h2 className="text-lg font-semibold text-gray-900">プロフィール</h2>
          </div>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                表示名
              </label>
              <input
                type="text"
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
                placeholder="表示名を入力"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                メールアドレス
              </label>
              <input
                type="email"
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
                placeholder="email@example.com"
                disabled
              />
            </div>
          </div>
          
          <div className="mt-4">
            <button className="bg-primary-600 text-white px-4 py-2 rounded-md hover:bg-primary-700 transition-colors">
              プロフィールを更新
            </button>
          </div>
        </div>

        {/* Family Settings */}
        <div className="bg-white border border-gray-200 rounded-lg p-6">
          <div className="flex items-center space-x-3 mb-4">
            <Users className="h-5 w-5 text-gray-600" />
            <h2 className="text-lg font-semibold text-gray-900">家族設定</h2>
          </div>
          
          <p className="text-gray-600 mb-4">
            現在の家族グループのメンバーを管理します。
          </p>
          
          <div className="space-y-3">
            <div className="flex items-center justify-between py-3 border-b border-gray-100">
              <div className="flex items-center space-x-3">
                <div className="w-8 h-8 bg-primary-100 rounded-full flex items-center justify-center">
                  <span className="text-primary-600 font-medium text-sm">U1</span>
                </div>
                <div>
                  <p className="font-medium text-gray-900">あなた</p>
                  <p className="text-sm text-gray-500">管理者</p>
                </div>
              </div>
            </div>
          </div>
          
          <div className="mt-4">
            <button className="bg-primary-600 text-white px-4 py-2 rounded-md hover:bg-primary-700 transition-colors">
              メンバーを招待
            </button>
          </div>
        </div>

        {/* Notifications */}
        <div className="bg-white border border-gray-200 rounded-lg p-6">
          <div className="flex items-center space-x-3 mb-4">
            <Bell className="h-5 w-5 text-gray-600" />
            <h2 className="text-lg font-semibold text-gray-900">通知設定</h2>
          </div>
          
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-gray-900">新しいToDoが追加されたとき</p>
                <p className="text-sm text-gray-500">家族が新しいToDoを作成したときに通知</p>
              </div>
              <label className="relative inline-flex items-center cursor-pointer">
                <input type="checkbox" className="sr-only peer" />
                <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary-600"></div>
              </label>
            </div>
            
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-gray-900">ToDoが完了されたとき</p>
                <p className="text-sm text-gray-500">家族がToDoを完了したときに通知</p>
              </div>
              <label className="relative inline-flex items-center cursor-pointer">
                <input type="checkbox" className="sr-only peer" defaultChecked />
                <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary-600"></div>
              </label>
            </div>
          </div>
        </div>

        {/* Security */}
        <div className="bg-white border border-gray-200 rounded-lg p-6">
          <div className="flex items-center space-x-3 mb-4">
            <Shield className="h-5 w-5 text-gray-600" />
            <h2 className="text-lg font-semibold text-gray-900">セキュリティ</h2>
          </div>
          
          <div className="space-y-4">
            <div>
              <button className="text-primary-600 hover:text-primary-700 font-medium">
                パスワードを変更
              </button>
            </div>
            <div>
              <button className="text-primary-600 hover:text-primary-700 font-medium">
                二要素認証を設定
              </button>
            </div>
            <div>
              <button className="text-red-600 hover:text-red-700 font-medium">
                アカウントを削除
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}