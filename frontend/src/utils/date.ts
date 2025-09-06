// 日付関連のユーティリティ

// 日付の妥当性チェック
export function isValidDate(date: unknown): date is Date {
  return date instanceof Date && !Number.isNaN(date.getTime())
}

// 文字列から日付オブジェクトを作成
export function parseDate(dateString: string): Date | null {
  try {
    const date = new Date(dateString)
    return isValidDate(date) ? date : null
  } catch {
    return null
  }
}

// ISO 8601 形式の文字列に変換
export function toISOString(date: Date): string {
  return date.toISOString()
}

// ローカル日付文字列に変換（YYYY-MM-DD）
export function toLocalDateString(date: Date): string {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')

  return `${year}-${month}-${day}`
}

// ローカル時刻文字列に変換（HH:MM）
export function toLocalTimeString(date: Date): string {
  const hours = String(date.getHours()).padStart(2, '0')
  const minutes = String(date.getMinutes()).padStart(2, '0')

  return `${hours}:${minutes}`
}

// ローカル日時文字列に変換（YYYY-MM-DD HH:MM）
export function toLocalDateTimeString(date: Date): string {
  return `${toLocalDateString(date)} ${toLocalTimeString(date)}`
}

// 日付の比較
export function isSameDay(date1: Date, date2: Date): boolean {
  return (
    date1.getFullYear() === date2.getFullYear() &&
    date1.getMonth() === date2.getMonth() &&
    date1.getDate() === date2.getDate()
  )
}

// 今日かどうかの判定
export function isToday(date: Date): boolean {
  return isSameDay(date, new Date())
}

// 昨日かどうかの判定
export function isYesterday(date: Date): boolean {
  const yesterday = new Date()
  yesterday.setDate(yesterday.getDate() - 1)
  return isSameDay(date, yesterday)
}

// 明日かどうかの判定
export function isTomorrow(date: Date): boolean {
  const tomorrow = new Date()
  tomorrow.setDate(tomorrow.getDate() + 1)
  return isSameDay(date, tomorrow)
}

// 過去の日付かどうかの判定
export function isPast(date: Date): boolean {
  return date < new Date()
}

// 未来の日付かどうかの判定
export function isFuture(date: Date): boolean {
  return date > new Date()
}

// 日付の差分を計算（日数）
export function getDaysDifference(date1: Date, date2: Date): number {
  const timeDifference = date2.getTime() - date1.getTime()
  return Math.floor(timeDifference / (1000 * 60 * 60 * 24))
}

// 日付の差分を計算（時間）
export function getHoursDifference(date1: Date, date2: Date): number {
  const timeDifference = date2.getTime() - date1.getTime()
  return Math.floor(timeDifference / (1000 * 60 * 60))
}

// 日付の差分を計算（分）
export function getMinutesDifference(date1: Date, date2: Date): number {
  const timeDifference = date2.getTime() - date1.getTime()
  return Math.floor(timeDifference / (1000 * 60))
}

// 日付に日数を追加
export function addDays(date: Date, days: number): Date {
  const result = new Date(date)
  result.setDate(result.getDate() + days)
  return result
}

// 日付に時間を追加
export function addHours(date: Date, hours: number): Date {
  const result = new Date(date)
  result.setHours(result.getHours() + hours)
  return result
}

// 日付に分を追加
export function addMinutes(date: Date, minutes: number): Date {
  const result = new Date(date)
  result.setMinutes(result.getMinutes() + minutes)
  return result
}

// 月の開始日を取得
export function getStartOfMonth(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), 1)
}

// 月の終了日を取得
export function getEndOfMonth(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth() + 1, 0)
}

// 週の開始日を取得（月曜日始まり）
export function getStartOfWeek(date: Date): Date {
  const result = new Date(date)
  const day = result.getDay()
  const diff = result.getDate() - day + (day === 0 ? -6 : 1) // 月曜日を週の開始とする
  result.setDate(diff)
  result.setHours(0, 0, 0, 0)
  return result
}

// 週の終了日を取得（日曜日終わり）
export function getEndOfWeek(date: Date): Date {
  const result = getStartOfWeek(date)
  result.setDate(result.getDate() + 6)
  result.setHours(23, 59, 59, 999)
  return result
}

// 日付の範囲内かどうかの判定
export function isDateInRange(date: Date, startDate: Date, endDate: Date): boolean {
  return date >= startDate && date <= endDate
}

// 年齢を計算
export function calculateAge(birthDate: Date): number {
  const today = new Date()
  let age = today.getFullYear() - birthDate.getFullYear()
  const monthDiff = today.getMonth() - birthDate.getMonth()

  if (monthDiff < 0 || (monthDiff === 0 && today.getDate() < birthDate.getDate())) {
    age--
  }

  return age
}

// タイムゾーンを考慮した日付の作成
export function createDateInTimezone(
  year: number,
  month: number,
  day: number,
  timezone = 'Asia/Tokyo'
): Date {
  const date = new Date()
  date.setFullYear(year, month - 1, day)
  date.setHours(0, 0, 0, 0)

  // タイムゾーンオフセットを調整
  const formatter = new Intl.DateTimeFormat('ja-JP', {
    timeZone: timezone,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  })

  const parts = formatter.formatToParts(date)
  const formattedYear = Number.parseInt(parts.find((part) => part.type === 'year')?.value || '0')
  const formattedMonth = Number.parseInt(parts.find((part) => part.type === 'month')?.value || '0')
  const formattedDay = Number.parseInt(parts.find((part) => part.type === 'day')?.value || '0')

  return new Date(formattedYear, formattedMonth - 1, formattedDay)
}
