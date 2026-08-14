import type { SubscriptionBillingCycle } from "./types";

const DATE_PATTERN = /^(\d{4})-(\d{2})-(\d{2})$/;

/**
 * Materialize the inclusive coverage end shown before a subscription receipt
 * is selected. The native layer performs the same calculation authoritatively
 * when the file is registered.
 */
export function subscriptionCoverageEnd(
  coverageStart: string,
  billingCycle: SubscriptionBillingCycle
): string | null {
  const match = DATE_PATTERN.exec(coverageStart);
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const start = new Date(Date.UTC(year, month - 1, day));
  if (
    start.getUTCFullYear() !== year ||
    start.getUTCMonth() !== month - 1 ||
    start.getUTCDate() !== day
  ) return null;

  const months = billingCycle === "monthly" ? 1 : 12;
  const targetMonthIndex = month - 1 + months;
  const targetYear = year + Math.floor(targetMonthIndex / 12);
  const targetMonth = targetMonthIndex % 12;
  const lastTargetDay = new Date(Date.UTC(targetYear, targetMonth + 1, 0)).getUTCDate();
  const nextRenewal = new Date(Date.UTC(targetYear, targetMonth, Math.min(day, lastTargetDay)));
  nextRenewal.setUTCDate(nextRenewal.getUTCDate() - 1);
  return nextRenewal.toISOString().slice(0, 10);
}
