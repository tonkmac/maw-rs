export default async function handle(_ctx: { source: string; args: string[] }) {
  return { ok: true, output: JSON.stringify({ items: [], stats: { totalItems: 0, byRecipient: {}, byType: {}, oldestAgeHours: null, newestAgeHours: null }, errors: [], schemaVersion: 1 }) };
}
