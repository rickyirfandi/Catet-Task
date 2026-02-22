import { jiraLogin, jiraLogout, jiraVerify } from '$lib/api/tauri';
import type { JiraUser } from '$lib/types';

let user = $state<JiraUser | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);

export function getUser() { return user; }
export function getLoading() { return loading; }
export function getError() { return error; }
export function isLoggedIn() { return user !== null; }

export function getInitials(): string {
  if (!user?.displayName) return '??';
  const parts = user.displayName.split(' ');
  return parts.map(p => p[0]).join('').toUpperCase().slice(0, 2);
}

export async function login(domain: string, email: string, token: string) {
  loading = true;
  error = null;
  try {
    user = await jiraLogin(domain, email, token);
  } catch (e) {
    error = String(e);
    throw e;
  } finally {
    loading = false;
  }
}

export async function logout() {
  try {
    await jiraLogout();
  } finally {
    user = null;
  }
}

export async function tryAutoLogin() {
  loading = true;
  try {
    user = await jiraVerify();
  } catch {
    user = null;
  } finally {
    loading = false;
  }
}
