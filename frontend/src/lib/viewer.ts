import {writable} from 'svelte/store';

export const DEFAULT_OPEN_VIEWER_IN_NEW_TAB = true;

/** How the IGC track viewer is opened. */
export interface ViewerPrefs {
    openViewerInNewTab: boolean;
}

const STORAGE_KEY = 'flugbuech-viewer';

/**
 * Store with the preferences for the IGC track viewer.
 *
 * Mirrors the preference saved on the user record; persisted per browser so
 * it is available on flight pages without loading the profile first.
 */
export const viewer = writable<ViewerPrefs>({
    openViewerInNewTab: DEFAULT_OPEN_VIEWER_IN_NEW_TAB,
});

/**
 * Restore the stored viewer preference, if any. Called once on client-side
 * app initialization.
 */
export function initializeViewer(): void {
    if (typeof localStorage === 'undefined') {
        return;
    }
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === null) {
        return;
    }
    let parsed: Partial<ViewerPrefs>;
    try {
        parsed = JSON.parse(raw) as Partial<ViewerPrefs>;
    } catch {
        return;
    }
    if (typeof parsed.openViewerInNewTab === 'boolean') {
        viewer.update((v) => ({...v, openViewerInNewTab: parsed.openViewerInNewTab as boolean}));
    }
}

/**
 * Apply the user's saved viewer preference (from their profile) and persist
 * it for this browser.
 */
export function applyProfileViewer(openViewerInNewTab: boolean): void {
    viewer.update((v) => ({...v, openViewerInNewTab}));
    if (typeof localStorage !== 'undefined') {
        localStorage.setItem(STORAGE_KEY, JSON.stringify({openViewerInNewTab}));
    }
}
