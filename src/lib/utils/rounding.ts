/**
 * Round seconds to the nearest minute increment.
 * Minimum result is 1 increment.
 */
export function roundToNearest(seconds: number, minuteIncrement: number): number {
  const totalMinutes = seconds / 60;
  const rounded = Math.round(totalMinutes / minuteIncrement) * minuteIncrement;
  return Math.max(minuteIncrement, rounded) * 60;
}
