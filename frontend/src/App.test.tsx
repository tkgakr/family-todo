import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import App from './App'

describe('App', () => {
  it('renders the app title', () => {
    render(<App />)
    expect(screen.getByText('家族用 ToDo アプリ')).toBeInTheDocument()
  })

  it('renders the placeholder message', () => {
    render(<App />)
    expect(screen.getByText('アプリケーションの基盤が構築されました。')).toBeInTheDocument()
  })
})
