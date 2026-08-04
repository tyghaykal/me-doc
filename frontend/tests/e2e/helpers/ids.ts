const suffix = () => `${Date.now()}-${crypto.randomUUID().slice(0, 8)}`

export const uniqueEmail = () => `test-${suffix()}@example.com`
export const uniqueName = (prefix: string) => `${prefix}-${suffix()}`
