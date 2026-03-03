const SQLITE_UTC_RE = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)?$/;

export function parseAppDate(value: string): Date | null {
  const raw = value?.trim();
  if (!raw) return null;

  if (SQLITE_UTC_RE.test(raw)) {
    const sqliteAsUtc = `${raw.replace(' ', 'T')}Z`;
    const parsedSqlite = new Date(sqliteAsUtc);
    if (!Number.isNaN(parsedSqlite.getTime())) {
      return parsedSqlite;
    }
  }

  const parsed = new Date(raw);
  if (!Number.isNaN(parsed.getTime())) {
    return parsed;
  }

  const normalizedOffset = raw.replace(/([+-]\d{2})(\d{2})$/, '$1:$2');
  if (normalizedOffset !== raw) {
    const parsedNormalized = new Date(normalizedOffset);
    if (!Number.isNaN(parsedNormalized.getTime())) {
      return parsedNormalized;
    }
  }

  return null;
}

/** Format a Date into local YYYY-MM-DD key */
export function toLocalDateKey(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

/** Convert an app datetime string into local YYYY-MM-DD, if parsable */
export function toLocalDateKeyFromValue(value: string): string | null {
  const parsed = parseAppDate(value);
  return parsed ? toLocalDateKey(parsed) : null;
}

/** Current local date key in YYYY-MM-DD */
export function getTodayLocalDateKey(): string {
  return toLocalDateKey(new Date());
}

/** Format seconds as HH:MM:SS */
export function formatDuration(totalSecs: number): string {
  const h = Math.floor(totalSecs / 3600);
  const m = Math.floor((totalSecs % 3600) / 60);
  const s = Math.floor(totalSecs % 60);
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

/** Format seconds as short duration like "2h 30m" */
export function formatDurationShort(totalSecs: number): string {
  const h = Math.floor(totalSecs / 3600);
  const m = Math.floor((totalSecs % 3600) / 60);
  if (h === 0 && m === 0) return '0m';
  if (h === 0) return `${m}m`;
  if (m === 0) return `${h}h`;
  return `${h}h ${String(m).padStart(2, '0')}m`;
}

/** Format a date string as "HH:MM" */
export function formatTime(isoString: string): string {
  const d = parseAppDate(isoString);
  if (!d) return '--:--';
  return new Intl.DateTimeFormat([], {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(d);
}

/** Format a date string as "DD Mon YYYY, HH:MM" in local timezone */
export function formatDateTime(isoString: string): string {
  const d = parseAppDate(isoString);
  if (!d) return '--';
  return new Intl.DateTimeFormat([], {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(d);
}

/** Returns the current local timezone identifier */
export function getLocalTimezoneLabel(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || 'Local';
}

/** Format a date string as "DDD, DD MMM YYYY" */
export function formatDateHeader(isoString: string): string {
  const d = parseAppDate(isoString);
  if (!d) return 'INVALID DATE';
  const days = ['SUN', 'MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT'];
  const months = ['JAN', 'FEB', 'MAR', 'APR', 'MAY', 'JUN', 'JUL', 'AUG', 'SEP', 'OCT', 'NOV', 'DEC'];
  return `${days[d.getDay()]}, ${String(d.getDate()).padStart(2, '0')} ${months[d.getMonth()]} ${d.getFullYear()}`;
}
