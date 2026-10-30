let audioContext: AudioContext | null = null
const NOTIFICATION_TONE_FREQUENCY_HERTZ = 660
const NOTIFICATION_TONE_VOLUME = 0.035
const NOTIFICATION_TONE_END_VOLUME = 0.001
const NOTIFICATION_TONE_DURATION_SECONDS = 0.16

export function enableNotificationSound(): boolean {
  try {
    audioContext ??= new AudioContext()
    if (audioContext.state === 'suspended') {
      void audioContext.resume().catch((error: unknown) => {
        console.error('Could not resume notification sound', error)
      })
    }
    return true
  } catch (error) {
    console.error('Could not enable notification sound', error)
    return false
  }
}

export function playNotificationSound(): void {
  if (!audioContext || audioContext.state !== 'running') return

  const oscillator = audioContext.createOscillator()
  const volume = audioContext.createGain()
  oscillator.type = 'sine'
  oscillator.frequency.value = NOTIFICATION_TONE_FREQUENCY_HERTZ
  volume.gain.setValueAtTime(NOTIFICATION_TONE_VOLUME, audioContext.currentTime)
  volume.gain.exponentialRampToValueAtTime(NOTIFICATION_TONE_END_VOLUME, audioContext.currentTime + NOTIFICATION_TONE_DURATION_SECONDS)
  oscillator.connect(volume)
  volume.connect(audioContext.destination)
  oscillator.start()
  oscillator.stop(audioContext.currentTime + NOTIFICATION_TONE_DURATION_SECONDS)
}
