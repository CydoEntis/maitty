import { useQuery } from '@tanstack/react-query'
import { isDesktop, loadPerformanceSample } from '@/api/daemon'
import { SAMPLE_EVERY_MS } from '@/lib/perf-meter'
import type { SettingsPreferences } from './preferences'
import type { PerformanceEnvironment } from './performance-report'

/** The host's process figures, asked for once a second while this page or the overlay shows them. */
export function useProcessSample() {
  return useQuery({ queryKey: ['performance-sample'], queryFn: loadPerformanceSample, enabled: isDesktop(), refetchInterval: SAMPLE_EVERY_MS, retry: false })
}

export function environmentOf(preferences: SettingsPreferences, appVersion: string | null): PerformanceEnvironment {
  return {
    version: appVersion,
    userAgent: navigator.userAgent,
    gpu: document.documentElement.dataset.gpu ?? 'unknown',
    style: preferences.style,
    reduceMotion: preferences.reduceMotion,
    terminalFontSize: preferences.terminalFontSize,
  }
}

