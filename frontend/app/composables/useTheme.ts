export function useTheme() {
  const isDark = useState('theme-isDark', () => true) // matches the SSR default

  function apply(dark: boolean) {
    document.documentElement.classList.toggle('dark', dark)
    localStorage.setItem('theme', dark ? 'dark' : 'light')
    isDark.value = dark
  }

  onMounted(() => {
    // SSR pages: the inline script in nuxt.config.ts already resolved the class
    // pre-paint, this just reads it back into reactive state. /app/** (ssr:
    // false): nothing has run yet, so this IS the resolution.
    const stored = localStorage.getItem('theme')
    apply(stored ? stored === 'dark' : true)
  })

  function toggleTheme() {
    apply(!isDark.value)
  }

  return { isDark, toggleTheme }
}
