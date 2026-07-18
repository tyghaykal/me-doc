// Shared flag so LandingNav/LandingHero can wait for index.vue's session
// check to resolve before picking Login/Get-started vs. Open App, instead of
// rendering the logged-out state first and flashing to the right one.
export function useAuthReady() {
  return useState('landing-auth-ready', () => false)
}
