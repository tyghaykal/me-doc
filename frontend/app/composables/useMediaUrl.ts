// The attachment/avatar bucket is private — reads go through the backend's
// authenticated download endpoints (see backend/src/pages/mod.rs
// `download_attachment`, backend/src/users/mod.rs `download_avatar`) instead
// of a direct MinIO URL. Auth rides the httpOnly refresh-token cookie, which
// the browser attaches automatically on a same-origin `<img src>` request —
// these URLs are stable and safe to persist inside document content, unlike
// one embedding a short-lived access token would be.

export function resolveAttachmentUrl(key: string): string {
  return `${useApiBase()}/attachments/download?key=${encodeURIComponent(key)}`
}

export function resolveAvatarUrl(key: string | null | undefined): string | null {
  if (!key) return null
  return `${useApiBase()}/auth/avatars/download?key=${encodeURIComponent(key)}`
}
