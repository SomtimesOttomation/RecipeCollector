import { checkForUpdate, installUpdate, type UpdateStatus } from "./api";

/** Shared update state, so the banner and the Settings card agree. */
export const updates = $state({
  status: null as UpdateStatus | null,
  checking: false,
  installing: false,
  error: "",
  dismissed: false,
});

export async function runCheck() {
  if (updates.checking) return;
  updates.checking = true;
  updates.error = "";
  try {
    updates.status = await checkForUpdate();
  } catch (e) {
    updates.error = String(e);
  } finally {
    updates.checking = false;
  }
}

export async function runInstall() {
  const s = updates.status;
  if (!s || s.state !== "available" || !s.installer_url || updates.installing) return;
  updates.installing = true;
  updates.error = "";
  try {
    await installUpdate(s.installer_url, s.checksum_url);
  } catch (e) {
    updates.error = String(e);
  } finally {
    updates.installing = false;
  }
}
