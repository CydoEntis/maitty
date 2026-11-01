import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { afterEach } from 'vitest'

Object.defineProperty(globalThis, 'ResizeObserver', {
  configurable: true,
  writable: true,
  value: class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  },
})

// jsdom defaults to 1024px, which is the compact (drawer) layout; tests assume the docked sidebar.
Object.defineProperty(window, 'innerWidth', { configurable: true, writable: true, value: 1280 })

afterEach(() => { cleanup(); })
